// Store anti-pattern evaluation from evidence objects, not keyword grep.

import { evaluateWeb2AppAntiPattern } from './web2app-anti-patterns.mjs'
import { asArray, isObject, text } from './util.mjs'
import { SHARED_WEB2APP_ANTI_PATTERNS, STORE_ANTI_PATTERNS } from './store-vocab.mjs'

function foundIf(condition) {
  return condition ? 'FOUND' : 'NOT_FOUND'
}

export function evaluateStoreAntiPattern(tag, evidence) {
  if (SHARED_WEB2APP_ANTI_PATTERNS.includes(tag)) {
    return evaluateWeb2AppAntiPattern(tag, evidence)
  }
  const errors = []
  if (!STORE_ANTI_PATTERNS.includes(tag)) {
    return { ok: false, finding: null, errors: [`unknown store anti-pattern ${tag}`] }
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
    case 'HOSTING_PREFLIGHT_AS_DEPLOYMENT':
      finding = foundIf(evidence.preflight_or_clear_for_review === true && evidence.claimed_as_deploy === true)
      break
    case 'STATIC_ARTIFACT_AS_LIVE_STORE':
      finding = foundIf(evidence.artifact_or_html_bundle === true && evidence.claimed_as_store === true && evidence.resource_graph !== true)
      break
    case 'PROVIDER_RESOURCE_AS_STORE':
    case 'PROVIDER_PROJECT_AS_STORE':
      finding = foundIf(evidence.provider_project_or_resource === true && evidence.claimed_as_store === true && evidence.chronica_store_identity !== true)
      break
    case 'DATABASE_PROVIDER_AS_STORE':
      finding = foundIf(evidence.database_project === true && evidence.claimed_as_store === true)
      break
    case 'SHADOW_COMPANY_OS':
      finding = foundIf(evidence.writable_duplicate_of_canonical_business_object === true)
      break
    case 'COMPOSITION_ROOT_AS_DOMAIN_OWNER':
      finding = foundIf(evidence.store_touches_domain === true && evidence.store_claims_kernel_ownership === true)
      break
    case 'DATABASE_BYPASSES_BUSINESS_KERNEL':
      finding = foundIf(evidence.business_write_via_raw_sql_or_table === true && evidence.typed_domain_operation !== true)
      break
    case 'DATABASE_DUPLICATES_BUSINESS_KERNEL':
      finding = foundIf(evidence.store_table_named_like_canonical === true && evidence.authoritative_owner_is_store === true)
      break
    case 'DATA_WITHOUT_SOURCE_OF_TRUTH':
      finding = foundIf(evidence.business_bearing_binding === true && evidence.authoritative_owner !== true)
      break
    case 'RESOURCE_WITHOUT_OWNER':
      finding = foundIf(evidence.resource_bound === true && evidence.owner_declared !== true)
      break
    case 'BINDING_WITHOUT_SCOPE':
      finding = foundIf(evidence.binding_present === true && evidence.scope_declared !== true)
      break
    case 'STORE_HAS_ALL_COMPANY_PERMISSIONS':
      finding = foundIf(evidence.store_exists === true && evidence.unscoped_company_access === true)
      break
    case 'SCHEMA_CENSUS_AS_SQL_AUTHORITY':
      finding = foundIf(evidence.can_introspect === true && evidence.treated_as_arbitrary_sql === true)
      break
    case 'MANIFEST_AS_AUTHORITY':
      finding = foundIf(evidence.manifest_production_flag === true && evidence.production_authorized !== true && evidence.claimed_production === true)
      break
    case 'CONFIG_WITHOUT_RECONCILIATION':
      finding = foundIf(evidence.desired_or_config_recorded === true && evidence.actual_observed !== true && evidence.claimed_reconciled === true)
      break
    case 'DRIFT_AS_AUTO_MUTATION':
      finding = foundIf(evidence.drift_detected === true && evidence.auto_mutated_without_authorization === true)
      break
    case 'PROVIDER_200_AS_LIVE':
      finding = foundIf(evidence.provider_success === true && evidence.health_or_routing_observed !== true && evidence.claimed_live === true)
      break
    case 'DEPLOY_WITHOUT_RECEIPT':
      finding = foundIf(evidence.deploy_executed === true && evidence.receipt_persisted !== true)
      break
    case 'ROLLBACK_AS_MESSAGE':
      finding = foundIf(evidence.rollback_is_log_or_message_only === true && evidence.claimed_rollback === true)
      break
    case 'DNS_WITHOUT_RIGHTS':
      finding = foundIf(evidence.dns_or_domain_mutated === true && evidence.rights_and_authority !== true)
      break
    case 'SECRET_IN_STORE_MANIFEST':
      finding = foundIf(evidence.plaintext_secret_in_manifest === true)
      break
    case 'SECRET_LEAK_IN_LOG':
      finding = foundIf(evidence.secret_plaintext_in_logs_api_ui_or_agent === true)
      break
    case 'AGENT_AS_DEPLOY_AUTHORITY':
      finding = foundIf(evidence.agent_promotes_or_deploys_production === true && evidence.control_authorized !== true)
      break
    case 'UI_WITHOUT_STORE_RUNTIME':
      finding = foundIf(evidence.ui_exists === true && evidence.store_runtime !== true && evidence.claimed_store === true)
      break
    case 'STORE_KIND_AS_AUTHORITY':
      finding = foundIf(evidence.kind_label_authorizes === true)
      break
    case 'ALL_IN_ONE_MONOLITH':
      finding = foundIf(evidence.one_crate_or_one_copied_database_claimed_as_all_in_one === true)
      break
    case 'STORE_AS_CRM':
    case 'STORE_AS_HELPDESK':
    case 'STORE_AS_COMMERCE':
      finding = foundIf(evidence.store_owns_canonical_domain_kernel === true)
      break
    case 'PROVIDER_LOCK_IN_AS_STORE_IDENTITY':
      finding = foundIf(evidence.provider_id_is_store_id === true)
      break
    case 'FRONTEND_EVENT_AS_BUSINESS_FACT':
      finding = foundIf(evidence.frontend_event_claimed_as_canonical_business_change === true && evidence.canonical_state_change !== true)
      break
    case 'PERSISTENCE_ADAPTER_AS_STORE':
      finding = foundIf(evidence.repository_or_adapter_named_store === true && evidence.claimed_cap17_store === true)
      break
    default:
      break
  }

  if (finding === 'FOUND' && asArray(evidence.evidence_paths).length === 0 && !text(evidence.note)) {
    errors.push(`${tag}: FOUND requires evidence_paths or note`)
  }
  return { ok: errors.length === 0, finding, errors }
}

export function evaluateStoreAntiPatternSet(entries) {
  const errors = []
  const findings = {}
  const found = []
  const tags = [...STORE_ANTI_PATTERNS, ...SHARED_WEB2APP_ANTI_PATTERNS]
  for (const tag of tags) {
    const entry = asArray(entries).find((row) => text(row.tag) === tag)
    if (!entry) {
      findings[tag] = 'NOT_EVALUATED'
      continue
    }
    const result = evaluateStoreAntiPattern(tag, entry.evidence || entry)
    findings[tag] = result.finding
    if (!result.ok) errors.push(...result.errors)
    if (result.finding === 'FOUND') found.push(tag)
  }
  return { ok: errors.length === 0, errors, findings, found }
}
