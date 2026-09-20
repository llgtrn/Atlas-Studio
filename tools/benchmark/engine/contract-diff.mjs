import { asArray, text } from './util.mjs'
import { sha256Hex } from './contract-hash.mjs'

function indexById(rows) {
  return new Map(rows.map((row) => [text(row.id) || text(row.cap), row]))
}

export function diffReleases(oldIr, newIr) {
  const oldFlows = indexById(oldIr.flows || [])
  const newFlows = indexById(newIr.flows || [])
  const added_flows = [...newFlows.keys()].filter((id) => !oldFlows.has(id))
  const removed_flows = [...oldFlows.keys()].filter((id) => !newFlows.has(id))
  const modified_flows = [...newFlows.keys()].filter((id) => {
    if (!oldFlows.has(id)) return false
    return sha256Hex(stripEval(oldFlows.get(id))) !== sha256Hex(stripEval(newFlows.get(id)))
  })

  const oldGates = new Set(collectGates(oldIr))
  const newGates = new Set(collectGates(newIr))
  const added_hard_gates = [...newGates].filter((g) => !oldGates.has(g))
  const removed_hard_gates = [...oldGates].filter((g) => !newGates.has(g))

  const affected_caps = new Set()
  for (const id of [...added_flows, ...removed_flows, ...modified_flows]) {
    const flow = newFlows.get(id) || oldFlows.get(id)
    if (!flow) continue
    affected_caps.add(text(flow.owner))
    for (const cap of asArray(flow.participants).map(text)) affected_caps.add(cap)
  }

  const revalidation_required = added_hard_gates.length + removed_hard_gates.length + modified_flows.length > 0
  return {
    added_flows,
    removed_flows,
    modified_flows,
    added_hard_gates,
    removed_hard_gates,
    affected_caps: [...affected_caps].filter(Boolean).sort(),
    revalidation_required,
  }
}

function stripEval(flow) {
  const copy = { ...flow }
  delete copy.derived_status
  delete copy.obligation_states
  delete copy.missing_obligations
  delete copy.verdict
  delete copy.hard_gate_failures
  delete copy.evidence_ids
  return copy
}

function collectGates(ir) {
  const gates = []
  for (const flow of ir.flows || []) gates.push(...asArray(flow.hard_gates).map(text))
  for (const env of ir.envelopes || []) gates.push(...asArray(env.hard_gates).map(text))
  return gates.filter(Boolean)
}
