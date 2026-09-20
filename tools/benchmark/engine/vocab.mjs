// Frozen vocabularies for the benchmark truth engine.
// Architecture conclusions stay provisional. These are tooling terms, not a second FIC ladder.

export const SCHEMA = 'chronica.benchmark-truth-snapshot.v1'
export const GENERATOR_VERSION = 'chronica.benchmark-truth-engine.v1'
export const BENCHMARK_SCHEMA_VERSION = SCHEMA

export const FRESHNESS = Object.freeze(['CURRENT', 'PROVISIONAL', 'STALE', 'INVALID'])

export const PLANES = Object.freeze([
  'INTELLIGENCE',
  'CONTROL_AUTHORITY',
  'DETERMINISTIC_KERNEL',
  'EFFECT_DATA',
  'EVIDENCE_OPERATIONS',
])

export const PLANE_ASSESSMENTS = Object.freeze(['MISSING', 'WEAK', 'PARTIAL', 'STRONG'])

export const CONFIDENCE = Object.freeze(['LOW', 'MEDIUM', 'HIGH'])

export const NO_AI_CLASSES = Object.freeze([
  'STILL_WORKS_WITHOUT_AI',
  'OPTIONAL_INTELLIGENCE',
  'REQUIRED_INFERENCE',
  'REQUIRES_AI_TO_PROGRESS_BUSINESS_STATE',
])

export const PROVIDER_KINDS = Object.freeze([
  'AI_INFERENCE_PROVIDER',
  'DETERMINISTIC_EXTERNAL_EFFECT_PROVIDER',
  'NOT_A_PROVIDER',
])

export const PEER_ACTORS = Object.freeze(['human', 'agent', 'API', 'scheduler', 'webhook', 'system'])

export const CONTRACT_RELATIONS = Object.freeze([
  'PARENT',
  'CHILD',
  'SUPERSEDES',
  'REFINES',
  'CONSUMES',
  'CONFLICTS',
  'INDEPENDENT',
])

export const AUTHORITY_MODEL_STATES = Object.freeze(['EXISTING_MODEL_SUFFICIENT', 'MISSING', 'NOT_PROVEN'])
export const EXECUTION_GRANT_STATES = Object.freeze(['PROVEN', 'NOT_YET_PROVEN', 'MISSING'])

export const RUNTIME_KINDS = Object.freeze([
  'PRODUCT_RUNTIME',
  'INTERNAL_RUNTIME',
  'TOOLING_ONLY',
  'VALIDATE_ONLY',
  'ISLAND',
  'TRACKING_ONLY',
])

export const EDGE_STATUSES = Object.freeze([
  'LIVE',
  'LIVE_BUT_UNVERIFIED',
  'PARTIAL_RUNTIME',
  'ISLAND',
  'MISSING',
  'BLOCKED',
  'FAKE_COUPLING',
])

export const CLAIM_LEVELS = Object.freeze(['Substrate', 'NativeSlice', 'ExternalClass', 'ProductionReady'])

export const DIFF_CLASSES = Object.freeze([
  'CODE_CHANGE',
  'EVIDENCE_CHANGE',
  'CLASSIFICATION_CHANGE',
  'DENOMINATOR_CHANGE',
])

export const ANTI_PATTERNS = Object.freeze([
  'AGENT_AS_KERNEL',
  'HARNESS_WITHOUT_ENGINE',
  'AI_IN_HOT_PATH',
  'CONTROL_PLANE_OWNS_BUSINESS_STATE',
  'PROMPT_AS_SOURCE_OF_TRUTH',
  'LLM_SELF_AUTHORIZATION',
  'EXECUTION_WITHOUT_RECEIPT',
  'WORK_WITHOUT_DURABILITY',
  'RECOVERY_REQUIRES_AGENT_REASONING',
])

export const ANTI_INFLATION_CODES = Object.freeze([
  'SIDE_EFFECT_CLASS_MISMATCH',
  'VALIDATE_ONLY_CREATE_CLAIM',
  'NON_PRODUCT_CALLER',
  'EXECUTION_WITHOUT_RECEIPT',
  'WORK_WITHOUT_DURABILITY',
])

export const FINDING_DRIFT = 'PROVISIONAL_FINDING_DRIFT'

export const FROZEN_VERDICT = Object.freeze({
  five_plane_audit_status: 'PROVISIONAL_BASELINE',
  verdict: 'B',
  verdict_label: 'HARNESS-HEAVY HYBRID',
  core_spine: 'PROVISIONALLY_REQUIRED',
  authoritative_architecture_verdict: false,
})

export const FIC_DENOMINATOR = 55
export const FAMILY_DENOMINATOR = 11

export const DETERMINISTIC_EFFECT_EXAMPLES = Object.freeze([
  'payment provider',
  'email transport',
  'carrier',
  'webhook delivery',
  'browser effect',
  'bank transport',
])

export const LIVE_PLANE_KEY_TO_ENGINE = Object.freeze({
  intelligence: 'INTELLIGENCE',
  control: 'CONTROL_AUTHORITY',
  kernel: 'DETERMINISTIC_KERNEL',
  effect_data: 'EFFECT_DATA',
  evidence_ops: 'EVIDENCE_OPERATIONS',
})
