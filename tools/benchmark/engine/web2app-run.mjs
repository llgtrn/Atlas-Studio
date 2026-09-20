#!/usr/bin/env node
// Evaluate WEB2APP records. Does not mutate canonical capability rows or product crates.

import { readdirSync, readFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { evaluateWeb2AppRecord, validateFixedPointReport } from './web2app.mjs'
import { WEB2APP_STANDARD_VERSION } from './web2app-vocab.mjs'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..')
const FIX = join(ROOT, 'tools/benchmark/engine/fixtures/web2app')

function arg(args, name) {
  const idx = args.indexOf(name)
  return idx >= 0 ? args[idx + 1] : undefined
}

function loadJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'))
}

function evaluatePath(path) {
  const record = loadJson(path)
  if (record.schema === 'chronica.web2app.fixed-point.v1' || record.pass1) {
    return { path, ...validateFixedPointReport(record) }
  }
  return { path, ...evaluateWeb2AppRecord(record) }
}

function main() {
  const args = process.argv.slice(2)
  const file = arg(args, '--file')
  const fixtures = args.includes('--fixtures') || (!file && args.length === 0)
  const paths = file
    ? [resolve(file)]
    : fixtures
      ? readdirSync(FIX)
          .filter((name) => name.endsWith('.json'))
          .map((name) => join(FIX, name))
      : args.filter((a) => !a.startsWith('--'))

  const results = paths.map(evaluatePath)
  const failed = results.filter((row) => row.hard_gate_failures?.length || (row.errors && row.errors.length && !row.verdict))
  const summary = results.map((row) => ({
    file: row.path.replace(`${ROOT}/`, ''),
    capability_key: row.capability_key,
    web2app_level: row.web2app_level,
    observation_maturity: row.observation_maturity,
    verdict: row.verdict,
    hard_gate_failures: row.hard_gate_failures,
    anti_patterns_found: row.anti_patterns_found,
    score: row.score?.total,
    contradictions: row.contradictions,
  }))

  console.log(
    JSON.stringify(
      {
        standard_version: WEB2APP_STANDARD_VERSION,
        ok: results.every((row) => row.errors?.length === 0),
        evaluated: results.length,
        summary,
      },
      null,
      2,
    ),
  )
  process.exitCode = 0
  if (failed.some((row) => row.errors?.length && row.verdict !== 'PARTIAL' && row.verdict !== 'PASS' && row.verdict !== 'FAIL' && row.verdict !== 'FIXED_POINT_INVALID' && row.verdict !== 'FIXED_POINT_VALID')) {
    process.exitCode = 1
  }
  if (results.some((row) => (row.errors || []).length > 0 && row.ok === false && row.verdict !== 'FAIL' && row.verdict !== 'PARTIAL' && row.verdict !== 'FIXED_POINT_INVALID')) {
    process.exitCode = 1
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
