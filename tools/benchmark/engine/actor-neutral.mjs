// Actor-neutral execution substrate check. Benchmark only — does not implement WorkRun.

import { PEER_ACTORS } from './vocab.mjs'
import { asArray, hasText, isObject, text } from './util.mjs'

export function evaluateActorNeutralSubstrate(substrate) {
  const errors = []
  const findings = []
  if (!isObject(substrate)) return { ok: false, errors: ['substrate evidence object required'], peer: false }

  const actors = substrate.actors && typeof substrate.actors === 'object' ? substrate.actors : {}
  const missingActors = PEER_ACTORS.filter((actor) => actors[actor] !== true && actors[actor] !== false)
  if (missingActors.length) {
    errors.push(`actor-neutral check must declare peer actors: ${missingActors.join(', ')}`)
  }

  const shared = substrate.shared_deterministic_command_run_boundary === true
  const tableFamily = text(substrate.table_family)
  const specialAgentPath = substrate.agent_special_path === true && substrate.human_uses_same_boundary !== true

  if (specialAgentPath) {
    findings.push('AGENT_SPECIAL_PATH_NOT_PEER')
  }
  if (substrate.durable_job_claimed === true && text(substrate.persistence_kind) === 'IN_MEMORY') {
    findings.push('WORK_WITHOUT_DURABILITY')
  }
  if (substrate.external_effect === true && substrate.receipt_persisted !== true) {
    findings.push('EXECUTION_WITHOUT_RECEIPT')
  }
  if (shared && !hasText(tableFamily) && asArray(substrate.evidence_paths).length === 0) {
    errors.push('shared command/run boundary requires table_family or evidence_paths')
  }

  const declaredPeers = PEER_ACTORS.filter((actor) => actors[actor] === true)
  const peer =
    shared &&
    !specialAgentPath &&
    declaredPeers.includes('human') &&
    declaredPeers.includes('agent') &&
    declaredPeers.length >= 3

  return {
    ok: errors.length === 0,
    errors,
    findings,
    peer,
    shared_deterministic_command_run_boundary: shared,
    declared_peers: declaredPeers,
    question: 'Does the business operation execute through a shared deterministic command/run boundary?',
  }
}
