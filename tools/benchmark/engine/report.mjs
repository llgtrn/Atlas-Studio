// Human Markdown is generated from machine JSON. Hand-edited disagreement fails closed.

import { FIC_DENOMINATOR, FROZEN_VERDICT } from './vocab.mjs'
import { text } from './util.mjs'

export function renderTruthReport(snapshot) {
  const p = snapshot.provenance || {}
  const fic = snapshot.fic || {}
  const freeze = snapshot.frozen_architecture || FROZEN_VERDICT
  const lines = [
    '# Chronica benchmark truth snapshot',
    '',
    `schema: ${snapshot.schema}`,
    `repository_sha: ${p.repository_sha}`,
    `evaluated_sha: ${p.evaluated_sha}`,
    `base_sha: ${p.base_sha}`,
    `product_truth_sha: ${p.product_truth_sha || 'UNKNOWN'}`,
    `benchmark_schema_version: ${p.benchmark_schema_version}`,
    `generator_version: ${p.generator_version}`,
    `generated_at: ${p.generated_at}`,
    `freshness: ${p.freshness}`,
    `stale_reason: ${p.stale_reason || 'none'}`,
    '',
    '## Frozen architecture (not rewritten by this engine)',
    '',
    `FIVE_PLANE_AUDIT_STATUS: ${freeze.five_plane_audit_status}`,
    `VERDICT: ${freeze.verdict} — ${freeze.verdict_label}`,
    `CORE_SPINE: ${freeze.core_spine}`,
    `AUTHORITATIVE_ARCHITECTURE_VERDICT: ${freeze.authoritative_architecture_verdict === true ? 'YES' : 'NO'}`,
    '',
    '## FIC',
    '',
    `FIC: ${fic.l3_or_higher}/${fic.denominator || FIC_DENOMINATOR}`,
    `not_capability_inventory: ${fic.not_capability_inventory === true ? 'true' : 'false'}`,
    `PEC: ${fic.pec || 'LOCAL_AUDIT_REQUIRED'}`,
    `CSE: ${fic.cse || 'UNKNOWN_NOT_PROVEN'}`,
    '',
    '## Freshness rule',
    '',
    'Tests passing do not imply CURRENT or an authoritative architecture verdict.',
    '',
  ]

  const drift = snapshot.finding_drift || []
  lines.push('## Provisional finding drift')
  lines.push('')
  if (drift.length === 0) lines.push('PROVISIONAL_FINDING_DRIFT: NONE')
  else {
    for (const row of drift) lines.push(`- ${row.code || 'PROVISIONAL_FINDING_DRIFT'}: ${row.detail}`)
  }
  lines.push('')
  return `${lines.join('\n')}\n`
}

export function validateReportMatchesSnapshot(markdown, snapshot) {
  const errors = []
  const md = String(markdown || '')
  const p = snapshot.provenance || {}
  const fic = snapshot.fic || {}
  const freeze = snapshot.frozen_architecture || {}

  if (!md.includes(`repository_sha: ${p.repository_sha}`)) errors.push('Markdown cannot disagree with machine JSON: repository_sha')
  if (!md.includes(`freshness: ${p.freshness}`)) errors.push('Markdown cannot disagree with machine JSON: freshness')
  if (!md.includes(`FIC: ${fic.l3_or_higher}/${fic.denominator}`)) {
    errors.push('Markdown cannot disagree with machine JSON: FIC')
  }
  if (freeze.verdict && !md.includes(`VERDICT: ${freeze.verdict}`)) {
    errors.push('Markdown cannot disagree with machine JSON: verdict')
  }

  const ficFromCaps = /FIC:\s*(\d+)\s*\/\s*(\d+)/.exec(md)
  if (ficFromCaps && Number(ficFromCaps[2]) !== FIC_DENOMINATOR) {
    errors.push('Markdown cannot disagree with machine JSON: FIC denominator')
  }
  if (/FIC coverage is \d+ verified capabilities/i.test(md) || /FIC:\s*1290\/5151/.test(md)) {
    errors.push('capability count cannot substitute FIC')
  }
  if (p.freshness !== 'CURRENT' && /AUTHORITATIVE_ARCHITECTURE_VERDICT: YES/.test(md)) {
    errors.push('Markdown cannot claim authoritative architecture from a non-CURRENT snapshot')
  }
  return { ok: errors.length === 0, errors }
}

export function extractClaims(markdown) {
  return {
    sha: text((/repository_sha:\s*([0-9a-f]{40})/i.exec(markdown) || [])[1]),
    freshness: text((/freshness:\s*([A-Z]+)/.exec(markdown) || [])[1]),
    fic: text((/FIC:\s*(\d+\/\d+)/.exec(markdown) || [])[1]),
  }
}
