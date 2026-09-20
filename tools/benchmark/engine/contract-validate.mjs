import { asArray, hasText, isObject, text } from './util.mjs'
import {
  ACTOR_CLASSES,
  CAP_IDS,
  CI_TIERS,
  CONTRACT_LEVELS,
  CONTRACT_SCHEMA,
  DEP_STATUSES,
  EVIDENCE_KINDS,
  FLOW_STATUSES,
  PROOF_OBLIGATIONS,
  PROSE_CLASSES,
  RECORD_TYPES,
  SEVERITIES,
  STANDARD_CATALOG,
  VALUE_LEVELS,
  WITNESS_CHAIN,
} from './contract-vocab.mjs'

const FLOW_ID = /^FLOW-(?:CAP\d{2}|CROSS)-[A-Z0-9-]+-\d{3}$/
const GBF_ID = /^GBF-\d{3}$/
const FAM_ID = /^FAM-[A-Z0-9-]+$/
const LAW_ID = /^LAW-[A-Z0-9-]+$/
const EV_ID = /^EV-[A-Z0-9-]+$/
const EDGE_ID = /^XCAP-[A-Z0-9-]+$/

function fail(errors, msg) {
  errors.push(msg)
}

function requireId(errors, record, pattern, label) {
  const id = text(record.id)
  if (!id) fail(errors, `${label}: id required`)
  else if (pattern && !pattern.test(id)) fail(errors, `${label} ${id}: invalid id shape`)
  return id
}

function checkCaps(errors, values, label) {
  for (const cap of asArray(values).map(text).filter(Boolean)) {
    if (!CAP_IDS.includes(cap)) fail(errors, `${label}: unknown CAP ${cap}`)
  }
}

function checkStandards(errors, standards, label) {
  if (standards == null) return
  if (!isObject(standards)) {
    fail(errors, `${label}: standards must be an object`)
    return
  }
  for (const [name, version] of Object.entries(standards)) {
    if (!STANDARD_CATALOG[name]) fail(errors, `${label}: unknown standard ${name}`)
    else if (hasText(version) && text(version) !== STANDARD_CATALOG[name].version) {
      fail(errors, `${label}: standard ${name} version ${version} != pinned ${STANDARD_CATALOG[name].version}`)
    }
  }
}

function checkObligations(errors, values, label) {
  for (const po of asArray(values).map(text).filter(Boolean)) {
    if (!PROOF_OBLIGATIONS.includes(po)) fail(errors, `${label}: unknown obligation ${po}`)
  }
}

export function validateContractRecord(record, ctx = {}) {
  const errors = []
  if (!isObject(record)) return { ok: false, errors: ['record must be an object'] }
  if (hasText(record.schema) && text(record.schema) !== CONTRACT_SCHEMA) {
    fail(errors, `schema must be ${CONTRACT_SCHEMA}`)
  }
  const type = text(record.type)
  if (!RECORD_TYPES.includes(type)) fail(errors, `unknown record type ${type || '(empty)'}`)

  if (type === 'global_law') {
    requireId(errors, record, LAW_ID, 'law')
    if (!hasText(record.statement)) fail(errors, `${record.id}: statement required`)
    if (hasText(record.level) && !CONTRACT_LEVELS.includes(text(record.level))) fail(errors, `${record.id}: bad level`)
    if (asArray(record.scope).length === 0) fail(errors, `${record.id}: scope required`)
  }

  if (type === 'family') {
    requireId(errors, record, FAM_ID, 'family')
    if (!hasText(record.owner) || !CAP_IDS.includes(text(record.owner))) fail(errors, `${record.id}: owner CAP required`)
    checkCaps(errors, record.participants, record.id)
    checkStandards(errors, record.standards, record.id)
  }

  if (type === 'cap_envelope') {
    const cap = text(record.cap)
    if (!CAP_IDS.includes(cap)) fail(errors, `envelope: unknown CAP ${cap || '(empty)'}`)
    if (!Array.isArray(record.owns) || record.owns.length === 0) fail(errors, `${cap}: owns required`)
    if (!Array.isArray(record.does_not_own) || record.does_not_own.length === 0) {
      fail(errors, `${cap}: does_not_own required`)
    }
    checkStandards(errors, record.standards, cap)
    checkCaps(errors, record.participants, cap)
  }

  if (type === 'flow') {
    requireId(errors, record, FLOW_ID, 'flow')
    const owner = text(record.owner)
    if (!CAP_IDS.includes(owner)) fail(errors, `${record.id}: exactly one owner CAP required`)
    if (text(record.owner) === 'everyone') fail(errors, `${record.id}: owner cannot be everyone`)
    checkCaps(errors, record.participants, record.id)
    if (asArray(record.participants).map(text).includes(owner) === false && asArray(record.participants).length) {
      /* owner need not be duplicated in participants */
    }
    const src = asArray(record.semantic_source)
    if (!hasText(record.objective) && src.length === 0 && !hasText(record.semantic_source)) {
      fail(errors, `${record.id}: no semantic source`)
    }
    checkStandards(errors, record.standards, record.id)
    checkObligations(errors, record.obligations, record.id)
    if (hasText(record.status) && !FLOW_STATUSES.includes(text(record.status))) fail(errors, `${record.id}: bad status`)
    if (hasText(record.value) && !VALUE_LEVELS.includes(text(record.value))) fail(errors, `${record.id}: bad value`)
    if (hasText(record.actor_class) && !ACTOR_CLASSES.includes(text(record.actor_class))) {
      fail(errors, `${record.id}: bad actor_class`)
    }
    if (record.blocked === true) fail(errors, `${record.id}: blocked must name class/dependency/revisit`)
    if (isObject(record.blocked)) {
      if (!hasText(record.blocked.class) || !hasText(record.blocked.revisit)) {
        fail(errors, `${record.id}: blocked requires class and revisit`)
      }
    }
    for (const dep of asArray(record.dependencies)) {
      if (isObject(dep) && hasText(dep.status) && !DEP_STATUSES.includes(text(dep.status))) {
        fail(errors, `${record.id}: bad dependency status`)
      }
    }
    const cross = asArray(record.participants).map(text).filter((cap) => cap && cap !== owner)
    if (text(record.id).startsWith('FLOW-CROSS-') && cross.length === 0) {
      fail(errors, `${record.id}: cross-CAP flow missing participant`)
    }
  }

  if (type === 'golden_flow') {
    requireId(errors, record, GBF_ID, 'golden')
    if (asArray(record.flow_ids).length === 0) fail(errors, `${record.id}: flow_ids required`)
    checkCaps(errors, record.participants, record.id)
  }

  if (type === 'evidence') {
    requireId(errors, record, EV_ID, 'evidence')
    if (!EVIDENCE_KINDS.includes(text(record.kind))) fail(errors, `${record.id}: bad evidence kind`)
    if (!hasText(record.path) && !hasText(record.symbol)) fail(errors, `${record.id}: path or symbol required`)
    if (text(record.kind) === 'benchmark_fixture' && text(record.runtime_kind) === 'PRODUCT_RUNTIME') {
      fail(errors, `${record.id}: FIXTURE_AS_RUNTIME`)
    }
    for (const link of asArray(record.proves)) {
      if (!isObject(link) || !hasText(link.flow) || !hasText(link.obligation)) {
        fail(errors, `${record.id}: proves[] requires flow and obligation`)
      } else if (!PROOF_OBLIGATIONS.includes(text(link.obligation))) {
        fail(errors, `${record.id}: unknown obligation ${link.obligation}`)
      }
    }
    if (isObject(record.witness)) {
      for (const node of Object.keys(record.witness)) {
        if (!WITNESS_CHAIN.includes(node)) fail(errors, `${record.id}: unknown witness node ${node}`)
      }
    }
  }

  if (type === 'cross_cap_edge') {
    requireId(errors, record, EDGE_ID, 'edge')
    if (!CAP_IDS.includes(text(record.producer)) || !CAP_IDS.includes(text(record.consumer))) {
      fail(errors, `${record.id}: producer and consumer CAPs required`)
    }
    if (!hasText(record.semantic_owner) || !CAP_IDS.includes(text(record.semantic_owner))) {
      fail(errors, `${record.id}: semantic_owner required`)
    }
    if (!hasText(record.operation)) fail(errors, `${record.id}: operation required`)
    if (!hasText(record.flow)) fail(errors, `${record.id}: flow required`)
  }

  if (type === 'prose_statement') {
    if (!hasText(record.id)) fail(errors, 'prose: id required')
    if (!PROSE_CLASSES.includes(text(record.class))) fail(errors, `${record.id}: bad prose class`)
    if (!hasText(record.standard)) fail(errors, `${record.id}: standard required`)
    if (!hasText(record.statement) && !hasText(record.ref)) fail(errors, `${record.id}: statement or ref required`)
  }

  if (type === 'anti_pattern_meta') {
    if (!hasText(record.id)) fail(errors, 'anti_pattern_meta: id required')
    if (hasText(record.severity) && !SEVERITIES.includes(text(record.severity))) fail(errors, `${record.id}: bad severity`)
    if (!hasText(record.detector) && text(record.review) !== 'MANUAL_REVIEW') {
      fail(errors, `${record.id}: detector or MANUAL_REVIEW required`)
    }
  }

  if (ctx.knownHardGates && asArray(record.hard_gates).length) {
    for (const gate of asArray(record.hard_gates).map(text)) {
      if (gate && !ctx.knownHardGates.has(gate)) fail(errors, `${record.id || record.cap}: unknown hard gate ${gate}`)
    }
  }

  for (const tier of asArray(record.ci_tiers).map(text).filter(Boolean)) {
    if (!CI_TIERS.includes(tier)) fail(errors, `${record.id || record.cap}: unknown CI tier ${tier}`)
  }

  return { ok: errors.length === 0, errors, type, id: text(record.id) || text(record.cap) }
}
