#!/usr/bin/env node
// benchmark-pin-check-lib.mjs — doctrine 094/116 pin, license, floating-ref, and portfolio-minimum gates.
// Extends tools/benchmark/query-benchmark-ratio.mjs; does not replace it.
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'

export const FLOATING_REFS = new Set(['main', 'master', 'develop', 'HEAD', 'trunk', 'default', 'latest'])
export const SOURCE_CLASSES = new Set([
  'external_implementation',
  'standard_protocol',
  'internal_baseline',
  'adversarial_failure',
  'official_architecture',
])
export const FRESHNESS = new Set(['CURRENT', 'STALE_BUT_STILL_VALID', 'API_DRIFT', 'ARCHIVED', 'UNMAINTAINED', 'SUPERSEDED'])
export const INTEGRATION_MODES = new Set([
  'PERMISSIVE_NATIVE_REUSE',
  'CLEAN_ROOM_REIMPLEMENTATION',
  'PROTOCOL_ADAPTER',
  'ISOLATED_SIDECAR',
  'REFERENCE_ONLY',
  'LICENSE_BLOCKED',
  'TERMS_BLOCKED',
])
export const POSTURES = new Set(['MUST_MATCH', 'MUST_EXCEED', 'INTENTIONAL_DIFFERENCE', 'INFORMATIONAL'])

const SHA40 = /^[0-9a-f]{40}$/i

function loadJsonl(path) {
  if (!existsSync(path)) return []
  return readFileSync(path, 'utf8')
    .split('\n')
    .filter((l) => l.trim().length > 0)
    .map((l) => JSON.parse(l))
}

export function loadPortfolio(root) {
  return loadJsonl(join(root, 'docs/_machine/world-class-benchmark-portfolio.jsonl'))
}

export function loadGroundingLedger(root) {
  return loadJsonl(join(root, 'docs/_machine/reference-repo-grounding-ledger.jsonl'))
}

export function classifyPin(entry) {
  const pin = String(entry.pinned_sha || entry.pinned_version_or_sha || '').trim()
  const ref = String(entry.pin_ref || entry.branch || '').trim()
  if (!pin) return { status: 'PIN_REQUIRED', reason: 'missing pinned_sha' }
  if (FLOATING_REFS.has(pin) || FLOATING_REFS.has(ref)) {
    return { status: 'PIN_REQUIRED', reason: `floating_ref:${pin || ref}` }
  }
  if (!SHA40.test(pin)) {
    return { status: 'PIN_REQUIRED', reason: `pinned_sha_not_commit:${pin}` }
  }
  return { status: 'PINNED', sha: pin.toLowerCase() }
}

export function classifyLicense(entry) {
  const license = String(entry.license || '').trim()
  if (!license || /unknown|tbd|n\/a/i.test(license)) {
    return { status: 'LICENSE_BLOCKED', reason: 'license missing or unknown' }
  }
  if (!entry.license_file && !entry.license_files_read) {
    return { status: 'LICENSE_BLOCKED', reason: 'license file not read' }
  }
  return { status: 'LICENSE_KNOWN', license }
}

export function validatePortfolioEntry(entry, index) {
  const errors = []
  const id = entry.id || `index:${index}`
  if (entry.schema !== 'chronica.world_class_benchmark_portfolio.v1') {
    errors.push(`${id}: invalid schema`)
  }
  if (!entry.id) errors.push(`${id}: missing id`)
  if (!entry.repository && entry.source_class !== 'internal_baseline' && entry.source_class !== 'standard_protocol') {
    errors.push(`${id}: missing repository`)
  }
  const pin = classifyPin(entry)
  if (pin.status !== 'PINNED' && entry.source_class !== 'internal_baseline') {
    errors.push(`${id}: ${pin.status} (${pin.reason})`)
  }
  if (entry.source_class && !SOURCE_CLASSES.has(entry.source_class)) {
    errors.push(`${id}: invalid source_class ${entry.source_class}`)
  }
  if (entry.freshness && !FRESHNESS.has(entry.freshness)) {
    errors.push(`${id}: invalid freshness ${entry.freshness}`)
  }
  if (entry.integration_mode && !INTEGRATION_MODES.has(entry.integration_mode)) {
    errors.push(`${id}: invalid integration_mode ${entry.integration_mode}`)
  }
  const license = classifyLicense(entry)
  if (license.status === 'LICENSE_BLOCKED' && entry.source_class !== 'internal_baseline') {
    errors.push(`${id}: ${license.status} (${license.reason})`)
  }
  if (Array.isArray(entry.family_keys) && new Set(entry.family_keys).size !== entry.family_keys.length) {
    errors.push(`${id}: duplicate family_keys`)
  }
  return errors
}

export function validateLedgerEntry(entry, index) {
  const errors = []
  const id = entry.source_id || `index:${index}`
  if (entry.schema !== 'chronica.reference_repo_grounding_ledger.v1') {
    errors.push(`${id}: invalid schema`)
  }
  const required = [
    'source_id',
    'repository',
    'pinned_sha',
    'license',
    'license_files_read',
    'exact_paths_read',
    'intended_use',
    'integration_mode',
    'copy_data_model_boundary',
    'behavior_and_failures_extracted',
    'adopted',
    'rejected',
    'chronica_owner_caller_consumer',
    'money_security_regulatory_effect',
    'what_remains_unproven',
    'verdict',
  ]
  for (const field of required) {
    const value = entry[field]
    const empty = value == null || (typeof value === 'string' && !value.trim()) || (Array.isArray(value) && value.length === 0)
    if (empty) errors.push(`${id}: missing ${field}`)
  }
  const pin = classifyPin(entry)
  if (pin.status !== 'PINNED') errors.push(`${id}: ${pin.status} (${pin.reason})`)
  if (entry.integration_mode && !INTEGRATION_MODES.has(entry.integration_mode)) {
    errors.push(`${id}: invalid integration_mode`)
  }
  if (entry.integration_mode === 'LICENSE_BLOCKED' || entry.verdict === 'LICENSE_BLOCKED') {
    // valid fail-closed outcome
  }
  return errors
}

export function portfolioMinimumForFamily(family, matched) {
  const strategic = !!family.money_adjacent
  const impl = matched.filter((e) => e.source_class === 'external_implementation')
  const standard = matched.filter((e) => e.source_class === 'standard_protocol')
  const baseline = matched.filter((e) => e.source_class === 'internal_baseline')
  const adversarial = matched.filter((e) => e.source_class === 'adversarial_failure')
  const lineages = new Set(impl.map((e) => e.architectural_lineage).filter(Boolean))
  const uniqueRepos = new Set(impl.map((e) => e.repository).filter(Boolean))

  const required = strategic
    ? {
        impl: 3,
        lineages: 2,
        standard: 1,
        baseline: 1,
        adversarial: 1,
        text: 'strategic/high-risk per docs/116 section 5',
      }
    : {
        impl: 2,
        lineages: 0,
        standard: 1,
        baseline: 1,
        adversarial: 0,
        text: 'ordinary bounded per docs/116 section 5 (standard required when available)',
      }

  const gaps = []
  if (uniqueRepos.size < required.impl) gaps.push(`external_impl ${uniqueRepos.size}<${required.impl}`)
  if (required.lineages && lineages.size < required.lineages) gaps.push(`lineages ${lineages.size}<${required.lineages}`)
  if (required.standard && standard.length < required.standard) gaps.push(`standard ${standard.length}<${required.standard}`)
  if (baseline.length < required.baseline) gaps.push(`internal_baseline ${baseline.length}<${required.baseline}`)
  if (required.adversarial && adversarial.length < required.adversarial) {
    gaps.push(`adversarial ${adversarial.length}<${required.adversarial}`)
  }

  return {
    risk_tier: strategic ? 'strategic_high_risk' : 'ordinary_bounded',
    required,
    counts: {
      external_impl: uniqueRepos.size,
      lineages: lineages.size,
      standard: standard.length,
      baseline: baseline.length,
      adversarial: adversarial.length,
    },
    meets_minimum: gaps.length === 0,
    gaps,
  }
}

export function checkPortfolioAndLedger(root, { requireLedger094 = true } = {}) {
  const portfolio = loadPortfolio(root)
  const ledger = loadGroundingLedger(root)
  const errors = []
  const seenIds = new Set()
  portfolio.forEach((entry, i) => {
    if (entry.id) {
      if (seenIds.has(entry.id)) errors.push(`duplicate_benchmark_target:${entry.id}`)
      seenIds.add(entry.id)
    }
    errors.push(...validatePortfolioEntry(entry, i))
  })
  const seenLedger = new Set()
  ledger.forEach((entry, i) => {
    if (entry.source_id) {
      if (seenLedger.has(entry.source_id)) errors.push(`duplicate_ledger_source:${entry.source_id}`)
      seenLedger.add(entry.source_id)
    }
    if (requireLedger094) errors.push(...validateLedgerEntry(entry, i))
  })
  return {
    ok: errors.length === 0,
    errors,
    portfolio_count: portfolio.length,
    ledger_count: ledger.length,
    portfolio,
    ledger,
  }
}
