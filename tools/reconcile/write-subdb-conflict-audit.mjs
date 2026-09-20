#!/usr/bin/env node
import { randomUUID } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { basename, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { CONFLICT_CLASSES } from './validate-cloud-shard-report.mjs'

export const MAX_INPUT_BYTES = 1024 * 1024
export const MAX_CONFLICTS = 100
export const MAX_CORRECTIONS = 100

const FAIL_CLOSED_SEVERITY = new Map([
  ['MONEY_CLASSIFICATION_MISMATCH', 'MONEY_BLOCKED'],
  ['CROSS_CRATE_AUTHORITY_CONFLICT', 'SECURITY_BLOCKED'],
  ['AUTHORITY_CONFLICT', 'SECURITY_BLOCKED'],
  ['SECRET_OR_VAULT_SURFACE_MISMATCH', 'SECURITY_BLOCKED'],
])
const SAFE_ID = /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/

function requiredText(value, path) {
  if (typeof value !== 'string' || !value.trim()) throw new Error(`${path} is required`)
  return value.trim()
}

function boundedArray(value, path, maximum) {
  if (!Array.isArray(value)) throw new Error(`${path} must be an array`)
  if (value.length > maximum) throw new Error(`${path} exceeds the limit of ${maximum}`)
  return value
}

export function normalizeConflictAudit(input) {
  if (!input || typeof input !== 'object' || Array.isArray(input)) throw new Error('input must be an object')
  const conflicts = boundedArray(input.conflicts, 'conflicts', MAX_CONFLICTS)
  const corrections = boundedArray(input.agreed_corrections ?? [], 'agreed_corrections', MAX_CORRECTIONS)
  const conflictIds = new Set()

  const normalizedConflicts = conflicts.map((entry, index) => {
    const id = requiredText(entry?.id, `conflicts[${index}].id`)
    if (!SAFE_ID.test(id)) throw new Error(`conflicts[${index}].id is not a safe bounded identifier`)
    if (conflictIds.has(id)) throw new Error(`duplicate conflict id: ${id}`)
    conflictIds.add(id)
    const conflictClass = requiredText(entry.class, `conflicts[${index}].class`)
    if (!CONFLICT_CLASSES.includes(conflictClass)) throw new Error(`conflicts[${index}].class is invalid: ${conflictClass}`)
    const severity = requiredText(entry.severity, `conflicts[${index}].severity`)
    const requiredSeverity = FAIL_CLOSED_SEVERITY.get(conflictClass)
    if (requiredSeverity && severity !== requiredSeverity) {
      throw new Error(`conflicts[${index}].severity must be ${requiredSeverity} for ${conflictClass}`)
    }
    return {
      id,
      class: conflictClass,
      severity,
      summary: requiredText(entry.summary, `conflicts[${index}].summary`),
      evidence: boundedArray(entry.evidence, `conflicts[${index}].evidence`, 20)
        .map((item, evidenceIndex) => requiredText(item, `conflicts[${index}].evidence[${evidenceIndex}]`)),
    }
  })

  const correctedIds = new Set()
  const normalizedCorrections = corrections.map((entry, index) => {
    const conflictId = requiredText(entry?.conflict_id, `agreed_corrections[${index}].conflict_id`)
    if (!conflictIds.has(conflictId)) throw new Error(`agreed_corrections[${index}] references unknown conflict: ${conflictId}`)
    if (correctedIds.has(conflictId)) throw new Error(`duplicate agreed correction for conflict: ${conflictId}`)
    correctedIds.add(conflictId)
    return {
      conflict_id: conflictId,
      correction: requiredText(entry.correction, `agreed_corrections[${index}].correction`),
      agreed_by: requiredText(entry.agreed_by, `agreed_corrections[${index}].agreed_by`),
      evidence: boundedArray(entry.evidence, `agreed_corrections[${index}].evidence`, 20)
        .map((item, evidenceIndex) => requiredText(item, `agreed_corrections[${index}].evidence[${evidenceIndex}]`)),
      application_status: 'NOT_APPLIED_LOCAL_AUDIT_REQUIRED',
    }
  })

  return {
    schema_version: 'chronica-subdb-conflict-audit-v1',
    source: requiredText(input.source, 'source'),
    local_aggregate_status: 'LOCAL_AUDIT_REQUIRED',
    conflicts: normalizedConflicts,
    agreed_corrections: normalizedCorrections,
    non_claims: ['This audit does not apply corrections or promote root capability/architecture truth.'],
  }
}

function markdown(audit) {
  const lines = [
    '# Sub-DB Conflict Audit', '',
    `- Source: \`${audit.source}\``,
    `- Local aggregate status: \`${audit.local_aggregate_status}\``,
    `- Conflicts: ${audit.conflicts.length}`,
    `- Agreed corrections (not applied): ${audit.agreed_corrections.length}`, '',
    '## Conflicts', '',
  ]
  if (audit.conflicts.length === 0) lines.push('_None recorded._', '')
  for (const conflict of audit.conflicts) {
    lines.push(`### ${conflict.id}`, '', `- Class: \`${conflict.class}\``, `- Severity: \`${conflict.severity}\``, `- Summary: ${conflict.summary}`, '- Evidence:')
    lines.push(...conflict.evidence.map((item) => `  - ${item}`), '')
  }
  lines.push('## Agreed corrections', '')
  if (audit.agreed_corrections.length === 0) lines.push('_None recorded._', '')
  for (const correction of audit.agreed_corrections) {
    lines.push(`### ${correction.conflict_id}`, '', `- Correction: ${correction.correction}`, `- Agreed by: ${correction.agreed_by}`, `- Application status: \`${correction.application_status}\``, '- Evidence:')
    lines.push(...correction.evidence.map((item) => `  - ${item}`), '')
  }
  lines.push('## Non-claims', '', `- ${audit.non_claims[0]}`, '')
  return lines.join('\n')
}

export function writeConflictAudit({ inputPath, outDir, timestamp = new Date().toISOString() }) {
  if (statSync(inputPath).size > MAX_INPUT_BYTES) throw new Error(`input exceeds the limit of ${MAX_INPUT_BYTES} bytes`)
  const audit = normalizeConflictAudit(JSON.parse(readFileSync(inputPath, 'utf8')))
  const slug = timestamp.replaceAll(':', '-').replaceAll('.', '-')
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}-\d{2}-\d{2}(?:-\d{3})?Z$/.test(slug)) throw new Error('timestamp must be an ISO-8601 UTC instant')
  const directory = resolve(outDir)
  mkdirSync(directory, { recursive: true })
  const jsonPath = join(directory, `${slug}.json`)
  const markdownPath = join(directory, `${slug}.md`)
  if (existsSync(jsonPath) || existsSync(markdownPath)) throw new Error(`refusing to overwrite conflict audit: ${basename(jsonPath)}`)
  const token = randomUUID()
  const jsonTemp = `${jsonPath}.${token}.tmp`
  const markdownTemp = `${markdownPath}.${token}.tmp`
  try {
    writeFileSync(jsonTemp, `${JSON.stringify({ ...audit, written_at: timestamp }, null, 2)}\n`, { flag: 'wx' })
    writeFileSync(markdownTemp, markdown(audit), { flag: 'wx' })
    renameSync(jsonTemp, jsonPath)
    renameSync(markdownTemp, markdownPath)
  } finally {
    rmSync(jsonTemp, { force: true })
    rmSync(markdownTemp, { force: true })
  }
  return { json_path: jsonPath, markdown_path: markdownPath, conflict_count: audit.conflicts.length, correction_count: audit.agreed_corrections.length }
}

function parseArgs(argv) {
  const options = { outDir: 'docs/_machine/subdb-conflicts' }
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === '--input') options.inputPath = argv[++index]
    else if (argv[index] === '--out-dir') options.outDir = argv[++index]
    else if (argv[index] === '--timestamp') options.timestamp = argv[++index]
    else throw new Error(`unknown argument: ${argv[index]}`)
  }
  if (!options.inputPath) throw new Error('--input is required')
  return options
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = writeConflictAudit(parseArgs(process.argv.slice(2)))
  console.log(JSON.stringify(result, null, 2))
}
