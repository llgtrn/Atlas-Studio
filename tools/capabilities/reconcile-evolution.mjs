#!/usr/bin/env node
// reconcile-evolution.mjs — apply committed evolution-scores.json into caps.db.evolution_score (DIM 15,
// organism mode). evolution_score = product of the four 1-5 axes. Warns (does NOT fail) on scores whose
// opportunity_id no longer exists so the durable record survives opportunity churn. Run AFTER
// reconcile-opportunities so the existence check sees current ids. Idempotent; removes stale.
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const { scores } = JSON.parse(readFileSync(join(process.cwd(), 'tools', 'capabilities', 'evolution-scores.json'), 'utf8'))
if (!Array.isArray(scores)) { console.error('evolution-scores.json: missing "scores"'); process.exit(1) }
const clamp = (n) => Math.max(1, Math.min(5, Number(n) || 1))

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
db.exec(`CREATE TABLE IF NOT EXISTS evolution_score (
  opportunity_id TEXT PRIMARY KEY, learning_value INTEGER, optionality INTEGER, adaptability INTEGER,
  knowledge_gain INTEGER, evolution_score INTEGER, rationale TEXT)`)

const oppIds = new Set(db.prepare('SELECT id FROM opportunity').all().map(r => r.id))
const orphans = scores.filter(s => !oppIds.has(s.opportunity_id)).map(s => s.opportunity_id)
const dupes = scores.length - new Set(scores.map(s => s.opportunity_id)).size
if (dupes) { console.error(`REFUSED — ${dupes} duplicate opportunity_id(s) in evolution-scores.json`); db.close(); process.exit(1) }

const upsert = db.prepare(`INSERT INTO evolution_score (opportunity_id,learning_value,optionality,adaptability,knowledge_gain,evolution_score,rationale)
  VALUES (@opportunity_id,@learning_value,@optionality,@adaptability,@knowledge_gain,@evolution_score,@rationale)
  ON CONFLICT(opportunity_id) DO UPDATE SET learning_value=excluded.learning_value, optionality=excluded.optionality,
    adaptability=excluded.adaptability, knowledge_gain=excluded.knowledge_gain, evolution_score=excluded.evolution_score, rationale=excluded.rationale`)
const keep = new Set(scores.map(s => s.opportunity_id))
const tx = db.transaction(() => {
  for (const s of scores) {
    const lv = clamp(s.learning_value), op = clamp(s.optionality), ad = clamp(s.adaptability), kg = clamp(s.knowledge_gain)
    upsert.run({ opportunity_id: s.opportunity_id, learning_value: lv, optionality: op, adaptability: ad, knowledge_gain: kg, evolution_score: lv * op * ad * kg, rationale: s.rationale ?? null })
  }
  for (const r of db.prepare('SELECT opportunity_id FROM evolution_score').all()) if (!keep.has(r.opportunity_id)) db.prepare('DELETE FROM evolution_score WHERE opportunity_id=?').run(r.opportunity_id)
})
tx()
db.close()
console.log(`reconcile-evolution: applied ${scores.length} evolution score(s)${orphans.length ? ` — WARN ${orphans.length} score(s) for unknown opportunity: ${orphans.join(', ')}` : ''}.`)
