#!/usr/bin/env node
// fic-recompute-lib.mjs — fail-closed FIC revalidation against doctrine 180.
//
// This EXTENDS tools/reconcile/fic-domain-evidence-lib.mjs. That aggregator never assigns L0-L6.
// This controller library DOES apply doctrine 180 section 2's ladder to a hand-authored census,
// and it refuses illegal promotions. It never invents a 56th domain, L7, or a second denominator.
//
// Authority:
//   docs/doctrines/180-doctrine-world-class-infrastructure-coverage.md sections 1, 2, 6, 15
//   docs/_machine/world-class-infrastructure-coverage-v1.json (55-key taxonomy)
//
// Capability-row counts are subordinate inventory. They cannot raise a level.
import { readFileSync } from 'node:fs'
import { join } from 'node:path'

export const FIC_LEVELS = Object.freeze(['L0', 'L1', 'L2', 'L3', 'L4', 'L5', 'L6'])
export const LEVEL_RANK = Object.freeze({ L0: 0, L1: 1, L2: 2, L3: 3, L4: 4, L5: 5, L6: 6 })

export const CENSUS_REQUIRED_FIELDS = Object.freeze([
  'domain',
  'previous_level',
  'evidence_sha',
  'owner',
  'runtime_entry',
  'caller',
  'business_consumer',
  'state_boundary',
  'external_effect',
  'failure_path',
  'recovery_boundary',
  'security_boundary',
  'tests',
  'telemetry',
  'operations',
  'new_level',
  'level_changed',
  'reason',
  'remaining_gap',
  'next_required_evidence',
])

export const HAND_WAVY_REASON_PATTERNS = Object.freeze([
  /crate exists therefore/i,
  /route exists therefore/i,
  /tests pass therefore functionally covered/i,
  /ci exists therefore/i,
  /kubernetes yaml exists therefore/i,
  /opentelemetry crate exists therefore/i,
  /capability row/i,
  /verified_count/i,
])

export const FLOATING_REFS = Object.freeze(['main', 'master', 'develop', 'HEAD', 'trunk', 'default'])

const FILLER = new Set(['', 'n/a', 'na', 'none', 'unknown', 'tbd', 'todo', 'not sure', '-', 'null'])

function text(value) {
  return typeof value === 'string' ? value.trim() : ''
}

function isFiller(value) {
  return FILLER.has(text(value).toLowerCase())
}

function hasText(value) {
  return text(value).length > 0 && !isFiller(value)
}

function isAbsentRuntime(value) {
  return /^(ISLAND|MISSING|NONE|DOC_ONLY|FAKE_COUPLING)\b/i.test(text(value))
}

function isAbsentCaller(value) {
  return /^(ISLAND|MISSING|NONE)\b/i.test(text(value)) || /no shipped caller/i.test(text(value))
}

function isAbsentRecovery(value) {
  return /^(MISSING|NOT_BUILT|unproven|none documented)\b/i.test(text(value))
}

export function loadJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'))
}

export function loadV1Denominator(root) {
  return loadJson(join(root, 'docs/_machine/world-class-infrastructure-coverage-v1.json'))
}

export function denominatorKeys(v1) {
  return (v1.domains || []).map((d) => d.key)
}

function testsOk(tests) {
  if (!tests || typeof tests !== 'object' || Array.isArray(tests)) return false
  const positive = Array.isArray(tests.positive) ? tests.positive.filter((t) => hasText(t)) : []
  const negative = Array.isArray(tests.negative) ? tests.negative.filter((t) => hasText(t)) : []
  return { positive, negative, ok: positive.length > 0 && negative.length > 0 }
}

export function l3Gate(row) {
  const blockers = []
  const requiredNamed = [
    'owner',
    'runtime_entry',
    'caller',
    'business_consumer',
    'state_boundary',
    'failure_path',
    'recovery_boundary',
    'security_boundary',
    'telemetry',
  ]
  for (const field of requiredNamed) {
    if (!hasText(row[field])) blockers.push(`missing_${field}`)
  }
  const tests = testsOk(row.tests)
  if (!tests.ok) blockers.push('missing_positive_and_negative_tests')
  if (!hasText(row.operations) && !(row.operations && hasText(row.operations.owner))) {
    blockers.push('missing_operations_owner')
  }
  if (isAbsentRuntime(row.runtime_entry)) blockers.push('runtime_entry_not_shipped')
  if (isAbsentCaller(row.caller)) blockers.push('caller_not_shipped')
  if (isAbsentRecovery(row.recovery_boundary)) blockers.push('recovery_missing')
  return { ok: blockers.length === 0, blockers, tests }
}

export function l4Gate(row) {
  const l3 = l3Gate(row)
  const blockers = [...l3.blockers]
  const pec = row.pec || row.production_evidence || ''
  if (!hasText(pec) || /NOT_PROVEN|LOCAL_AUDIT|UNKNOWN/i.test(String(pec))) {
    blockers.push('pec_not_proven')
  }
  if (!hasText(row.slo) && !/SLO|sli/i.test(text(row.telemetry))) {
    blockers.push('slo_not_proven')
  }
  return { ok: blockers.length === 0, blockers }
}

/**
 * Apply doctrine 180: a census row may demote freely, but may not claim L3+ unless the L3 gate
 * passes, and may not claim L4+ unless PEC-shaped evidence is present. Capability counts never
 * raise a level. Hand-wavy reasons are BLOCKED.
 */
export function applyLevelRules(row) {
  const errors = []
  const claimed = row.new_level
  if (!FIC_LEVELS.includes(claimed)) {
    errors.push(`invalid_level:${claimed}`)
    return { ok: false, errors, effective_level: 'L0', promotion: 'INVALID' }
  }
  if (!FIC_LEVELS.includes(row.previous_level)) {
    errors.push(`invalid_previous_level:${row.previous_level}`)
  }
  for (const pat of HAND_WAVY_REASON_PATTERNS) {
    if (pat.test(text(row.reason))) {
      errors.push(`hand_wavy_reason:${pat}`)
    }
  }
  if (typeof row.capability_count === 'number' && LEVEL_RANK[claimed] >= 3) {
    if (/because .{0,40}(verified|capability).{0,20}count/i.test(text(row.reason))) {
      errors.push('capability_count_used_as_fic_promotion')
    }
  }

  const l3 = l3Gate(row)
  const l4 = l4Gate(row)
  let effective = claimed
  if (LEVEL_RANK[claimed] >= 6) {
    errors.push('L6_requires_comparable_workload_evidence')
    effective = l4.ok ? 'L4' : l3.ok ? 'L3' : 'L2'
  } else if (LEVEL_RANK[claimed] >= 5) {
    errors.push('L5_requires_failure_domain_drills')
    effective = l4.ok ? 'L4' : l3.ok ? 'L3' : 'L2'
  } else if (LEVEL_RANK[claimed] >= 4 && !l4.ok) {
    errors.push(`invalid_fic_promotion_L4:${l4.blockers.join(',')}`)
    effective = l3.ok ? 'L3' : 'L2'
  } else if (LEVEL_RANK[claimed] >= 3 && !l3.ok) {
    errors.push(`invalid_fic_promotion_L3:${l3.blockers.join(',')}`)
    effective = 'L2'
    if (!hasText(row.runtime_entry) && !hasText(row.caller) && !hasText(row.owner)) effective = 'L0'
    else if (!hasText(row.runtime_entry) || /MISSING|NOT_BUILT|DOC_ONLY/i.test(row.runtime_entry)) effective = 'L1'
  }

  const expectedChanged = row.previous_level !== effective
  if (row.level_changed !== expectedChanged && row.level_changed !== (row.previous_level !== claimed)) {
    // Allow level_changed to reflect the author's claimed delta; flag only if neither matches.
    if (row.level_changed !== (row.previous_level !== claimed)) {
      errors.push('level_changed_inconsistent')
    }
  }

  return {
    ok: errors.length === 0 && claimed === effective,
    errors,
    effective_level: effective,
    promotion: LEVEL_RANK[effective] > LEVEL_RANK[row.previous_level] ? 'PROMOTED' : LEVEL_RANK[effective] < LEVEL_RANK[row.previous_level] ? 'DEMOTED' : 'UNCHANGED',
    l3_blockers: l3.blockers,
  }
}

export function validateCensusShape(census, v1) {
  const errors = []
  const keys = denominatorKeys(v1)
  if (keys.length !== 55) errors.push(`denominator_drift:v1_has_${keys.length}_not_55`)
  if (!census || census.schema !== 'chronica.fic_revalidation_census.v1') {
    errors.push('missing_or_invalid_census_schema')
  }
  const rows = Array.isArray(census?.domains) ? census.domains : []
  if (rows.length !== 55) errors.push(`census_row_count:${rows.length}_not_55`)

  const seen = new Set()
  for (const row of rows) {
    const domain = text(row.domain)
    if (!domain) {
      errors.push('missing_domain')
      continue
    }
    if (seen.has(domain)) errors.push(`duplicate_domain:${domain}`)
    seen.add(domain)
    if (!keys.includes(domain)) errors.push(`unknown_domain:${domain}`)
    for (const field of CENSUS_REQUIRED_FIELDS) {
      if (row[field] === undefined || row[field] === null) errors.push(`${domain}:missing_field:${field}`)
    }
    if (!hasText(row.evidence_sha) || !/^[0-9a-f]{40}$/i.test(text(row.evidence_sha))) {
      errors.push(`${domain}:evidence_sha_must_be_full_git_sha`)
    }
    const applied = applyLevelRules(row)
    if (!applied.ok) {
      for (const err of applied.errors) errors.push(`${domain}:${err}`)
    }
  }
  for (const key of keys) {
    if (!seen.has(key)) errors.push(`skipped_domain:${key}`)
  }
  return { ok: errors.length === 0, errors }
}

export function computeFic(census) {
  const counts = { L0: 0, L1: 0, L2: 0, L3: 0, L4: 0, L5: 0, L6: 0 }
  const appliedRows = []
  for (const row of census.domains) {
    const applied = applyLevelRules(row)
    const level = applied.ok ? row.new_level : applied.effective_level
    counts[level] += 1
    appliedRows.push({ ...row, applied })
  }
  const l3plus = counts.L3 + counts.L4 + counts.L5 + counts.L6
  return {
    denominator: 55,
    counts,
    l3_or_higher: l3plus,
    fic_percent: Math.round((l3plus / 55) * 10000) / 100,
    target_l3_domains: 50,
    target_percent: 90.91,
    pec: census.pec || 'LOCAL_AUDIT_REQUIRED',
    cse: census.cse || 'UNKNOWN_NOT_PROVEN',
    rows: appliedRows,
  }
}

export function diffLevels(v1, census) {
  const prev = new Map((v1.domains || []).map((d) => [d.key, d.level]))
  const changes = []
  for (const row of census.domains) {
    const previous = prev.get(row.domain) || row.previous_level
    if (previous !== row.new_level) {
      changes.push({
        domain: row.domain,
        previous_level: previous,
        new_level: row.new_level,
        direction: LEVEL_RANK[row.new_level] < LEVEL_RANK[previous] ? 'DEMOTE' : 'PROMOTE',
        reason: row.reason,
      })
    }
  }
  return changes
}
