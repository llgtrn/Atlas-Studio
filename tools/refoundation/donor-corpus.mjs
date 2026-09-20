#!/usr/bin/env node
// Donor corpus checklist tooling (CHRONICA CORPUS MASTER CHECKLIST directive, 2026-09-16).
//
// tools/refoundation/donor-corpus.yaml is the single canonical machine-readable checklist for the
// donor corpus. This script is the only thing allowed to derive projections from it:
//   node tools/refoundation/donor-corpus.mjs --validate   # schema + duplicate-upstream check, exit 0/1
//   node tools/refoundation/donor-corpus.mjs --report      # regenerate docs/ops_production/donor/MASTER-CHECKLIST.md
// Both are report-only: neither mutates donor-corpus.yaml itself. Editing the checklist is a normal
// YAML edit; this script only reads it.
import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'
import { parse as parseYaml } from 'yaml'
import { buildSummaryLines } from './donor-burndown.mjs'

const HERE = dirname(fileURLToPath(import.meta.url))
const ROOT = join(HERE, '..', '..')
const CHECKLIST_PATH = join(ROOT, 'tools/refoundation/donor-corpus.yaml')
const REPORT_PATH = join(ROOT, 'docs/ops_production/donor/MASTER-CHECKLIST.md')

const REQUIRED_FIELDS = [
  'id',
  'repo',
  'canonical_upstream',
  'family',
  'status',
  'needed',
  'source_present',
  'census_complete',
  'absorption_state',
  'can_delete_source',
]

const VALID_STATUS = new Set([
  'DISCOVERED',
  'NEEDED',
  'REVIEW_NEEDED',
  'SOURCE_PRESENT',
  'CENSUSED',
  'ABSORBING',
  'PARTIALLY_ABSORBED',
  'ABSORBED',
  'RETIRE_SOURCE',
  'RETIRED',
  'REFERENCE_ONLY',
  'REDUNDANT',
  'DUPLICATE',
  'NOT_NEEDED',
])

const VALID_NEEDED = new Set(['YES', 'NO', 'UNKNOWN'])

const TERMINAL_ABSORPTION_STATUS = new Set(['ABSORBED', 'RETIRE_SOURCE', 'RETIRED'])
const VALID_STRATEGIC_CLASS = new Set(['REFERENCE', 'USEFUL', 'STRATEGIC', 'FOUNDATIONAL'])
const VALID_NATIVE_LIFECYCLE = new Set([
  'DONOR_TECHNOLOGY',
  'CENSUSED',
  'PRESERVED_LINEAGE',
  'NATIVE_CANDIDATE',
  'NATIVE_PROTOTYPE',
  'NATIVE_PARITY',
  'NATIVE_PRIMARY',
  'FULLY_NATIVE',
  'DONOR_EXTINCT',
  'DEFERRED_BY_SEQUENCE',
  'REJECTED_FOR_NATIVE_PROMOTION',
])
const EXTINCTION_SAFE_PROMOTION_STATE = new Set(['FULLY_NATIVE', 'DONOR_EXTINCT', 'REJECTED_FOR_NATIVE_PROMOTION'])

export function loadChecklist() {
  return parseYaml(readFileSync(CHECKLIST_PATH, 'utf8'))
}

export function validateChecklist(doc) {
  const errors = []
  if (doc?.schema_version !== 1) errors.push(`schema_version must be 1, got ${JSON.stringify(doc?.schema_version)}`)
  if (!Array.isArray(doc?.donors)) {
    errors.push('donors must be an array')
    return errors
  }
  const seenIds = new Set()
  const seenUpstream = new Map()
  doc.donors.forEach((d, i) => {
    const tag = `donors[${i}] (${d?.repo ?? 'unknown'})`
    for (const field of REQUIRED_FIELDS) {
      if (d[field] === undefined) errors.push(`${tag}: missing required field "${field}"`)
    }
    if (d.id) {
      if (seenIds.has(d.id)) errors.push(`${tag}: duplicate id "${d.id}"`)
      seenIds.add(d.id)
    }
    if (d.status && !VALID_STATUS.has(d.status)) errors.push(`${tag}: invalid status "${d.status}"`)
    if (d.needed && !VALID_NEEDED.has(String(d.needed).toUpperCase()) && typeof d.needed !== 'boolean') {
      errors.push(`${tag}: needed must be YES/NO/UNKNOWN, got "${d.needed}"`)
    }
    if (d.canonical_upstream) {
      const key = d.canonical_upstream.toLowerCase().replace(/\.git$/, '').replace(/\/$/, '')
      if (seenUpstream.has(key) && d.status !== 'DUPLICATE') {
        errors.push(
          `${tag}: duplicate canonical_upstream "${d.canonical_upstream}" also used by ${seenUpstream.get(key)} (mark one status: DUPLICATE with duplicate_of set)`,
        )
      }
      seenUpstream.set(key, d.repo)
    }

    // ADR-0022 / Native Technology Strategy reconciliation:
    // a terminal donor status is not allowed to outrun the technology census.
    if (TERMINAL_ABSORPTION_STATUS.has(d.status) && d.census_complete !== true) {
      errors.push(`${tag}: terminal status "${d.status}" requires census_complete: true`)
    }
    if (d.absorption_state === 'complete' && d.census_complete !== true) {
      errors.push(`${tag}: absorption_state "complete" requires census_complete: true`)
    }

    const promotion = d.native_promotion
    if (promotion !== undefined && promotion !== null) {
      if (!VALID_STRATEGIC_CLASS.has(promotion.strategic_class)) {
        errors.push(`${tag}: native_promotion.strategic_class must be one of ${[...VALID_STRATEGIC_CLASS].join(', ')}, got "${promotion.strategic_class}"`)
      }
      if (!VALID_NATIVE_LIFECYCLE.has(promotion.lifecycle_state)) {
        errors.push(`${tag}: native_promotion.lifecycle_state must be a Native Technology Strategy lifecycle state, got "${promotion.lifecycle_state}"`)
      }
      if (typeof promotion.extinction_eligible !== 'boolean') {
        errors.push(`${tag}: native_promotion.extinction_eligible must be boolean`)
      }

      const promoted = promotion.strategic_class === 'STRATEGIC' || promotion.strategic_class === 'FOUNDATIONAL'
      const extinctionSafe = EXTINCTION_SAFE_PROMOTION_STATE.has(promotion.lifecycle_state)
      if (promoted && !extinctionSafe && d.can_delete_source === true) {
        errors.push(`${tag}: ${promotion.strategic_class} technology at ${promotion.lifecycle_state} cannot set can_delete_source: true before FULLY_NATIVE / DONOR_EXTINCT / REJECTED_FOR_NATIVE_PROMOTION`)
      }
      if (promoted && !extinctionSafe && promotion.extinction_eligible === true) {
        errors.push(`${tag}: ${promotion.strategic_class} technology at ${promotion.lifecycle_state} cannot be extinction_eligible`)
      }
      if (promotion.extinction_eligible === true && d.census_complete !== true) {
        errors.push(`${tag}: extinction_eligible requires census_complete: true`)
      }
    }
  })
  return errors
}

function countBy(donors, fn) {
  const out = {}
  for (const d of donors) {
    const v = fn(d)
    const list = Array.isArray(v) ? v : [v]
    for (const x of list) out[x] = (out[x] || 0) + 1
  }
  return out
}

export function generateReport(doc) {
  const donors = doc.donors
  const total = donors.length
  const byStatus = countBy(donors, (d) => d.status)
  const byFamily = countBy(donors, (d) => d.family)
  const duplicates = donors.filter((d) => d.status === 'DUPLICATE').length

  const lines = []
  lines.push('# Donor Corpus Master Checklist (generated)')
  lines.push('')
  lines.push(
    '> **Generated from `tools/refoundation/donor-corpus.yaml` by `node tools/refoundation/donor-corpus.mjs --report`. ' +
      'Do not hand-edit this file — edit the YAML checklist and regenerate.**',
  )
  lines.push('')
  lines.push(
    'This checklist is repository governance metadata (CHRONICA CORPUS MASTER CHECKLIST directive, ' +
      '2026-09-16), not canonical runtime truth. It answers what donor repositories exist, which are ' +
      'relevant, which have source present, and which have been censused/absorbed/retired. It does not ' +
      'sequence donor execution order and does not cover licensing (see `license/donors/` and ' +
      '`docs/ops_production/donor/DONOR-CENSUS-PROVENANCE.md` for that).',
  )
  lines.push('')
  lines.push(`Generated at: ${doc.generated_at}`)
  lines.push('')
  lines.push('## Source evidence')
  lines.push('')
  for (const s of doc.source_evidence || []) lines.push(`- ${s}`)
  lines.push('')
  lines.push('## Totals')
  lines.push('')
  lines.push(`- **Total unique donors:** ${total}`)
  lines.push(`- **Duplicates collapsed:** ${duplicates}`)
  lines.push('')
  lines.push('## By status')
  lines.push('')
  lines.push('| Status | Count |')
  lines.push('|---|---|')
  for (const status of VALID_STATUS) lines.push(`| ${status} | ${byStatus[status] || 0} |`)
  lines.push('')
  lines.push('## By family')
  lines.push('')
  lines.push('| Family | Count |')
  lines.push('|---|---|')
  for (const [family, count] of Object.entries(byFamily).sort((a, b) => b[1] - a[1])) {
    lines.push(`| ${family} | ${count} |`)
  }
  lines.push('')
  lines.push('## Absorption burn-down (live, computed from Git/filesystem at generation time)')
  lines.push('')
  lines.push(
    'This section is never hand-edited and never persisted as static data -- it is recomputed by ' +
      '`node tools/refoundation/donor-burndown.mjs --summary` every time this report regenerates. ' +
      'See that tool for per-donor detail (`--donor <slug>`) and the stall/doc-churn detectors ' +
      '(`--stalled`, `--doc-churn`).',
  )
  lines.push('')
  lines.push('```text')
  for (const line of buildSummaryLines()) lines.push(line)
  lines.push('```')
  lines.push('')
  lines.push('## Full donor list')
  lines.push('')
  lines.push('| ID | Repo | Family | Status | Needed | Source present | Absorption state |')
  lines.push('|---|---|---|---|---|---|---|')
  for (const d of donors) {
    lines.push(
      `| ${d.id} | [${d.repo}](${d.canonical_upstream}) | ${(d.family || []).join(', ')} | ${d.status} | ${d.needed} | ${d.source_present ? 'yes' : 'no'} | ${d.absorption_state} |`,
    )
  }
  lines.push('')
  return lines.join('\n')
}

function main() {
  const args = new Set(process.argv.slice(2))
  const doc = loadChecklist()

  if (args.has('--validate') || args.size === 0) {
    const errors = validateChecklist(doc)
    if (errors.length) {
      console.error(`donor-corpus: FAILED validation (${errors.length} error(s)):`)
      for (const e of errors) console.error(`  - ${e}`)
      process.exitCode = 1
      return
    }
    console.log(`donor-corpus: OK (${doc.donors.length} donors, schema_version ${doc.schema_version})`)
  }

  if (args.has('--report')) {
    const report = generateReport(doc)
    writeFileSync(REPORT_PATH, report)
    console.log(`donor-corpus: wrote ${REPORT_PATH}`)
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main()
