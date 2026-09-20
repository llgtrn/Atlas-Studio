// Benchmark Contract IR vocabulary. Sits above existing evaluators. Does not replace them.
// Does not implement CAP runtime, Thread 4, WorkRequest, or product authority.

import { PEER_ACTORS } from './vocab.mjs'
import { WEB2APP_STANDARD_VERSION } from './web2app-vocab.mjs'
import { EMPLOYMENT_STANDARD_VERSION } from './employment-vocab.mjs'
import { STORE_STANDARD_VERSION } from './store-vocab.mjs'
import { BLACKBOX_STANDARD_VERSION } from './blackbox-vocab.mjs'

export const CONTRACT_SCHEMA = 'chronica.benchmark.contract.v1'
export const CONTRACT_SCHEMA_VERSION = '1.0.0'
export const CONTRACT_RELEASE_PREFIX = 'BR'

export const CAP_IDS = Object.freeze([
  'CAP01',
  'CAP02',
  'CAP03',
  'CAP04',
  'CAP05',
  'CAP06',
  'CAP07',
  'CAP08',
  'CAP09',
  'CAP10',
  'CAP11',
  'CAP12',
  'CAP13',
  'CAP14',
  'CAP15',
  'CAP16',
  'CAP17',
  'CAP18',
])

export const CAP_NAMES = Object.freeze({
  CAP01: 'Helpdesk',
  CAP02: 'Social/feed',
  CAP03: 'Backup/restore',
  CAP04: 'Observability',
  CAP05: 'CRM',
  CAP06: 'Scheduler/routines',
  CAP07: 'Notifications',
  CAP08: 'Artifacts/documents',
  CAP09: 'Company/organization',
  CAP10: 'Data lifecycle',
  CAP11: 'Comments/collaboration',
  CAP12: 'Identity/sessions/secrets',
  CAP13: 'Inbound webhooks',
  CAP14: 'Pages/knowledge',
  CAP15: 'WEB2APP',
  CAP16: 'Digital worker employment',
  CAP17: 'Store platform',
  CAP18: 'Confidential black-box data plane',
})

export const STANDARD_CATALOG = Object.freeze({
  WEB2APP: { id: '205', version: WEB2APP_STANDARD_VERSION, schema: 'chronica.web2app.evaluation.v1' },
  EMPLOYMENT: { id: '206', version: EMPLOYMENT_STANDARD_VERSION, schema: 'chronica.employment.evaluation.v1' },
  STORE: { id: '207', version: STORE_STANDARD_VERSION, schema: 'chronica.store.evaluation.v1' },
  BLACKBOX: { id: '208', version: BLACKBOX_STANDARD_VERSION, schema: 'chronica.blackbox.evaluation.v1' },
  FIC: { id: '180', version: '55', schema: 'chronica.benchmark-truth-snapshot.v1' },
  FIVE_PLANE: { id: 'audit', version: 'PROVISIONAL_BASELINE', schema: 'chronica.benchmark-truth-snapshot.v1' },
})

export const RECORD_TYPES = Object.freeze([
  'global_law',
  'family',
  'cap_envelope',
  'flow',
  'golden_flow',
  'evidence',
  'cross_cap_edge',
  'prose_statement',
  'anti_pattern_meta',
])

export const CONTRACT_LEVELS = Object.freeze(['L0', 'L1', 'L2', 'L3'])

export const PROOF_OBLIGATIONS = Object.freeze([
  'PO-ENTRYPOINT',
  'PO-STATE',
  'PO-PERSISTENCE',
  'PO-TENANCY',
  'PO-IDENTITY',
  'PO-AUTHORITY',
  'PO-APPROVAL',
  'PO-DURABILITY',
  'PO-IDEMPOTENCY',
  'PO-CONCURRENCY',
  'PO-FAILURE',
  'PO-RETRY',
  'PO-EVIDENCE',
  'PO-RECEIPT',
  'PO-CONSUMER',
  'PO-NO_AI',
  'PO-DISCLOSURE',
  'PO-REVOCATION',
  'PO-ROLLBACK',
  'PO-PRODUCTION',
  'PO-LOGGING',
])

export const FLOW_STATUSES = Object.freeze([
  'SPEC_ONLY',
  'NO_RUNTIME',
  'PARTIAL',
  'BLOCKED',
  'RUNTIME_CONNECTED',
  'VERIFIED',
  'PRODUCTION_EVIDENCED',
  'REVALIDATION_REQUIRED',
])

export const DEP_STATUSES = Object.freeze(['READY', 'PARTIAL', 'BLOCKED', 'UNKNOWN', 'WRONG_OWNER'])
export const OBLIGATION_STATES = Object.freeze(['PASS', 'FAIL', 'MISSING', 'BLOCKED', 'MANUAL_REVIEW'])
export const VALUE_LEVELS = Object.freeze(['LOW', 'MEDIUM', 'HIGH'])
export const ACTOR_CLASSES = PEER_ACTORS
export const PROSE_CLASSES = Object.freeze([
  'GLOBAL_LAW',
  'CAPABILITY_REQUIREMENT',
  'FLOW_REQUIREMENT',
  'ABUSE_INVARIANT',
  'EXPLANATORY_ONLY',
  'DONOR_REFERENCE',
  'MANUAL_REVIEW',
])
export const SEVERITIES = Object.freeze(['INFO', 'WARNING', 'HARD_FAIL', 'SECURITY_BLOCKED', 'MONEY_BLOCKED'])
export const CI_TIERS = Object.freeze([
  'TIER0_CONTRACT',
  'TIER1_AFFECTED_CAP',
  'TIER2_GLOBAL_HARD',
  'TIER3_GOLDEN',
  'TIER4_FULL_TRUTH',
])
export const EVIDENCE_KINDS = Object.freeze([
  'source_symbol',
  'http_route',
  'cli',
  'worker',
  'scheduler',
  'event_consumer',
  'migration',
  'db_constraint',
  'test',
  'integration_test',
  'runtime_receipt',
  'provider_operation',
  'audit_event',
  'projection',
  'ui_consumer',
  'production_evidence',
  'benchmark_fixture',
])
export const WITNESS_CHAIN = Object.freeze(['entrypoint', 'control', 'kernel', 'persistence', 'receipt', 'consumer'])
export const FIXTURE_KINDS = Object.freeze(['BENCHMARK_FIXTURE_ONLY', 'BENCHMARK_FIXTURE', 'CURRENT_RUNTIME_CALIBRATION'])

export const CONTRACT_ANTI_PATTERNS = Object.freeze([
  'CROSS_CAP_CONTRACT_FAIL',
  'FLOW_RUNTIME_WITNESS_BROKEN',
  'STALE_BENCHMARK',
  'STALE_CODE_EVIDENCE',
  'GENERATED_PACKET_DRIFT',
  'CAPABILITY_AS_FLOW',
  'FIXTURE_AS_RUNTIME',
  'SCORE_OVERRIDES_HARD_GATE',
  'LATEST_BENCHMARK_UNPINNED',
  'DONOR_REFERENCE_AS_FLOW',
  'BENCHMARK_AS_PRODUCT_AUTHORITY',
  'CAP_WEAKENS_OWN_STANDARD',
])

export const VERSION_CHANGE_RULES = Object.freeze({
  PATCH: 'typo, fixture repair, citation correction; no intended semantic change',
  MINOR: 'compatible new facet, flow, optional witness, or additive rule',
  MAJOR: 'hard gate, flow, or authority meaning changes incompatibly',
})

export const NON_NEGOTIABLES = Object.freeze({
  BENCHMARK_PROSE_IS_NOT_CAP_INSTRUCTION: true,
  CAPABILITY_ROW_IS_NOT_FLOW: true,
  TEST_EXISTS_IS_NOT_FLOW_PROVEN: true,
  LOCAL_CAP_GREEN_IS_NOT_CROSS_CAP_GREEN: true,
  SCORE_IS_NOT_HARD_GATE_PASS: true,
  LATEST_IS_NOT_PINNED_RELEASE: true,
  DONOR_REFERENCE_IS_NOT_PRODUCT_CONTRACT: true,
  FIXTURE_IS_NOT_RUNTIME: true,
  GENERATED_PACKET_IS_NOT_SOURCE: true,
  BENCHMARK_CONTRACT_IS_NOT_PRODUCT_AUTHORITY: true,
  CI_RUNNER_IS_NOT_PRODUCT_KERNEL: true,
})
