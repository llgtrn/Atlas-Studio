#!/usr/bin/env node
// reconcile-reality-path.mjs — REALITY LEVERAGE. Apply reality-path.json: the L4 dependency graph
// (ordered milestones to the first real-world L4) into reality_milestone, and the per-opportunity
// expected_new_L4 map onto opportunity.expected_l4. Run AFTER reconcile-opportunities (so the opportunity
// rows exist to stamp). Idempotent; JSON authoritative; resets expected_l4 to 0 then sets the mapped ones.
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const { milestones, opportunity_l4 } = JSON.parse(readFileSync(join(process.cwd(), 'tools', 'capabilities', 'reality-path.json'), 'utf8'))
if (!Array.isArray(milestones)) { console.error('reality-path.json: missing "milestones"'); process.exit(1) }
const STATUSES = new Set(['done', 'in_progress', 'todo', 'blocked'])

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
db.exec(`CREATE TABLE IF NOT EXISTS reality_milestone (id TEXT PRIMARY KEY, ordinal INTEGER, label TEXT NOT NULL, description TEXT,
  status TEXT DEFAULT 'todo', effort INTEGER, produces_l4 INTEGER DEFAULT 0, delivered_by TEXT, blocked_by TEXT, would_create_event TEXT, notes TEXT)`)

const bad = milestones.filter(m => !m.id || !m.label || (m.status && !STATUSES.has(m.status)))
if (bad.length) { console.error(`REFUSED — ${bad.length} invalid milestone(s) (need id+label, valid status)`); db.close(); process.exit(1) }

const upsert = db.prepare(`INSERT INTO reality_milestone (id,ordinal,label,description,status,effort,produces_l4,delivered_by,blocked_by,would_create_event,notes)
  VALUES (@id,@ordinal,@label,@description,@status,@effort,@produces_l4,@delivered_by,@blocked_by,@would_create_event,@notes)
  ON CONFLICT(id) DO UPDATE SET ordinal=excluded.ordinal, label=excluded.label, description=excluded.description, status=excluded.status,
    effort=excluded.effort, produces_l4=excluded.produces_l4, delivered_by=excluded.delivered_by, blocked_by=excluded.blocked_by, would_create_event=excluded.would_create_event, notes=excluded.notes`)
const keep = new Set(milestones.map(m => m.id))
const hasOpp = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='opportunity'").get().n > 0
const hasCol = hasOpp && db.prepare("SELECT count(*) n FROM pragma_table_info('opportunity') WHERE name='expected_l4'").get().n > 0
const tx = db.transaction(() => {
  for (const m of milestones) upsert.run({ id: m.id, ordinal: m.ordinal ?? 99, label: m.label, description: m.description ?? null, status: m.status ?? 'todo',
    effort: Math.max(1, Math.min(5, Number(m.effort) || 1)), produces_l4: m.produces_l4 ? 1 : 0, delivered_by: m.delivered_by ?? null, blocked_by: m.blocked_by ?? null, would_create_event: m.would_create_event ?? null, notes: m.notes ?? null })
  for (const r of db.prepare('SELECT id FROM reality_milestone').all()) if (!keep.has(r.id)) db.prepare('DELETE FROM reality_milestone WHERE id=?').run(r.id)
  if (hasCol) {
    db.prepare('UPDATE opportunity SET expected_l4=0').run()
    for (const [oid, n] of Object.entries(opportunity_l4 || {})) db.prepare('UPDATE opportunity SET expected_l4=? WHERE id=?').run(Math.max(0, Number(n) || 0), oid)
  }
})
tx()
const done = db.prepare("SELECT count(*) n FROM reality_milestone WHERE status='done'").get().n
const l4ms = db.prepare('SELECT count(*) n FROM reality_milestone WHERE produces_l4=1').get().n
const tagged = hasCol ? db.prepare('SELECT count(*) n FROM opportunity WHERE expected_l4>0').get().n : 0
db.close()
console.log(`reconcile-reality-path: ${milestones.length} milestones (${done} done, ${l4ms} produce L4); ${tagged} opportunity(ies) tagged with expected L4.`)
