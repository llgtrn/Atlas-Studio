#!/usr/bin/env node
// reconcile-end-state.mjs — apply the committed end-state.json into caps.db.end_state_pillar (DIM 7).
// caps.db is gitignored, so end-state.json is the durable record. Idempotent; JSON authoritative.
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const { pillars } = JSON.parse(readFileSync(join(process.cwd(), 'tools', 'capabilities', 'end-state.json'), 'utf8'))
if (!Array.isArray(pillars)) { console.error('end-state.json: missing "pillars"'); process.exit(1) }

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
db.exec(`CREATE TABLE IF NOT EXISTS end_state_pillar (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT, domains TEXT, crates TEXT,
  target_caps INTEGER, weight INTEGER DEFAULT 3, ordinal INTEGER, notes TEXT)`)
// allow manual_pct via notes-json? keep simple: store manual_pct in notes as 'manual_pct=NN' if present.
const upsert = db.prepare(`INSERT INTO end_state_pillar (id,name,description,domains,crates,target_caps,weight,ordinal,notes)
  VALUES (@id,@name,@description,@domains,@crates,@target_caps,@weight,@ordinal,@notes)
  ON CONFLICT(id) DO UPDATE SET name=excluded.name, description=excluded.description, domains=excluded.domains,
    crates=excluded.crates, target_caps=excluded.target_caps, weight=excluded.weight, ordinal=excluded.ordinal, notes=excluded.notes`)
const keep = new Set(pillars.map(p => p.id))
const existing = db.prepare('SELECT id FROM end_state_pillar').all().map(r => r.id)
const tx = db.transaction(() => {
  for (const p of pillars) upsert.run({
    id: p.id, name: p.name, description: p.description ?? null, domains: p.domains ?? '', crates: p.crates ?? '',
    target_caps: p.target_caps ?? null, weight: p.weight ?? 3, ordinal: p.ordinal ?? 99,
    notes: p.manual_pct != null ? `manual_pct=${p.manual_pct}${p.notes ? '; ' + p.notes : ''}` : (p.notes ?? null),
  })
  for (const id of existing) if (!keep.has(id)) db.prepare('DELETE FROM end_state_pillar WHERE id=?').run(id)
})
tx()
db.close()
console.log(`reconcile-end-state: applied ${pillars.length} pillars.`)
