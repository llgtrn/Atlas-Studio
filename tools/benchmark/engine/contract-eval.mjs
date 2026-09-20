import { existsSync } from 'node:fs'
import { asArray, hasText, isObject, text } from './util.mjs'
import { FLOW_STATUSES, OBLIGATION_STATES } from './contract-vocab.mjs'
import { RUNTIME_KINDS } from './vocab.mjs'

function flagBlocked(flow) {
  return isObject(flow.blocked) && hasText(flow.blocked.class)
}

export function evaluateObligations(flow, evidenceByFlow) {
  const required = asArray(flow.obligations).map(text).filter(Boolean)
  const linked = asArray(evidenceByFlow.get(text(flow.id)))
  const states = {}
  const missing = []
  for (const po of required) {
    const hits = linked.filter((ev) => asArray(ev.proves).some((row) => text(row.obligation) === po && text(row.flow) === text(flow.id)))
    if (hits.some((ev) => text(ev.obligation_state) === 'FAIL' || ev.failed === true)) {
      states[po] = 'FAIL'
    } else if (hits.some((ev) => text(ev.kind) === 'benchmark_fixture')) {
      states[po] = hits.some((ev) => text(ev.runtime_kind) === 'PRODUCT_RUNTIME') ? 'PASS' : 'MISSING'
      if (states[po] === 'MISSING') missing.push(po)
    } else if (hits.length === 0) {
      states[po] = 'MISSING'
      missing.push(po)
    } else if (hits.some((ev) => text(ev.review) === 'MANUAL_REVIEW')) {
      states[po] = 'MANUAL_REVIEW'
    } else {
      states[po] = 'PASS'
    }
    if (!OBLIGATION_STATES.includes(states[po])) states[po] = 'MISSING'
  }
  return { required, states, missing }
}

export function hasProductRuntimeWitness(flow, evidenceRows) {
  return asArray(evidenceRows).some((ev) => {
    const kind = text(ev.runtime_kind) || text(ev.kind)
    return (
      ['PRODUCT_RUNTIME', 'INTERNAL_RUNTIME'].includes(text(ev.runtime_kind)) ||
      ['http_route', 'test', 'integration_test', 'source_symbol'].includes(text(ev.kind))
    ) && text(ev.kind) !== 'benchmark_fixture'
  })
}

export function deriveFlowStatus(flow, obligationEval, evidenceRows) {
  if (flagBlocked(flow)) return 'BLOCKED'
  if (hasText(flow.status) && flow.status === 'REVALIDATION_REQUIRED') return 'REVALIDATION_REQUIRED'
  const product = hasProductRuntimeWitness(flow, evidenceRows)
  const failed = Object.values(obligationEval.states).includes('FAIL')
  if (failed) return 'PARTIAL'
  if (product && obligationEval.missing.length === 0) {
    if (asArray(evidenceRows).some((ev) => text(ev.kind) === 'production_evidence')) return 'PRODUCTION_EVIDENCED'
    return 'VERIFIED'
  }
  if (product && obligationEval.missing.length) return 'RUNTIME_CONNECTED'
  if (!product && obligationEval.missing.length === obligationEval.required.length) {
    return text(flow.status) === 'NO_RUNTIME' ? 'NO_RUNTIME' : 'SPEC_ONLY'
  }
  if (!product) return obligationEval.missing.length ? 'SPEC_ONLY' : 'PARTIAL'
  return FLOW_STATUSES.includes(text(flow.status)) ? text(flow.status) : 'PARTIAL'
}

export function flowVerdict(obligationEval, hardGateFailures) {
  if (asArray(hardGateFailures).length) return 'FAIL'
  if (Object.values(obligationEval.states).includes('FAIL')) return 'FAIL'
  if (obligationEval.missing.length) return 'PARTIAL'
  if (Object.values(obligationEval.states).includes('MANUAL_REVIEW')) return 'PARTIAL'
  return 'PASS'
}

export function goldenVerdict(gbf, flowsById) {
  const statuses = asArray(gbf.flow_ids).map((id) => flowsById.get(text(id))?.derived_status || 'MISSING')
  if (statuses.includes('MISSING') || statuses.includes('BLOCKED')) return 'FAIL'
  if (statuses.every((status) => status === 'VERIFIED' || status === 'PRODUCTION_EVIDENCED')) return 'PASS'
  return 'PARTIAL'
}

export function indexEvidence(evidenceRecords) {
  const byFlow = new Map()
  for (const ev of asArray(evidenceRecords)) {
    for (const link of asArray(ev.proves)) {
      const flow = text(link.flow)
      if (!byFlow.has(flow)) byFlow.set(flow, [])
      byFlow.get(flow).push({ ...ev, obligation_state: ev.obligation_state })
    }
  }
  return byFlow
}

export function checkEvidencePaths(evidenceRecords, root) {
  const errors = []
  const broken = []
  for (const ev of asArray(evidenceRecords)) {
    const path = text(ev.path)
    if (!path) continue
    if (text(ev.kind) === 'production_evidence' && ev.pending === true) continue
    const abs = path.startsWith('/') ? path : `${root}/${path}`
    if (!existsSync(abs)) {
      errors.push(`FLOW_RUNTIME_WITNESS_BROKEN: ${ev.id} missing ${path}`)
      broken.push(ev.id)
    }
  }
  return { ok: errors.length === 0, errors, broken }
}

export function validateWitnessKinds(evidenceRecords) {
  const errors = []
  for (const ev of asArray(evidenceRecords)) {
    if (hasText(ev.runtime_kind) && !RUNTIME_KINDS.includes(text(ev.runtime_kind))) {
      errors.push(`${ev.id}: invalid runtime_kind ${ev.runtime_kind}`)
    }
  }
  return { ok: errors.length === 0, errors }
}
