// Black-box anti-pattern evaluation from evidence objects, not keyword grep.

import { evaluateWeb2AppAntiPattern } from './web2app-anti-patterns.mjs'
import { asArray, isObject, text } from './util.mjs'
import { BLACKBOX_ANTI_PATTERNS, SHARED_WEB2APP_ANTI_PATTERNS } from './blackbox-vocab.mjs'

function foundIf(condition) {
  return condition ? 'FOUND' : 'NOT_FOUND'
}

export function evaluateBlackboxAntiPattern(tag, evidence) {
  if (SHARED_WEB2APP_ANTI_PATTERNS.includes(tag)) {
    return evaluateWeb2AppAntiPattern(tag, evidence)
  }
  const errors = []
  if (!BLACKBOX_ANTI_PATTERNS.includes(tag)) {
    return { ok: false, finding: null, errors: [`unknown blackbox anti-pattern ${tag}`] }
  }
  if (typeof evidence === 'string') return { ok: true, finding: 'NOT_EVALUATED', errors: [] }
  if (!isObject(evidence)) {
    return { ok: false, finding: null, errors: [`${tag}: evidence object required (keyword grep is not enough)`] }
  }
  if (evidence.keyword_only === true || evidence.detected_by === 'grep') {
    return { ok: false, finding: null, errors: [`${tag}: do not mark merely by keyword grep`] }
  }

  let finding = 'NOT_FOUND'
  switch (tag) {
    case 'ENCRYPTION_AT_REST_AS_BLACKBOX':
    case 'VAULT_EXISTS_AS_BLACKBOX':
    case 'CRYPTO_HELPER_WITHOUT_PRODUCT_BOUNDARY':
      finding = foundIf(evidence.primitive_or_vault_or_aes === true && evidence.claimed_system_blackbox === true)
      break
    case 'OPAQUE_REF_AS_AUTHORITY':
      finding = foundIf(evidence.handle_or_ref_known === true && evidence.treated_as_authorized === true)
      break
    case 'REVERSIBLE_OPAQUE_REFERENCE':
      finding = foundIf(evidence.encoding_of_raw_id === true || evidence.base64_or_hex_of_raw_id === true)
      break
    case 'RAW_OBJECT_TO_AGENT':
    case 'FULL_CONTEXT_TO_LLM':
      finding = foundIf(evidence.agent_or_llm_received_raw_business_or_full_context === true)
      break
    case 'FULL_CONTEXT_TO_PROVIDER':
    case 'PROVIDER_FACADE_LEAKS_INTERNAL_TOPOLOGY':
      finding = foundIf(evidence.provider_received_raw_internal_topology_or_id === true)
      break
    case 'REDACT_AFTER_PROVIDER':
      finding = foundIf(evidence.raw_sent_then_asked_provider_to_redact === true)
      break
    case 'GLOBAL_RETRIEVE_THEN_FILTER':
      finding = foundIf(evidence.global_retrieval_before_scope_filter === true)
      break
    case 'SECRET_READ_WHEN_USE_SUFFICES':
      finding = foundIf(evidence.raw_secret_read === true && evidence.use_without_disclose_would_suffice === true)
      break
    case 'SECRET_IN_AGENT_CONTEXT':
    case 'SECRET_IN_STORE_MANIFEST':
    case 'SECRET_IN_LOG':
    case 'SECRET_IN_ERROR':
    case 'SECRET_IN_AUDIT':
      finding = foundIf(evidence.secret_plaintext_present === true)
      break
    case 'RAW_INTERNAL_ID_TO_PROVIDER':
    case 'RAW_INTERNAL_ID_TO_ANALYTICS':
      finding = foundIf(evidence.raw_internal_primary_key_emitted === true)
      break
    case 'PLAINTEXT_CACHE_BYPASS':
    case 'PLAINTEXT_SEARCH_INDEX_BYPASS':
    case 'PLAINTEXT_VECTOR_INDEX_BYPASS':
    case 'PLAINTEXT_BACKUP_BYPASS':
    case 'PLAINTEXT_QUEUE_BYPASS':
    case 'PLAINTEXT_EVENT_BYPASS':
      finding = foundIf(evidence.encrypted_primary_or_claimed_blackbox === true && evidence.plaintext_side_channel === true)
      break
    case 'LOG_AS_DATA_EXFILTRATION':
    case 'AUDIT_AS_RAW_DUMP':
      finding = foundIf(evidence.observability_or_audit_contains_raw_sensitive === true)
      break
    case 'HASH_AS_ENCRYPTION':
      finding = foundIf(evidence.hash_claimed_as_encryption === true)
      break
    case 'PSEUDONYM_AS_ANONYMITY':
      finding = foundIf(evidence.opaque_or_pseudonym_claimed_anonymous === true)
      break
    case 'DERIVED_DATA_AS_PUBLIC':
      finding = foundIf(evidence.embedding_or_summary_treated_as_public === true && evidence.source_sensitive === true)
      break
    case 'REGEX_ONLY_AS_DLP':
      finding = foundIf(evidence.regex_scanner_only === true && evidence.claimed_output_dlp === true)
      break
    case 'AGENT_SELF_AUTHORIZES_DISCLOSURE':
      finding = foundIf(evidence.agent_downgrades_or_authorizes_disclosure === true)
      break
    case 'CLASSIFICATION_LABEL_WITHOUT_ENFORCEMENT':
      finding = foundIf(evidence.label_stored === true && evidence.enforced !== true)
      break
    case 'CONTEXT_PACK_WITHOUT_SCOPE':
      finding = foundIf(evidence.pack_issued === true && evidence.scope_bound !== true)
      break
    case 'CONTEXT_PACK_WITHOUT_EXPIRY':
      finding = foundIf(evidence.pack_issued === true && evidence.expires !== true && evidence.reused_after_revocation === true)
      break
    case 'VIEW_CACHE_SCOPE_CONFUSION':
      finding = foundIf(evidence.cached_view_served_wrong_actor_or_purpose === true)
      break
    case 'STORE_BINDING_EXPOSES_DATABASE_SECRET':
      finding = foundIf(evidence.store_received_raw_db_or_domain_secret === true)
      break
    case 'CONFIDENTIAL_COMPUTE_CLAIM_WITHOUT_ATTESTATION':
      finding = foundIf(evidence.claimed_confidential_compute_or_b6 === true && evidence.attestation_evidence !== true)
      break
    case 'ZERO_KNOWLEDGE_CLAIM_WITH_SERVER_PLAINTEXT':
      finding = foundIf(evidence.zero_knowledge_claimed === true && evidence.server_plaintext === true)
      break
    default:
      break
  }

  if (finding === 'FOUND' && asArray(evidence.evidence_paths).length === 0 && !text(evidence.note)) {
    errors.push(`${tag}: FOUND requires evidence_paths or note`)
  }
  return { ok: errors.length === 0, finding, errors }
}

export function evaluateBlackboxAntiPatternSet(entries) {
  const errors = []
  const findings = {}
  const found = []
  const tags = [...BLACKBOX_ANTI_PATTERNS, ...SHARED_WEB2APP_ANTI_PATTERNS]
  for (const tag of tags) {
    const entry = asArray(entries).find((row) => text(row.tag) === tag)
    if (!entry) {
      findings[tag] = 'NOT_EVALUATED'
      continue
    }
    const result = evaluateBlackboxAntiPattern(tag, entry.evidence || entry)
    findings[tag] = result.finding
    if (!result.ok) errors.push(...result.errors)
    if (result.finding === 'FOUND') found.push(tag)
  }
  return { ok: errors.length === 0, errors, findings, found }
}
