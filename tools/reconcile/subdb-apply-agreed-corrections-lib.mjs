// subdb-apply-agreed-corrections-lib.mjs -- the "safe to self-correct" half of the conflict-audit
// writer (docs/doctrines/023-five-dimension-cloud-shard-contract.md, marked NOT_BUILT there).
//
// Doc 023 section 4 makes a crate's own `.chronica/sub-cap-arch.jsonl` shard cloud-writable
// evidence, never root truth. When `tools/reconcile/sync-anchor-v2-lib.mjs` mechanically proves a
// capability's code is real and reachable (EXACT module-match confidence only -- PREFIX-confidence
// matches are a fuzzy heuristic and are never auto-applied, see doc 023 sec 6's honesty note on the
// parser's unmeasured false-positive rate), a shard row still claiming `status: "unimplemented"` for
// that capability is simply wrong at the crate-local level. This module appends a hash-chained
// `capability_status_correction` ledger record fixing that -- it never rewrites or deletes the
// original `capability` row (the shard is a graph-chain ledger, not a mutable table). Only the
// SHARD dimension is touched; root `docs/capabilities.db`'s "Verified" ratio is untouched and stays
// the separate local-audit promotion process's job (doc 023 sec 3/4).
import { appendFileSync, existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { discoverWorkspaceCrates, REASON } from './sync-anchor-v2-lib.mjs'
import { hashRecord, SUBDB_GENESIS_HASH, verifyCrateSubdbs, writeSqliteDb } from '../subdb/subdb-lib.mjs'

export const CORRECTION_RECORD_TYPE = 'capability_status_correction'
export const CORRECTABLE_REASON = REASON.SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL

export const SKIP_REASON = {
  NOT_A_LIVE_WORKSPACE_CRATE: 'NOT_A_LIVE_WORKSPACE_CRATE',
  SHARD_FILE_MISSING: 'SHARD_FILE_MISSING',
  PRE_EXISTING_HASH_CHAIN_BROKEN: 'PRE_EXISTING_HASH_CHAIN_BROKEN',
  CAPABILITY_KEY_NOT_FOUND_IN_CURRENT_SHARD: 'CAPABILITY_KEY_NOT_FOUND_IN_CURRENT_SHARD',
  ALREADY_NOT_UNIMPLEMENTED_IN_CURRENT_SHARD: 'ALREADY_NOT_UNIMPLEMENTED_IN_CURRENT_SHARD',
}

/** Only EXACT-confidence findings are mechanically safe to self-correct. PREFIX-confidence hits
 * (fuzzy last-segment matching, see sync-anchor-v2-lib.mjs's matchCapabilityModule) are deferred to
 * the Category-B human/local-audit review queue instead -- never auto-applied. */
export function selectEligibleFindings(findings) {
  return findings.filter(
    (f) => f.verdict === 'DISAGREE' && f.reason === CORRECTABLE_REASON && f.evidence?.match_confidence === 'exact',
  )
}

export function selectDeferredToPrefixFindings(findings) {
  return findings.filter(
    (f) => f.verdict === 'DISAGREE' && f.reason === CORRECTABLE_REASON && f.evidence?.match_confidence !== 'exact',
  )
}

function readRawShardRows(shardPath) {
  return readFileSync(shardPath, 'utf8')
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => JSON.parse(line))
}

/** Recomputes the sequence/prev_hash/record_hash chain over rows already on disk, using the exact
 * hashing convention `tools/subdb/gen-crate-subdb.mjs` uses (imported, not reimplemented). This is a
 * pre-flight safety check -- appending onto an already-broken chain would just extend the damage. */
function existingChainIsValid(rows) {
  let previous = SUBDB_GENESIS_HASH
  for (let i = 0; i < rows.length; i += 1) {
    const row = rows[i]
    if (row.sequence !== i) return false
    if (row.prev_hash !== previous) return false
    const { record_hash, ...withoutHash } = row
    if (record_hash !== hashRecord(withoutHash)) return false
    previous = record_hash
  }
  return rows.length > 0 && rows[0].record_type === 'meta'
}

function buildCorrectionRecord({ capabilityKey, crateName, finding, sequence, prevHash, appliedAt }) {
  const withChain = {
    sequence,
    prev_hash: prevHash,
    record_id: `${CORRECTION_RECORD_TYPE}:${capabilityKey}:${sequence}`,
    record_type: CORRECTION_RECORD_TYPE,
    capability_key: capabilityKey,
    crate: crateName,
    previous_status: 'unimplemented',
    new_status: 'implemented',
    reason_code: finding.reason,
    match_confidence: finding.evidence.match_confidence,
    target_module: finding.target_module,
    matched_modules: finding.evidence.matched_modules,
    code_state: finding.evidence.code_state,
    reachable: finding.evidence.reachable,
    corrected_by: 'tools/reconcile/subdb-apply-agreed-corrections.mjs',
    applied_at: appliedAt,
    scope_note:
      'Crate-local shard correction only (doc 023 sec 4/7). Does NOT promote to docs/capabilities.db ' +
      'root Verified ratio -- that remains LOCAL_AUDIT_REQUIRED and is a separate local-audit process.',
  }
  const record_hash = hashRecord(withChain)
  return { ...withChain, record_hash }
}

/**
 * Applies every eligible (EXACT-confidence) finding for one crate: appends one hash-chained
 * `capability_status_correction` record per capability_key, never touching existing lines, then
 * rewrites the crate's SQLite cache (`sub-cap-arch.db`) from the full updated row set so the two
 * committed files stay in sync (doc 023 sec 4: "both files are committed by subdb:gen").
 */
export function applyCorrectionsForCrate({ root, crateName, cratePath, findings, now = () => new Date().toISOString() }) {
  const shardPath = join(root, cratePath, '.chronica', 'sub-cap-arch.jsonl')
  const sqlitePath = join(root, cratePath, '.chronica', 'sub-cap-arch.db')

  if (!existsSync(shardPath)) {
    return {
      crate: crateName,
      applied: [],
      skipped: findings.map((f) => ({ capability_key: f.capability_key, reason: SKIP_REASON.SHARD_FILE_MISSING })),
    }
  }

  const rows = readRawShardRows(shardPath)
  if (!existingChainIsValid(rows)) {
    return {
      crate: crateName,
      shardPath,
      applied: [],
      skipped: findings.map((f) => ({
        capability_key: f.capability_key,
        reason: SKIP_REASON.PRE_EXISTING_HASH_CHAIN_BROKEN,
      })),
    }
  }

  const baseStatus = new Map()
  const effectiveStatus = new Map()
  for (const row of rows) {
    if (row.record_type === 'capability') {
      baseStatus.set(row.capability_key, row.status)
      effectiveStatus.set(row.capability_key, row.status)
    } else if (row.record_type === CORRECTION_RECORD_TYPE) {
      effectiveStatus.set(row.capability_key, row.new_status)
    }
  }

  let sequence = rows.length
  let prevHash = rows[rows.length - 1].record_hash
  const appended = []
  const applied = []
  const skipped = []

  for (const finding of findings) {
    const key = finding.capability_key
    if (!baseStatus.has(key)) {
      skipped.push({ capability_key: key, reason: SKIP_REASON.CAPABILITY_KEY_NOT_FOUND_IN_CURRENT_SHARD })
      continue
    }
    if (effectiveStatus.get(key) !== 'unimplemented') {
      skipped.push({ capability_key: key, reason: SKIP_REASON.ALREADY_NOT_UNIMPLEMENTED_IN_CURRENT_SHARD })
      continue
    }
    const record = buildCorrectionRecord({
      capabilityKey: key,
      crateName,
      finding,
      sequence,
      prevHash,
      appliedAt: now(),
    })
    appended.push(record)
    effectiveStatus.set(key, 'implemented')
    prevHash = record.record_hash
    sequence += 1
    applied.push({ capability_key: key, target_module: finding.target_module })
  }

  if (appended.length) {
    appendFileSync(shardPath, `${appended.map((r) => JSON.stringify(r)).join('\n')}\n`)
    writeSqliteDb(sqlitePath, [...rows, ...appended])
  }

  return { crate: crateName, shardPath, applied, skipped, appendedCount: appended.length }
}

/**
 * Top-level entry point: given the flat findings list from `sync-anchor-v2-lib.mjs`'s
 * `verifyCrates(...).disagreements`, applies every EXACT-confidence
 * SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL finding, crate by crate, and (when `verify` is true,
 * the default) runs the same check `pnpm subdb:verify -- --crate <name>` runs for every crate that
 * was actually written to, confirming the hash chain is still internally consistent.
 */
export function applyAgreedCorrections({ root, findings, verify = true, now }) {
  const eligible = selectEligibleFindings(findings)
  const deferredToPrefix = selectDeferredToPrefixFindings(findings)

  const byCrate = new Map()
  for (const finding of eligible) {
    if (!byCrate.has(finding.crate)) byCrate.set(finding.crate, [])
    byCrate.get(finding.crate).push(finding)
  }

  const workspaceCrates = discoverWorkspaceCrates(root)
  const perCrate = []
  for (const [crateName, crateFindings] of byCrate) {
    const crate = workspaceCrates.find((c) => c.name === crateName)
    if (!crate) {
      perCrate.push({
        crate: crateName,
        applied: [],
        skipped: crateFindings.map((f) => ({
          capability_key: f.capability_key,
          reason: SKIP_REASON.NOT_A_LIVE_WORKSPACE_CRATE,
        })),
      })
      continue
    }
    const result = applyCorrectionsForCrate({
      root,
      crateName,
      cratePath: crate.crate_path,
      findings: crateFindings,
      ...(now ? { now } : {}),
    })
    if (verify && result.appendedCount) {
      result.verify = verifyCrateSubdbs({ root, crates: [crateName] })
    }
    perCrate.push(result)
  }

  const correctedCount = perCrate.reduce((n, r) => n + r.applied.length, 0)
  const skippedCount = perCrate.reduce((n, r) => n + r.skipped.length, 0)
  const cratesWithFailedVerify = perCrate.filter((r) => r.verify && r.verify.ok === false)

  return {
    exact_confidence_findings: eligible.length,
    corrected_count: correctedCount,
    skipped_count: skippedCount,
    deferred_to_prefix_count: deferredToPrefix.length,
    crates_touched: perCrate.filter((r) => r.applied.length > 0).length,
    verify_failures: cratesWithFailedVerify.map((r) => ({ crate: r.crate, errors: r.verify.errors })),
    per_crate: perCrate,
  }
}
