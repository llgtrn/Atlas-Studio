#!/usr/bin/env node
// Evaluate Agent Employment Contract records. Does not mutate product crates or CAP16.

import { readdirSync, readFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { evaluateEmploymentRecord } from './employment.mjs'
import { EMPLOYMENT_STANDARD_VERSION } from './employment-vocab.mjs'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..')
const FIX = join(ROOT, 'tools/benchmark/engine/fixtures/employment')

function arg(args, name) {
  const idx = args.indexOf(name)
  return idx >= 0 ? args[idx + 1] : undefined
}

function loadJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'))
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

  const results = paths.map((path) => ({ path, ...evaluateEmploymentRecord(loadJson(path)) }))
  const summary = results.map((row) => ({
    file: row.path.replace(`${ROOT}/`, ''),
    case_id: row.case_id,
    worker_class: row.worker_class,
    employment_maturity: row.employment_maturity,
    claimed_employment_maturity: row.claimed_employment_maturity,
    verdict: row.verdict,
    hard_gate_failures: row.hard_gate_failures,
    hard_failures: row.hard_failures,
    anti_patterns_found: row.anti_patterns_found,
  }))

  console.log(
    JSON.stringify(
      {
        standard_version: EMPLOYMENT_STANDARD_VERSION,
        ok: results.every((row) => row.errors?.length === 0),
        evaluated: results.length,
        summary,
      },
      null,
      2,
    ),
  )
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
