#!/usr/bin/env node

import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'

const root = process.cwd()
const declarationDir = join(root, 'bindings', 'core')
const required = [
  'pattern', 'legacy_key', 'legacy_provider', 'graph_native_implementation',
  'implementation_version', 'availability', 'migration_status', 'migration_ledger_entry',
]
const forbiddenTopLevel = ['provider', 'provider_node_id', 'node_id']
const errors = []
const patterns = new Map()
const legacyKeys = new Map()

for (const file of readdirSync(declarationDir).filter((name) => name.endsWith('.json')).sort()) {
  const path = join(declarationDir, file)
  let value
  try {
    value = JSON.parse(readFileSync(path, 'utf8'))
  } catch (error) {
    errors.push(`${file}: invalid JSON: ${error.message}`)
    continue
  }

  for (const field of required) {
    if (!(field in value)) errors.push(`${file}: missing required BindingDeclaration field '${field}'`)
  }
  for (const field of forbiddenTopLevel) {
    if (field in value) errors.push(`${file}: static BindingDeclaration must not carry live runtime field '${field}'`)
  }

  if (typeof value.pattern !== 'string' || !value.pattern.trim()) errors.push(`${file}: pattern must be a non-empty string`)
  if (typeof value.legacy_key !== 'string' || !value.legacy_key.trim()) errors.push(`${file}: legacy_key must be a non-empty string`)
  if (typeof value.legacy_provider !== 'object' || value.legacy_provider == null || Array.isArray(value.legacy_provider)) errors.push(`${file}: legacy_provider must be an object`)
  if (typeof value.graph_native_implementation !== 'object' || value.graph_native_implementation == null || Array.isArray(value.graph_native_implementation)) errors.push(`${file}: graph_native_implementation must be an object`)

  if (typeof value.pattern === 'string') {
    const prior = patterns.get(value.pattern)
    if (prior) errors.push(`${file}: duplicate pattern '${value.pattern}' already declared by ${prior}`)
    else patterns.set(value.pattern, file)
  }
  if (typeof value.legacy_key === 'string') {
    const prior = legacyKeys.get(value.legacy_key)
    if (prior) errors.push(`${file}: duplicate legacy_key '${value.legacy_key}' already declared by ${prior}`)
    else legacyKeys.set(value.legacy_key, file)
  }

  const ledger = value.migration_ledger_entry
  if (typeof ledger === 'string' && !ledger.startsWith('graph/migrations/legacy-cap/')) {
    errors.push(`${file}: migration_ledger_entry must resolve under graph/migrations/legacy-cap/`)
  }
}

if (errors.length) {
  console.error('Chronica BindingDeclaration contract FAILED.')
  for (const error of errors) console.error(`\n- ${error}`)
  process.exit(1)
}

console.log(`Chronica BindingDeclaration contract OK (${patterns.size} declarations).`)
