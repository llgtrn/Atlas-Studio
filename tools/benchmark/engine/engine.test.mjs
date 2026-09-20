import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'
import { loadJson, loadV1Denominator } from '../fic-recompute-lib.mjs'
import { bindProvenance, classifyFreshness, assertNotSilentShaSwap } from './provenance.mjs'
import { classifyProviderSurface, validateNoAiTaxonomy, engineProviderSplitFromFreeze } from './no-ai.mjs'
import { validateAuthorityDistinction } from './authority.mjs'
import { rejectCapabilityCountAsFic, missingRecoveryBlocksL3, guardFicDenominator } from './fic-guard.mjs'
import { detectDuplicateExecutionSubstrate } from './contracts.mjs'
import { evaluateInflationClaim, evaluateInflationClaims } from './anti-inflation.mjs'
import { evaluateAntiPattern } from './anti-patterns.mjs'
import { evaluateActorNeutralSubstrate } from './actor-neutral.mjs'
import { validateEvidencePath } from './evidence-graph.mjs'
import { recomputeFirstVertical } from './first-vertical.mjs'
import { renderTruthReport, validateReportMatchesSnapshot } from './report.mjs'
import { diffSnapshots, assertUnmixed } from './diff.mjs'
import { buildSnapshot } from './snapshot.mjs'
import { validatePlaneRecord, liveAuditToFivePlaneData } from './five-plane.mjs'
import { computeAndAdmit } from './family-admission.mjs'
import { loadFivePlaneAudit } from '../five-plane-audit-lib.mjs'
import { FIC_DENOMINATOR, FROZEN_VERDICT, SCHEMA } from './vocab.mjs'

const ROOT = fileURLToPath(new URL('../../../', import.meta.url))
const FIX = join(ROOT, 'tools/benchmark/engine/fixtures')
const SHA_A = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
const SHA_B = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'
const SHA_C = 'cccccccccccccccccccccccccccccccccccccccc'
const GENERATED_AT = '2026-08-21T00:00:00.000Z'

function loadFix(name) {
  return JSON.parse(readFileSync(join(FIX, name), 'utf8'))
}

test('exact SHA mismatch is STALE_BASELINE, never a silent SHA B report', () => {
  const provenance = bindProvenance({
    repository_sha: SHA_A,
    evaluated_sha: SHA_A,
    base_sha: SHA_B,
    product_truth_sha: null,
    generated_at: GENERATED_AT,
    tests_passing: true,
  })
  assert.equal(provenance.freshness, 'STALE')
  assert.equal(provenance.stale_reason, 'STALE_BASELINE')
  assert.equal(provenance.authoritative_architecture_verdict, false)
  const swap = assertNotSilentShaSwap({ provenance: { repository_sha: SHA_A } }, SHA_B)
  assert.equal(swap.ok, false)
})

test('unknown product-truth SHA is PROVISIONAL even if tests pass', () => {
  const provenance = bindProvenance({
    repository_sha: SHA_A,
    base_sha: SHA_A,
    product_truth_sha: null,
    generated_at: GENERATED_AT,
    tests_passing: true,
  })
  assert.equal(provenance.freshness, 'PROVISIONAL')
  assert.equal(provenance.stale_reason, 'PRODUCT_TRUTH_SHA_UNKNOWN')
  assert.equal(provenance.tests_passing_does_not_imply_authoritative, true)
})

test('malformed SHA is INVALID, not CURRENT', () => {
  const classified = classifyFreshness({
    repository_sha: 'not-a-sha',
    evaluated_sha: 'not-a-sha',
    base_sha: 'not-a-sha',
    product_truth_sha: null,
    benchmark_schema_version: SCHEMA,
    generator_version: 'x',
    generated_at: GENERATED_AT,
  })
  assert.equal(classified.freshness, 'INVALID')
})

test('CURRENT still is not an authoritative architecture verdict', () => {
  const provenance = bindProvenance({
    repository_sha: SHA_A,
    base_sha: SHA_A,
    product_truth_sha: SHA_A,
    generated_at: GENERATED_AT,
    tests_passing: true,
  })
  assert.equal(provenance.freshness, 'CURRENT')
  assert.equal(provenance.authoritative_architecture_verdict, false)
})

test('AI provider classified REQUIRED_INFERENCE', () => {
  const result = classifyProviderSurface({
    name: 'claude messages',
    provider_kind: 'AI_INFERENCE_PROVIDER',
    no_ai_class: 'REQUIRED_INFERENCE',
  })
  assert.equal(result.ok, true)
  assert.equal(result.no_ai_class, 'REQUIRED_INFERENCE')
})

test('deterministic effect provider classified STILL_WORKS_WITHOUT_AI', () => {
  const result = classifyProviderSurface({
    name: 'stripe approved refund',
    provider_kind: 'DETERMINISTIC_EXTERNAL_EFFECT_PROVIDER',
    no_ai_class: 'STILL_WORKS_WITHOUT_AI',
  })
  assert.equal(result.ok, true)
})

test('collapsed provider taxonomy fails closed', () => {
  const fixture = loadFix('collapsed-provider.json')
  const result = validateNoAiTaxonomy(fixture.surfaces)
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => /COLLAPSED_PROVIDER_TAXONOMY/.test(e)))
})

test('stripe cannot be classified as AI inference', () => {
  const result = classifyProviderSurface({
    name: 'stripe payout',
    provider_kind: 'AI_INFERENCE_PROVIDER',
    no_ai_class: 'REQUIRED_INFERENCE',
  })
  assert.equal(result.ok, false)
})

test('approval primitive does not imply execution grant', () => {
  const fixture = loadFix('approval-implies-grant.json')
  const result = validateAuthorityDistinction(fixture)
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => /approval primitive does not imply execution grant/.test(e)))
})

test('duplicate execution substrate is detected', () => {
  const fixture = loadFix('duplicate-workrun-contracts.json')
  const result = detectDuplicateExecutionSubstrate(fixture.contracts, fixture.relationships)
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => /DUPLICATE_EXECUTION_SUBSTRATE/.test(e)))
})

test('parent/consumes relationship allows one WorkRun family', () => {
  const result = detectDuplicateExecutionSubstrate(
    [
      { intent_id: 'CORE-EXECUTION-SPINE-001', required_state: 'WorkRequest/WorkRun/Attempt' },
      { intent_id: 'AI-DURABLE-001', required_state: 'WorkRequest/WorkRun consumer' },
    ],
    [
      { from: 'CORE-EXECUTION-SPINE-001', to: 'AI-DURABLE-001', relation: 'PARENT' },
      { from: 'AI-DURABLE-001', to: 'CORE-EXECUTION-SPINE-001', relation: 'CONSUMES' },
    ],
  )
  assert.equal(result.ok, true, result.errors.join('\n'))
})

test('fake product / CLI-only caller is rejected where product runtime is required', () => {
  const fixture = loadFix('inflated-claim.json')
  const result = evaluateInflationClaim(fixture)
  const codes = result.findings.map((row) => row.code)
  assert.ok(codes.includes('NON_PRODUCT_CALLER'))
  assert.ok(codes.includes('VALIDATE_ONLY_CREATE_CLAIM'))
  assert.ok(codes.includes('WORK_WITHOUT_DURABILITY'))
  assert.equal(result.findings.every((row) => row.demote === false), true)
})

test('validate-only create is rejected', () => {
  const result = evaluateInflationClaim({
    name: 'create_invoice',
    behavior: 'VALIDATE_ONLY',
    runtime_kind: 'VALIDATE_ONLY',
  })
  assert.ok(result.findings.some((row) => row.code === 'VALIDATE_ONLY_CREATE_CLAIM'))
})

test('external_write without effect is rejected', () => {
  const result = evaluateInflationClaim({ name: 'ledger.external_write', external_effect: false })
  assert.ok(result.findings.some((row) => row.code === 'SIDE_EFFECT_CLASS_MISMATCH'))
})

test('external effect without receipt is rejected', () => {
  const result = evaluateInflationClaim({ name: 'send_email', external_effect: true, receipt: false })
  assert.ok(result.findings.some((row) => row.code === 'EXECUTION_WITHOUT_RECEIPT'))
})

test('durable claim with memory-only queue is rejected', () => {
  const result = evaluateInflationClaims([
    { name: 'ai.durable_job', durable_job: true, persistence_kind: 'IN_MEMORY' },
  ])
  assert.ok(result.findings.some((row) => row.code === 'WORK_WITHOUT_DURABILITY'))
  assert.deepEqual(result.demoted, [])
})

test('anti-patterns require evidence objects, not keyword grep', () => {
  const grep = evaluateAntiPattern('AGENT_AS_KERNEL', { keyword_only: true, detected_by: 'grep', note: 'file mentions agent' })
  assert.equal(grep.ok, false)
  const found = evaluateAntiPattern('AGENT_AS_KERNEL', {
    business_state_progresses_only_because_agent_reasons: true,
    deterministic_kernel_path_exists: false,
    evidence_paths: ['crates/example.rs'],
  })
  assert.equal(found.ok, true)
  assert.equal(found.finding, 'FOUND')
})

test('actor-neutral check asks for a shared command/run boundary', () => {
  const result = evaluateActorNeutralSubstrate({
    actors: { human: true, agent: true, API: true, scheduler: true, webhook: true, system: true },
    shared_deterministic_command_run_boundary: false,
    agent_special_path: true,
    human_uses_same_boundary: false,
    durable_job_claimed: true,
    persistence_kind: 'IN_MEMORY',
  })
  assert.equal(result.peer, false)
  assert.ok(result.findings.includes('WORK_WITHOUT_DURABILITY'))
  assert.ok(result.findings.includes('AGENT_SPECIAL_PATH_NOT_PEER'))
})

test('evidence graph distinguishes VALIDATE_ONLY from PRODUCT_RUNTIME', () => {
  const result = validateEvidencePath({
    claim_id: 'create_widget',
    runtime_kind: 'VALIDATE_ONLY',
    nodes: { entrypoint: 'POST /validate' },
  })
  assert.equal(result.mismatch, 'VALIDATE_ONLY_CREATE_CLAIM')
  assert.equal(result.runtime_kind, 'VALIDATE_ONLY')
})

test('contract existence cannot promote first-vertical runtime status', () => {
  const fixture = loadFix('contract-promoted-edge.json')
  const result = recomputeFirstVertical(fixture)
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => /CONTRACT_IS_NOT_RUNTIME|cannot promote runtime/.test(e)))
})

test('first-vertical graph is recomputable with engine fields', () => {
  const live = loadJson(join(ROOT, 'docs/_machine/first-vertical-gap-graph.json'))
  const result = recomputeFirstVertical(live)
  assert.equal(result.total, live.edges.length)
  assert.ok(result.edges.every((edge) => edge.edge_id && edge.producer && edge.consumer && edge.current_status))
  assert.equal(result.edges[0].current_status, live.edges[0].status)
})

test('FIC denominator cannot silently change', () => {
  const v1 = loadV1Denominator(ROOT)
  const census = loadJson(join(ROOT, 'docs/_machine/fic-revalidation-census.json'))
  const guard = guardFicDenominator(v1, census)
  assert.equal(guard.fic.denominator, FIC_DENOMINATOR)
  const drifted = { ...census, domains: census.domains.slice(0, 54) }
  const bad = guardFicDenominator(v1, drifted)
  assert.equal(bad.ok, false)
  assert.ok(bad.errors.some((e) => /55/.test(e)))
})

test('capability count cannot substitute FIC', () => {
  const result = rejectCapabilityCountAsFic({
    as: 'FIC',
    source: 'capability_verified_count',
    covered: 1290,
    denominator: 5151,
    verified_capability_count: 1290,
    label: 'FIC',
  })
  assert.equal(result.ok, false)
})

test('missing recovery prevents FIC L3', () => {
  const result = missingRecoveryBlocksL3({
    domain: 'durable-job-queue',
    previous_level: 'L1',
    evidence_sha: SHA_A,
    owner: 'jobs',
    runtime_entry: 'HTTP',
    caller: 'api',
    business_consumer: 'operator',
    state_boundary: 'jobs table',
    external_effect: 'none',
    failure_path: 'lease expiry',
    recovery_boundary: 'MISSING',
    security_boundary: 'tenant',
    tests: { positive: ['ok'], negative: ['deny'] },
    telemetry: 'trace',
    operations: 'owner jobs',
    new_level: 'L3',
    level_changed: true,
    reason: 'Shipped queue without recovery.',
    remaining_gap: 'restore',
    next_required_evidence: 'restore drill',
  })
  assert.equal(result.ok, false)
})

test('Markdown cannot disagree with machine JSON', () => {
  const snapshot = {
    schema: SCHEMA,
    provenance: {
      repository_sha: SHA_A,
      evaluated_sha: SHA_A,
      base_sha: SHA_A,
      product_truth_sha: null,
      benchmark_schema_version: SCHEMA,
      generator_version: 'chronica.benchmark-truth-engine.v1',
      generated_at: GENERATED_AT,
      freshness: 'PROVISIONAL',
      stale_reason: 'PRODUCT_TRUTH_SHA_UNKNOWN',
    },
    frozen_architecture: FROZEN_VERDICT,
    fic: { l3_or_higher: 6, denominator: 55, not_capability_inventory: true, pec: 'LOCAL_AUDIT_REQUIRED', cse: 'UNKNOWN_NOT_PROVEN' },
    finding_drift: [],
  }
  const md = renderTruthReport(snapshot)
  assert.equal(validateReportMatchesSnapshot(md, snapshot).ok, true)
  const edited = md.replace('FIC: 6/55', 'FIC: 1290/5151')
  const bad = validateReportMatchesSnapshot(edited, snapshot)
  assert.equal(bad.ok, false)
})

test('five-plane records forbid numeric precision', () => {
  const result = validatePlaneRecord({
    domain: 'helpdesk',
    plane: 'DETERMINISTIC_KERNEL',
    assessment: 'STRONG',
    assessment_score: 0.91,
    evidence_paths: ['crates/chronica-helpdesk/src/lib.rs'],
    confidence: 'HIGH',
  })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => /numeric precision/.test(e)))
})

test('diff engine separates CODE/EVIDENCE/CLASSIFICATION/DENOMINATOR', () => {
  const previous = {
    provenance: { repository_sha: SHA_A, freshness: 'PROVISIONAL' },
    fic: { denominator: 55, l3_or_higher: 6 },
    first_vertical: {
      edges: [{ edge_id: 'E1', current_status: 'MISSING', evidence: 'none', runtime_owner: 'UNSTATED' }],
    },
    contracts: { errors: ['DUPLICATE_EXECUTION_SUBSTRATE: x'] },
    families: { family_count: 11 },
  }
  const current = {
    provenance: { repository_sha: SHA_B, freshness: 'STALE' },
    fic: { denominator: 55, l3_or_higher: 6 },
    first_vertical: {
      edges: [{ edge_id: 'E1', current_status: 'LIVE', evidence: 'crates/api.rs', runtime_owner: 'chronica-api' }],
    },
    contracts: { errors: [] },
    families: { family_count: 11 },
  }
  const diff = diffSnapshots(previous, current)
  assert.ok(diff.classes.CODE_CHANGE.some((row) => row.field === 'repository_sha'))
  assert.ok(diff.classes.EVIDENCE_CHANGE.some((row) => row.field === 'edge.E1.evidence'))
  assert.ok(diff.classes.CLASSIFICATION_CHANGE.some((row) => row.field === 'edge.E1.status'))
  assert.equal(diff.classes.DENOMINATOR_CHANGE.length, 0)
  assert.ok(diff.summary.newly_proven.includes('E1'))
  assert.ok(diff.summary.stale.length > 0)
  assert.ok(diff.summary.contracts_satisfied.length > 0)
  assert.equal(assertUnmixed(diff).ok, true)
})

test('denominator change is not mixed into classification', () => {
  const diff = diffSnapshots(
    { fic: { denominator: 55, l3_or_higher: 6 }, families: { family_count: 11 }, provenance: {}, contracts: { errors: [] }, first_vertical: { edges: [] } },
    { fic: { denominator: 56, l3_or_higher: 6 }, families: { family_count: 11 }, provenance: {}, contracts: { errors: [] }, first_vertical: { edges: [] } },
  )
  assert.equal(diff.classes.DENOMINATOR_CHANGE.length, 1)
  assert.equal(diff.classes.CLASSIFICATION_CHANGE.some((row) => row.field === 'fic.denominator'), false)
})

test('live snapshot stays frozen B / 6/55 and binds SHA', () => {
  const built = buildSnapshot({
    root: ROOT,
    generated_at: GENERATED_AT,
    repository_sha: SHA_A,
    base_sha: SHA_B,
    product_truth_sha: null,
    skip_families: true,
  })
  assert.equal(built.snapshot.schema, SCHEMA)
  assert.equal(built.snapshot.frozen_architecture.verdict, 'B')
  assert.equal(built.snapshot.frozen_architecture.verdict_label, 'HARNESS-HEAVY HYBRID')
  assert.equal(built.snapshot.frozen_architecture.authoritative_architecture_verdict, false)
  assert.equal(built.snapshot.fic.l3_or_higher, 6)
  assert.equal(built.snapshot.fic.denominator, 55)
  assert.equal(built.snapshot.fic.not_capability_inventory, true)
  assert.equal(built.snapshot.provenance.freshness, 'STALE')
  assert.equal(built.snapshot.finding_drift.length, 0)
  const split = engineProviderSplitFromFreeze(loadFivePlaneAudit(ROOT).freeze)
  assert.equal(split[0].no_ai_class, 'REQUIRED_INFERENCE')
  assert.equal(split[1].no_ai_class, 'STILL_WORKS_WITHOUT_AI')
  const plane = liveAuditToFivePlaneData(loadFivePlaneAudit(ROOT), { no_ai_surfaces: split })
  assert.ok(plane.plane_records.some((row) => row.domain === 'helpdesk' && row.plane === 'DETERMINISTIC_KERNEL'))
})

test('11-family admission still uses the existing ratio engine', () => {
  const { ratio, admission } = computeAndAdmit()
  assert.equal(ratio.ratios.total_families, 11)
  assert.equal(admission.ok, true, admission.errors.join('\n'))
  assert.equal(ratio.wave_admission_ready, false)
})

test('CLI emits freshness and does not claim authority', () => {
  const { status, stdout, stderr } = spawnSync(process.execPath, ['tools/benchmark/engine/run.mjs', '--generated-at', GENERATED_AT, '--repository-sha', SHA_A], {
    encoding: 'utf8',
    cwd: ROOT,
  })
  assert.equal(status, 0, stderr)
  assert.match(stdout, /freshness=STALE|freshness=PROVISIONAL/)
  assert.match(stdout, /authoritative=false/)
  assert.match(stdout, /FIC=6\/55/)
})
