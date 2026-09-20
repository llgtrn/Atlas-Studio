// island-lib.mjs — pure, deterministic logic for the reverse-Cargo-dependency /
// zero-caller "island" gate (issue #1524). Mechanizes the chronica-web-compiler
// precedent (docs/367-crate-chronica-web-compiler.md, issue #550, PR #588):
// a workspace crate with zero reverse Cargo dependents and no shipped binary
// entry point is an ISLAND per docs/design/2026-07-12-runtime-reachability-doctrine.md.
//
// Design note (validated, not copied from the source mission text): the mission
// bank text proposed walking `cargo metadata`'s *resolved* dependency graph
// (`resolve.nodes[].deps`). That requires full dependency resolution, which in
// this environment needs registry-index access and fails under `--offline` for
// crates not already vendored — a network dependency this gate must NOT have
// (see AGENTS.md security rules + issue anti-claims: "no network/provider
// dependency"). Manifest-declared dependencies from `cargo metadata --no-deps`
// are sufficient: every workspace-internal edge (normal/dev/build, optional,
// target-specific) is fully visible per-package without resolving the external
// registry graph at all, and it is still parsed structured JSON, not grep.
//
// All functions here are pure (no fs/process/Date access, except the sha256
// digest in computeIslandEvidenceFingerprint which is a deterministic pure
// function of its input, not real I/O) so they are exactly replayable in
// tests. I/O boundaries (spawning cargo, reading the allowlist file,
// wall-clock "today") live only in island-scan.mjs.
import { createHash } from 'node:crypto'

// 2026-09-16 hard refoundation: root crates (core, runtime, adapter, organism) are
// named for responsibility only, with no `chronica-` prefix (see AGENTS.md section 4,
// README.md "Repository responsibility"). The prefix is still accepted for any crate
// that legitimately carries it elsewhere in the workspace; only the plain
// lowercase-hyphenated identity is now required.
const CRATE_NAME_RE = /^(?:chronica-)?[a-z0-9]+(?:-[a-z0-9]+)*$/
const ISO_DATE_RE = /^\d{4}-\d{2}-\d{2}$/
const SHA256_HEX_RE = /^[0-9a-f]{64}$/
const MAX_GRACE_PERIOD_DAYS = 60

// Bump this whenever computeIslandEvidenceFingerprint()'s input shape or
// classification semantics change in a way that should invalidate every
// previously-granted allowlist fingerprint (forces re-review rather than a
// stale fingerprint accidentally still matching under new rules).
export const ISLAND_SCAN_POLICY_VERSION = 1

/** Deterministic JSON.stringify: recursively sorts object keys, 2-space indent. */
export function stableStringify(value) {
  return JSON.stringify(sortKeysDeep(value), null, 2) + '\n'
}

function sortKeysDeep(value) {
  if (Array.isArray(value)) return value.map(sortKeysDeep)
  if (value && typeof value === 'object') {
    const out = {}
    for (const key of Object.keys(value).sort()) out[key] = sortKeysDeep(value[key])
    return out
  }
  return value
}

function relativeManifestPath(manifestPath, workspaceRoot) {
  if (!workspaceRoot) return manifestPath
  const prefix = workspaceRoot.endsWith('/') ? workspaceRoot : workspaceRoot + '/'
  return manifestPath.startsWith(prefix) ? manifestPath.slice(prefix.length) : manifestPath
}

/**
 * Classify every workspace member from a parsed `cargo metadata --format-version 1
 * --no-deps` document. Pure function: same metadata object -> byte-identical result
 * (see stableStringify + island-scan.mjs's replay test).
 *
 * @param {object} metadata - parsed cargo metadata JSON (--no-deps).
 * @param {object} [opts]
 * @param {string[]} [opts.additionalEntrypoints] - crate names to treat as shipped
 *   entry points even without a `bin` Cargo target (e.g. a worker embedded in
 *   another binary). Must reference real workspace members; unknown names are
 *   reported as scan errors rather than silently ignored.
 */
export function classifyWorkspace(metadata, opts = {}) {
  const additionalEntrypoints = new Set(opts.additionalEntrypoints ?? [])
  const packages = metadata.packages ?? []
  const workspaceMemberIds = new Set(metadata.workspace_members ?? packages.map((p) => p.id))
  const members = packages.filter((p) => workspaceMemberIds.has(p.id))
  const byName = new Map(members.map((p) => [p.name, p]))
  const names = new Set(byName.keys())

  const errors = []
  for (const extra of additionalEntrypoints) {
    if (!names.has(extra)) errors.push(`UNKNOWN_ADDITIONAL_ENTRYPOINT: "${extra}" is not a current workspace member.`)
  }

  const crateKind = new Map()
  const isProcMacro = new Map()
  const isEntrypointByBinTarget = new Set()
  for (const pkg of members) {
    const targetKinds = new Set()
    let procMacro = false
    for (const target of pkg.targets ?? []) {
      for (const k of target.kind ?? []) targetKinds.add(k)
      if ((target.crate_types ?? []).includes('proc-macro')) procMacro = true
    }
    isProcMacro.set(pkg.name, procMacro)
    if (targetKinds.has('bin')) isEntrypointByBinTarget.add(pkg.name)

    let kind
    if (procMacro) kind = 'proc-macro'
    else if (targetKinds.has('bin') && targetKinds.has('lib')) kind = 'lib+bin'
    else if (targetKinds.has('bin')) kind = 'bin'
    else if (targetKinds.has('lib')) kind = 'lib'
    else kind = 'other' // e.g. a crate with only test/example/bench targets and no lib/bin
    crateKind.set(pkg.name, kind)
  }

  const entrypoints = new Set([...isEntrypointByBinTarget, ...additionalEntrypoints].filter((n) => names.has(n)))

  // Reverse-dependent edges, split by kind. Only workspace-internal targets count.
  // dependency.kind: null = normal (production), "dev" = dev-dependency (test/
  // example/bench only, per Cargo semantics — never linked into the shipped lib/bin
  // build), "build" = build-dependency (build.rs tooling only, not shipped runtime).
  const reverseProduction = new Map(members.map((p) => [p.name, []]))
  const reverseDevOnly = new Map(members.map((p) => [p.name, []]))
  const reverseBuildOnly = new Map(members.map((p) => [p.name, []]))
  // Forward graph of production (kind=null) edges only, used for entrypoint reachability.
  // Optional (feature-gated) edges are still production code paths (real callers, just
  // opt-in at build time) so they count for reachability; they are flagged separately
  // in evidence as feature_gated so the distinction is visible, not lost.
  const forwardProduction = new Map(members.map((p) => [p.name, []]))

  for (const pkg of members) {
    for (const dep of pkg.dependencies ?? []) {
      if (!names.has(dep.name)) continue // external crate, not workspace-internal
      const edge = { from: pkg.name, feature_gated: Boolean(dep.optional), target: dep.target ?? null }
      if (dep.kind == null) {
        reverseProduction.get(dep.name).push(edge)
        forwardProduction.get(pkg.name).push({ to: dep.name, feature_gated: Boolean(dep.optional), target: dep.target ?? null })
      } else if (dep.kind === 'dev') {
        reverseDevOnly.get(dep.name).push(edge)
      } else if (dep.kind === 'build') {
        reverseBuildOnly.get(dep.name).push(edge)
      }
    }
  }

  // BFS reachability from every entrypoint, following production (kind=null) edges
  // only. This is transitive: a crate depended on only by another crate that is
  // itself unreachable is correctly still an island, not a false LIVE.
  const reachable = new Set(entrypoints)
  const queue = [...entrypoints]
  while (queue.length) {
    const cur = queue.pop()
    for (const { to } of forwardProduction.get(cur) ?? []) {
      if (!reachable.has(to)) {
        reachable.add(to)
        queue.push(to)
      }
    }
  }

  const crates = []
  for (const name of [...names].sort()) {
    const pkg = byName.get(name)
    const production = sortEdges(reverseProduction.get(name))
    const devOnly = sortEdges(reverseDevOnly.get(name))
    const buildOnly = sortEdges(reverseBuildOnly.get(name))
    const isEntrypoint = entrypoints.has(name)
    const reachableFromEntrypoint = reachable.has(name)

    let status
    let reason
    const evidence = []
    if (isEntrypoint) {
      status = 'ENTRYPOINT'
      reason = 'Ships a Cargo `bin` target (or is a declared additional entrypoint): it is its own runtime caller.'
      evidence.push(`crate_kind=${crateKind.get(name)}`)
    } else if (reachableFromEntrypoint) {
      status = 'LIVE'
      const directCallers = production.map((e) => e.from)
      reason = `Reachable from a shipped entry point (${[...entrypoints].sort().join(', ')}) via ${production.length} direct production reverse-dependent(s): ${directCallers.join(', ')}.`
      for (const e of production) {
        evidence.push(`production reverse dependent: ${e.from}${e.feature_gated ? ' (feature-gated/optional)' : ''}${e.target ? ` (target: ${e.target})` : ''}`)
      }
    } else {
      status = 'ISLAND'
      const parts = []
      if (production.length === 0 && devOnly.length === 0 && buildOnly.length === 0) {
        parts.push('zero reverse Cargo dependents of any kind (production, dev, or build)')
      } else if (production.length === 0 && devOnly.length > 0) {
        // Generated from structured edge data (e.from vs. this crate's own
        // name), never hardcoded prose: "self-reference" is only ever said
        // when the dev-only caller genuinely IS this same crate. A dev-only
        // edge from a DIFFERENT crate (the common case — e.g. crate B's own
        // test suite dev-depending on crate A) must never be mislabeled as a
        // self-reference just because it's dev-only.
        const selfEdges = devOnly.filter((e) => e.from === name)
        const crossEdges = devOnly.filter((e) => e.from !== name)
        const bits = []
        if (crossEdges.length) {
          bits.push(`dev-only (test/bench/example) reverse dependent(s) from a different crate: ${crossEdges.map((e) => e.from).join(', ')}`)
        }
        if (selfEdges.length) {
          bits.push('a genuine dev-only self-reference from its own test/bench/example target')
        }
        parts.push(`zero production reverse dependents; only ${bits.join('; ')} — a dev-only edge is not a shipped production caller`)
      } else if (production.length === 0 && buildOnly.length > 0) {
        parts.push(`zero production reverse dependents; only build-tooling (build-dependency) reverse dependent(s): ${buildOnly.map((e) => e.from).join(', ')} — build-time tooling is not a shipped production caller`)
      } else {
        parts.push(`has ${production.length} production reverse dependent(s) but none is transitively reachable from a shipped entry point (${[...entrypoints].sort().join(', ') || 'none'})`)
      }
      parts.push('not reachable from any workspace binary entry point')
      reason = parts.join('; ') + '.'
      for (const e of devOnly) evidence.push(`test-only (dev-dependency) reverse dependent: ${e.from}`)
      for (const e of buildOnly) evidence.push(`build-only (build-dependency) reverse dependent: ${e.from}`)
      if (production.length === 0 && devOnly.length === 0 && buildOnly.length === 0) evidence.push('no reverse Cargo dependents found in workspace manifests')
    }

    crates.push({
      crate: name,
      version: pkg.version ?? null,
      manifest_path: relativeManifestPath(pkg.manifest_path, metadata.workspace_root),
      crate_kind: crateKind.get(name),
      is_entrypoint: isEntrypoint,
      reachable_from_entrypoint: reachableFromEntrypoint,
      reverse_dependents: { production, dev_only: devOnly, build_only: buildOnly },
      status,
      reason,
      evidence,
    })
  }

  const summary = {
    total: crates.length,
    entrypoint: crates.filter((c) => c.status === 'ENTRYPOINT').length,
    live: crates.filter((c) => c.status === 'LIVE').length,
    island: crates.filter((c) => c.status === 'ISLAND').length,
  }

  return {
    schema_version: 1,
    entrypoints: [...entrypoints].sort(),
    crates,
    summary,
    errors,
  }
}

function sortEdges(edges) {
  return [...edges].sort((a, b) => a.from.localeCompare(b.from))
}

// ── Canonical evidence fingerprint ──────────────────────────────────────────
//
// The allowlist's free-text "evidence" field is human-readable documentation
// ONLY — it is never authoritative and is never compared against reality (that
// was flagged in the PR #1565 admission review: prose evidence can drift
// silently). The `evidence_fingerprint` field is the actual machine-checked
// authority: a SHA-256 digest over a canonical, stable, structured snapshot of
// exactly the evidence that made a crate classify as ISLAND — crate identity,
// version, target kinds, and every sorted production/dev/build reverse edge
// (source crate, feature-gated flag, target cfg), plus the policy version.
// ANY change to that underlying evidence — a new caller, a renamed dev-only
// source crate, a feature/target edge changing, even a crate version bump —
// changes the fingerprint and invalidates the exception (applyAllowlistPolicy
// treats a mismatch as a stale, fail-closed exception, never a silent pass).
function fingerprintEdges(edges) {
  return [...edges]
    .map((e) => ({ from: e.from, feature_gated: Boolean(e.feature_gated), target: e.target ?? null }))
    .sort((a, b) => a.from.localeCompare(b.from) || (a.target ?? '').localeCompare(b.target ?? ''))
}

/**
 * Compute the canonical SHA-256 evidence fingerprint for one crate's current
 * classification, as produced by classifyWorkspace(). Pure function of its
 * input (crate scan entry + policy version) — no I/O, no wall clock.
 */
export function computeIslandEvidenceFingerprint(crateScanEntry, policyVersion = ISLAND_SCAN_POLICY_VERSION) {
  const canonical = {
    policy_version: policyVersion,
    crate: crateScanEntry.crate,
    version: crateScanEntry.version ?? null,
    manifest_path: crateScanEntry.manifest_path,
    crate_kind: crateScanEntry.crate_kind,
    is_entrypoint: crateScanEntry.is_entrypoint,
    reachable_from_entrypoint: crateScanEntry.reachable_from_entrypoint,
    production: fingerprintEdges(crateScanEntry.reverse_dependents.production),
    dev_only: fingerprintEdges(crateScanEntry.reverse_dependents.dev_only),
    build_only: fingerprintEdges(crateScanEntry.reverse_dependents.build_only),
  }
  return createHash('sha256').update(stableStringify(canonical)).digest('hex')
}

// ── Allowlist schema + validation ──────────────────────────────────────────
//
// Fail-closed, migration-safe: an exception must name the exact crate, real
// evidence, an accountable owner, a stated reason, a created date, and an
// expiry date bounded by MAX_GRACE_PERIOD_DAYS. No wildcards, no duplicates,
// no references to crates that do not currently exist, no unbounded/permanent
// exemptions. Any structural problem is a hard validation error: the caller
// (island-scan.mjs) must treat a non-empty error list as fail-closed (refuse
// to grant ANY exception from a malformed file, not just skip the bad rows) —
// a partially-malformed allowlist must not silently fall back to "everything
// blocked" being interpreted as "everything fine".

const REQUIRED_ENTRY_FIELDS = ['crate', 'reason', 'evidence', 'evidence_fingerprint', 'owner', 'created', 'expires', 'issue']

export function validateAllowlistDoc(doc, { knownCrates }) {
  const errors = []
  const known = new Set(knownCrates)

  if (doc == null || typeof doc !== 'object' || Array.isArray(doc)) {
    return { errors: ['ALLOWLIST_MALFORMED: root must be a JSON object.'], entries: [] }
  }
  if (doc.schema_version !== 1) {
    errors.push(`ALLOWLIST_SCHEMA_VERSION_MISMATCH: expected 1, got ${JSON.stringify(doc.schema_version)}.`)
  }
  if (!Array.isArray(doc.exceptions)) {
    errors.push('ALLOWLIST_MALFORMED: "exceptions" must be an array.')
    return { errors, entries: [] }
  }

  const seenCrates = new Set()
  const validEntries = []
  doc.exceptions.forEach((entry, idx) => {
    const tag = `exceptions[${idx}]`
    if (entry == null || typeof entry !== 'object' || Array.isArray(entry)) {
      errors.push(`${tag}: must be an object.`)
      return
    }
    for (const field of REQUIRED_ENTRY_FIELDS) {
      if (typeof entry[field] !== 'string' || entry[field].trim() === '') {
        errors.push(`${tag}: missing or empty required field "${field}".`)
      }
    }
    if (errors.some((e) => e.startsWith(tag))) return // don't cascade further checks on a structurally broken entry

    const crate = entry.crate
    if (crate.includes('*') || !CRATE_NAME_RE.test(crate)) {
      errors.push(`${tag}: "crate" must be an exact crate identity matching ${CRATE_NAME_RE}, not a wildcard/pattern (got "${crate}").`)
    }
    if (!known.has(crate)) {
      errors.push(`${tag}: unknown crate "${crate}" — it is not a current workspace member (stale allowlist entry; remove it).`)
    }
    if (seenCrates.has(crate)) {
      errors.push(`${tag}: duplicate exception for crate "${crate}".`)
    }
    seenCrates.add(crate)

    if (!SHA256_HEX_RE.test(entry.evidence_fingerprint)) {
      errors.push(`${tag}: "evidence_fingerprint" must be a 64-character lowercase hex SHA-256 digest (got "${entry.evidence_fingerprint}"). Compute it with \`node tools/build/island-scan.mjs --print-fingerprint ${crate}\`.`)
    }

    if (!ISO_DATE_RE.test(entry.created) || Number.isNaN(Date.parse(entry.created))) {
      errors.push(`${tag}: "created" must be an ISO 8601 date (YYYY-MM-DD), got "${entry.created}".`)
    }
    if (!ISO_DATE_RE.test(entry.expires) || Number.isNaN(Date.parse(entry.expires))) {
      errors.push(`${tag}: "expires" must be an ISO 8601 date (YYYY-MM-DD), got "${entry.expires}".`)
    }
    if (ISO_DATE_RE.test(entry.created) && ISO_DATE_RE.test(entry.expires)) {
      const created = Date.parse(entry.created)
      const expires = Date.parse(entry.expires)
      if (!(expires > created)) {
        errors.push(`${tag}: "expires" (${entry.expires}) must be strictly after "created" (${entry.created}).`)
      } else {
        const spanDays = (expires - created) / 86_400_000
        if (spanDays > MAX_GRACE_PERIOD_DAYS) {
          errors.push(`${tag}: grace period of ${spanDays} day(s) exceeds the bounded maximum of ${MAX_GRACE_PERIOD_DAYS} days — no permanent/long-lived exemptions.`)
        }
      }
    }
    if (!/^https:\/\/github\.com\/llgtrn\/Chronica\/issues\/\d+$/.test(entry.issue)) {
      errors.push(`${tag}: "issue" must be a real linked GitHub issue URL in this repo (got "${entry.issue}").`)
    }

    if (!errors.some((e) => e.startsWith(tag))) validEntries.push(entry)
  })

  return { errors, entries: validEntries }
}

/**
 * Fold a classifyWorkspace() scan + a validated allowlist into the final
 * pass/fail policy result. `today` is an ISO date string (YYYY-MM-DD),
 * injected by the caller (never Date.now() inside this pure function) so
 * this is independently replayable in tests.
 *
 * Every allowlist entry must be consumed exactly once against a crate that is
 * CURRENTLY `ISLAND` with a matching `evidence_fingerprint` and an unexpired
 * `expires` date. Any of the following fails the gate closed — never a silent
 * pass — and is reported as a `stale_exceptions` entry the allowlist author
 * must actively remove or regrant:
 *   - the crate is no longer ISLAND (promoted to LIVE, or became an
 *     ENTRYPOINT): the exception outlived the problem it was granted for;
 *   - the crate is still ISLAND but its evidence_fingerprint no longer
 *     matches current classification evidence (a caller edge was added/
 *     removed/renamed, a feature/target changed, etc. — the exception was
 *     granted for evidence that no longer describes reality);
 *   - the entry's `expires` date has passed.
 * An ISLAND crate with no allowlist entry at all is `BLOCKED` as before.
 */
export function applyAllowlistPolicy(scan, allowlistEntries, { today, policyVersion = ISLAND_SCAN_POLICY_VERSION }) {
  const byCrate = new Map(allowlistEntries.map((e) => [e.crate, e]))
  const blocked = []
  const grantedExceptions = []
  const staleExceptions = []

  const finalCrates = scan.crates.map((c) => {
    const entry = byCrate.get(c.crate)

    if (c.status !== 'ISLAND') {
      if (!entry) return { ...c, final_status: c.status, exception: null }
      // Evidence changed for the better (a real caller landed, or it became
      // an entry point) but the allowlist entry was never cleaned up. That
      // omission must not pass silently: it is stale housekeeping debt that
      // fails the gate until the entry is removed.
      staleExceptions.push({
        crate: c.crate,
        reason: 'NOW_LIVE_OR_ENTRYPOINT',
        detail: `crate is now ${c.status}, not ISLAND — remove its now-unnecessary allowlist entry from tools/build/island-scan-allowlist.json.`,
      })
      return { ...c, final_status: c.status, exception: { ...entry, stale: true, stale_reason: 'NOW_LIVE_OR_ENTRYPOINT' } }
    }

    // c.status === 'ISLAND'
    if (!entry) {
      blocked.push(c.crate)
      return { ...c, final_status: 'BLOCKED', exception: null }
    }

    const currentFingerprint = computeIslandEvidenceFingerprint(c, policyVersion)
    if (entry.evidence_fingerprint !== currentFingerprint) {
      staleExceptions.push({
        crate: c.crate,
        reason: 'FINGERPRINT_MISMATCH',
        detail: `allowlist evidence_fingerprint (${entry.evidence_fingerprint}) does not match current classification evidence (${currentFingerprint}) — the crate's edges/evidence changed since this exception was granted. Regrant with a fresh fingerprint (\`node tools/build/island-scan.mjs --print-fingerprint ${c.crate}\`) or remove the entry.`,
      })
      blocked.push(c.crate)
      return {
        ...c,
        final_status: 'BLOCKED',
        exception: { ...entry, stale: true, stale_reason: 'FINGERPRINT_MISMATCH', current_fingerprint: currentFingerprint },
      }
    }

    const expired = !(entry.expires > today)
    if (expired) {
      blocked.push(c.crate)
      return {
        ...c,
        final_status: 'BLOCKED',
        exception: { ...entry, expired: true },
      }
    }
    grantedExceptions.push(c.crate)
    return {
      ...c,
      final_status: 'ALLOWLISTED_GRACE_PERIOD',
      exception: { ...entry, expired: false },
    }
  })

  return {
    schema_version: 1,
    policy_version: policyVersion,
    today,
    crates: finalCrates,
    summary: {
      ...scan.summary,
      blocked: blocked.length,
      allowlisted_grace_period: grantedExceptions.length,
      stale_exceptions: staleExceptions.length,
    },
    blocked_crates: [...blocked].sort(),
    allowlisted_crates: [...grantedExceptions].sort(),
    stale_exceptions: [...staleExceptions].sort((a, b) => a.crate.localeCompare(b.crate)),
    pass: blocked.length === 0 && staleExceptions.length === 0,
  }
}
