// Contract relationship model. Detects competing WorkRequest/WorkRun/Attempt families.
// Does not rewrite product schemas.

import { CONTRACT_RELATIONS } from './vocab.mjs'
import { asArray, isObject, text } from './util.mjs'

const LINKED = new Set(['PARENT', 'CHILD', 'SUPERSEDES', 'REFINES', 'CONSUMES'])

const SUBSTRATE_FIELDS = Object.freeze(['WorkRequest', 'WorkRun', 'Attempt', 'job_table', 'receipt_model'])

export function extractSubstrateMarkers(contract) {
  const state = String(contract?.required_state || '')
  return {
    WorkRequest: /WorkRequest/.test(state),
    WorkRun: /WorkRun/.test(state),
    Attempt: /\bAttempt\b/.test(state),
    job_table: /job table|jobs schema|job_queue/i.test(state),
    receipt_model: /effect_receipt|receipt model|ProviderReceipt/i.test(state),
  }
}

export function isExecutionSubstrateDefiner(markers) {
  return (markers.WorkRequest && markers.WorkRun) || markers.job_table
}

export function validateRelationships(relationships) {
  const errors = []
  const rows = asArray(relationships)
  for (const row of rows) {
    if (!isObject(row)) {
      errors.push('relationship must be an object')
      continue
    }
    if (!text(row.from) || !text(row.to)) errors.push('relationship requires from and to')
    if (!CONTRACT_RELATIONS.includes(text(row.relation))) {
      errors.push(`${row.from}->${row.to}: invalid relation ${row.relation}`)
    }
    if (text(row.from) === text(row.to)) errors.push(`${row.from}: relationship cannot be reflexive`)
  }
  return { ok: errors.length === 0, errors }
}

function linked(relationships, a, b) {
  return asArray(relationships).some((row) => {
    const relation = text(row.relation)
    if (!LINKED.has(relation) && relation !== 'PARENT') return false
    const from = text(row.from)
    const to = text(row.to)
    return (from === a && to === b) || (from === b && to === a)
  })
}

export function detectDuplicateExecutionSubstrate(contracts, relationships = []) {
  const errors = []
  const relCheck = validateRelationships(relationships)
  if (!relCheck.ok) errors.push(...relCheck.errors)

  const definers = asArray(contracts)
    .map((contract) => ({
      id: text(contract.intent_id),
      markers: extractSubstrateMarkers(contract),
      family: text(contract.execution_intent_family || contract.intent_family || 'execution-spine'),
    }))
    .filter((row) => row.id && isExecutionSubstrateDefiner(row.markers))

  for (let i = 0; i < definers.length; i += 1) {
    for (let j = i + 1; j < definers.length; j += 1) {
      const left = definers[i]
      const right = definers[j]
      if (left.family !== right.family) continue
      const shared = SUBSTRATE_FIELDS.filter((field) => left.markers[field] && right.markers[field])
      if (shared.length === 0) continue
      if (linked(relationships, left.id, right.id)) continue
      const independent = asArray(relationships).some(
        (row) =>
          text(row.relation) === 'INDEPENDENT' &&
          ((text(row.from) === left.id && text(row.to) === right.id) ||
            (text(row.from) === right.id && text(row.to) === left.id)),
      )
      if (independent || !linked(relationships, left.id, right.id)) {
        errors.push(
          `DUPLICATE_EXECUTION_SUBSTRATE: ${left.id} and ${right.id} both define ${shared.join(', ')} without PARENT/CHILD/SUPERSEDES/REFINES/CONSUMES`,
        )
      }
    }
  }

  const conflicts = asArray(relationships).filter((row) => text(row.relation) === 'CONFLICTS')
  for (const row of conflicts) {
    errors.push(`CONFLICTS: ${row.from} vs ${row.to}`)
  }

  return { ok: errors.length === 0, errors, definers }
}

export function provisionalSpineRelationship() {
  return [
    {
      from: 'CORE-EXECUTION-SPINE-001',
      to: 'AI-DURABLE-001',
      relation: 'PARENT',
      note: 'CORE-EXECUTION-SPINE-001 is the parent/generalization. AI-DURABLE-001 consumes the same table family.',
    },
    {
      from: 'AI-DURABLE-001',
      to: 'CORE-EXECUTION-SPINE-001',
      relation: 'CONSUMES',
    },
  ]
}

export function relationshipsFromLiveContracts(contracts) {
  const rows = []
  for (const contract of asArray(contracts)) {
    const id = text(contract.intent_id)
    if (id === 'CORE-EXECUTION-SPINE-001' && text(contract.relationship_to_ai_durable) === 'PARENT_OR_SUPERSEDE') {
      rows.push({ from: id, to: 'AI-DURABLE-001', relation: 'PARENT' })
    }
    if (id === 'AI-DURABLE-001' && (text(contract.role) === 'CONSUMER_USE_CASE' || text(contract.parent_contract) === 'CORE-EXECUTION-SPINE-001')) {
      rows.push({ from: id, to: 'CORE-EXECUTION-SPINE-001', relation: 'CONSUMES' })
    }
  }
  return rows.length ? rows : provisionalSpineRelationship()
}
