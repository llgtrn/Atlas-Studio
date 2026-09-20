#!/usr/bin/env node
// reconcile-opportunities.mjs — apply the committed opportunities.json into caps.db.opportunity, and
// recompute the derived scores: ev = impact*reach*leverage (Expected Value), priority = ev*confidence/effort.
//
// caps.db is gitignored, so opportunities.json is the DURABLE record (the project's long-term strategic
// memory). Re-applied on every caps:rebuild (wired into rebuild-pipeline). track.mjs (caps:track opp)
// edits the JSON + DB together. Idempotent; JSON is authoritative (rows absent from JSON are removed).
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const SRC = join(process.cwd(), 'tools', 'capabilities', 'opportunities.json')
const CATEGORIES = new Set(['capability', 'workflow', 'integration', 'dedup', 'architecture', 'product', 'revenue', 'automation', 'ux', 'observability', 'security', 'testing', 'synergy'])
const STATUSES = new Set(['discovered', 'validated', 'planned', 'executing', 'implemented', 'rejected'])
const score = (n) => Math.max(1, Math.min(5, Number(n ?? 3))) // clamp 1..5, default 3

const { opportunities } = JSON.parse(readFileSync(SRC, 'utf8'))
if (!Array.isArray(opportunities)) { console.error('opportunities.json: missing "opportunities" array'); process.exit(1) }

const seen = new Set(); const bad = []
for (const o of opportunities) {
  if (!o.id || seen.has(o.id)) bad.push(`duplicate/empty id: ${o.id}`); seen.add(o.id)
  if (!o.title) bad.push(`${o.id}: missing title`)
  if (!CATEGORIES.has(o.category)) bad.push(`${o.id}: bad category ${o.category}`)
  if (o.status && !STATUSES.has(o.status)) bad.push(`${o.id}: bad status ${o.status}`)
}
if (bad.length) { console.error('REFUSED — invalid opportunities:\n  ' + bad.join('\n  ')); process.exit(1) }

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
db.exec(`CREATE TABLE IF NOT EXISTS opportunity (
  id TEXT PRIMARY KEY, title TEXT NOT NULL, description TEXT, category TEXT NOT NULL,
  impact INTEGER, reach INTEGER, leverage INTEGER, effort INTEGER, confidence INTEGER, ev INTEGER, priority REAL,
  source_repo TEXT, related_capabilities TEXT, related_architecture_nodes TEXT,
  status TEXT NOT NULL DEFAULT 'discovered', created_at TEXT, updated_at TEXT, notes TEXT)`)

const upsert = db.prepare(`INSERT INTO opportunity
  (id,title,description,category,impact,reach,leverage,effort,confidence,ev,priority,source_repo,related_capabilities,related_architecture_nodes,status,created_at,updated_at,notes)
  VALUES (@id,@title,@description,@category,@impact,@reach,@leverage,@effort,@confidence,@ev,@priority,@source_repo,@related_capabilities,@related_architecture_nodes,@status,@created_at,@updated_at,@notes)
  ON CONFLICT(id) DO UPDATE SET title=excluded.title, description=excluded.description, category=excluded.category,
    impact=excluded.impact, reach=excluded.reach, leverage=excluded.leverage, effort=excluded.effort, confidence=excluded.confidence,
    ev=excluded.ev, priority=excluded.priority, source_repo=excluded.source_repo, related_capabilities=excluded.related_capabilities,
    related_architecture_nodes=excluded.related_architecture_nodes, status=excluded.status, created_at=excluded.created_at,
    updated_at=excluded.updated_at, notes=excluded.notes`)
const keep = new Set(opportunities.map(o => o.id))
const existing = db.prepare('SELECT id FROM opportunity').all().map(r => r.id)
const del = db.prepare('DELETE FROM opportunity WHERE id=?')

const tx = db.transaction(() => {
  for (const o of opportunities) {
    const impact = score(o.impact), reach = score(o.reach), leverage = score(o.leverage), effort = score(o.effort), confidence = score(o.confidence)
    const ev = impact * reach * leverage
    const priority = Math.round((ev * confidence / effort) * 100) / 100
    upsert.run({
      id: o.id, title: o.title, description: o.description ?? null, category: o.category,
      impact, reach, leverage, effort, confidence, ev, priority,
      source_repo: o.source_repo ?? null, related_capabilities: o.related_capabilities ?? null,
      related_architecture_nodes: o.related_architecture_nodes ?? null, status: o.status ?? 'discovered',
      created_at: o.created_at ?? null, updated_at: o.updated_at ?? null, notes: o.notes ?? null,
    })
  }
  let removed = 0
  for (const id of existing) if (!keep.has(id)) { del.run(id); removed++ }
  return removed
})
const removed = tx()

const open = db.prepare("SELECT count(*) n FROM opportunity WHERE status NOT IN ('implemented','rejected')").get().n
const byCat = Object.fromEntries(db.prepare("SELECT category, count(*) n FROM opportunity WHERE status NOT IN ('implemented','rejected') GROUP BY category ORDER BY n DESC").all().map(r => [r.category, r.n]))
const top = db.prepare("SELECT id, priority FROM opportunity WHERE status NOT IN ('implemented','rejected') ORDER BY priority DESC LIMIT 3").all()
db.close()
console.log(`reconcile-opportunities: applied ${opportunities.length} opportunities (${open} active)${removed ? `, removed ${removed} stale` : ''}.`)
console.log(`  by category: ${JSON.stringify(byCat)}`)
console.log(`  top priority: ${top.map(t => `${t.id}(${t.priority})`).join(', ')}`)
