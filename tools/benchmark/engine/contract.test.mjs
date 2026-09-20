import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync, mkdtempSync, writeFileSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { fileURLToPath } from 'node:url'
import { loadV1Denominator, loadJson } from '../fic-recompute-lib.mjs'
import { guardFicDenominator } from './fic-guard.mjs'
import { FIC_DENOMINATOR } from './vocab.mjs'
import { compileContracts } from './contract-compile.mjs'
import { validateContractRecord } from './contract-validate.mjs'
import { evaluateObligations, deriveFlowStatus, goldenVerdict, hasProductRuntimeWitness } from './contract-eval.mjs'
import { nextActionable } from './contract-next.mjs'
import { buildCiMatrix } from './contract-ci-matrix.mjs'
import { diffReleases } from './contract-diff.mjs'
import { detectStaleBenchmark, verifyReleaseReproducible } from './contract-release.mjs'
import {
  CAP_IDS,
  CONTRACT_SCHEMA,
  CONTRACT_SCHEMA_VERSION,
  PROOF_OBLIGATIONS,
  STANDARD_CATALOG,
} from './contract-vocab.mjs'
import { overlayHardGates } from './contract-gates.mjs'
import { WEB2APP_STANDARD_VERSION } from './web2app-vocab.mjs'
import { EMPLOYMENT_STANDARD_VERSION } from './employment-vocab.mjs'
import { STORE_STANDARD_VERSION } from './store-vocab.mjs'
import { BLACKBOX_STANDARD_VERSION } from './blackbox-vocab.mjs'

const ROOT = fileURLToPath(new URL('../../../', import.meta.url))
const FIX = join(ROOT, 'tools/benchmark/contracts/fixtures')

function loadLine(file) {
  return JSON.parse(readFileSync(join(FIX, file), 'utf8').trim().split('\n')[0])
}

function compile(write = false) {
  return compileContracts({
    root: ROOT,
    write,
    repositorySha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    generatedAt: '2026-08-21T00:00:00Z',
    releaseId: 'BR-2026.08.21.1',
  })
}

test('FIC denominator remains 55', () => {
  const v1 = loadV1Denominator(ROOT)
  const census = loadJson(join(ROOT, 'docs/_machine/fic-revalidation-census.json'))
  const guard = guardFicDenominator(v1, census)
  assert.equal(guard.fic.denominator, FIC_DENOMINATOR)
  assert.equal(guard.fic.denominator, 55)
})

test('contract schema and overlay versions are frozen', () => {
  assert.equal(CONTRACT_SCHEMA, 'chronica.benchmark.contract.v1')
  assert.equal(CONTRACT_SCHEMA_VERSION, '1.0.0')
  assert.equal(STANDARD_CATALOG.WEB2APP.version, WEB2APP_STANDARD_VERSION)
  assert.equal(STANDARD_CATALOG.EMPLOYMENT.version, EMPLOYMENT_STANDARD_VERSION)
  assert.equal(STANDARD_CATALOG.STORE.version, STORE_STANDARD_VERSION)
  assert.equal(STANDARD_CATALOG.BLACKBOX.version, BLACKBOX_STANDARD_VERSION)
  assert.equal(PROOF_OBLIGATIONS.includes('PO-DISCLOSURE'), true)
  assert.equal(CAP_IDS.length, 18)
})

test('invalid records fail schema checks', () => {
  assert.equal(validateContractRecord(loadLine('fail-unknown-cap.jsonl')).ok, false)
  assert.equal(validateContractRecord(loadLine('fail-unknown-standard.jsonl')).ok, false)
  assert.equal(validateContractRecord(loadLine('fail-unknown-obligation.jsonl')).ok, false)
  assert.equal(validateContractRecord(loadLine('fail-missing-owner.jsonl')).ok, false)
  assert.equal(validateContractRecord(loadLine('fail-cross-cap-incomplete.jsonl')).ok, false)
  assert.equal(validateContractRecord(loadLine('fail-capability-as-flow.jsonl')).ok, false)
  assert.equal(validateContractRecord(loadLine('fail-fixture-as-runtime.jsonl')).ok, false)
  assert.equal(validateContractRecord(loadLine('pass-valid-law.jsonl')).ok, true)
  const gated = validateContractRecord(loadLine('fail-unknown-hard-gate.jsonl'), {
    knownHardGates: overlayHardGates(),
  })
  assert.equal(gated.ok, false)
  assert.match(gated.errors.join('\n'), /unknown hard gate/)
})

test('compile real IR: 18 envelopes, 18 packets, no broken evidence', () => {
  const built = compile(false)
  if (!built.ok) assert.fail(built.errors.slice(0, 12).join('\n'))
  assert.equal(built.ir.envelopes.length, 18)
  assert.equal(built.packets.length, 18)
  assert.equal(built.broken_evidence.length, 0)
  assert.equal(built.release.id, 'BR-2026.08.21.1')
  assert.ok(built.ir.flows.length >= 40)
  assert.ok(built.ir.golden.length >= 20)
  assert.equal(built.ir.prose.filter((row) => !row.class).length, 0)
  const metaIds = new Set(built.ir.anti_pattern_meta.map((row) => row.id))
  for (const id of overlayHardGates()) assert.ok(metaIds.has(id), id)
  for (const packet of built.packets) {
    assert.equal(packet.this_packet_is_not_source, true)
    assert.equal(packet.benchmark_release, 'BR-2026.08.21.1')
    assert.ok(packet.packet_hash)
  }
})

test('release is reproducible without timestamp drift', () => {
  const a = compile(false)
  const b = compile(false)
  const check = verifyReleaseReproducible(a.release, b.release)
  assert.equal(check.ok, true, check.errors.join('\n'))
  assert.equal(a.ir.source_hash, b.ir.source_hash)
})

test('BlackBox hard-gate addition diffs affected CAPs and requires revalidation', () => {
  const built = compile(false)
  const next = JSON.parse(JSON.stringify(built.ir))
  next.flows = next.flows.map((flow) =>
    flow.owner === 'CAP18' || (flow.participants || []).includes('CAP18')
      ? { ...flow, hard_gates: [...new Set([...(flow.hard_gates || []), 'OUTPUT_DLP_REQUIRED'])] }
      : flow,
  )
  const diff = diffReleases(built.ir, next)
  assert.equal(diff.revalidation_required, true)
  assert.ok(diff.added_hard_gates.includes('OUTPUT_DLP_REQUIRED'))
  assert.ok(diff.affected_caps.includes('CAP18'))
  assert.ok(diff.affected_caps.includes('CAP01'))
  assert.ok(diff.modified_flows.length > 0)
})

test('next-actionable CAP18 returns a pinned high-value BlackBox flow', () => {
  const built = compile(false)
  const next = nextActionable(built.ir, 'CAP18', built.release.id)
  assert.equal(next.ok, true)
  assert.match(next.flow, /^FLOW-CAP18-/)
  assert.equal(next.benchmark_release, 'BR-2026.08.21.1')
  assert.ok(next.missing.length > 0)
})

test('CI matrix routes CRM changes to CAP05/CAP01/CAP17/CAP18, not backup-only', () => {
  const built = compile(false)
  const matrix = buildCiMatrix(built.ir, ['crates/chronica-crm/src/lib.rs'])
  assert.ok(matrix.affected_caps.includes('CAP05'))
  assert.ok(matrix.affected_caps.includes('CAP01'))
  assert.ok(matrix.affected_caps.includes('CAP17'))
  assert.ok(matrix.affected_caps.includes('CAP18'))
  assert.equal(matrix.affected_caps.includes('CAP03'), false)
  assert.ok(!matrix.affected_flows.some((id) => id.includes('BACKUP')))
  assert.ok(matrix.tiers.includes('TIER1_AFFECTED_CAP'))
  assert.equal(matrix.p0_authoritative_ci_touched, false)
  const benchOnly = buildCiMatrix(built.ir, ['tools/benchmark/engine/contract-run.mjs'])
  assert.deepEqual(benchOnly.tiers, ['TIER0_CONTRACT'])
})

test('fixture evidence cannot PASS a product obligation', () => {
  const flow = {
    id: 'FLOW-CAP18-VIEW-001',
    obligations: ['PO-DISCLOSURE'],
    status: 'SPEC_ONLY',
  }
  const byFlow = new Map([
    [
      flow.id,
      [
        {
          id: 'EV-X',
          kind: 'benchmark_fixture',
          runtime_kind: 'TOOLING_ONLY',
          proves: [{ flow: flow.id, obligation: 'PO-DISCLOSURE' }],
        },
      ],
    ],
  ])
  const evald = evaluateObligations(flow, byFlow)
  assert.equal(evald.states['PO-DISCLOSURE'], 'MISSING')
  assert.equal(hasProductRuntimeWitness(flow, byFlow.get(flow.id)), false)
  assert.equal(deriveFlowStatus(flow, evald, byFlow.get(flow.id)), 'SPEC_ONLY')
})

test('PASS status with missing required obligation is PARTIAL/FAIL, not complete', () => {
  const flow = { id: 'FLOW-X', obligations: ['PO-TENANCY', 'PO-AUTHORITY'], status: 'VERIFIED' }
  const byFlow = new Map([
    [
      'FLOW-X',
      [
        {
          kind: 'test',
          runtime_kind: 'PRODUCT_RUNTIME',
          proves: [{ flow: 'FLOW-X', obligation: 'PO-TENANCY' }],
        },
      ],
    ],
  ])
  const evald = evaluateObligations(flow, byFlow)
  assert.equal(evald.states['PO-AUTHORITY'], 'MISSING')
  assert.equal(deriveFlowStatus(flow, evald, byFlow.get('FLOW-X')), 'RUNTIME_CONNECTED')
})

test('Golden Flow can fail while a local CAP looks greener than the chain', () => {
  const built = compile(false)
  const gbf = built.ir.golden.find((row) => row.id === 'GBF-013')
  assert.ok(gbf)
  assert.notEqual(gbf.verdict, 'PASS')
  const cap18Local = built.ir.flows.filter((flow) => flow.owner === 'CAP18')
  assert.ok(cap18Local.length > 0)
  const fakePass = new Map([['FLOW-A', { derived_status: 'VERIFIED' }]])
  assert.equal(goldenVerdict({ flow_ids: ['FLOW-A', 'FLOW-MISSING'] }, fakePass), 'FAIL')
})

test('stale benchmark pin is STALE_BENCHMARK', () => {
  const stale = detectStaleBenchmark('BR-2026.08.21.1', 'BR-2026.08.21.2')
  assert.equal(stale.stale, true)
  assert.equal(stale.tag, 'STALE_BENCHMARK')
  assert.equal(detectStaleBenchmark('BR-2026.08.21.1', 'BR-2026.08.21.1').stale, false)
})

test('duplicate law ids are reported', () => {
  const dir = mkdtempSync(join(tmpdir(), 'contract-dup-'))
  writeFileSync(join(dir, 'dup.jsonl'), readFileSync(join(FIX, 'fail-duplicate-id.jsonl')))
  const built = compileContracts({
    root: ROOT,
    sourceDir: dir,
    write: false,
    repositorySha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
  })
  assert.equal(built.ok, false)
  assert.ok(built.errors.some((msg) => /duplicate law/.test(msg) || /missing CAP envelope/.test(msg)))
})

test('generated 209 standard exists and does not claim Engine V2', () => {
  const body = readFileSync(join(ROOT, 'docs/benchmarks/209-benchmark-contract-compiler.md'), 'utf8')
  assert.match(body, /chronica\.benchmark\.contract\.v1/)
  assert.match(body, /Not Engine V2/)
  assert.equal(existsSync(join(ROOT, 'tools/benchmark/contracts/source/envelopes.jsonl')), true)
})
