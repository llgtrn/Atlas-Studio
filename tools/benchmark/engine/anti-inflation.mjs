// Anti-inflation / anti-Goodhart findings. Produce findings only — do not demote canonical rows.

import { ANTI_INFLATION_CODES, RUNTIME_KINDS } from './vocab.mjs'
import { asArray, hasText, isObject, text } from './util.mjs'

export function evaluateInflationClaim(claim) {
  const findings = []
  if (!isObject(claim)) return { findings: [], errors: ['inflation claim must be an object'] }

  const name = text(claim.name || claim.capability_key || claim.claim_id)
  const runtimeKind = text(claim.runtime_kind)
  if (runtimeKind && !RUNTIME_KINDS.includes(runtimeKind)) {
    return { findings: [], errors: [`${name}: invalid runtime_kind`] }
  }

  if (/external_write/i.test(name) && claim.external_effect !== true && !hasText(claim.effect)) {
    findings.push({ code: 'SIDE_EFFECT_CLASS_MISMATCH', claim: name, demote: false })
  }
  if (/create/i.test(name) && (claim.behavior === 'VALIDATE_ONLY' || runtimeKind === 'VALIDATE_ONLY')) {
    findings.push({ code: 'VALIDATE_ONLY_CREATE_CLAIM', claim: name, demote: false })
  }
  if (
    claim.verified === true &&
    (claim.caller_kind === 'CLI' || claim.caller_kind === 'TEST' || runtimeKind === 'TOOLING_ONLY') &&
    claim.required_runtime === 'PRODUCT_RUNTIME'
  ) {
    findings.push({ code: 'NON_PRODUCT_CALLER', claim: name, demote: false })
  }
  if (claim.external_effect === true && claim.receipt !== true && !hasText(claim.receipt_id)) {
    findings.push({ code: 'EXECUTION_WITHOUT_RECEIPT', claim: name, demote: false })
  }
  if (claim.durable_job === true && (claim.queue === 'IN_MEMORY' || claim.persistence_kind === 'IN_MEMORY')) {
    findings.push({ code: 'WORK_WITHOUT_DURABILITY', claim: name, demote: false })
  }

  return { findings, errors: [] }
}

export function evaluateInflationClaims(claims) {
  const findings = []
  const errors = []
  for (const claim of asArray(claims)) {
    const result = evaluateInflationClaim(claim)
    findings.push(...result.findings)
    errors.push(...result.errors)
  }
  const unknown = findings.filter((row) => !ANTI_INFLATION_CODES.includes(row.code))
  if (unknown.length) errors.push('unknown anti-inflation code')
  return { ok: errors.length === 0, errors, findings, demoted: [] }
}
