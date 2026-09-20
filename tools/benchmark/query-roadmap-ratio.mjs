#!/usr/bin/env node
// query-roadmap-ratio.mjs — cross-references docs/doctrines/020-roadmap.md's hand-maintained claims (the 9
// named money-safety `architecture_invariant` rows in its table, and the crate-existence claims in
// its "Closed gaps"/"Where we actually are" prose) against real, git-tracked, cloud-safe evidence:
// docs/architecture-canonical/{evidence.jsonl,nodes.jsonl} and a live crates/ + root Cargo.toml scan.
// No docs/capabilities.db, no docs/architecture.db, no cargo invocation.
//
// This generalizes the pattern in tools/benchmark/query-benchmark-ratio.mjs (a doctrine doc names
// claims, a tool computes real status from tracked evidence) to a second doc family. Named
// query-roadmap-ratio.mjs rather than the originally-suggested query-roadmap-invariant-status.mjs
// because it verifies TWO claim families from the same doc (invariant status AND crate-existence),
// not just invariants -- see docs/benchmarks/127-operating-roadmap-invariant-truth-gate.md section 1 for why.
//
// ── What "verified" actually means here (read this before trusting a CONFIRMED) ──────────────────
// docs/architecture-canonical/evidence.jsonl carries a THIRD record type beyond the crate-shard
// vocabulary (architecture_node, architecture_evidence): `architecture_invariant`, a canonical
// registry row (key/name/description/enforcing_node/verification_command/status) exported straight
// from the hand-authored `INVARIANTS` array in tools/architecture/workspace-discovery.mjs. Its
// `status` field ("implemented"/"generated") is vocabulary-compatible with docs/doctrines/020-roadmap.md's
// table and is the primary comparison target below.
//
// Separately, evidence.jsonl also carries an `architecture_evidence` row per invariant
// (evidence_kind="architecture_invariant", evidence_ref="architecture_invariant:<key>",
// status="verified"). Read tools/architecture/capability-coverage.mjs's
// insertGeneratedArchitectureEvidence() before trusting that "verified": it is written
// UNCONDITIONALLY for every row in `architecture_invariant`, with no execution of the cited
// `verification_command`. All 9 rows in this repo currently share the exact same `verified_at`
// timestamp (one DB-build moment, see docs/architecture-canonical/meta.json's `generated_at`), which
// is the tell that this is one mechanical stamp, not 9 independent test runs. This tool reports that
// row's real fields (status/test_command/verified_at/verified_by/blocker_reason) because the task
// brief asks for them, but treats it as "structurally wired to an existing enforcing_node", never as
// "the cargo test passed" -- and says so in every report's `non_claims`.
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const ROOT = path.resolve(__dirname, '..', '..')

const ROADMAP_DOC = 'docs/doctrines/020-roadmap.md'

// ── crate-existence claim registry ──────────────────────────────────────────────────────────────
// docs/doctrines/020-roadmap.md's crate-existence claims live in unstructured prose (bulleted "Closed gaps"
// items, one "Where we actually are" bullet), not a parseable table like the invariants section, so
// unlike parseInvariantTable() below this cannot be live-regex-parsed without real risk of a false
// match. Following the same hand-curated-registry-plus-live-citation pattern
// tools/benchmark/query-benchmark-ratio.mjs already uses for its 11 platform families: each entry
// below carries an EXACT substring lifted from the current doc text. At run time this tool checks
// that substring is still literally present in docs/doctrines/020-roadmap.md before trusting the claim -- if a
// future doc edit removes/rewords the cited sentence, the entry is reported REGISTRY_CITATION_STALE
// rather than silently reused as if still current. This registry itself is therefore never the
// source of truth for "does the doc still make this claim" -- the live doc text is.
const CRATE_CLAIMS = [
  {
    crate: 'chronica-ai-workforce',
    claim: 'exists',
    citation: '**`chronica-ai-workforce`** — skill-vector worker routing exists',
  },
  {
    crate: 'chronica-web-runtime',
    claim: 'exists',
    citation: '`chronica-web-runtime` (`engine/`, `resolve.rs`) and `chronica-web-extract` exist.',
  },
  {
    crate: 'chronica-web-extract',
    claim: 'exists',
    citation: 'and `chronica-web-extract` exist.',
  },
  {
    crate: 'chronica-replication',
    claim: 'exists',
    citation: '**`chronica-replication`** — record-sync-adjacent replication crate exists',
  },
  {
    crate: 'chronica-injection-defense',
    claim: 'exists',
    citation: '**`chronica-injection-defense`** — deterministic prompt-injection defense primitive exists',
  },
  {
    crate: 'chronica-meta-plane',
    claim: 'exists',
    citation: '**`chronica-meta-plane`** — analysis dependency contract exists',
  },
  {
    crate: 'chronica-platform-plane',
    claim: 'exists',
    citation: '**`chronica-platform-plane`** — benchmark-platform replication contract exists',
  },
  {
    crate: 'chronica-pki',
    claim: 'exists',
    citation: '**`chronica-pki` now has a first landed slice**',
  },
  {
    crate: 'chronica-federation',
    claim: 'exists',
    citation: 'present under `chronica-federation/src/identity/`',
  },
  {
    crate: 'chronica-social',
    claim: 'exists',
    citation: 'present across `chronica-social/src/domain/`',
  },
  {
    crate: 'chronica-web-compiler',
    claim: 'deleted',
    citation:
      '`chronica-web-compiler` (`compile.rs`, `codegen.rs`, `wais.rs`, `schema_inference.rs`, `semantic_index.rs`, `vm_seam.rs`) and `chronica-web-types` existed as of this claim\'s original writing but were **DELETED**',
  },
  {
    crate: 'chronica-web-types',
    claim: 'deleted',
    citation:
      '`chronica-web-compiler` (`compile.rs`, `codegen.rs`, `wais.rs`, `schema_inference.rs`, `semantic_index.rs`, `vm_seam.rs`) and `chronica-web-types` existed as of this claim\'s original writing but were **DELETED**',
  },
]

function readRoadmapDoc(root) {
  const docPath = path.join(root, ROADMAP_DOC)
  if (!existsSync(docPath)) {
    throw new Error(`${ROADMAP_DOC} not found at ${docPath}`)
  }
  return readFileSync(docPath, 'utf8')
}

// Live-parses the "Money-safety invariants" markdown table out of docs/doctrines/020-roadmap.md itself (never
// a hand-copied registry) so a future edit to the table -- a new invariant, a changed status word --
// is picked up automatically, with no separate copy of the doc's own claims that could itself go
// stale. Cells cannot contain a literal "|" anywhere in this table (checked against the live text),
// so a plain split on "|" per row is safe and avoids a brittle single-shot regex across three cells.
export function parseInvariantTable(docText) {
  const lines = docText.split('\n')
  const startIndex = lines.findIndex((l) => l.startsWith('## Money-safety invariants'))
  if (startIndex === -1) {
    throw new Error(`${ROADMAP_DOC}: could not find "## Money-safety invariants" section heading`)
  }
  const endIndex = lines.findIndex((l, i) => i > startIndex && /^## /.test(l))
  const section = lines.slice(startIndex, endIndex === -1 ? lines.length : endIndex)

  const rows = []
  for (let i = 0; i < section.length; i += 1) {
    const line = section[i].trim()
    if (!line.startsWith('|')) continue
    const cells = line.split('|').map((c) => c.trim())
    // A well-formed "| a | b | c |" row split on "|" yields ['', a, b, c, '']. Any other shape is
    // either a header/separator row (skipped by the backtick check below) or a malformed table row
    // this tool refuses to silently coerce.
    if (cells.length !== 5) continue
    const keyCell = cells[1]
    const statusCell = cells[3]
    const keyMatch = /^`([a-z0-9_]+)`$/.exec(keyCell)
    if (!keyMatch) continue // header row ("Invariant") and separator row ("---") both fail this match
    rows.push({
      key: keyMatch[1],
      meaning: cells[2],
      doc_status: statusCell,
      line_number: startIndex + i + 1,
    })
  }
  return rows
}

function loadCanonicalJsonl(root, relPath) {
  const full = path.join(root, relPath)
  if (!existsSync(full)) return { path: relPath, exists: false, records: [] }
  const records = readFileSync(full, 'utf8')
    .split('\n')
    .filter((l) => l.trim().length > 0)
    .map((l) => JSON.parse(l))
  return { path: relPath, exists: true, records }
}

function loadArchitectureCanon(root) {
  const evidence = loadCanonicalJsonl(root, 'docs/architecture-canonical/evidence.jsonl')
  const nodes = loadCanonicalJsonl(root, 'docs/architecture-canonical/nodes.jsonl')

  const invariantsByKey = new Map()
  const invariantEvidenceByRef = new Map()
  for (const rec of evidence.records) {
    if (rec.record_type === 'architecture_invariant') {
      invariantsByKey.set(rec.key, rec)
    }
    if (rec.record_type === 'architecture_evidence' && rec.evidence_kind === 'architecture_invariant') {
      invariantEvidenceByRef.set(rec.evidence_ref, rec)
    }
  }
  const nodesById = new Map(nodes.records.filter((r) => r.record_type === 'architecture_node').map((n) => [n.id, n]))

  return { evidence, nodes, invariantsByKey, invariantEvidenceByRef, nodesById }
}

// Classifies one docs/doctrines/020-roadmap.md invariant table row against the canonical architecture_invariant
// registry row (vocabulary-compatible: implemented/generated) and the architecture_evidence
// verification-stamp row (see file header for what "verified" does and doesn't prove here).
function classifyInvariant(docRow, canon) {
  const registryRow = canon.invariantsByKey.get(docRow.key)
  const evidenceRef = `architecture_invariant:${docRow.key}`
  const evidenceRow = canon.invariantEvidenceByRef.get(evidenceRef)

  if (!registryRow) {
    return {
      key: docRow.key,
      doc_status: docRow.doc_status,
      doc_line: docRow.line_number,
      classification: 'UNVERIFIABLE',
      reason: `docs/doctrines/020-roadmap.md names "${docRow.key}" but no architecture_invariant record with key="${docRow.key}" exists in docs/architecture-canonical/evidence.jsonl -- this claim has no tracked evidence at all.`,
      registry: null,
      evidence: evidenceRow ?? null,
      enforcing_node: null,
    }
  }

  const enforcingNode = canon.nodesById.get(registryRow.enforcing_node) ?? null
  const diffs = []

  if (registryRow.status !== docRow.doc_status) {
    diffs.push(
      `docs/doctrines/020-roadmap.md:${docRow.line_number} says status="${docRow.doc_status}" but docs/architecture-canonical/evidence.jsonl's architecture_invariant record (key="${docRow.key}") says status="${registryRow.status}"`,
    )
  }
  if (!evidenceRow) {
    diffs.push(`no architecture_evidence row found for evidence_ref="${evidenceRef}" -- this invariant is registered but has no verification-stamp row at all`)
  } else {
    if (evidenceRow.blocker_reason) {
      diffs.push(`architecture_evidence row for "${evidenceRef}" carries a non-null blocker_reason: "${evidenceRow.blocker_reason}"`)
    }
    if (evidenceRow.architecture_node !== registryRow.enforcing_node) {
      diffs.push(
        `architecture_evidence.architecture_node="${evidenceRow.architecture_node}" does not match architecture_invariant.enforcing_node="${registryRow.enforcing_node}" for the same key -- the registry row and its own evidence row disagree about which node this invariant enforces`,
      )
    }
  }
  if (!enforcingNode) {
    diffs.push(`enforcing_node="${registryRow.enforcing_node}" (cited by the architecture_invariant registry row) does not exist in docs/architecture-canonical/nodes.jsonl`)
  }

  return {
    key: docRow.key,
    doc_status: docRow.doc_status,
    doc_line: docRow.line_number,
    classification: diffs.length > 0 ? 'DRIFTED' : 'CONFIRMED',
    diffs,
    registry: {
      status: registryRow.status,
      enforcing_node: registryRow.enforcing_node,
      verification_command: registryRow.verification_command,
    },
    evidence: evidenceRow
      ? {
          status: evidenceRow.status,
          test_command: evidenceRow.test_command,
          test_result: evidenceRow.test_result,
          verified_at: evidenceRow.verified_at,
          verified_by: evidenceRow.verified_by,
          blocker_reason: evidenceRow.blocker_reason,
        }
      : null,
    enforcing_node: enforcingNode ? { id: enforcingNode.id, kind: enforcingNode.kind, status: enforcingNode.status } : null,
  }
}

function parseWorkspaceMembers(cargoTomlText) {
  const match = cargoTomlText.match(/members\s*=\s*\[([\s\S]*?)\]/m)
  if (!match) return []
  return [...match[1].matchAll(/"([^"]+)"/g)].map((m) => m[1])
}

function parsePackageName(cargoTomlText) {
  return cargoTomlText.match(/^\s*name\s*=\s*"([^"]+)"/m)?.[1] ?? null
}

// Classifies one crate-existence claim: is the registry's citation still literally present in the
// live doc (else REGISTRY_CITATION_STALE, never silently trusted), and does a real crates/<name>
// directory + Cargo.toml + root-workspace-membership scan match the claim?
function classifyCrateClaim(claimEntry, docText, root) {
  if (!docText.includes(claimEntry.citation)) {
    return {
      crate: claimEntry.crate,
      claim: claimEntry.claim,
      classification: 'REGISTRY_CITATION_STALE',
      reason: `This tool's own registry entry for "${claimEntry.crate}" cites a sentence that no longer appears verbatim in ${ROADMAP_DOC}. Refusing to trust a possibly-outdated registry copy -- update tools/benchmark/query-roadmap-ratio.mjs's CRATE_CLAIMS citation for this crate against the current doc text.`,
      citation: claimEntry.citation,
    }
  }

  const crateDir = path.join(root, 'crates', claimEntry.crate)
  const cargoTomlPath = path.join(crateDir, 'Cargo.toml')
  const dirExists = existsSync(crateDir)
  const cargoTomlExists = dirExists && existsSync(cargoTomlPath)
  let packageName = null
  if (cargoTomlExists) {
    packageName = parsePackageName(readFileSync(cargoTomlPath, 'utf8'))
  }

  const rootCargoTomlPath = path.join(root, 'Cargo.toml')
  const workspaceMembers = existsSync(rootCargoTomlPath) ? parseWorkspaceMembers(readFileSync(rootCargoTomlPath, 'utf8')) : []
  const isWorkspaceMember = workspaceMembers.includes(`crates/${claimEntry.crate}`)

  const realState = {
    directory_exists: dirExists,
    cargo_toml_exists: cargoTomlExists,
    package_name: packageName,
    package_name_matches_dir: packageName === claimEntry.crate,
    is_workspace_member: isWorkspaceMember,
  }

  const diffs = []
  if (claimEntry.claim === 'exists') {
    if (!dirExists) diffs.push(`crates/${claimEntry.crate}/ does not exist`)
    else if (!cargoTomlExists) diffs.push(`crates/${claimEntry.crate}/Cargo.toml does not exist`)
    else if (!realState.package_name_matches_dir) diffs.push(`crates/${claimEntry.crate}/Cargo.toml declares package name "${packageName}", not "${claimEntry.crate}"`)
    if (!isWorkspaceMember) diffs.push(`"crates/${claimEntry.crate}" is not listed in root Cargo.toml's [workspace] members`)
  } else if (claimEntry.claim === 'deleted') {
    if (dirExists) diffs.push(`docs/doctrines/020-roadmap.md claims "${claimEntry.crate}" was DELETED, but crates/${claimEntry.crate}/ still exists on disk`)
    if (isWorkspaceMember) diffs.push(`docs/doctrines/020-roadmap.md claims "${claimEntry.crate}" was DELETED, but "crates/${claimEntry.crate}" is still listed in root Cargo.toml's [workspace] members`)
  }

  return {
    crate: claimEntry.crate,
    claim: claimEntry.claim,
    classification: diffs.length > 0 ? 'DRIFTED' : 'CONFIRMED',
    diffs,
    real_state: realState,
    citation: claimEntry.citation,
  }
}

export function computeRoadmapRatio({ root = ROOT } = {}) {
  const docText = readRoadmapDoc(root)
  const canon = loadArchitectureCanon(root)
  const docInvariantRows = parseInvariantTable(docText)

  const invariants = docInvariantRows.map((row) => classifyInvariant(row, canon))
  const crateClaims = CRATE_CLAIMS.map((entry) => classifyCrateClaim(entry, docText, root))

  function countBy(list, field, value) {
    return list.filter((x) => x[field] === value).length
  }

  return {
    schema_version: 1,
    generated_at: null, // stamped by main()/caller
    roadmap_doc: ROADMAP_DOC,
    truth_label: 'LOCAL_AUDIT_REQUIRED',
    authority:
      'This tool reads docs/doctrines/020-roadmap.md, docs/architecture-canonical/{evidence.jsonl,nodes.jsonl}, ' +
      'crates/*/Cargo.toml, and the root Cargo.toml only. It never reads or requires docs/capabilities.db ' +
      'or docs/architecture.db, and it never invokes `cargo test` -- see architecture_invariant_evidence_caveat below.',
    invariants: {
      total: invariants.length,
      confirmed: countBy(invariants, 'classification', 'CONFIRMED'),
      drifted: countBy(invariants, 'classification', 'DRIFTED'),
      unverifiable: countBy(invariants, 'classification', 'UNVERIFIABLE'),
      rows: invariants,
    },
    crate_claims: {
      total: crateClaims.length,
      confirmed: countBy(crateClaims, 'classification', 'CONFIRMED'),
      drifted: countBy(crateClaims, 'classification', 'DRIFTED'),
      registry_citation_stale: countBy(crateClaims, 'classification', 'REGISTRY_CITATION_STALE'),
      rows: crateClaims,
    },
    architecture_invariant_evidence_caveat:
      'Every architecture_evidence row with evidence_kind="architecture_invariant" in this repo has ' +
      'status="verified" and shares the exact same verified_at timestamp -- that is tools/architecture/' +
      'capability-coverage.mjs\'s insertGeneratedArchitectureEvidence() writing one unconditional stamp per ' +
      'invariant at DB-build time (verified_by="tools/architecture/verify.mjs"), NOT proof that the cited ' +
      'test_command (a real `cargo test` invocation) was ever actually run and passed. This tool never runs ' +
      'cargo itself. A CONFIRMED invariant below means "docs/doctrines/020-roadmap.md\'s claimed status matches the ' +
      'canonical architecture_invariant registry, and that registry\'s enforcing_node exists and carries a ' +
      'structurally-linked evidence row" -- it does NOT mean "the cargo test was executed this session."',
    non_claims: [
      'This tool does not read or require docs/capabilities.db or docs/architecture.db.',
      'This tool never invokes cargo; no test_command cited below was executed by this tool.',
      'A CONFIRMED crate-existence claim means the cited doc sentence is still present verbatim AND the real crates/ + Cargo.toml scan matches it -- REGISTRY_CITATION_STALE means this tool\'s own registry entry could not be matched against the live doc and was NOT silently trusted.',
      'architecture_evidence.status="verified" for evidence_kind="architecture_invariant" rows is a mechanical build-time stamp, never a cargo-test-passed proof -- see architecture_invariant_evidence_caveat.',
      'Routing a finding into tools/capabilities/opportunities.json only happens with an explicit --route-findings flag, never by default, and only ADDS a new entry for a finding not already tracked (matched by a stable id) -- it never edits or removes an existing opportunity, and never promotes/closes one just because a re-run now finds it CONFIRMED.',
    ],
  }
}

// ── benchmark-finding -> opportunity routing (--route-findings, opt-in, off by default) ────────────
// tools/capabilities/opportunities.json is the REAL, already-consumed strategic backlog (129 active
// ranked items feed docs/_generated/027-opportunities.md; Lane A-U reads it via `pnpm caps:track opp
// list`). Read tools/capabilities/reconcile-opportunities.mjs before touching this: it validates
// category against a fixed enum, status against a fixed enum, clamps every score to 1..5, and is the
// ONLY thing allowed to apply opportunities.json into the local capabilities.db cache. This routing
// code reuses that exact schema and, for the real repo, that exact reconcile script -- it does not
// invent a parallel intake format. It ONLY writes a NEW entry for a finding not already tracked
// (idempotent by id) and NEVER edits or removes an existing opportunity, matching the same
// no-blind-bulk-edit posture find-verification-candidates.mjs and record-verified.mjs already hold.
const OPPORTUNITY_CATEGORY = 'architecture'
const OPPORTUNITY_STATUS = 'discovered'

function todayIso() {
  return new Date().toISOString().slice(0, 10)
}

export function buildOpportunityForInvariantFinding(row) {
  const isUnverifiable = row.classification === 'UNVERIFIABLE'
  const id = `roadmap-invariant-${isUnverifiable ? 'unverifiable' : 'drift'}-${row.key}`
  const diffText = isUnverifiable ? row.reason : row.diffs.join('; ')
  return {
    id,
    title: `Roadmap invariant ${isUnverifiable ? 'has no tracked evidence' : 'drifted from tracked evidence'}: ${row.key}`,
    description: `tools/benchmark/query-roadmap-ratio.mjs found docs/doctrines/020-roadmap.md's "${row.key}" money-safety invariant row ${isUnverifiable ? 'UNVERIFIABLE' : 'DRIFTED'} against docs/architecture-canonical/evidence.jsonl. ${diffText}`,
    category: OPPORTUNITY_CATEGORY,
    impact: isUnverifiable ? 3 : 4,
    reach: 2,
    leverage: 3,
    effort: 1,
    confidence: 5,
    source_repo: 'native',
    related_capabilities: '',
    related_architecture_nodes: row.registry?.enforcing_node ?? '',
    status: OPPORTUNITY_STATUS,
    created_at: todayIso(),
    notes: `Auto-routed by \`pnpm roadmap:ratio -- --route-findings\` (tools/benchmark/query-roadmap-ratio.mjs). Re-run \`pnpm roadmap:ratio\` to confirm this is still current before acting.`,
  }
}

export function buildOpportunityForCrateClaimFinding(row) {
  const isStale = row.classification === 'REGISTRY_CITATION_STALE'
  const id = `roadmap-crate-claim-${isStale ? 'stale-citation' : 'drift'}-${row.crate}`
  const diffText = isStale ? row.reason : row.diffs.join('; ')
  return {
    id,
    title: `Roadmap crate-existence claim ${isStale ? 'citation is stale' : 'drifted from disk'}: ${row.crate}`,
    description: `tools/benchmark/query-roadmap-ratio.mjs found docs/doctrines/020-roadmap.md's "${row.crate}" (claim="${row.claim}") ${isStale ? 'REGISTRY_CITATION_STALE' : 'DRIFTED'} against a real crates/ + Cargo.toml scan. ${diffText}`,
    category: OPPORTUNITY_CATEGORY,
    impact: isStale ? 2 : 3,
    reach: 1,
    leverage: 2,
    effort: 1,
    confidence: 5,
    source_repo: 'native',
    related_capabilities: '',
    related_architecture_nodes: '',
    status: OPPORTUNITY_STATUS,
    created_at: todayIso(),
    notes: `Auto-routed by \`pnpm roadmap:ratio -- --route-findings\` (tools/benchmark/query-roadmap-ratio.mjs). Re-run \`pnpm roadmap:ratio\` to confirm this is still current before acting.`,
  }
}

function findingsToRoute(result) {
  const entries = []
  for (const row of result.invariants.rows) {
    if (row.classification === 'DRIFTED' || row.classification === 'UNVERIFIABLE') {
      entries.push(buildOpportunityForInvariantFinding(row))
    }
  }
  for (const row of result.crate_claims.rows) {
    if (row.classification === 'DRIFTED' || row.classification === 'REGISTRY_CITATION_STALE') {
      entries.push(buildOpportunityForCrateClaimFinding(row))
    }
  }
  return entries
}

// Writes new (never edits/removes existing) opportunity entries into tools/capabilities/
// opportunities.json using the exact schema reconcile-opportunities.mjs validates, then -- for a real
// repo checkout that actually has reconcile-opportunities.mjs on disk -- runs that REAL script
// (never a reimplementation) so the local capabilities.db cache picks up the new entries exactly like
// a human running `pnpm caps:track opp add` would. In a fixture/test root without that script present,
// this step is skipped and reported as such, never faked.
export function routeFindingsToOpportunities({ result, root = ROOT, applyReconcile = true } = {}) {
  const oppPath = path.join(root, 'tools', 'capabilities', 'opportunities.json')
  if (!existsSync(oppPath)) {
    return { attempted: false, reason: `${path.relative(root, oppPath)} not found`, added: [], skipped_existing: [] }
  }
  const opp = JSON.parse(readFileSync(oppPath, 'utf8'))
  if (!Array.isArray(opp.opportunities)) {
    return { attempted: false, reason: `${path.relative(root, oppPath)} has no "opportunities" array`, added: [], skipped_existing: [] }
  }
  const existingIds = new Set(opp.opportunities.map((o) => o.id))
  const candidates = findingsToRoute(result)

  const added = []
  const skipped = []
  for (const entry of candidates) {
    if (existingIds.has(entry.id)) {
      skipped.push(entry.id)
      continue
    }
    opp.opportunities.push(entry)
    existingIds.add(entry.id)
    added.push(entry.id)
  }

  if (added.length > 0) {
    writeFileSync(oppPath, JSON.stringify(opp, null, 2) + '\n')
  }

  let dbReconcile = { attempted: false, reason: added.length === 0 ? 'nothing new to reconcile' : 'applyReconcile=false' }
  if (applyReconcile && added.length > 0) {
    const reconcileScript = path.join(root, 'tools', 'capabilities', 'reconcile-opportunities.mjs')
    if (existsSync(reconcileScript)) {
      const proc = spawnSync(process.execPath, [reconcileScript], { cwd: root, encoding: 'utf8' })
      dbReconcile = { attempted: true, ok: proc.status === 0, exit_code: proc.status, stdout: proc.stdout, stderr: proc.stderr }
    } else {
      dbReconcile = { attempted: false, reason: `${path.relative(root, reconcileScript)} not found under root` }
    }
  }

  return {
    attempted: true,
    opportunities_json: path.relative(root, oppPath),
    added,
    skipped_existing: skipped,
    db_reconcile: dbReconcile,
  }
}

function pct(count, total) {
  if (total === 0) return 'n/a'
  return `${((count / total) * 100).toFixed(1)}%`
}

function printSummary(result) {
  console.log(`roadmap-ratio: ${result.roadmap_doc}`)
  const inv = result.invariants
  console.log(`  invariants: ${inv.total} total`)
  console.log(`    CONFIRMED: ${inv.confirmed}/${inv.total} (${pct(inv.confirmed, inv.total)})`)
  console.log(`    DRIFTED: ${inv.drifted}/${inv.total} (${pct(inv.drifted, inv.total)})`)
  console.log(`    UNVERIFIABLE: ${inv.unverifiable}/${inv.total} (${pct(inv.unverifiable, inv.total)})`)
  for (const row of inv.rows) {
    if (row.classification !== 'CONFIRMED') {
      console.log(`    ! ${row.key}: ${row.classification}`)
      for (const d of row.diffs ?? [row.reason]) console.log(`      - ${d}`)
    }
  }
  const cc = result.crate_claims
  console.log(`  crate_claims: ${cc.total} total`)
  console.log(`    CONFIRMED: ${cc.confirmed}/${cc.total} (${pct(cc.confirmed, cc.total)})`)
  console.log(`    DRIFTED: ${cc.drifted}/${cc.total} (${pct(cc.drifted, cc.total)})`)
  console.log(`    REGISTRY_CITATION_STALE: ${cc.registry_citation_stale}/${cc.total} (${pct(cc.registry_citation_stale, cc.total)})`)
  for (const row of cc.rows) {
    if (row.classification !== 'CONFIRMED') {
      console.log(`    ! ${row.crate} (${row.claim}): ${row.classification}`)
      for (const d of row.diffs ?? [row.reason]) console.log(`      - ${d}`)
    }
  }
  console.log(`  CAVEAT: ${result.architecture_invariant_evidence_caveat}`)
}

function usage() {
  return 'Usage: node tools/benchmark/query-roadmap-ratio.mjs [summary|full] [--root <path>] [--route-findings]'
}

function printRouting(routing) {
  if (!routing.attempted) {
    console.log(`  route-findings: skipped (${routing.reason})`)
    return
  }
  console.log(`  route-findings: added ${routing.added.length} new opportunity(ies) to ${routing.opportunities_json}${routing.skipped_existing.length ? `, ${routing.skipped_existing.length} already tracked` : ''}`)
  for (const id of routing.added) console.log(`    + ${id}`)
  if (routing.db_reconcile.attempted) {
    console.log(`    reconcile-opportunities.mjs: ${routing.db_reconcile.ok ? 'OK' : 'FAILED'}`)
  } else if (routing.added.length > 0) {
    console.log(`    reconcile-opportunities.mjs: skipped (${routing.db_reconcile.reason})`)
  }
}

function main() {
  const argv = process.argv.slice(2)
  const cmd = argv.find((a) => !a.startsWith('--')) || 'summary'
  const rootFlagIndex = argv.indexOf('--root')
  const root = rootFlagIndex >= 0 ? path.resolve(argv[rootFlagIndex + 1]) : ROOT
  const shouldRoute = argv.includes('--route-findings')

  let result
  try {
    result = computeRoadmapRatio({ root })
  } catch (error) {
    console.error(`query-roadmap-ratio failed: ${error.message}`)
    process.exitCode = 1
    return
  }
  result.generated_at = new Date().toISOString()

  if (shouldRoute) {
    result.routing = routeFindingsToOpportunities({ result, root })
  }

  if (cmd === 'summary') {
    printSummary(result)
    if (shouldRoute) printRouting(result.routing)
  } else if (cmd === 'full') {
    console.log(JSON.stringify(result, null, 2))
  } else {
    console.error(usage())
    process.exitCode = 2
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
