import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { loadV1Denominator } from '../fic-recompute-lib.mjs'
import { loadJson } from '../fic-recompute-lib.mjs'
import { guardFicDenominator } from './fic-guard.mjs'
import { FIC_DENOMINATOR } from './vocab.mjs'
import { evaluateWeb2AppAntiPattern } from './web2app-anti-patterns.mjs'
import { evaluateWeb2AppRecord, validateFixedPointReport, validateSemanticContract } from './web2app.mjs'
import {
  ACTIONABLE_ZERO_DOES_NOT_PROVE_FIXED_POINT_IF_DENOMINATOR_WRONG,
  CANNOT_WEAKEN_MEANING_BECAUSE_SUBSTRATE_MISSING,
  OBSERVATION_LEVELS,
  WEB2APP_PRIOR_STANDARD_VERSION,
  WEB2APP_SCHEMA,
  WEB2APP_STANDARD_VERSION,
} from './web2app-vocab.mjs'
import {
  createReactiveNewsFixture,
  FREE_SECRET_CANARY,
  PREMIUM_SECRET_CANARY,
} from './web2app-reactive-fixture.mjs'

const ROOT = fileURLToPath(new URL('../../../', import.meta.url))
const FIX = join(ROOT, 'tools/benchmark/engine/fixtures/web2app')

function loadFix(name) {
  return JSON.parse(readFileSync(join(FIX, name), 'utf8'))
}

function baseContract(overrides = {}) {
  return {
    schema: 'chronica.web2app.semantic-contract.v1',
    capability_key: 'test.cap',
    canonical_name: 'Test Cap',
    product_kind: 'APP',
    actor: 'user',
    user_intent: 'do the thing',
    preconditions: 'auth',
    input: 'fields',
    product_action: 'create',
    typed_operations: [{ name: 'thing.create', kind: 'WRITE', authority: 'company', receipt: 'row' }],
    state_transition: 'absent → persisted',
    output: 'row',
    persistence_requirement: 'durable store',
    authority_requirement: 'verified principal',
    tenant_requirement: 'company-scoped',
    visibility_access_semantics: 'company',
    failure_semantics: '403/404',
    idempotency_semantics: 'none',
    concurrency_semantics: 'none',
    external_effect_requirement: 'none',
    money_impact: 'none',
    user_observable_outcome: 'row visible',
    no_ai_behavior: 'deterministic',
    donor_semantics: 'test donor',
    chronica_allowed_divergences: 'none',
    known_unimplemented_facets: 'none',
    requires_persistence: true,
    requires_ui: true,
    requires_external_effect: false,
    moves_money: false,
    requires_ai: false,
    ...overrides,
  }
}

test('WEB2APP hard answers are frozen', () => {
  assert.equal(CANNOT_WEAKEN_MEANING_BECAUSE_SUBSTRATE_MISSING, true)
  assert.equal(ACTIONABLE_ZERO_DOES_NOT_PROVE_FIXED_POINT_IF_DENOMINATOR_WRONG, true)
  assert.equal(WEB2APP_STANDARD_VERSION, '1.1.0')
  assert.equal(WEB2APP_PRIOR_STANDARD_VERSION, '1.0.0')
  assert.equal(WEB2APP_SCHEMA, 'chronica.web2app.evaluation.v1')
  assert.deepEqual(OBSERVATION_LEVELS, ['O0', 'O1', 'O2', 'O3', 'O4', 'O5'])
})

test('FIC denominator remains 55 and is not WEB2APP coverage', () => {
  const v1 = loadV1Denominator(ROOT)
  const census = loadJson(join(ROOT, 'docs/_machine/fic-revalidation-census.json'))
  const guard = guardFicDenominator(v1, census)
  assert.equal(guard.fic.denominator, FIC_DENOMINATOR)
  assert.equal(guard.fic.denominator, 55)
})

test('semantic contract cannot be filler', () => {
  const result = validateSemanticContract(baseContract({ user_intent: 'TBD' }))
  assert.equal(result.ok, false)
  assert.equal(validateSemanticContract(baseContract({ money_impact: 'none', external_effect_requirement: 'none' })).ok, true)
})

test('verified durable capability with no persistence witness is a contradiction', () => {
  const result = evaluateWeb2AppRecord({
    schema: WEB2APP_SCHEMA,
    canonical_status: 'verified',
    claimed_web2app_level: 'W2',
    semantic_contract: baseContract(),
    actual_witnesses: [
      'real_actor',
      'real_product_entrypoint',
      'deterministic_kernel',
      'negative_failure_paths',
      'no_ai_path',
      'typed_operations',
      'tenant_boundary',
      'auth_boundary',
    ],
    entrypoints: ['POST /documents/preview'],
    facets: [],
    semantic_mutations: [],
  })
  assert.ok(result.contradictions.some((row) => /verified durable/.test(row)))
  assert.equal(result.web2app_level, 'W2')
  assert.ok(result.anti_patterns_found.includes('PREVIEW_AS_PRODUCT'))
  assert.ok(result.anti_patterns_found.includes('WORK_WITHOUT_DURABILITY'))
})

test('W3/W4 claim with no real runtime caller fails', () => {
  const result = evaluateWeb2AppRecord({
    schema: WEB2APP_SCHEMA,
    claimed_web2app_level: 'W4',
    semantic_contract: baseContract(),
    actual_witnesses: ['real_actor', 'deterministic_kernel', 'negative_failure_paths', 'no_ai_path', 'typed_operations'],
    entrypoints: [],
    facets: [],
    semantic_mutations: [],
  })
  assert.ok(result.hard_gate_failures.includes('W3/W4 claim with no real runtime caller'))
  assert.equal(result.verdict, 'FAIL')
  assert.equal(result.web2app_level, 'W1')
})

test('LABEL_ONLY facet claimed LIVE fails', () => {
  const result = evaluateWeb2AppRecord({
    schema: WEB2APP_SCHEMA,
    claimed_web2app_level: 'W4',
    semantic_contract: baseContract(),
    actual_witnesses: [
      'real_actor',
      'real_product_entrypoint',
      'deterministic_kernel',
      'negative_failure_paths',
      'persistence',
      'real_readback',
      'real_ui_consumer',
      'no_ai_path',
      'typed_operations',
      'tenant_boundary',
      'auth_boundary',
    ],
    facets: [{ name: 'visibility.holding', semantic_state: 'LIVE', label_only: true }],
    semantic_mutations: [],
  })
  assert.ok(result.hard_gate_failures.includes('LABEL_ONLY facet claimed LIVE'))
  assert.equal(result.verdict, 'FAIL')
})

test('UI-only capability claiming runtime fails', () => {
  const result = evaluateWeb2AppRecord({
    schema: WEB2APP_SCHEMA,
    claimed_web2app_level: 'W4',
    semantic_contract: baseContract(),
    actual_witnesses: ['real_actor', 'real_ui_consumer'],
    entrypoints: [],
    facets: [],
    semantic_mutations: [],
  })
  assert.ok(result.hard_gate_failures.includes('UI-only capability claiming runtime'))
})

test('preview-only route claiming durable app fails', () => {
  const result = evaluateWeb2AppRecord({
    schema: WEB2APP_SCHEMA,
    claimed_web2app_level: 'W4',
    semantic_contract: baseContract(),
    actual_witnesses: [
      'real_actor',
      'real_product_entrypoint',
      'deterministic_kernel',
      'negative_failure_paths',
      'no_ai_path',
      'typed_operations',
      'tenant_boundary',
      'auth_boundary',
    ],
    entrypoints: ['POST /documents/preview'],
    facets: [],
    semantic_mutations: [],
  })
  assert.ok(result.hard_gate_failures.includes('preview-only route claiming durable app'))
})

test('GOALPOST_MUTATION fails regardless of score', () => {
  const result = evaluateWeb2AppRecord({
    schema: WEB2APP_SCHEMA,
    claimed_web2app_level: 'W4',
    semantic_contract: baseContract({
      visibility_access_semantics: 'Holding = future label only',
    }),
    actual_witnesses: [
      'real_actor',
      'real_product_entrypoint',
      'deterministic_kernel',
      'negative_failure_paths',
      'persistence',
      'real_readback',
      'real_ui_consumer',
      'no_ai_path',
      'typed_operations',
      'tenant_boundary',
      'auth_boundary',
    ],
    facets: [],
    semantic_mutations: [
      {
        field: 'visibility.holding',
        from: 'visible across holding',
        to: 'future label only',
        reason: 'store cannot authorize holding-wide',
        independent_review: false,
      },
    ],
  })
  assert.ok(result.hard_gate_failures.includes('GOALPOST_MUTATION'))
  assert.equal(result.verdict, 'FAIL')
  assert.equal(result.score.authoritative, false)
})

test('money capability without authority witness fails', () => {
  const result = evaluateWeb2AppRecord({
    schema: WEB2APP_SCHEMA,
    claimed_web2app_level: 'W3',
    semantic_contract: baseContract({
      product_kind: 'INTEGRATION',
      money_impact: 'moves_money',
      moves_money: true,
      requires_ui: false,
      external_effect_requirement: 'payment provider',
      requires_external_effect: true,
    }),
    actual_witnesses: [
      'real_actor',
      'real_product_entrypoint',
      'deterministic_kernel',
      'negative_failure_paths',
      'persistence',
      'real_readback',
      'no_ai_path',
      'typed_operations',
      'external_effect',
      'effect_receipt',
    ],
    facets: [],
    semantic_mutations: [],
  })
  assert.ok(result.hard_gate_failures.includes('money capability without authority witness'))
  assert.equal(result.verdict, 'FAIL')
})

test('external-effect LIVE facet without receipt fails', () => {
  const result = evaluateWeb2AppRecord({
    schema: WEB2APP_SCHEMA,
    claimed_web2app_level: 'W3',
    semantic_contract: baseContract({
      requires_ui: false,
      requires_external_effect: true,
      external_effect_requirement: 'provider call',
    }),
    actual_witnesses: [
      'real_actor',
      'real_product_entrypoint',
      'deterministic_kernel',
      'negative_failure_paths',
      'persistence',
      'real_readback',
      'no_ai_path',
      'typed_operations',
      'external_effect',
    ],
    facets: [{ name: 'provider_dispatch', semantic_state: 'LIVE' }],
    semantic_mutations: [],
  })
  assert.ok(result.hard_gate_failures.includes('external-effect completion with no effect receipt'))
})

test('fixed-point semantic denominator change without justification is invalid', () => {
  const result = validateFixedPointReport({
    schema: 'chronica.web2app.fixed-point.v1',
    pass0_contracts_frozen: true,
    pass1: { semantic_owner_denominator: 40, actionable: 0, unclassified: 0 },
    pass2: { semantic_owner_denominator: 12, actionable: 0, unclassified: 0 },
    semantic_contract_mutations: 0,
    unjustified_ownership_changes: 0,
  })
  assert.equal(result.verdict, 'FIXED_POINT_INVALID')
  assert.ok(result.hard_gate_failures.includes('DENOMINATOR_SHRINKING'))
  assert.equal(ACTIONABLE_ZERO_DOES_NOT_PROVE_FIXED_POINT_IF_DENOMINATOR_WRONG, true)
})

test('ACTIONABLE=0 twice is not enough if contracts were mutated', () => {
  const result = validateFixedPointReport({
    schema: 'chronica.web2app.fixed-point.v1',
    pass0_contracts_frozen: true,
    pass1: { semantic_owner_denominator: 40, actionable: 0, unclassified: 0 },
    pass2: { semantic_owner_denominator: 40, actionable: 0, unclassified: 0 },
    semantic_contract_mutations: 1,
    unjustified_ownership_changes: 0,
  })
  assert.equal(result.code, 'CAPx_SEMANTIC_REVIEW_REQUIRED')
  assert.ok(result.hard_gate_failures.includes('GOALPOST_MUTATION'))
})

test('anti-pattern keyword grep is rejected', () => {
  const grep = evaluateWeb2AppAntiPattern('PREVIEW_AS_PRODUCT', { keyword_only: true, detected_by: 'grep' })
  assert.equal(grep.ok, false)
})

test('Case A pages_cms is W4 PARTIAL because holding is LABEL_ONLY', () => {
  const result = evaluateWeb2AppRecord(loadFix('other.pages_cms.json'))
  assert.equal(result.errors.length, 0, result.errors.join('\n'))
  assert.equal(result.web2app_level, 'W4')
  assert.equal(result.verdict, 'PARTIAL')
  assert.ok(result.anti_patterns_found.includes('LABEL_AS_ENFORCEMENT'))
  assert.equal(result.canonical_status, 'verified')
  assert.equal(result.observation_maturity, 'O0')
})

test('Case B documents is W2 preview kernel, not durable notebook', () => {
  const result = evaluateWeb2AppRecord(loadFix('agent-knowledge.documents.json'))
  assert.equal(result.errors.length, 0, result.errors.join('\n'))
  assert.equal(result.web2app_level, 'W2')
  assert.ok(result.contradictions.some((row) => /verified durable/.test(row)))
  assert.ok(result.anti_patterns_found.includes('PREVIEW_AS_PRODUCT'))
  assert.ok(result.anti_patterns_found.includes('DONOR_NAME_WITHOUT_DONOR_SEMANTICS'))
  assert.notEqual(result.verdict, 'PASS')
  assert.equal(result.observation_maturity, 'O0')
})

test('Case C notes is W4 PASS positive path', () => {
  const result = evaluateWeb2AppRecord(loadFix('other.note_create_engine.json'))
  assert.equal(result.errors.length, 0, result.errors.join('\n'))
  assert.equal(result.web2app_level, 'W4')
  assert.equal(result.verdict, 'PASS')
  assert.equal(result.observation_maturity, 'O0')
})

test('Case D schedule is W3 PARTIAL backend runtime without action execution', () => {
  const result = evaluateWeb2AppRecord(loadFix('workflow-runtime.schedule_triggers.json'))
  assert.equal(result.errors.length, 0, result.errors.join('\n'))
  assert.equal(result.web2app_level, 'W3')
  assert.equal(result.verdict, 'PARTIAL')
  assert.ok(result.anti_patterns_found.includes('EXECUTION_WITHOUT_RECEIPT'))
  assert.equal(result.observation_maturity, 'O0')
})

test('Case E backup CLI is legitimate product kind and currently W1 island', () => {
  const result = evaluateWeb2AppRecord(loadFix('infra.backup_restore.json'))
  assert.equal(result.errors.length, 0, result.errors.join('\n'))
  assert.equal(result.web2app_level, 'W1')
  assert.equal(result.semantic_contract_ok ?? true, true)
  assert.equal(loadFix('infra.backup_restore.json').semantic_contract.product_kind, 'CLI_OPERATOR')
  assert.equal(result.observation_maturity, 'O0')
})

test('routines_engine CLI is CALLER_FARMING, unlike backup CLI', () => {
  const result = evaluateWeb2AppRecord(loadFix('workflow-runtime.routines_engine.json'))
  assert.equal(result.errors.length, 0, result.errors.join('\n'))
  assert.ok(result.anti_patterns_found.includes('CALLER_FARMING'))
  assert.ok(['W1', 'W2'].includes(result.web2app_level))
})

test('weakening Holding because substrate is missing is GOALPOST_MUTATION, not a verified facet', () => {
  const honest = evaluateWeb2AppRecord(loadFix('other.pages_cms.json'))
  assert.equal(honest.verdict, 'PARTIAL')
  const mutated = structuredClone(loadFix('other.pages_cms.json'))
  mutated.semantic_contract.visibility_access_semantics = 'Holding is a future label only'
  mutated.facets = mutated.facets.map((facet) =>
    facet.name === 'visibility.holding'
      ? { ...facet, semantic_state: 'LIVE', label_only: true, claimed_as: 'LIVE' }
      : facet,
  )
  mutated.semantic_mutations = [
    {
      field: 'visibility.holding',
      from: 'visible across holding',
      to: 'future label only',
      independent_review: false,
    },
  ]
  const result = evaluateWeb2AppRecord(mutated)
  assert.ok(result.hard_gate_failures.includes('GOALPOST_MUTATION'))
  assert.ok(result.hard_gate_failures.includes('LABEL_ONLY facet claimed LIVE'))
  assert.equal(honest.observation_maturity, 'O0')
})

test('1.1.0 observation fields are optional and default to O0', () => {
  const result = evaluateWeb2AppRecord(loadFix('other.note_create_engine.json'))
  assert.equal(result.observation_maturity, 'O0')
  assert.equal(result.standard_version, '1.1.0')
})

test('news poll fixture is W3 + O2 and does not claim O5', () => {
  const result = evaluateWeb2AppRecord(loadFix('fixture.news.reactive_poll.json'))
  assert.equal(result.errors.length, 0, result.errors.join('\n'))
  assert.equal(result.web2app_level, 'W3')
  assert.equal(result.observation_maturity, 'O2')
  assert.notEqual(result.observation_maturity, 'O5')
})

test('news push+reconcile fixture is O4, not O5, and not a second engine', () => {
  const result = evaluateWeb2AppRecord(loadFix('fixture.news.reactive_push_reconcile.json'))
  assert.equal(result.errors.length, 0, result.errors.join('\n'))
  assert.equal(result.observation_maturity, 'O4')
  assert.ok(!result.hard_gate_failures.includes('claimed observation O4 exceeds honest O4'))
})

test('unsigned webhook cannot claim trusted O3', () => {
  const result = evaluateWeb2AppRecord(loadFix('fixture.news.webhook_without_auth.json'))
  assert.equal(result.observation_maturity, 'O1')
  assert.ok(result.hard_gate_failures.some((row) => /claimed observation O3 exceeds honest O1/.test(row)))
  assert.ok(result.anti_patterns_found.includes('WEBHOOK_WITHOUT_AUTH'))
  assert.ok(result.anti_patterns_found.includes('WEBHOOK_WITHOUT_DEDUPE'))
})

test('persistent poll without checkpoint caps at O1', () => {
  const result = evaluateWeb2AppRecord({
    schema: WEB2APP_SCHEMA,
    claimed_observation_maturity: 'O2',
    semantic_contract: baseContract({
      product_kind: 'INGEST',
      requires_ui: false,
      requires_persistence: false,
      persistence_requirement: 'none',
    }),
    actual_witnesses: [
      'real_actor',
      'real_product_entrypoint',
      'deterministic_kernel',
      'negative_failure_paths',
      'no_ai_path',
    ],
    observation: {
      poll_supported: true,
      persistent_poll: true,
      checkpoint_durable: false,
      observation_sources: ['poll.no-checkpoint'],
    },
    entrypoints: ['GET /poll'],
    facets: [],
    semantic_mutations: [],
  })
  assert.equal(result.observation_maturity, 'O1')
  assert.ok(result.anti_patterns_found.includes('POLL_WITHOUT_CHECKPOINT'))
  assert.ok(result.hard_gate_failures.some((row) => /claimed observation O2 exceeds honest O1/.test(row)))
})

test('process-local dedupe only caps at O2', () => {
  const result = evaluateWeb2AppRecord({
    schema: WEB2APP_SCHEMA,
    claimed_observation_maturity: 'O4',
    semantic_contract: baseContract({ product_kind: 'INGEST', requires_ui: false, requires_persistence: true }),
    actual_witnesses: [
      'real_actor',
      'real_product_entrypoint',
      'deterministic_kernel',
      'negative_failure_paths',
      'persistence',
      'real_readback',
      'no_ai_path',
    ],
    observation: {
      poll_supported: true,
      push_supported: true,
      checkpoint_durable: true,
      dedupe_durable: false,
      process_local_dedupe_only: true,
      reconciliation: true,
      webhook_authenticated: true,
      restart_replay_proven: true,
      observation_sources: ['mem.dedupe'],
    },
    entrypoints: ['POST /hooks'],
    facets: [],
    semantic_mutations: [],
  })
  assert.equal(result.observation_maturity, 'O2')
  assert.ok(result.hard_gate_failures.some((row) => /claimed observation O4 exceeds honest O2/.test(row)))
})

test('forbidden REALTIME freshness class is rejected', () => {
  const result = evaluateWeb2AppRecord({
    schema: WEB2APP_SCHEMA,
    semantic_contract: baseContract({ product_kind: 'INGEST', requires_ui: false, persistence_requirement: 'none', requires_persistence: false }),
    actual_witnesses: ['real_actor', 'deterministic_kernel', 'negative_failure_paths', 'no_ai_path', 'real_product_entrypoint'],
    observation: { poll_supported: true, freshness_class: 'REALTIME', observation_sources: ['poll'] },
    entrypoints: ['GET /poll'],
    facets: [],
    semantic_mutations: [],
  })
  assert.ok(result.errors.some((row) => /REALTIME is forbidden/.test(row)))
})

test('observation anti-patterns evaluate from evidence objects', () => {
  const pull = evaluateWeb2AppAntiPattern('PULL_ONLY_AS_REALTIME', {
    claimed_realtime: true,
    push_supported: false,
    poll_supported: true,
    note: 'poll billed as realtime',
  })
  assert.equal(pull.finding, 'FOUND')
  const command = evaluateWeb2AppAntiPattern('EVENT_AS_COMMAND', {
    external_event_becomes_command: true,
    authorized_execution: false,
    note: 'gold drop auto-buy',
  })
  assert.equal(command.finding, 'FOUND')
  const raw = evaluateWeb2AppAntiPattern('RAW_PROVIDER_PAYLOAD_AS_WORLD_MODEL', {
    raw_provider_payload_as_world_model: true,
    note: 'stored provider JSON',
  })
  assert.equal(raw.finding, 'FOUND')
  const cred = evaluateWeb2AppAntiPattern('CREDENTIAL_IN_AGENT_CONTEXT', {
    credential_plaintext_in_agent_context: true,
    note: 'secret in tool schema',
  })
  assert.equal(cred.finding, 'FOUND')
})

test('reactive fixture poll advances checkpoint and survives restart without duplicate', () => {
  const world = createReactiveNewsFixture()
  const before = world.canonical().length
  world.publishUpdate({
    id: 'art-2',
    title: 'BOJ raised rates',
    body: 'Policy change.',
    premium: false,
    event_id: 'evt-2',
    provider_timestamp: '2026-08-21T01:00:00Z',
  })
  const polled = world.poll()
  assert.equal(polled.changed, true)
  assert.equal(polled.canonical_added, 1)
  assert.equal(world.canonical().length, before + 1)
  const restarted = world.restart()
  const again = restarted.poll()
  assert.equal(again.changed, false)
  assert.equal(restarted.canonical().length, world.canonical().length)
  assert.equal(restarted.canonical().filter((row) => row.provider_event_id === 'evt-2').length, 1)
})

test('reactive fixture webhook is a signal; duplicate does not duplicate logical state', () => {
  const world = createReactiveNewsFixture()
  const { webhook } = world.publishUpdate({
    id: 'art-3',
    title: 'Gold moved',
    body: 'Typed later.',
    premium: false,
    event_id: 'evt-3',
    provider_timestamp: '2026-08-21T02:00:00Z',
  })
  const first = world.ingestWebhook({ event_id: webhook.event_id, signature_ok: true })
  assert.equal(first.outcome, 'reconciled')
  assert.equal(first.canonical_added, 1)
  const dup = world.ingestWebhook({ event_id: webhook.event_id, signature_ok: true })
  assert.equal(dup.outcome, 'duplicate_ignored')
  assert.equal(world.canonical().filter((row) => row.provider_event_id === 'evt-3').length, 1)
  assert.equal(world.agentQuery().events.some((row) => row.headline === 'Gold moved'), true)
})

test('premium entitlement and credential handle never leak secrets', () => {
  const world = createReactiveNewsFixture()
  world.publishUpdate({
    id: 'art-p',
    title: 'Premium filing',
    body: 'Subscriber-only.',
    premium: true,
    event_id: 'evt-p',
    provider_timestamp: '2026-08-21T03:00:00Z',
  })
  assert.throws(() => world.invokeWebapp('get_article', { credential_handle: 'cred_free', article_id: 'art-p' }), /entitlement missing/)
  const article = world.invokeWebapp('get_article', { credential_handle: 'cred_premium', article_id: 'art-p' })
  assert.equal(article.premium, true)
  const surfaces = JSON.stringify(world.leakSurfaces())
  assert.equal(surfaces.includes(PREMIUM_SECRET_CANARY), false)
  assert.equal(surfaces.includes(FREE_SECRET_CANARY), false)
  assert.equal(world.agentToolSchema().credential_handle, 'cred_premium')
  assert.equal('secret' in world.agentToolSchema(), false)
})

test('revoked credential cannot keep a stale premium session', () => {
  const world = createReactiveNewsFixture()
  world.revoke('cred_premium')
  assert.throws(() => world.invokeWebapp('search_news', { credential_handle: 'cred_premium' }), /revoked/)
  assert.equal(world.sourceHealth(), 'AUTH_EXPIRED')
})

test('unsigned fixture webhook is rejected and agent does not ingest via LLM', () => {
  const world = createReactiveNewsFixture()
  const { webhook } = world.publishUpdate({
    id: 'art-x',
    title: 'Unsigned',
    body: 'no',
    premium: false,
    event_id: 'evt-x',
    provider_timestamp: '2026-08-21T04:00:00Z',
  })
  assert.throws(() => world.ingestWebhook({ event_id: webhook.event_id, signature_ok: false }), /signature/)
  assert.equal(world.canonical().some((row) => row.provider_event_id === 'evt-x'), false)
})

test('product-kernel yfinance is W1 + O1 on-demand and money-free', () => {
  const result = evaluateWeb2AppRecord(loadFix('trading-markets.yfinance_adapter.json'))
  assert.equal(result.errors.length, 0, result.errors.join('\n'))
  assert.equal(result.web2app_level, 'W1')
  assert.equal(result.observation_maturity, 'O1')
  assert.equal(loadFix('trading-markets.yfinance_adapter.json').semantic_contract.moves_money, false)
  assert.equal(result.hard_gate_failures.length, 0)
  assert.notEqual(result.observation_maturity, 'O5')
})

test('CAP13 webhook_ingest kernel is W1 + O2 and cannot claim trusted O3', () => {
  const result = evaluateWeb2AppRecord(loadFix('infra.webhook_ingest.json'))
  assert.equal(result.errors.length, 0, result.errors.join('\n'))
  assert.equal(result.web2app_level, 'W1')
  assert.equal(result.observation_maturity, 'O2')
  assert.ok(result.anti_patterns_found.includes('WEBHOOK_WITHOUT_DEDUPE'))
  assert.equal(result.hard_gate_failures.length, 0)
  const overclaim = structuredClone(loadFix('infra.webhook_ingest.json'))
  overclaim.claimed_observation_maturity = 'O3'
  const failed = evaluateWeb2AppRecord(overclaim)
  assert.ok(failed.hard_gate_failures.some((row) => /claimed observation O3 exceeds honest O2/.test(row)))
})

