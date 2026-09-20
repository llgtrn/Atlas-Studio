import { readFileSync } from 'node:fs'
import { execFileSync } from 'node:child_process'
import { isAbsolute, resolve } from 'node:path'
import {
  architectureOwnerIdFromPath,
  canonicalArchitectureOwners,
  isCanonicalArchitectureOwner,
} from '../docs/architecture-registry.mjs'

export const CONTRACT_FENCE = /```chronica-contract\s*\n([\s\S]*?)\n```/g
export const CONTRACT_ID = /^(ARCH-EQ|INV|ARCH-PERF)-[A-Z0-9]+(?:-[A-Z0-9]+)*$/
export const CONTRACT_KINDS = new Set(['equation', 'invariant', 'performance'])
export const SEVERITIES = new Set(['hard', 'guardrail', 'advisory'])
export const PERFORMANCE_GATES = new Set(['STRUCTURAL', 'MEASURED', 'PRODUCTION'])

const OWNER_PREFIXES = [
  // 2026-09-16 hard refoundation: crates/{core,runtime,adapter,cap}/ no longer exists (crates/ is
  // retired -- see docs/decisions/0016-legacy-backend-retired-to-git-history.md). core/, runtime/,
  // adapter/, organism/ are top-level single-package crate roots now.
  'core/',
  'runtime/',
  'adapter/',
  'organism/',
  'graph/',
  'bindings/',
  'apps/',
  'ops/',
  'tools/',
]

function list(value) {
  if (Array.isArray(value)) return value
  if (value == null) return []
  return [value]
}

function nonEmptyStrings(value) {
  return list(value).every((item) => typeof item === 'string' && item.trim().length > 0)
}

function push(errors, condition, message) {
  if (!condition) errors.push(message)
}

function safeRepoRelativePath(value) {
  return typeof value === 'string'
    && value.trim().length > 0
    && !isAbsolute(value)
    && !value.split('/').includes('..')
    && !value.includes('\\')
}

export function parseContractsFromText(text, source = '<memory>') {
  const contracts = []
  for (const match of text.matchAll(CONTRACT_FENCE)) {
    let value
    try {
      value = JSON.parse(match[1])
    } catch (error) {
      throw new Error(`${source}: invalid chronica-contract JSON: ${error.message}`)
    }
    contracts.push({ ...value, __source: source })
  }
  return contracts
}

// Used only for topology/audit tests. Contract loading itself is closed-world and
// uses canonicalArchitectureOwners below rather than trusting every tracked file.
export function trackedArchitectureDocs(root = process.cwd()) {
  const output = execFileSync('git', ['ls-files', 'docs/architecture'], {
    cwd: root,
    encoding: 'utf8',
  })
  return output.split('\n').map((line) => line.trim()).filter((line) => line.endsWith('.md'))
}

export function loadArchitectureContracts(root = process.cwd()) {
  const docs = [...canonicalArchitectureOwners]
  const contracts = []
  for (const file of docs) {
    const text = readFileSync(resolve(root, file), 'utf8')
    contracts.push(...parseContractsFromText(text, file))
  }
  return { docs, contracts }
}

export function validateContract(contract) {
  const errors = []
  const label = contract?.id || '<missing-id>'
  push(errors, typeof contract === 'object' && contract !== null, `${label}: contract must be an object`)
  if (!contract || typeof contract !== 'object') return errors

  push(errors, typeof contract.id === 'string' && CONTRACT_ID.test(contract.id), `${label}: invalid id`)
  push(errors, CONTRACT_KINDS.has(contract.kind), `${label}: kind must be equation|invariant|performance`)
  push(errors, typeof contract.owner === 'string' && contract.owner.trim().length > 0, `${label}: owner is required`)
  push(errors, SEVERITIES.has(contract.severity), `${label}: severity must be hard|guardrail|advisory`)
  push(errors, Array.isArray(contract.runtime_owner) && contract.runtime_owner.length > 0 && nonEmptyStrings(contract.runtime_owner), `${label}: runtime_owner must be a non-empty string array`)
  push(errors, Array.isArray(contract.verification) && contract.verification.length > 0 && nonEmptyStrings(contract.verification), `${label}: verification must be a non-empty string array`)

  if (Array.isArray(contract.runtime_owner)) {
    for (const owner of contract.runtime_owner) {
      if (typeof owner !== 'string') continue
      push(errors, safeRepoRelativePath(owner), `${label}: runtime_owner '${owner}' must be a safe repository-relative path`)
      push(errors, OWNER_PREFIXES.some((prefix) => owner.startsWith(prefix)), `${label}: runtime_owner '${owner}' is outside approved repository responsibilities`)
    }
  }

  if (contract.kind === 'equation') {
    push(errors, contract.id?.startsWith('ARCH-EQ-'), `${label}: equation id must start ARCH-EQ-`)
    push(errors, typeof contract.equation === 'string' && contract.equation.trim().length > 0, `${label}: equation text is required`)
  }

  if (contract.kind === 'invariant') {
    push(errors, contract.id?.startsWith('INV-'), `${label}: invariant id must start INV-`)
    push(errors, typeof contract.predicate === 'string' && contract.predicate.trim().length > 0, `${label}: predicate is required`)
  }

  if (contract.kind === 'performance') {
    push(errors, contract.id?.startsWith('ARCH-PERF-'), `${label}: performance id must start ARCH-PERF-`)
    push(errors, PERFORMANCE_GATES.has(contract.gate), `${label}: gate must be STRUCTURAL|MEASURED|PRODUCTION`)
    push(errors, typeof contract.metric === 'string' && contract.metric.trim().length > 0, `${label}: metric is required`)
    push(errors, contract.direction === 'max' || contract.direction === 'min', `${label}: direction must be max|min`)
    push(errors, Array.isArray(contract.paths) && contract.paths.length > 0 && nonEmptyStrings(contract.paths), `${label}: paths must be a non-empty string array`)
    push(errors, Array.isArray(contract.hard_invariants) && contract.hard_invariants.length > 0 && nonEmptyStrings(contract.hard_invariants), `${label}: hard_invariants must be a non-empty string array`)

    if (Array.isArray(contract.paths)) {
      for (const path of contract.paths) {
        if (typeof path !== 'string') continue
        push(errors, safeRepoRelativePath(path), `${label}: performance path '${path}' must be a safe repository-relative path`)
      }
    }

    if (contract.gate === 'MEASURED' || contract.gate === 'PRODUCTION') {
      push(errors, contract.threshold && typeof contract.threshold === 'object', `${label}: measured/production contract requires threshold`)
      if (contract.threshold && typeof contract.threshold === 'object') {
        push(errors, ['<=', '>='].includes(contract.threshold.op), `${label}: threshold.op must be <= or >=`)
        push(errors, Number.isFinite(contract.threshold.value), `${label}: threshold.value must be finite`)
        push(errors, typeof contract.threshold.unit === 'string' && contract.threshold.unit.length > 0, `${label}: threshold.unit is required`)
      }
      push(errors, contract.ci && typeof contract.ci === 'object', `${label}: measured/production contract requires ci object`)
      push(errors, typeof contract.ci?.command === 'string' && contract.ci.command.trim().length > 0, `${label}: measured/production contract requires ci.command`)
      push(errors, typeof contract.fixture === 'string' && contract.fixture.trim().length > 0, `${label}: measured/production contract requires fixture/workload identity`)
      push(errors, typeof contract.baseline === 'string' && contract.baseline.trim().length > 0, `${label}: measured/production contract requires baseline identity`)
    }

    if (contract.regression_ratio_max != null) {
      push(errors, Number.isFinite(contract.regression_ratio_max) && contract.regression_ratio_max >= 1, `${label}: regression_ratio_max must be >= 1`)
      push(errors, typeof contract.ci?.baseline_command === 'string' && contract.ci.baseline_command.trim().length > 0, `${label}: regression ratio requires ci.baseline_command`)
    }
  }

  return errors
}

export function validateContractSet(contracts) {
  const errors = []
  const byId = new Map()
  for (const contract of contracts) {
    errors.push(...validateContract(contract).map((error) => `${contract.__source}: ${error}`))

    if (isCanonicalArchitectureOwner(contract.__source)) {
      const expectedOwner = architectureOwnerIdFromPath(contract.__source)
      push(
        errors,
        contract.owner === expectedOwner,
        `${contract.__source}: ${contract.id ?? '<missing-id>'}: owner '${contract.owner ?? '<missing>'}' must match canonical document owner '${expectedOwner}'`,
      )
    }

    if (!contract?.id) continue
    if (byId.has(contract.id)) errors.push(`${contract.id}: duplicate contract in ${byId.get(contract.id).__source} and ${contract.__source}`)
    else byId.set(contract.id, contract)
  }

  for (const contract of contracts.filter((row) => row.kind === 'performance')) {
    for (const invariant of list(contract.hard_invariants)) {
      const target = byId.get(invariant)
      push(errors, Boolean(target), `${contract.__source}: ${contract.id}: hard invariant ${invariant} is not declared by an architecture contract`)
      if (target) {
        push(errors, target.kind === 'invariant', `${contract.__source}: ${contract.id}: hard invariant ${invariant} must reference kind=invariant, found ${target.kind}`)
        push(errors, target.severity === 'hard', `${contract.__source}: ${contract.id}: hard invariant ${invariant} must reference severity=hard, found ${target.severity}`)
      }
    }
  }

  return errors
}

export function changedPaths(root, base, head) {
  if (!base || !head) return []
  const output = execFileSync('git', ['diff', '--name-only', `${base}...${head}`], {
    cwd: root,
    encoding: 'utf8',
  })
  return output.split('\n').map((line) => line.trim()).filter(Boolean)
}

export function affectsContract(contract, paths) {
  if (contract.kind !== 'performance') return false
  return paths.some((path) => list(contract.paths).some((prefix) => path === prefix || path.startsWith(prefix)))
}
