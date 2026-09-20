// First-vertical graph recompute. A contract is not runtime.

import { EDGE_STATUSES } from '../first-vertical-gap.mjs'
import { asArray, hasText, isObject, text } from './util.mjs'

export const ENGINE_EDGE_FIELDS = Object.freeze([
  'edge_id',
  'producer',
  'consumer',
  'runtime_owner',
  'current_status',
  'evidence',
  'blocking_reason',
  'required_witness',
])

export function normalizeEdge(edge) {
  const status = text(edge.current_status || edge.status)
  const evidence = edge.evidence
  const evidenceText = typeof evidence === 'string' ? evidence : JSON.stringify(evidence || '')
  return {
    edge_id: text(edge.edge_id || edge.id),
    producer: text(edge.producer || edge.from),
    consumer: text(edge.consumer || edge.to),
    runtime_owner: hasText(edge.runtime_owner) ? text(edge.runtime_owner) : 'UNSTATED',
    current_status: status,
    evidence: evidenceText,
    blocking_reason: hasText(edge.blocking_reason)
      ? text(edge.blocking_reason)
      : ['MISSING', 'BLOCKED', 'ISLAND'].includes(status)
        ? evidenceText || 'UNSTATED'
        : null,
    required_witness: hasText(edge.required_witness) ? text(edge.required_witness) : 'UNSTATED',
    fic_domains: asArray(edge.fic_domains),
    promoted_because: text(edge.promoted_because) || null,
  }
}

export function validateEngineEdge(edge) {
  const errors = []
  const row = isObject(edge) && edge.edge_id ? edge : normalizeEdge(edge || {})
  for (const field of ['edge_id', 'producer', 'consumer', 'current_status', 'evidence']) {
    if (!hasText(row[field])) errors.push(`${row.edge_id || '?'}: missing ${field}`)
  }
  if (row.current_status && !EDGE_STATUSES.includes(row.current_status)) {
    errors.push(`${row.edge_id}: invalid status ${row.current_status}`)
  }

  const evidenceIsContractOnly =
    /intent-contracts\/.+\.json/.test(row.evidence) &&
    !/crates\//.test(row.evidence) &&
    row.promoted_because === 'contract'

  if ((row.current_status === 'LIVE' || row.current_status === 'LIVE_BUT_UNVERIFIED') && evidenceIsContractOnly) {
    errors.push(`${row.edge_id}: CONTRACT_IS_NOT_RUNTIME`)
  }
  if (row.promoted_because === 'contract' && ['LIVE', 'LIVE_BUT_UNVERIFIED'].includes(row.current_status)) {
    errors.push(`${row.edge_id}: contract existence cannot promote runtime status`)
  }
  return { ok: errors.length === 0, errors, edge: row }
}

export function recomputeFirstVertical(doc) {
  const errors = []
  if (doc?.schema && doc.schema !== 'chronica.first_vertical_gap_graph.v1') {
    errors.push('invalid first-vertical schema')
  }
  const edges = asArray(doc?.edges).map((edge) => normalizeEdge(edge))
  const seen = new Set()
  const evaluated = []
  for (const edge of edges) {
    if (seen.has(edge.edge_id)) errors.push(`duplicate edge ${edge.edge_id}`)
    seen.add(edge.edge_id)
    const result = validateEngineEdge(edge)
    evaluated.push(result.edge)
    if (!result.ok) errors.push(...result.errors)
  }
  const counts = Object.fromEntries(EDGE_STATUSES.map((status) => [status, 0]))
  for (const edge of evaluated) {
    if (counts[edge.current_status] != null) counts[edge.current_status] += 1
  }
  return {
    ok: errors.length === 0,
    errors,
    doctrine: doc?.doctrine || 'docs/doctrines/183-roadmap-world-class-infrastructure-90-coverage.md',
    evidence_sha: doc?.evidence_sha || null,
    edges: evaluated,
    counts,
    total: evaluated.length,
  }
}
