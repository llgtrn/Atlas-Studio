#!/usr/bin/env node
// reconcile-value-flow.mjs — apply committed value-flow.json into caps.db.value_node + value_edge (DIM 12).
// Synthesizes nodes from distinct flow endpoints (id = kind:slug(label)) and from the orphans list
// (capability nodes with NO outbound edge, so the orphan-detection compute flags them). Idempotent;
// JSON authoritative; rebuilds the node/edge sets from scratch each run.
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const { flows, orphans } = JSON.parse(readFileSync(join(process.cwd(), 'tools', 'capabilities', 'value-flow.json'), 'utf8'))
if (!Array.isArray(flows)) { console.error('value-flow.json: missing "flows"'); process.exit(1) }
const KINDS = new Set(['capability', 'user_value', 'business_value', 'revenue'])
const slug = (s) => String(s).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 70)
const nodeId = (kind, label) => `${kind}:${slug(label)}`

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
db.exec(`CREATE TABLE IF NOT EXISTS value_node (id TEXT PRIMARY KEY, kind TEXT NOT NULL, label TEXT NOT NULL, notes TEXT);
  CREATE TABLE IF NOT EXISTS value_edge (id TEXT PRIMARY KEY, from_id TEXT NOT NULL, to_id TEXT NOT NULL, weight INTEGER DEFAULT 3, rationale TEXT)`)

const bad = flows.filter(f => !KINDS.has(f.from_kind) || !KINDS.has(f.to_kind) || !f.from_label || !f.to_label)
if (bad.length) { console.error(`REFUSED — ${bad.length} invalid flow(s) (need valid from/to kind + label)`); db.close(); process.exit(1) }

const nodes = new Map()  // id -> {id,kind,label,notes}
const addNode = (kind, label, notes) => { const id = nodeId(kind, label); if (!nodes.has(id)) nodes.set(id, { id, kind, label, notes: notes ?? null }); return id }
const edges = new Map()  // id -> {id,from_id,to_id,weight,rationale}
for (const f of flows) {
  const from = addNode(f.from_kind, f.from_label), to = addNode(f.to_kind, f.to_label)
  const id = `${from}|${to}`
  edges.set(id, { id, from_id: from, to_id: to, weight: f.weight ?? 3, rationale: f.rationale ?? null })
}
// orphans: capability areas with NO honest path to revenue — add as edge-less capability nodes so the
// compute (capability node not reaching any revenue node) flags them.
for (const o of (orphans ?? [])) addNode('capability', o, 'orphan-flagged: no value path identified')

const upN = db.prepare('INSERT INTO value_node (id,kind,label,notes) VALUES (@id,@kind,@label,@notes) ON CONFLICT(id) DO UPDATE SET kind=excluded.kind, label=excluded.label, notes=excluded.notes')
const upE = db.prepare('INSERT INTO value_edge (id,from_id,to_id,weight,rationale) VALUES (@id,@from_id,@to_id,@weight,@rationale) ON CONFLICT(id) DO UPDATE SET from_id=excluded.from_id, to_id=excluded.to_id, weight=excluded.weight, rationale=excluded.rationale')
const tx = db.transaction(() => {
  for (const n of nodes.values()) upN.run(n)
  for (const e of edges.values()) upE.run(e)
  for (const r of db.prepare('SELECT id FROM value_node').all()) if (!nodes.has(r.id)) db.prepare('DELETE FROM value_node WHERE id=?').run(r.id)
  for (const r of db.prepare('SELECT id FROM value_edge').all()) if (!edges.has(r.id)) db.prepare('DELETE FROM value_edge WHERE id=?').run(r.id)
})
tx()
db.close()
console.log(`reconcile-value-flow: applied ${nodes.size} value-nodes + ${edges.size} edges (${(orphans ?? []).length} orphan capability area(s)).`)
