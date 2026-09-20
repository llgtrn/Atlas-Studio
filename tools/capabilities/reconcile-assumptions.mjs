#!/usr/bin/env node
// reconcile-assumptions.mjs — apply committed assumptions.json into caps.db.assumption (DIM 10).
// Computes fragility = risk_if_wrong * (6 - confidence) (1-25; higher = more dangerous). Idempotent;
// JSON authoritative; caps.db is gitignored so the JSON is the durable record. Removes stale rows.
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const { assumptions } = JSON.parse(readFileSync(join(process.cwd(), 'tools', 'capabilities', 'assumptions.json'), 'utf8'))
if (!Array.isArray(assumptions)) { console.error('assumptions.json: missing "assumptions"'); process.exit(1) }
const STATUSES = new Set(['unvalidated', 'validating', 'validated', 'invalidated'])
const clamp = (n) => Math.max(1, Math.min(5, Number(n) || 1))

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
db.exec(`CREATE TABLE IF NOT EXISTS assumption (
  id TEXT PRIMARY KEY, statement TEXT NOT NULL, underpins TEXT, evidence_for TEXT, evidence_against TEXT,
  confidence INTEGER, risk_if_wrong INTEGER, validation_status TEXT DEFAULT 'unvalidated', fragility INTEGER,
  source TEXT, created_at TEXT, updated_at TEXT, notes TEXT)`)

const bad = assumptions.filter(a => !a.id || !a.statement || (a.validation_status && !STATUSES.has(a.validation_status)))
if (bad.length) { console.error(`REFUSED — ${bad.length} invalid assumption(s) (need id+statement, valid validation_status)`); db.close(); process.exit(1) }

const upsert = db.prepare(`INSERT INTO assumption (id,statement,underpins,evidence_for,evidence_against,confidence,risk_if_wrong,validation_status,fragility,source,created_at,updated_at,notes)
  VALUES (@id,@statement,@underpins,@evidence_for,@evidence_against,@confidence,@risk_if_wrong,@validation_status,@fragility,@source,@created_at,@updated_at,@notes)
  ON CONFLICT(id) DO UPDATE SET statement=excluded.statement, underpins=excluded.underpins, evidence_for=excluded.evidence_for,
    evidence_against=excluded.evidence_against, confidence=excluded.confidence, risk_if_wrong=excluded.risk_if_wrong,
    validation_status=excluded.validation_status, fragility=excluded.fragility, source=excluded.source, notes=excluded.notes`)
const keep = new Set(assumptions.map(a => a.id))
const tx = db.transaction(() => {
  for (const a of assumptions) {
    const conf = clamp(a.confidence), risk = clamp(a.risk_if_wrong)
    upsert.run({
      id: a.id, statement: a.statement, underpins: a.underpins ?? null, evidence_for: a.evidence_for ?? null,
      evidence_against: a.evidence_against ?? null, confidence: conf, risk_if_wrong: risk,
      validation_status: a.validation_status ?? 'unvalidated', fragility: risk * (6 - conf),
      source: a.source ?? 'seed', created_at: a.created_at ?? '2026-06-19', updated_at: a.updated_at ?? null, notes: a.notes ?? null,
    })
  }
  for (const r of db.prepare('SELECT id FROM assumption').all()) if (!keep.has(r.id)) db.prepare('DELETE FROM assumption WHERE id=?').run(r.id)
})
tx()
const fragile = db.prepare("SELECT count(*) n FROM assumption WHERE fragility >= 12 AND validation_status IN ('unvalidated','validating','invalidated')").get().n
db.close()
console.log(`reconcile-assumptions: applied ${assumptions.length} assumptions (${fragile} fragile + unvalidated).`)
