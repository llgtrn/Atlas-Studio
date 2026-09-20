#!/usr/bin/env node
// Validates docs/_machine/benchmark-family-to-fic-map.json against the 11 families and 55 FIC keys.
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import { loadRegistry } from './query-benchmark-ratio.mjs'
import { denominatorKeys, loadV1Denominator } from './fic-recompute-lib.mjs'

const ROOT = path.resolve(dirname(fileURLToPath(import.meta.url)), '..', '..')

export function loadMap(root = ROOT) {
  return JSON.parse(readFileSync(join(root, 'docs/_machine/benchmark-family-to-fic-map.json'), 'utf8'))
}

export function validateFamilyToFicMap(doc, { familyKeys, ficKeys }) {
  const errors = []
  if (doc.schema !== 'chronica.benchmark_family_to_fic_map.v1') errors.push('invalid schema')
  const families = doc.families || {}
  for (const key of familyKeys) {
    if (!families[key]) errors.push(`skipped_family:${key}`)
  }
  for (const key of Object.keys(families)) {
    if (!familyKeys.includes(key)) errors.push(`unknown_family:${key}`)
    for (const fic of families[key] || []) {
      if (!ficKeys.includes(fic)) errors.push(`${key}:unknown_fic:${fic}`)
    }
  }
  return { ok: errors.length === 0, errors }
}

function main() {
  const registry = loadRegistry()
  const v1 = loadV1Denominator(ROOT)
  const doc = loadMap()
  const result = validateFamilyToFicMap(doc, {
    familyKeys: registry.families.map((f) => f.family_key),
    ficKeys: denominatorKeys(v1),
  })
  console.log(`benchmark-to-fic-map: families=${Object.keys(doc.families || {}).length}`)
  if (!result.ok) {
    console.error(result.errors.join('\n'))
    process.exitCode = 1
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
