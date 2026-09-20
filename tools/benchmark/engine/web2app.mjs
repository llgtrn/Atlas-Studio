// WEB2APP evaluator. Detects semantic contradictions; does not mutate canonical capability rows.

import { asArray, hasText, isFiller, isObject, text } from './util.mjs'
import { evaluateWeb2AppAntiPatternSet } from './web2app-anti-patterns.mjs'
import {
  CANONICAL_STATUSES,
  FACET_STATES,
  JURISDICTION_MODELS,
  LEVEL_RANK,
  minLevel,
  NORMALIZATION_PLANES,
  OPERATION_KINDS,
  PREVIEW_SURFACE_RE,
  PRODUCT_CHAIN_SHAPES,
  PRODUCT_KINDS,
  rank,
  RUNTIME_STATUSES,
  SCORE_WEIGHTS,
  SEMANTIC_CONTRACT_FIELDS,
  SEMANTIC_CONTRACT_SCHEMA,
  VERDICTS,
  WEB2APP_LEVELS,
  WEB2APP_SCHEMA,
  WEB2APP_STANDARD_VERSION,
  WITNESS_NAMES,
  FIXED_POINT_SCHEMA,
  CLI_SURFACE_RE,
  OBSERVATION_LEVELS,
  observationRank,
} from './web2app-vocab.mjs'
import {
  assignObservationMaturity,
  deriveObservationAntiPatternEvidence,
  validateObservationFields,
} from './web2app-observation.mjs'

function requiredText(value) {
  if (value === true) return true
  if (value === false) return false
  const raw = text(value).toLowerCase()
  if (['no', 'none', 'false', 'not required', 'n/a', 'not_applicable', 'unneeded', 'not applicable'].includes(raw)) {
    return false
  }
  return hasText(value)
}

function flag(contract, key) {
  return requiredText(contract?.[key])
}

function requiresPersistence(contract) {
  if (contract.requires_persistence === true) return true
  if (contract.requires_persistence === false) return false
  const kind = text(contract.product_kind)
  if (kind === 'TRANSFORM' || kind === 'INGEST') return flag(contract, 'persistence_requirement')
  return flag(contract, 'persistence_requirement') || ['APP', 'DOCUMENT', 'WORKFLOW', 'SCHEDULER'].includes(kind)
}

function requiresUi(contract) {
  if (contract.requires_ui === true) return true
  if (contract.requires_ui === false) return false
  if (['CLI_OPERATOR', 'SCHEDULER', 'TRANSFORM', 'INGEST'].includes(text(contract.product_kind))) return false
  return ['APP', 'DOCUMENT'].includes(text(contract.product_kind))
}

function requiresExternalEffect(contract) {
  if (contract.requires_external_effect === true) return true
  if (contract.requires_external_effect === false) return false
  return flag(contract, 'external_effect_requirement')
}

function movesMoney(contract) {
  if (contract.moves_money === true) return true
  const impact = text(contract.money_impact).toLowerCase()
  return impact === 'true' || impact === 'yes' || impact === 'moves_money' || impact === 'money'
}

function requiresAi(contract) {
  return contract.requires_ai === true
}

function requiresTypedOperations(contract) {
  if (contract.requires_typed_operations === false) return false
  return !['TRANSFORM', 'INGEST'].includes(text(contract.product_kind))
}

function witnessSet(record) {
  const named = new Set(asArray(record.actual_witnesses).map(text).filter(Boolean))
  const matrix = isObject(record.witness_matrix) ? record.witness_matrix : {}
  for (const name of WITNESS_NAMES) {
    const cell = matrix[name]
    if (cell === true || (isObject(cell) && (cell.present === true || hasText(cell.evidence)))) named.add(name)
  }
  return named
}

function hasWitness(set, name) {
  return set.has(name)
}

export function validateSemanticContract(contract) {
  const errors = []
  if (!isObject(contract)) return { ok: false, errors: ['semantic_contract must be an object'] }
  if (hasText(contract.schema) && text(contract.schema) !== SEMANTIC_CONTRACT_SCHEMA) {
    errors.push(`semantic_contract.schema must be ${SEMANTIC_CONTRACT_SCHEMA}`)
  }
  for (const field of SEMANTIC_CONTRACT_FIELDS) {
    const value = contract[field]
    if (value == null) {
      errors.push(`semantic_contract.${field} is required`)
      continue
    }
    if (typeof value === 'string' && isFiller(value)) {
      const allowsNone = ['external_effect_requirement', 'money_impact', 'chronica_allowed_divergences', 'known_unimplemented_facets', 'idempotency_semantics', 'concurrency_semantics'].includes(field)
      if (!(allowsNone && ['none', 'n/a', 'not_applicable'].includes(text(value).toLowerCase()))) {
        errors.push(`semantic_contract.${field} must not be filler`)
      }
    }
  }
  if (hasText(contract.product_kind) && !PRODUCT_KINDS.includes(text(contract.product_kind))) {
    errors.push(`semantic_contract.product_kind is invalid: ${contract.product_kind}`)
  }
  if (hasText(contract.jurisdiction_model) && !JURISDICTION_MODELS.includes(text(contract.jurisdiction_model))) {
    errors.push(`semantic_contract.jurisdiction_model is invalid: ${contract.jurisdiction_model}`)
  }
  const ops = asArray(contract.typed_operations)
  if (requiresTypedOperations(contract) && ops.length === 0) {
    errors.push('semantic_contract.typed_operations is required for non-transform capabilities')
  }
  for (const [i, op] of ops.entries()) {
    if (!isObject(op) || !hasText(op.name)) errors.push(`typed_operations[${i}].name is required`)
    if (hasText(op.kind) && !OPERATION_KINDS.includes(text(op.kind))) {
      errors.push(`typed_operations[${i}].kind is invalid: ${op.kind}`)
    }
  }
  return { ok: errors.length === 0, errors }
}

function collectDeclaredAntiPatterns(record) {
  return asArray(record.anti_patterns).map((row) => (typeof row === 'string' ? { tag: row, evidence: record.anti_pattern_evidence?.[row] || { note: 'declared' } } : row))
}

function deriveAntiPatternEvidence(record, contract, actual) {
  const entrypoints = asArray(record.entrypoints).map((row) => (typeof row === 'string' ? row : text(row.path || row.kind)))
  const onlyPreview = entrypoints.length > 0 && entrypoints.every((p) => PREVIEW_SURFACE_RE.test(p))
  const derived = []
  const push = (tag, evidence) => derived.push({ tag, evidence: { ...evidence, note: evidence.note || tag } })
  derived.push(...deriveObservationAntiPatternEvidence(record))

  if (onlyPreview && requiresPersistence(contract)) {
    push('PREVIEW_AS_PRODUCT', {
      only_preview_surface: true,
      requires_durable_lifecycle: true,
      evidence_paths: entrypoints,
    })
  }
  if (requiresPersistence(contract) && !hasWitness(actual, 'persistence')) {
    push('WORK_WITHOUT_DURABILITY', {
      requires_persistence: true,
      persistence_present: false,
      persistence_kind: text(record.persistence_kind) || 'REQUEST_LOCAL',
      evidence_paths: asArray(record.persistence),
    })
  }
  if (requiresExternalEffect(contract) && !hasWitness(actual, 'effect_receipt')) {
    push('EXECUTION_WITHOUT_RECEIPT', {
      external_effect: true,
      receipt_persisted: false,
      evidence_paths: asArray(record.effects),
    })
  }
  const onlyCli =
    entrypoints.length > 0 &&
    entrypoints.every((p) => CLI_SURFACE_RE.test(p)) &&
    text(contract.product_kind) !== 'CLI_OPERATOR'
  if (onlyCli && record.caller_created_to_escape_island === true) {
    push('CALLER_FARMING', {
      caller_exists: true,
      intended_product_outcome: false,
      caller_created_to_escape_island: true,
      evidence_paths: entrypoints,
    })
  }
  const facets = asArray(record.facets)
  for (const facet of facets) {
    if (['LABEL_ONLY', 'STORED_NOT_ENFORCED', 'SCHEMA_ONLY'].includes(text(facet.semantic_state))) {
      push('LABEL_AS_ENFORCEMENT', {
        token_stored: true,
        behavior_enforced: false,
        note: `facet ${facet.name}`,
        evidence_paths: asArray(facet.evidence),
      })
    }
  }
  return derived
}

export function assignWeb2AppLevel(record) {
  const contract = record.semantic_contract || {}
  const actual = witnessSet(record)
  const caps = []
  const missing = []

  let level = 'W0'
  if (hasWitness(actual, 'deterministic_kernel') && hasWitness(actual, 'negative_failure_paths')) level = 'W1'
  if (hasWitness(actual, 'real_product_entrypoint')) level = rank(level) >= 2 ? level : 'W2'
  if (
    (!requiresPersistence(contract) || hasWitness(actual, 'persistence')) &&
    hasWitness(actual, 'real_product_entrypoint') &&
    (!requiresPersistence(contract) || hasWitness(actual, 'real_readback'))
  ) {
    const effectOk = !requiresExternalEffect(contract) || hasWitness(actual, 'effect_receipt')
    const durableOk = requiresPersistence(contract) && hasWitness(actual, 'persistence') && hasWitness(actual, 'real_readback')
    if (rank(level) < 3 && (effectOk || durableOk)) {
      level = 'W3'
    }
  }
  if (rank(level) >= 3 && (!requiresUi(contract) || hasWitness(actual, 'real_ui_consumer'))) {
    if (requiresUi(contract) && hasWitness(actual, 'real_ui_consumer')) level = 'W4'
    if (!requiresUi(contract) && rank(level) >= 3) {
      // Backend-complete scheduler/CLI/operator caps may sit at W3; W4 is app-consumer.
    }
  }
  if (hasWitness(actual, 'production_evidence') && rank(level) >= 3) level = 'W5'

  if (!hasWitness(actual, 'real_product_entrypoint')) {
    caps.push('no shipped caller → max W1')
    level = minLevel(level, 'W1')
    missing.push('real_product_entrypoint')
  }
  if (requiresPersistence(contract) && !hasWitness(actual, 'persistence')) {
    caps.push('required persistence absent → max W2')
    level = minLevel(level, 'W2')
    missing.push('persistence')
  }
  if (requiresExternalEffect(contract) && !hasWitness(actual, 'effect_receipt')) {
    caps.push('external effect without receipt → cannot claim effect-complete')
    if (requiresPersistence(contract) && hasWitness(actual, 'persistence') && hasWitness(actual, 'real_readback')) {
      // durable non-effect path may remain W3
    } else {
      level = minLevel(level, 'W2')
    }
    missing.push('effect_receipt')
  }
  if (requiresUi(contract) && !hasWitness(actual, 'real_ui_consumer')) {
    caps.push('user-facing capability with no real product consumer → max W3')
    level = minLevel(level, 'W3')
    missing.push('real_ui_consumer')
  }
  if (!hasWitness(actual, 'production_evidence')) {
    level = minLevel(level, 'W4')
  }
  if (!hasWitness(actual, 'deterministic_kernel') || !hasWitness(actual, 'negative_failure_paths')) {
    if (!hasWitness(actual, 'deterministic_kernel')) missing.push('deterministic_kernel')
    if (!hasWitness(actual, 'negative_failure_paths')) missing.push('negative_failure_paths')
    if (!hasWitness(actual, 'deterministic_kernel')) level = 'W0'
  }

  const claimed = text(record.claimed_web2app_level || record.web2app_level)
  return { level, claimed, caps, missing, actual: [...actual] }
}

function requiredWitnessesFor(contract) {
  const required = ['real_actor', 'deterministic_kernel', 'negative_failure_paths']
  required.push('real_product_entrypoint')
  if (contract.tenant_requirement && flag(contract, 'tenant_requirement')) required.push('tenant_boundary')
  if (contract.authority_requirement && flag(contract, 'authority_requirement')) required.push('auth_boundary')
  if (requiresPersistence(contract)) {
    required.push('persistence', 'real_readback')
  }
  if (requiresExternalEffect(contract)) required.push('external_effect', 'effect_receipt')
  if (requiresUi(contract)) required.push('real_ui_consumer')
  if (!requiresAi(contract)) required.push('no_ai_path')
  if (movesMoney(contract)) required.push('money_authority')
  if (requiresTypedOperations(contract)) required.push('typed_operations')
  return [...new Set(required)]
}

function scoreRecord(record, assignment, contract, actual, antiFound, hardFails) {
  const clamp = (n, max) => Math.max(0, Math.min(max, n))
  const semanticMutations = asArray(record.semantic_mutations)
  const unreviewed = semanticMutations.filter((row) => row.independent_review !== true)
  let semantic = 8
  if (hasText(contract.donor_semantics)) semantic += 4
  if (contract.chronica_allowed_divergences != null) semantic += 4
  if (unreviewed.length === 0) semantic += 4
  else semantic = 0
  if (antiFound.includes('DONOR_NAME_WITHOUT_DONOR_SEMANTICS')) semantic = Math.min(semantic, 8)

  let runtime = 0
  if (hasWitness(actual, 'deterministic_kernel')) runtime += 5
  if (hasWitness(actual, 'real_product_entrypoint')) runtime += 8
  else runtime += 2
  if (hasWitness(actual, 'real_readback') || !requiresPersistence(contract)) runtime += 2
  runtime = clamp(runtime, 15)

  let durability = 0
  if (!requiresPersistence(contract) && !requiresExternalEffect(contract)) durability = 15
  else {
    if (!requiresPersistence(contract) || hasWitness(actual, 'persistence')) durability += 8
    if (!requiresExternalEffect(contract) || hasWitness(actual, 'effect_receipt')) durability += 7
  }

  let auth = 0
  if (!flag(contract, 'tenant_requirement') || hasWitness(actual, 'tenant_boundary')) auth += 5
  if (!flag(contract, 'authority_requirement') || hasWitness(actual, 'auth_boundary')) auth += 5
  if (movesMoney(contract) && !hasWitness(actual, 'money_authority')) auth = 0

  const failure = hasWitness(actual, 'negative_failure_paths') ? 10 : 0

  let idem = 0
  const retryable = flag(contract, 'idempotency_semantics') || text(contract.idempotency_semantics).length > 0
  const concurrent = flag(contract, 'concurrency_semantics') || text(contract.concurrency_semantics).length > 0
  if (!retryable || hasWitness(actual, 'idempotency')) idem += 5
  if (!concurrent || hasWitness(actual, 'concurrency')) idem += 5

  let consumer = 0
  if (!requiresUi(contract)) consumer = hasWitness(actual, 'real_product_entrypoint') ? 10 : 3
  else consumer = hasWitness(actual, 'real_ui_consumer') ? 10 : 2

  let evidence = 0
  if (hasWitness(actual, 'negative_failure_paths')) evidence += 5
  if (hasWitness(actual, 'production_evidence')) evidence += 5
  else evidence += 3

  const dimensions = {
    semantic_fidelity: clamp(semantic, SCORE_WEIGHTS.semantic_fidelity),
    runtime_reachability: clamp(runtime, SCORE_WEIGHTS.runtime_reachability),
    durability_effect: clamp(durability, SCORE_WEIGHTS.durability_effect),
    auth_tenancy: clamp(auth, SCORE_WEIGHTS.auth_tenancy),
    failure_semantics: clamp(failure, SCORE_WEIGHTS.failure_semantics),
    idempotency_concurrency: clamp(idem, SCORE_WEIGHTS.idempotency_concurrency),
    product_consumer: clamp(consumer, SCORE_WEIGHTS.product_consumer),
    evidence_quality: clamp(evidence, SCORE_WEIGHTS.evidence_quality),
  }
  const total = Object.values(dimensions).reduce((a, b) => a + b, 0)
  return {
    dimensions,
    total,
    authoritative: hardFails.length === 0,
    hard_caps_applied: assignment.caps,
  }
}

function highestLivePlane(assignment, contract, actual) {
  if (hasWitness(actual, 'effect_receipt') || hasWitness(actual, 'production_evidence')) return 'P5'
  if (hasWitness(actual, 'typed_operations') && (hasWitness(actual, 'auth_boundary') || !flag(contract, 'authority_requirement'))) {
    if (rank(assignment.level) >= 3) return 'P4'
  }
  if (hasWitness(actual, 'persistence')) return 'P3'
  if (hasWitness(actual, 'deterministic_kernel')) return 'P2'
  if (hasWitness(actual, 'real_product_entrypoint')) return 'P1'
  return 'P0'
}

export function evaluateWeb2AppRecord(record) {
  const errors = []
  const hardFails = []
  const contradictions = []
  if (!isObject(record)) return { ok: false, errors: ['record must be an object'], hard_gate_failures: [], verdict: 'FAIL' }
  if (hasText(record.schema) && text(record.schema) !== WEB2APP_SCHEMA) {
    errors.push(`schema must be ${WEB2APP_SCHEMA}`)
  }

  const contract = record.semantic_contract
  const contractCheck = validateSemanticContract(contract)
  if (!contractCheck.ok) errors.push(...contractCheck.errors)

  const canonicalStatus = text(record.canonical_status)
  if (canonicalStatus && !CANONICAL_STATUSES.includes(canonicalStatus)) {
    errors.push(`canonical_status is invalid: ${canonicalStatus}`)
  }
  const runtimeStatus = text(record.runtime_status)
  if (runtimeStatus && !RUNTIME_STATUSES.includes(runtimeStatus)) {
    errors.push(`runtime_status is invalid: ${runtimeStatus}`)
  }
  if (hasText(record.product_chain_shape) && !PRODUCT_CHAIN_SHAPES.includes(text(record.product_chain_shape))) {
    errors.push(`product_chain_shape is invalid: ${record.product_chain_shape}`)
  }
  if (hasText(record.normalization_plane) && !NORMALIZATION_PLANES.includes(text(record.normalization_plane))) {
    errors.push(`normalization_plane is invalid: ${record.normalization_plane}`)
  }
  const observationCheck = validateObservationFields(record)
  if (!observationCheck.ok) errors.push(...observationCheck.errors)

  const facets = asArray(record.facets)
  for (const [i, facet] of facets.entries()) {
    if (!isObject(facet) || !hasText(facet.name)) errors.push(`facets[${i}].name is required`)
    const state = text(facet.semantic_state)
    if (state && !FACET_STATES.includes(state)) errors.push(`facets[${i}].semantic_state is invalid: ${state}`)
    if (state === 'LIVE' && (facet.label_only === true || facet.stored_not_enforced === true)) {
      hardFails.push('LABEL_ONLY facet claimed LIVE')
    }
    if (['LABEL_ONLY', 'STORED_NOT_ENFORCED', 'SCHEMA_ONLY', 'CONFIG_ONLY'].includes(state) && text(facet.claimed_as) === 'LIVE') {
      hardFails.push('LABEL_ONLY facet claimed LIVE')
    }
  }

  const assignment = assignWeb2AppLevel(record)
  const observation = assignObservationMaturity(record)
  const actual = witnessSet(record)
  const required = isObject(contract) ? requiredWitnessesFor(contract) : []
  const claimedLevel = text(record.claimed_web2app_level || record.web2app_level)
  if (claimedLevel && !WEB2APP_LEVELS.includes(claimedLevel)) errors.push(`web2app_level is invalid: ${claimedLevel}`)
  if (claimedLevel && rank(claimedLevel) > rank(assignment.level)) {
    hardFails.push(`claimed ${claimedLevel} exceeds honest ${assignment.level}`)
  }
  if (observation.claimed && !OBSERVATION_LEVELS.includes(observation.claimed)) {
    errors.push(`observation_maturity is invalid: ${observation.claimed}`)
  }
  if (observation.claimed && observationRank(observation.claimed) > observationRank(observation.level)) {
    hardFails.push(`claimed observation ${observation.claimed} exceeds honest ${observation.level}`)
  }
  if (['W3', 'W4', 'W5'].includes(claimedLevel) && !hasWitness(actual, 'real_product_entrypoint')) {
    hardFails.push('W3/W4 claim with no real runtime caller')
  }

  if (canonicalStatus === 'verified' && isObject(contract) && requiresPersistence(contract) && !hasWitness(actual, 'persistence')) {
    contradictions.push('verified durable capability with no persistence witness')
  }
  if (requiresUi(contract) && hasWitness(actual, 'real_ui_consumer') && !hasWitness(actual, 'real_product_entrypoint') && !hasWitness(actual, 'deterministic_kernel')) {
    hardFails.push('UI-only capability claiming runtime')
  }

  const entrypoints = asArray(record.entrypoints).map((row) => (typeof row === 'string' ? row : text(row.path || row.kind)))
  const onlyPreview = entrypoints.length > 0 && entrypoints.every((p) => PREVIEW_SURFACE_RE.test(p))
  if (onlyPreview && requiresPersistence(contract) && (claimedLevel === 'W3' || claimedLevel === 'W4' || claimedLevel === 'W5' || text(record.verdict) === 'PASS')) {
    hardFails.push('preview-only route claiming durable app')
  }

  const mutations = asArray(record.semantic_mutations)
  for (const mutation of mutations) {
    if (mutation.independent_review !== true) hardFails.push('GOALPOST_MUTATION')
  }

  if (requiresExternalEffect(contract) && !hasWitness(actual, 'effect_receipt')) {
    const effectFacetLive = asArray(record.facets).some(
      (facet) => /effect|dispatch|execute|provider/i.test(text(facet.name)) && text(facet.semantic_state) === 'LIVE',
    )
    if (effectFacetLive || claimedLevel === 'W4' || claimedLevel === 'W5' || text(record.verdict) === 'PASS') {
      hardFails.push('external-effect completion with no effect receipt')
    }
  }
  if (movesMoney(contract) && !hasWitness(actual, 'money_authority')) {
    hardFails.push('money capability without authority witness')
  }

  const derived = isObject(contract) ? deriveAntiPatternEvidence(record, contract, actual) : []
  const declared = collectDeclaredAntiPatterns(record)
  const anti = evaluateWeb2AppAntiPatternSet([...declared, ...derived])
  if (!anti.ok) errors.push(...anti.errors)
  if (anti.found.includes('GOALPOST_MUTATION')) hardFails.push('GOALPOST_MUTATION')
  if (anti.found.includes('LLM_SELF_AUTHORIZATION') || anti.found.includes('AGENT_AS_KERNEL')) {
    hardFails.push(anti.found.includes('LLM_SELF_AUTHORIZATION') ? 'LLM_SELF_AUTHORIZATION' : 'AGENT_AS_KERNEL')
  }

  const uniqueHard = [...new Set(hardFails)]
  if (uniqueHard.includes('GOALPOST_MUTATION')) {
    // FAIL regardless of score
  }

  const plane = highestLivePlane(assignment, contract || {}, actual)
  const score = scoreRecord(record, assignment, contract || {}, actual, anti.found, uniqueHard)

  let verdict = 'PASS'
  if (uniqueHard.length) verdict = uniqueHard.some((x) => /MONEY|LLM_SELF|AGENT_AS_KERNEL/.test(x)) ? 'FAIL' : 'FAIL'
  else if (anti.found.length || contradictions.length || assignment.missing.length || facets.some((f) => ['LABEL_ONLY', 'ABSENT', 'STORED_NOT_ENFORCED', 'PARTIAL'].includes(text(f.semantic_state)))) {
    verdict = 'PARTIAL'
  }
  if (text(record.runtime_status) === 'MONEY_BLOCKED' || text(record.runtime_status) === 'SECURITY_BLOCKED') verdict = 'BLOCKED'
  if (VERDICTS.includes(text(record.forced_verdict))) {
    // tests may set expected; evaluator still computes honest verdict
  }

  const missingRequiredForLevel = required.filter((name) => {
    if (rank(assignment.level) < 1) return false
    if (name === 'real_product_entrypoint' && rank(assignment.level) < 2) return false
    if (name === 'real_ui_consumer' && rank(assignment.level) < 4) return false
    if (name === 'production_evidence') return false
    return !hasWitness(actual, name)
  })

  const result = {
    ok: errors.length === 0 && uniqueHard.length === 0,
    schema: WEB2APP_SCHEMA,
    standard_version: WEB2APP_STANDARD_VERSION,
    capability_key: contract?.capability_key || record.capability_key,
    canonical_status: canonicalStatus || null,
    web2app_level: assignment.level,
    claimed_web2app_level: claimedLevel || null,
    observation_maturity: observation.level,
    claimed_observation_maturity: observation.claimed,
    observation_caps: observation.caps,
    observation_sources: observation.sources,
    freshness_class: observation.freshness_class,
    source_health: observation.source_health,
    staleness_visible: observation.staleness_visible,
    runtime_status: runtimeStatus || null,
    normalization_plane: plane,
    product_chain_shape: text(record.product_chain_shape) || null,
    hard_gate_failures: uniqueHard,
    contradictions,
    missing_witnesses: [...new Set([...assignment.missing, ...missingRequiredForLevel])],
    required_witnesses: required,
    actual_witnesses: [...actual],
    anti_patterns: anti.findings,
    anti_patterns_found: anti.found,
    score,
    verdict,
    cannot_weaken_meaning_because_substrate_missing: true,
    errors,
  }
  return result
}

export function validateFixedPointReport(report) {
  const errors = []
  const hardFails = []
  if (!isObject(report)) return { ok: false, errors: ['fixed-point report must be an object'], verdict: 'FIXED_POINT_INVALID' }
  if (hasText(report.schema) && text(report.schema) !== FIXED_POINT_SCHEMA) {
    errors.push(`schema must be ${FIXED_POINT_SCHEMA}`)
  }
  if (report.pass0_contracts_frozen !== true) errors.push('PASS 0 semantic contracts must be frozen')

  const pass1 = report.pass1 || {}
  const pass2 = report.pass2 || {}
  for (const [name, pass] of [['pass1', pass1], ['pass2', pass2]]) {
    if (pass.actionable !== 0) errors.push(`${name} ACTIONABLE must be 0`)
    if (pass.unclassified !== 0) errors.push(`${name} UNCLASSIFIED must be 0`)
  }

  const d1 = pass1.semantic_owner_denominator
  const d2 = pass2.semantic_owner_denominator
  if (d1 != null && d2 != null && d1 !== d2 && report.independent_denominator_justification !== true) {
    hardFails.push('DENOMINATOR_SHRINKING')
  }
  if ((report.semantic_contract_mutations || 0) !== 0 && report.independent_semantic_review !== true) {
    hardFails.push('GOALPOST_MUTATION')
  }
  if ((report.unjustified_ownership_changes || 0) !== 0) hardFails.push('OWNERSHIP_LAUNDERING')

  const valid =
    errors.length === 0 &&
    hardFails.length === 0 &&
    report.pass0_contracts_frozen === true &&
    pass1.actionable === 0 &&
    pass2.actionable === 0 &&
    pass1.unclassified === 0 &&
    pass2.unclassified === 0

  return {
    ok: valid,
    errors,
    hard_gate_failures: hardFails,
    verdict: valid ? 'FIXED_POINT_VALID' : 'FIXED_POINT_INVALID',
    code: valid ? 'CAPx_FIXED_POINT_REACHED' : hardFails.includes('GOALPOST_MUTATION') ? 'CAPx_SEMANTIC_REVIEW_REQUIRED' : 'CAPx_FIXED_POINT_INVALID',
  }
}

export function evaluateFixtureFile(record) {
  return evaluateWeb2AppRecord(record)
}

export { LEVEL_RANK, assignObservationMaturity }
