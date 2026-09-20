#!/usr/bin/env node
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import { diffLevels, loadJson, loadV1Denominator } from './fic-recompute-lib.mjs'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..')

function main() {
  const censusPath = process.argv[2] || path.join(ROOT, 'docs/_machine/fic-revalidation-census.json')
  const v1 = loadV1Denominator(ROOT)
  const census = loadJson(censusPath)
  const changes = diffLevels(v1, census)
  console.log(`fic-diff: ${changes.length} level change(s) vs v1.json`)
  for (const change of changes) {
    console.log(`  ${change.direction} ${change.domain}: ${change.previous_level} -> ${change.new_level}`)
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
