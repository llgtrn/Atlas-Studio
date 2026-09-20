#!/usr/bin/env node
// One-shot generator for overlay AP meta + 205–208 prose classification.
// Regenerating is optional; committed JSONL is the source of truth.
import { writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { WEB2APP_ANTI_PATTERNS } from '../../engine/web2app-vocab.mjs'
import {
  EMPLOYMENT_ANTI_PATTERNS,
  HARD_FAILURE_TAGS as EMPLOYMENT_HARD,
} from '../../engine/employment-vocab.mjs'
import { STORE_ANTI_PATTERNS, HARD_FAILURE_TAGS as STORE_HARD } from '../../engine/store-vocab.mjs'
import { BLACKBOX_ANTI_PATTERNS, HARD_FAILURE_TAGS as BLACKBOX_HARD } from '../../engine/blackbox-vocab.mjs'
import { CONTRACT_ANTI_PATTERNS, CONTRACT_SCHEMA } from '../../engine/contract-vocab.mjs'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const SOURCE = join(ROOT, 'source')

const FIX = {
  WORKER_SELF_AMENDMENT: 'tools/benchmark/engine/fixtures/employment/worker-self-amendment.json',
  SKILL_OVERRIDES_AUTHORITY: 'tools/benchmark/engine/fixtures/employment/skill-without-grant-denied.json',
  TOOL_AVAILABILITY_AS_AUTHORITY: 'tools/benchmark/engine/fixtures/employment/tool-without-grant-denied.json',
  TITLE_AS_AUTHORITY: 'tools/benchmark/engine/fixtures/employment/title-as-authority.json',
  PROSE_AS_AUTHORITY: 'tools/benchmark/engine/fixtures/employment/doc-only-employment-contract.json',
  PROMPT_AS_EMPLOYMENT_CONTRACT: 'tools/benchmark/engine/fixtures/employment/prompt-as-employment-contract.json',
  SECRET_IN_EMPLOYMENT_CONTRACT: 'tools/benchmark/engine/fixtures/employment/secret-in-employment-contract.json',
  EXPIRED_CONTRACT_EXECUTES_NEW_WORK: 'tools/benchmark/engine/fixtures/employment/expired-contract-incorrectly-executes.json',
  REVOKED_CONTRACT_EXECUTES_NEW_WORK: 'tools/benchmark/engine/fixtures/employment/suspended-worker-denied.json',
  DELEGATION_ESCALATES_AUTHORITY: 'tools/benchmark/engine/fixtures/employment/delegation-overreach-denied.json',
  CONTRACT_HISTORY_MUTATED_IN_PLACE: 'tools/benchmark/engine/fixtures/employment/contract-v1-assignment-preserved-after-v2.json',
  SHADOW_COMPANY_OS: 'tools/benchmark/engine/fixtures/store/store-with-shadow-crm.json',
  SECRET_IN_STORE_MANIFEST: 'tools/benchmark/engine/fixtures/store/store-with-secret-in-manifest.json',
  MANIFEST_AS_AUTHORITY: 'tools/benchmark/engine/fixtures/store/manifest-only.json',
  PROVIDER_200_AS_LIVE: 'tools/benchmark/engine/fixtures/store/store-with-provider-success-but-health-fail.json',
  AGENT_AS_DEPLOY_AUTHORITY: 'tools/benchmark/engine/fixtures/store/store-with-production-no-approval.json',
  SCHEMA_CENSUS_AS_SQL_AUTHORITY: 'tools/benchmark/engine/fixtures/store/store-with-database-census.json',
  DRIFT_AS_AUTO_MUTATION: 'tools/benchmark/engine/fixtures/store/store-with-desired-actual-drift.json',
  HOSTING_PREFLIGHT_AS_DEPLOYMENT: 'tools/benchmark/engine/fixtures/store/hosting-preflight-claimed-as-deploy.json',
  STATIC_ARTIFACT_AS_LIVE_STORE: 'tools/benchmark/engine/fixtures/store/static-artifact-only.json',
  PROVIDER_PROJECT_AS_STORE: 'tools/benchmark/engine/fixtures/store/provider-project-only.json',
  PERSISTENCE_ADAPTER_AS_STORE: 'tools/benchmark/engine/fixtures/store/persistence-adapter-as-store.json',
  SECRET_IN_AGENT_CONTEXT: 'tools/benchmark/engine/fixtures/blackbox/agent-full-raw-customer.json',
  SECRET_IN_LOG: 'tools/benchmark/engine/fixtures/blackbox/encrypted-db-plaintext-log.json',
  RAW_INTERNAL_ID_TO_PROVIDER: 'tools/benchmark/engine/fixtures/blackbox/provider-raw-internal-id-leak.json',
  GLOBAL_RETRIEVE_THEN_FILTER: 'tools/benchmark/engine/fixtures/blackbox/global-rag-retrieve-then-filter.json',
  REVERSIBLE_OPAQUE_REFERENCE: 'tools/benchmark/engine/fixtures/blackbox/reversible-opaque-ref.json',
  ZERO_KNOWLEDGE_CLAIM_WITH_SERVER_PLAINTEXT: 'tools/benchmark/engine/fixtures/blackbox/confidential-compute-fake-claim.json',
  CONFIDENTIAL_COMPUTE_CLAIM_WITHOUT_ATTESTATION: 'tools/benchmark/engine/fixtures/blackbox/confidential-compute-fake-claim.json',
  STORE_BINDING_EXPOSES_DATABASE_SECRET: 'tools/benchmark/engine/fixtures/blackbox/store-secret-manifest-leak.json',
  OPAQUE_REF_AS_AUTHORITY: 'tools/benchmark/engine/fixtures/blackbox/wrong-purpose-handle.json',
  RAW_OBJECT_TO_AGENT: 'tools/benchmark/engine/fixtures/blackbox/agent-full-raw-customer.json',
  PLAINTEXT_BACKUP_BYPASS: 'tools/benchmark/engine/fixtures/blackbox/plaintext-backup-bypass.json',
  PLAINTEXT_CACHE_BYPASS: 'tools/benchmark/engine/fixtures/blackbox/plaintext-cache-bypass.json',
  PLAINTEXT_VECTOR_INDEX_BYPASS: 'tools/benchmark/engine/fixtures/blackbox/encrypted-db-plaintext-vector-index.json',
  VIEW_CACHE_SCOPE_CONFUSION: 'tools/benchmark/engine/fixtures/blackbox/view-cache-scope-confusion.json',
  ENCRYPTION_AT_REST_AS_BLACKBOX: 'tools/benchmark/engine/fixtures/blackbox/vault-only.json',
  VAULT_EXISTS_AS_BLACKBOX: 'tools/benchmark/engine/fixtures/blackbox/vault-only.json',
  WEBHOOK_WITHOUT_AUTH: 'tools/benchmark/engine/fixtures/web2app/fixture.news.webhook_without_auth.json',
  CREDENTIAL_IN_AGENT_CONTEXT: 'tools/benchmark/engine/fixtures/blackbox/webapp-credential-handle-positive.json',
  FIXTURE_AS_RUNTIME: 'tools/benchmark/contracts/fixtures/fail-fixture-as-runtime.jsonl',
  CAPABILITY_AS_FLOW: 'tools/benchmark/contracts/fixtures/fail-capability-as-flow.jsonl',
  GENERATED_PACKET_DRIFT: 'tools/benchmark/contracts/fixtures/fail-packet-drift.jsonl',
  STALE_BENCHMARK: 'tools/benchmark/contracts/fixtures/fail-stale-benchmark.jsonl',
  CROSS_CAP_CONTRACT_FAIL: 'tools/benchmark/contracts/fixtures/fail-cross-cap-incomplete.jsonl',
  FLOW_RUNTIME_WITNESS_BROKEN: 'tools/benchmark/contracts/fixtures/fail-broken-evidence.jsonl',
  SCORE_OVERRIDES_HARD_GATE: 'tools/benchmark/contracts/fixtures/fail-score-overrides-gate.jsonl',
  STALE_CODE_EVIDENCE: 'tools/benchmark/contracts/fixtures/fail-stale-code-evidence.jsonl',
  LATEST_BENCHMARK_UNPINNED: 'tools/benchmark/contracts/fixtures/fail-unpinned-latest.jsonl',
  DONOR_REFERENCE_AS_FLOW: 'tools/benchmark/contracts/fixtures/fail-donor-as-flow.jsonl',
  BENCHMARK_AS_PRODUCT_AUTHORITY: 'tools/benchmark/contracts/fixtures/fail-benchmark-as-authority.jsonl',
  CAP_WEAKENS_OWN_STANDARD: 'tools/benchmark/contracts/fixtures/fail-cap-weakens-standard.jsonl',
  CROSS_TENANT_WORKER_ASSIGNMENT: 'tools/benchmark/engine/fixtures/employment/skill-without-grant-denied.json',
  MONEY_AUTHORITY_BYPASS: 'tools/benchmark/engine/fixtures/employment/tool-without-grant-denied.json',
  DATABASE_BYPASSES_BUSINESS_KERNEL: 'tools/benchmark/engine/fixtures/store/store-with-database-census.json',
  DNS_WITHOUT_RIGHTS: 'tools/benchmark/engine/fixtures/store/store-with-production-no-approval.json',
  AGENT_SELF_AUTHORIZES_DISCLOSURE: 'tools/benchmark/engine/fixtures/blackbox/agent-full-raw-customer.json',
  DEPLOY_WITHOUT_RECEIPT: 'tools/benchmark/engine/fixtures/store/store-with-provider-success-but-health-fail.json',
}

const DETECTOR = {
  WEB2APP: 'web2app',
  EMPLOYMENT: 'employment',
  STORE: 'store',
  BLACKBOX: 'blackbox',
  CONTRACT: 'contract',
}

function row(id, detector, standards, hard) {
  const fixture = FIX[id] || null
  const review = fixture ? undefined : 'MANUAL_REVIEW'
  return {
    schema: CONTRACT_SCHEMA,
    type: 'anti_pattern_meta',
    id,
    detector,
    severity: hard ? 'HARD_FAIL' : fixture ? 'HARD_FAIL' : 'WARNING',
    standards,
    ...(fixture ? { fixture } : {}),
    ...(review ? { review } : {}),
  }
}

const hardEmp = new Set(EMPLOYMENT_HARD)
const hardStore = new Set(STORE_HARD)
const hardBb = new Set(BLACKBOX_HARD)
const seen = new Map()

function add(id, detector, standards, hard) {
  if (seen.has(id)) return
  seen.set(id, row(id, detector, standards, hard))
}

for (const id of WEB2APP_ANTI_PATTERNS) add(id, DETECTOR.WEB2APP, ['WEB2APP'], false)
for (const id of EMPLOYMENT_ANTI_PATTERNS) add(id, DETECTOR.EMPLOYMENT, ['EMPLOYMENT'], hardEmp.has(id))
for (const id of STORE_ANTI_PATTERNS) add(id, DETECTOR.STORE, ['STORE'], hardStore.has(id))
for (const id of BLACKBOX_ANTI_PATTERNS) add(id, DETECTOR.BLACKBOX, ['BLACKBOX'], hardBb.has(id))
for (const id of CONTRACT_ANTI_PATTERNS) add(id, DETECTOR.CONTRACT, ['FIC'], true)
for (const id of EMPLOYMENT_HARD) add(id, DETECTOR.EMPLOYMENT, ['EMPLOYMENT'], true)
for (const id of STORE_HARD) add(id, DETECTOR.STORE, ['STORE'], true)
for (const id of BLACKBOX_HARD) add(id, DETECTOR.BLACKBOX, ['BLACKBOX'], true)

const metaLines = [...seen.values()].sort((a, b) => a.id.localeCompare(b.id)).map((r) => JSON.stringify(r))

const prose = []
function p(id, standard, cls, statement, ref) {
  prose.push(
    JSON.stringify({
      schema: CONTRACT_SCHEMA,
      type: 'prose_statement',
      id,
      standard,
      class: cls,
      statement,
      ...(ref ? { ref } : {}),
    }),
  )
}

const ha205 = [
  ['GLOBAL_LAW', 'Missing substrate changes status, not meaning'],
  ['GLOBAL_LAW', 'ACTIONABLE=0 does not prove a wrong-denominator fixed point'],
  ['FLOW_REQUIREMENT', 'Preview/CLI/validator is product only when intrinsic to the frozen contract'],
  ['GLOBAL_LAW', 'Canonical verified is not WEB2APP-complete'],
  ['GLOBAL_LAW', 'FIC coverage is not WEB2APP maturity; FIC denominator stays 55'],
  ['GLOBAL_LAW', 'WEB2APP does not rewrite the five-plane freeze'],
  ['ABUSE_INVARIANT', 'Agents may not receive do_whatever against the Internet'],
  ['GLOBAL_LAW', 'ACT != OBSERVE; WEB2APP_LEVEL independent of OBSERVATION_MATURITY'],
  ['CAPABILITY_REQUIREMENT', 'CAP13 owns inbound; CAP15 operations/observations; CAP12 credentials'],
  ['FLOW_REQUIREMENT', 'A webhook is not automatically final business state'],
  ['ABUSE_INVARIANT', 'Agents may not receive credential plaintext or keep an LLM watching a page'],
]
ha205.forEach((row, i) => p(`PROSE-205-HA-${String(i + 1).padStart(2, '0')}`, 'WEB2APP', row[0], row[1], '205§1'))

const ha206 = [
  ['GLOBAL_LAW', 'CompanyAgentRole is not an employment contract'],
  ['GLOBAL_LAW', 'chronica-ai-workforce Worker is not a Digital Employee'],
  ['GLOBAL_LAW', 'Skill does not imply authority'],
  ['GLOBAL_LAW', 'Installed tool does not imply authority'],
  ['GLOBAL_LAW', 'Credential existence does not imply eligibility'],
  ['GLOBAL_LAW', 'Title does not imply authority'],
  ['GLOBAL_LAW', 'Natural-language job description does not imply grants'],
  ['ABUSE_INVARIANT', 'Worker may not amend its own contract'],
  ['ABUSE_INVARIANT', 'Expired/revoked contract may not admit new work'],
  ['ABUSE_INVARIANT', 'WorkOrder may not broaden the employment contract'],
  ['FLOW_REQUIREMENT', 'Contract versions must remain historically reproducible'],
  ['FLOW_REQUIREMENT', 'Persistent digital employees work under effective contracts'],
  ['GLOBAL_LAW', 'Utilities/stateless algorithms do not require employment contracts'],
  ['EXPLANATORY_ONLY', 'This standard is not CAP16 implementation evidence'],
]
ha206.forEach((row, i) => p(`PROSE-206-HA-${String(i + 1).padStart(2, '0')}`, 'EMPLOYMENT', row[0], row[1], '206§1'))

const ha207 = [
  ['GLOBAL_LAW', 'A static artifact is not a Store'],
  ['GLOBAL_LAW', 'A provider project is not automatically a Store'],
  ['GLOBAL_LAW', 'Hosting preflight is not deployment'],
  ['GLOBAL_LAW', 'Provider success is not sufficient for Live'],
  ['ABUSE_INVARIANT', 'Store may not duplicate CRM/Helpdesk/Commerce as shadow systems by default'],
  ['GLOBAL_LAW', 'Business source-of-truth ownership must be explicit'],
  ['ABUSE_INVARIANT', 'Database census does not imply arbitrary SQL authority'],
  ['ABUSE_INVARIANT', 'StoreManifest cannot authorize production'],
  ['ABUSE_INVARIANT', 'Secrets must not be embedded in StoreManifest'],
  ['ABUSE_INVARIANT', 'An AI agent cannot self-promote production'],
  ['ABUSE_INVARIANT', 'Store cannot change DNS without rights/authority evidence'],
  ['GLOBAL_LAW', 'ALL-IN-ONE does not mean one giant crate'],
  ['FLOW_REQUIREMENT', 'A Store may use federated data without copying it'],
  ['FLOW_REQUIREMENT', 'Store-local state may exist'],
  ['GLOBAL_LAW', 'Provider identity is not Store identity'],
  ['FLOW_REQUIREMENT', 'A mature Store distinguishes desired from actual state'],
]
ha207.forEach((row, i) => p(`PROSE-207-HA-${String(i + 1).padStart(2, '0')}`, 'STORE', row[0], row[1], '207§3'))

const ha208 = [
  ['GLOBAL_LAW', 'Encryption at rest alone does not prove Black-Box'],
  ['GLOBAL_LAW', 'Vault alone does not prove Black-Box'],
  ['GLOBAL_LAW', 'OpaqueTenantRef alone does not prove Black-Box'],
  ['FLOW_REQUIREMENT', 'An Agent can be useful without raw business data'],
  ['FLOW_REQUIREMENT', 'A secret can be used without being readable'],
  ['GLOBAL_LAW', 'An opaque reference does not imply permission'],
  ['ABUSE_INVARIANT', 'Forbidden data must not be retrieved globally then filtered'],
  ['GLOBAL_LAW', 'Embeddings can remain sensitive'],
  ['GLOBAL_LAW', 'Summaries can remain sensitive'],
  ['ABUSE_INVARIANT', 'Logs/backups/plaintext search can defeat encrypted storage'],
  ['GLOBAL_LAW', 'A provider is not trusted internal memory'],
  ['ABUSE_INVARIANT', 'A Store must not receive entire company DB credentials by default'],
  ['FLOW_REQUIREMENT', 'A WorkOrder can narrow data visibility'],
  ['GLOBAL_LAW', 'Agent capability does not imply data visibility'],
  ['GLOBAL_LAW', 'Admin is not unlimited raw plaintext'],
  ['GLOBAL_LAW', 'B5 does not require a zero-knowledge server'],
  ['FLOW_REQUIREMENT', 'B6 requires real confidential-compute / ZK evidence'],
  ['EXPLANATORY_ONLY', 'This standard is not CAP18 implementation evidence'],
]
ha208.forEach((row, i) => p(`PROSE-208-HA-${String(i + 1).padStart(2, '0')}`, 'BLACKBOX', row[0], row[1], '208§3'))

for (const [id, rec] of seen) {
  p(`PROSE-AP-${id}`, rec.standards[0], 'ABUSE_INVARIANT', id, rec.id)
}

const sections = [
  ['WEB2APP', '205§0', 'EXPLANATORY_ONLY', 'One-line WEB2APP truth'],
  ['WEB2APP', '205§2', 'GLOBAL_LAW', 'Core WEB2APP law / Internet ABI'],
  ['WEB2APP', '205§19', 'ABUSE_INVARIANT', 'WEB2APP anti-pattern taxonomy'],
  ['WEB2APP', '205§20', 'DONOR_REFERENCE', 'Donor divergence notes'],
  ['WEB2APP', '205§21', 'GLOBAL_LAW', 'Score does not hide hard gates'],
  ['WEB2APP', '205§23', 'EXPLANATORY_ONLY', 'Calibration cases; not product evidence'],
  ['WEB2APP', '205§27', 'CAPABILITY_REQUIREMENT', 'Reactive WebApp observation plane'],
  ['WEB2APP', '205§28', 'CAPABILITY_REQUIREMENT', 'CAP12/CAP13/CAP15 triangle'],
  ['EMPLOYMENT', '206§7', 'MANUAL_REVIEW', 'Legal terminology / personhood not asserted'],
  ['EMPLOYMENT', '206§14', 'CAPABILITY_REQUIREMENT', 'E0–E6 maturity meanings'],
  ['EMPLOYMENT', '206§21', 'EXPLANATORY_ONLY', 'Current runtime calibration'],
  ['STORE', '207§11', 'CAPABILITY_REQUIREMENT', 'S0–S6 maturity meanings'],
  ['STORE', '207§14', 'EXPLANATORY_ONLY', 'Current Chronica Store calibration'],
  ['STORE', '207§15', 'DONOR_REFERENCE', 'Store donor census'],
  ['BLACKBOX', '208§4', 'CAPABILITY_REQUIREMENT', 'Trust zones and disclosure depth'],
  ['BLACKBOX', '208§9', 'CAPABILITY_REQUIREMENT', 'B0–B6 maturity meanings'],
  ['BLACKBOX', '208§13', 'EXPLANATORY_ONLY', 'Current Chronica Black-Box calibration'],
  ['BLACKBOX', '208§14', 'DONOR_REFERENCE', 'Black-Box donor census'],
  ['BLACKBOX', '208§5', 'MANUAL_REVIEW', 'Privacy/legal-purpose classification remains reviewable'],
]
sections.forEach((row, i) => p(`PROSE-SEC-${String(i + 1).padStart(2, '0')}`, row[0], row[2], row[3], row[1]))

writeFileSync(join(SOURCE, 'anti-pattern-meta.jsonl'), `${metaLines.join('\n')}\n`)
writeFileSync(join(SOURCE, 'prose-classification.jsonl'), `${prose.join('\n')}\n`)
console.log(JSON.stringify({ meta: metaLines.length, prose: prose.length }, null, 2))
