import test from 'node:test'
import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { loadV1Denominator, loadJson } from '../fic-recompute-lib.mjs'
import { guardFicDenominator } from './fic-guard.mjs'
import { FIC_DENOMINATOR } from './vocab.mjs'
import { evaluateStoreAntiPattern } from './store-anti-patterns.mjs'
import { assignStoreMaturity, evaluateStoreRecord, validateStoreRecord } from './store.mjs'
import {
  AGENT_CANNOT_SELF_PROMOTE_PRODUCTION,
  ALL_IN_ONE_IS_NOT_ONE_CRATE,
  BUSINESS_SOURCE_OF_TRUTH_MUST_BE_EXPLICIT,
  CENSUS_IS_NOT_SQL_AUTHORITY,
  DESIRED_MUST_DISTINGUISH_ACTUAL,
  DNS_REQUIRES_RIGHTS_AND_AUTHORITY,
  EMPLOYMENT_E_LEVEL_IS_NOT_STORE_S_LEVEL,
  FEDERATED_DATA_WITHOUT_COPY_IS_ALLOWED,
  HARD_FAILURE_TAGS,
  HOSTING_PREFLIGHT_IS_NOT_DEPLOY,
  MANIFEST_IS_NOT_AUTHORITY,
  PROVIDER_IDENTITY_IS_NOT_STORE_IDENTITY,
  PROVIDER_PROJECT_IS_NOT_STORE,
  PROVIDER_SUCCESS_IS_NOT_LIVE,
  SECRETS_MUST_NOT_EMBED_IN_MANIFEST,
  SHADOW_COMPANY_OS_FORBIDDEN,
  STATIC_ARTIFACT_IS_NOT_STORE,
  STORE_DELETE_IS_NOT_COMPANY_DATA_DELETE,
  STORE_LOCAL_STATE_IS_LEGITIMATE,
  STORE_SCHEMA,
  STORE_STANDARD_ID,
  STORE_STANDARD_VERSION,
  THIS_STANDARD_IS_NOT_CAP17_IMPLEMENTATION,
  WEB2APP_W_LEVEL_IS_NOT_STORE_S_LEVEL,
} from './store-vocab.mjs'
import { WEB2APP_STANDARD_VERSION } from './web2app-vocab.mjs'
import { EMPLOYMENT_STANDARD_VERSION } from './employment-vocab.mjs'

const ROOT = fileURLToPath(new URL('../../../', import.meta.url))
const FIX = join(ROOT, 'tools/benchmark/engine/fixtures/store')

function loadFix(name) {
  return JSON.parse(readFileSync(join(FIX, name), 'utf8'))
}

function evalFix(name) {
  return evaluateStoreRecord(loadFix(name))
}

test('store hard answers are frozen', () => {
  assert.equal(STORE_STANDARD_VERSION, '1.0.0')
  assert.equal(STORE_SCHEMA, 'chronica.store.evaluation.v1')
  assert.equal(STORE_STANDARD_ID, '207')
  assert.equal(STATIC_ARTIFACT_IS_NOT_STORE, true)
  assert.equal(PROVIDER_PROJECT_IS_NOT_STORE, true)
  assert.equal(HOSTING_PREFLIGHT_IS_NOT_DEPLOY, true)
  assert.equal(PROVIDER_SUCCESS_IS_NOT_LIVE, true)
  assert.equal(SHADOW_COMPANY_OS_FORBIDDEN, true)
  assert.equal(BUSINESS_SOURCE_OF_TRUTH_MUST_BE_EXPLICIT, true)
  assert.equal(CENSUS_IS_NOT_SQL_AUTHORITY, true)
  assert.equal(MANIFEST_IS_NOT_AUTHORITY, true)
  assert.equal(SECRETS_MUST_NOT_EMBED_IN_MANIFEST, true)
  assert.equal(AGENT_CANNOT_SELF_PROMOTE_PRODUCTION, true)
  assert.equal(DNS_REQUIRES_RIGHTS_AND_AUTHORITY, true)
  assert.equal(ALL_IN_ONE_IS_NOT_ONE_CRATE, true)
  assert.equal(FEDERATED_DATA_WITHOUT_COPY_IS_ALLOWED, true)
  assert.equal(STORE_LOCAL_STATE_IS_LEGITIMATE, true)
  assert.equal(PROVIDER_IDENTITY_IS_NOT_STORE_IDENTITY, true)
  assert.equal(DESIRED_MUST_DISTINGUISH_ACTUAL, true)
  assert.equal(THIS_STANDARD_IS_NOT_CAP17_IMPLEMENTATION, true)
  assert.equal(WEB2APP_W_LEVEL_IS_NOT_STORE_S_LEVEL, true)
  assert.equal(EMPLOYMENT_E_LEVEL_IS_NOT_STORE_S_LEVEL, true)
  assert.equal(STORE_DELETE_IS_NOT_COMPANY_DATA_DELETE, true)
  assert.ok(HARD_FAILURE_TAGS.includes('SHADOW_COMPANY_OS'))
  assert.ok(HARD_FAILURE_TAGS.includes('HOSTING_PREFLIGHT_AS_DEPLOYMENT'))
  assert.ok(HARD_FAILURE_TAGS.includes('PROVIDER_200_AS_LIVE'))
  assert.ok(HARD_FAILURE_TAGS.includes('DEPLOY_WITHOUT_RECEIPT'))
})

test('FIC denominator remains 55 and is not Store coverage', () => {
  const v1 = loadV1Denominator(ROOT)
  const census = loadJson(join(ROOT, 'docs/_machine/fic-revalidation-census.json'))
  const guard = guardFicDenominator(v1, census)
  assert.equal(guard.fic.denominator, FIC_DENOMINATOR)
  assert.equal(guard.fic.denominator, 55)
  assert.equal(WEB2APP_STANDARD_VERSION, '1.1.0')
  assert.equal(EMPLOYMENT_STANDARD_VERSION, '1.0.0')
})

test('anti-patterns reject keyword grep', () => {
  const result = evaluateStoreAntiPattern('SHADOW_COMPANY_OS', { keyword_only: true })
  assert.equal(result.ok, false)
})

test('validateStoreRecord rejects unknown schema and facets', () => {
  const bad = validateStoreRecord({ schema: 'nope', facets: { NOT_A_FACET: 'ABSENT' } })
  assert.equal(bad.ok, false)
  const good = validateStoreRecord(loadFix('manifest-only.json'))
  assert.equal(good.ok, true)
})

test('static artifact only is S0 PASS', () => {
  const result = evalFix('static-artifact-only.json')
  assert.equal(result.store_maturity, 'S0')
  assert.equal(result.verdict, 'PASS')
  assert.equal(assignStoreMaturity(loadFix('static-artifact-only.json')), 'S0')
})

test('chronica-site-hosting preflight calibrates to S0 and is not deploy', () => {
  const result = evalFix('hosting-preflight-only.json')
  assert.equal(result.store_maturity, 'S0')
  assert.equal(result.verdict, 'PASS')
  assert.equal(result.fixture_kind, 'CURRENT_RUNTIME_CALIBRATION')
  assert.equal(result.this_standard_is_not_cap17, true)
  assert.ok(!result.hard_failures.includes('HOSTING_PREFLIGHT_AS_DEPLOYMENT'))
})

test('hosting preflight claimed as deploy is FAIL', () => {
  const result = evalFix('hosting-preflight-claimed-as-deploy.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_failures.includes('HOSTING_PREFLIGHT_AS_DEPLOYMENT'))
  assert.equal(result.store_maturity, 'S0')
})

test('provider project used as Store identity is FAIL', () => {
  const result = evalFix('provider-project-only.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.anti_patterns_found.includes('PROVIDER_PROJECT_AS_STORE'))
  assert.ok(result.anti_patterns_found.includes('PROVIDER_LOCK_IN_AS_STORE_IDENTITY'))
  assert.equal(result.store_maturity, 'S0')
})

test('manifest-only reaches S1 PASS', () => {
  const result = evalFix('manifest-only.json')
  assert.equal(result.store_maturity, 'S1')
  assert.equal(result.verdict, 'PASS')
})

test('preview-store reaches S2 PASS', () => {
  const result = evalFix('preview-store.json')
  assert.equal(result.store_maturity, 'S2')
  assert.equal(result.verdict, 'PASS')
})

test('real deploy without business/data bindings cannot strong-S3', () => {
  const result = evalFix('real-deploy-no-business-bindings.json')
  assert.equal(result.store_maturity, 'S2')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_gate_failures.includes('CLAIMED_MATURITY_EXCEEDS_EVIDENCE'))
})

test('shadow CRM is FAIL SHADOW_COMPANY_OS', () => {
  const result = evalFix('store-with-shadow-crm.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_failures.includes('SHADOW_COMPANY_OS'))
  assert.ok(result.anti_patterns_found.includes('STORE_AS_CRM'))
})

test('canonical CRM binding is S3 PASS', () => {
  const result = evalFix('store-with-canonical-crm-binding.json')
  assert.equal(result.store_maturity, 'S3')
  assert.equal(result.verdict, 'PASS')
  assert.equal(result.web2app_independent, true)
  assert.equal(result.employment_independent, true)
})

test('Helpdesk ticket binding is S3 PASS', () => {
  const result = evalFix('store-with-helpdesk-ticket-binding.json')
  assert.equal(result.store_maturity, 'S3')
  assert.equal(result.verdict, 'PASS')
})

test('database census classified is S3 PASS', () => {
  const result = evalFix('store-with-database-census.json')
  assert.equal(result.store_maturity, 'S3')
  assert.equal(result.verdict, 'PASS')
})

test('unclassified database is PARTIAL semantic review', () => {
  const result = evalFix('store-with-unclassified-database.json')
  assert.equal(result.store_maturity, 'S3')
  assert.equal(result.verdict, 'PARTIAL')
  assert.ok(result.semantic_review_required.some((row) => row.includes('UNKNOWN')))
})

test('federated data with explicit owner is S3 PASS', () => {
  const result = evalFix('store-with-federated-data.json')
  assert.equal(result.store_maturity, 'S3')
  assert.equal(result.verdict, 'PASS')
})

test('provider success with failed health is FAIL PROVIDER_200_AS_LIVE', () => {
  const result = evalFix('store-with-provider-success-but-health-fail.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_failures.includes('PROVIDER_200_AS_LIVE'))
  assert.equal(result.store_maturity, 'S2')
})

test('production without approval is FAIL', () => {
  const result = evalFix('store-with-production-no-approval.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_failures.includes('MANIFEST_AS_AUTHORITY'))
  assert.ok(result.hard_gate_failures.includes('PRODUCTION_WITHOUT_AUTHORITY'))
  assert.equal(result.store_maturity, 'S3')
})

test('secret in StoreManifest is FAIL', () => {
  const result = evalFix('store-with-secret-in-manifest.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_failures.includes('SECRET_IN_STORE_MANIFEST'))
})

test('desired/actual drift with reconciliation is S5 PASS', () => {
  const result = evalFix('store-with-desired-actual-drift.json')
  assert.equal(result.store_maturity, 'S5')
  assert.equal(result.verdict, 'PASS')
  assert.equal(result.production_evidence, false)
})

test('rollback-capable production store is S4 PASS', () => {
  const result = evalFix('store-with-rollback.json')
  assert.equal(result.store_maturity, 'S4')
  assert.equal(result.verdict, 'PASS')
})

test('multi-store shared CRM is S3 PASS', () => {
  const result = evalFix('multi-store-shared-crm.json')
  assert.equal(result.store_maturity, 'S3')
  assert.equal(result.verdict, 'PASS')
})

test('persistence adapter named Store is FAIL when claimed as CAP17', () => {
  const result = evalFix('persistence-adapter-as-store.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.anti_patterns_found.includes('PERSISTENCE_ADAPTER_AS_STORE'))
  assert.equal(result.store_maturity, 'S0')
})

test('BENCHMARK_FIXTURE cannot reach S6', () => {
  const result = evalFix('s6-blocked-on-fixture.json')
  assert.equal(result.store_maturity, 'S5')
  assert.equal(result.production_evidence, false)
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_gate_failures.includes('CLAIMED_MATURITY_EXCEEDS_EVIDENCE'))
})

test('cross-tenant Store resource access is FAIL', () => {
  const result = evalFix('cross-tenant-store-access.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_gate_failures.includes('CROSS_TENANT_STORE_RESOURCE_ACCESS'))
})

test('collapsing WEB2APP or employment into Store is FAIL', () => {
  const record = {
    ...loadFix('preview-store.json'),
    web2app: { independent: true, implies_store: true },
  }
  const result = evaluateStoreRecord(record)
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_gate_failures.includes('AXIS_COLLAPSED_INTO_STORE'))
})

test('all store fixtures parse and evaluate without engine errors', () => {
  const files = readdirSync(FIX).filter((name) => name.endsWith('.json'))
  assert.ok(files.length >= 22)
  for (const name of files) {
    const result = evalFix(name)
    assert.equal(result.ok, true, `${name}: ${result.errors.join('; ')}`)
    assert.equal(result.schema, STORE_SCHEMA)
    assert.equal(result.standard_version, STORE_STANDARD_VERSION)
  }
})
