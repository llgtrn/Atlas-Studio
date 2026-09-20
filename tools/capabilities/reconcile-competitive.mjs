#!/usr/bin/env node
// reconcile-competitive.mjs — apply committed competitive.json into caps.db.competitor + competitor_feature
// (DIM 13). Validates that every feature points at a known competitor. Idempotent; JSON authoritative.
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const { competitors, features } = JSON.parse(readFileSync(join(process.cwd(), 'tools', 'capabilities', 'competitive.json'), 'utf8'))
if (!Array.isArray(competitors) || !Array.isArray(features)) { console.error('competitive.json: need "competitors" + "features"'); process.exit(1) }
const MARKETS = new Set(['browser', 'ai_ide', 'agent_platform', 'company_os', 'erp', 'crm'])
const RELS = new Set(['gap', 'parity', 'advantage', 'differentiation'])

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
db.exec(`CREATE TABLE IF NOT EXISTS competitor (id TEXT PRIMARY KEY, name TEXT NOT NULL, market TEXT, notes TEXT);
  CREATE TABLE IF NOT EXISTS competitor_feature (id TEXT PRIMARY KEY, competitor_id TEXT NOT NULL, feature TEXT NOT NULL, our_capability_ref TEXT, relation TEXT, notes TEXT)`)

const compIds = new Set(competitors.map(c => c.id))
const bad = []
for (const c of competitors) if (!c.id || !c.name || (c.market && !MARKETS.has(c.market))) bad.push(`bad competitor ${c.id}`)
for (const f of features) {
  if (!f.id || !f.competitor_id || !f.feature) bad.push(`bad feature ${f.id}`)
  if (f.relation && !RELS.has(f.relation)) bad.push(`bad relation '${f.relation}' on ${f.id}`)
  if (!compIds.has(f.competitor_id)) bad.push(`feature ${f.id} -> unknown competitor ${f.competitor_id}`)
}
if (bad.length) { console.error(`REFUSED — ${bad.length} invalid:\n  ` + bad.join('\n  ')); db.close(); process.exit(1) }

const upC = db.prepare('INSERT INTO competitor (id,name,market,notes) VALUES (@id,@name,@market,@notes) ON CONFLICT(id) DO UPDATE SET name=excluded.name, market=excluded.market, notes=excluded.notes')
const upF = db.prepare('INSERT INTO competitor_feature (id,competitor_id,feature,our_capability_ref,relation,notes) VALUES (@id,@competitor_id,@feature,@our_capability_ref,@relation,@notes) ON CONFLICT(id) DO UPDATE SET competitor_id=excluded.competitor_id, feature=excluded.feature, our_capability_ref=excluded.our_capability_ref, relation=excluded.relation, notes=excluded.notes')
const keepC = new Set(competitors.map(c => c.id)), keepF = new Set(features.map(f => f.id))
const tx = db.transaction(() => {
  for (const c of competitors) upC.run({ id: c.id, name: c.name, market: c.market ?? null, notes: c.notes ?? null })
  for (const f of features) upF.run({ id: f.id, competitor_id: f.competitor_id, feature: f.feature, our_capability_ref: f.our_capability_ref ?? null, relation: f.relation ?? null, notes: f.notes ?? null })
  for (const r of db.prepare('SELECT id FROM competitor').all()) if (!keepC.has(r.id)) db.prepare('DELETE FROM competitor WHERE id=?').run(r.id)
  for (const r of db.prepare('SELECT id FROM competitor_feature').all()) if (!keepF.has(r.id)) db.prepare('DELETE FROM competitor_feature WHERE id=?').run(r.id)
})
tx()
const byRel = Object.fromEntries(db.prepare('SELECT relation, count(*) n FROM competitor_feature GROUP BY relation ORDER BY n DESC').all().map(r => [r.relation, r.n]))
db.close()
console.log(`reconcile-competitive: applied ${competitors.length} competitors + ${features.length} features. by relation: ${JSON.stringify(byRel)}`)
