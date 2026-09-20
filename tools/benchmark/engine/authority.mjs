// AUTHORITY_MODEL vs EXECUTION_GRANT_BRIDGE.
// An approval primitive is not a scoped, expiring, consumable execution grant.

import { AUTHORITY_MODEL_STATES, EXECUTION_GRANT_STATES } from './vocab.mjs'
import { hasText, isObject, text } from './util.mjs'

const WEAK_JUSTIFICATIONS = Object.freeze([
  'approval_system_exists',
  'evaluate_gate_exists',
  'sod_exists',
  'board_exists',
  'approval primitive exists',
])

export function validateAuthorityDistinction(record) {
  const errors = []
  if (!isObject(record)) return { ok: false, errors: ['authority record required'] }

  const model = text(record.authority_model)
  const grant = text(record.execution_grant_bridge)
  if (!AUTHORITY_MODEL_STATES.includes(model)) errors.push(`invalid authority_model ${model}`)
  if (!EXECUTION_GRANT_STATES.includes(grant)) errors.push(`invalid execution_grant_bridge ${grant}`)

  if (model === 'EXISTING_MODEL_SUFFICIENT' && grant === 'PROVEN') {
    const evidence = record.grant_evidence || {}
    const justifiedByApprovalOnly = WEAK_JUSTIFICATIONS.includes(text(record.justification)) || record.approval_implies_grant === true
    if (justifiedByApprovalOnly) {
      errors.push('approval primitive does not imply execution grant')
    }
    const consumable =
      hasText(evidence.grant_id) &&
      hasText(evidence.scope) &&
      hasText(evidence.expires_at) &&
      evidence.consumable === true &&
      hasText(evidence.executor_bound)
    if (!consumable) {
      errors.push('EXECUTION_GRANT_BRIDGE PROVEN requires scoped, expiring, consumable grant evidence')
    }
  }

  if (record.board_authority_invented === true) {
    errors.push('do not invent BoardAuthority as a duplicate primitive')
  }

  return { ok: errors.length === 0, errors }
}

export function frozenAuthorityFromLive(board) {
  return {
    authority_model: text(board?.authority_model) || 'EXISTING_MODEL_SUFFICIENT',
    execution_grant_bridge: text(board?.execution_grant_bridge) || 'NOT_YET_PROVEN',
    justification: 'evaluate_gate_and_approvals_encode_authority_model_only',
    approval_implies_grant: false,
    board_authority_invented: false,
    grant_evidence: board?.grant_evidence || null,
    note: 'EXISTING_MODEL_SUFFICIENT is not LIVE_EXECUTION_GRANT_SUFFICIENT',
  }
}
