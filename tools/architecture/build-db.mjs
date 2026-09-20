#!/usr/bin/env node
import Database from 'better-sqlite3'
import { join } from 'node:path'
import { buildArchitectureDb } from './architecture-lib.mjs'

const root = process.cwd()
const dbPath = join(root, 'docs', 'architecture.db')
const result = buildArchitectureDb({ root, dbPath })
const db = new Database(dbPath, { readonly: true })
const evidenceRows = db.prepare('SELECT count(*) n FROM architecture_evidence').get().n
db.close()

console.log(`architecture-db: wrote ${dbPath}`)
console.log(`  nodes: ${result.nodes.length}`)
console.log(`  edges: ${result.edges.length}`)
console.log(`  invariants: ${result.invariants.length}`)
console.log(`  authority rules: ${result.authorityRules.length}`)
console.log(`  data flows: ${result.dataFlows.length}`)
console.log(`  capability links: ${result.capabilityLinks}`)
console.log(`  evidence rows: ${evidenceRows}`)
