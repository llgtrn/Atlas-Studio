#!/usr/bin/env node
// reconcile-evidence.mjs — THE TRUTH LAYER. For every meta-node (assumption/decision/opportunity/
// arch_debt/competitor_feature/pillar) assign an evidence_level (override > default) + provenance, then:
//   intrinsic_confidence = min(stated_confidence, ceiling(level))     ← EVIDENCE CAPS CONFIDENCE
//   effective_confidence = intrinsic propagated WEAKEST-LINK down the support graph (an opportunity is
//     at most as trustworthy as the weakest assumption it rests on + its weakest upstream enabler).
// Run AFTER the dimension reconciles (it reads assumptions/opportunities/strategy_edge). Idempotent.
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const ev = JSON.parse(readFileSync(join(process.cwd(), 'tools', 'capabilities', 'evidence.json'), 'utf8'))
const CEIL = ev.ceiling, DEF = ev.defaults
const LEVELS = new Set(['L0', 'L1', 'L2', 'L3', 'L4'])
const ovr = new Map(ev.overrides.map(o => [`${o.node_kind}:${o.node_id}`, o]))

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
db.exec(`CREATE TABLE IF NOT EXISTS node_evidence (
  node_kind TEXT NOT NULL, node_id TEXT NOT NULL, evidence_level TEXT NOT NULL, provenance TEXT, evidence_ref TEXT,
  intrinsic_confidence REAL, effective_confidence REAL, rationale TEXT, assessed_at TEXT, assessed_by TEXT,
  PRIMARY KEY (node_kind, node_id))`)
const hasTable = (n) => db.prepare("SELECT count(*) c FROM sqlite_master WHERE type='table' AND name=?").get(n).c > 0

// ── gather the meta-nodes + their stated confidence (1-5 → 0-1 where present) ──
const nodes = []  // {kind,id,stated}
const add = (kind, id, stated) => nodes.push({ kind, id, stated })
if (hasTable('assumption')) for (const r of db.prepare('SELECT id, confidence FROM assumption').all()) add('assumption', r.id, r.confidence ? r.confidence / 5 : null)
if (hasTable('opportunity')) for (const r of db.prepare('SELECT id, confidence FROM opportunity').all()) add('opportunity', r.id, r.confidence ? r.confidence / 5 : null)
if (hasTable('architecture_decision')) for (const r of db.prepare('SELECT id FROM architecture_decision').all()) add('decision', r.id, null)
if (hasTable('arch_debt')) for (const r of db.prepare('SELECT id FROM arch_debt').all()) add('arch_debt', r.id, null)
if (hasTable('competitor_feature')) for (const r of db.prepare('SELECT id FROM competitor_feature').all()) add('competitor_feature', r.id, null)
if (hasTable('end_state_pillar')) for (const r of db.prepare('SELECT id FROM end_state_pillar').all()) add('pillar', r.id, null)
// keep any externally-tagged rows (external_signal evidence is written by reconcile-external)
const externalRows = db.prepare("SELECT node_kind, node_id, evidence_level, provenance, evidence_ref, intrinsic_confidence, rationale FROM node_evidence WHERE node_kind='external_signal'").all()

// ── intrinsic = min(stated ?? ceiling, ceiling(level)) ──
const meta = new Map()  // key -> {kind,id,level,prov,ref,rat,intrinsic}
for (const n of nodes) {
  const key = `${n.kind}:${n.id}`
  const o = ovr.get(key), d = DEF[n.kind] || { level: 'L1', provenance: 'unassessed' }
  const level = o?.level || d.level
  if (!LEVELS.has(level)) { console.error(`bad evidence_level '${level}' for ${key}`); db.close(); process.exit(1) }
  const ceil = CEIL[level]
  const intrinsic = n.stated != null ? Math.min(n.stated, ceil) : ceil
  meta.set(key, { kind: n.kind, id: n.id, level, prov: o?.provenance || d.provenance, ref: o?.evidence_ref || null, rat: o?.rationale || null, intrinsic })
}

// ── HONESTY GUARD: L4 is NON-HAND-ASSIGNABLE — an L4 evidence level is refused unless a real
// production reality_event (user_real + data_real) backs it. The top rung is earned, never claimed.
if (hasTable('reality_event')) {
  const l4backed = new Set(db.prepare("SELECT node_kind||':'||node_id k FROM reality_event WHERE reality_level='L4_production' AND user_real=1 AND data_real=1").all().map(r => r.k))
  for (const [key, v] of meta) if (v.level === 'L4' && !l4backed.has(key)) {
    v.level = 'L3'; v.intrinsic = Math.min(v.intrinsic, CEIL.L3)
    v.rat = `${v.rat ? v.rat + ' ' : ''}[L4 claimed but no production reality_event — capped at L3 by the reality guard]`
  }
}

// ── support graph: a node is capped by the weakest node it RESTS ON ──
const oppIds = new Set(nodes.filter(n => n.kind === 'opportunity').map(n => n.id))
const decIds = new Set(nodes.filter(n => n.kind === 'decision').map(n => n.id))
const supports = new Map()  // key -> [keys it depends on]
const dep = (k, on) => { if (!supports.has(k)) supports.set(k, []); supports.get(k).push(on) }
// assumption.underpins names opportunity/decision ids → those depend on the assumption
if (hasTable('assumption')) for (const a of db.prepare('SELECT id, underpins FROM assumption').all()) {
  for (const tok of String(a.underpins || '').split(/[,\s]+/).map(s => s.trim()).filter(Boolean)) {
    if (oppIds.has(tok)) dep(`opportunity:${tok}`, `assumption:${a.id}`)
    if (decIds.has(tok)) dep(`decision:${tok}`, `assumption:${a.id}`)
  }
}
// strategy graph: A unlocks B → B rests on A;  A depends_on B → A rests on B  (only evidence-tagged endpoints)
if (hasTable('strategy_edge')) for (const e of db.prepare("SELECT from_kind,from_id,relation,to_kind,to_id FROM strategy_edge WHERE relation IN ('unlocks','depends_on')").all()) {
  const from = `${e.from_kind}:${e.from_id}`, to = `${e.to_kind}:${e.to_id}`
  if (!meta.has(from) || !meta.has(to)) continue
  if (e.relation === 'unlocks') dep(to, from)
  else dep(from, to)
}

// ── weakest-link fixpoint: effective[n] = min(intrinsic[n], min over rests-on of effective) ──
const eff = new Map([...meta].map(([k, v]) => [k, v.intrinsic]))
for (let i = 0; i < 16; i++) {
  let changed = false
  for (const [k, v] of meta) {
    let e = v.intrinsic
    for (const on of (supports.get(k) || [])) if (eff.has(on)) e = Math.min(e, eff.get(on))
    if (Math.abs(e - eff.get(k)) > 1e-9) { eff.set(k, e); changed = true }
  }
  if (!changed) break
}

// ── write ──
const round = (x) => Math.round(x * 100) / 100
const upsert = db.prepare(`INSERT INTO node_evidence (node_kind,node_id,evidence_level,provenance,evidence_ref,intrinsic_confidence,effective_confidence,rationale,assessed_at,assessed_by)
  VALUES (@node_kind,@node_id,@evidence_level,@provenance,@evidence_ref,@intrinsic_confidence,@effective_confidence,@rationale,@assessed_at,@assessed_by)
  ON CONFLICT(node_kind,node_id) DO UPDATE SET evidence_level=excluded.evidence_level, provenance=excluded.provenance, evidence_ref=excluded.evidence_ref,
    intrinsic_confidence=excluded.intrinsic_confidence, effective_confidence=excluded.effective_confidence, rationale=excluded.rationale, assessed_at=excluded.assessed_at`)
const keep = new Set([...meta.keys(), ...externalRows.map(r => `${r.node_kind}:${r.node_id}`)])
const tx = db.transaction(() => {
  for (const [k, v] of meta) upsert.run({ node_kind: v.kind, node_id: v.id, evidence_level: v.level, provenance: v.prov, evidence_ref: v.ref,
    intrinsic_confidence: round(v.intrinsic), effective_confidence: round(eff.get(k)), rationale: v.rat, assessed_at: '2026-06-19', assessed_by: 'reconcile-evidence' })
  for (const r of db.prepare('SELECT node_kind, node_id FROM node_evidence').all()) if (!keep.has(`${r.node_kind}:${r.node_id}`)) db.prepare('DELETE FROM node_evidence WHERE node_kind=? AND node_id=?').run(r.node_kind, r.node_id)
})
tx()

const byLevel = Object.fromEntries(db.prepare('SELECT evidence_level, count(*) n FROM node_evidence GROUP BY evidence_level ORDER BY evidence_level').all().map(r => [r.evidence_level, r.n]))
const dragged = db.prepare('SELECT count(*) n FROM node_evidence WHERE effective_confidence < intrinsic_confidence - 0.001').get().n
const opinion = db.prepare("SELECT count(*) n FROM node_evidence WHERE evidence_level IN ('L0','L1') AND provenance LIKE '%authored%'").get().n
db.close()
console.log(`reconcile-evidence: tagged ${meta.size} meta-nodes. by level: ${JSON.stringify(byLevel)}; ${opinion} self-authored opinion/inference; ${dragged} node(s) confidence-dragged by a weaker support.`)
