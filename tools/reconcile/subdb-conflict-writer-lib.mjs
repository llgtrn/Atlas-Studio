// subdb-conflict-writer-lib.mjs -- the "needs human judgment" half of the conflict-audit writer
// (docs/doctrines/023-five-dimension-cloud-shard-contract.md, marked NOT_BUILT there; section 7's
// `docs/_machine/subdb-conflicts/<timestamp>.json` / `.md` conflict-writing flow).
//
// Every sync-anchor-v2 disagreement that is NOT an EXACT-confidence
// SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL finding (that class is mechanically safe to
// self-correct -- see subdb-apply-agreed-corrections-lib.mjs) is Category B: an ISLAND could mean
// "wire it in" or "delete it", an overclaim needs a human to decide whether the shard or the code
// is wrong, and a PREFIX-confidence match needs a human to confirm the fuzzy name match is even
// real. This module never auto-resolves any of it -- it only writes a structured, crate-grouped
// backlog for local audit to work from.
import { CORRECTABLE_REASON } from './subdb-apply-agreed-corrections-lib.mjs'
import {
  comparePlain,
  redactValue,
  sanitizeCapabilityKey,
  sanitizeCrateName,
  sanitizeEnumField,
  sanitizeTargetModule,
} from './sync-anchor-v2-lib.mjs'

/** Anything left over after subdb-apply-agreed-corrections.mjs has run is Category B by
 * construction, but this stays defensive (does not assume the caller pre-filtered) and explicitly
 * excludes the one class that tool auto-fixes. */
export function isCategoryBFinding(finding) {
  if (finding.verdict !== 'DISAGREE') return false
  if (finding.reason === CORRECTABLE_REASON && finding.evidence?.match_confidence === 'exact') return false
  return true
}

const REASON_LABEL = {
  CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT: 'ISLAND',
  SHARD_CLAIMS_IMPLEMENTED_BUT_NO_REAL_CODE_FOUND: 'overclaim',
  SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL: 'PREFIX-confidence (deferred)',
  DOC_STATUS_CONTRADICTS_SHARD_STATUS: 'doc/shard mismatch',
  SHARD_CLAIMS_TESTED_BUT_NO_TEST_FILE_REFERENCES_MODULE: 'missing test evidence',
}

const KNOWN_REASON_CODES = new Set(Object.keys(REASON_LABEL))
const MATCH_CONFIDENCES = new Set(['exact', 'prefix', 'none'])
const STATUS_VALUES = new Set(['unimplemented', 'implemented', 'implemented_unverified', 'verified'])

function safeReasonCode(reason) {
  return KNOWN_REASON_CODES.has(reason) ? reason : 'UNKNOWN_REASON_CODE'
}

function nullable(value, sanitizer) {
  return value == null ? null : sanitizer(value)
}

function safeFindingFields(finding) {
  return {
    capability_key: sanitizeCapabilityKey(finding.capability_key),
    crate: sanitizeCrateName(finding.crate),
    reason_code: safeReasonCode(finding.reason),
    match_confidence: nullable(finding.evidence?.match_confidence, (value) => sanitizeEnumField(value, MATCH_CONFIDENCES)),
    target_module: nullable(finding.target_module, sanitizeTargetModule),
    file: nullable(finding.evidence?.matched_modules?.[0], () => redactValue()),
    shard_status: nullable(finding.shard_status, (value) => sanitizeEnumField(value, STATUS_VALUES)),
    doc_status: nullable(finding.doc_status, (value) => sanitizeEnumField(value, STATUS_VALUES)),
  }
}

export function reasonLabel(reasonCode) {
  return REASON_LABEL[reasonCode] ?? reasonCode
}

export function suggestedNextStep(finding) {
  const safe = safeFindingFields(finding)
  switch (finding.reason) {
    case 'CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT':
      return (
        `ISLAND: code exists at ${safe.file ?? `target_module "${safe.target_module}"`} in ${safe.crate} but is ` +
        'not reachable from any mod-tree entry point. Decide: wire it into a real caller (see PR #2400\'s ' +
        'other.repo_scripts_registry pattern) or delete/deprecate it -- do not leave it silently unreachable.'
      )
    case 'SHARD_CLAIMS_IMPLEMENTED_BUT_NO_REAL_CODE_FOUND':
      return (
        `Overclaim: shard says "${safe.shard_status}" for ${safe.capability_key} but no real ` +
        `(non-stub) code was found at target_module "${safe.target_module}" in ${safe.crate}. ` +
        'Confirm the shard status is correct (was the code removed/renamed?) or downgrade it to ' +
        'unimplemented -- do not auto-resolve.'
      )
    case 'SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL':
      return (
        `PREFIX-confidence match only (fuzzy last-segment match, not exact) between target_module ` +
        `"${safe.target_module}" and ${safe.file ?? redactValue()} in ${safe.crate}. ` +
        'subdb-apply-agreed-corrections.mjs deliberately skips PREFIX-confidence hits -- ' +
        'a human must confirm the match is real before the shard status is corrected.'
      )
    case 'DOC_STATUS_CONTRADICTS_SHARD_STATUS':
      return (
        `Crate doc claims "${safe.doc_status}" but the shard says "${safe.shard_status}" for ` +
        `${safe.capability_key}. Confirm which surface is stale -- if the shard is now correct (e.g. a ` +
        'recent crate-local correction), update the crate doc\'s capabilities table row; otherwise correct ' +
        'the shard.'
      )
    case 'SHARD_CLAIMS_TESTED_BUT_NO_TEST_FILE_REFERENCES_MODULE':
      return (
        `Shard claims "verified" for ${safe.capability_key} but no test file references target_module ` +
        `"${safe.target_module}". Add test evidence or downgrade the shard status from verified.`
      )
    default:
      return `Unrecognized reason code "${safe.reason_code}" for ${safe.capability_key} in ${safe.crate} -- needs local audit review.`
  }
}

export function buildReviewQueueRecords(findings) {
  return findings.filter(isCategoryBFinding).map((finding) => {
    const safe = safeFindingFields(finding)
    return {
      ...safe,
      suggested_next_step: suggestedNextStep(finding),
    }
  })
}

function groupByCrate(records) {
  const byCrate = new Map()
  for (const record of records) {
    if (!byCrate.has(record.crate)) byCrate.set(record.crate, [])
    byCrate.get(record.crate).push(record)
  }
  return [...byCrate.entries()].sort((a, b) => b[1].length - a[1].length || comparePlain(a[0], b[0]))
}

function reasonBreakdown(records) {
  const counts = new Map()
  for (const record of records) counts.set(record.reason_code, (counts.get(record.reason_code) ?? 0) + 1)
  return [...counts.entries()].sort((a, b) => b[1] - a[1] || comparePlain(a[0], b[0]))
}

function mdEscape(value) {
  return String(value ?? '').replace(/\|/g, '\\|')
}

export function buildReviewQueueMarkdown(records, { date }) {
  const grouped = groupByCrate(records)
  const lines = []
  lines.push(`# Subdb Conflict Review Queue -- ${date}`)
  lines.push('')
  lines.push(
    'Generated by `tools/reconcile/subdb-conflict-writer.mjs` from the post-correction sync-anchor-v2 ' +
      'findings (docs/doctrines/023-five-dimension-cloud-shard-contract.md sections 4 and 7). Every row here is a ' +
      'Category-B finding: it needs human/local-audit judgment and was deliberately **not** auto-fixed.',
  )
  lines.push('')
  lines.push(`**${records.length} finding(s) across ${grouped.length} crate(s).**`)
  lines.push('')
  lines.push('## By reason code')
  lines.push('')
  lines.push('| Reason code | Meaning | Count |')
  lines.push('| --- | --- | --- |')
  for (const [reason, count] of reasonBreakdown(records)) {
    lines.push(`| \`${reason}\` | ${reasonLabel(reason)} | ${count} |`)
  }
  lines.push('')
  lines.push('## By crate (top offenders first)')
  lines.push('')
  lines.push('| Crate | Findings |')
  lines.push('| --- | --- |')
  for (const [crate, crateRecords] of grouped) lines.push(`| ${crate} | ${crateRecords.length} |`)
  lines.push('')
  for (const [crate, crateRecords] of grouped) {
    lines.push(`### ${crate} (${crateRecords.length})`)
    lines.push('')
    lines.push('| Capability | Reason | Confidence | Target module | File |')
    lines.push('| --- | --- | --- | --- | --- |')
    for (const record of crateRecords) {
      lines.push(
        `| ${mdEscape(record.capability_key)} | ${reasonLabel(record.reason_code)} | ` +
          `${mdEscape(record.match_confidence)} | ${mdEscape(record.target_module)} | ${mdEscape(record.file)} |`,
      )
    }
    lines.push('')
  }
  return `${lines.join('\n')}\n`
}
