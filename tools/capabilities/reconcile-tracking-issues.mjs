#!/usr/bin/env node
// reconcile-tracking-issues.mjs — apply the committed tracking-issues.json into caps.db.tracking_issue.
//
// caps.db is gitignored, so tracking-issues.json is the DURABLE record; this re-applies it on every
// `caps:rebuild` (wired into rebuild-pipeline.mjs). track.mjs (pnpm caps:track) edits the JSON + DB
// together, so the JSON is always the source of truth. Idempotent: full upsert by id. Rows present in
// the DB but absent from the JSON are removed (the JSON is authoritative), so a resolved/deleted issue
// stays gone after a rebuild.
//
// Run: node tools/capabilities/reconcile-tracking-issues.mjs
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const SRC = join(process.cwd(), 'tools', 'capabilities', 'tracking-issues.json')
const SEVERITIES = new Set(['P0', 'P1', 'P2', 'P3', 'P4'])
const STATUSES = new Set(['open', 'in_progress', 'fixed', 'wontfix', 'deferred'])

const { issues } = JSON.parse(readFileSync(SRC, 'utf8'))
if (!Array.isArray(issues)) { console.error('tracking-issues.json: missing "issues" array'); process.exit(1) }

// validate before any write (all-or-nothing).
const seen = new Set()
const bad = []
for (const it of issues) {
  if (!it.id || seen.has(it.id)) bad.push(`duplicate/empty id: ${it.id}`)
  seen.add(it.id)
  if (!SEVERITIES.has(it.severity)) bad.push(`${it.id}: bad severity ${it.severity}`)
  if (!STATUSES.has(it.status)) bad.push(`${it.id}: bad status ${it.status}`)
  if (!it.area || !it.title) bad.push(`${it.id}: missing area/title`)
}
if (bad.length) { console.error('REFUSED — invalid issues:\n  ' + bad.join('\n  ')); process.exit(1) }

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
// the table is created by census-schema.mjs; create here too so this script is runnable standalone.
db.exec(`CREATE TABLE IF NOT EXISTS tracking_issue (
  id TEXT PRIMARY KEY, severity TEXT NOT NULL, area TEXT NOT NULL, title TEXT NOT NULL, detail TEXT,
  evidence_ref TEXT, status TEXT NOT NULL DEFAULT 'open', source TEXT, autonomous_safe INTEGER DEFAULT 0,
  created_at TEXT, resolved_at TEXT, resolved_by TEXT, fix_ref TEXT)`)

const upsert = db.prepare(`INSERT INTO tracking_issue
  (id, severity, area, title, detail, evidence_ref, status, source, autonomous_safe, created_at, resolved_at, resolved_by, fix_ref)
  VALUES (@id, @severity, @area, @title, @detail, @evidence_ref, @status, @source, @autonomous_safe, @created_at, @resolved_at, @resolved_by, @fix_ref)
  ON CONFLICT(id) DO UPDATE SET
    severity=excluded.severity, area=excluded.area, title=excluded.title, detail=excluded.detail,
    evidence_ref=excluded.evidence_ref, status=excluded.status, source=excluded.source,
    autonomous_safe=excluded.autonomous_safe, created_at=excluded.created_at, resolved_at=excluded.resolved_at,
    resolved_by=excluded.resolved_by, fix_ref=excluded.fix_ref`)
const keepIds = new Set(issues.map(i => i.id))
const existing = db.prepare('SELECT id FROM tracking_issue').all().map(r => r.id)
const del = db.prepare('DELETE FROM tracking_issue WHERE id=?')

const tx = db.transaction(() => {
  for (const it of issues) upsert.run({
    id: it.id, severity: it.severity, area: it.area, title: it.title, detail: it.detail ?? null,
    evidence_ref: it.evidence_ref ?? null, status: it.status, source: it.source ?? null,
    autonomous_safe: it.autonomous_safe ? 1 : 0, created_at: it.created_at ?? null,
    resolved_at: it.resolved_at ?? null, resolved_by: it.resolved_by ?? null, fix_ref: it.fix_ref ?? null,
  })
  let removed = 0
  for (const id of existing) if (!keepIds.has(id)) { del.run(id); removed++ }
  return removed
})
const removed = tx()

const byStatus = Object.fromEntries(db.prepare('SELECT status, count(*) n FROM tracking_issue GROUP BY status').all().map(r => [r.status, r.n]))
const open = db.prepare("SELECT count(*) n FROM tracking_issue WHERE status IN ('open','in_progress')").get().n
db.close()
console.log(`reconcile-tracking-issues: applied ${issues.length} issues (${open} open/in-progress)${removed ? `, removed ${removed} stale` : ''}.`)
console.log(`  by status: ${JSON.stringify(byStatus)}`)
