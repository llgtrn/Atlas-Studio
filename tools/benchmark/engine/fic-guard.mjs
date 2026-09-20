// FIC remains the north-star denominator. Capability inventory cannot substitute for FIC.

import { applyLevelRules, computeFic, denominatorKeys, validateCensusShape } from '../fic-recompute-lib.mjs'
import { FIC_DENOMINATOR } from './vocab.mjs'
import { isObject, text } from './util.mjs'

export function guardFicDenominator(v1, census) {
  const errors = []
  const keys = denominatorKeys(v1)
  if (keys.length !== FIC_DENOMINATOR) {
    errors.push(`DENOMINATOR_DRIFT: v1 has ${keys.length} not ${FIC_DENOMINATOR}`)
  }
  const shape = validateCensusShape(census, v1)
  if (!shape.ok) errors.push(...shape.errors)
  const fic = computeFic(census)
  if (fic.denominator !== FIC_DENOMINATOR) {
    errors.push(`DENOMINATOR_DRIFT: computeFic denominator ${fic.denominator}`)
  }
  return { ok: errors.length === 0, errors, fic, keys }
}

export function rejectCapabilityCountAsFic(claim) {
  const errors = []
  if (!isObject(claim)) return { ok: false, errors: ['FIC claim must be an object'] }

  if (claim.as === 'FIC' && (claim.source === 'capability_verified_count' || claim.source === 'verified_capability_count')) {
    errors.push('capability count cannot substitute FIC')
  }
  if (Number(claim.denominator) === Number(claim.capability_inventory_total) && Number(claim.denominator) !== FIC_DENOMINATOR) {
    errors.push('capability count cannot substitute FIC')
  }
  if (claim.fic_percent_from === 'verified_capability_count') {
    errors.push('capability count cannot substitute FIC')
  }
  if (text(claim.label) === 'FIC' && Number(claim.covered) === Number(claim.verified_capability_count) && Number(claim.denominator) !== FIC_DENOMINATOR) {
    errors.push('capability count cannot substitute FIC')
  }
  if (claim.not_capability_inventory !== true && claim.require_explicit_guard === true) {
    errors.push('FIC snapshot must set not_capability_inventory=true')
  }
  return { ok: errors.length === 0, errors }
}

export function missingRecoveryBlocksL3(row) {
  const applied = applyLevelRules(row)
  const recoveryBlocked = (applied.l3_blockers || []).includes('recovery_missing') || applied.errors.some((e) => /recovery/i.test(e))
  const claimedL3plus = ['L3', 'L4', 'L5', 'L6'].includes(row.new_level)
  if (claimedL3plus && recoveryBlocked) {
    return { ok: false, errors: ['missing recovery prevents required FIC L3'], applied }
  }
  return { ok: true, errors: [], applied }
}
