// Store Platform evaluator. Detects semantic contradictions. Does not implement CAP17.

import { asArray, hasText, isObject, text } from './util.mjs'
import { evaluateStoreAntiPattern, evaluateStoreAntiPatternSet } from './store-anti-patterns.mjs'
import {
  FACET_NAMES,
  FACET_STATES,
  FIXTURE_KINDS,
  HARD_FAILURE_TAGS,
  SHARED_WEB2APP_ANTI_PATTERNS,
  STORE_ANTI_PATTERNS,
  STORE_LEVELS,
  STORE_SCHEMA,
  STORE_STANDARD_VERSION,
  minStore,
  storeRank,
} from './store-vocab.mjs'

function flag(obj, key) {
  return isObject(obj) && obj[key] === true
}

function nest(record, key) {
  return isObject(record[key]) ? record[key] : {}
}

function claimedLevel(record) {
  const claimed = text(record.claimed_store_maturity)
  return STORE_LEVELS.includes(claimed) ? claimed : 'S0'
}

export function validateStoreRecord(record) {
  const errors = []
  if (!isObject(record)) return { ok: false, errors: ['record must be an object'] }
  if (hasText(record.schema) && text(record.schema) !== STORE_SCHEMA) {
    errors.push(`schema must be ${STORE_SCHEMA}`)
  }
  if (hasText(record.claimed_store_maturity) && !STORE_LEVELS.includes(text(record.claimed_store_maturity))) {
    errors.push(`claimed_store_maturity is invalid: ${record.claimed_store_maturity}`)
  }
  if (hasText(record.fixture_kind) && !FIXTURE_KINDS.includes(text(record.fixture_kind))) {
    errors.push(`fixture_kind is invalid: ${record.fixture_kind}`)
  }
  const facets = isObject(record.facets) ? record.facets : {}
  for (const [name, state] of Object.entries(facets)) {
    if (hasText(name) && !FACET_NAMES.includes(name)) errors.push(`unknown facet ${name}`)
    if (hasText(state) && !FACET_STATES.includes(text(state))) errors.push(`facet ${name} has invalid state ${state}`)
  }
  return { ok: errors.length === 0, errors }
}

function deriveAntiPatternEvidence(record) {
  const identity = nest(record, 'identity')
  const tenant = nest(record, 'tenant')
  const manifest = nest(record, 'manifest')
  const actual = nest(record, 'actual')
  const graph = nest(record, 'resource_graph')
  const deploy = nest(record, 'deployment')
  const domain = nest(record, 'domain')
  const database = nest(record, 'database')
  const data = nest(record, 'data')
  const recon = nest(record, 'reconciliation')
  const agent = nest(record, 'agent')
  const derived = []
  const push = (tag, ev) => derived.push({ tag, evidence: { note: tag, ...ev } })
  const claimedStore = storeRank(claimedLevel(record)) >= 1

  if ((flag(record, 'preflight_only') || flag(record, 'hosting_preflight')) && flag(record, 'claimed_as_deploy')) {
    push('HOSTING_PREFLIGHT_AS_DEPLOYMENT', { preflight_or_clear_for_review: true, claimed_as_deploy: true })
  }
  if ((flag(record, 'static_artifact_only') || flag(record, 'artifact_only')) && claimedStore) {
    push('STATIC_ARTIFACT_AS_LIVE_STORE', { artifact_or_html_bundle: true, claimed_as_store: true, resource_graph: flag(graph, 'queryable') })
  }
  if ((flag(record, 'provider_project_only') || flag(identity, 'provider_id_as_store_id')) && claimedStore) {
    push('PROVIDER_PROJECT_AS_STORE', { provider_project_or_resource: true, claimed_as_store: true, chronica_store_identity: flag(identity, 'stable_store_id') && !flag(identity, 'provider_id_as_store_id') })
    push('PROVIDER_LOCK_IN_AS_STORE_IDENTITY', { provider_id_is_store_id: true })
  }
  if (flag(record, 'database_provider_as_store') && claimedStore) {
    push('DATABASE_PROVIDER_AS_STORE', { database_project: true, claimed_as_store: true })
  }
  if (flag(data, 'shadow_canonical') || flag(data, 'writable_duplicate')) {
    push('SHADOW_COMPANY_OS', { writable_duplicate_of_canonical_business_object: true })
  }
  if (flag(data, 'store_owns_crm') || flag(data, 'store_owns_helpdesk') || flag(data, 'store_owns_commerce')) {
    push('COMPOSITION_ROOT_AS_DOMAIN_OWNER', { store_touches_domain: true, store_claims_kernel_ownership: true })
    if (flag(data, 'store_owns_crm')) push('STORE_AS_CRM', { store_owns_canonical_domain_kernel: true })
    if (flag(data, 'store_owns_helpdesk')) push('STORE_AS_HELPDESK', { store_owns_canonical_domain_kernel: true })
    if (flag(data, 'store_owns_commerce')) push('STORE_AS_COMMERCE', { store_owns_canonical_domain_kernel: true })
  }
  if (flag(data, 'bypasses_kernel')) {
    push('DATABASE_BYPASSES_BUSINESS_KERNEL', { business_write_via_raw_sql_or_table: true, typed_domain_operation: false })
  }
  if (flag(data, 'business_bearing') && !flag(data, 'source_of_truth_map')) {
    push('DATA_WITHOUT_SOURCE_OF_TRUTH', { business_bearing_binding: true, authoritative_owner: false })
  }
  if (flag(database, 'census_as_sql_authority')) {
    push('SCHEMA_CENSUS_AS_SQL_AUTHORITY', { can_introspect: true, treated_as_arbitrary_sql: true })
  }
  if (flag(manifest, 'used_as_authority') || (flag(manifest, 'production_flag') && flag(deploy, 'production') && !flag(deploy, 'production_approved'))) {
    push('MANIFEST_AS_AUTHORITY', { manifest_production_flag: true, production_authorized: false, claimed_production: true })
  }
  if (flag(manifest, 'contains_secret_plaintext')) {
    push('SECRET_IN_STORE_MANIFEST', { plaintext_secret_in_manifest: true })
  }
  if (flag(actual, 'provider_success') && flag(actual, 'health_failed') && (flag(actual, 'claimed_live') || storeRank(claimedLevel(record)) >= 3)) {
    push('PROVIDER_200_AS_LIVE', { provider_success: true, health_or_routing_observed: false, claimed_live: true })
  }
  if (flag(deploy, 'executed') && !flag(deploy, 'receipt')) {
    push('DEPLOY_WITHOUT_RECEIPT', { deploy_executed: true, receipt_persisted: false })
    push('EXECUTION_WITHOUT_RECEIPT', { external_effect: true, receipt_persisted: false })
  }
  if (flag(deploy, 'rollback_message_only')) {
    push('ROLLBACK_AS_MESSAGE', { rollback_is_log_or_message_only: true, claimed_rollback: true })
  }
  if (flag(domain, 'dns_mutated') && !flag(domain, 'rights_evidence')) {
    push('DNS_WITHOUT_RIGHTS', { dns_or_domain_mutated: true, rights_and_authority: false })
  }
  if (flag(recon, 'drift_auto_mutates')) {
    push('DRIFT_AS_AUTO_MUTATION', { drift_detected: true, auto_mutated_without_authorization: true })
  }
  if (flag(agent, 'self_authorized_production')) {
    push('AGENT_AS_DEPLOY_AUTHORITY', { agent_promotes_or_deploys_production: true, control_authorized: false })
  }
  if (flag(record, 'persistence_adapter_named_store') && claimedStore) {
    push('PERSISTENCE_ADAPTER_AS_STORE', { repository_or_adapter_named_store: true, claimed_cap17_store: true })
  }
  if (flag(data, 'frontend_event_as_fact')) {
    push('FRONTEND_EVENT_AS_BUSINESS_FACT', { frontend_event_claimed_as_canonical_business_change: true, canonical_state_change: false })
  }
  if (flag(tenant, 'unscoped_company_access')) {
    push('STORE_HAS_ALL_COMPANY_PERMISSIONS', { store_exists: true, unscoped_company_access: true })
  }
  if (storeRank(claimedLevel(record)) >= 5 && flag(nest(record, 'desired'), 'present') && !flag(recon, 'present')) {
    push('CONFIG_WITHOUT_RECONCILIATION', { desired_or_config_recorded: true, actual_observed: flag(actual, 'observed'), claimed_reconciled: true })
  }
  if (flag(identity, 'kind_as_authority')) {
    push('STORE_KIND_AS_AUTHORITY', { kind_label_authorizes: true })
  }
  return derived
}

export function assignStoreMaturity(record) {
  const identity = nest(record, 'identity')
  const tenant = nest(record, 'tenant')
  const manifest = nest(record, 'manifest')
  const desired = nest(record, 'desired')
  const actual = nest(record, 'actual')
  const graph = nest(record, 'resource_graph')
  const preview = nest(record, 'preview')
  const deploy = nest(record, 'deployment')
  const domain = nest(record, 'domain')
  const secrets = nest(record, 'secrets')
  const database = nest(record, 'database')
  const data = nest(record, 'data')
  const business = nest(record, 'business_bindings')
  const recon = nest(record, 'reconciliation')
  const evidence = nest(record, 'evidence')

  if (flag(record, 'static_artifact_only') || flag(record, 'artifact_only') || flag(record, 'preflight_only') || flag(record, 'provider_project_only')) {
    if (!(flag(identity, 'stable_store_id') && flag(tenant, 'company_bound') && (flag(manifest, 'present') || flag(desired, 'present')))) {
      return 'S0'
    }
  }

  let level = 'S0'
  const identityOk = flag(identity, 'stable_store_id') && !flag(identity, 'provider_id_as_store_id') && flag(tenant, 'company_bound')
  if (identityOk && (flag(manifest, 'present') || flag(desired, 'present') || flag(graph, 'declared'))) level = 'S1'

  const previewOk = flag(graph, 'durable') && flag(graph, 'queryable') && flag(preview, 'live') && flag(actual, 'observed') && flag(tenant, 'isolated')
  if (storeRank(level) >= 1 && previewOk) level = 'S2'

  const deployOk = flag(deploy, 'executed') && flag(deploy, 'receipt') && !flag(actual, 'health_failed')
  const bindingOk = flag(database, 'bound') || flag(business, 'present') || flag(data, 'bound')
  if (storeRank(level) >= 2 && deployOk && bindingOk) level = 'S3'

  const prodOk =
    flag(deploy, 'production') &&
    flag(deploy, 'production_approved') &&
    !flag(manifest, 'used_as_authority') &&
    (flag(secrets, 'handle_only') || !flag(manifest, 'contains_secret_plaintext')) &&
    flag(deploy, 'rollback') &&
    flag(tenant, 'cross_tenant_negative') &&
    flag(actual, 'observed') &&
    (!flag(domain, 'required') || (flag(domain, 'rights_evidence') && flag(domain, 'observed')))
  if (storeRank(level) >= 3 && prodOk) level = 'S4'

  const reconOk =
    flag(recon, 'present') &&
    flag(desired, 'present') &&
    flag(actual, 'observed') &&
    flag(data, 'source_of_truth_map') &&
    (flag(database, 'census') || flag(data, 'federated') || flag(business, 'present')) &&
    !flag(recon, 'drift_auto_mutates')
  if (storeRank(level) >= 4 && reconOk) level = 'S5'

  const prodEvidence =
    flag(evidence, 'production_evidenced') &&
    text(record.fixture_kind) !== 'BENCHMARK_FIXTURE' &&
    text(record.runtime_status) !== 'TEST_ONLY' &&
    text(record.runtime_status) !== 'DOC_ONLY'
  if (storeRank(level) >= 5 && prodEvidence) level = 'S6'

  if (flag(record, 'preflight_only') || flag(record, 'static_artifact_only') || flag(record, 'provider_project_only')) {
    level = minStore(level, identityOk && flag(manifest, 'present') ? 'S1' : 'S0')
  }
  if (!flag(graph, 'durable') || !flag(preview, 'live')) {
    if (!(flag(deploy, 'executed') && flag(deploy, 'receipt'))) level = minStore(level, 'S1')
  }
  if (!deployOk) level = minStore(level, previewOk ? 'S2' : level)
  if (flag(deploy, 'executed') && !flag(deploy, 'receipt')) level = minStore(level, 'S2')
  if (flag(actual, 'health_failed')) level = minStore(level, 'S2')
  if (!bindingOk) level = minStore(level, 'S2')
  if (flag(deploy, 'production') && !flag(deploy, 'production_approved')) level = minStore(level, 'S3')
  if (!reconOk) level = minStore(level, storeRank(level) >= 4 ? 'S4' : level)
  if (!flag(data, 'source_of_truth_map') && storeRank(level) >= 5) level = minStore(level, 'S4')
  if (!prodEvidence) level = minStore(level, 'S5')
  if (flag(record, 'doc_only')) level = minStore(level, 'S0')
  return level
}

function semanticReview(record, assigned) {
  const flags = asArray(record.semantic_review_required).map(text).filter(Boolean)
  const database = nest(record, 'database')
  const data = nest(record, 'data')
  if (flag(database, 'census') && flag(database, 'unclassified')) {
    flags.push('SEMANTIC_REVIEW_REQUIRED: unclassified database objects remain UNKNOWN')
  }
  if (flag(data, 'federated') && !hasText(data.authoritative_owner)) {
    flags.push('SEMANTIC_REVIEW_REQUIRED: federated source-of-truth owner')
  }
  if (flag(record, 'classification_uncertain')) flags.push('SEMANTIC_REVIEW_REQUIRED: store vs substrate classification')
  if (storeRank(assigned) >= 5 && !flag(data, 'source_of_truth_map')) {
    flags.push('SEMANTIC_REVIEW_REQUIRED: S5 without explicit ownership map')
  }
  return [...new Set(flags)]
}

export function evaluateStoreRecord(record) {
  const validation = validateStoreRecord(record)
  const claimed = claimedLevel(record)
  const assigned = assignStoreMaturity(record)
  const derived = deriveAntiPatternEvidence(record)
  const declared = asArray(record.anti_patterns).map((row) =>
    typeof row === 'string' ? { tag: row, evidence: record.anti_pattern_evidence?.[row] || { note: 'declared' } } : row,
  )
  const merged = [...declared, ...derived]
  const set = evaluateStoreAntiPatternSet(merged)
  for (const entry of merged) {
    const tag = text(entry.tag)
    if (![...STORE_ANTI_PATTERNS, ...SHARED_WEB2APP_ANTI_PATTERNS].includes(tag)) continue
    if (set.findings[tag] === 'NOT_EVALUATED' || set.findings[tag] == null) {
      const one = evaluateStoreAntiPattern(tag, entry.evidence || entry)
      if (one.finding === 'FOUND' && !set.found.includes(tag)) {
        set.found.push(tag)
        set.findings[tag] = 'FOUND'
      }
      if (!one.ok) validation.errors.push(...one.errors)
    }
  }

  const hardFailures = set.found.filter((tag) => HARD_FAILURE_TAGS.includes(tag))
  const hardGates = []
  if (storeRank(claimed) > storeRank(assigned)) hardGates.push('CLAIMED_MATURITY_EXCEEDS_EVIDENCE')
  if (flag(nest(record, 'tenant'), 'cross_tenant')) hardGates.push('CROSS_TENANT_STORE_RESOURCE_ACCESS')
  if (flag(nest(record, 'web2app'), 'implies_store') || flag(nest(record, 'employment'), 'implies_store')) {
    hardGates.push('AXIS_COLLAPSED_INTO_STORE')
  }
  if (flag(nest(record, 'deployment'), 'production') && !flag(nest(record, 'deployment'), 'production_approved') && storeRank(claimed) >= 4) {
    hardGates.push('PRODUCTION_WITHOUT_AUTHORITY')
  }

  const review = semanticReview(record, assigned)
  let verdict = 'PASS'
  if (hardFailures.length || hardGates.length) verdict = 'FAIL'
  else if (review.length) verdict = 'PARTIAL'

  return {
    ok: validation.ok && set.ok,
    errors: [...validation.errors, ...set.errors],
    schema: STORE_SCHEMA,
    standard_version: STORE_STANDARD_VERSION,
    case_id: text(record.case_id) || text(record.capability_key),
    fixture_kind: text(record.fixture_kind),
    claimed_store_maturity: claimed,
    store_maturity: assigned,
    canonical_status: text(record.canonical_status) || null,
    runtime_status: text(record.runtime_status) || null,
    product_status: text(record.product_status) || null,
    production_evidence: flag(nest(record, 'evidence'), 'production_evidenced') && text(record.fixture_kind) !== 'BENCHMARK_FIXTURE',
    verdict,
    hard_gate_failures: hardGates,
    hard_failures: hardFailures,
    anti_patterns_found: set.found,
    anti_pattern_findings: set.findings,
    semantic_review_required: review,
    facets: isObject(record.facets) ? record.facets : {},
    web2app_independent: nest(record, 'web2app').independent !== false,
    employment_independent: nest(record, 'employment').independent !== false,
    this_standard_is_not_cap17: true,
    notes: record.notes || null,
  }
}
