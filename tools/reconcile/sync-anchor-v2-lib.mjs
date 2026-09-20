// sync-anchor-v2-lib.mjs — pure(-ish) logic for the atom-level sync-anchor-v2 parser
// (docs/doctrines/023-five-dimension-cloud-shard-contract.md Section 6, marked NOT_BUILT there).
//
// For one crate-local capability record (from `.chronica/sub-cap-arch.jsonl`) this compares
// three surfaces mechanically: (a) CODE — a real (non-stub) source file at the capability's
// target_module, reachable from the crate's own module tree AND referenced from somewhere else
// in the crate, not just declared (doc 180's "code without a shipped caller is ISLAND" rule,
// made mechanical); (b) SHARD — the capability's own `status` field in the committed shard; (c)
// DOCS — the crate doc's (`docs/3NN-crate-<crate>.md`) capabilities table row for the same key.
//
// Deliberately NOT attempted here (out of scope, needs human/agent judgment): donor-parity
// (tier 4, does the implementation match donor behavior) and production-observed (tier 6, is it
// proven in a live deployment). A capability with no code match and shard status "unimplemented"
// is reported AGREE — the expected, honest state for most of the donor-capability catalog today.
//
// Module-target matching is name-based, a heuristic not a compiler -- see
// sync-anchor-v2-module-index.mjs's own header for the exact/prefix confidence rules; every
// finding carries `match_confidence` so a human/CI reviewer can weight it, which is why
// deliverable 3 wires the gate as report-only, not blocking.
//
// Shard-file loading, schema validation, path-traversal/symlink-containment safety, bounded
// reads, and output-sanitization primitives live in sibling modules (sync-anchor-v2-shard.mjs,
// sync-anchor-v2-schema.mjs, sync-anchor-v2-fs-safety.mjs, sync-anchor-v2-sanitize.mjs), and
// reachable-module-file walking/target_module matching lives in sync-anchor-v2-module-index.mjs
// -- split out across several repair rounds to keep every file under this repo's ~500-line
// convention (docs/doctrines/023a-sync-anchor-v2-repair-log.md has the full history).
import { join, relative, sep } from 'node:path'
import {
  BOUNDS, boundedText, comparePlain, discoverWorkspaceCrates, discoverWorkspaceCratesWithFindings,
  escapeMarkdownCell, isPathWithinRoot, loadCrateDocCapabilityStatus, loadCrateShard, redactValue,
  sanitizeCapabilityKey, sanitizeCrateName, sanitizeEnumField, sanitizeErrorForDisplay,
  sanitizePathField, sanitizeTargetModule, SourceCache,
} from './sync-anchor-v2-shard.mjs'
import {
  buildCrateModuleIndex, buildReachableModuleFiles, classifyFileBody, hasTestEvidence,
  isModuleReferencedElsewhere, matchCapabilityModule,
} from './sync-anchor-v2-module-index.mjs'

const slash = (path) => path.split(sep).join('/')

// Re-exported so the CLI and every pre-existing test can keep importing shard-file/module-index
// concerns from this module's public surface without knowing about the internal module split.
export {
  BOUNDS, boundedText, buildCrateModuleIndex, buildReachableModuleFiles, classifyFileBody,
  comparePlain, discoverWorkspaceCrates, discoverWorkspaceCratesWithFindings, escapeMarkdownCell,
  hasTestEvidence, isModuleReferencedElsewhere, isPathWithinRoot, loadCrateDocCapabilityStatus,
  loadCrateShard, matchCapabilityModule, redactValue, sanitizeCapabilityKey, sanitizeCrateName,
  sanitizeEnumField, sanitizeErrorForDisplay, sanitizePathField, sanitizeTargetModule, SourceCache,
}

// ── capability classification ───────────────────────────────────────────────────────────────

export const STATUS = {
  UNIMPLEMENTED: 'unimplemented',
  IMPLEMENTED: 'implemented',
  IMPLEMENTED_UNVERIFIED: 'implemented_unverified',
  VERIFIED: 'verified',
}

export const REASON = {
  // per-capability classification reasons (returned by classifyCapability)
  SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL: 'SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL',
  CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT: 'CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT',
  SHARD_CLAIMS_IMPLEMENTED_BUT_NO_REAL_CODE_FOUND: 'SHARD_CLAIMS_IMPLEMENTED_BUT_NO_REAL_CODE_FOUND',
  SHARD_CLAIMS_TESTED_BUT_NO_TEST_FILE_REFERENCES_MODULE: 'SHARD_CLAIMS_TESTED_BUT_NO_TEST_FILE_REFERENCES_MODULE',
  DOC_STATUS_CONTRADICTS_SHARD_STATUS: 'DOC_STATUS_CONTRADICTS_SHARD_STATUS',
  SHARD_STATUS_VALUE_INVALID: 'SHARD_STATUS_VALUE_INVALID',
  DOC_STATUS_VALUE_INVALID: 'DOC_STATUS_VALUE_INVALID',
  AMBIGUOUS_MODULE_MATCH: 'AMBIGUOUS_MODULE_MATCH',
  // shard-level structural findings (surfaced separately by verifyCrate/verifyCrates so a clean
  // disagree_count can never hide a structural defect)
  SHARD_PARSE_ERROR: 'SHARD_PARSE_ERROR',
  SHARD_RECORD_INVALID: 'SHARD_RECORD_INVALID',
  SHARD_CAPABILITY_KEY_DUPLICATE: 'SHARD_CAPABILITY_KEY_DUPLICATE',
  // repo/workspace-level structural findings
  WORKSPACE_MEMBER_PATH_ESCAPES_ROOT: 'WORKSPACE_MEMBER_PATH_ESCAPES_ROOT',
  // operational findings: the scan itself could not fully/safely run over this input (distinct
  // from a DATA finding about shard content) -- always reported, never suppressed by --report,
  // always forces a non-zero exit (see sync-anchor-v2-parser.mjs).
  SOURCE_FILES_TRUNCATED: 'SOURCE_FILES_TRUNCATED',
  SOURCE_FILE_OVERSIZED: 'SOURCE_FILE_OVERSIZED',
  SOURCE_FILE_UNREADABLE: 'SOURCE_FILE_UNREADABLE',
  SOURCE_PATH_ESCAPES_ROOT: 'SOURCE_PATH_ESCAPES_ROOT',
  MANIFEST_OVERSIZED: 'MANIFEST_OVERSIZED',
  MANIFEST_UNREADABLE: 'MANIFEST_UNREADABLE',
  DOC_OVERSIZED: 'DOC_OVERSIZED',
  DOC_UNREADABLE: 'DOC_UNREADABLE',
  DOC_DIR_TRUNCATED: 'DOC_DIR_TRUNCATED',
  // a workspace member's Cargo.toml path that could not even be RESOLVED (not found, or a
  // stat/realpath failure other than an escape); opendir/readdir itself failing mid-enumeration
  // gets its own code instead of surfacing as an uncaught native exception.
  WORKSPACE_MEMBER_UNRESOLVED: 'WORKSPACE_MEMBER_UNRESOLVED',
  DIR_ENUMERATION_UNREADABLE: 'DIR_ENUMERATION_UNREADABLE',
  // repair round 7, item 1: a "claim" shard_status (implemented/implemented_unverified/verified)
  // whose doc-status could not be checked -- doc consulted but silent on this key (MISSING) vs.
  // doc itself unavailable (UNVERIFIABLE) -- absence of doc evidence is never doc agreement.
  DOC_STATUS_MISSING_FOR_CLAIM: 'DOC_STATUS_MISSING_FOR_CLAIM',
  DOC_STATUS_UNVERIFIABLE_FOR_CLAIM: 'DOC_STATUS_UNVERIFIABLE_FOR_CLAIM',
  // repair round 7, item 4: a source-tree symlink whose stat() itself failed, and a directory
  // symlink CYCLE (still contained within src/, so distinct from an escape) -- each its own
  // accurate reason rather than folded into SOURCE_PATH_ESCAPES_ROOT.
  SOURCE_SYMLINK_UNREADABLE: 'SOURCE_SYMLINK_UNREADABLE',
  SOURCE_SYMLINK_CYCLE: 'SOURCE_SYMLINK_CYCLE',
  DUPLICATE_RUST_ROOT_ID: 'DUPLICATE_RUST_ROOT_ID',
}

const IMPLEMENTED_STATUSES = new Set([STATUS.IMPLEMENTED, STATUS.IMPLEMENTED_UNVERIFIED, STATUS.VERIFIED])
const VALID_SHARD_STATUSES = new Set(Object.values(STATUS))

export function classifyCapability({ targetModule, shardStatus, moduleIndex, docStatus, docStatusInvalid, docAvailable, cache = new SourceCache({ root: moduleIndex.crateAbsPath ?? moduleIndex.srcDir }) }) {
  const { matches, confidence } = matchCapabilityModule(targetModule, moduleIndex.modules)

  const realMatches = matches.filter((candidate) => classifyFileBody(cache.read(candidate.absPath)) === 'real')

  // Do not expose paths, stable per-module references, cardinality, or ordering: all allow
  // correlation with attacker-controlled target_module values. Only fixed-vocabulary states
  // needed to explain the verdict leave this process.
  const baseEvidence = { match_confidence: confidence }

  // More than one real (non-stub) file matching the same target_module is genuinely ambiguous
  // (which one governs reachability/test-evidence?), so it's reported explicitly rather than
  // silently picking whichever candidate happened to be first.
  if (realMatches.length > 1) {
    return {
      verdict: 'DISAGREE',
      reason: REASON.AMBIGUOUS_MODULE_MATCH,
      evidence: { ...baseEvidence, code_state: 'real', reachable: null },
    }
  }

  let codeState = 'absent'
  let bestModule = null
  if (realMatches.length === 1) {
    bestModule = realMatches[0]
    codeState = 'real'
  } else if (matches.length) {
    bestModule = matches[0]
    codeState = 'stub'
  }

  let reachable = null
  if (codeState === 'real') {
    reachable = moduleIndex.reachable.has(bestModule.absPath) && isModuleReferencedElsewhere(moduleIndex, bestModule, cache)
  }

  const evidence = { ...baseEvidence, code_state: codeState, reachable }

  // A status outside the declared enum is a data-integrity defect, never a silent AGREE: checked
  // before any status-dependent rule below so an out-of-enum value can't slip through a branch
  // that only tests for one specific known status. Always a full value-free redaction here --
  // shardStatus is by construction not a valid enum member, so there's no shape worth checking.
  if (!VALID_SHARD_STATUSES.has(shardStatus)) {
    return {
      verdict: 'DISAGREE',
      reason: REASON.SHARD_STATUS_VALUE_INVALID,
      evidence: { ...evidence, shard_status: redactValue(shardStatus) },
    }
  }

  if (codeState === 'real' && reachable === false) {
    return { verdict: 'DISAGREE', reason: REASON.CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT, evidence }
  }
  if (shardStatus === STATUS.UNIMPLEMENTED && codeState === 'real' && reachable === true) {
    return { verdict: 'DISAGREE', reason: REASON.SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL, evidence }
  }
  if (IMPLEMENTED_STATUSES.has(shardStatus) && codeState !== 'real') {
    return { verdict: 'DISAGREE', reason: REASON.SHARD_CLAIMS_IMPLEMENTED_BUT_NO_REAL_CODE_FOUND, evidence }
  }
  if (shardStatus === STATUS.VERIFIED && codeState === 'real' && !hasTestEvidence(moduleIndex, bestModule, cache)) {
    return { verdict: 'DISAGREE', reason: REASON.SHARD_CLAIMS_TESTED_BUT_NO_TEST_FILE_REFERENCES_MODULE, evidence }
  }

  // An unrecognized doc-table status cell is a doc-side data-integrity defect, symmetric with the
  // shard-status enum check above: never silently normalized into "unimplemented" and thus
  // treated as tautologically agreeing with an unimplemented shard status. Always a full
  // value-free redaction, same rationale as the shardStatus check above.
  if (docStatusInvalid != null) {
    return {
      verdict: 'DISAGREE',
      reason: REASON.DOC_STATUS_VALUE_INVALID,
      evidence: { ...evidence, doc_status_raw: redactValue(docStatusInvalid) },
    }
  }
  if (docStatus != null && docStatus !== shardStatus) {
    // Both are already-normalized, known-enum values here; sanitizeEnumField is still applied for
    // defense-in-depth (costs nothing, never changes a genuinely valid enum member).
    return {
      verdict: 'DISAGREE',
      reason: REASON.DOC_STATUS_CONTRADICTS_SHARD_STATUS,
      evidence: { ...evidence, doc_status: sanitizeEnumField(docStatus, VALID_SHARD_STATUSES), shard_status: sanitizeEnumField(shardStatus, VALID_SHARD_STATUSES) },
    }
  }
  // repair round 7, item 1: docStatus == null previously fell straight to AGREE whether the doc
  // genuinely had nothing to say OR a real read failure happened -- making "triple-confirmed" and
  // "docs never consulted" byte-identical. `unimplemented` + missing coverage stays AGREE (nothing
  // is claimed, so nothing for a doc to contradict), but any "claim" status with missing coverage
  // is a disclosed gap: doc consulted but silent on this key (MISSING_FOR_CLAIM) vs. doc
  // unavailable entirely (UNVERIFIABLE_FOR_CLAIM) -- never silently normalized into agreement.
  if (docStatus == null && shardStatus !== STATUS.UNIMPLEMENTED) {
    return {
      verdict: 'DISAGREE',
      reason: docAvailable ? REASON.DOC_STATUS_MISSING_FOR_CLAIM : REASON.DOC_STATUS_UNVERIFIABLE_FOR_CLAIM,
      evidence: { ...evidence, shard_status: sanitizeEnumField(shardStatus, VALID_SHARD_STATUSES), doc_available: Boolean(docAvailable) },
    }
  }
  return { verdict: 'AGREE', evidence }
}

// ── crate / repo orchestration ───────────────────────────────────────────────────────────────

function shardFindingsFor(crateName, shard) {
  const safeCrate = sanitizeCrateName(crateName)
  // repair round 7 (post-push audit): shard.path is computed by this tool itself, so it goes
  // through the same sanitizePathField guard every other call site already used -- this function
  // was the one place still emitting it raw.
  const safeShardPath = sanitizePathField(shard.path)
  const findings = []
  for (const e of shard.parseErrors) {
    findings.push({ crate: safeCrate, shard_path: safeShardPath, line: e.line, column: e.column, reason: REASON.SHARD_PARSE_ERROR })
  }
  for (const e of shard.recordErrors) {
    findings.push({ crate: safeCrate, shard_path: safeShardPath, line: e.line, detail: e.detail, reason: REASON.SHARD_RECORD_INVALID })
  }
  for (const d of shard.duplicateCapabilityKeys) {
    findings.push({
      crate: safeCrate,
      shard_path: safeShardPath,
      capability_key: sanitizeCapabilityKey(d.capability_key),
      lines: d.lines,
      reason: REASON.SHARD_CAPABILITY_KEY_DUPLICATE,
    })
  }
  return findings.sort((a, b) => (a.line ?? 0) - (b.line ?? 0) || comparePlain(String(a.capability_key ?? ''), String(b.capability_key ?? '')))
}

/** Operational findings mean the scan itself could not fully/safely run (vs. a DATA finding --
 * a DISAGREE or shard-content defect -- which is what --report makes non-blocking); never
 * suppressed by --report. `crateName` is sanitized here as the single choke point. */
function operationalFinding(crateName, reason, extra = {}) {
  return { crate: sanitizeCrateName(crateName), reason, ...extra }
}

export function verifyCrate(root, crateName, crates) {
  const safeCrateName = sanitizeCrateName(crateName)
  const crate = crates.find((c) => c.name === crateName)
  if (!crate) {
    return { crate: safeCrateName, applicable: false, reason: 'NOT_A_LIVE_WORKSPACE_CRATE', results: [], shard_findings: [], operational_findings: [] }
  }

  const shard = loadCrateShard(root, crate)
  if (!shard.exists) {
    return { crate: safeCrateName, applicable: false, reason: 'NO_SHARD_FOUND', shardPath: sanitizePathField(shard.path), results: [], shard_findings: [], operational_findings: [] }
  }
  // SHARD_UNREADABLE/SHARD_OVERSIZED are OPERATIONAL (the scan could not run at all), never a
  // shard_findings/DATA entry, so shard_finding_count keeps its documented meaning. `detail`
  // distinguishes a symlink/junction escape (ESCAPES_ROOT) from a genuine OS error.
  if (shard.unreadable) {
    const finding = { crate: safeCrateName, shard_path: sanitizePathField(shard.path), code: shard.unreadable.code, detail: shard.unreadable.reason ?? null, reason: 'SHARD_UNREADABLE' }
    return { crate: safeCrateName, applicable: false, reason: 'SHARD_UNREADABLE', shardPath: sanitizePathField(shard.path), results: [], shard_findings: [], operational_findings: [finding] }
  }
  if (shard.oversized) {
    const finding = { crate: safeCrateName, shard_path: sanitizePathField(shard.path), ...shard.oversized, reason: 'SHARD_OVERSIZED' }
    return { crate: safeCrateName, applicable: false, reason: 'SHARD_OVERSIZED', shardPath: sanitizePathField(shard.path), results: [], shard_findings: [], operational_findings: [finding] }
  }

  // Malformed lines, structurally invalid records, and duplicate capability keys never abort the
  // parse and are never silently dropped: they are surfaced here as explicit, deterministic
  // findings while every other, well-formed/unambiguous line is still classified normally.
  const shardFindings = shardFindingsFor(crateName, shard)

  // Rooted at the CRATE's own root, not just src/: hasTestEvidence must also read tests/ (Rust's
  // sibling integration-test dir), and this is the one shared cache whose oversized/unreadable
  // tracking already flows into operational_findings below, so widening its root keeps that
  // aggregation single instead of standing up an uncoordinated second cache.
  const cache = new SourceCache({ root: crate.abs_path })
  const moduleIndex = buildCrateModuleIndex(crate.abs_path, cache, crate.manifest_text)

  const rootEnumeration = moduleIndex.rootEnumeration ?? {}
  const rootFindings = [
    ...(rootEnumeration.duplicateRootIds ?? []).map(() => operationalFinding(crateName, REASON.DUPLICATE_RUST_ROOT_ID)),
    ...(rootEnumeration.unsafeEntries ?? []).map((entry) => operationalFinding(
      crateName,
      entry.reason === 'SYMLINK_CYCLE' ? REASON.SOURCE_SYMLINK_CYCLE : entry.reason === 'STAT_FAILED' ? REASON.SOURCE_SYMLINK_UNREADABLE : REASON.SOURCE_PATH_ESCAPES_ROOT,
      { path: sanitizePathField(slash(relative(root, entry.path))), ...(entry.code ? { code: entry.code } : {}) },
    )),
    ...(rootEnumeration.unreadableDirs ?? []).map((entry) => operationalFinding(crateName, REASON.DIR_ENUMERATION_UNREADABLE, {
      path: sanitizePathField(slash(relative(root, entry.path))), code: entry.code,
    })),
  ]
  if (rootEnumeration.truncated || rootFindings.length) {
    if (rootEnumeration.truncated) rootFindings.push(operationalFinding(crateName, REASON.SOURCE_FILES_TRUNCATED, { file_count_limit: BOUNDS.MAX_SOURCE_FILES_PER_CRATE }))
    return {
      crate: safeCrateName, applicable: false, reason: rootEnumeration.duplicateRootIds?.length ? 'DUPLICATE_RUST_ROOT_ID' : 'SOURCE_ROOT_ENUMERATION_FAILED',
      shardPath: sanitizePathField(shard.path), results: [], shard_findings: shardFindings,
      operational_findings: rootFindings,
    }
  }

  // opendir/readdir failing mid-walk is distinct from merely hitting the entry-count cap, so it
  // gets its own reason code, checked FIRST so a genuine I/O failure is never misreported as "just
  // hit the size limit"; either way the listing below is unprovably incomplete, refused exactly
  // like SOURCE_FILES_TRUNCATED below.
  if (moduleIndex.unreadableDirs.length > 0) {
    const findings = moduleIndex.unreadableDirs.map((d) =>
      operationalFinding(crateName, REASON.DIR_ENUMERATION_UNREADABLE, { path: sanitizePathField(slash(relative(root, d.path))), code: d.code }),
    )
    return {
      crate: safeCrateName,
      applicable: false,
      reason: 'DIR_ENUMERATION_UNREADABLE',
      shardPath: sanitizePathField(shard.path),
      results: [],
      shard_findings: shardFindings,
      operational_findings: findings,
    }
  }

  // A truncated source-file listing means the module index is INCOMPLETE: a capability's real
  // implementation file could have been excluded, silently turning a real disagreement into a
  // false AGREE. This crate's classification is refused entirely (fail-closed); shard-level
  // findings already computed above are still surfaced, since they don't depend on the listing.
  if (moduleIndex.sourceFilesTruncated) {
    const finding = operationalFinding(crateName, REASON.SOURCE_FILES_TRUNCATED, { file_count_limit: BOUNDS.MAX_SOURCE_FILES_PER_CRATE })
    return {
      crate: safeCrateName,
      applicable: false,
      reason: 'SOURCE_FILES_TRUNCATED',
      shardPath: sanitizePathField(shard.path),
      results: [],
      shard_findings: shardFindings,
      operational_findings: [finding],
    }
  }

  const doc = loadCrateDocCapabilityStatus(root, crateName)
  // repair round 7, item 1: true only when the crate doc was found AND successfully loaded
  // (docFinding is set on every failure path, even ones that also carry a non-null `path`) --
  // the one signal distinguishing "doc consulted, silent on this key" from "doc unavailable".
  const docAvailable = doc.path != null && doc.docFinding == null

  const results = shard.capabilities.map((capability) => {
    const docStatus = doc.statuses.get(capability.capability_key) ?? null
    const docStatusInvalid = doc.invalidStatuses.get(capability.capability_key) ?? null
    const verdict = classifyCapability({
      targetModule: capability.target_module,
      shardStatus: capability.status,
      moduleIndex,
      docStatus,
      docStatusInvalid,
      docAvailable,
      cache,
    })
    return {
      // capability_key has a real, this-tool-authoritative grammar (the dotted identifier the
      // doc-status matcher requires): a matching value is shown as-is, anything else is
      // value-free redacted via the fixed static token. target_module has NO such grammar (see
      // sanitizeTargetModule) -- always redacted without a correlation identifier.
      capability_key: sanitizeCapabilityKey(capability.capability_key),
      crate: safeCrateName,
      target_module: sanitizeTargetModule(capability.target_module),
      shard_status: sanitizeEnumField(capability.status, VALID_SHARD_STATUSES),
      doc_status: docStatus,
      ...verdict,
    }
  })

  // An oversized individual source file is a per-file operational finding: the file WAS listed
  // (the module index is complete), but its content could not be safely read, so any matching
  // capability was conservatively classified against empty content. Deduped/sorted for determinism.
  const oversizedSourceFindings = [...new Map(cache.oversizedFiles.map((f) => [f.absPath, f])).values()]
    .map((f) =>
      operationalFinding(crateName, REASON.SOURCE_FILE_OVERSIZED, {
        path: sanitizePathField(slash(relative(root, f.absPath))),
        size_bytes: f.size_bytes,
        limit_bytes: f.limit_bytes,
      }),
    )
    .sort((a, b) => comparePlain(a.path, b.path))

  // A source-tree read that failed for a reason OTHER than oversize (a stat/open/read error, or a
  // symlink whose real path escaped the crate's own source tree) is reported here, never folded
  // silently into an empty '' read with no trace.
  const unreadableSourceFindings = [...new Map(cache.unreadableFiles.map((f) => [f.absPath, f])).values()]
    .map((f) =>
      operationalFinding(crateName, f.reason === 'ESCAPES_ROOT' ? REASON.SOURCE_PATH_ESCAPES_ROOT : REASON.SOURCE_FILE_UNREADABLE, {
        path: sanitizePathField(slash(relative(root, f.absPath))),
        code: f.code,
      }),
    )
    .sort((a, b) => comparePlain(a.path, b.path))

  // A directory-walk entry whose symlink target escaped `srcDir`, whose target could not even be
  // resolved/stat-ed, or that formed a directory symlink CYCLE (still fully contained within
  // `srcDir`, so distinct from an escape) is reported here -- refused during ENUMERATION, before
  // any read was attempted. Each entry keeps its own accurate reason, never one blanket label.
  const symlinkReasonFor = (entryReason) => {
    if (entryReason === 'SYMLINK_CYCLE') return REASON.SOURCE_SYMLINK_CYCLE
    if (entryReason === 'ESCAPES_ROOT') return REASON.SOURCE_PATH_ESCAPES_ROOT
    return REASON.SOURCE_SYMLINK_UNREADABLE
  }
  const symlinkEscapeFindings = (moduleIndex.unsafeEntries ?? [])
    .map((e) => operationalFinding(crateName, symlinkReasonFor(e.reason), { path: sanitizePathField(slash(relative(root, e.path))), ...(e.code ? { code: e.code } : {}) }))
    .sort((a, b) => comparePlain(a.path, b.path))

  // A doc read failure means the DOC surface could not be safely consulted for this crate; every
  // capability is still classified against CODE+SHARD alone, but the failure itself must stay
  // visible rather than silently degrading to "no doc row found".
  const docFindings = []
  if (doc.docFinding) {
    const { reason: docReason, path: docFindingPath, ...docExtra } = doc.docFinding
    docFindings.push(operationalFinding(crateName, REASON[docReason], docFindingPath ? { ...docExtra, path: sanitizePathField(docFindingPath) } : docExtra))
  }

  // An unreadable tests/ directory only risks a MISSED test-evidence match (a false DISAGREE),
  // never a false AGREE, so unlike src/ this does not refuse the whole crate -- but it must still
  // be visible, not silently folded into "no evidence found".
  const testDirFindings = (moduleIndex.integrationTestsUnreadableDirs ?? [])
    .map((d) => operationalFinding(crateName, REASON.DIR_ENUMERATION_UNREADABLE, { path: sanitizePathField(slash(relative(root, d.path))), code: d.code }))
    .sort((a, b) => comparePlain(a.path, b.path))

  return {
    crate: safeCrateName,
    applicable: true,
    shardPath: sanitizePathField(shard.path),
    docPath: doc.path ? sanitizePathField(doc.path) : doc.path,
    results,
    shard_findings: shardFindings,
    operational_findings: [...oversizedSourceFindings, ...unreadableSourceFindings, ...symlinkEscapeFindings, ...docFindings, ...testDirFindings].sort((a, b) =>
      comparePlain(String(a.path ?? ''), String(b.path ?? '')),
    ),
    source_files_truncated: moduleIndex.sourceFilesTruncated,
  }
}

function sortedByCount(counts) {
  return Object.fromEntries(Object.entries(counts).sort(([a], [b]) => comparePlain(a, b)))
}

function sortFindings(findings) {
  return [...findings].sort(
    (a, b) =>
      comparePlain(String(a.crate ?? ''), String(b.crate ?? '')) ||
      comparePlain(String(a.reason ?? ''), String(b.reason ?? '')) ||
      comparePlain(String(a.path ?? a.member ?? ''), String(b.path ?? b.member ?? '')),
  )
}

export function verifyCrates(root, crateNames) {
  const { crates, escapedMembers, manifestFindings } = discoverWorkspaceCratesWithFindings(root)
  const perCrate = [...new Set(crateNames)].sort().map((name) => verifyCrate(root, name, crates))
  const flat = perCrate.flatMap((c) => c.results)
  const disagreements = flat.filter((r) => r.verdict === 'DISAGREE')
  const disagreeByReason = {}
  for (const d of disagreements) disagreeByReason[d.reason] = (disagreeByReason[d.reason] ?? 0) + 1

  const shardFindings = perCrate.flatMap((c) => c.shard_findings ?? [])
  // `member` is raw, PRE-verification path text straight out of a PR-controlled root Cargo.toml's
  // `members = [...]` array -- the highest-risk kind of "path" field -- so it is always fully
  // redacted, never a shape-allowlist passthrough like the POST-verification paths this tool
  // computes itself. An oversized/unreadable root or member Cargo.toml is reported the same way
  // (`member` is '(root)' for the workspace root manifest itself, or the raw member text).
  const workspaceFindings = escapedMembers.map((member) => ({
    member: redactValue(member),
    reason: REASON.WORKSPACE_MEMBER_PATH_ESCAPES_ROOT,
  }))
  const manifestFindingsSafe = (manifestFindings ?? []).map((f) => ({ ...f, member: f.member === '(root)' ? f.member : redactValue(f.member) }))
  // Every operational finding, repo-wide, none suppressed by --report (see
  // sync-anchor-v2-parser.mjs) -- they mean the scan itself could not fully/safely run, not "here
  // is a disagreement to note and move past".
  const operationalFindings = sortFindings([...workspaceFindings, ...manifestFindingsSafe, ...perCrate.flatMap((c) => c.operational_findings ?? [])])

  return {
    scope_note:
      'Mechanical Tier-1/2/3 agreement check only (code existence/reachability, shard status, crate-doc status). ' +
      'Does NOT check donor-parity or production-observed evidence, and AGREE never means "verified correct". ' +
      'A non-zero disagree_count is not the only signal of a problem: shard_finding_count reports ' +
      'shard-CONTENT structural defects (malformed JSON, invalid records, duplicate capability keys) ' +
      'that are report-gated exactly like a DISAGREE; operational_finding_count reports scan-level ' +
      'failures (workspace-path escape, oversized/unreadable/truncated input) that are NEVER ' +
      'suppressed by --report -- neither is ever folded into AGREE/DISAGREE. Output redacts ' +
      'module content, paths, equality, and ordering; fixed-vocabulary match/code/reachability ' +
      'states still disclose coarse classification correlation and are not a zero-leak proof.',
    crates_checked: perCrate.filter((c) => c.applicable).length,
    crates_skipped: perCrate.filter((c) => !c.applicable).map((c) => ({ crate: c.crate, reason: c.reason })),
    capabilities_checked: flat.length,
    agree_count: flat.length - disagreements.length,
    disagree_count: disagreements.length,
    disagree_by_reason: sortedByCount(disagreeByReason),
    disagreements,
    shard_finding_count: shardFindings.length,
    shard_findings: shardFindings,
    workspace_finding_count: workspaceFindings.length,
    workspace_findings: workspaceFindings,
    operational_finding_count: operationalFindings.length,
    operational_findings: operationalFindings,
    per_crate: perCrate,
  }
}
