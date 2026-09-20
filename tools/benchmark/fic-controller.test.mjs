import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdtempSync, writeFileSync, mkdirSync, readFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import {
  applyLevelRules,
  computeFic,
  denominatorKeys,
  loadV1Denominator,
  validateCensusShape,
} from './fic-recompute-lib.mjs'
import {
  checkPortfolioAndLedger,
  classifyLicense,
  classifyPin,
  portfolioMinimumForFamily,
  validateLedgerEntry,
  validatePortfolioEntry,
} from './benchmark-pin-check-lib.mjs'
import { validateIntentContract, loadContracts } from './intent-contract-validate.mjs'
import { validateFirstVertical, EDGE_STATUSES } from './first-vertical-gap.mjs'
import { validateTopology } from './benchmark-source-topology.mjs'
import { loadMap, validateFamilyToFicMap } from './benchmark-to-fic-map.mjs'
import { computeBenchmarkRatio, loadRegistry } from './query-benchmark-ratio.mjs'
import { recompute } from './fic-recompute.mjs'
import {
  loadFivePlaneAudit,
  validateFivePlaneAudit,
  REQUIRED_NO_AI_DOMAINS,
  ANTI_PATTERN_TAGS,
} from './five-plane-audit-lib.mjs'

const ROOT = fileURLToPath(new URL('../../', import.meta.url))

function liveContracts() {
  return loadContracts(join(ROOT, 'docs/_machine/intent-contracts')).map((row) => row.contract)
}

function tmp() {
  return mkdtempSync(join(tmpdir(), 'fic-ctrl-'))
}

function writeJsonl(dir, name, rows) {
  const path = join(dir, name)
  writeFileSync(path, rows.map((r) => JSON.stringify(r)).join('\n') + '\n')
  return path
}

const BASE_ROW = {
  domain: 'tenancy-isolation',
  previous_level: 'L2',
  evidence_sha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
  owner: 'chronica-api',
  runtime_entry: 'HTTP company routes',
  caller: 'chronica-api handlers',
  business_consumer: 'tenant product surfaces',
  state_boundary: 'company_id columns',
  external_effect: 'none',
  failure_path: 'cross-tenant 404',
  recovery_boundary: 'fail-closed deny after restart',
  security_boundary: 'assert_company_access',
  tests: { positive: ['iso_ok'], negative: ['cross_tenant'] },
  telemetry: 'tracing on deny',
  operations: 'code owner chronica-api',
  new_level: 'L3',
  level_changed: true,
  reason: 'Shipped tenant gate with positive and negative isolation tests.',
  remaining_gap: 'on-call',
  next_required_evidence: 'operator catalog',
}

test('fixed denominator: v1.json still has exactly 55 domains and no L7', () => {
  const v1 = loadV1Denominator(ROOT)
  const keys = denominatorKeys(v1)
  assert.equal(keys.length, 55)
  assert.equal(new Set(keys).size, 55)
  for (const d of v1.domains) {
    assert.match(d.level, /^L[0-6]$/)
  }
})

test('live census: 55 rows, schema valid, no skipped/duplicate domains', () => {
  const result = recompute({ root: ROOT })
  assert.equal(result.shape.ok, true, result.shape.errors.join('\n'))
  assert.equal(result.census.domains.length, 55)
  assert.equal(result.fic.denominator, 55)
  assert.equal(result.fic.cse, 'UNKNOWN_NOT_PROVEN')
  const sum = Object.values(result.fic.counts).reduce((a, b) => a + b, 0)
  assert.equal(sum, 55)
  assert.equal(result.fic.counts.L3, 6)
  assert.equal(result.fic.l3_or_higher, 6)
})

test('duplicate domain fails census validation', () => {
  const v1 = loadV1Denominator(ROOT)
  const census = {
    schema: 'chronica.fic_revalidation_census.v1',
    domains: v1.domains.map((d) => ({
      ...BASE_ROW,
      domain: d.key,
      previous_level: d.level,
      new_level: 'L1',
      runtime_entry: 'MISSING',
      caller: 'none',
      recovery_boundary: 'MISSING',
      level_changed: d.level !== 'L1',
      reason: 'fixture',
    })),
  }
  census.domains.push({ ...census.domains[0] })
  const result = validateCensusShape(census, v1)
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e.startsWith('duplicate_domain:')))
})

test('invalid FIC promotion L3 without caller/recovery is rejected', () => {
  const promoted = applyLevelRules({
    ...BASE_ROW,
    caller: 'ISLAND no shipped caller',
    recovery_boundary: 'MISSING',
    new_level: 'L3',
    reason: 'crate exists therefore L3',
  })
  assert.equal(promoted.ok, false)
  assert.ok(promoted.errors.some((e) => e.includes('invalid_fic_promotion_L3') || e.includes('hand_wavy')))
  assert.notEqual(promoted.effective_level, 'L3')
})

test('missing owner/caller/recovery cannot stay L3', () => {
  const r = applyLevelRules({
    ...BASE_ROW,
    owner: '',
    caller: '',
    recovery_boundary: '',
    new_level: 'L3',
    reason: 'tests pass therefore functionally covered',
  })
  assert.equal(r.ok, false)
  assert.ok(r.l3_blockers.includes('missing_owner'))
  assert.ok(r.l3_blockers.includes('missing_caller') || r.l3_blockers.includes('caller_not_shipped'))
})

test('capability-count is not FIC promotion', () => {
  const r = applyLevelRules({
    ...BASE_ROW,
    capability_count: 64,
    new_level: 'L3',
    reason: 'because verified capability count is 37 of 64',
  })
  assert.equal(r.ok, false)
  assert.ok(r.errors.includes('capability_count_used_as_fic_promotion'))
})

test('computeFic never treats L2 as covered', () => {
  const fic = computeFic({
    pec: 'LOCAL_AUDIT_REQUIRED',
    cse: 'UNKNOWN_NOT_PROVEN',
    domains: [
      { ...BASE_ROW, new_level: 'L2', previous_level: 'L2', level_changed: false, reason: 'partial' },
    ],
  })
  assert.equal(fic.l3_or_higher, 0)
  assert.equal(fic.counts.L2, 1)
})

test('missing benchmark pin and floating branch fail closed', () => {
  assert.equal(classifyPin({}).status, 'PIN_REQUIRED')
  assert.equal(classifyPin({ pinned_sha: 'main' }).status, 'PIN_REQUIRED')
  assert.equal(classifyPin({ pinned_sha: 'deadbeef', branch: 'master' }).status, 'PIN_REQUIRED')
  assert.equal(classifyPin({ pinned_sha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' }).status, 'PINNED')
})

test('license missing fails closed', () => {
  assert.equal(classifyLicense({}).status, 'LICENSE_BLOCKED')
  assert.equal(classifyLicense({ license: 'MIT' }).status, 'LICENSE_BLOCKED')
  assert.equal(classifyLicense({ license: 'MIT', license_file: 'LICENSE' }).status, 'LICENSE_KNOWN')
})

test('duplicate benchmark target is reported', () => {
  const dir = tmp()
  mkdirSync(join(dir, 'docs/_machine'), { recursive: true })
  const entry = {
    schema: 'chronica.world_class_benchmark_portfolio.v1',
    id: 'dup',
    repository: 'foo/bar',
    pinned_sha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    license: 'MIT',
    license_file: 'LICENSE',
    source_class: 'external_implementation',
  }
  writeJsonl(join(dir, 'docs/_machine'), 'world-class-benchmark-portfolio.jsonl', [entry, entry])
  writeJsonl(join(dir, 'docs/_machine'), 'reference-repo-grounding-ledger.jsonl', [])
  const result = checkPortfolioAndLedger(dir, { requireLedger094: false })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e.includes('duplicate_benchmark_target:dup')))
})

test('portfolio minimum fails when standard/baseline missing', () => {
  const family = { money_adjacent: false, family_key: 'helpdesk_platform' }
  const matched = [
    { source_class: 'external_implementation', repository: 'chatwoot/chatwoot', architectural_lineage: 'a' },
    { source_class: 'external_implementation', repository: 'zammad/zammad', architectural_lineage: 'b' },
  ]
  const min = portfolioMinimumForFamily(family, matched)
  assert.equal(min.meets_minimum, false)
  assert.ok(min.gaps.some((g) => g.startsWith('standard')))
  assert.ok(min.gaps.some((g) => g.startsWith('internal_baseline')))
})

test('strategic portfolio minimum requires adversarial', () => {
  const family = { money_adjacent: true, family_key: 'fintech_core' }
  const matched = [
    { source_class: 'external_implementation', repository: 'a/a', architectural_lineage: 'x' },
    { source_class: 'external_implementation', repository: 'b/b', architectural_lineage: 'y' },
    { source_class: 'external_implementation', repository: 'c/c', architectural_lineage: 'x' },
    { source_class: 'standard_protocol', repository: 'spec/spec' },
    { source_class: 'internal_baseline', repository: 'llgtrn/chronica' },
  ]
  const min = portfolioMinimumForFamily(family, matched)
  assert.equal(min.meets_minimum, false)
  assert.ok(min.gaps.some((g) => g.startsWith('adversarial')))
})

test('INFORMATIONAL misuse on money-class contract is rejected', () => {
  const contract = {
    intent_id: 'MONEY-FAKE-001',
    fic_domains: ['commerce'],
    benchmark_families: ['commerce_platform'],
    benchmark_sources: ['x'],
    goal: 'Capture a payment without saying so',
    user_value: 'checkout',
    current_chronica: 'none',
    gap: 'payments',
    owner: 'chronica-commerce',
    expected_runtime_entry: 'POST /pay',
    expected_business_consumer: 'merchant',
    required_state: 'order',
    required_authority: 'none',
    required_invariants: ['money moves'],
    required_failure_behavior: 'retry',
    required_recovery: 'replay',
    security_class: 'HIGH',
    money_class: 'MONEY_ADJACENT',
    external_io: 'stripe',
    non_goals: ['not a goal'],
    positive_tests: ['ok'],
    negative_tests: ['bad'],
    product_witness: 'user sees charged card',
    benchmark_posture: 'INFORMATIONAL',
    done_when: 'never',
  }
  const result = validateIntentContract(contract, { denominatorKeys: ['commerce'], familyKeys: ['commerce_platform'] })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e.includes('INFORMATIONAL misuse')))
})

test('fake product witness is rejected', () => {
  const result = validateIntentContract({
    intent_id: 'FAKE-WITNESS-001',
    fic_domains: ['helpdesk'],
    benchmark_families: [],
    benchmark_sources: ['x'],
    goal: 'Ship a conversation',
    user_value: 'agent sees ticket',
    current_chronica: 'none',
    gap: 'gap',
    owner: 'chronica-helpdesk',
    expected_runtime_entry: 'GET /t',
    expected_business_consumer: 'agent',
    required_state: 'row',
    required_authority: 'company',
    required_invariants: ['tenant'],
    required_failure_behavior: '404',
    required_recovery: 'outbox',
    security_class: 'TENANT_SCOPED',
    money_class: 'MONEY_FREE',
    external_io: 'none',
    non_goals: ['zendesk-class'],
    positive_tests: ['ok'],
    negative_tests: ['bad'],
    product_witness: 'tests pass and crate exists',
    benchmark_posture: 'MUST_MATCH',
    done_when: 'later',
  })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e.includes('fake product witness')))
})

test('generic improve-goal is rejected', () => {
  const result = validateIntentContract({
    intent_id: 'AI-GATEWAY-001',
    fic_domains: ['ai-provider-gateway'],
    benchmark_families: [],
    benchmark_sources: ['x'],
    goal: 'Improve AI gateway',
    user_value: 'better',
    current_chronica: 'island',
    gap: 'gap',
    owner: 'x',
    expected_runtime_entry: 'x',
    expected_business_consumer: 'x',
    required_state: 'x',
    required_authority: 'x',
    required_invariants: ['x'],
    required_failure_behavior: 'x',
    required_recovery: 'x',
    security_class: 'LOW',
    money_class: 'MONEY_FREE',
    external_io: 'x',
    non_goals: ['x'],
    positive_tests: ['x'],
    negative_tests: ['x'],
    product_witness: 'a user sees a receipt id that is not the internal work id',
    benchmark_posture: 'MUST_MATCH',
    done_when: 'x',
  })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e.includes('generic')))
})

test('stale floating pin on ledger is PIN_REQUIRED', () => {
  const errors = validateLedgerEntry({
    schema: 'chronica.reference_repo_grounding_ledger.v1',
    source_id: 'stale',
    repository: 'foo/bar',
    pinned_sha: 'main',
    license: 'MIT',
    license_files_read: ['LICENSE'],
    exact_paths_read: ['LICENSE'],
    intended_use: 'x',
    integration_mode: 'REFERENCE_ONLY',
    copy_data_model_boundary: 'x',
    behavior_and_failures_extracted: 'x',
    adopted: ['x'],
    rejected: ['x'],
    chronica_owner_caller_consumer: 'x',
    money_security_regulatory_effect: 'x',
    what_remains_unproven: 'x',
    verdict: 'PIN_REQUIRED',
  })
  assert.ok(errors.some((e) => e.includes('PIN_REQUIRED')))
})

test('benchmark family mapping drift is detected', () => {
  const registry = loadRegistry()
  const v1 = loadV1Denominator(ROOT)
  const doc = loadMap()
  const ok = validateFamilyToFicMap(doc, {
    familyKeys: registry.families.map((f) => f.family_key),
    ficKeys: denominatorKeys(v1),
  })
  assert.equal(ok.ok, true, ok.errors.join('\n'))
  const drifted = validateFamilyToFicMap(
    { schema: 'chronica.benchmark_family_to_fic_map.v1', families: { not_a_family: ['helpdesk'] } },
    { familyKeys: registry.families.map((f) => f.family_key), ficKeys: denominatorKeys(v1) },
  )
  assert.equal(drifted.ok, false)
  assert.ok(drifted.errors.some((e) => e.startsWith('unknown_family:')))
  assert.ok(drifted.errors.some((e) => e.startsWith('skipped_family:')))
})

test('live topology and first-vertical validate', () => {
  const topo = JSON.parse(readFileSync(join(ROOT, 'docs/_machine/benchmark-source-topology.json'), 'utf8'))
  const t = validateTopology(topo)
  assert.equal(t.ok, true, t.errors.join('\n'))
  const fv = JSON.parse(readFileSync(join(ROOT, 'docs/_machine/first-vertical-gap-graph.json'), 'utf8'))
  const v = validateFirstVertical(fv)
  assert.equal(v.ok, true, v.errors.join('\n'))
  assert.equal(fv.edges.length, 14)
  assert.equal(fv.roadmap_priority_change, null)
  assert.equal(fv.board_authority_decision.authority_model, 'EXISTING_MODEL_SUFFICIENT')
  assert.equal(fv.board_authority_decision.execution_grant_bridge, 'NOT_YET_PROVEN')
  for (const edge of fv.edges) assert.ok(EDGE_STATUSES.includes(edge.status))
})

test('live intent contracts validate and have unique ids', () => {
  const { status, stdout, stderr } = spawnSync(process.execPath, ['tools/benchmark/intent-contract-validate.mjs'], {
    encoding: 'utf8',
    cwd: ROOT,
  })
  assert.equal(status, 0, stdout + stderr)
})

test('11-family ratio still has denominator 11 and no ProductionReady/WAVE_ADMISSION_READY', () => {
  const result = computeBenchmarkRatio()
  assert.equal(result.ratios.total_families, 11)
  assert.equal(result.wave_admission_ready, false)
  assert.equal(result.ratios.production_evidence_proven.count, 0)
  assert.equal(result.ratios.runtime_closure_live.count, 0)
  assert.equal(result.ratios.claim_ready_external_class_or_better.count, 0)
  for (const family of result.families) {
    assert.notEqual(family.claim_level_reached, 'ProductionReady')
    assert.notEqual(family.claim_level_reached, 'ExternalClass')
  }
})

test('portfolio entry with unknown source_class fails', () => {
  const errors = validatePortfolioEntry({
    schema: 'chronica.world_class_benchmark_portfolio.v1',
    id: 'x',
    repository: 'a/b',
    pinned_sha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    license: 'MIT',
    license_file: 'LICENSE',
    source_class: 'famous_vendor',
  })
  assert.ok(errors.some((e) => e.includes('invalid source_class')))
})

test('CLI fic-recompute exits 0 on live census', () => {
  const { status, stdout, stderr } = spawnSync(process.execPath, ['tools/benchmark/fic-recompute.mjs'], {
    encoding: 'utf8',
    cwd: ROOT,
  })
  assert.equal(status, 0, stderr)
  assert.match(stdout, /FIC=6\/55/)
})

test('live five-plane audit validates and does not change the 55-domain FIC denominator', () => {
  const audit = loadFivePlaneAudit(ROOT)
  assert.ok(audit)
  const result = validateFivePlaneAudit(audit, { contracts: liveContracts() })
  assert.equal(result.ok, true, result.errors.join('\n'))
  assert.equal(audit.verdict, 'B')
  assert.equal(audit.verdict_label, 'HARNESS-HEAVY HYBRID')
  assert.equal(audit.freeze.five_plane_audit_status, 'PROVISIONAL_BASELINE')
  assert.equal(audit.freeze.authoritative_architecture_verdict, false)
  assert.equal(audit.freeze.benchmark_controller_status, 'FROZEN_PROVISIONAL_BASELINE')
  assert.equal(audit.freeze.core_spine, 'PROVISIONALLY_REQUIRED')
  assert.equal(audit.freeze.rerun_trigger, 'POST_THREAD4_PRODUCT_TRUTH_MAIN')
  assert.equal(audit.freeze.until_then, 'NO_NEW_BENCHMARK_ARCHITECTURE_WORK')
  assert.equal(audit.freeze.no_new_benchmark_architecture_work, true)
  assert.ok(audit.freeze.after_product_truth_main_sha.includes('reassess CORE-EXECUTION-SPINE-001'))
  assert.equal(audit.spine_decision.verdict, 'REQUIRED')
  assert.equal(audit.spine_decision.intent_id, 'CORE-EXECUTION-SPINE-001')
  assert.ok(audit.spine_decision.generalizes.includes('AI-DURABLE-001'))
  assert.equal(audit.not_a_denominator, true)
  assert.equal(audit.no_ai_matrix.length, REQUIRED_NO_AI_DOMAINS.length)
  assert.equal(audit.anti_patterns.length, ANTI_PATTERN_TAGS.length)
  const helpdesk = audit.no_ai_matrix.find((d) => d.domain === 'helpdesk')
  assert.equal(helpdesk.no_ai, 'STILL_WORKS_WITHOUT_AI')
  const providers = audit.no_ai_matrix.find((d) => d.domain === 'external-provider-execution')
  assert.equal(providers.taxonomy_status, 'BUNDLED_PROVIDER_SURFACES_MUST_SPLIT_ON_RERUN')
  const rewrite = audit.anti_patterns.find((t) => t.tag === 'LANGUAGE_REWRITE_THEATER')
  assert.equal(rewrite.finding, 'REJECTED')
  const v1 = loadV1Denominator(ROOT)
  assert.equal(denominatorKeys(v1).length, 55)
})

test('five-plane CLI exits 0 on live audit', () => {
  const { status, stdout, stderr } = spawnSync(process.execPath, ['tools/benchmark/five-plane-audit.mjs'], {
    encoding: 'utf8',
    cwd: ROOT,
  })
  assert.equal(status, 0, stdout + stderr)
  assert.match(stdout, /verdict=B HARNESS-HEAVY HYBRID/)
  assert.match(stdout, /FROZEN_PROVISIONAL_BASELINE/)
})

test('missing NO_AI domain fails closed', () => {
  const audit = structuredClone(loadFivePlaneAudit(ROOT))
  audit.no_ai_matrix = audit.no_ai_matrix.filter((d) => d.domain !== 'helpdesk')
  const result = validateFivePlaneAudit(audit, { contracts: liveContracts() })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e === 'missing_no_ai_domain:helpdesk'))
})

test('helpdesk ticket transition cannot require AI to progress business state', () => {
  const audit = structuredClone(loadFivePlaneAudit(ROOT))
  const helpdesk = audit.no_ai_matrix.find((d) => d.domain === 'helpdesk')
  helpdesk.no_ai = 'REQUIRES_AI_TO_PROGRESS_BUSINESS_STATE'
  helpdesk.operations[0].no_ai = 'REQUIRES_AI_TO_PROGRESS_BUSINESS_STATE'
  const result = validateFivePlaneAudit(audit, { contracts: liveContracts() })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e.includes('helpdesk') && e.includes('must not require AI')))
})

test('LANGUAGE_REWRITE_THEATER FOUND is rejected', () => {
  const audit = structuredClone(loadFivePlaneAudit(ROOT))
  const row = audit.anti_patterns.find((t) => t.tag === 'LANGUAGE_REWRITE_THEATER')
  row.finding = 'FOUND'
  const result = validateFivePlaneAudit(audit, { contracts: liveContracts() })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e.includes('LANGUAGE_REWRITE_THEATER')))
})

test('numeric plane scores are forbidden', () => {
  const audit = structuredClone(loadFivePlaneAudit(ROOT))
  audit.kernel_score = 4.2
  const result = validateFivePlaneAudit(audit, { contracts: liveContracts() })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e.startsWith('numeric_precision_forbidden:')))
})

test('language bake-off prose is forbidden', () => {
  const audit = structuredClone(loadFivePlaneAudit(ROOT))
  audit.verdict_rationale = 'Chronica should rewrite in Go vs Rust for throughput.'
  const result = validateFivePlaneAudit(audit, { contracts: liveContracts() })
  assert.equal(result.ok, false)
  assert.ok(result.errors.includes('language_bakeoff_forbidden'))
})

test('invalid architecture verdict fails closed', () => {
  const audit = structuredClone(loadFivePlaneAudit(ROOT))
  audit.verdict = 'E'
  audit.verdict_label = 'SOMETHING ELSE'
  const result = validateFivePlaneAudit(audit, { contracts: liveContracts() })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e.includes('verdict must be one of A|B|C|D')))
})

test('EXISTING_SPINE_SUFFICIENT without runtime evidence fails', () => {
  const audit = structuredClone(loadFivePlaneAudit(ROOT))
  audit.spine_decision = {
    verdict: 'EXISTING_SPINE_SUFFICIENT',
    evidence: 'because crates exist',
    evidence_files: ['crates/chronica-workflows/src/lib.rs'],
  }
  const result = validateFivePlaneAudit(audit, { contracts: liveContracts() })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e.includes('EXISTING_SPINE_SUFFICIENT')))
})

test('REQUIRED spine must generalize AI-DURABLE-001', () => {
  const audit = structuredClone(loadFivePlaneAudit(ROOT))
  audit.spine_decision.generalizes = ['NEW-TEMPORAL-CLONE-001']
  const result = validateFivePlaneAudit(audit, { contracts: liveContracts() })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e.includes('AI-DURABLE-001')))
})

test('AGENT_AS_KERNEL FOUND without kernel proof fails', () => {
  const audit = structuredClone(loadFivePlaneAudit(ROOT))
  const row = audit.anti_patterns.find((t) => t.tag === 'AGENT_AS_KERNEL')
  row.finding = 'FOUND'
  delete row.business_state_progresses_only_because_agent_reasons
  const result = validateFivePlaneAudit(audit, { contracts: liveContracts() })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e.includes('AGENT_AS_KERNEL')))
})

test('CORE-EXECUTION-SPINE-001 contract is present and cites AI-DURABLE-001', () => {
  const contracts = liveContracts()
  const spine = contracts.find((c) => c.intent_id === 'CORE-EXECUTION-SPINE-001')
  const durable = contracts.find((c) => c.intent_id === 'AI-DURABLE-001')
  assert.ok(spine)
  assert.ok(durable)
  assert.equal(spine.assignment, 'DEFER')
  assert.equal(spine.relationship_to_ai_durable, 'PARENT_OR_SUPERSEDE')
  assert.ok(spine.generalizes.includes('AI-DURABLE-001'))
  assert.equal(durable.related_spine, 'CORE-EXECUTION-SPINE-001')
  assert.equal(durable.role, 'CONSUMER_USE_CASE')
  assert.equal(durable.do_not_create_independent_workrun_schema, true)
  assert.equal(durable.shared_substrate_implementation, 'DEFER_UNTIL_POST_THREAD4_RERUN')
  const validated = validateIntentContract(spine, {
    denominatorKeys: denominatorKeys(loadV1Denominator(ROOT)),
    familyKeys: loadRegistry().families.map((f) => f.family_key),
  })
  assert.equal(validated.ok, true, validated.errors.join('\n'))
})

test('missing freeze is not an authoritative architecture verdict', () => {
  const audit = structuredClone(loadFivePlaneAudit(ROOT))
  delete audit.freeze
  const result = validateFivePlaneAudit(audit, { contracts: liveContracts() })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e.includes('PROVISIONAL_BASELINE')))
})

test('authoritative_architecture_verdict true fails closed until Thread 4', () => {
  const audit = structuredClone(loadFivePlaneAudit(ROOT))
  audit.freeze.authoritative_architecture_verdict = true
  const result = validateFivePlaneAudit(audit, { contracts: liveContracts() })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e.includes('authoritative_architecture_verdict')))
})

test('CORE-EXECUTION-SPINE-001 Builder A assignment is frozen', () => {
  const audit = structuredClone(loadFivePlaneAudit(ROOT))
  const contracts = liveContracts().map((c) =>
    c.intent_id === 'CORE-EXECUTION-SPINE-001' ? { ...c, assignment: 'Builder A' } : c,
  )
  const result = validateFivePlaneAudit(audit, { contracts })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e.includes('DEFER')))
})

test('an eighth architecture contract is rejected while frozen', () => {
  const audit = structuredClone(loadFivePlaneAudit(ROOT))
  const contracts = [...liveContracts(), { intent_id: 'NEW-SPINE-CLONE-001' }]
  const result = validateFivePlaneAudit(audit, { contracts })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e.includes('no new architecture contracts')))
})

test('until_then and post-Thread4 rerun checklist are fail-closed', () => {
  const audit = structuredClone(loadFivePlaneAudit(ROOT))
  audit.freeze.until_then = 'CONTINUE_BENCHMARK'
  audit.freeze.after_product_truth_main_sha = []
  const result = validateFivePlaneAudit(audit, { contracts: liveContracts() })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => e.includes('NO_NEW_BENCHMARK_ARCHITECTURE_WORK')))
  assert.ok(result.errors.some((e) => e.includes('reassess CORE-EXECUTION-SPINE-001')))
})

test('bundled provider inference without split correction fails freeze', () => {
  const audit = structuredClone(loadFivePlaneAudit(ROOT))
  delete audit.freeze.rerun_corrections.no_ai_provider_split
  const result = validateFivePlaneAudit(audit, { contracts: liveContracts() })
  assert.equal(result.ok, false)
  assert.ok(result.errors.some((e) => /split AI inference/i.test(e)))
})
