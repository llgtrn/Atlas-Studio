import test from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { computeCrateRatio, loadCrateShard, shardPathForCrate, verifyHashChain } from './query-crate-ratio.mjs'

const SCRIPT_PATH = fileURLToPath(new URL('./query-crate-ratio.mjs', import.meta.url))
const REPO_ROOT = fileURLToPath(new URL('../../', import.meta.url))

function runCli(args) {
  return spawnSync(process.execPath, [SCRIPT_PATH, ...args], { encoding: 'utf8' })
}

// Counts every dimension the tool reports by re-parsing the raw shard file with a completely
// independent code path (plain reduce over JSON.parse'd lines, no shared helper functions with
// query-crate-ratio.mjs) so a bug in the tool's own counting logic cannot hide from this test.
function independentCount(shardPath) {
  const records = readFileSync(shardPath, 'utf8')
    .split('\n')
    .filter((l) => l.trim().length > 0)
    .map((l) => JSON.parse(l))
  const out = {
    total: 0,
    verified: 0,
    implemented_unverified: 0,
    unimplemented: 0,
    moneyMoving: 0,
    moneyMovingVerified: 0,
    requiresApproval: 0,
    nodesTotal: 0,
    edgesTotal: 0,
    evidenceRowsInShard: 0,
    linksTotal: 0,
    gapsTotal: 0,
  }
  for (const rec of records) {
    if (rec.record_type === 'capability') {
      out.total += 1
      if (rec.status === 'verified') out.verified += 1
      if (rec.status === 'implemented_unverified') out.implemented_unverified += 1
      if (rec.status === 'unimplemented') out.unimplemented += 1
      if (Number(rec.moves_money ?? 0) === 1) {
        out.moneyMoving += 1
        if (rec.status === 'verified') out.moneyMovingVerified += 1
      }
      if (Number(rec.requires_approval ?? 0) === 1) out.requiresApproval += 1
    }
    if (rec.record_type === 'architecture_node') out.nodesTotal += 1
    if (rec.record_type === 'architecture_edge') out.edgesTotal += 1
    if (rec.record_type === 'architecture_evidence') out.evidenceRowsInShard += 1
    if (rec.record_type === 'capability_architecture_link') out.linksTotal += 1
    if (rec.record_type === 'architecture_target_gap') out.gapsTotal += 1
  }
  return out
}

// Real crates known (from this session's exploration of every committed
// crates/*/.chronica/sub-cap-arch.jsonl shard) to have at least one verified capability, so the
// verified-count assertions below exercise a non-zero path, not just the all-zero default.
const REAL_CRATES_WITH_VERIFIED_CAPS = ['chronica-platform-plane', 'chronica-social', 'chronica-crm']

for (const crate of REAL_CRATES_WITH_VERIFIED_CAPS) {
  test(`computeCrateRatio(${crate}) matches an independent recount of its real committed shard`, () => {
    const shardPath = shardPathForCrate(crate, REPO_ROOT)
    assert.ok(existsSync(shardPath), `expected ${shardPath} to exist -- every crate must carry a committed shard`)
    const expected = independentCount(shardPath)
    const result = computeCrateRatio(crate, { root: REPO_ROOT })

    assert.equal(result.capability_ratio.total, expected.total)
    assert.equal(result.capability_ratio.by_status.verified, expected.verified)
    assert.equal(result.capability_ratio.by_status.implemented_unverified, expected.implemented_unverified)
    assert.equal(result.capability_ratio.by_status.unimplemented, expected.unimplemented)
    assert.equal(result.money_moving.total, expected.moneyMoving)
    assert.equal(result.money_moving.verified, expected.moneyMovingVerified)
    assert.equal(result.requires_approval.total, expected.requiresApproval)
    assert.equal(result.architecture.nodes_total, expected.nodesTotal)
    assert.equal(result.architecture.edges_total, expected.edgesTotal)
    assert.equal(result.architecture.evidence_rows_in_shard, expected.evidenceRowsInShard)
    assert.equal(result.architecture.capability_architecture_links_total, expected.linksTotal)
    assert.equal(result.architecture.target_gaps_total, expected.gapsTotal)
    assert.ok(expected.verified >= 1, `expected ${crate} to have at least one verified capability in its real shard`)
  })
}

test('computeCrateRatio(chronica-core) matches an independent recount and never claims root DB authority', () => {
  const shardPath = shardPathForCrate('chronica-core', REPO_ROOT)
  const expected = independentCount(shardPath)
  const result = computeCrateRatio('chronica-core', { root: REPO_ROOT })
  assert.equal(result.capability_ratio.total, expected.total)
  assert.equal(result.capability_ratio.by_status.verified, expected.verified)
  assert.equal(result.truth_label, 'LOCAL_AUDIT_REQUIRED')
  assert.ok(result.non_claims.some((c) => c.includes('docs/capabilities.db')))
  assert.ok(result.non_claims.some((c) => c.includes('does not read any other crate')))
})

test('computeCrateRatio never counts a verified capability count greater than the total for any real crate shard', () => {
  const result = computeCrateRatio('chronica-social', { root: REPO_ROOT })
  const cr = result.capability_ratio
  const sum = cr.by_status.unimplemented + cr.by_status.implemented + cr.by_status.implemented_unverified + cr.by_status.verified
  assert.equal(sum + cr.unknown_status_values.length, cr.total)
  assert.ok(result.money_moving.verified <= result.money_moving.total)
  assert.ok(result.requires_approval.verified <= result.requires_approval.total)
})

test('real committed crate shards this tool reads pass their own hash-chain integrity check', () => {
  for (const crate of ['chronica-core', 'chronica-platform-plane', 'chronica-vault', 'chronica-approvals']) {
    const { records } = loadCrateShard(crate, { root: REPO_ROOT })
    const chain = verifyHashChain(records)
    assert.equal(chain.intact, true, `${crate} shard hash chain broken: ${chain.errors.join('; ')}`)
  }
})

test('verifyHashChain detects a tampered record_hash', () => {
  const { records } = loadCrateShard('chronica-vault', { root: REPO_ROOT })
  const tampered = records.map((r, i) => (i === 1 ? { ...r, capability_key: 'tampered.injected' } : r))
  const chain = verifyHashChain(tampered)
  assert.equal(chain.intact, false)
  assert.ok(chain.errors.length > 0)
})

test('root evidence cross-check flags a real committed shard as stale when root canonical evidence.jsonl has more rows than the shard', () => {
  const result = computeCrateRatio('chronica-core', { root: REPO_ROOT })
  const crossCheck = result.architecture.root_evidence_cross_check
  if (!existsSync(join(REPO_ROOT, 'docs', 'architecture-canonical', 'evidence.jsonl'))) {
    assert.equal(crossCheck.performed, false)
    return
  }
  assert.equal(crossCheck.performed, true)
  assert.equal(typeof crossCheck.root_evidence_rows_for_this_crates_nodes, 'number')
  assert.equal(crossCheck.shard_evidence_rows, result.architecture.evidence_rows_in_shard)
})

test('--no-cross-check disables the root evidence cross-check', () => {
  const result = computeCrateRatio('chronica-core', { root: REPO_ROOT, crossCheckRootEvidenceFile: false })
  assert.equal(result.architecture.root_evidence_cross_check.performed, false)
  assert.equal(result.architecture.root_evidence_cross_check.reason, 'disabled by caller')
})

test('loadCrateShard throws a clear, actionable error for a crate with no committed shard', () => {
  assert.throws(
    () => loadCrateShard('chronica-does-not-exist-xyz', { root: REPO_ROOT }),
    /no crate-local shard found for "chronica-does-not-exist-xyz"/,
  )
})

test('an unknown capability status value is flagged, not silently counted as a known bucket', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-crate-ratio-fixture-'))
  const crateDir = join(root, 'crates', 'chronica-fixture', '.chronica')
  mkdirSync(crateDir, { recursive: true })
  const rows = [
    { sequence: 0, prev_hash: 'sha256:GENESIS', record_id: 'meta:chronica-fixture', record_type: 'meta', schema_version: 'chronica-subdb-v1', crate: 'chronica-fixture', crate_path: 'crates/chronica-fixture', cargo_path: 'crates/chronica-fixture/Cargo.toml' },
    { record_type: 'capability', capability_key: 'fixture.weird_status', status: 'mid_flight_unknown_value', moves_money: 0, requires_approval: 0 },
  ]
  // These fixture rows deliberately do NOT carry a valid hash chain (this test only exercises
  // status-vocabulary handling, not chain integrity -- that is covered by the hash-chain tests above).
  writeFileSync(join(crateDir, 'sub-cap-arch.jsonl'), rows.map((r) => JSON.stringify(r)).join('\n') + '\n')

  const result = computeCrateRatio('chronica-fixture', { root, crossCheckRootEvidenceFile: false })
  assert.equal(result.capability_ratio.total, 1)
  assert.equal(result.capability_ratio.by_status.verified, 0)
  assert.equal(result.capability_ratio.by_status.unimplemented, 0)
  assert.deepEqual(result.capability_ratio.unknown_status_values, ['mid_flight_unknown_value'])
})

test('CLI smoke: default text summary for a real crate, run as a real child process', () => {
  const { status, stdout, stderr } = runCli(['chronica-vault'])
  assert.equal(status, 0, `expected exit 0, got ${status}; stderr: ${stderr}`)
  assert.match(stdout, /crate-ratio: chronica-vault/)
  assert.match(stdout, /hash_chain_intact: true/)
})

test('CLI smoke: --json prints parseable JSON matching the library result', () => {
  const { status, stdout, stderr } = runCli(['chronica-vault', '--json'])
  assert.equal(status, 0, `expected exit 0, got ${status}; stderr: ${stderr}`)
  const parsed = JSON.parse(stdout)
  assert.equal(parsed.crate, 'chronica-vault')
  assert.equal(parsed.shard_path, 'crates/chronica-vault/.chronica/sub-cap-arch.jsonl')
  assert.ok(parsed.generated_at)
})

test('CLI smoke: unknown crate name exits non-zero with a clear stderr message, never a stack trace', () => {
  const { status, stdout, stderr } = runCli(['chronica-totally-not-real'])
  assert.notEqual(status, 0)
  assert.equal(stdout, '')
  assert.match(stderr, /no crate-local shard found for "chronica-totally-not-real"/)
})

test('CLI smoke: no crate name argument prints usage and exits non-zero', () => {
  const { status, stderr } = runCli([])
  assert.notEqual(status, 0)
  assert.match(stderr, /Usage: node tools\/capabilities\/query-crate-ratio\.mjs/)
})
