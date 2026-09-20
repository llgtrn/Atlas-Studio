#!/usr/bin/env node
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const MEANING_FIELDS = ['purpose', 'doc_claim', 'code_reality', 'judgment', 'action_taken']
const DIMENSIONS = ['code_tests', 'docs_meaning', 'capability_tracking', 'architecture_tracking']
const FILLER_VALUES = new Set(['', 'n/a', 'na', 'none', 'unknown', 'tbd', 'todo', 'not sure'])
const WEB_COMMERCE_PATTERN = /\b(web|wce|commerce|e-?commerce|storefront|checkout|hosting|domain|dns|tls|ssl|deploy|rollback|design|template|clone|reconstruction)\b/i
const ECOSYSTEM_CONNECTIONS = [
  'identity_scope',
  'policy_approval',
  'vault_secrets',
  'template_artifact',
  'commerce',
  'erp',
  'crm',
  'helpdesk',
  'legal_privacy',
  'events_projection',
  'analytics_observability',
  'workflow_automation',
  'strategy_memory',
  'deployment_runtime',
]
const ECOSYSTEM_STATUS = new Set(['CONNECTED', 'LOCAL_AUDIT_REQUIRED', 'BLOCKED_WITH_EVIDENCE', 'NOT_APPLICABLE'])

const STATUS = {
  code_tests: new Set(['CONFIRMED', 'BLOCKED_WITH_EVIDENCE', 'NOT_APPLICABLE']),
  docs_meaning: new Set(['CONFIRMED', 'BLOCKED_WITH_EVIDENCE', 'NOT_APPLICABLE']),
  capability_tracking: new Set(['CLOUD_CONFIRMED', 'LOCAL_AUDIT_REQUIRED', 'BLOCKED_WITH_EVIDENCE', 'NOT_APPLICABLE']),
  architecture_tracking: new Set(['CLOUD_CONFIRMED', 'LOCAL_AUDIT_REQUIRED', 'BLOCKED_WITH_EVIDENCE', 'NOT_APPLICABLE']),
}

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
  if (Array.isArray(value)) {
    return value.some((item) => text(item).length > 0)
  }
  return text(value).length > 0
}

function requireText(errors, object, path) {
  const value = path.split('.').reduce((acc, part) => (acc == null ? undefined : acc[part]), object)
  if (!text(value)) {
    errors.push(`${path} is required`)
    return
  }
  if (isFiller(value)) {
    errors.push(`${path} must not be filler`)
  }
}

function requiresEcosystemConnections(report) {
  return WEB_COMMERCE_PATTERN.test([
    text(report.domain),
    text(report.scope),
    text(report.meaning?.purpose),
    text(report.meaning?.doc_claim),
    text(report.meaning?.code_reality),
    text(report.meaning?.judgment),
    text(report.meaning?.action_taken),
  ].join(' '))
}

function validateEcosystemConnections(errors, report) {
  if (!requiresEcosystemConnections(report)) {
    return
  }

  if (!isObject(report.ecosystem_connections)) {
    errors.push('ecosystem_connections is required for web/commerce/design/hosting reports')
    return
  }

  for (const connection of ECOSYSTEM_CONNECTIONS) {
    const entry = report.ecosystem_connections[connection]
    if (!isObject(entry)) {
      errors.push(`ecosystem_connections.${connection} is required`)
      continue
    }

    const status = text(entry.status)
    if (!status) {
      errors.push(`ecosystem_connections.${connection}.status is required`)
    } else if (!ECOSYSTEM_STATUS.has(status)) {
      errors.push(`ecosystem_connections.${connection}.status is invalid: ${status}`)
    }

    if (status === 'CONNECTED' || status === 'BLOCKED_WITH_EVIDENCE') {
      if (!hasEvidence(entry.evidence)) {
        errors.push(`ecosystem_connections.${connection}.evidence is required when status is ${status}`)
      }
    }

    if (status === 'LOCAL_AUDIT_REQUIRED' || status === 'NOT_APPLICABLE') {
      if (!hasEvidence(entry.reason)) {
        errors.push(`ecosystem_connections.${connection}.reason is required when status is ${status}`)
      }
    }
  }
}

export function validateFourWayReport(report) {
  const errors = []
  const warnings = []

  if (!isObject(report)) {
    return { ok: false, errors: ['report must be a JSON object'], warnings }
  }

  for (const field of ['branch', 'domain', 'scope', 'truth_label']) {
    requireText(errors, report, field)
  }

  if (/production[_ -]?ready/i.test(text(report.truth_label))) {
    errors.push('truth_label must not claim PRODUCTION_READY from a cloud/harvest report')
  }

  if (!isObject(report.meaning)) {
    errors.push('meaning is required')
  } else {
    for (const field of MEANING_FIELDS) {
      requireText(errors, report, `meaning.${field}`)
    }
  }

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
      if (!status) {
        errors.push(`four_way.${dimension}.status is required`)
      } else if (!STATUS[dimension].has(status)) {
        errors.push(`four_way.${dimension}.status is invalid: ${status}`)
      }

      if (status === 'CONFIRMED' || status === 'CLOUD_CONFIRMED' || status === 'BLOCKED_WITH_EVIDENCE') {
        if (!hasEvidence(entry.evidence)) {
          errors.push(`four_way.${dimension}.evidence is required when status is ${status}`)
        }
      }

      if (status === 'LOCAL_AUDIT_REQUIRED' || status === 'NOT_APPLICABLE') {
        if (!hasEvidence(entry.reason)) {
          errors.push(`four_way.${dimension}.reason is required when status is ${status}`)
        }
      }
    }
  }

  validateEcosystemConnections(errors, report)

  if (Array.isArray(report.blockers) && report.blockers.length > 0 && !/BLOCKED_WITH_EVIDENCE|LOCAL_AUDIT_REQUIRED/.test(text(report.truth_label))) {
    warnings.push('blockers are present but truth_label is not BLOCKED_WITH_EVIDENCE or LOCAL_AUDIT_REQUIRED')
  }

  return {
    ok: errors.length === 0,
    errors,
    warnings,
  }
}

function readStdin() {
  return readFileSync(0, 'utf8')
}

function main() {
  const path = process.argv[2]
  const raw = path && path !== '-' ? readFileSync(path, 'utf8') : readStdin()
  let report
  try {
    report = JSON.parse(raw)
  } catch (error) {
    const result = { ok: false, errors: [`invalid JSON: ${error.message}`], warnings: [] }
    console.log(JSON.stringify(result, null, 2))
    process.exitCode = 1
    return
  }

  const result = validateFourWayReport(report)
  console.log(JSON.stringify(result, null, 2))
  process.exitCode = result.ok ? 0 : 1
}

if (fileURLToPath(import.meta.url) === process.argv[1]) {
  main()
}
