#!/usr/bin/env node
// intent-contract-validate.mjs — doctrine 116/180/183 builder contract gate.
// Builders may choose implementation details. They may not redefine goal, user_value,
// required_invariants, product_witness, non_goals, or benchmark_posture without a contract revision.
import { readdirSync, readFileSync, existsSync } from 'node:fs'
import path from 'node:path'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

export const CONTRACT_REQUIRED_FIELDS = Object.freeze([
  'intent_id',
  'fic_domains',
  'benchmark_families',
  'benchmark_sources',
  'goal',
  'user_value',
  'current_chronica',
  'gap',
  'owner',
  'expected_runtime_entry',
  'expected_business_consumer',
  'required_state',
  'required_authority',
  'required_invariants',
  'required_failure_behavior',
  'required_recovery',
  'security_class',
  'money_class',
  'external_io',
  'non_goals',
  'positive_tests',
  'negative_tests',
  'product_witness',
  'benchmark_posture',
  'done_when',
])

export const IMMUTABLE_INTENT_FIELDS = Object.freeze([
  'goal',
  'user_value',
  'required_invariants',
  'product_witness',
  'non_goals',
  'benchmark_posture',
])

export const ASSIGNMENTS = Object.freeze(['Builder A', 'Builder B', 'DEFER', 'BLOCKED'])
export const POSTURES = Object.freeze(['MUST_MATCH', 'MUST_EXCEED', 'INTENTIONAL_DIFFERENCE', 'INFORMATIONAL'])
export const MONEY_CLASSES = Object.freeze(['MONEY_FREE', 'MONEY_ADJACENT', 'MONEY_MOVING', 'MONEY_BLOCKED'])
export const SECURITY_CLASSES = Object.freeze(['LOW', 'TENANT_SCOPED', 'SECRET_SCOPED', 'PROVIDER_BOUNDARY', 'HIGH'])

const FILLER = new Set(['', 'n/a', 'na', 'none', 'unknown', 'tbd', 'todo', 'improve', 'fix'])
const FAKE_WITNESS = [/tests pass/i, /crate exists/i, /route returns 200/i, /capability row/i]

function text(value) {
  return typeof value === 'string' ? value.trim() : ''
}

function isFiller(value) {
  return FILLER.has(text(value).toLowerCase())
}

export function validateIntentContract(contract, { denominatorKeys = [], familyKeys = [] } = {}) {
  const errors = []
  if (!contract || typeof contract !== 'object') return { ok: false, errors: ['contract must be an object'] }
  for (const field of CONTRACT_REQUIRED_FIELDS) {
    if (contract[field] == null) errors.push(`missing ${field}`)
  }
  const id = text(contract.intent_id)
  if (!/^[A-Z0-9]+(?:-[A-Z0-9]+)+-\d{3}$/.test(id)) {
    errors.push('intent_id must look like AI-DURABLE-001')
  }
  if (isFiller(contract.goal) || /^improve /i.test(text(contract.goal))) {
    errors.push('goal is generic or filler')
  }
  if (!Array.isArray(contract.fic_domains) || contract.fic_domains.length === 0) {
    errors.push('fic_domains required')
  } else if (denominatorKeys.length) {
    for (const key of contract.fic_domains) {
      if (!denominatorKeys.includes(key)) errors.push(`unknown fic_domain ${key}`)
    }
  }
  if (Array.isArray(contract.benchmark_families) && familyKeys.length) {
    for (const key of contract.benchmark_families) {
      if (!familyKeys.includes(key)) errors.push(`unknown benchmark_family ${key}`)
    }
  }
  if (!POSTURES.includes(contract.benchmark_posture)) {
    errors.push(`invalid benchmark_posture ${contract.benchmark_posture}`)
  }
  if (!MONEY_CLASSES.includes(contract.money_class)) {
    errors.push(`invalid money_class ${contract.money_class}`)
  }
  if (contract.security_class && !SECURITY_CLASSES.includes(contract.security_class)) {
    errors.push(`invalid security_class ${contract.security_class}`)
  }
  if (!Array.isArray(contract.positive_tests) || contract.positive_tests.filter((t) => text(t)).length === 0) {
    errors.push('positive_tests required')
  }
  if (!Array.isArray(contract.negative_tests) || contract.negative_tests.filter((t) => text(t)).length === 0) {
    errors.push('negative_tests required')
  }
  if (!Array.isArray(contract.required_invariants) || contract.required_invariants.length === 0) {
    errors.push('required_invariants required')
  }
  if (!Array.isArray(contract.non_goals) || contract.non_goals.length === 0) {
    errors.push('non_goals required')
  }
  if (FAKE_WITNESS.some((p) => p.test(text(contract.product_witness)))) {
    errors.push('fake product witness')
  }
  if (!text(contract.product_witness) || isFiller(contract.product_witness)) {
    errors.push('product_witness required')
  }
  if (contract.assignment && !ASSIGNMENTS.includes(contract.assignment)) {
    errors.push(`invalid assignment ${contract.assignment}`)
  }
  if (contract.money_class === 'MONEY_MOVING' && !/MONEY_BLOCKED|approval/i.test(JSON.stringify(contract.required_authority))) {
    errors.push('MONEY_MOVING contract must name approval/money-gate authority')
  }
  if (contract.benchmark_posture === 'INFORMATIONAL' && ['MONEY_MOVING', 'MONEY_ADJACENT'].includes(contract.money_class)) {
    errors.push('INFORMATIONAL misuse on money-class contract')
  }
  if (contract.benchmark_posture === 'INFORMATIONAL' && /MUST_MATCH|MUST_EXCEED/.test(JSON.stringify(contract.required_invariants || []))) {
    errors.push('INFORMATIONAL misuse: invariants demand MUST_MATCH/MUST_EXCEED')
  }
  return { ok: errors.length === 0, errors, immutable_fields: IMMUTABLE_INTENT_FIELDS }
}

export function loadContracts(dir) {
  if (!existsSync(dir)) return []
  return readdirSync(dir)
    .filter((name) => name.endsWith('.json'))
    .sort()
    .map((name) => ({
      path: join(dir, name),
      contract: JSON.parse(readFileSync(join(dir, name), 'utf8')),
    }))
}

function main() {
  const root = process.cwd()
  const dir = process.argv[2] || join(root, 'docs/_machine/intent-contracts')
  const files = loadContracts(dir)
  const seen = new Set()
  let failed = 0
  for (const { path, contract } of files) {
    const result = validateIntentContract(contract)
    if (seen.has(contract.intent_id)) {
      result.ok = false
      result.errors.push(`duplicate intent_id ${contract.intent_id}`)
    }
    seen.add(contract.intent_id)
    const status = result.ok ? 'OK' : 'FAIL'
    if (!result.ok) failed += 1
    console.log(`${status} ${path}${result.ok ? '' : `\n  ${result.errors.join('\n  ')}`}`)
  }
  if (files.length === 0) {
    console.error(`no contracts in ${dir}`)
    process.exitCode = 2
    return
  }
  process.exitCode = failed ? 1 : 0
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
