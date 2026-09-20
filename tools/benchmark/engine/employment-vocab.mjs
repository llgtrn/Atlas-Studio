// Agent Employment Contract Standard vocab. Extends the benchmark truth engine.
// Does not replace FIC, WEB2APP, or the five-plane freeze. Does not implement CAP16.

export const EMPLOYMENT_STANDARD_VERSION = '1.0.0'
export const EMPLOYMENT_SCHEMA = 'chronica.employment.evaluation.v1'
export const EMPLOYMENT_STANDARD_ID = '206'

export const AGENT_WORKS_BY_CONTRACT = true
export const COMPANY_AGENT_ROLE_IS_NOT_EMPLOYMENT_CONTRACT = true
export const RUNTIME_WORKER_IS_NOT_DIGITAL_EMPLOYEE = true
export const SKILL_IS_NOT_AUTHORITY = true
export const TOOL_IS_NOT_AUTHORITY = true
export const CREDENTIAL_EXISTENCE_IS_NOT_ELIGIBILITY = true
export const TITLE_IS_NOT_AUTHORITY = true
export const PROSE_IS_NOT_GRANT = true
export const WORKER_MAY_NOT_SELF_AMEND = true
export const EXPIRED_OR_REVOKED_MAY_NOT_ADMIT_NEW_WORK = true
export const WORKORDER_MAY_NOT_BROADEN_CONTRACT = true
export const CONTRACT_VERSIONS_MUST_BE_HISTORICALLY_REPRODUCIBLE = true
export const DIGITAL_EMPLOYEES_REQUIRE_EFFECTIVE_CONTRACTS = true
export const UTILITIES_DO_NOT_REQUIRE_EMPLOYMENT_CONTRACTS = true
export const THIS_STANDARD_IS_NOT_CAP16_IMPLEMENTATION = true
export const AUTHORITY_EVALUATION_IS_NO_AI = true
export const EFFECTIVE_AUTHORITY_CAN_ONLY_NARROW = true
export const WEB2APP_W_LEVEL_IS_NOT_EMPLOYMENT_E_LEVEL = true
export const LEGAL_PERSONHOOD_NOT_ASSERTED = true

export const EMPLOYMENT_LEVELS = Object.freeze(['E0', 'E1', 'E2', 'E3', 'E4', 'E5', 'E6'])
export const EMPLOYMENT_RANK = Object.freeze({ E0: 0, E1: 1, E2: 2, E3: 3, E4: 4, E5: 5, E6: 6 })

export const WORKER_CLASSES = Object.freeze(['utility', 'runtime_worker', 'digital_employee', 'position'])
export const FIXTURE_KINDS = Object.freeze(['BENCHMARK_FIXTURE', 'CURRENT_RUNTIME_CALIBRATION'])
export const VERDICTS = Object.freeze(['PASS', 'PARTIAL', 'FAIL', 'BLOCKED'])

export const FACET_NAMES = Object.freeze([
  'IDENTITY',
  'ORGANIZATION',
  'POSITION',
  'SUPERVISOR',
  'ACCOUNTABLE_HUMAN',
  'DUTIES',
  'OUTPUTS',
  'GRANTS',
  'PROHIBITIONS',
  'DATA',
  'TOOLS',
  'CREDENTIALS',
  'COST',
  'TRANSACTION_SEMANTICS',
  'APPROVAL',
  'EVIDENCE',
  'KPI',
  'SCHEDULE',
  'CONCURRENCY',
  'DELEGATION',
  'INCIDENT',
  'LEARNING',
  'TERM',
  'REVOCATION',
  'RUNTIME_BINDING',
])

export const FACET_STATES = Object.freeze([
  'ABSENT',
  'METADATA_ONLY',
  'DURABLE',
  'ENFORCED',
  'PRODUCTION_EVIDENCED',
  'BLOCKED',
])

export const FACET_RANK = Object.freeze({
  ABSENT: 0,
  METADATA_ONLY: 1,
  DURABLE: 2,
  ENFORCED: 3,
  PRODUCTION_EVIDENCED: 4,
  BLOCKED: -1,
})

export const SHARED_WEB2APP_ANTI_PATTERNS = Object.freeze([
  'LABEL_AS_ENFORCEMENT',
  'GOALPOST_MUTATION',
  'TEST_AS_RUNTIME',
  'TRACKING_AS_PRODUCT',
  'CALLER_FARMING',
])

export const EMPLOYMENT_ANTI_PATTERNS = Object.freeze([
  'ROLE_AS_WORKER',
  'WORKER_AS_EMPLOYEE',
  'PROVIDER_AS_EMPLOYEE',
  'AGENT_CONFIG_AS_CONTRACT',
  'PROMPT_AS_EMPLOYMENT_CONTRACT',
  'PROSE_AS_AUTHORITY',
  'TITLE_AS_AUTHORITY',
  'SKILL_AS_AUTHORITY',
  'SKILL_OVERRIDES_AUTHORITY',
  'TOOL_AVAILABILITY_AS_AUTHORITY',
  'CREDENTIAL_EXISTENCE_AS_AUTHORITY',
  'SECRET_IN_EMPLOYMENT_CONTRACT',
  'MUTABLE_HISTORY_CONTRACT',
  'UNPINNED_ASSIGNMENT_AUTHORITY',
  'CONTRACT_STATUS_WITHOUT_ENFORCEMENT',
  'LIFECYCLE_LABEL_WITHOUT_ENFORCEMENT',
  'WORKER_SELF_AMENDMENT',
  'LEARNING_EXPANDS_AUTHORITY',
  'KPI_EXPANDS_AUTHORITY',
  'DELEGATION_ESCALATES_AUTHORITY',
  'COST_BUDGET_AS_TRANSACTION_AUTHORITY',
  'WORKORDER_BROADENS_CONTRACT',
  'WORKER_COMPLETION_AS_EVIDENCE',
  'WORKER_ERASES_EVIDENCE',
  'RUNTIME_REPLACEMENT_RESETS_EMPLOYMENT_HISTORY',
  'TEST_ONLY_EMPLOYMENT_RUNTIME',
  'DOC_SNIPPET_AS_RUNTIME',
  'PROCESS_ID_AS_WORKER_IDENTITY',
  'HASH_AS_VERSION_HISTORY',
  'EXPIRED_CONTRACT_EXECUTES_NEW_WORK',
  'REVOKED_CONTRACT_EXECUTES_NEW_WORK',
  'CROSS_TENANT_WORKER_ASSIGNMENT',
  'MONEY_AUTHORITY_BYPASS',
  'CONTRACT_HISTORY_MUTATED_IN_PLACE',
  'AI_DECIDES_AUTHORITY',
  'EVENT_AS_EMPLOYMENT_GRANT',
])

export const HARD_FAILURE_TAGS = Object.freeze([
  'WORKER_SELF_AMENDMENT',
  'SKILL_OVERRIDES_AUTHORITY',
  'TOOL_AVAILABILITY_AS_AUTHORITY',
  'TITLE_AS_AUTHORITY',
  'PROSE_AS_AUTHORITY',
  'PROMPT_AS_EMPLOYMENT_CONTRACT',
  'SECRET_IN_EMPLOYMENT_CONTRACT',
  'EXPIRED_CONTRACT_EXECUTES_NEW_WORK',
  'REVOKED_CONTRACT_EXECUTES_NEW_WORK',
  'CROSS_TENANT_WORKER_ASSIGNMENT',
  'DELEGATION_ESCALATES_AUTHORITY',
  'MONEY_AUTHORITY_BYPASS',
  'CONTRACT_HISTORY_MUTATED_IN_PLACE',
])

export const WARNING_TAGS = Object.freeze(['HASH_AS_VERSION_HISTORY'])

export const ASSIGNMENT_WITNESSES = Object.freeze([
  'stable_worker_identity',
  'company_scope',
  'active_contract',
  'immutable_contract_version',
  'work_order',
  'contract_version_pinned',
  'capability_required',
  'grant_covers_capability',
  'no_prohibition',
  'lifecycle_admission',
  'tenant_negative',
  'expiry_negative',
  'suspension_or_revocation_negative',
])

export const CONDITIONAL_WITNESSES = Object.freeze([
  'data_scope',
  'tool_eligibility',
  'credential_eligibility',
  'policy',
  'approval',
  'cost_admission',
  'evidence_requirement',
])

export const DENIAL_REASONS = Object.freeze([
  'CONTRACT_EXPIRED',
  'WORKER_SUSPENDED',
  'CAPABILITY_NOT_GRANTED',
  'CAPABILITY_PROHIBITED',
  'WORKORDER_OUT_OF_SCOPE',
  'APPROVAL_REQUIRED',
  'CREDENTIAL_NOT_ELIGIBLE',
  'COST_ENVELOPE_EXCEEDED',
  'WORKER_TERMINATED',
  'CONTRACT_REVOKED',
  'CROSS_TENANT',
  'DELEGATION_OVERREACH',
])

export const AUDIT_EVENT_CATEGORIES = Object.freeze([
  'worker_hired',
  'contract_created',
  'contract_superseded',
  'worker_activated',
  'worker_restricted',
  'worker_suspended',
  'assignment_created',
  'assignment_denied',
  'assignment_completed',
  'worker_transferred',
  'worker_terminated',
])

export const CAP16_FUTURE_OWNERS = Object.freeze({
  CAP9: 'organization/position',
  CAP12: 'credentials/session',
  CAP15: 'external WebApps (WEB2APP)',
  CAP16: 'worker employment governance (future; not implemented by this standard)',
  policy_approval: 'authority primitives already canonical',
  CAP6: 'schedule',
  CAP8: 'evidence/artifacts',
  CAP10: 'retention',
})

export function employmentRank(level) {
  return EMPLOYMENT_RANK[level] ?? -1
}

export function minEmployment(a, b) {
  return employmentRank(a) <= employmentRank(b) ? a : b
}

export function maxEmployment(a, b) {
  return employmentRank(a) >= employmentRank(b) ? a : b
}
