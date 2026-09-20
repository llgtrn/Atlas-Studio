#!/usr/bin/env node
// reconcile-reality.mjs — THE REALITY ENGINE (DIM 16). Apply reality-events.json + outcomes.json +
// predictions.json. HARD HONESTY GUARD: an L4_production reality_event is REFUSED unless user_real=1 AND
// data_real=1 — L4 cannot be claimed, only earned by a real occurrence. Computes the Brier score on any
// resolved prediction (the brain grading its own forecast). Runs BEFORE reconcile-evidence (which then
// refuses any hand-claimed L4 not backed by a real event). Idempotent; JSON authoritative.
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const HERE = join(process.cwd(), 'tools', 'capabilities')
const read = (f) => JSON.parse(readFileSync(join(HERE, f), 'utf8'))
const { events } = read('reality-events.json')
const { outcomes } = read('outcomes.json')
const { predictions } = read('predictions.json')
const RLEVELS = new Set(['L3_synthetic', 'L4_production'])
const RESOLUTIONS = new Set(['pending', 'true', 'false', 'partial'])
const outcomeVal = { true: 1, false: 0, partial: 0.5 }

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
db.exec(`CREATE TABLE IF NOT EXISTS reality_event (id TEXT PRIMARY KEY, node_kind TEXT, node_id TEXT, reality_level TEXT NOT NULL, event_type TEXT, occurred_at TEXT, description TEXT, data_real INTEGER DEFAULT 0, user_real INTEGER DEFAULT 0, outcome_observed TEXT, evidence_ref TEXT);
  CREATE TABLE IF NOT EXISTS outcome (id TEXT PRIMARY KEY, subject_kind TEXT, subject_id TEXT, metric TEXT NOT NULL, baseline TEXT, target TEXT, actual TEXT, unit TEXT, status TEXT DEFAULT 'predicted', observed_at TEXT, evidence_ref TEXT, notes TEXT);
  CREATE TABLE IF NOT EXISTS prediction (id TEXT PRIMARY KEY, subject_kind TEXT, subject_id TEXT, prediction TEXT NOT NULL, predicted_at TEXT, expiry TEXT, confidence REAL, resolution TEXT DEFAULT 'pending', resolved_at TEXT, actual_outcome TEXT, brier REAL, notes TEXT)`)

// ── HONESTY GUARDS ──
const bad = []
for (const e of events) {
  if (!RLEVELS.has(e.reality_level)) bad.push(`reality_event ${e.id}: bad reality_level '${e.reality_level}'`)
  if (e.reality_level === 'L4_production' && !(e.user_real && e.data_real)) bad.push(`reality_event ${e.id}: claims L4_production but lacks real user + real data — L4 is earned, not claimed`)
}
for (const p of predictions) if (!RESOLUTIONS.has(p.resolution || 'pending')) bad.push(`prediction ${p.id}: bad resolution '${p.resolution}'`)
if (bad.length) { console.error(`REFUSED — ${bad.length} reality violation(s):\n  ` + bad.join('\n  ')); db.close(); process.exit(1) }

const upE = db.prepare(`INSERT INTO reality_event (id,node_kind,node_id,reality_level,event_type,occurred_at,description,data_real,user_real,outcome_observed,evidence_ref)
  VALUES (@id,@node_kind,@node_id,@reality_level,@event_type,@occurred_at,@description,@data_real,@user_real,@outcome_observed,@evidence_ref)
  ON CONFLICT(id) DO UPDATE SET node_kind=excluded.node_kind, node_id=excluded.node_id, reality_level=excluded.reality_level, event_type=excluded.event_type,
    occurred_at=excluded.occurred_at, description=excluded.description, data_real=excluded.data_real, user_real=excluded.user_real, outcome_observed=excluded.outcome_observed, evidence_ref=excluded.evidence_ref`)
const upO = db.prepare(`INSERT INTO outcome (id,subject_kind,subject_id,metric,baseline,target,actual,unit,status,observed_at,evidence_ref,notes)
  VALUES (@id,@subject_kind,@subject_id,@metric,@baseline,@target,@actual,@unit,@status,@observed_at,@evidence_ref,@notes)
  ON CONFLICT(id) DO UPDATE SET subject_kind=excluded.subject_kind, subject_id=excluded.subject_id, metric=excluded.metric, baseline=excluded.baseline,
    target=excluded.target, actual=excluded.actual, unit=excluded.unit, status=excluded.status, observed_at=excluded.observed_at, notes=excluded.notes`)
const upP = db.prepare(`INSERT INTO prediction (id,subject_kind,subject_id,prediction,predicted_at,expiry,confidence,resolution,resolved_at,actual_outcome,brier,notes)
  VALUES (@id,@subject_kind,@subject_id,@prediction,@predicted_at,@expiry,@confidence,@resolution,@resolved_at,@actual_outcome,@brier,@notes)
  ON CONFLICT(id) DO UPDATE SET subject_kind=excluded.subject_kind, subject_id=excluded.subject_id, prediction=excluded.prediction, predicted_at=excluded.predicted_at,
    expiry=excluded.expiry, confidence=excluded.confidence, resolution=excluded.resolution, resolved_at=excluded.resolved_at, actual_outcome=excluded.actual_outcome, brier=excluded.brier, notes=excluded.notes`)

const keepE = new Set(events.map(e => e.id)), keepO = new Set(outcomes.map(o => o.id)), keepP = new Set(predictions.map(p => p.id))
const tx = db.transaction(() => {
  for (const e of events) upE.run({ id: e.id, node_kind: e.node_kind ?? null, node_id: e.node_id ?? null, reality_level: e.reality_level, event_type: e.event_type ?? null,
    occurred_at: e.occurred_at ?? null, description: e.description ?? null, data_real: e.data_real ? 1 : 0, user_real: e.user_real ? 1 : 0, outcome_observed: e.outcome_observed ?? null, evidence_ref: e.evidence_ref ?? null })
  for (const o of outcomes) upO.run({ id: o.id, subject_kind: o.subject_kind ?? null, subject_id: o.subject_id ?? null, metric: o.metric, baseline: o.baseline ?? null,
    target: o.target ?? null, actual: o.actual ?? null, unit: o.unit ?? null, status: o.status ?? 'predicted', observed_at: o.observed_at ?? null, evidence_ref: o.evidence_ref ?? null, notes: o.notes ?? null })
  for (const p of predictions) {
    const res = p.resolution || 'pending'
    const brier = (res !== 'pending' && p.confidence != null) ? Math.round(Math.pow(p.confidence - outcomeVal[res], 2) * 1000) / 1000 : null
    upP.run({ id: p.id, subject_kind: p.subject_kind ?? null, subject_id: p.subject_id ?? null, prediction: p.prediction, predicted_at: p.predicted_at ?? null,
      expiry: p.expiry ?? null, confidence: p.confidence ?? null, resolution: res, resolved_at: p.resolved_at ?? null, actual_outcome: p.actual_outcome ?? null, brier, notes: p.notes ?? null })
  }
  for (const r of db.prepare('SELECT id FROM reality_event').all()) if (!keepE.has(r.id)) db.prepare('DELETE FROM reality_event WHERE id=?').run(r.id)
  for (const r of db.prepare('SELECT id FROM outcome').all()) if (!keepO.has(r.id)) db.prepare('DELETE FROM outcome WHERE id=?').run(r.id)
  for (const r of db.prepare('SELECT id FROM prediction').all()) if (!keepP.has(r.id)) db.prepare('DELETE FROM prediction WHERE id=?').run(r.id)
})
tx()

const l4 = db.prepare("SELECT count(*) n FROM reality_event WHERE reality_level='L4_production'").get().n
const l3 = db.prepare("SELECT count(*) n FROM reality_event WHERE reality_level='L3_synthetic'").get().n
const measured = db.prepare("SELECT count(*) n FROM outcome WHERE status='measured'").get().n
const resolved = db.prepare("SELECT count(*) n FROM prediction WHERE resolution<>'pending'").get().n
db.close()
console.log(`reconcile-reality: ${events.length} reality events (${l3} L3_synthetic, ${l4} L4_production), ${outcomes.length} outcomes (${measured} measured), ${predictions.length} predictions (${resolved} resolved). L4 production reality = ${l4}.`)
