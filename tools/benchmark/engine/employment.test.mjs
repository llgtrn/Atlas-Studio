import test from 'node:test'
import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { loadV1Denominator, loadJson } from '../fic-recompute-lib.mjs'
import { guardFicDenominator } from './fic-guard.mjs'
import { FIC_DENOMINATOR } from './vocab.mjs'
import { evaluateEmploymentAntiPattern } from './employment-anti-patterns.mjs'
import { assignEmploymentMaturity, evaluateEmploymentRecord, validateEmploymentRecord } from './employment.mjs'
import {
  AGENT_WORKS_BY_CONTRACT,
  AUTHORITY_EVALUATION_IS_NO_AI,
  COMPANY_AGENT_ROLE_IS_NOT_EMPLOYMENT_CONTRACT,
  CONTRACT_VERSIONS_MUST_BE_HISTORICALLY_REPRODUCIBLE,
  CREDENTIAL_EXISTENCE_IS_NOT_ELIGIBILITY,
  DIGITAL_EMPLOYEES_REQUIRE_EFFECTIVE_CONTRACTS,
  EMPLOYMENT_SCHEMA,
  EMPLOYMENT_STANDARD_VERSION,
  EXPIRED_OR_REVOKED_MAY_NOT_ADMIT_NEW_WORK,
  HARD_FAILURE_TAGS,
  LEGAL_PERSONHOOD_NOT_ASSERTED,
  PROSE_IS_NOT_GRANT,
  RUNTIME_WORKER_IS_NOT_DIGITAL_EMPLOYEE,
  SHARED_WEB2APP_ANTI_PATTERNS,
  SKILL_IS_NOT_AUTHORITY,
  THIS_STANDARD_IS_NOT_CAP16_IMPLEMENTATION,
  TITLE_IS_NOT_AUTHORITY,
  TOOL_IS_NOT_AUTHORITY,
  UTILITIES_DO_NOT_REQUIRE_EMPLOYMENT_CONTRACTS,
  WEB2APP_W_LEVEL_IS_NOT_EMPLOYMENT_E_LEVEL,
  WORKER_MAY_NOT_SELF_AMEND,
  WORKORDER_MAY_NOT_BROADEN_CONTRACT,
} from './employment-vocab.mjs'
import { WEB2APP_STANDARD_VERSION } from './web2app-vocab.mjs'

const ROOT = fileURLToPath(new URL('../../../', import.meta.url))
const FIX = join(ROOT, 'tools/benchmark/engine/fixtures/employment')

function loadFix(name) {
  return JSON.parse(readFileSync(join(FIX, name), 'utf8'))
}

function evalFix(name) {
  return evaluateEmploymentRecord(loadFix(name))
}

test('employment hard answers are frozen', () => {
  assert.equal(EMPLOYMENT_STANDARD_VERSION, '1.0.0')
  assert.equal(EMPLOYMENT_SCHEMA, 'chronica.employment.evaluation.v1')
  assert.equal(AGENT_WORKS_BY_CONTRACT, true)
  assert.equal(COMPANY_AGENT_ROLE_IS_NOT_EMPLOYMENT_CONTRACT, true)
  assert.equal(RUNTIME_WORKER_IS_NOT_DIGITAL_EMPLOYEE, true)
  assert.equal(SKILL_IS_NOT_AUTHORITY, true)
  assert.equal(TOOL_IS_NOT_AUTHORITY, true)
  assert.equal(CREDENTIAL_EXISTENCE_IS_NOT_ELIGIBILITY, true)
  assert.equal(TITLE_IS_NOT_AUTHORITY, true)
  assert.equal(PROSE_IS_NOT_GRANT, true)
  assert.equal(WORKER_MAY_NOT_SELF_AMEND, true)
  assert.equal(EXPIRED_OR_REVOKED_MAY_NOT_ADMIT_NEW_WORK, true)
  assert.equal(WORKORDER_MAY_NOT_BROADEN_CONTRACT, true)
  assert.equal(CONTRACT_VERSIONS_MUST_BE_HISTORICALLY_REPRODUCIBLE, true)
  assert.equal(DIGITAL_EMPLOYEES_REQUIRE_EFFECTIVE_CONTRACTS, true)
  assert.equal(UTILITIES_DO_NOT_REQUIRE_EMPLOYMENT_CONTRACTS, true)
  assert.equal(THIS_STANDARD_IS_NOT_CAP16_IMPLEMENTATION, true)
  assert.equal(AUTHORITY_EVALUATION_IS_NO_AI, true)
  assert.equal(LEGAL_PERSONHOOD_NOT_ASSERTED, true)
  assert.equal(WEB2APP_W_LEVEL_IS_NOT_EMPLOYMENT_E_LEVEL, true)
  assert.deepEqual(SHARED_WEB2APP_ANTI_PATTERNS, [
    'LABEL_AS_ENFORCEMENT',
    'GOALPOST_MUTATION',
    'TEST_AS_RUNTIME',
    'TRACKING_AS_PRODUCT',
    'CALLER_FARMING',
  ])
  assert.ok(HARD_FAILURE_TAGS.includes('WORKER_SELF_AMENDMENT'))
  assert.ok(HARD_FAILURE_TAGS.includes('SECRET_IN_EMPLOYMENT_CONTRACT'))
})

test('FIC denominator remains 55 and is not employment coverage', () => {
  const v1 = loadV1Denominator(ROOT)
  const census = loadJson(join(ROOT, 'docs/_machine/fic-revalidation-census.json'))
  const guard = guardFicDenominator(v1, census)
  assert.equal(guard.fic.denominator, FIC_DENOMINATOR)
  assert.equal(guard.fic.denominator, 55)
  assert.equal(WEB2APP_STANDARD_VERSION, '1.1.0')
})

test('anti-patterns reject keyword grep', () => {
  const result = evaluateEmploymentAntiPattern('TITLE_AS_AUTHORITY', { keyword_only: true })
  assert.equal(result.ok, false)
})

test('Case A CompanyAgentRole calibrates to E1 and is not a contract', () => {
  const result = evalFix('company-agent-role-only.json')
  assert.equal(result.employment_maturity, 'E1')
  assert.equal(result.verdict, 'PASS')
  assert.equal(result.worker_class, 'position')
  assert.equal(result.this_standard_is_not_cap16, true)
})

test('Case B runtime Worker calibrates to E0', () => {
  const result = evalFix('runtime-worker-only.json')
  assert.equal(result.employment_maturity, 'E0')
  assert.equal(result.verdict, 'PASS')
  assert.equal(result.worker_class, 'runtime_worker')
})

test('Case C CapabilityRegistry employment is E0 (orthogonal skill registry)', () => {
  const result = evalFix('capability-registry-orthogonal.json')
  assert.equal(result.employment_maturity, 'E0')
  assert.equal(result.verdict, 'PASS')
})

test('Case D doctrine 173 is semantically strong and runtime E0', () => {
  const result = evalFix('doc-only-employment-contract.json')
  assert.equal(result.employment_maturity, 'E0')
  assert.equal(result.employment_semantic_target, 'STRONG')
  assert.equal(result.verdict, 'PASS')
  assert.equal(result.runtime_status, 'DOC_ONLY')
})

test('durable contract without assignment is max E2', () => {
  const result = evalFix('durable-contract-no-assignment.json')
  assert.equal(result.employment_maturity, 'E2')
  assert.equal(result.verdict, 'PASS')
})

test('unpinned assignment cannot claim E3', () => {
  const result = evalFix('assignment-unpinned-contract.json')
  assert.equal(result.employment_maturity, 'E2')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.anti_patterns_found.includes('UNPINNED_ASSIGNMENT_AUTHORITY'))
  assert.ok(result.hard_gate_failures.includes('CLAIMED_MATURITY_EXCEEDS_EVIDENCE'))
})

test('expired contract executing new work is a hard failure', () => {
  const result = evalFix('expired-contract-incorrectly-executes.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_failures.includes('EXPIRED_CONTRACT_EXECUTES_NEW_WORK'))
  assert.ok(result.hard_gate_failures.includes('NO_EFFECTIVE_CONTRACT_NO_NEW_PRODUCTION_WORK'))
})

test('suspended worker denied is honest E4', () => {
  const result = evalFix('suspended-worker-denied.json')
  assert.equal(result.employment_maturity, 'E4')
  assert.equal(result.verdict, 'PASS')
})

test('skill without grant is denied at E4', () => {
  const result = evalFix('skill-without-grant-denied.json')
  assert.equal(result.employment_maturity, 'E4')
  assert.equal(result.verdict, 'PASS')
  assert.equal(result.anti_patterns_found.includes('SKILL_AS_AUTHORITY'), false)
})

test('tool without grant is denied at E4', () => {
  const result = evalFix('tool-without-grant-denied.json')
  assert.equal(result.employment_maturity, 'E4')
  assert.equal(result.verdict, 'PASS')
})

test('v1 assignment is preserved after v2 successor', () => {
  const result = evalFix('contract-v1-assignment-preserved-after-v2.json')
  assert.equal(result.employment_maturity, 'E3')
  assert.equal(result.verdict, 'PASS')
})

test('model upgrade retains worker identity at E2', () => {
  const result = evalFix('worker-model-upgrade-same-identity.json')
  assert.equal(result.employment_maturity, 'E2')
  assert.equal(result.verdict, 'PASS')
})

test('delegation overreach denied is E4', () => {
  const result = evalFix('delegation-overreach-denied.json')
  assert.equal(result.employment_maturity, 'E4')
  assert.equal(result.verdict, 'PASS')
})

test('WEB2APP W4 does not imply employment E4 collapse', () => {
  const result = evalFix('web2app-method-contract-denial.json')
  assert.equal(result.employment_maturity, 'E4')
  assert.equal(result.web2app_application_maturity, 'W4')
  assert.equal(result.web2app_independent, true)
  assert.equal(result.verdict, 'PASS')
})

test('utility does not require an employment contract', () => {
  const result = evalFix('utility-no-contract.json')
  assert.equal(result.employment_maturity, 'E0')
  assert.equal(result.verdict, 'PASS')
})

test('role overclaim as digital employee fails', () => {
  const result = evalFix('role-overclaim-e4.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.anti_patterns_found.includes('ROLE_AS_WORKER'))
  assert.ok(employmentRankSafe(result.employment_maturity) <= 1)
})

function employmentRankSafe(level) {
  return { E0: 0, E1: 1, E2: 2, E3: 3, E4: 4, E5: 5, E6: 6 }[level] ?? -1
}

test('prompt as contract is a hard failure', () => {
  const result = evalFix('prompt-as-employment-contract.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_failures.includes('PROMPT_AS_EMPLOYMENT_CONTRACT'))
})

test('secret in contract is a hard failure', () => {
  const result = evalFix('secret-in-employment-contract.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_failures.includes('SECRET_IN_EMPLOYMENT_CONTRACT'))
})

test('worker self-amendment is a hard failure', () => {
  const result = evalFix('worker-self-amendment.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_failures.includes('WORKER_SELF_AMENDMENT'))
})

test('title as authority is a hard failure', () => {
  const result = evalFix('title-as-authority.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_failures.includes('TITLE_AS_AUTHORITY'))
})

test('E6 cannot be claimed from a benchmark fixture', () => {
  const record = loadFix('suspended-worker-denied.json')
  record.claimed_employment_maturity = 'E6'
  record.evidence = { production_evidenced: true }
  const result = evaluateEmploymentRecord(record)
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.employment_maturity !== 'E6')
  assert.ok(result.hard_gate_failures.includes('CLAIMED_MATURITY_EXCEEDS_EVIDENCE'))
})

test('skill override of authority is a hard failure', () => {
  const result = evaluateEmploymentRecord({
    schema: EMPLOYMENT_SCHEMA,
    worker_class: 'digital_employee',
    claimed_employment_maturity: 'E4',
    fixture_kind: 'BENCHMARK_FIXTURE',
    identity: { stable_worker_id: true },
    organization: { company_bound: true },
    contract: { durable: true, versioned: true, immutable_history: true },
    assignment: { work_order_present: true, production_path: true, contract_version_pinned: true },
    authority: {
      enforced: true,
      witness_present: true,
      no_ai_enforcement: true,
      skill_overrides_authority: true,
    },
  })
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_failures.includes('SKILL_OVERRIDES_AUTHORITY'))
})

test('WorkOrder broadening contract is found', () => {
  const result = evaluateEmploymentRecord({
    schema: EMPLOYMENT_SCHEMA,
    worker_class: 'digital_employee',
    claimed_employment_maturity: 'E3',
    fixture_kind: 'BENCHMARK_FIXTURE',
    identity: { stable_worker_id: true },
    organization: { company_bound: true },
    contract: { durable: true, versioned: true },
    assignment: {
      work_order_present: true,
      production_path: true,
      contract_version_pinned: true,
      broadens_contract: true,
    },
    authority: { no_ai_enforcement: true },
  })
  assert.ok(result.anti_patterns_found.includes('WORKORDER_BROADENS_CONTRACT'))
})

test('doc-only struct claimed as E2 runtime fails DOC_SNIPPET_AS_RUNTIME', () => {
  const record = loadFix('doc-only-employment-contract.json')
  record.claimed_employment_maturity = 'E2'
  const result = evaluateEmploymentRecord(record)
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.anti_patterns_found.includes('DOC_SNIPPET_AS_RUNTIME'))
  assert.equal(result.employment_maturity, 'E0')
})

test('status separation is returned for every evaluation', () => {
  const result = evalFix('company-agent-role-only.json')
  assert.equal(result.canonical_status, 'implemented_unverified')
  assert.equal(result.employment_maturity, 'E1')
  assert.equal(result.runtime_status, 'LIVE')
  assert.equal(result.product_status, 'POSITION_TEMPLATE')
  assert.equal(result.production_evidence, false)
})

test('validateEmploymentRecord rejects unknown worker class', () => {
  const result = validateEmploymentRecord({ worker_class: 'salaryman', claimed_employment_maturity: 'E1' })
  assert.equal(result.ok, false)
})

test('assignEmploymentMaturity does not treat utilities as employees', () => {
  assert.equal(assignEmploymentMaturity({ worker_class: 'utility', organization: { company_bound: true } }), 'E0')
})

test('every employment fixture is valid JSON of the employment schema', () => {
  const files = readdirSync(FIX).filter((name) => name.endsWith('.json'))
  assert.ok(files.length >= 13)
  for (const name of files) {
    const record = loadFix(name)
    assert.equal(record.schema, EMPLOYMENT_SCHEMA)
    const result = evaluateEmploymentRecord(record)
    assert.equal(result.errors.length, 0, `${name}: ${result.errors.join('; ')}`)
    assert.equal(result.this_standard_is_not_cap16, true)
  }
})
