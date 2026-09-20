#!/usr/bin/env node

import { execFileSync } from 'node:child_process'
import {
  affectsContract,
  changedPaths,
  loadArchitectureContracts,
  validateContractSet,
} from '../ci/architecture-contract-lib.mjs'

function arg(name) {
  const index = process.argv.indexOf(name)
  return index >= 0 ? process.argv[index + 1] : undefined
}

function parseMeasurement(stdout, id) {
  const lines = String(stdout).trim().split('\n').map((line) => line.trim()).filter(Boolean)
  if (!lines.length) throw new Error(`${id}: benchmark emitted no measurement`)
  const last = lines.at(-1)
  try {
    const parsed = JSON.parse(last)
    if (!Number.isFinite(parsed.value)) throw new Error('value is not finite')
    return parsed.value
  } catch (error) {
    const numeric = Number(last)
    if (Number.isFinite(numeric)) return numeric
    throw new Error(`${id}: benchmark must emit a final JSON line {"value": number} or a numeric line (${error.message})`)
  }
}

function passesThreshold(value, threshold) {
  if (threshold.op === '<=') return value <= threshold.value
  if (threshold.op === '>=') return value >= threshold.value
  return false
}

function runCommand(command, env = {}) {
  return execFileSync('bash', ['-lc', command], {
    cwd: process.cwd(),
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'inherit'],
    env: { ...process.env, ...env },
  })
}

const base = arg('--base')
const head = arg('--head')
const explicitPaths = process.env.CHRONICA_CHANGED_PATHS
  ? process.env.CHRONICA_CHANGED_PATHS.split('\n').map((line) => line.trim()).filter(Boolean)
  : null

const { contracts } = loadArchitectureContracts(process.cwd())
const errors = validateContractSet(contracts)
if (errors.length) {
  console.error('Architecture performance CI cannot run because contracts are invalid.')
  for (const error of errors) console.error(`- ${error}`)
  process.exit(1)
}

const paths = explicitPaths ?? changedPaths(process.cwd(), base, head)
const performance = contracts.filter((row) => row.kind === 'performance')
const affected = performance.filter((contract) => affectsContract(contract, paths))

console.log(`Architecture performance contracts affected: ${affected.length}/${performance.length}`)
if (paths.length) console.log(`Changed paths considered: ${paths.length}`)

let failed = false
for (const contract of affected) {
  console.log(`\n[${contract.gate}] ${contract.id} ${contract.metric}`)
  if (contract.gate === 'STRUCTURAL') {
    console.log('  structural contract only: benchmark/threshold not yet claimed; no fabricated performance verdict')
    continue
  }

  try {
    const candidateStdout = runCommand(contract.ci.command, {
      CHRONICA_BENCH_ROLE: 'candidate',
      CHRONICA_BENCH_BASE: base ?? '',
      CHRONICA_BENCH_HEAD: head ?? '',
    })
    const candidate = parseMeasurement(candidateStdout, contract.id)
    const thresholdPass = passesThreshold(candidate, contract.threshold)
    console.log(`  candidate=${candidate} ${contract.threshold.unit}; threshold ${contract.threshold.op} ${contract.threshold.value}`)
    if (!thresholdPass) {
      console.error(`  FAIL: absolute performance budget violated`)
      failed = true
    }

    if (contract.regression_ratio_max != null) {
      const baselineStdout = runCommand(contract.ci.baseline_command, {
        CHRONICA_BENCH_ROLE: 'baseline',
        CHRONICA_BENCH_BASE: base ?? '',
        CHRONICA_BENCH_HEAD: head ?? '',
      })
      const baseline = parseMeasurement(baselineStdout, contract.id)
      if (baseline <= 0 || candidate < 0) throw new Error(`${contract.id}: regression comparison requires non-negative candidate and positive baseline`)
      const ratio = contract.direction === 'max' ? candidate / baseline : baseline / candidate
      console.log(`  baseline=${baseline}; regression ratio=${ratio.toFixed(4)} <= ${contract.regression_ratio_max}`)
      if (ratio > contract.regression_ratio_max) {
        console.error('  FAIL: regression envelope violated')
        failed = true
      }
    }
  } catch (error) {
    console.error(`  FAIL: ${error.message}`)
    failed = true
  }
}

if (failed) process.exit(1)
console.log('\nArchitecture performance CI OK.')
