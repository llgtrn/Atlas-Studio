#!/usr/bin/env node
// reconcile-strategy-edges.mjs — apply the committed strategy-edges.json into caps.db.strategy_edge
// (DIM 5: the strategy graph). Computes id = from|relation|to, validates that opportunity/issue/pillar
// endpoints exist (a dangling capability ref is allowed — it may be unbuilt), and removes stale edges.
// caps.db is gitignored; strategy-edges.json is the durable record. Run after reconcile-opportunities/
// -tracking-issues/-end-state so the endpoint validation sees the current objects.
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const RELATIONS = new Set(['unlocks', 'depends_on', 'blocks', 'obsoletes', 'contributes_to', 'relates'])
const KINDS = new Set(['opportunity', 'capability', 'issue', 'donor', 'pillar'])
const { edges } = JSON.parse(readFileSync(join(process.cwd(), 'tools', 'capabilities', 'strategy-edges.json'), 'utf8'))
if (!Array.isArray(edges)) { console.error('strategy-edges.json: missing "edges"'); process.exit(1) }

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
db.exec(`CREATE TABLE IF NOT EXISTS strategy_edge (
  id TEXT PRIMARY KEY, from_kind TEXT NOT NULL, from_id TEXT NOT NULL, relation TEXT NOT NULL,
  to_kind TEXT NOT NULL, to_id TEXT NOT NULL, weight INTEGER DEFAULT 3, rationale TEXT, source TEXT, created_at TEXT)`)

// endpoint existence sets (for validation; capability refs are NOT required to exist — may be unbuilt)
const oppIds = new Set(db.prepare('SELECT id FROM opportunity').all().map(r => r.id))
const hasTable = (n) => db.prepare("SELECT count(*) c FROM sqlite_master WHERE type='table' AND name=?").get(n).c > 0
const issueIds = new Set(hasTable('tracking_issue') ? db.prepare('SELECT id FROM tracking_issue').all().map(r => r.id) : [])
const pillarIds = new Set(hasTable('end_state_pillar') ? db.prepare('SELECT id FROM end_state_pillar').all().map(r => r.id) : [])
const exists = (kind, id) => kind === 'opportunity' ? oppIds.has(id) : kind === 'issue' ? issueIds.has(id) : kind === 'pillar' ? pillarIds.has(id) : true // capability/donor not validated

const bad = []
for (const e of edges) {
  if (!KINDS.has(e.from_kind) || !KINDS.has(e.to_kind)) bad.push(`bad kind on ${e.from_id}->${e.to_id}`)
  if (!RELATIONS.has(e.relation)) bad.push(`bad relation '${e.relation}' on ${e.from_id}->${e.to_id}`)
  if (!exists(e.from_kind, e.from_id)) bad.push(`dangling from ${e.from_kind}:${e.from_id}`)
  if (!exists(e.to_kind, e.to_id)) bad.push(`dangling to ${e.to_kind}:${e.to_id}`)
}
if (bad.length) { console.error(`REFUSED — ${bad.length} invalid edge(s):\n  ` + bad.join('\n  ')); db.close(); process.exit(1) }

const upsert = db.prepare(`INSERT INTO strategy_edge (id,from_kind,from_id,relation,to_kind,to_id,weight,rationale,source,created_at)
  VALUES (@id,@from_kind,@from_id,@relation,@to_kind,@to_id,@weight,@rationale,@source,@created_at)
  ON CONFLICT(id) DO UPDATE SET from_kind=excluded.from_kind, from_id=excluded.from_id, relation=excluded.relation,
    to_kind=excluded.to_kind, to_id=excluded.to_id, weight=excluded.weight, rationale=excluded.rationale, source=excluded.source`)
const keep = new Set()
const tx = db.transaction(() => {
  for (const e of edges) {
    const id = `${e.from_id}|${e.relation}|${e.to_id}`; keep.add(id)
    upsert.run({ id, from_kind: e.from_kind, from_id: e.from_id, relation: e.relation, to_kind: e.to_kind, to_id: e.to_id,
      weight: e.weight ?? 3, rationale: e.rationale ?? null, source: e.source ?? 'seed', created_at: e.created_at ?? '2026-06-19' })
  }
  for (const r of db.prepare('SELECT id FROM strategy_edge').all()) if (!keep.has(r.id)) db.prepare('DELETE FROM strategy_edge WHERE id=?').run(r.id)
})
tx()

const byRel = Object.fromEntries(db.prepare('SELECT relation, count(*) n FROM strategy_edge GROUP BY relation ORDER BY n DESC').all().map(r => [r.relation, r.n]))
db.close()
console.log(`reconcile-strategy-edges: applied ${edges.length} edges. by relation: ${JSON.stringify(byRel)}`)
