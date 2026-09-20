// Employment anti-pattern evaluation from evidence objects, not keyword grep.
// Shared WEB2APP tags are delegated; this file owns employment-specific tags.

import { evaluateWeb2AppAntiPattern } from './web2app-anti-patterns.mjs'
import { asArray, isObject, text } from './util.mjs'
import {
  EMPLOYMENT_ANTI_PATTERNS,
  SHARED_WEB2APP_ANTI_PATTERNS,
} from './employment-vocab.mjs'

function foundIf(condition) {
  return condition ? 'FOUND' : 'NOT_FOUND'
}

export function evaluateEmploymentAntiPattern(tag, evidence) {
  if (SHARED_WEB2APP_ANTI_PATTERNS.includes(tag)) {
    return evaluateWeb2AppAntiPattern(tag, evidence)
  }
  const errors = []
  if (!EMPLOYMENT_ANTI_PATTERNS.includes(tag)) {
    return { ok: false, finding: null, errors: [`unknown employment anti-pattern ${tag}`] }
  }
  if (typeof evidence === 'string') {
    return { ok: true, finding: 'NOT_EVALUATED', errors: [] }
  }
  if (!isObject(evidence)) {
    return { ok: false, finding: null, errors: [`${tag}: evidence object required (keyword grep is not enough)`] }
  }
  if (evidence.keyword_only === true || evidence.detected_by === 'grep') {
    return { ok: false, finding: null, errors: [`${tag}: do not mark merely by keyword grep`] }
  }

  let finding = 'NOT_FOUND'
  switch (tag) {
    case 'ROLE_AS_WORKER':
      finding = foundIf(evidence.role_template_exists === true && evidence.stable_worker_identity !== true && evidence.claimed_as_digital_employee === true)
      break
    case 'WORKER_AS_EMPLOYEE':
      finding = foundIf(evidence.runtime_worker_exists === true && evidence.company_employment_binding !== true && evidence.claimed_as_digital_employee === true)
      break
    case 'PROVIDER_AS_EMPLOYEE':
      finding = foundIf((evidence.provider_registered === true || evidence.model_registered === true) && evidence.hire_of_digital_employee !== true && evidence.claimed_as_hire === true)
      break
    case 'AGENT_CONFIG_AS_CONTRACT':
      finding = foundIf(evidence.agent_config_used_as_authority === true)
      break
    case 'PROMPT_AS_EMPLOYMENT_CONTRACT':
      finding = foundIf(evidence.system_prompt_used_as_contract === true || evidence.prompt_grants_authority === true)
      break
    case 'PROSE_AS_AUTHORITY':
      finding = foundIf(evidence.job_description_used_as_grant === true || evidence.responsibility_prose_authorizes === true)
      break
    case 'TITLE_AS_AUTHORITY':
      finding = foundIf(evidence.title_string_authorizes_action === true)
      break
    case 'SKILL_AS_AUTHORITY':
      finding = foundIf(evidence.skill_score_used_as_grant === true)
      break
    case 'SKILL_OVERRIDES_AUTHORITY':
      finding = foundIf(evidence.skill_routing_precedes_or_bypasses_eligibility === true || evidence.smartest_worker_gets_needed_grants === true)
      break
    case 'TOOL_AVAILABILITY_AS_AUTHORITY':
      finding = foundIf(evidence.tool_or_app_available === true && evidence.contract_grant !== true && evidence.treated_as_allowed === true)
      break
    case 'CREDENTIAL_EXISTENCE_AS_AUTHORITY':
      finding = foundIf(evidence.company_credential_exists === true && evidence.worker_lease_eligible !== true && evidence.treated_as_allowed === true)
      break
    case 'SECRET_IN_EMPLOYMENT_CONTRACT':
      finding = foundIf(evidence.plaintext_secret_in_contract === true)
      break
    case 'MUTABLE_HISTORY_CONTRACT':
    case 'CONTRACT_HISTORY_MUTATED_IN_PLACE':
      finding = foundIf(evidence.prior_version_overwritten === true || evidence.mutated_in_place === true)
      break
    case 'UNPINNED_ASSIGNMENT_AUTHORITY':
      finding = foundIf(evidence.assignment_stores_worker_only === true && evidence.later_evaluates_current_contract === true)
      break
    case 'CONTRACT_STATUS_WITHOUT_ENFORCEMENT':
      finding = foundIf(evidence.status_label_set === true && evidence.admission_blocked !== true)
      break
    case 'LIFECYCLE_LABEL_WITHOUT_ENFORCEMENT':
      finding = foundIf(evidence.lifecycle_label_set === true && evidence.assignment_or_execution_affected !== true)
      break
    case 'WORKER_SELF_AMENDMENT':
      finding = foundIf(evidence.worker_modifies_own_grants_or_limits === true && evidence.external_governance !== true)
      break
    case 'LEARNING_EXPANDS_AUTHORITY':
      finding = foundIf(evidence.skill_or_outcome_learning === true && evidence.authority_expanded_as_side_effect === true)
      break
    case 'KPI_EXPANDS_AUTHORITY':
      finding = foundIf(evidence.high_kpi === true && evidence.authority_auto_increased === true)
      break
    case 'DELEGATION_ESCALATES_AUTHORITY':
      finding = foundIf(evidence.delegated_authority_exceeds_delegator === true)
      break
    case 'COST_BUDGET_AS_TRANSACTION_AUTHORITY':
      finding = foundIf(evidence.operational_cost_budget_treated_as_business_spend === true)
      break
    case 'WORKORDER_BROADENS_CONTRACT':
      finding = foundIf(evidence.work_order_grants_beyond_contract === true)
      break
    case 'WORKER_COMPLETION_AS_EVIDENCE':
      finding = foundIf(evidence.self_assertion_accepted_as_completion === true && evidence.required_evidence_class !== true)
      break
    case 'WORKER_ERASES_EVIDENCE':
      finding = foundIf(evidence.worker_may_delete_unfavorable_evidence_or_incidents === true)
      break
    case 'RUNTIME_REPLACEMENT_RESETS_EMPLOYMENT_HISTORY':
      finding = foundIf(evidence.model_or_runtime_change === true && evidence.identity_or_history_reset === true && evidence.explicit_new_worker !== true)
      break
    case 'TEST_ONLY_EMPLOYMENT_RUNTIME':
      finding = foundIf(evidence.tests_are_only_caller === true && evidence.claimed_production_employment === true)
      break
    case 'DOC_SNIPPET_AS_RUNTIME':
      finding = foundIf(evidence.rust_struct_or_spec_in_docs === true && evidence.runtime_implementation !== true && evidence.claimed_as_runtime === true)
      break
    case 'PROCESS_ID_AS_WORKER_IDENTITY':
      finding = foundIf(evidence.execution_instance_id_used_as_employee_id === true)
      break
    case 'HASH_AS_VERSION_HISTORY':
      finding = foundIf(evidence.hash_present === true && evidence.lineage_and_effective_period !== true && evidence.treated_as_full_history === true)
      break
    case 'EXPIRED_CONTRACT_EXECUTES_NEW_WORK':
      finding = foundIf(evidence.contract_expired === true && evidence.new_production_work_admitted === true)
      break
    case 'REVOKED_CONTRACT_EXECUTES_NEW_WORK':
      finding = foundIf((evidence.contract_revoked === true || evidence.worker_revoked === true) && evidence.new_production_work_admitted === true)
      break
    case 'CROSS_TENANT_WORKER_ASSIGNMENT':
      finding = foundIf(evidence.worker_company_or_tenant !== evidence.work_order_company_or_tenant && evidence.assignment_accepted === true)
      break
    case 'MONEY_AUTHORITY_BYPASS':
      finding = foundIf(evidence.employment_layer_authorizes_money_commit === true || evidence.skips_canonical_money_gate === true)
      break
    case 'AI_DECIDES_AUTHORITY':
      finding = foundIf(evidence.authority_decision_uses_model_inference === true)
      break
    case 'EVENT_AS_EMPLOYMENT_GRANT':
      finding = foundIf(evidence.external_event_creates_worker_authority === true)
      break
    default:
      break
  }

  if (finding === 'FOUND' && asArray(evidence.evidence_paths).length === 0 && !text(evidence.note)) {
    errors.push(`${tag}: FOUND requires evidence_paths or note`)
  }
  return { ok: errors.length === 0, finding, errors }
}

export function evaluateEmploymentAntiPatternSet(entries) {
  const errors = []
  const findings = {}
  const found = []
  const tags = [...EMPLOYMENT_ANTI_PATTERNS, ...SHARED_WEB2APP_ANTI_PATTERNS]
  for (const tag of tags) {
    const entry = asArray(entries).find((row) => text(row.tag) === tag)
    if (!entry) {
      findings[tag] = 'NOT_EVALUATED'
      continue
    }
    const result = evaluateEmploymentAntiPattern(tag, entry.evidence || entry)
    findings[tag] = result.finding
    if (!result.ok) errors.push(...result.errors)
    if (result.finding === 'FOUND') found.push(tag)
  }
  return { ok: errors.length === 0, errors, findings, found }
}
