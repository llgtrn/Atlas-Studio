// WEB2APP standard vocab. Extends the benchmark truth engine; does not replace FIC or the five-plane freeze.

export const WEB2APP_STANDARD_VERSION = '1.1.0'
export const WEB2APP_SCHEMA = 'chronica.web2app.evaluation.v1'
export const SEMANTIC_CONTRACT_SCHEMA = 'chronica.web2app.semantic-contract.v1'
export const FIXED_POINT_SCHEMA = 'chronica.web2app.fixed-point.v1'
export const WEB2APP_PRIOR_STANDARD_VERSION = '1.0.0'

export const CANNOT_WEAKEN_MEANING_BECAUSE_SUBSTRATE_MISSING = true
export const ACTIONABLE_ZERO_DOES_NOT_PROVE_FIXED_POINT_IF_DENOMINATOR_WRONG = true

export const WEB2APP_LEVELS = Object.freeze(['W0', 'W1', 'W2', 'W3', 'W4', 'W5'])
export const OBSERVATION_LEVELS = Object.freeze(['O0', 'O1', 'O2', 'O3', 'O4', 'O5'])
export const FRESHNESS_CLASSES = Object.freeze([
  'PUSH_REALTIME',
  'NEAR_REALTIME',
  'POLL_INTERVAL',
  'ON_DEMAND',
  'MANUAL_REFRESH',
])
export const SOURCE_HEALTH_STATES = Object.freeze([
  'CONNECTED',
  'AUTH_EXPIRED',
  'ENTITLEMENT_MISSING',
  'STALE',
  'RATE_LIMITED',
  'HUMAN_INTERACTION_REQUIRED',
  'SOURCE_DOWN',
])
export const OBSERVATION_OPTIONAL_FIELDS = Object.freeze([
  'observation_maturity',
  'claimed_observation_maturity',
  'observation_sources',
  'push_supported',
  'poll_supported',
  'on_demand_refresh',
  'checkpoint_durable',
  'dedupe_durable',
  'process_local_dedupe_only',
  'reconciliation',
  'freshness_target',
  'freshness_class',
  'freshness_observed',
  'credential_required',
  'credential_handle_only',
  'subscription_entitlement',
  'source_health',
  'staleness_visible',
  'webhook_authenticated',
  'webhook_signature_verified',
  'restart_replay_proven',
  'real_provider_evidence',
  'persistent_poll',
])

export const PRODUCT_KINDS = Object.freeze([
  'APP',
  'DOCUMENT',
  'TRANSFORM',
  'CLI_OPERATOR',
  'SCHEDULER',
  'INTEGRATION',
  'WORKFLOW',
  'INGEST',
])

export const PRODUCT_CHAIN_SHAPES = Object.freeze([
  'KERNEL_ONLY',
  'HARNESS_ONLY',
  'PRODUCT_EDGE_ONLY',
  'BACKEND_COMPLETE',
  'FULL_WEB2APP',
])

export const NORMALIZATION_PLANES = Object.freeze(['P0', 'P1', 'P2', 'P3', 'P4', 'P5'])

export const FACET_STATES = Object.freeze([
  'LIVE',
  'PARTIAL',
  'LABEL_ONLY',
  'SCHEMA_ONLY',
  'CONFIG_ONLY',
  'STORED_NOT_ENFORCED',
  'ABSENT',
  'BLOCKED',
])

export const VERDICTS = Object.freeze(['PASS', 'PARTIAL', 'FAIL', 'BLOCKED'])

export const OPERATION_KINDS = Object.freeze(['READ', 'WRITE', 'MONEY', 'EXTERNAL'])

export const JURISDICTION_MODELS = Object.freeze([
  'COMMON_CORE_PLUS_EXTENSIONS',
  'SINGLE_JURISDICTION',
  'NOT_APPLICABLE',
])

export const CANONICAL_STATUSES = Object.freeze([
  'verified',
  'implemented_unverified',
  'unimplemented',
  'blocked',
  'excluded',
])

export const RUNTIME_STATUSES = Object.freeze([
  'LIVE',
  'LIVE_BUT_UNVERIFIED',
  'ISLAND',
  'FAKE_COUPLING',
  'DOC_ONLY',
  'TRACKING_ONLY',
  'TEST_ONLY',
  'EXTERNAL_ACTION_BLOCKED',
  'LOCAL_AUDIT_REQUIRED',
  'MONEY_BLOCKED',
  'SECURITY_BLOCKED',
])

export const SEMANTIC_CONTRACT_FIELDS = Object.freeze([
  'capability_key',
  'canonical_name',
  'product_kind',
  'actor',
  'user_intent',
  'preconditions',
  'input',
  'product_action',
  'state_transition',
  'output',
  'persistence_requirement',
  'authority_requirement',
  'tenant_requirement',
  'visibility_access_semantics',
  'failure_semantics',
  'idempotency_semantics',
  'concurrency_semantics',
  'external_effect_requirement',
  'money_impact',
  'user_observable_outcome',
  'no_ai_behavior',
  'donor_semantics',
  'chronica_allowed_divergences',
  'known_unimplemented_facets',
])

export const WITNESS_NAMES = Object.freeze([
  'real_actor',
  'real_product_entrypoint',
  'tenant_boundary',
  'auth_boundary',
  'deterministic_kernel',
  'persistence',
  'external_effect',
  'idempotency',
  'concurrency',
  'negative_failure_paths',
  'real_readback',
  'real_ui_consumer',
  'no_ai_path',
  'money_authority',
  'effect_receipt',
  'production_evidence',
  'typed_operations',
  'jurisdiction_extension',
])

export const WEB2APP_ANTI_PATTERNS = Object.freeze([
  'HARNESS_WITHOUT_ENGINE',
  'WORK_WITHOUT_DURABILITY',
  'EXECUTION_WITHOUT_RECEIPT',
  'PREVIEW_AS_PRODUCT',
  'CALLER_FARMING',
  'LABEL_AS_ENFORCEMENT',
  'UI_WITHOUT_RUNTIME',
  'ROUTE_WITHOUT_PRODUCT',
  'TRACKING_AS_PRODUCT',
  'DONOR_NAME_WITHOUT_DONOR_SEMANTICS',
  'GOALPOST_MUTATION',
  'DENOMINATOR_SHRINKING',
  'OWNERSHIP_LAUNDERING',
  'TEST_AS_RUNTIME',
  'MOCK_AS_PRODUCTION',
  'AI_IN_HOT_PATH',
  'AGENT_AS_KERNEL',
  'PROMPT_AS_SOURCE_OF_TRUTH',
  'LLM_SELF_AUTHORIZATION',
  'DATA_WITHOUT_OPERATIONS',
  'ADAPTER_AS_APP',
  'DOM_AS_KERNEL',
  'UNTYPED_DO_WHATEVER',
  'JURISDICTION_COLLAPSE',
  'TRANSPORT_LEAKAGE',
  'ASSISTANT_WITHOUT_STATE_CHANGE',
  'PULL_ONLY_AS_REALTIME',
  'WEBHOOK_WITHOUT_AUTH',
  'WEBHOOK_WITHOUT_DEDUPE',
  'POLL_WITHOUT_CHECKPOINT',
  'EVENT_WITHOUT_NORMALIZATION',
  'EVENT_AS_COMMAND',
  'PUSH_WITHOUT_RECONCILIATION',
  'STALE_STATE_AS_CURRENT',
  'CREDENTIAL_IN_APP_DEFINITION',
  'CREDENTIAL_IN_AGENT_CONTEXT',
  'ENTITLEMENT_WITHOUT_TENANT_SCOPE',
  'FULL_RECOMPILE_ON_EVERY_POLL',
  'RAW_PROVIDER_PAYLOAD_AS_WORLD_MODEL',
])

export const SCORE_WEIGHTS = Object.freeze({
  semantic_fidelity: 20,
  runtime_reachability: 15,
  durability_effect: 15,
  auth_tenancy: 10,
  failure_semantics: 10,
  idempotency_concurrency: 10,
  product_consumer: 10,
  evidence_quality: 10,
})

export const LEVEL_RANK = Object.freeze({ W0: 0, W1: 1, W2: 2, W3: 3, W4: 4, W5: 5 })
export const OBSERVATION_RANK = Object.freeze({ O0: 0, O1: 1, O2: 2, O3: 3, O4: 4, O5: 5 })

export function rank(level) {
  return LEVEL_RANK[level] ?? -1
}

export function minLevel(a, b) {
  return rank(a) <= rank(b) ? a : b
}

export function observationRank(level) {
  return OBSERVATION_RANK[level] ?? -1
}

export function minObservation(a, b) {
  return observationRank(a) <= observationRank(b) ? a : b
}

export const PREVIEW_SURFACE_RE = /preview|fold|replay|simulate/i
export const CLI_SURFACE_RE = /\bcli\b|command-line|stdin evaluator/i
