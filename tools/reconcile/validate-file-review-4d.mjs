#!/usr/bin/env node
import Database from 'better-sqlite3'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const MEANING_FIELDS = ['purpose', 'source_reality', 'chronica_action', 'judgment']
const DIMENSIONS = ['code_tests', 'docs_meaning', 'capability_tracking', 'architecture_tracking']
const FILLER_VALUES = new Set(['', 'n/a', 'na', 'none', 'unknown', 'tbd', 'todo', 'not sure'])
const CONFIRMED_STATUSES = new Set(['CONFIRMED', 'BLOCKED_WITH_EVIDENCE'])
const REASON_STATUSES = new Set(['LOCAL_AUDIT_REQUIRED', 'NOT_APPLICABLE'])
const STATUS = new Set([...CONFIRMED_STATUSES, ...REASON_STATUSES])
const REVIEW_CLASSIFICATIONS = new Set([
  'mapped',
  'duplicate_variant_mapped',
  'behavioral_excluded',
  'non_behavioral_support',
  'generated_vendor_build_artifact',
  'blocked',
])
const REVIEW_STATUSES = new Set(['reviewed', 'classified_not_read', 'blocked'])
const MAPPED_CLASSIFICATIONS = new Set(['mapped', 'duplicate_variant_mapped'])

function isObject(value) {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function text(value) {
  return typeof value === 'string' ? value.trim() : ''
}

function isFiller(value) {
  return FILLER_VALUES.has(text(value).toLowerCase())
}

function hasEvidence(value) {
  if (Array.isArray(value)) return value.some((item) => text(item).length > 0)
  return text(value).length > 0
}

function requireText(errors, object, path) {
  const value = path.split('.').reduce((acc, part) => (acc == null ? undefined : acc[part]), object)
  if (!text(value)) {
    errors.push(`${path} is required`)
    return
  }
  if (isFiller(value)) errors.push(`${path} must not be filler`)
}

function idList(value) {
  if (Array.isArray(value)) return value.map((id) => String(id).trim()).filter(Boolean)
  return String(value || '')
    .split(',')
    .map((id) => id.trim())
    .filter(Boolean)
}

function tableExists(db, table) {
  return db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name=?").get(table).n > 0
}

function fileRows(report) {
  if (Array.isArray(report?.file_review?.files)) return report.file_review.files
  if (Array.isArray(report?.file_reviews)) return report.file_reviews
  return []
}

function validateShape(report) {
  const errors = []
  const warnings = []
  if (!isObject(report)) return { errors: ['report must be a JSON object'], warnings, files: [] }

  for (const field of ['batch_id', 'donor', 'reviewed_by', 'scope']) requireText(errors, report, field)

  if (!isObject(report.meaning)) {
    errors.push('meaning is required')
  } else {
    for (const field of MEANING_FIELDS) requireText(errors, report, `meaning.${field}`)
  }

  const files = fileRows(report)
  if (!files.length) errors.push('file_review.files is required and must be non-empty')

  files.forEach((row, idx) => {
    if (!isObject(row)) {
      errors.push(`file_review.files[${idx}] must be an object`)
      return
    }
    const prefix = `file_review.files[${idx}]`
    requireText(errors, row, 'path')
    const classification = text(row.classification)
    const readStatus = text(row.read_status)
    if (!REVIEW_CLASSIFICATIONS.has(classification)) errors.push(`${prefix}.classification is invalid: ${classification || '(empty)'}`)
    if (!REVIEW_STATUSES.has(readStatus)) errors.push(`${prefix}.read_status is invalid: ${readStatus || '(empty)'}`)
    if (classification === 'blocked' && readStatus !== 'blocked') errors.push(`${prefix} blocked classification requires blocked read_status`)
    if (!hasEvidence(row.evidence) && !hasEvidence(row.evidence_note) && !hasEvidence(row.exclusion_reason)) {
      errors.push(`${prefix}.evidence is required`)
    }
    if (MAPPED_CLASSIFICATIONS.has(classification) && idList(row.mapped_source_ids).length === 0) {
      errors.push(`${prefix}.mapped_source_ids is required for mapped files`)
    }
  })

  if (!isObject(report.four_way)) {
    errors.push('four_way is required')
  } else {
    for (const dimension of DIMENSIONS) {
      const entry = report.four_way[dimension]
      if (!isObject(entry)) {
        errors.push(`four_way.${dimension} is required`)
        continue
      }
      const status = text(entry.status)
      if (!STATUS.has(status)) errors.push(`four_way.${dimension}.status is invalid: ${status || '(empty)'}`)
      if (CONFIRMED_STATUSES.has(status) && !hasEvidence(entry.evidence)) {
        errors.push(`four_way.${dimension}.evidence is required when status is ${status}`)
      }
      if (REASON_STATUSES.has(status) && !hasEvidence(entry.reason)) {
        errors.push(`four_way.${dimension}.reason is required when status is ${status}`)
      }
      if (dimension === 'architecture_tracking' && status === 'CONFIRMED' && idList(entry.capability_keys).length === 0) {
        errors.push('four_way.architecture_tracking.capability_keys is required when architecture tracking is CONFIRMED')
      }
    }
  }

  return { errors, warnings, files }
}

function validateCapabilityDb(errors, report, files, root) {
  const dbPath = join(root, 'docs', 'capabilities.db')
  if (!existsSync(dbPath)) {
    errors.push('docs/capabilities.db is missing')
    return
  }
  const db = new Database(dbPath, { readonly: true })
  try {
    for (const table of ['donor_file_census', 'source_capability', 'source_file_capability_link']) {
      if (!tableExists(db, table)) errors.push(`capability DB table missing: ${table}`)
    }
    if (errors.some((error) => error.startsWith('capability DB table missing'))) return

    const census = db.prepare('SELECT classification, read_status, mapped_source_ids, exclusion_reason FROM donor_file_census WHERE donor=? AND path=?')
    const source = db.prepare('SELECT donor FROM source_capability WHERE id=?')
    const link = db.prepare('SELECT evidence_note FROM source_file_capability_link WHERE donor=? AND path=? AND source_id=?')
    for (const file of files) {
      const path = text(file.path)
      const row = census.get(report.donor, path)
      if (!row) {
        errors.push(`donor_file_census row missing: ${report.donor}/${path}`)
        continue
      }
      if (row.read_status === 'unread_pending') errors.push(`donor_file_census row still unread_pending: ${report.donor}/${path}`)
      if (row.classification !== text(file.classification)) {
        errors.push(`donor_file_census classification mismatch for ${report.donor}/${path}: report=${text(file.classification)} db=${row.classification}`)
      }
      if (row.read_status !== text(file.read_status)) {
        errors.push(`donor_file_census read_status mismatch for ${report.donor}/${path}: report=${text(file.read_status)} db=${row.read_status}`)
      }

      if (!MAPPED_CLASSIFICATIONS.has(text(file.classification))) continue
      const expectedIds = idList(file.mapped_source_ids)
      const dbIds = new Set(idList(row.mapped_source_ids))
      for (const id of expectedIds) {
        if (!dbIds.has(id)) errors.push(`donor_file_census mapped_source_ids missing ${id} for ${report.donor}/${path}`)
        const sourceRow = source.get(Number(id))
        if (!sourceRow) errors.push(`source_capability missing for source_id=${id}`)
        else if (sourceRow.donor !== report.donor) errors.push(`source_capability donor mismatch for source_id=${id}: report=${report.donor} db=${sourceRow.donor}`)
        const linkRow = link.get(report.donor, path, Number(id))
        if (!linkRow) errors.push(`source_file_capability_link missing for ${report.donor}/${path} source_id=${id}`)
        else if (!hasEvidence(linkRow.evidence_note)) errors.push(`source_file_capability_link evidence missing for ${report.donor}/${path} source_id=${id}`)
      }
    }

    const pending = db.prepare("SELECT count(*) n FROM donor_file_census WHERE donor=? AND read_status='unread_pending'").get(report.donor).n
    if (Number.isFinite(Number(report.file_review?.pending_after)) && pending !== Number(report.file_review.pending_after)) {
      errors.push(`donor pending count mismatch for ${report.donor}: report=${report.file_review.pending_after} db=${pending}`)
    }
    if (report.require_zero_pending_for_donor === true && pending !== 0) {
      errors.push(`${report.donor} still has unread_pending rows: ${pending}`)
    }
  } finally {
    db.close()
  }
}

function validateArchitectureDb(errors, report, root) {
  const arch = report.four_way?.architecture_tracking
  if (!isObject(arch) || text(arch.status) !== 'CONFIRMED') return
  const dbPath = join(root, 'docs', 'architecture.db')
  if (!existsSync(dbPath)) {
    errors.push('docs/architecture.db is missing')
    return
  }
  const db = new Database(dbPath, { readonly: true })
  try {
    for (const table of ['capability_architecture_link', 'architecture_target_gap']) {
      if (!tableExists(db, table)) errors.push(`architecture DB table missing: ${table}`)
    }
    if (errors.some((error) => error.startsWith('architecture DB table missing'))) return
    const accounted = db.prepare(`SELECT 1 FROM (
      SELECT capability_key FROM capability_architecture_link WHERE capability_key=?
      UNION
      SELECT capability_key FROM architecture_target_gap WHERE capability_key=?
    ) LIMIT 1`)
    for (const key of idList(arch.capability_keys)) {
      if (!accounted.get(key, key)) errors.push(`architecture accounting missing for capability key: ${key}`)
    }
  } finally {
    db.close()
  }
}

export function validateFileReview4dReport(report, options = {}) {
  const root = options.root || process.cwd()
  const { errors, warnings, files } = validateShape(report)

  if (options.checkDb && isObject(report)) {
    validateCapabilityDb(errors, report, files, root)
    validateArchitectureDb(errors, report, root)
  }

  return {
    ok: errors.length === 0,
    errors,
    warnings,
    summary: {
      donor: text(report?.donor) || null,
      files_reviewed: files.length,
      mapped_files: files.filter((row) => MAPPED_CLASSIFICATIONS.has(text(row.classification))).length,
      check_db: Boolean(options.checkDb),
    },
  }
}

function argValue(name) {
  const i = process.argv.indexOf(name)
  return i >= 0 ? process.argv[i + 1] : null
}

function readStdin() {
  return readFileSync(0, 'utf8')
}

function main() {
  const checkDb = process.argv.includes('--check-db')
  const root = argValue('--root') || process.cwd()
  const path = process.argv.find((arg, idx) => idx > 1 && !arg.startsWith('--') && process.argv[idx - 1] !== '--root')
  const raw = path && path !== '-' ? readFileSync(path, 'utf8') : readStdin()
  let report
  try {
    report = JSON.parse(raw)
  } catch (error) {
    console.log(JSON.stringify({ ok: false, errors: [`invalid JSON: ${error.message}`], warnings: [] }, null, 2))
    process.exitCode = 1
    return
  }
  const result = validateFileReview4dReport(report, { root, checkDb })
  console.log(JSON.stringify(result, null, 2))
  process.exitCode = result.ok ? 0 : 1
}

if (fileURLToPath(import.meta.url) === process.argv[1]) main()
