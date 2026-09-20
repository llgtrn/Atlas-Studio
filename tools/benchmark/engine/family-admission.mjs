// 11-family admission. Reuses query-benchmark-ratio; does not invent a second family system.

import { computeBenchmarkRatio } from '../query-benchmark-ratio.mjs'
import { CLAIM_LEVELS, FAMILY_DENOMINATOR } from './vocab.mjs'
import { asArray, hasText, text } from './util.mjs'

const LEVEL_RANK = Object.freeze({
  Substrate: 1,
  NativeSlice: 2,
  ExternalClass: 3,
  ProductionReady: 4,
})

function normalizeLevel(value) {
  const raw = text(value)
  for (const level of CLAIM_LEVELS) {
    if (raw === level || raw.startsWith(`${level} `) || raw.startsWith(`${level}(`)) return level
  }
  return raw
}

export function evaluateFamilyAdmission(familyRow) {
  const errors = []
  const findings = []
  const level = normalizeLevel(familyRow.claim_level_reached)
  if (level && !CLAIM_LEVELS.includes(level)) {
    errors.push(`${familyRow.family_key}: unknown claim level ${familyRow.claim_level_reached}`)
  }

  if (level === 'ExternalClass' && familyRow.claim_ready !== true) {
    findings.push({ family: familyRow.family_key, code: 'CLAIM_LEVEL_WITHOUT_EVIDENCE', level })
  }
  if (level === 'ProductionReady' && text(familyRow.production_evidence) !== 'PROVEN') {
    findings.push({ family: familyRow.family_key, code: 'PRODUCTION_READY_WITHOUT_PEC', level })
  }
  if (familyRow.no_applicable_standard === true || text(familyRow.standard_applicability) === 'NO_APPLICABLE_STANDARD_WITH_EVIDENCE') {
    if (!hasText(familyRow.standard_research_evidence) && !hasText(familyRow.doctrine_permission)) {
      findings.push({
        family: familyRow.family_key,
        code: 'MISSING_STANDARD_RESEARCH',
        note: 'NO_APPLICABLE_STANDARD_WITH_EVIDENCE is not a substitute for missing research',
      })
    }
  }

  const portfolio = familyRow.world_class_gate?.portfolio
  if (portfolio && portfolio.meets_minimum !== true && asArray(portfolio.gaps).some((gap) => /standard/.test(String(gap)))) {
    if (text(familyRow.standard_applicability) === 'NO_APPLICABLE_STANDARD_WITH_EVIDENCE' && hasText(familyRow.standard_research_evidence)) {
      findings.push({ family: familyRow.family_key, code: 'NO_APPLICABLE_STANDARD_WITH_EVIDENCE' })
    }
  }

  return { ok: errors.length === 0, errors, findings, level, rank: LEVEL_RANK[level] || 0 }
}

export function admitFamilies(ratioResult) {
  const families = asArray(ratioResult?.families)
  const errors = []
  const findings = []
  if ((ratioResult?.ratios?.total_families || families.length) !== FAMILY_DENOMINATOR) {
    errors.push(`family denominator must remain ${FAMILY_DENOMINATOR}`)
  }
  for (const family of families) {
    const result = evaluateFamilyAdmission(family)
    errors.push(...result.errors)
    findings.push(...result.findings)
  }
  return { ok: errors.length === 0, errors, findings, family_count: families.length }
}

export function computeAndAdmit(options = {}) {
  const ratio = computeBenchmarkRatio(options)
  const admission = admitFamilies(ratio)
  return { ratio, admission }
}
