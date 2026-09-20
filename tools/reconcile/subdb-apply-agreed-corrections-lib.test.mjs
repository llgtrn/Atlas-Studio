import assert from 'node:assert/strict'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { chainRecords, verifyCrateSubdbs } from '../subdb/subdb-lib.mjs'
import { REASON } from './sync-anchor-v2-lib.mjs'
import {
  applyAgreedCorrections,
  applyCorrectionsForCrate,
  selectDeferredToPrefixFindings,
  selectEligibleFindings,
  SKIP_REASON,
} from './subdb-apply-agreed-corrections-lib.mjs'

function makeWorkspace(capabilityRows) {
  const root = mkdtempSync(join(tmpdir(), 'chronica-apply-corrections-'))
  writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["crates/chronica-fixture"]\n')
  const cratePath = join(root, 'crates', 'chronica-fixture')
  mkdirSync(join(cratePath, 'src'), { recursive: true })
  mkdirSync(join(cratePath, '.chronica'), { recursive: true })
  writeFileSync(join(cratePath, 'Cargo.toml'), '[package]\nname = "chronica-fixture"\nversion = "0.1.0"\n')

  const plain = [
    { record_id: 'meta:chronica-fixture', record_type: 'meta', schema_version: 'chronica-subdb-v1', crate: 'chronica-fixture' },
    ...capabilityRows.map((r) => ({
      record_id: `capability:${r.capability_key}`,
      record_type: 'capability',
      moves_money: 0,
      requires_approval: 0,
      ...r,
    })),
  ]
  const chained = chainRecords(plain)
  writeFileSync(
    join(cratePath, '.chronica', 'sub-cap-arch.jsonl'),
    `${chained.map((r) => JSON.stringify(r)).join('\n')}\n`,
  )
  return { root, cratePath }
}

function finding({ capability_key, crate = 'chronica-fixture', confidence = 'exact', target_module = 'some_mod' }) {
  return {
    capability_key,
    crate,
    target_module,
    shard_status: 'unimplemented',
    doc_status: null,
    verdict: 'DISAGREE',
    reason: REASON.SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL,
    evidence: {
      match_confidence: confidence,
      matched_modules: [`src/${target_module}.rs`],
      code_state: 'real',
      reachable: true,
    },
  }
}

function readRows(cratePath) {
  return readFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), 'utf8')
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => JSON.parse(line))
}

test('selectEligibleFindings: only EXACT-confidence UNIMPLEMENTED_BUT_CODE_IS_REAL findings qualify', () => {
  const findings = [
    finding({ capability_key: 'a.one', confidence: 'exact' }),
    finding({ capability_key: 'a.two', confidence: 'prefix' }),
    { ...finding({ capability_key: 'a.three', confidence: 'exact' }), reason: REASON.CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT },
  ]
  const eligible = selectEligibleFindings(findings)
  assert.deepEqual(eligible.map((f) => f.capability_key), ['a.one'])
  const deferred = selectDeferredToPrefixFindings(findings)
  assert.deepEqual(deferred.map((f) => f.capability_key), ['a.two'])
})

test('applyCorrectionsForCrate: appends a correction record without touching the original row', () => {
  const { root, cratePath } = makeWorkspace([
    { capability_key: 'fixture.alpha', target_module: 'alpha', status: 'unimplemented' },
    { capability_key: 'fixture.beta', target_module: 'beta', status: 'unimplemented' },
  ])
  try {
    const before = readRows(cratePath)
    const result = applyCorrectionsForCrate({
      root,
      crateName: 'chronica-fixture',
      cratePath: 'crates/chronica-fixture',
      findings: [finding({ capability_key: 'fixture.alpha', target_module: 'alpha' })],
      now: () => '2026-07-31T00:00:00.000Z',
    })
    assert.equal(result.appendedCount, 1)
    assert.deepEqual(result.applied, [{ capability_key: 'fixture.alpha', target_module: 'alpha' }])

    const after = readRows(cratePath)
    assert.equal(after.length, before.length + 1)
    // original rows are byte-identical (never rewritten)
    for (let i = 0; i < before.length; i += 1) assert.deepEqual(after[i], before[i])

    const appended = after[after.length - 1]
    assert.equal(appended.record_type, 'capability_status_correction')
    assert.equal(appended.capability_key, 'fixture.alpha')
    assert.equal(appended.previous_status, 'unimplemented')
    assert.equal(appended.new_status, 'implemented')
    assert.equal(appended.sequence, before.length)
    assert.equal(appended.prev_hash, before[before.length - 1].record_hash)

    const verify = verifyCrateSubdbs({ root, crates: ['chronica-fixture'] })
    assert.equal(verify.ok, true, verify.errors.join('; '))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('applyCorrectionsForCrate: PREFIX-confidence findings are never selected for correction', () => {
  const { root } = makeWorkspace([{ capability_key: 'fixture.alpha', target_module: 'alpha', status: 'unimplemented' }])
  try {
    const eligible = selectEligibleFindings([finding({ capability_key: 'fixture.alpha', confidence: 'prefix' })])
    assert.equal(eligible.length, 0)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('applyCorrectionsForCrate: re-running is idempotent (already-corrected capability is skipped, not double-appended)', () => {
  const { root, cratePath } = makeWorkspace([{ capability_key: 'fixture.alpha', target_module: 'alpha', status: 'unimplemented' }])
  try {
    const f = finding({ capability_key: 'fixture.alpha', target_module: 'alpha' })
    const first = applyCorrectionsForCrate({ root, crateName: 'chronica-fixture', cratePath: 'crates/chronica-fixture', findings: [f] })
    assert.equal(first.appendedCount, 1)

    const second = applyCorrectionsForCrate({ root, crateName: 'chronica-fixture', cratePath: 'crates/chronica-fixture', findings: [f] })
    assert.equal(second.appendedCount, 0)
    assert.deepEqual(second.skipped, [
      { capability_key: 'fixture.alpha', reason: SKIP_REASON.ALREADY_NOT_UNIMPLEMENTED_IN_CURRENT_SHARD },
    ])

    const verify = verifyCrateSubdbs({ root, crates: ['chronica-fixture'] })
    assert.equal(verify.ok, true, verify.errors.join('; '))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('applyCorrectionsForCrate: unknown capability_key is skipped, not fabricated', () => {
  const { root } = makeWorkspace([{ capability_key: 'fixture.alpha', target_module: 'alpha', status: 'unimplemented' }])
  try {
    const result = applyCorrectionsForCrate({
      root,
      crateName: 'chronica-fixture',
      cratePath: 'crates/chronica-fixture',
      findings: [finding({ capability_key: 'fixture.does_not_exist' })],
    })
    assert.equal(result.appendedCount, 0)
    assert.deepEqual(result.skipped, [
      { capability_key: 'fixture.does_not_exist', reason: SKIP_REASON.CAPABILITY_KEY_NOT_FOUND_IN_CURRENT_SHARD },
    ])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('applyCorrectionsForCrate: a tampered pre-existing chain is refused, not extended', () => {
  const { root, cratePath } = makeWorkspace([{ capability_key: 'fixture.alpha', target_module: 'alpha', status: 'unimplemented' }])
  try {
    const shardPath = join(cratePath, '.chronica', 'sub-cap-arch.jsonl')
    const rows = readRows(cratePath)
    rows[1].status = 'implemented' // tamper without recomputing record_hash
    writeFileSync(shardPath, `${rows.map((r) => JSON.stringify(r)).join('\n')}\n`)

    const result = applyCorrectionsForCrate({
      root,
      crateName: 'chronica-fixture',
      cratePath: 'crates/chronica-fixture',
      findings: [finding({ capability_key: 'fixture.alpha' })],
    })
    assert.equal(result.appendedCount, undefined)
    assert.deepEqual(result.skipped, [
      { capability_key: 'fixture.alpha', reason: SKIP_REASON.PRE_EXISTING_HASH_CHAIN_BROKEN },
    ])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('applyAgreedCorrections: end-to-end over multiple findings reports corrected/skipped/deferred counts and runs subdb:verify', () => {
  const { root, cratePath } = makeWorkspace([
    { capability_key: 'fixture.alpha', target_module: 'alpha', status: 'unimplemented' },
    { capability_key: 'fixture.beta', target_module: 'beta', status: 'unimplemented' },
  ])
  try {
    const findings = [
      finding({ capability_key: 'fixture.alpha', target_module: 'alpha', confidence: 'exact' }),
      finding({ capability_key: 'fixture.beta', target_module: 'beta', confidence: 'prefix' }),
    ]
    const result = applyAgreedCorrections({ root, findings })
    assert.equal(result.exact_confidence_findings, 1)
    assert.equal(result.corrected_count, 1)
    assert.equal(result.skipped_count, 0)
    assert.equal(result.deferred_to_prefix_count, 1)
    assert.equal(result.crates_touched, 1)
    assert.deepEqual(result.verify_failures, [])

    const rows = readRows(cratePath)
    const correction = rows.find((r) => r.record_type === 'capability_status_correction')
    assert.ok(correction)
    assert.equal(correction.capability_key, 'fixture.alpha')
    assert.ok(existsSync(join(cratePath, '.chronica', 'sub-cap-arch.db')), 'sqlite cache must be rebuilt too')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('applyAgreedCorrections: findings for a crate outside the live workspace are skipped, not crashed on', () => {
  const { root } = makeWorkspace([{ capability_key: 'fixture.alpha', target_module: 'alpha', status: 'unimplemented' }])
  try {
    const findings = [finding({ capability_key: 'ghost.cap', crate: 'chronica-does-not-exist' })]
    const result = applyAgreedCorrections({ root, findings })
    assert.equal(result.corrected_count, 0)
    assert.equal(result.skipped_count, 1)
    assert.equal(result.per_crate[0].skipped[0].reason, SKIP_REASON.NOT_A_LIVE_WORKSPACE_CRATE)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
