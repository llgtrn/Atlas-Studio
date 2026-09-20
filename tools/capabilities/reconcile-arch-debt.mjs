#!/usr/bin/env node
// reconcile-arch-debt.mjs — apply committed arch-debt.json into caps.db.arch_debt (DIM 14). Idempotent;
// JSON authoritative; removes stale. Debt = works-but-expensive, distinct from tracking_issue (broken).
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const { debts } = JSON.parse(readFileSync(join(process.cwd(), 'tools', 'capabilities', 'arch-debt.json'), 'utf8'))
if (!Array.isArray(debts)) { console.error('arch-debt.json: missing "debts"'); process.exit(1) }
const KINDS = new Set(['coupling', 'complexity', 'temporary', 'duplication', 'migration', 'operational'])
const STATUSES = new Set(['active', 'accepted', 'scheduled', 'paid'])
const clamp = (n) => Math.max(1, Math.min(5, Number(n) || 1))

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
db.exec(`CREATE TABLE IF NOT EXISTS arch_debt (
  id TEXT PRIMARY KEY, title TEXT NOT NULL, location TEXT, kind TEXT, description TEXT, maintenance_cost INTEGER,
  migration_cost INTEGER, status TEXT DEFAULT 'active', related_capabilities TEXT, created_at TEXT, notes TEXT)`)

const bad = debts.filter(d => !d.id || !d.title || (d.kind && !KINDS.has(d.kind)) || (d.status && !STATUSES.has(d.status)))
if (bad.length) { console.error(`REFUSED — ${bad.length} invalid debt item(s) (need id+title, valid kind/status)`); db.close(); process.exit(1) }

const upsert = db.prepare(`INSERT INTO arch_debt (id,title,location,kind,description,maintenance_cost,migration_cost,status,related_capabilities,created_at,notes)
  VALUES (@id,@title,@location,@kind,@description,@maintenance_cost,@migration_cost,@status,@related_capabilities,@created_at,@notes)
  ON CONFLICT(id) DO UPDATE SET title=excluded.title, location=excluded.location, kind=excluded.kind, description=excluded.description,
    maintenance_cost=excluded.maintenance_cost, migration_cost=excluded.migration_cost, status=excluded.status,
    related_capabilities=excluded.related_capabilities, notes=excluded.notes`)
const keep = new Set(debts.map(d => d.id))
const tx = db.transaction(() => {
  for (const d of debts) upsert.run({
    id: d.id, title: d.title, location: d.location ?? null, kind: d.kind ?? null, description: d.description ?? null,
    maintenance_cost: clamp(d.maintenance_cost), migration_cost: clamp(d.migration_cost), status: d.status ?? 'active',
    related_capabilities: d.related_capabilities ?? null, created_at: d.created_at ?? '2026-06-19', notes: d.notes ?? null,
  })
  for (const r of db.prepare('SELECT id FROM arch_debt').all()) if (!keep.has(r.id)) db.prepare('DELETE FROM arch_debt WHERE id=?').run(r.id)
})
tx()
const load = db.prepare("SELECT COALESCE(SUM(maintenance_cost+migration_cost),0) l FROM arch_debt WHERE status='active'").get().l
db.close()
console.log(`reconcile-arch-debt: applied ${debts.length} debt item(s). active cost-load (maint+migr) = ${load}.`)
