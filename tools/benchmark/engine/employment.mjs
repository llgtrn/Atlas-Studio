// Agent Employment Contract evaluator. Detects semantic contradictions.
// Does not mutate canonical capability rows or implement CAP16 runtime.

import { asArray, hasText, isObject, text } from './util.mjs'
import { evaluateEmploymentAntiPattern, evaluateEmploymentAntiPatternSet } from './employment-anti-patterns.mjs'
import {
  ASSIGNMENT_WITNESSES,
  EMPLOYMENT_ANTI_PATTERNS,
  EMPLOYMENT_LEVELS,
  EMPLOYMENT_SCHEMA,
  EMPLOYMENT_STANDARD_VERSION,
  FACET_NAMES,
  FACET_STATES,
  FIXTURE_KINDS,
  HARD_FAILURE_TAGS,
  SHARED_WEB2APP_ANTI_PATTERNS,
  WARNING_TAGS,
  WORKER_CLASSES,
  employmentRank,
  minEmployment,
} from './employment-vocab.mjs'

function flag(obj, key) {
  return isObject(obj) && obj[key] === true
}

function nest(record, key) {
  return isObject(record[key]) ? record[key] : {}
}

function claimedLevel(record) {
  const claimed = text(record.claimed_employment_maturity)
  return EMPLOYMENT_LEVELS.includes(claimed) ? claimed : 'E0'
}

export function validateEmploymentRecord(record) {
  const errors = []
  if (!isObject(record)) return { ok: false, errors: ['record must be an object'] }
  if (hasText(record.schema) && text(record.schema) !== EMPLOYMENT_SCHEMA) {
    errors.push(`schema must be ${EMPLOYMENT_SCHEMA}`)
  }
  if (!WORKER_CLASSES.includes(text(record.worker_class))) {
    errors.push('worker_class must be utility | runtime_worker | digital_employee | position')
  }
  if (hasText(record.claimed_employment_maturity) && !EMPLOYMENT_LEVELS.includes(text(record.claimed_employment_maturity))) {
    errors.push(`claimed_employment_maturity is invalid: ${record.claimed_employment_maturity}`)
  }
  if (hasText(record.fixture_kind) && !FIXTURE_KINDS.includes(text(record.fixture_kind))) {
    errors.push(`fixture_kind is invalid: ${record.fixture_kind}`)
  }
  const facets = isObject(record.facets) ? record.facets : {}
  for (const [name, state] of Object.entries(facets)) {
    if (hasText(name) && !FACET_NAMES.includes(name) && !FACET_NAMES.includes(text(name).toUpperCase())) {
      errors.push(`unknown facet ${name}`)
    }
    if (hasText(state) && !FACET_STATES.includes(text(state))) {
      errors.push(`facet ${name} has invalid state ${state}`)
    }
  }
  return { ok: errors.length === 0, errors }
}

function deriveAntiPatternEvidence(record) {
  const identity = nest(record, 'identity')
  const org = nest(record, 'organization')
  const contract = nest(record, 'contract')
  const assignment = nest(record, 'assignment')
  const authority = nest(record, 'authority')
  const lifecycle = nest(record, 'lifecycle')
  const evidence = nest(record, 'evidence')
  const derived = []
  const push = (tag, ev) => derived.push({ tag, evidence: { note: tag, ...ev } })

  if (flag(org, 'role_only') && !flag(identity, 'stable_worker_id') && employmentRank(claimedLevel(record)) >= 2) {
    push('ROLE_AS_WORKER', { role_template_exists: true, stable_worker_identity: false, claimed_as_digital_employee: true })
  }
  if (flag(record, 'runtime_worker_only') && !flag(org, 'company_bound') && text(record.worker_class) === 'digital_employee') {
    push('WORKER_AS_EMPLOYEE', { runtime_worker_exists: true, company_employment_binding: false, claimed_as_digital_employee: true })
  }
  if (flag(record, 'provider_as_employee') || flag(org, 'provider_registered_as_hire')) {
    push('PROVIDER_AS_EMPLOYEE', { provider_registered: true, hire_of_digital_employee: false, claimed_as_hire: true })
  }
  if (flag(contract, 'agent_config_as_contract')) {
    push('AGENT_CONFIG_AS_CONTRACT', { agent_config_used_as_authority: true })
  }
  if (flag(contract, 'prompt_as_contract')) {
    push('PROMPT_AS_EMPLOYMENT_CONTRACT', { system_prompt_used_as_contract: true, prompt_grants_authority: true })
  }
  if (flag(authority, 'prose_as_authority')) {
    push('PROSE_AS_AUTHORITY', { job_description_used_as_grant: true, responsibility_prose_authorizes: true })
  }
  if (flag(authority, 'title_as_authority')) {
    push('TITLE_AS_AUTHORITY', { title_string_authorizes_action: true })
  }
  if (flag(authority, 'skill_as_authority')) {
    push('SKILL_AS_AUTHORITY', { skill_score_used_as_grant: true })
  }
  if (flag(authority, 'skill_overrides_authority')) {
    push('SKILL_OVERRIDES_AUTHORITY', { skill_routing_precedes_or_bypasses_eligibility: true })
  }
  if (flag(authority, 'tool_availability_as_authority')) {
    push('TOOL_AVAILABILITY_AS_AUTHORITY', { tool_or_app_available: true, contract_grant: false, treated_as_allowed: true })
  }
  if (flag(authority, 'credential_existence_as_authority')) {
    push('CREDENTIAL_EXISTENCE_AS_AUTHORITY', { company_credential_exists: true, worker_lease_eligible: false, treated_as_allowed: true })
  }
  if (flag(contract, 'contains_secret_plaintext')) {
    push('SECRET_IN_EMPLOYMENT_CONTRACT', { plaintext_secret_in_contract: true })
  }
  if (flag(contract, 'mutated_in_place')) {
    push('MUTABLE_HISTORY_CONTRACT', { mutated_in_place: true, prior_version_overwritten: true })
    push('CONTRACT_HISTORY_MUTATED_IN_PLACE', { mutated_in_place: true, prior_version_overwritten: true })
  }
  if (flag(assignment, 'unpinned') || (flag(assignment, 'work_order_present') && !flag(assignment, 'contract_version_pinned'))) {
    if (flag(assignment, 'work_order_present')) {
      push('UNPINNED_ASSIGNMENT_AUTHORITY', { assignment_stores_worker_only: true, later_evaluates_current_contract: true })
    }
  }
  if (flag(contract, 'status_label_only') || (flag(contract, 'revoked_but_admits_new_work') && flag(contract, 'status_revoked'))) {
    push('CONTRACT_STATUS_WITHOUT_ENFORCEMENT', { status_label_set: true, admission_blocked: false })
  }
  if (flag(lifecycle, 'label_only') || flag(lifecycle, 'suspended_but_admits_new_work')) {
    push('LIFECYCLE_LABEL_WITHOUT_ENFORCEMENT', { lifecycle_label_set: true, assignment_or_execution_affected: false })
  }
  if (flag(contract, 'self_amendment_possible') || flag(contract, 'created_by_worker')) {
    push('WORKER_SELF_AMENDMENT', { worker_modifies_own_grants_or_limits: true, external_governance: false })
  }
  if (flag(authority, 'learning_expands_authority')) {
    push('LEARNING_EXPANDS_AUTHORITY', { skill_or_outcome_learning: true, authority_expanded_as_side_effect: true })
  }
  if (flag(authority, 'kpi_expands_authority')) {
    push('KPI_EXPANDS_AUTHORITY', { high_kpi: true, authority_auto_increased: true })
  }
  if (flag(authority, 'delegation_escalates')) {
    push('DELEGATION_ESCALATES_AUTHORITY', { delegated_authority_exceeds_delegator: true })
  }
  if (flag(authority, 'cost_budget_as_transaction_authority')) {
    push('COST_BUDGET_AS_TRANSACTION_AUTHORITY', { operational_cost_budget_treated_as_business_spend: true })
  }
  if (flag(assignment, 'broadens_contract')) {
    push('WORKORDER_BROADENS_CONTRACT', { work_order_grants_beyond_contract: true })
  }
  if (flag(evidence, 'worker_completion_as_evidence')) {
    push('WORKER_COMPLETION_AS_EVIDENCE', { self_assertion_accepted_as_completion: true, required_evidence_class: false })
  }
  if (flag(evidence, 'worker_erases_evidence')) {
    push('WORKER_ERASES_EVIDENCE', { worker_may_delete_unfavorable_evidence_or_incidents: true })
  }
  if (flag(identity, 'runtime_change_resets_history')) {
    push('RUNTIME_REPLACEMENT_RESETS_EMPLOYMENT_HISTORY', { model_or_runtime_change: true, identity_or_history_reset: true, explicit_new_worker: false })
  }
  if (text(record.fixture_kind) === 'BENCHMARK_FIXTURE' && employmentRank(claimedLevel(record)) >= 6) {
    push('TEST_ONLY_EMPLOYMENT_RUNTIME', { tests_are_only_caller: true, claimed_production_employment: true })
  }
  if (flag(record, 'doc_only') && employmentRank(claimedLevel(record)) >= 2) {
    push('DOC_SNIPPET_AS_RUNTIME', { rust_struct_or_spec_in_docs: true, runtime_implementation: false, claimed_as_runtime: true })
  }
  if (flag(identity, 'process_id_as_identity')) {
    push('PROCESS_ID_AS_WORKER_IDENTITY', { execution_instance_id_used_as_employee_id: true })
  }
  if (flag(contract, 'hash_as_version_history')) {
    push('HASH_AS_VERSION_HISTORY', { hash_present: true, lineage_and_effective_period: false, treated_as_full_history: true })
  }
  if (flag(contract, 'expired_but_admits_new_work')) {
    push('EXPIRED_CONTRACT_EXECUTES_NEW_WORK', { contract_expired: true, new_production_work_admitted: true })
  }
  if (flag(contract, 'revoked_but_admits_new_work')) {
    push('REVOKED_CONTRACT_EXECUTES_NEW_WORK', { contract_revoked: true, new_production_work_admitted: true })
  }
  if (flag(authority, 'cross_tenant_assignment')) {
    push('CROSS_TENANT_WORKER_ASSIGNMENT', { worker_company_or_tenant: 'A', work_order_company_or_tenant: 'B', assignment_accepted: true })
  }
  if (flag(authority, 'money_authority_bypass')) {
    push('MONEY_AUTHORITY_BYPASS', { employment_layer_authorizes_money_commit: true, skips_canonical_money_gate: true })
  }
  if (flag(authority, 'no_ai_enforcement') === false && employmentRank(claimedLevel(record)) >= 4 && authority.enforced === true) {
    push('AI_DECIDES_AUTHORITY', { authority_decision_uses_model_inference: true })
  }
  if (flag(authority, 'event_as_grant')) {
    push('EVENT_AS_EMPLOYMENT_GRANT', { external_event_creates_worker_authority: true })
  }
  if (text(record.runtime_status) === 'TEST_ONLY' && employmentRank(claimedLevel(record)) >= 6) {
    push('TEST_AS_RUNTIME', { tests_are_only_caller: true, claimed_shipped_runtime: true })
  }
  if (text(record.canonical_status) && flag(record, 'tracking_only') && employmentRank(claimedLevel(record)) >= 2) {
    push('TRACKING_AS_PRODUCT', { only_canonical_or_arch_row: true, implementation_claimed: true })
  }
  return derived
}

export function assignEmploymentMaturity(record) {
  const cls = text(record.worker_class)
  const identity = nest(record, 'identity')
  const org = nest(record, 'organization')
  const contract = nest(record, 'contract')
  const assignment = nest(record, 'assignment')
  const authority = nest(record, 'authority')
  const lifecycle = nest(record, 'lifecycle')
  const evidence = nest(record, 'evidence')

  if (cls === 'utility') return 'E0'

  let level = 'E0'
  const orgMeta = flag(org, 'role_only') || flag(org, 'company_bound') || flag(org, 'department_bound') || flag(org, 'position_bound')
  if (orgMeta) level = 'E1'

  const stableId = flag(identity, 'stable_worker_id') && !flag(identity, 'process_id_as_identity')
  const runtimeContract =
    flag(contract, 'durable') &&
    flag(contract, 'versioned') &&
    !flag(record, 'doc_only') &&
    !flag(contract, 'prompt_as_contract') &&
    !flag(contract, 'agent_config_as_contract') &&
    !flag(contract, 'contains_secret_plaintext')
  if (stableId && runtimeContract && flag(org, 'company_bound')) level = 'E2'

  const pinned = flag(assignment, 'work_order_present') && flag(assignment, 'contract_version_pinned') && !flag(assignment, 'unpinned')
  const productionPath = flag(assignment, 'production_path')
  const admissionOk = !flag(contract, 'expired_but_admits_new_work') && !flag(contract, 'revoked_but_admits_new_work')
  if (employmentRank(level) >= 2 && pinned && productionPath && admissionOk && !flag(assignment, 'broadens_contract')) {
    level = 'E3'
  }

  const authorityOk =
    flag(authority, 'enforced') &&
    flag(authority, 'witness_present') &&
    flag(authority, 'no_ai_enforcement') &&
    !flag(authority, 'skill_overrides_authority') &&
    !flag(authority, 'title_as_authority') &&
    !flag(authority, 'prose_as_authority') &&
    !flag(authority, 'tool_availability_as_authority') &&
    !flag(authority, 'skill_as_authority')
  if (employmentRank(level) >= 3 && authorityOk) level = 'E4'

  const lifeOk =
    flag(lifecycle, 'enforced') &&
    !flag(lifecycle, 'label_only') &&
    flag(lifecycle, 'suspended_blocks_new_work') &&
    flag(lifecycle, 'terminated_blocks_new_work') &&
    flag(contract, 'immutable_history') &&
    !flag(contract, 'self_amendment_possible') &&
    !flag(contract, 'mutated_in_place')
  if (employmentRank(level) >= 4 && lifeOk) level = 'E5'

  const prod =
    flag(evidence, 'production_evidenced') &&
    text(record.fixture_kind) !== 'BENCHMARK_FIXTURE' &&
    text(record.runtime_status) !== 'TEST_ONLY' &&
    text(record.runtime_status) !== 'DOC_ONLY'
  if (employmentRank(level) >= 5 && prod) level = 'E6'

  if (flag(record, 'doc_only')) level = minEmployment(level, 'E0')
  if (flag(record, 'runtime_worker_only') && !flag(org, 'company_bound')) level = minEmployment(level, 'E0')
  if ((flag(org, 'role_only') || cls === 'runtime_worker' || cls === 'position') && !runtimeContract) {
    level = minEmployment(level, orgMeta ? 'E1' : 'E0')
  }
  if (!pinned || !productionPath) {
    if (stableId && runtimeContract && flag(org, 'company_bound')) level = minEmployment(level, 'E2')
  }
  if (flag(assignment, 'unpinned') || (flag(assignment, 'work_order_present') && !flag(assignment, 'contract_version_pinned'))) {
    level = minEmployment(level, 'E2')
  }
  if (!authorityOk) {
    if (pinned && productionPath) level = minEmployment(level, 'E3')
  }
  if (!lifeOk) level = minEmployment(level, 'E4')
  if (!prod) level = minEmployment(level, 'E5')
  if (cls === 'runtime_worker' && !runtimeContract) level = minEmployment(level, orgMeta ? 'E1' : 'E0')
  return level
}

function contractRequiredGate(record) {
  const contract = nest(record, 'contract')
  const assignment = nest(record, 'assignment')
  if (text(record.worker_class) !== 'digital_employee') return null
  const effective =
    flag(contract, 'durable') &&
    flag(contract, 'versioned') &&
    !flag(record, 'doc_only') &&
    !flag(contract, 'prompt_as_contract') &&
    !flag(contract, 'agent_config_as_contract') &&
    !flag(contract, 'expired_but_admits_new_work') &&
    !flag(contract, 'revoked_but_admits_new_work')
  const newWork = flag(assignment, 'production_path') || flag(assignment, 'executes_new_work')
  if (newWork && !effective) return 'NO_EFFECTIVE_CONTRACT_NO_NEW_PRODUCTION_WORK'
  return null
}

function witnessSet(record) {
  const named = new Set(asArray(record.actual_witnesses).map(text).filter(Boolean))
  const matrix = isObject(record.witness_matrix) ? record.witness_matrix : {}
  for (const name of ASSIGNMENT_WITNESSES) {
    const cell = matrix[name]
    if (cell === true || (isObject(cell) && (cell.present === true || hasText(cell.evidence)))) named.add(name)
  }
  return named
}

function semanticReview(record, assigned) {
  const flags = asArray(record.semantic_review_required).map(text).filter(Boolean)
  const authority = nest(record, 'authority')
  if (flag(authority, 'enforced') && !flag(authority, 'witness_present')) flags.push('E4 claim without authority witness')
  if (employmentRank(assigned) >= 3 && !flag(nest(record, 'assignment'), 'contract_version_pinned')) {
    flags.push('E3 claim without contract version pin')
  }
  const facets = isObject(record.facets) ? record.facets : {}
  for (const [name, state] of Object.entries(facets)) {
    if (text(state) === 'ENFORCED' && !flag(authority, 'enforced') && ['GRANTS', 'PROHIBITIONS', 'TERM', 'REVOCATION'].includes(name)) {
      flags.push(`SEMANTIC_REVIEW_REQUIRED: facet ${name} claimed ENFORCED without authority.enforced`)
    }
  }
  if (flag(record, 'classification_uncertain')) {
    flags.push('SEMANTIC_REVIEW_REQUIRED: employee classification')
  }
  return [...new Set(flags)]
}

export function evaluateEmploymentRecord(record) {
  const validation = validateEmploymentRecord(record)
  const claimed = claimedLevel(record)
  const assigned = assignEmploymentMaturity(record)
  const derived = deriveAntiPatternEvidence(record)
  const declared = asArray(record.anti_patterns).map((row) => (typeof row === 'string' ? { tag: row, evidence: record.anti_pattern_evidence?.[row] || { note: 'declared' } } : row))
  const merged = [...declared, ...derived]
  const set = evaluateEmploymentAntiPatternSet(merged)
  const extra = []
  for (const entry of merged) {
    const tag = text(entry.tag)
    if (![...EMPLOYMENT_ANTI_PATTERNS, ...SHARED_WEB2APP_ANTI_PATTERNS].includes(tag)) continue
    if (set.findings[tag] === 'NOT_EVALUATED' || set.findings[tag] == null) {
      const one = evaluateEmploymentAntiPattern(tag, entry.evidence || entry)
      if (one.finding === 'FOUND' && !set.found.includes(tag)) {
        set.found.push(tag)
        set.findings[tag] = 'FOUND'
      }
      if (!one.ok) extra.push(...one.errors)
    }
  }

  const hardFailures = set.found.filter((tag) => HARD_FAILURE_TAGS.includes(tag))
  const warnings = set.found.filter((tag) => WARNING_TAGS.includes(tag))
  const hardGates = []
  const required = contractRequiredGate(record)
  if (required) hardGates.push(required)
  if (employmentRank(claimed) > employmentRank(assigned)) {
    hardGates.push('CLAIMED_MATURITY_EXCEEDS_EVIDENCE')
  }
  if (flag(nest(record, 'web2app'), 'implies_employment')) {
    hardGates.push('WEB2APP_COLLAPSED_INTO_EMPLOYMENT')
  }
  if (text(record.worker_class) === 'utility' && employmentRank(claimed) >= 2) {
    hardGates.push('UTILITY_OVERWRAPPED_AS_DIGITAL_EMPLOYEE')
  }

  const review = semanticReview(record, assigned)
  const contradictions = []
  if (text(record.canonical_status) === 'verified' && flag(record, 'doc_only') && employmentRank(claimed) >= 2) {
    contradictions.push('canonical verified plus doc-only employment contract claimed as runtime')
  }
  if (flag(nest(record, 'organization'), 'role_only') && text(record.worker_class) === 'digital_employee' && !flag(nest(record, 'identity'), 'stable_worker_id')) {
    contradictions.push('CompanyAgentRole/position template claimed as DigitalWorkerIdentity')
  }

  let verdict = 'PASS'
  if (hardFailures.length || hardGates.length) verdict = 'FAIL'
  else if (review.length) verdict = 'PARTIAL'

  const productionEvidence = flag(nest(record, 'evidence'), 'production_evidenced') && text(record.fixture_kind) !== 'BENCHMARK_FIXTURE'
  return {
    ok: validation.ok && set.ok && extra.length === 0,
    errors: [...validation.errors, ...set.errors, ...extra],
    schema: EMPLOYMENT_SCHEMA,
    standard_version: EMPLOYMENT_STANDARD_VERSION,
    case_id: text(record.case_id) || text(record.capability_key),
    worker_class: text(record.worker_class),
    fixture_kind: text(record.fixture_kind),
    claimed_employment_maturity: claimed,
    employment_maturity: assigned,
    employment_semantic_target: text(record.employment_semantic_target) || (flag(record, 'doc_only') ? 'STRONG' : 'RUNTIME'),
    canonical_status: text(record.canonical_status) || null,
    runtime_status: text(record.runtime_status) || null,
    product_status: text(record.product_status) || null,
    production_evidence: productionEvidence,
    verdict,
    hard_gate_failures: hardGates,
    hard_failures: hardFailures,
    anti_patterns_found: set.found,
    anti_pattern_findings: set.findings,
    warnings,
    contradictions,
    facets: isObject(record.facets) ? record.facets : {},
    actual_witnesses: [...witnessSet(record)],
    semantic_review_required: review,
    web2app_application_maturity: nest(record, 'web2app').application_maturity || null,
    web2app_independent: nest(record, 'web2app').independent !== false,
    effective_authority_trace: record.effective_authority_trace || null,
    notes: record.notes || null,
    this_standard_is_not_cap16: true,
  }
}
