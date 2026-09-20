#!/usr/bin/env node
// reconcile-external.mjs — External Reality Loop. Apply committed external-signals.json into
// caps.db.external_signal, and tag each signal in node_evidence (web-scan = L1, evidence-capped at 0.60).
// caps.db is gitignored; the JSON is the durable record. Refresh the JSON by re-running the
// scan-external-reality workflow. Run AFTER reconcile-evidence (which preserves external node_evidence).
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB } from '../_paths.mjs'

const { signals } = JSON.parse(readFileSync(join(process.cwd(), 'tools', 'capabilities', 'external-signals.json'), 'utf8'))
if (!Array.isArray(signals)) { console.error('external-signals.json: missing "signals"'); process.exit(1) }
const L1_CEIL = 0.60

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
db.exec(`CREATE TABLE IF NOT EXISTS external_signal (
  id TEXT PRIMARY KEY, observed_at TEXT, source TEXT, category TEXT, headline TEXT NOT NULL, implication TEXT,
  affects_kind TEXT, affects_id TEXT, recommended_action TEXT, url TEXT, confidence INTEGER);
  CREATE TABLE IF NOT EXISTS node_evidence (
  node_kind TEXT NOT NULL, node_id TEXT NOT NULL, evidence_level TEXT NOT NULL, provenance TEXT, evidence_ref TEXT,
  intrinsic_confidence REAL, effective_confidence REAL, rationale TEXT, assessed_at TEXT, assessed_by TEXT,
  PRIMARY KEY (node_kind, node_id))`)

const bad = signals.filter(s => !s.id || !s.headline)
if (bad.length) { console.error(`REFUSED — ${bad.length} signal(s) missing id/headline`); db.close(); process.exit(1) }

const upS = db.prepare(`INSERT INTO external_signal (id,observed_at,source,category,headline,implication,affects_kind,affects_id,recommended_action,url,confidence)
  VALUES (@id,@observed_at,@source,@category,@headline,@implication,@affects_kind,@affects_id,@recommended_action,@url,@confidence)
  ON CONFLICT(id) DO UPDATE SET observed_at=excluded.observed_at, source=excluded.source, category=excluded.category, headline=excluded.headline,
    implication=excluded.implication, affects_kind=excluded.affects_kind, affects_id=excluded.affects_id, recommended_action=excluded.recommended_action, url=excluded.url, confidence=excluded.confidence`)
const upE = db.prepare(`INSERT INTO node_evidence (node_kind,node_id,evidence_level,provenance,evidence_ref,intrinsic_confidence,effective_confidence,rationale,assessed_at,assessed_by)
  VALUES ('external_signal',@id,'L1','web-scan',@url,@conf,@conf,@rationale,'2026-06-19','reconcile-external')
  ON CONFLICT(node_kind,node_id) DO UPDATE SET evidence_level='L1', provenance='web-scan', evidence_ref=excluded.evidence_ref, intrinsic_confidence=excluded.intrinsic_confidence, effective_confidence=excluded.effective_confidence`)
const keep = new Set(signals.map(s => s.id))
const tx = db.transaction(() => {
  for (const s of signals) {
    upS.run({ id: s.id, observed_at: s.observed_at ?? null, source: s.source ?? null, category: s.category ?? null, headline: s.headline,
      implication: s.implication ?? null, affects_kind: s.affects_kind ?? null, affects_id: s.affects_id ?? null,
      recommended_action: s.recommended_action ?? null, url: s.url ?? null, confidence: Math.max(1, Math.min(5, Number(s.confidence) || 3)) })
    const conf = Math.min((Math.max(1, Math.min(5, Number(s.confidence) || 3)) / 5), L1_CEIL)
    upE.run({ id: s.id, url: s.url ?? null, conf, rationale: 'web-scan external signal (L1 inferred from the open web)' })
  }
  for (const r of db.prepare('SELECT id FROM external_signal').all()) if (!keep.has(r.id)) { db.prepare('DELETE FROM external_signal WHERE id=?').run(r.id); db.prepare("DELETE FROM node_evidence WHERE node_kind='external_signal' AND node_id=?").run(r.id) }
})
tx()
const byCat = Object.fromEntries(db.prepare('SELECT category, count(*) n FROM external_signal GROUP BY category ORDER BY n DESC').all().map(r => [r.category, r.n]))
db.close()
console.log(`reconcile-external: applied ${signals.length} external signals. by category: ${JSON.stringify(byCat)}`)
