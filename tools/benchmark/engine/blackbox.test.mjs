import test from 'node:test'
import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { loadV1Denominator, loadJson } from '../fic-recompute-lib.mjs'
import { guardFicDenominator } from './fic-guard.mjs'
import { FIC_DENOMINATOR } from './vocab.mjs'
import { evaluateBlackboxAntiPattern } from './blackbox-anti-patterns.mjs'
import { assignBlackboxMaturity, evaluateBlackboxRecord, validateBlackboxRecord } from './blackbox.mjs'
import {
  ADMIN_IS_NOT_UNLIMITED_PLAINTEXT,
  AGENT_CAN_BE_USEFUL_WITHOUT_RAW_DATA,
  AGENT_CAPABILITY_IS_NOT_DATA_VISIBILITY,
  B5_DOES_NOT_REQUIRE_ZERO_KNOWLEDGE_SERVER,
  B6_REQUIRES_CONFIDENTIAL_COMPUTE_EVIDENCE,
  BACKUPS_CAN_DEFEAT_ENCRYPTED_STORAGE,
  BLACKBOX_SCHEMA,
  BLACKBOX_STANDARD_ID,
  BLACKBOX_STANDARD_VERSION,
  EMBEDDINGS_CAN_REMAIN_SENSITIVE,
  EMPLOYMENT_E_LEVEL_IS_NOT_BLACKBOX_B_LEVEL,
  ENCRYPTION_AT_REST_IS_NOT_BLACKBOX,
  FORBIDDEN_MUST_NOT_ENTER_RETRIEVAL_CANDIDATES,
  HARD_FAILURE_TAGS,
  HASHING_IS_NOT_ENCRYPTION,
  LOGS_CAN_DEFEAT_ENCRYPTED_STORAGE,
  OPAQUE_REF_IS_NOT_PERMISSION,
  OPAQUE_TENANT_REF_ALONE_IS_NOT_BLACKBOX,
  PLAINTEXT_SEARCH_CAN_DEFEAT_ENCRYPTED_STORAGE,
  PROVIDER_IS_NOT_TRUSTED_INTERNAL_MEMORY,
  PSEUDONYM_IS_NOT_ANONYMITY,
  SECRET_CAN_BE_USED_WITHOUT_BEING_READABLE,
  SECRET_READ_IS_NOT_SECRET_USE,
  STORE_MUST_NOT_RECEIVE_COMPANY_DB_CREDENTIALS_BY_DEFAULT,
  STORE_S_LEVEL_IS_NOT_BLACKBOX_B_LEVEL,
  SUMMARIES_CAN_REMAIN_SENSITIVE,
  THIS_STANDARD_IS_NOT_CAP18_IMPLEMENTATION,
  VAULT_ALONE_IS_NOT_BLACKBOX,
  WEB2APP_W_LEVEL_IS_NOT_BLACKBOX_B_LEVEL,
  WORKORDER_CAN_NARROW_VISIBILITY,
} from './blackbox-vocab.mjs'
import { WEB2APP_STANDARD_VERSION } from './web2app-vocab.mjs'
import { EMPLOYMENT_STANDARD_VERSION } from './employment-vocab.mjs'
import { STORE_STANDARD_VERSION } from './store-vocab.mjs'

const ROOT = fileURLToPath(new URL('../../../', import.meta.url))
const FIX = join(ROOT, 'tools/benchmark/engine/fixtures/blackbox')

function loadFix(name) {
  return JSON.parse(readFileSync(join(FIX, name), 'utf8'))
}

function evalFix(name) {
  return evaluateBlackboxRecord(loadFix(name))
}

test('blackbox hard answers are frozen', () => {
  assert.equal(BLACKBOX_STANDARD_VERSION, '1.0.0')
  assert.equal(BLACKBOX_SCHEMA, 'chronica.blackbox.evaluation.v1')
  assert.equal(BLACKBOX_STANDARD_ID, '208')
  assert.equal(ENCRYPTION_AT_REST_IS_NOT_BLACKBOX, true)
  assert.equal(VAULT_ALONE_IS_NOT_BLACKBOX, true)
  assert.equal(OPAQUE_TENANT_REF_ALONE_IS_NOT_BLACKBOX, true)
  assert.equal(AGENT_CAN_BE_USEFUL_WITHOUT_RAW_DATA, true)
  assert.equal(SECRET_CAN_BE_USED_WITHOUT_BEING_READABLE, true)
  assert.equal(OPAQUE_REF_IS_NOT_PERMISSION, true)
  assert.equal(FORBIDDEN_MUST_NOT_ENTER_RETRIEVAL_CANDIDATES, true)
  assert.equal(EMBEDDINGS_CAN_REMAIN_SENSITIVE, true)
  assert.equal(SUMMARIES_CAN_REMAIN_SENSITIVE, true)
  assert.equal(LOGS_CAN_DEFEAT_ENCRYPTED_STORAGE, true)
  assert.equal(BACKUPS_CAN_DEFEAT_ENCRYPTED_STORAGE, true)
  assert.equal(PLAINTEXT_SEARCH_CAN_DEFEAT_ENCRYPTED_STORAGE, true)
  assert.equal(PROVIDER_IS_NOT_TRUSTED_INTERNAL_MEMORY, true)
  assert.equal(STORE_MUST_NOT_RECEIVE_COMPANY_DB_CREDENTIALS_BY_DEFAULT, true)
  assert.equal(WORKORDER_CAN_NARROW_VISIBILITY, true)
  assert.equal(AGENT_CAPABILITY_IS_NOT_DATA_VISIBILITY, true)
  assert.equal(ADMIN_IS_NOT_UNLIMITED_PLAINTEXT, true)
  assert.equal(B5_DOES_NOT_REQUIRE_ZERO_KNOWLEDGE_SERVER, true)
  assert.equal(B6_REQUIRES_CONFIDENTIAL_COMPUTE_EVIDENCE, true)
  assert.equal(THIS_STANDARD_IS_NOT_CAP18_IMPLEMENTATION, true)
  assert.equal(WEB2APP_W_LEVEL_IS_NOT_BLACKBOX_B_LEVEL, true)
  assert.equal(EMPLOYMENT_E_LEVEL_IS_NOT_BLACKBOX_B_LEVEL, true)
  assert.equal(STORE_S_LEVEL_IS_NOT_BLACKBOX_B_LEVEL, true)
  assert.equal(HASHING_IS_NOT_ENCRYPTION, true)
  assert.equal(PSEUDONYM_IS_NOT_ANONYMITY, true)
  assert.equal(SECRET_READ_IS_NOT_SECRET_USE, true)
  assert.ok(HARD_FAILURE_TAGS.includes('SECRET_IN_AGENT_CONTEXT'))
  assert.ok(HARD_FAILURE_TAGS.includes('ZERO_KNOWLEDGE_CLAIM_WITH_SERVER_PLAINTEXT'))
  assert.ok(HARD_FAILURE_TAGS.includes('GLOBAL_RETRIEVE_THEN_FILTER'))
})

test('FIC denominator remains 55 and prior standards are preserved', () => {
  const v1 = loadV1Denominator(ROOT)
  const census = loadJson(join(ROOT, 'docs/_machine/fic-revalidation-census.json'))
  const guard = guardFicDenominator(v1, census)
  assert.equal(guard.fic.denominator, FIC_DENOMINATOR)
  assert.equal(guard.fic.denominator, 55)
  assert.equal(WEB2APP_STANDARD_VERSION, '1.1.0')
  assert.equal(EMPLOYMENT_STANDARD_VERSION, '1.0.0')
  assert.equal(STORE_STANDARD_VERSION, '1.0.0')
})

test('anti-patterns reject keyword grep', () => {
  const result = evaluateBlackboxAntiPattern('ENCRYPTION_AT_REST_AS_BLACKBOX', { keyword_only: true })
  assert.equal(result.ok, false)
})

test('validateBlackboxRecord rejects unknown schema and facets', () => {
  const bad = validateBlackboxRecord({ schema: 'nope', facets: { NOT_A_FACET: 'ABSENT' } })
  assert.equal(bad.ok, false)
  const good = validateBlackboxRecord(loadFix('current-global-b1.json'))
  assert.equal(good.ok, true)
})

test('global Chronica calibrates to B1', () => {
  const result = evalFix('current-global-b1.json')
  assert.equal(result.blackbox_maturity, 'B1')
  assert.equal(result.verdict, 'PASS')
  assert.equal(result.this_standard_is_not_cap18, true)
  assert.equal(assignBlackboxMaturity(loadFix('current-global-b1.json')), 'B1')
})

test('OpaqueTenantRef primitive calibrates to B2 not system B5', () => {
  const result = evalFix('opaque-tenant-positive.json')
  assert.equal(result.blackbox_maturity, 'B2')
  assert.equal(result.verdict, 'PASS')
  assert.equal(result.fixture_kind, 'CURRENT_RUNTIME_CALIBRATION')
})

test('vault-only cannot claim B5', () => {
  const result = evalFix('vault-only.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.anti_patterns_found.includes('VAULT_EXISTS_AS_BLACKBOX'))
  assert.ok(['B0', 'B1', 'B2'].includes(result.blackbox_maturity))
})

test('encrypted DB with plaintext log fails', () => {
  const result = evalFix('encrypted-db-plaintext-log.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_failures.includes('SECRET_IN_LOG'))
  assert.ok(result.hard_gate_failures.includes('B5_WITH_PLAINTEXT_SIDE_CHANNEL_OR_MISSING_BOUNDARY'))
})

test('plaintext vector index blocks B5', () => {
  const result = evalFix('encrypted-db-plaintext-vector-index.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.anti_patterns_found.includes('PLAINTEXT_VECTOR_INDEX_BYPASS'))
})

test('reversible opaque ref is FAIL', () => {
  const result = evalFix('reversible-opaque-ref.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_failures.includes('REVERSIBLE_OPAQUE_REFERENCE'))
})

test('raw customer to Agent is FAIL', () => {
  const result = evalFix('agent-full-raw-customer.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.anti_patterns_found.includes('RAW_OBJECT_TO_AGENT'))
})

test('support semantic view fixture is B3 PASS', () => {
  const result = evalFix('agent-support-semantic-view.json')
  assert.equal(result.blackbox_maturity, 'B3')
  assert.equal(result.verdict, 'PASS')
  assert.equal(result.production_evidence, false)
})

test('secret read when use suffices cannot claim B4', () => {
  const result = evalFix('secret-read-vs-secret-use.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.anti_patterns_found.includes('SECRET_READ_WHEN_USE_SUFFICES'))
})

test('provider raw internal id is FAIL', () => {
  const result = evalFix('provider-raw-internal-id-leak.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_failures.includes('RAW_INTERNAL_ID_TO_PROVIDER'))
})

test('provider opaque ref positive is B2 PASS', () => {
  const result = evalFix('provider-opaque-ref-positive.json')
  assert.equal(result.blackbox_maturity, 'B2')
  assert.equal(result.verdict, 'PASS')
})

test('global RAG retrieve-then-filter is FAIL', () => {
  const result = evalFix('global-rag-retrieve-then-filter.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_failures.includes('GLOBAL_RETRIEVE_THEN_FILTER'))
})

test('scope-before-retrieval is B3 PASS', () => {
  const result = evalFix('scope-before-rag-retrieval.json')
  assert.equal(result.blackbox_maturity, 'B3')
  assert.equal(result.verdict, 'PASS')
})

test('plaintext cache and backup bypass B5', () => {
  const cache = evalFix('plaintext-cache-bypass.json')
  const backup = evalFix('plaintext-backup-bypass.json')
  assert.equal(cache.verdict, 'FAIL')
  assert.equal(backup.verdict, 'FAIL')
  assert.ok(cache.anti_patterns_found.includes('PLAINTEXT_CACHE_BYPASS'))
  assert.ok(backup.anti_patterns_found.includes('PLAINTEXT_BACKUP_BYPASS'))
})

test('output DLP block fixture is B4 PASS', () => {
  const result = evalFix('output-dlp-block.json')
  assert.equal(result.blackbox_maturity, 'B4')
  assert.equal(result.verdict, 'PASS')
})

test('wrong-tenant ciphertext is FAIL', () => {
  const result = evalFix('wrong-tenant-ciphertext.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_gate_failures.includes('CROSS_TENANT_BLACKBOX_FAILURE'))
})

test('wrong-purpose handle is FAIL', () => {
  const result = evalFix('wrong-purpose-handle.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_gate_failures.includes('WRONG_PURPOSE_HANDLE_SUCCEEDED'))
  assert.ok(result.hard_failures.includes('OPAQUE_REF_AS_AUTHORITY'))
})

test('expired ContextPack reuse is FAIL', () => {
  const result = evalFix('expired-context-pack.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_gate_failures.includes('EXPIRED_CONTEXT_PACK_REUSED'))
})

test('view-cache served to the wrong actor is FAIL', () => {
  const result = evalFix('view-cache-scope-confusion.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_gate_failures.includes('VIEW_CACHE_SCOPE_CONFUSION'))
  assert.ok(result.anti_patterns_found.includes('VIEW_CACHE_SCOPE_CONFUSION'))
})

test('Store secret in manifest is FAIL', () => {
  const result = evalFix('store-secret-manifest-leak.json')
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_failures.includes('SECRET_IN_STORE_MANIFEST'))
  assert.ok(result.hard_failures.includes('STORE_BINDING_EXPOSES_DATABASE_SECRET'))
})

test('WEB2APP credential handle fixture is B4 PASS', () => {
  const result = evalFix('webapp-credential-handle-positive.json')
  assert.equal(result.blackbox_maturity, 'B4')
  assert.equal(result.verdict, 'PASS')
  assert.equal(result.web2app_independent, true)
})

test('fake confidential-compute / ZK claim is FAIL and cannot be B6', () => {
  const result = evalFix('confidential-compute-fake-claim.json')
  assert.equal(result.verdict, 'FAIL')
  assert.notEqual(result.blackbox_maturity, 'B6')
  assert.ok(result.hard_failures.includes('ZERO_KNOWLEDGE_CLAIM_WITH_SERVER_PLAINTEXT'))
  assert.ok(result.hard_failures.includes('CONFIDENTIAL_COMPUTE_CLAIM_WITHOUT_ATTESTATION'))
})

test('BENCHMARK_FIXTURE_ONLY cannot award B6', () => {
  const record = {
    ...loadFix('confidential-compute-fake-claim.json'),
    claimed_blackbox_maturity: 'B6',
    confidential_compute: { claimed: true, attestation: true },
    zero_knowledge: { claimed: false, server_plaintext: false },
    evidence: { production_evidenced: true },
  }
  const result = evaluateBlackboxRecord(record)
  assert.notEqual(result.blackbox_maturity, 'B6')
})

test('collapsing Store/WEB2APP into Black-Box is FAIL', () => {
  const result = evaluateBlackboxRecord({
    ...loadFix('current-global-b1.json'),
    web2app: { independent: true, implies_blackbox: true },
  })
  assert.equal(result.verdict, 'FAIL')
  assert.ok(result.hard_gate_failures.includes('AXIS_COLLAPSED_INTO_BLACKBOX'))
})

test('all blackbox fixtures parse without engine errors', () => {
  const files = readdirSync(FIX).filter((name) => name.endsWith('.json'))
  assert.ok(files.length >= 21)
  for (const name of files) {
    const result = evalFix(name)
    assert.equal(result.ok, true, `${name}: ${result.errors.join('; ')}`)
    assert.equal(result.schema, BLACKBOX_SCHEMA)
    assert.equal(result.standard_version, BLACKBOX_STANDARD_VERSION)
  }
})
