#!/usr/bin/env node
// query-crate-ratio.mjs — computes a per-crate capability/architecture coverage ratio by reading
// ONLY that crate's own committed crates/<crate>/.chronica/sub-cap-arch.jsonl shard.
//
// Why this exists: docs/doctrines/023-five-dimension-cloud-shard-contract.md establishes that every crate now
// carries a real, git-tracked crate-local shard (capability rows, architecture nodes/edges/links/gaps).
// A Lane task scoped to one crate does not need a full docs/capabilities.db / docs/architecture.db
// rebuild to report an honest ratio for that crate -- the shard itself already carries capability
// status, money/approval flags, and architecture rows. This tool reads NOTHING but that one committed
// text file as its primary source (plus, only as an explicitly separate/optional cross-check, the
// root docs/architecture-canonical/evidence.jsonl committed text file -- never a binary .db, never
// docs/capabilities.db or docs/architecture.db).
//
// Status vocabulary and money-flag convention match tools/capabilities/audit-truth.mjs and
// tools/benchmark/query-benchmark-ratio.mjs exactly: capability status is one of unimplemented /
// implemented / implemented_unverified / verified (docs/023 section "sync:verify-atoms" enum);
// money-moving means moves_money=1; requires_approval is reported separately, never merged into
// moves_money. This tool never claims root docs/capabilities.db or docs/architecture.db authority --
// every count is scoped to what the crate's own committed shard says, and anything the shard itself
// cannot confirm (e.g. architecture_evidence rows the generator supports but this shard does not yet
// carry) is flagged explicitly rather than assumed absent-at-root.
import { readFileSync, existsSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import crypto from 'node:crypto'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const ROOT = path.resolve(__dirname, '..', '..')

const SUBDB_SCHEMA_VERSION = 'chronica-subdb-v1'
const GENESIS_HASH = 'sha256:GENESIS'

// Same enum sync-anchor-v2 (docs/023) declares; a shard status outside this set is never silently
// dropped or coerced -- it is counted in unknown_status_values and never counted as a known bucket.
const CAPABILITY_STATUS_VOCAB = ['unimplemented', 'implemented', 'implemented_unverified', 'verified']

export function shardPathForCrate(crate, root = ROOT) {
  return path.join(root, 'crates', crate, '.chronica', 'sub-cap-arch.jsonl')
}

function sortedObject(value) {
  if (Array.isArray(value)) return value.map(sortedObject)
  if (!value || typeof value !== 'object') return value
  return Object.fromEntries(Object.keys(value).sort().map((key) => [key, sortedObject(value[key])]))
}

function stableJson(value) {
  return JSON.stringify(sortedObject(value))
}

// Reimplements tools/subdb/subdb-lib.mjs's hashRecord independently (rather than importing it) so
// this tool never pulls in better-sqlite3 / native bindings -- it stays a plain, fast text reader,
// matching the "no root .db build needed" design goal.
function hashRecord(record) {
  return `sha256:${crypto.createHash('sha256').update(stableJson(record)).digest('hex')}`
}

export function loadCrateShard(crate, { root = ROOT } = {}) {
  const shardPath = shardPathForCrate(crate, root)
  if (!existsSync(shardPath)) {
    throw new Error(
      `no crate-local shard found for "${crate}": ${path.relative(root, shardPath)} does not exist. ` +
      `Every crate must carry crates/<crate>/.chronica/sub-cap-arch.jsonl per docs/doctrines/023-five-dimension-cloud-shard-contract.md.`,
    )
  }
  const lines = readFileSync(shardPath, 'utf8').split('\n').filter((l) => l.trim().length > 0)
  const records = lines.map((line, index) => {
    try {
      return JSON.parse(line)
    } catch (error) {
      throw new Error(`${shardPath}:${index + 1}: SHARD_PARSE_ERROR: ${error.message}`)
    }
  })
  return { crate, shardPath, records }
}

// Independently re-derives every record_hash/prev_hash link in the shard. This is a real integrity
// check of the exact file the ratio below is computed from, not a trust-on-read assumption.
export function verifyHashChain(records) {
  const errors = []
  let previous = GENESIS_HASH
  records.forEach((row, index) => {
    if (row.sequence !== index) errors.push(`sequence mismatch at index ${index}: expected ${index}, got ${row.sequence}`)
    if (row.prev_hash !== previous) errors.push(`prev_hash mismatch at sequence ${index}`)
    const { record_hash, ...withoutHash } = row
    const expected = hashRecord(withoutHash)
    if (record_hash !== expected) errors.push(`record_hash mismatch at sequence ${index} (record_id=${row.record_id ?? 'unknown'})`)
    previous = record_hash
  })
  return { intact: errors.length === 0, errors }
}

function loadJsonlLoose(jsonlPath) {
  return readFileSync(jsonlPath, 'utf8')
    .split('\n')
    .filter((l) => l.trim().length > 0)
    .map((l) => JSON.parse(l))
}

// Explicitly separate, optional cross-check: does root docs/architecture-canonical/evidence.jsonl (a
// committed text shard, never a binary .db) carry MORE architecture_evidence rows for this crate's own
// node ids than the crate's own committed shard does? If so the crate shard is likely stale for the
// evidence dimension and needs regeneration -- this never feeds the shard-only ratio below, it only
// prevents a reader from mistaking "0 evidence rows in this shard" for "no evidence exists at root".
function crossCheckRootEvidence({ root, crate, architectureNodeIds, shardEvidenceCount }) {
  const evidencePath = path.join(root, 'docs', 'architecture-canonical', 'evidence.jsonl')
  if (!existsSync(evidencePath)) {
    return { performed: false, reason: 'docs/architecture-canonical/evidence.jsonl not found in this checkout' }
  }
  try {
    const rootRows = loadJsonlLoose(evidencePath)
    let rootRowsForCrateNodes = 0
    for (const entry of rootRows) {
      const record = entry.record ?? entry
      if (record.record_type === 'architecture_evidence' && architectureNodeIds.has(record.architecture_node)) {
        rootRowsForCrateNodes += 1
      }
    }
    const shardLooksStale = rootRowsForCrateNodes > shardEvidenceCount
    return {
      performed: true,
      root_evidence_rows_for_this_crates_nodes: rootRowsForCrateNodes,
      shard_evidence_rows: shardEvidenceCount,
      shard_looks_stale_for_evidence: shardLooksStale,
      note: shardLooksStale
        ? `docs/architecture-canonical/evidence.jsonl carries ${rootRowsForCrateNodes} architecture_evidence row(s) for this crate's own architecture_node ids, but the committed crate shard only carries ${shardEvidenceCount}. This shard likely predates evidence sharding for this crate (see tools/subdb/subdb-lib.mjs's architectureEvidenceRows) or needs regeneration: pnpm subdb:gen -- --crate ${crate}. Do not read the shard-only architecture_evidence count below as proof no evidence exists at root.`
        : 'Root docs/architecture-canonical/evidence.jsonl does not carry more architecture_evidence rows for this crate\'s nodes than the shard already has.',
    }
  } catch (error) {
    return { performed: false, reason: `cross-check read failed: ${error.message}` }
  }
}

export function computeCrateRatio(crate, { root = ROOT, crossCheckRootEvidenceFile = true } = {}) {
  const { shardPath, records } = loadCrateShard(crate, { root })
  const meta = records.find((r) => r.record_type === 'meta') ?? null
  const chain = verifyHashChain(records)

  const capabilities = records.filter((r) => r.record_type === 'capability')
  const architectureNodes = records.filter((r) => r.record_type === 'architecture_node')
  const architectureEdges = records.filter((r) => r.record_type === 'architecture_edge')
  const architectureEvidence = records.filter((r) => r.record_type === 'architecture_evidence')
  const links = records.filter((r) => r.record_type === 'capability_architecture_link')
  const gaps = records.filter((r) => r.record_type === 'architecture_target_gap')

  const byStatus = Object.fromEntries(CAPABILITY_STATUS_VOCAB.map((s) => [s, 0]))
  const unknownStatusValues = new Set()
  for (const cap of capabilities) {
    if (Object.prototype.hasOwnProperty.call(byStatus, cap.status)) {
      byStatus[cap.status] += 1
    } else {
      unknownStatusValues.add(String(cap.status))
    }
  }

  const moneyMoving = capabilities.filter((c) => Number(c.moves_money ?? 0) === 1)
  const moneyMovingVerified = moneyMoving.filter((c) => c.status === 'verified')
  const requiresApproval = capabilities.filter((c) => Number(c.requires_approval ?? 0) === 1)
  const requiresApprovalVerified = requiresApproval.filter((c) => c.status === 'verified')

  const nodesWithEvidenceRef = architectureNodes.filter((n) => n.evidence_ref != null && String(n.evidence_ref).trim().length > 0)
  const architectureNodeIds = new Set(architectureNodes.map((n) => n.node_id))
  const nodeIdsWithEvidenceRow = new Set(architectureEvidence.map((e) => e.architecture_node))
  const nodesWithEvidenceRow = architectureNodes.filter((n) => nodeIdsWithEvidenceRow.has(n.node_id))

  const gapsByStatus = {}
  for (const gap of gaps) {
    const key = String(gap.status ?? 'unknown')
    gapsByStatus[key] = (gapsByStatus[key] ?? 0) + 1
  }

  const rootEvidenceCrossCheck = crossCheckRootEvidenceFile
    ? crossCheckRootEvidence({ root, crate, architectureNodeIds, shardEvidenceCount: architectureEvidence.length })
    : { performed: false, reason: 'disabled by caller' }

  const capabilityTotal = capabilities.length

  return {
    schema_version: 1,
    generated_at: null, // stamped by main()/caller
    crate,
    shard_path: path.relative(root, shardPath).split(path.sep).join('/'),
    truth_label: 'LOCAL_AUDIT_REQUIRED',
    authority:
      'This crate\'s own committed crates/<crate>/.chronica/sub-cap-arch.jsonl shard only. Root ' +
      'docs/capabilities.db and docs/architecture.db are local aggregate audit targets not read here; ' +
      'per docs/doctrines/023-five-dimension-cloud-shard-contract.md, canonical JSONL (including this crate shard) ' +
      'is cloud-readable tracking authority, and any generated DB is a rebuildable cache, never authority.',
    shard_meta: meta && {
      schema_version: meta.schema_version,
      crate: meta.crate,
      crate_path: meta.crate_path,
      cargo_path: meta.cargo_path,
    },
    schema_version_matches_contract: meta?.schema_version === SUBDB_SCHEMA_VERSION,
    hash_chain: {
      intact: chain.intact,
      record_count: records.length,
      errors: chain.errors,
    },
    capability_ratio: {
      total: capabilityTotal,
      by_status: byStatus,
      unknown_status_values: [...unknownStatusValues],
      verified_ratio: `${byStatus.verified}/${capabilityTotal}`,
      implemented_unverified_ratio: `${byStatus.implemented_unverified}/${capabilityTotal}`,
      unimplemented_ratio: `${byStatus.unimplemented}/${capabilityTotal}`,
    },
    money_moving: {
      total: moneyMoving.length,
      verified: moneyMovingVerified.length,
      verified_ratio: `${moneyMovingVerified.length}/${moneyMoving.length}`,
      capability_keys: moneyMoving.map((c) => c.capability_key).sort(),
      capability_keys_verified: moneyMovingVerified.map((c) => c.capability_key).sort(),
    },
    requires_approval: {
      total: requiresApproval.length,
      verified: requiresApprovalVerified.length,
      verified_ratio: `${requiresApprovalVerified.length}/${requiresApproval.length}`,
      capability_keys: requiresApproval.map((c) => c.capability_key).sort(),
    },
    architecture: {
      nodes_total: architectureNodes.length,
      edges_total: architectureEdges.length,
      nodes_with_evidence_ref: nodesWithEvidenceRef.length,
      nodes_with_evidence_ref_ratio: `${nodesWithEvidenceRef.length}/${architectureNodes.length}`,
      evidence_rows_in_shard: architectureEvidence.length,
      nodes_with_evidence_row_in_shard: nodesWithEvidenceRow.length,
      nodes_with_evidence_row_in_shard_ratio: `${nodesWithEvidenceRow.length}/${architectureNodes.length}`,
      root_evidence_cross_check: rootEvidenceCrossCheck,
      capability_architecture_links_total: links.length,
      target_gaps_total: gaps.length,
      target_gaps_by_status: gapsByStatus,
    },
    non_claims: [
      'This tool does not read or require docs/capabilities.db or docs/architecture.db.',
      'This tool does not read any other crate\'s shard -- only crates/<crate>/.chronica/sub-cap-arch.jsonl for the requested crate.',
      'evidence_rows_in_shard reflects only what this crate\'s own committed shard carries today; see architecture.root_evidence_cross_check for whether root canonical text has more that has not been sharded here yet.',
      'A capability status outside unimplemented/implemented/implemented_unverified/verified is never silently counted as a known bucket -- see capability_ratio.unknown_status_values.',
      'hash_chain.intact=false means this tool cannot vouch that the counts above reflect an untampered, correctly-chained shard -- treat all counts as HUMAN_AUTH_REQUIRED until resolved.',
    ],
  }
}

function pct(count, total) {
  if (total === 0) return 'n/a'
  return `${((count / total) * 100).toFixed(1)}%`
}

function printSummary(result) {
  console.log(`crate-ratio: ${result.crate} (${result.shard_path})`)
  console.log(`  hash_chain_intact: ${result.hash_chain.intact} (${result.hash_chain.record_count} records)`)
  const cr = result.capability_ratio
  console.log(`  capabilities: total=${cr.total}`)
  for (const status of CAPABILITY_STATUS_VOCAB) {
    console.log(`    ${status}: ${cr.by_status[status]}/${cr.total} (${pct(cr.by_status[status], cr.total)})`)
  }
  if (cr.unknown_status_values.length > 0) {
    console.log(`    UNKNOWN status values found (not counted above): ${cr.unknown_status_values.join(', ')}`)
  }
  const mm = result.money_moving
  console.log(`  money_moving: ${mm.total} total, ${mm.verified}/${mm.total} verified (${pct(mm.verified, mm.total)})`)
  const ra = result.requires_approval
  console.log(`  requires_approval: ${ra.total} total, ${ra.verified}/${ra.total} verified (${pct(ra.verified, ra.total)})`)
  const arch = result.architecture
  console.log(`  architecture: nodes=${arch.nodes_total}, edges=${arch.edges_total}, links=${arch.capability_architecture_links_total}, target_gaps=${arch.target_gaps_total}`)
  console.log(`  architecture nodes with evidence_ref: ${arch.nodes_with_evidence_ref}/${arch.nodes_total} (${pct(arch.nodes_with_evidence_ref, arch.nodes_total)})`)
  console.log(`  architecture_evidence rows in shard: ${arch.evidence_rows_in_shard} (covers ${arch.nodes_with_evidence_row_in_shard}/${arch.nodes_total} nodes)`)
  if (arch.root_evidence_cross_check.performed && arch.root_evidence_cross_check.shard_looks_stale_for_evidence) {
    console.log(`  WARNING: ${arch.root_evidence_cross_check.note}`)
  } else if (!arch.root_evidence_cross_check.performed) {
    console.log(`  (root evidence cross-check skipped: ${arch.root_evidence_cross_check.reason})`)
  }
}

function usage() {
  return 'Usage: node tools/capabilities/query-crate-ratio.mjs <crate-name> [--json] [--root <path>] [--no-cross-check]'
}

function main() {
  const argv = process.argv.slice(2)
  const positional = argv.filter((a) => !a.startsWith('--'))
  const crate = positional[0]
  if (!crate) {
    console.error(usage())
    process.exitCode = 2
    return
  }
  const json = argv.includes('--json')
  const crossCheckRootEvidenceFile = !argv.includes('--no-cross-check')
  const rootFlagIndex = argv.indexOf('--root')
  const root = rootFlagIndex >= 0 ? path.resolve(argv[rootFlagIndex + 1]) : ROOT

  let result
  try {
    result = computeCrateRatio(crate, { root, crossCheckRootEvidenceFile })
  } catch (error) {
    console.error(`query-crate-ratio failed: ${error.message}`)
    process.exitCode = 1
    return
  }
  result.generated_at = new Date().toISOString()

  if (json) {
    console.log(JSON.stringify(result, null, 2))
  } else {
    printSummary(result)
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
