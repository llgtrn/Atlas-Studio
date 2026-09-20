#!/usr/bin/env node
// reconcile-decisions.mjs — apply committed decisions.json into caps.db.architecture_decision (DIM 11,
// ADR memory). alternatives is stored as a JSON string. Idempotent; JSON authoritative; removes stale.
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const { decisions } = JSON.parse(readFileSync(join(process.cwd(), 'tools', 'capabilities', 'decisions.json'), 'utf8'))
if (!Array.isArray(decisions)) { console.error('decisions.json: missing "decisions"'); process.exit(1) }
const STATUSES = new Set(['proposed', 'accepted', 'superseded', 'rejected'])

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
db.exec(`CREATE TABLE IF NOT EXISTS architecture_decision (
  id TEXT PRIMARY KEY, title TEXT NOT NULL, decision TEXT, status TEXT DEFAULT 'accepted', context TEXT,
  rationale TEXT, alternatives TEXT, consequences TEXT, related_opportunities TEXT, related_assumptions TEXT,
  related_capabilities TEXT, supersedes TEXT, decided_at TEXT, source TEXT)`)

const bad = decisions.filter(d => !d.id || !d.title || (d.status && !STATUSES.has(d.status)))
if (bad.length) { console.error(`REFUSED — ${bad.length} invalid decision(s) (need id+title, valid status)`); db.close(); process.exit(1) }

const upsert = db.prepare(`INSERT INTO architecture_decision (id,title,decision,status,context,rationale,alternatives,consequences,related_opportunities,related_assumptions,related_capabilities,supersedes,decided_at,source)
  VALUES (@id,@title,@decision,@status,@context,@rationale,@alternatives,@consequences,@related_opportunities,@related_assumptions,@related_capabilities,@supersedes,@decided_at,@source)
  ON CONFLICT(id) DO UPDATE SET title=excluded.title, decision=excluded.decision, status=excluded.status, context=excluded.context,
    rationale=excluded.rationale, alternatives=excluded.alternatives, consequences=excluded.consequences,
    related_opportunities=excluded.related_opportunities, related_assumptions=excluded.related_assumptions,
    related_capabilities=excluded.related_capabilities, supersedes=excluded.supersedes, decided_at=excluded.decided_at, source=excluded.source`)
const keep = new Set(decisions.map(d => d.id))
const tx = db.transaction(() => {
  for (const d of decisions) upsert.run({
    id: d.id, title: d.title, decision: d.decision ?? null, status: d.status ?? 'accepted', context: d.context ?? null,
    rationale: d.rationale ?? null, alternatives: JSON.stringify(d.alternatives ?? []), consequences: d.consequences ?? null,
    related_opportunities: d.related_opportunities ?? null, related_assumptions: d.related_assumptions ?? null,
    related_capabilities: d.related_capabilities ?? null, supersedes: d.supersedes ?? null, decided_at: d.decided_at ?? null, source: d.source ?? 'seed',
  })
  for (const r of db.prepare('SELECT id FROM architecture_decision').all()) if (!keep.has(r.id)) db.prepare('DELETE FROM architecture_decision WHERE id=?').run(r.id)
})
tx()
db.close()
console.log(`reconcile-decisions: applied ${decisions.length} ADRs.`)
