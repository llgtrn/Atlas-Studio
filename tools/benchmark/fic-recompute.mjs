#!/usr/bin/env node
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import {
  computeFic,
  diffLevels,
  loadJson,
  loadV1Denominator,
  validateCensusShape,
} from './fic-recompute-lib.mjs'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..')
const DEFAULT_CENSUS = join(ROOT, 'docs/_machine/fic-revalidation-census.json')

export function recompute({ root = ROOT, censusPath = DEFAULT_CENSUS } = {}) {
  const v1 = loadV1Denominator(root)
  const census = loadJson(censusPath)
  const shape = validateCensusShape(census, v1)
  const fic = computeFic(census)
  const changes = diffLevels(v1, census)
  return { shape, fic, changes, census, v1 }
}

function printSummary(result) {
  const { fic, shape, changes } = result
  console.log(`fic-recompute: denominator=${fic.denominator} FIC=${fic.l3_or_higher}/${fic.denominator} (${fic.fic_percent}%) target=50/55`)
  for (const level of ['L0', 'L1', 'L2', 'L3', 'L4', 'L5', 'L6']) {
    console.log(`  ${level}: ${fic.counts[level]}`)
  }
  console.log(`  PEC: ${fic.pec}`)
  console.log(`  CSE: ${fic.cse}`)
  console.log(`  level_changes: ${changes.length}`)
  if (!shape.ok) {
    console.error(`fic-recompute: CENSUS_INVALID (${shape.errors.length} errors)`)
    for (const err of shape.errors.slice(0, 40)) console.error(`  ${err}`)
    if (shape.errors.length > 40) console.error(`  ... ${shape.errors.length - 40} more`)
  }
}

function main() {
  const args = process.argv.slice(2)
  const json = args.includes('--json')
  const outIdx = args.indexOf('--out')
  const censusIdx = args.indexOf('--census')
  const censusPath = censusIdx >= 0 ? args[censusIdx + 1] : DEFAULT_CENSUS
  const result = recompute({ censusPath })
  if (json) console.log(JSON.stringify({ ...result, fic: { ...result.fic, rows: undefined }, census: undefined, v1: undefined }, null, 2))
  else printSummary(result)
  if (outIdx >= 0) {
    writeFileSync(args[outIdx + 1], `${JSON.stringify({ generated_at_note: 'not a product truth surface', ...result, census: undefined, v1: undefined, fic: { ...result.fic, rows: result.fic.rows.map((r) => ({ domain: r.domain, previous_level: r.previous_level, new_level: r.new_level, applied: r.applied })) } }, null, 2)}\n`)
  }
  process.exitCode = result.shape.ok ? 0 : 1
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
