// Black-box evaluator. Detects semantic contradictions. Does not implement CAP18.

import { asArray, hasText, isObject, text } from './util.mjs'
import { evaluateBlackboxAntiPattern, evaluateBlackboxAntiPatternSet } from './blackbox-anti-patterns.mjs'
import {
  BLACKBOX_ANTI_PATTERNS,
  BLACKBOX_LEVELS,
  BLACKBOX_SCHEMA,
  BLACKBOX_STANDARD_VERSION,
  FACET_NAMES,
  FACET_STATES,
  FIXTURE_KINDS,
  HARD_FAILURE_TAGS,
  SHARED_WEB2APP_ANTI_PATTERNS,
  blackboxRank,
  minBlackbox,
} from './blackbox-vocab.mjs'

function flag(obj, key) {
  return isObject(obj) && obj[key] === true
}

function nest(record, key) {
  return isObject(record[key]) ? record[key] : {}
}

function claimedLevel(record) {
  const claimed = text(record.claimed_blackbox_maturity)
  return BLACKBOX_LEVELS.includes(claimed) ? claimed : 'B0'
}

function isFixture(record) {
  const kind = text(record.fixture_kind)
  return kind === 'BENCHMARK_FIXTURE_ONLY' || kind === 'BENCHMARK_FIXTURE'
}

export function validateBlackboxRecord(record) {
  const errors = []
  if (!isObject(record)) return { ok: false, errors: ['record must be an object'] }
  if (hasText(record.schema) && text(record.schema) !== BLACKBOX_SCHEMA) {
    errors.push(`schema must be ${BLACKBOX_SCHEMA}`)
  }
  if (hasText(record.claimed_blackbox_maturity) && !BLACKBOX_LEVELS.includes(text(record.claimed_blackbox_maturity))) {
    errors.push(`claimed_blackbox_maturity is invalid: ${record.claimed_blackbox_maturity}`)
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
  const atRest = nest(record, 'at_rest')
  const opaque = nest(record, 'opaque')
  const agent = nest(record, 'agent')
  const provider = nest(record, 'provider')
  const store = nest(record, 'store')
  const secrets = nest(record, 'secrets')
  const context = nest(record, 'context')
  const logging = nest(record, 'logging')
  const search = nest(record, 'search')
  const vector = nest(record, 'vector')
  const cache = nest(record, 'cache')
  const backup = nest(record, 'backup')
  const output = nest(record, 'output_dlp')
  const classify = nest(record, 'classification')
  const zk = nest(record, 'zero_knowledge')
  const cc = nest(record, 'confidential_compute')
  const derived = []
  const push = (tag, ev) => derived.push({ tag, evidence: { note: tag, ...ev } })
  const claimedHigh = blackboxRank(claimedLevel(record)) >= 5

  if ((flag(record, 'vault_only') || flag(atRest, 'sealed')) && claimedHigh) {
    if (flag(record, 'vault_only')) {
      push('VAULT_EXISTS_AS_BLACKBOX', { primitive_or_vault_or_aes: true, claimed_system_blackbox: true })
    }
    if (flag(atRest, 'sealed') && !flag(agent, 'semantic_view')) {
      push('ENCRYPTION_AT_REST_AS_BLACKBOX', { primitive_or_vault_or_aes: true, claimed_system_blackbox: true })
    }
  }
  if (flag(opaque, 'reversible') || flag(opaque, 'base64_raw_id')) {
    push('REVERSIBLE_OPAQUE_REFERENCE', { encoding_of_raw_id: true, base64_or_hex_of_raw_id: flag(opaque, 'base64_raw_id') })
  }
  if (flag(opaque, 'treated_as_authority') || flag(opaque, 'wrong_purpose_succeeds')) {
    push('OPAQUE_REF_AS_AUTHORITY', { handle_or_ref_known: true, treated_as_authorized: true })
  }
  if (flag(agent, 'raw_customer') || flag(agent, 'full_company_context')) {
    push('RAW_OBJECT_TO_AGENT', { agent_or_llm_received_raw_business_or_full_context: true })
    if (flag(agent, 'full_company_context')) {
      push('FULL_CONTEXT_TO_LLM', { agent_or_llm_received_raw_business_or_full_context: true })
    }
  }
  if (flag(agent, 'secret_in_context')) push('SECRET_IN_AGENT_CONTEXT', { secret_plaintext_present: true })
  if (flag(agent, 'self_authorizes_disclosure')) {
    push('AGENT_SELF_AUTHORIZES_DISCLOSURE', { agent_downgrades_or_authorizes_disclosure: true })
  }
  if (flag(provider, 'raw_internal_id')) {
    push('RAW_INTERNAL_ID_TO_PROVIDER', { raw_internal_primary_key_emitted: true, provider_received_raw_internal_topology_or_id: true })
    push('PROVIDER_FACADE_LEAKS_INTERNAL_TOPOLOGY', { provider_received_raw_internal_topology_or_id: true })
  }
  if (flag(provider, 'redact_after_send')) {
    push('REDACT_AFTER_PROVIDER', { raw_sent_then_asked_provider_to_redact: true })
  }
  if (flag(context, 'global_retrieve_then_filter')) {
    push('GLOBAL_RETRIEVE_THEN_FILTER', { global_retrieval_before_scope_filter: true })
  }
  if (flag(secrets, 'read_when_use_suffices')) {
    push('SECRET_READ_WHEN_USE_SUFFICES', { raw_secret_read: true, use_without_disclose_would_suffice: true })
  }
  if (flag(store, 'secret_in_manifest') || flag(secrets, 'in_store_manifest')) {
    push('SECRET_IN_STORE_MANIFEST', { secret_plaintext_present: true })
  }
  if (flag(store, 'db_secret_exposed')) {
    push('STORE_BINDING_EXPOSES_DATABASE_SECRET', { store_received_raw_db_or_domain_secret: true })
  }
  if (flag(logging, 'secret_plaintext') || flag(logging, 'pii_canary')) {
    push('SECRET_IN_LOG', { secret_plaintext_present: flag(logging, 'secret_plaintext') })
    push('LOG_AS_DATA_EXFILTRATION', { observability_or_audit_contains_raw_sensitive: true })
  }
  if (flag(logging, 'in_error')) push('SECRET_IN_ERROR', { secret_plaintext_present: true })
  if (flag(vector, 'plaintext') && (flag(atRest, 'sealed') || claimedHigh)) {
    push('PLAINTEXT_VECTOR_INDEX_BYPASS', { encrypted_primary_or_claimed_blackbox: true, plaintext_side_channel: true })
  }
  if (flag(search, 'plaintext') && (flag(atRest, 'sealed') || claimedHigh)) {
    push('PLAINTEXT_SEARCH_INDEX_BYPASS', { encrypted_primary_or_claimed_blackbox: true, plaintext_side_channel: true })
  }
  if (flag(cache, 'plaintext') && (flag(atRest, 'sealed') || claimedHigh)) {
    push('PLAINTEXT_CACHE_BYPASS', { encrypted_primary_or_claimed_blackbox: true, plaintext_side_channel: true })
  }
  if (flag(backup, 'plaintext') && (flag(atRest, 'sealed') || claimedHigh)) {
    push('PLAINTEXT_BACKUP_BYPASS', { encrypted_primary_or_claimed_blackbox: true, plaintext_side_channel: true })
  }
  if (flag(output, 'regex_only') && flag(output, 'claimed_complete')) {
    push('REGEX_ONLY_AS_DLP', { regex_scanner_only: true, claimed_output_dlp: true })
  }
  if (flag(classify, 'labeled') && !flag(classify, 'enforced')) {
    push('CLASSIFICATION_LABEL_WITHOUT_ENFORCEMENT', { label_stored: true, enforced: false })
  }
  if (flag(zk, 'claimed') && flag(zk, 'server_plaintext')) {
    push('ZERO_KNOWLEDGE_CLAIM_WITH_SERVER_PLAINTEXT', { zero_knowledge_claimed: true, server_plaintext: true })
  }
  if ((flag(cc, 'claimed') || blackboxRank(claimedLevel(record)) >= 6) && !flag(cc, 'attestation')) {
    push('CONFIDENTIAL_COMPUTE_CLAIM_WITHOUT_ATTESTATION', { claimed_confidential_compute_or_b6: true, attestation_evidence: false })
  }
  if (flag(context, 'pack_without_scope')) {
    push('CONTEXT_PACK_WITHOUT_SCOPE', { pack_issued: true, scope_bound: false })
  }
  if (flag(context, 'expired_pack_reused')) {
    push('CONTEXT_PACK_WITHOUT_EXPIRY', { pack_issued: true, expires: false, reused_after_revocation: true })
  }
  if (flag(cache, 'wrong_actor')) {
    push('VIEW_CACHE_SCOPE_CONFUSION', { cached_view_served_wrong_actor_or_purpose: true })
  }
  if (flag(record, 'crypto_helper_only') && claimedHigh) {
    push('CRYPTO_HELPER_WITHOUT_PRODUCT_BOUNDARY', { primitive_or_vault_or_aes: true, claimed_system_blackbox: true })
  }
  if (flag(vector, 'embedding_public') || flag(record, 'derived_as_public')) {
    push('DERIVED_DATA_AS_PUBLIC', { embedding_or_summary_treated_as_public: true, source_sensitive: true })
  }
  return derived
}

export function assignBlackboxMaturity(record) {
  const atRest = nest(record, 'at_rest')
  const opaque = nest(record, 'opaque')
  const secrets = nest(record, 'secrets')
  const agent = nest(record, 'agent')
  const provider = nest(record, 'provider')
  const store = nest(record, 'store')
  const context = nest(record, 'context')
  const logging = nest(record, 'logging')
  const search = nest(record, 'search')
  const vector = nest(record, 'vector')
  const cache = nest(record, 'cache')
  const backup = nest(record, 'backup')
  const output = nest(record, 'output_dlp')
  const tenant = nest(record, 'cross_tenant')
  const evidence = nest(record, 'evidence')
  const cc = nest(record, 'confidential_compute')
  const zk = nest(record, 'zero_knowledge')

  let level = 'B0'
  if (flag(atRest, 'sealed') || flag(record, 'vault_only') || flag(secrets, 'envelope')) level = 'B1'

  const opaqueOk = flag(opaque, 'boundary_ref') && !flag(opaque, 'reversible') && !flag(opaque, 'base64_raw_id')
  const handleOk = flag(secrets, 'handle') || flag(opaque, 'handle')
  if (blackboxRank(level) >= 1 && opaqueOk && handleOk) level = 'B2'

  const viewOk = flag(agent, 'semantic_view') && !flag(agent, 'raw_customer') && !flag(agent, 'secret_in_context')
  if (blackboxRank(level) >= 2 && viewOk) level = 'B3'

  const useOk = flag(secrets, 'use_without_disclose') || flag(agent, 'send_by_handle') || flag(store, 'handle_binding')
  if (blackboxRank(level) >= 3 && useOk && !flag(agent, 'secret_in_context')) level = 'B4'

  const sideSafe =
    !flag(logging, 'secret_plaintext') &&
    !flag(logging, 'pii_canary') &&
    !flag(vector, 'plaintext') &&
    !flag(search, 'plaintext') &&
    !flag(cache, 'plaintext') &&
    !flag(backup, 'plaintext') &&
    flag(provider, 'minimized') &&
    flag(output, 'enforced') &&
    flag(context, 'scope_before_retrieve') &&
    flag(tenant, 'negative')
  if (blackboxRank(level) >= 4 && sideSafe) level = 'B5'

  const b6ok =
    flag(cc, 'attestation') &&
    !flag(zk, 'server_plaintext') &&
    !isFixture(record) &&
    text(record.runtime_status) !== 'TEST_ONLY' &&
    text(record.runtime_status) !== 'DOC_ONLY' &&
    flag(evidence, 'production_evidenced')
  if (blackboxRank(level) >= 5 && b6ok) level = 'B6'

  if (flag(record, 'vault_only') || flag(record, 'crypto_helper_only')) level = minBlackbox(level, 'B2')
  if (flag(record, 'opaque_tenant_only')) level = minBlackbox(level, 'B2')
  if (!viewOk) level = minBlackbox(level, 'B2')
  if (!useOk || flag(agent, 'secret_in_context') || flag(store, 'db_secret_exposed')) level = minBlackbox(level, blackboxRank(level) >= 3 ? 'B3' : level)
  if (flag(logging, 'secret_plaintext') || flag(logging, 'pii_canary') || flag(vector, 'plaintext') || flag(search, 'plaintext') || flag(cache, 'plaintext') || flag(backup, 'plaintext')) {
    level = minBlackbox(level, 'B4')
  }
  if (!flag(output, 'enforced') || !flag(provider, 'minimized') || !flag(context, 'scope_before_retrieve')) {
    if (blackboxRank(level) >= 5) level = minBlackbox(level, 'B4')
  }
  if (!flag(tenant, 'negative') && blackboxRank(level) >= 5) level = minBlackbox(level, 'B4')
  if (!b6ok) level = minBlackbox(level, 'B5')
  if (flag(record, 'doc_only')) level = minBlackbox(level, 'B0')
  if (flag(zk, 'claimed') && flag(zk, 'server_plaintext')) level = minBlackbox(level, 'B5')
  return level
}

function semanticReview(record, assigned) {
  const flags = asArray(record.semantic_review_required).map(text).filter(Boolean)
  if (flag(nest(record, 'search'), 'blind_index')) {
    flags.push('SEMANTIC_REVIEW_REQUIRED: blind-index leakage (equality/frequency/query) is not zero-knowledge')
  }
  if (flag(record, 'classification_uncertain')) flags.push('SEMANTIC_REVIEW_REQUIRED: data-class / surface ownership')
  if (flag(nest(record, 'agent'), 'product_witness') && isFixture(record)) {
    flags.push('SEMANTIC_REVIEW_REQUIRED: fixture cannot prove product Agent-view witness')
  }
  return [...new Set(flags)]
}

export function evaluateBlackboxRecord(record) {
  const validation = validateBlackboxRecord(record)
  const claimed = claimedLevel(record)
  const assigned = assignBlackboxMaturity(record)
  const derived = deriveAntiPatternEvidence(record)
  const declared = asArray(record.anti_patterns).map((row) =>
    typeof row === 'string' ? { tag: row, evidence: record.anti_pattern_evidence?.[row] || { note: 'declared' } } : row,
  )
  const merged = [...declared, ...derived]
  const set = evaluateBlackboxAntiPatternSet(merged)
  for (const entry of merged) {
    const tag = text(entry.tag)
    if (![...BLACKBOX_ANTI_PATTERNS, ...SHARED_WEB2APP_ANTI_PATTERNS].includes(tag)) continue
    if (set.findings[tag] === 'NOT_EVALUATED' || set.findings[tag] == null) {
      const one = evaluateBlackboxAntiPattern(tag, entry.evidence || entry)
      if (one.finding === 'FOUND' && !set.found.includes(tag)) {
        set.found.push(tag)
        set.findings[tag] = 'FOUND'
      }
      if (!one.ok) validation.errors.push(...one.errors)
    }
  }

  const hardFailures = set.found.filter((tag) => HARD_FAILURE_TAGS.includes(tag))
  const hardGates = []
  if (blackboxRank(claimed) > blackboxRank(assigned)) hardGates.push('CLAIMED_MATURITY_EXCEEDS_EVIDENCE')
  if (flag(nest(record, 'cross_tenant'), 'decrypt_succeeds') || flag(nest(record, 'cross_tenant'), 'handle_succeeds')) {
    hardGates.push('CROSS_TENANT_BLACKBOX_FAILURE')
  }
  if (flag(nest(record, 'web2app'), 'implies_blackbox') || flag(nest(record, 'employment'), 'implies_blackbox') || flag(nest(record, 'store'), 'implies_blackbox')) {
    hardGates.push('AXIS_COLLAPSED_INTO_BLACKBOX')
  }
  if (blackboxRank(claimed) >= 3 && !flag(nest(record, 'agent'), 'semantic_view') && !flag(nest(record, 'agent'), 'product_witness')) {
    if (claimed === 'B3' || blackboxRank(claimed) > 3) hardGates.push('B3_WITHOUT_SEMANTIC_VIEW_WITNESS')
  }
  if (blackboxRank(claimed) >= 4 && !(flag(nest(record, 'secrets'), 'use_without_disclose') || flag(nest(record, 'agent'), 'send_by_handle'))) {
    hardGates.push('B4_WITHOUT_USE_WITHOUT_DISCLOSE_WITNESS')
  }
  if (flag(nest(record, 'context'), 'expired_pack_reused')) hardGates.push('EXPIRED_CONTEXT_PACK_REUSED')
  if (flag(nest(record, 'cache'), 'wrong_actor')) hardGates.push('VIEW_CACHE_SCOPE_CONFUSION')
  if (flag(nest(record, 'handle'), 'wrong_purpose_succeeds') || flag(nest(record, 'opaque'), 'wrong_purpose_succeeds')) {
    hardGates.push('WRONG_PURPOSE_HANDLE_SUCCEEDED')
  }
  if (blackboxRank(claimed) >= 5) {
    const logging = nest(record, 'logging')
    const side =
      flag(logging, 'secret_plaintext') ||
      flag(logging, 'pii_canary') ||
      flag(nest(record, 'vector'), 'plaintext') ||
      flag(nest(record, 'search'), 'plaintext') ||
      flag(nest(record, 'cache'), 'plaintext') ||
      flag(nest(record, 'backup'), 'plaintext') ||
      !flag(nest(record, 'output_dlp'), 'enforced') ||
      !flag(nest(record, 'provider'), 'minimized')
    if (side) hardGates.push('B5_WITH_PLAINTEXT_SIDE_CHANNEL_OR_MISSING_BOUNDARY')
  }

  const review = semanticReview(record, assigned)
  let verdict = 'PASS'
  if (hardFailures.length || hardGates.length) verdict = 'FAIL'
  else if (review.length) verdict = 'PARTIAL'

  return {
    ok: validation.ok && set.ok,
    errors: [...validation.errors, ...set.errors],
    schema: BLACKBOX_SCHEMA,
    standard_version: BLACKBOX_STANDARD_VERSION,
    case_id: text(record.case_id) || text(record.capability_key),
    fixture_kind: text(record.fixture_kind),
    surface: text(record.surface) || 'global',
    claimed_blackbox_maturity: claimed,
    blackbox_maturity: assigned,
    canonical_status: text(record.canonical_status) || null,
    runtime_status: text(record.runtime_status) || null,
    product_status: text(record.product_status) || null,
    production_evidence: flag(nest(record, 'evidence'), 'production_evidenced') && !isFixture(record),
    verdict,
    hard_gate_failures: hardGates,
    hard_failures: hardFailures,
    anti_patterns_found: set.found,
    anti_pattern_findings: set.findings,
    semantic_review_required: review,
    facets: isObject(record.facets) ? record.facets : {},
    web2app_independent: nest(record, 'web2app').independent !== false,
    employment_independent: nest(record, 'employment').independent !== false,
    store_independent: nest(record, 'store').independent !== false,
    this_standard_is_not_cap18: true,
    notes: record.notes || null,
  }
}
