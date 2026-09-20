#!/usr/bin/env node
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { relative, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

export const MEANING_FIELDS = ['purpose', 'doc_claim', 'code_reality', 'judgment', 'action_taken']
export const FIVE_WAY_DIMENSIONS = ['code', 'tests_evidence', 'docs', 'crate_sub_cap_db', 'crate_sub_arch_db']
export const SIX_WAY_DIMENSIONS = [
  'code',
  'tests_evidence',
  'docs_specs_doctrine',
  'canonical_capability_shards',
  'canonical_architecture_shards',
  'generated_cache_reproducibility',
]
export const CONFLICT_CLASSES = [
  'DUPLICATE_CAPABILITY_KEY',
  'STATUS_MISMATCH',
  'MONEY_CLASSIFICATION_MISMATCH',
  'MISSING_IMPL_SYMBOL',
  'MISSING_TEST_SYMBOL',
  'DOC_VERDICT_MISMATCH',
  'ARCH_INVARIANT_MISMATCH',
  'STALE_SHARD_COMMIT',
  'SHARD_SCHEMA_MISMATCH',
  'UNOWNED_CAPABILITY',
  'CROSS_CRATE_AUTHORITY_CONFLICT',
  'AUTHORITY_CONFLICT',
  'SECRET_OR_VAULT_SURFACE_MISMATCH',
]

const FILLER_VALUES = new Set(['', 'n/a', 'na', 'none', 'unknown', 'tbd', 'todo', 'not sure'])
const WEB_COMMERCE_PATTERN = /\b(web|wce|commerce|e-?commerce|storefront|checkout|hosting|domain|dns|tls|ssl|deploy|rollback|design|template|clone|reconstruction)\b/i
const TRUTH_LABELS = new Set([
  'IMPLEMENTED_UNDER_TRACKED',
  'DOC_ONLY',
  'TRACKING_ONLY',
  'PARTIAL',
  'LOCAL_AUDIT_REQUIRED',
  'BLOCKED_WITH_EVIDENCE',
  'CLOUD_EVIDENCE_READY',
  'CONFLICT_BLOCKED',
  'CLOUD_FINAL_VERIFIED',
  'CLOUD_FINAL_BLOCKED',
  'GENERATED_CACHE_STALE',
  'HUMAN_AUTH_REQUIRED',
])
const CLOUD_FINAL_LABELS = new Set([
  'CLOUD_FINAL_VERIFIED',
  'CLOUD_FINAL_BLOCKED',
  'GENERATED_CACHE_STALE',
  'HUMAN_AUTH_REQUIRED',
])
const GENERIC_STATUS = new Set(['CONFIRMED', 'BLOCKED_WITH_EVIDENCE', 'NOT_APPLICABLE'])
const SHARD_STATUS = new Set(['CLOUD_CONFIRMED', 'LOCAL_AUDIT_REQUIRED', 'BLOCKED_WITH_EVIDENCE', 'NOT_APPLICABLE'])
const CLOUD_FINAL_STATUS = new Set(['CONFIRMED', 'CLOUD_FINAL_BLOCKED', 'GENERATED_CACHE_STALE', 'HUMAN_AUTH_REQUIRED', 'NOT_APPLICABLE'])
const LOCAL_AGGREGATE_STATUS = new Set(['LOCAL_AUDIT_ONLY', 'LOCAL_AUDIT_REQUIRED', 'LOCAL_VERIFIED', 'CONFLICT_BLOCKED'])
const ECOSYSTEM_STATUS = new Set(['CONNECTED', 'LOCAL_AUDIT_REQUIRED', 'BLOCKED_WITH_EVIDENCE', 'NOT_APPLICABLE'])
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
const CRATE_SUBDB_SHARD_PATTERN = /^crates\/chronica-[^/]+\/\.chronica\/sub-cap-arch\.jsonl$/
const CANONICAL_CAPABILITY_SHARD_PATTERN = /^docs\/capabilities-canonical\/.+\.jsonl$/
const CANONICAL_ARCHITECTURE_SHARD_PATTERN = /^docs\/architecture-canonical\/.+\.jsonl$/
const DB_ARTIFACT_PATTERN = /\.db(?:$|[?#])/i
const ARTIFACT_LIST_FIELDS = new Set([
  'artifacts',
  'pr_artifacts',
  'changed_files',
  'changedFiles',
  'files',
  'file_list',
  'pr_files',
])
const REQUIRED_CACHE_BUILD_COMMANDS = new Set([
  'pnpm caps:canonical:build-db',
  'pnpm arch:canonical:build-db',
])
// Fields conventionally used across cloud shard reports to bind a report to real git evidence. A
// SHORT sha (e.g. "abcdef1") is left alone -- it is inherently ambiguous outside a specific repo
// state, existing reports already use short shas as a lightweight reference, and this check only
// targets the FAILURE MODE PR #754's own admission review actually found: a full-length string that
// LOOKS like complete, unambiguous evidence but does not resolve to any real commit.
const GIT_SHA_FIELDS = ['commit', 'base_commit', 'implementation_commit', 'documentation_commit', 'last_report_refresh_commit']
const FULL_GIT_SHA_PATTERN = /^[0-9a-f]{40}$/i

let gitContextChecked = false
let gitContextAvailable = false
let currentBranch
let currentHead
let currentParents
let gitTopLevel

function hasGitContext() {
  if (!gitContextChecked) {
    gitContextChecked = true
    try {
      execFileSync('git', ['rev-parse', '--is-inside-work-tree'], { stdio: ['ignore', 'ignore', 'ignore'] })
      gitContextAvailable = true
    } catch {
      gitContextAvailable = false
    }
  }
  return gitContextAvailable
}

function gitObjectExists(sha) {
  try {
    execFileSync('git', ['cat-file', '-e', sha], { stdio: ['ignore', 'ignore', 'ignore'] })
    return true
  } catch {
    return false
  }
}

function gitCurrentBranch() {
  if (currentBranch !== undefined) return currentBranch
  try {
    currentBranch = execFileSync('git', ['rev-parse', '--abbrev-ref', 'HEAD'], { encoding: 'utf8' }).trim()
  } catch {
    currentBranch = ''
  }
  return currentBranch
}

function gitCurrentHead() {
  if (currentHead !== undefined) return currentHead
  try {
    currentHead = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim()
  } catch {
    currentHead = ''
  }
  return currentHead
}

function gitCurrentParents() {
  if (currentParents !== undefined) return currentParents
  try {
    currentParents = execFileSync('git', ['show', '-s', '--format=%P', 'HEAD'], { encoding: 'utf8' })
      .trim()
      .split(/\s+/)
      .filter(Boolean)
  } catch {
    currentParents = []
  }
  return currentParents
}

function gitRepoRoot() {
  if (gitTopLevel !== undefined) return gitTopLevel
  try {
    gitTopLevel = execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim()
  } catch {
    gitTopLevel = ''
  }
  return gitTopLevel
}

function gitLastCommitTouchingPath(reportPath) {
  const root = gitRepoRoot()
  if (!root || !reportPath) return ''

  const absolutePath = resolve(reportPath)
  const relativePath = relative(root, absolutePath).replace(/\\/g, '/')
  if (!relativePath || relativePath.startsWith('../') || relativePath === '..') return ''

  try {
    return execFileSync('git', ['log', '-1', '--format=%H', 'HEAD', '--', relativePath], { encoding: 'utf8' }).trim()
  } catch {
    return ''
  }
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

function getPathValue(object, path) {
  const parts = path.split('.')
  const nested = parts.reduce((acc, part) => (acc == null ? undefined : acc[part]), object)
  if (nested !== undefined) return nested
  return object?.[parts.at(-1)]
}

function requireText(errors, object, path) {
  const value = getPathValue(object, path)
  if (!text(value)) {
    errors.push(`${path} is required`)
    return ''
  }
  if (isFiller(value)) {
    errors.push(`${path} must not be filler`)
  }
  return text(value)
}

function requireArray(errors, object, path) {
  const value = getPathValue(object, path)
  if (!Array.isArray(value)) {
    errors.push(`${path} is required`)
    return []
  }
  if (value.some((item) => !text(item))) {
    errors.push(`${path} must contain only non-empty strings`)
  }
  return value
}

function requireStatus(errors, entry, path, allowed) {
  const status = requireText(errors, entry, `${path}.status`)
  if (status && !allowed.has(status)) {
    errors.push(`${path}.status is invalid: ${status}`)
  }
  return status
}

function validateEvidenceOrReason(errors, entry, path, status) {
  if (
    status === 'CONFIRMED' ||
    status === 'CLOUD_CONFIRMED' ||
    status === 'CONNECTED' ||
    status === 'LOCAL_VERIFIED' ||
    status === 'BLOCKED_WITH_EVIDENCE' ||
    status === 'CONFLICT_BLOCKED' ||
    status === 'CLOUD_FINAL_BLOCKED' ||
    status === 'GENERATED_CACHE_STALE'
  ) {
    if (!hasEvidence(entry.evidence)) {
      errors.push(`${path}.evidence is required when status is ${status}`)
    }
  }

  if (status === 'LOCAL_AUDIT_REQUIRED' || status === 'LOCAL_AUDIT_ONLY' || status === 'HUMAN_AUTH_REQUIRED' || status === 'NOT_APPLICABLE') {
    if (!hasEvidence(entry.reason)) {
      errors.push(`${path}.reason is required when status is ${status}`)
    }
  }
}

function validateGenericDimension(errors, report, key) {
  const entry = report.five_way[key]
  const path = `five_way.${key}`
  if (!isObject(entry)) {
    errors.push(`${path} is required`)
    return
  }
  const status = requireStatus(errors, entry, path, GENERIC_STATUS)
  validateEvidenceOrReason(errors, entry, path, status)
}

function validateShardDimension(errors, report, key) {
  const entry = report.five_way[key]
  const path = `five_way.${key}`
  if (!isObject(entry)) {
    errors.push(`${path} is required`)
    return
  }

  const status = requireStatus(errors, entry, path, SHARD_STATUS)
  if (status === 'CLOUD_CONFIRMED' || status === 'BLOCKED_WITH_EVIDENCE') {
    const shard = requireText(errors, entry, `${path}.shard`).replaceAll('\\', '/')
    if (shard && !CRATE_SUBDB_SHARD_PATTERN.test(shard)) {
      errors.push(`${path}.shard must be crates/<crate>/.chronica/sub-cap-arch.jsonl`)
    }
  }

  if (key === 'crate_sub_cap_db') {
    requireArray(errors, entry, `${path}.capability_keys`)
  }

  if (key === 'crate_sub_arch_db') {
    if (!Array.isArray(entry.architecture_nodes) && !Array.isArray(entry.invariant_keys)) {
      errors.push(`${path}.architecture_nodes or ${path}.invariant_keys is required`)
    }
  }

  validateEvidenceOrReason(errors, entry, path, status)
}

function validateLocalAggregate(errors, report) {
  if (!isObject(report.local_aggregate)) {
    errors.push('local_aggregate is required')
    return
  }

  for (const key of ['caps_db', 'arch_db']) {
    const entry = report.local_aggregate[key]
    const path = `local_aggregate.${key}`
    if (!isObject(entry)) {
      errors.push(`${path} is required`)
      continue
    }

    const status = text(entry.status)
    if (!status) {
      errors.push(`${path}.status is required`)
    } else if (!LOCAL_AGGREGATE_STATUS.has(status)) {
      errors.push(`${path}.status is invalid for a root aggregate DB: ${status}`)
    }

    requireArray(errors, entry, `${path}.targets`)
    validateEvidenceOrReason(errors, entry, path, status)
  }
}

function shardPaths(entry) {
  if (Array.isArray(entry.shards)) return entry.shards
  if (Array.isArray(entry.shard_paths)) return entry.shard_paths
  if (Array.isArray(entry.paths)) return entry.paths
  if (text(entry.shard)) return [entry.shard]
  return []
}

function validateCanonicalShardPaths(errors, entry, path, pattern, label) {
  const paths = shardPaths(entry).map((value) => text(value).replaceAll('\\', '/')).filter(Boolean)
  if (paths.length === 0) {
    errors.push(`${path}.shards is required for ${label}`)
    return
  }
  for (const shardPath of paths) {
    if (!pattern.test(shardPath)) errors.push(`${path}.shards contains invalid ${label} path: ${shardPath}`)
  }
}

function validateGeneratedCacheArtifacts(errors, entry, path, status) {
  const commands = Array.isArray(entry.commands) ? entry.commands.map((value) => text(value)).filter(Boolean) : []
  const commandRequired = status !== 'HUMAN_AUTH_REQUIRED' && status !== 'NOT_APPLICABLE'
  if (commandRequired && commands.length === 0) {
    errors.push(`${path}.commands is required for generated cache reproducibility`)
  }
  if (status === 'CONFIRMED') {
    for (const requiredCommand of REQUIRED_CACHE_BUILD_COMMANDS) {
      if (!commands.includes(requiredCommand)) errors.push(`${path}.commands must include ${requiredCommand}`)
    }
  }
  const artifactFields = ['artifacts', 'pr_artifacts', 'changed_files']
  for (const field of artifactFields) {
    const values = Array.isArray(entry[field]) ? entry[field] : []
    for (const value of values) {
      const artifact = text(value).replaceAll('\\', '/')
      if (DB_ARTIFACT_PATTERN.test(artifact)) {
        errors.push(`${path}.${field} must not list binary DB artifact for a cloud PR: ${artifact}`)
      }
    }
  }
}

function validateNoBinaryDbArtifactLists(errors, value, path = 'report') {
  if (Array.isArray(value)) {
    return
  }
  if (!isObject(value)) return
  for (const [key, nested] of Object.entries(value)) {
    const nestedPath = `${path}.${key}`
    if (ARTIFACT_LIST_FIELDS.has(key) && Array.isArray(nested)) {
      for (const item of nested) {
        const artifact = text(item).replaceAll('\\', '/')
        if (DB_ARTIFACT_PATTERN.test(artifact)) {
          errors.push(`${nestedPath} must not list binary DB artifact for a cloud PR: ${artifact}`)
        }
      }
    } else if (isObject(nested)) {
      validateNoBinaryDbArtifactLists(errors, nested, nestedPath)
    }
  }
}

function validateSixWayDimension(errors, report, key) {
  const entry = report.six_way[key]
  const path = `six_way.${key}`
  if (!isObject(entry)) {
    errors.push(`${path} is required`)
    return
  }
  const status = requireStatus(errors, entry, path, CLOUD_FINAL_STATUS)
  if (key === 'canonical_capability_shards') {
    validateCanonicalShardPaths(errors, entry, path, CANONICAL_CAPABILITY_SHARD_PATTERN, 'capability shard')
  } else if (key === 'canonical_architecture_shards') {
    validateCanonicalShardPaths(errors, entry, path, CANONICAL_ARCHITECTURE_SHARD_PATTERN, 'architecture shard')
  } else if (key === 'generated_cache_reproducibility') {
    validateGeneratedCacheArtifacts(errors, entry, path, status)
  }
  validateEvidenceOrReason(errors, entry, path, status)
}

function validateSixWayCloudFinal(errors, report) {
  if (!isObject(report.six_way)) {
    errors.push('six_way is required for cloud-final reports')
    return
  }
  for (const dimension of SIX_WAY_DIMENSIONS) {
    validateSixWayDimension(errors, report, dimension)
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

    const status = requireStatus(errors, entry, `ecosystem_connections.${connection}`, ECOSYSTEM_STATUS)
    validateEvidenceOrReason(errors, entry, `ecosystem_connections.${connection}`, status)
  }
}

function requiredSeverityForConflict(conflictClass) {
  if (conflictClass === 'MONEY_CLASSIFICATION_MISMATCH') return 'MONEY_BLOCKED'
  if (conflictClass === 'CROSS_CRATE_AUTHORITY_CONFLICT' || conflictClass === 'AUTHORITY_CONFLICT' || conflictClass === 'SECRET_OR_VAULT_SURFACE_MISMATCH') {
    return 'SECURITY_BLOCKED'
  }
  return null
}

function validateConflicts(errors, report) {
  if (report.conflicts == null) {
    return
  }

  if (!Array.isArray(report.conflicts)) {
    errors.push('conflicts must be an array when present')
    return
  }

  report.conflicts.forEach((entry, index) => {
    const path = `conflicts[${index}]`
    if (!isObject(entry)) {
      errors.push(`${path} must be an object`)
      return
    }

    const conflictClass = requireText(errors, entry, `${path}.class`)
    if (conflictClass && !CONFLICT_CLASSES.includes(conflictClass)) {
      errors.push(`${path}.class is invalid: ${conflictClass}`)
    }

    const severity = requireText(errors, entry, `${path}.severity`)
    const requiredSeverity = requiredSeverityForConflict(conflictClass)
    if (requiredSeverity && severity !== requiredSeverity) {
      errors.push(`${path}.severity must be ${requiredSeverity} for ${conflictClass}`)
    }

    if (!hasEvidence(entry.evidence)) {
      errors.push(`${path}.evidence is required`)
    }
  })
}

// Full-length (40 hex char) commit/base_commit/implementation_commit/documentation_commit values
// must resolve to a real git object when a git repository is actually available to check against --
// closing the exact gap an independent admission review found in PR #754's own reconcile reports
// (two full-shaped shas that shared a real commit's short prefix but were not that commit at all).
// When no git context is available (e.g. this validator invoked outside any checkout), the check is
// skipped with a warning rather than silently passing OR wrongly failing an unverifiable claim --
// preserving cloud-session compatibility for reports written where a full clone isn't guaranteed.
function validateGitEvidenceShas(errors, warnings, report) {
  const fullShaFields = GIT_SHA_FIELDS.filter((field) => FULL_GIT_SHA_PATTERN.test(text(report[field])))
  if (fullShaFields.length === 0) {
    return
  }

  if (!hasGitContext()) {
    warnings.push(
      `${fullShaFields.join(', ')}: full-length SHA value(s) present but could not be verified against git ` +
        'history -- no git repository context is available to this validator invocation (LOCAL_AUDIT_REQUIRED)'
    )
    return
  }

  for (const field of fullShaFields) {
    const sha = text(report[field])
    if (!gitObjectExists(sha)) {
      errors.push(
        `${field} is a full-length SHA (${sha}) that does not resolve to a real git object in this repository ` +
          '(git cat-file -e failed) -- bind it to a real, currently-reachable commit or use an explicit ' +
          'not-yet-pushed placeholder instead of a SHA-shaped value'
      )
    }
  }
}

function validateCurrentBranchHeadBinding(errors, report, options = {}) {
  if (!hasGitContext() && !options.currentBranch && !options.currentHead) return

  const reportBranch = text(report.branch)
  const branch = options.currentBranch ?? gitCurrentBranch()
  if (!reportBranch || !branch || branch === 'HEAD' || reportBranch !== branch) {
    return
  }

  const head = options.currentHead ?? gitCurrentHead()
  if (!FULL_GIT_SHA_PATTERN.test(head)) return

  const boundShas = [
    text(report.commit),
    text(report.last_report_refresh_commit),
  ].filter((value) => FULL_GIT_SHA_PATTERN.test(value))

  if (boundShas.includes(head)) {
    return
  }

  const refreshCommit = text(report.last_report_refresh_commit)
  const parentShas = options.currentParents ?? gitCurrentParents()
  const reportLastCommit = options.reportLastCommit ?? gitLastCommitTouchingPath(options.reportPath)
  if (
    FULL_GIT_SHA_PATTERN.test(refreshCommit) &&
    parentShas.includes(refreshCommit) &&
    reportLastCommit === head
  ) {
    return
  }

  errors.push(
    `report branch ${reportBranch} matches the current checkout, but neither commit nor ` +
      `last_report_refresh_commit equals current HEAD ${head}; if this commit only refreshes ` +
      'the report file, last_report_refresh_commit must equal a current HEAD parent and the ' +
      'report path must be last touched by current HEAD'
  )
}

export function validateCloudShardReport(report, options = {}) {
  const errors = []
  const warnings = []

  if (!isObject(report)) {
    return { ok: false, errors: ['report must be a JSON object'], warnings }
  }

  for (const field of ['branch', 'domain', 'scope', 'truth_label']) {
    requireText(errors, report, field)
  }

  const truthLabel = text(report.truth_label)
  if (/production[_ -]?ready/i.test(truthLabel)) {
    errors.push('truth_label must not claim PRODUCTION_READY from a cloud/harvest report')
  } else if (truthLabel && !TRUTH_LABELS.has(truthLabel)) {
    errors.push(`truth_label is invalid: ${truthLabel}`)
  }
  const cloudFinalReport = CLOUD_FINAL_LABELS.has(truthLabel) || isObject(report.six_way)

  if (!isObject(report.meaning)) {
    errors.push('meaning is required')
  } else {
    for (const field of MEANING_FIELDS) {
      requireText(errors, report, `meaning.${field}`)
    }
  }

  if (cloudFinalReport) {
    validateSixWayCloudFinal(errors, report)
    validateNoBinaryDbArtifactLists(errors, report)
    if (isObject(report.five_way)) {
      for (const dimension of FIVE_WAY_DIMENSIONS) {
        if (dimension === 'crate_sub_cap_db' || dimension === 'crate_sub_arch_db') {
          validateShardDimension(errors, report, dimension)
        } else {
          validateGenericDimension(errors, report, dimension)
        }
      }
    }
  } else if (!isObject(report.five_way)) {
    errors.push('five_way is required')
  } else {
    for (const dimension of FIVE_WAY_DIMENSIONS) {
      if (dimension === 'crate_sub_cap_db' || dimension === 'crate_sub_arch_db') {
        validateShardDimension(errors, report, dimension)
      } else {
        validateGenericDimension(errors, report, dimension)
      }
    }
  }

  if (!cloudFinalReport || isObject(report.local_aggregate)) {
    validateLocalAggregate(errors, report)
  }
  validateConflicts(errors, report)
  validateEcosystemConnections(errors, report)
  validateGitEvidenceShas(errors, warnings, report)
  validateCurrentBranchHeadBinding(errors, report, options)

  if (Array.isArray(report.blockers) && report.blockers.length > 0 && !/BLOCKED_WITH_EVIDENCE|LOCAL_AUDIT_REQUIRED|CONFLICT_BLOCKED|MONEY_BLOCKED|SECURITY_BLOCKED|CLOUD_FINAL_BLOCKED|GENERATED_CACHE_STALE|HUMAN_AUTH_REQUIRED/.test(truthLabel)) {
    warnings.push('blockers are present but truth_label is not blocked or local-audit-required')
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

  const result = validateCloudShardReport(report, { reportPath: path && path !== '-' ? path : undefined })
  console.log(JSON.stringify(result, null, 2))
  process.exitCode = result.ok ? 0 : 1
}

if (fileURLToPath(import.meta.url) === process.argv[1]) {
  main()
}
