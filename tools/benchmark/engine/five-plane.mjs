// Five-plane evaluation as data. Markdown is derived later. No numeric plane scores.

import { CONFIDENCE, LIVE_PLANE_KEY_TO_ENGINE, PLANE_ASSESSMENTS, PLANES } from './vocab.mjs'
import { asArray, hasText, isObject, text, unique } from './util.mjs'
import { validateNoAiTaxonomy } from './no-ai.mjs'

export function emptyPlaneRecord(domain, plane) {
  return {
    domain,
    plane,
    assessment: 'MISSING',
    evidence_paths: [],
    runtime_entrypoints: [],
    state_owners: [],
    effect_owners: [],
    receipts: [],
    recovery_paths: [],
    no_ai_class: null,
    anti_patterns: [],
    good_paths: [],
    confidence: 'LOW',
  }
}

export function validatePlaneRecord(record) {
  const errors = []
  if (!isObject(record)) return { ok: false, errors: ['plane record must be an object'] }
  if (!hasText(record.domain)) errors.push('domain required')
  if (!PLANES.includes(text(record.plane))) errors.push(`${record.domain || '?'}: invalid plane`)
  if (!PLANE_ASSESSMENTS.includes(text(record.assessment))) {
    errors.push(`${record.domain}.${record.plane}: assessment must be MISSING|WEAK|PARTIAL|STRONG`)
  }
  if (typeof record.assessment_score === 'number' || typeof record.maturity_index === 'number') {
    errors.push(`${record.domain}.${record.plane}: numeric precision forbidden`)
  }
  if (record.confidence && !CONFIDENCE.includes(text(record.confidence))) {
    errors.push(`${record.domain}.${record.plane}: confidence must be LOW|MEDIUM|HIGH`)
  }
  if (['PARTIAL', 'STRONG'].includes(text(record.assessment))) {
    const paths = asArray(record.evidence_paths).filter(hasText)
    const entries = asArray(record.runtime_entrypoints).filter(hasText)
    if (paths.length === 0 && entries.length === 0 && !hasText(record.evidence)) {
      errors.push(`${record.domain}.${record.plane}: PARTIAL/STRONG requires evidence_paths or runtime_entrypoints`)
    }
  }
  return { ok: errors.length === 0, errors }
}

export function validateFivePlaneData(doc) {
  const errors = []
  const records = asArray(doc?.plane_records)
  if (records.length === 0) errors.push('plane_records required')
  const seen = new Set()
  for (const record of records) {
    const key = `${text(record.domain)}::${text(record.plane)}`
    if (seen.has(key)) errors.push(`duplicate plane record ${key}`)
    seen.add(key)
    const result = validatePlaneRecord(record)
    if (!result.ok) errors.push(...result.errors)
  }
  const noAi = validateNoAiTaxonomy(asArray(doc?.no_ai_surfaces).length ? doc.no_ai_surfaces : asArray(doc?.domains))
  if (!noAi.ok) errors.push(...noAi.errors)
  return { ok: errors.length === 0, errors }
}

export function liveRowToPlaneRecords(row) {
  const domain = text(row.domain)
  const records = []
  for (const [liveKey, plane] of Object.entries(LIVE_PLANE_KEY_TO_ENGINE)) {
    const record = emptyPlaneRecord(domain, plane)
    record.assessment = PLANE_ASSESSMENTS.includes(row[liveKey]) ? row[liveKey] : 'MISSING'
    record.evidence_paths = unique(asArray(row.evidence_files))
    record.runtime_entrypoints = unique(asArray(row.evidence_symbols))
    record.no_ai_class = row.no_ai || null
    record.confidence = record.assessment === 'STRONG' ? 'HIGH' : record.assessment === 'PARTIAL' ? 'MEDIUM' : 'LOW'
    record.evidence = row.evidence
    records.push(record)
  }
  return records
}

export function liveAuditToFivePlaneData(audit, extras = {}) {
  const plane_records = asArray(audit?.no_ai_matrix).flatMap((row) => liveRowToPlaneRecords(row))
  return {
    schema: 'chronica.five_plane_engine_data.v1',
    source_schema: audit?.schema || null,
    freeze: audit?.freeze || null,
    verdict_frozen: audit?.verdict || null,
    plane_records,
    no_ai_surfaces: extras.no_ai_surfaces || [],
    kernel_good_paths: asArray(audit?.kernel_good_paths).map((row) => text(row.id)).filter(Boolean),
  }
}
