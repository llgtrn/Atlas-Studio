import { asArray, text } from './util.mjs'
import { rankFlow } from './contract-packet.mjs'
import { CAP_IDS } from './contract-vocab.mjs'

export function nextActionable(ir, cap, releaseId) {
  if (cap && !CAP_IDS.includes(cap)) return { ok: false, errors: [`unknown CAP ${cap}`] }
  const pool = ir.flows.filter((flow) => !cap || text(flow.owner) === cap)
  const ranked = [...pool]
    .map((flow) => ({ flow, score: rankFlow(flow) }))
    .filter((row) => row.score > 0 && row.flow.derived_status !== 'BLOCKED')
    .sort((a, b) => b.score - a.score || a.flow.id.localeCompare(b.flow.id))
  const top = ranked[0]
  if (!top) {
    return {
      ok: true,
      cap: cap || null,
      flow: null,
      note: 'no actionable flows',
      benchmark_release: releaseId || null,
    }
  }
  const flow = top.flow
  const deps = asArray(flow.dependencies).map((dep) => ({
    cap: dep.cap || dep.id,
    status: dep.status || 'UNKNOWN',
  }))
  return {
    ok: true,
    cap: text(flow.owner),
    flow: flow.id,
    current: flow.derived_status,
    missing: asArray(flow.missing_obligations),
    dependencies: deps,
    blockers: flow.blocked || 'NONE',
    value: flow.value || 'MEDIUM',
    score: top.score,
    benchmark_release: releaseId || null,
    golden: asArray(flow.golden),
  }
}
