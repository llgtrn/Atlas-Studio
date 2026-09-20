#!/usr/bin/env node
// benchmark-source-topology.mjs — classify existing benchmark surfaces. Does not invent a new atlas.
import { readFileSync, existsSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

const ROOT = path.resolve(dirname(fileURLToPath(import.meta.url)), '..', '..')

const CLASSES = new Set([
  'CANONICAL_BINDING',
  'OPERATING_DOCTRINE',
  'ROUTING_INDEX',
  'MACHINE_DATA',
  'DERIVED_SNAPSHOT',
  'STALE',
  'ARCHIVED',
])

export function loadTopology(root = ROOT) {
  const p = join(root, 'docs/_machine/benchmark-source-topology.json')
  return JSON.parse(readFileSync(p, 'utf8'))
}

export function validateTopology(doc) {
  const errors = []
  if (doc.schema !== 'chronica.benchmark_source_topology.v1') errors.push('invalid schema')
  const seen = new Set()
  for (const row of doc.sources || []) {
    if (!CLASSES.has(row.class)) errors.push(`${row.path}: invalid class ${row.class}`)
    if (seen.has(row.path)) errors.push(`duplicate path ${row.path}`)
    seen.add(row.path)
    if (!row.role) errors.push(`${row.path}: missing role`)
  }
  return { ok: errors.length === 0, errors }
}

function main() {
  const doc = loadTopology()
  const result = validateTopology(doc)
  console.log(`benchmark-source-topology: ${doc.sources.length} sources`)
  const by = {}
  for (const row of doc.sources) by[row.class] = (by[row.class] || 0) + 1
  for (const [k, v] of Object.entries(by).sort()) console.log(`  ${k}: ${v}`)
  if (!result.ok) {
    console.error(result.errors.join('\n'))
    process.exitCode = 1
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
