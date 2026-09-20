#!/usr/bin/env node
// reapply-overrides.mjs — re-project the SURVIVABLE side-tables onto the freshly
// rebuilt canonical_capability rows, after a census wipe-and-rebuild.
//
// WHY: apply-clusters.mjs does `DELETE FROM canonical_capability` and recreates every
// row with a NEW id and status reset to 'unimplemented'. The numbered docs are a
// co-equal tracking surface — an agent's progress (status/test ids) and the slice<->
// canonical link must NOT be erased by a rebuild. Those live in canonical_status_override
// and slice_canonical (keyed on the STABLE canonical_key). This tool re-applies them.
//
// Run order: census-schema -> ingest -> apply-clusters -> finalize -> reapply-overrides.
// Idempotent. Reports exactly what it re-applied and any keys that no longer resolve
// (a canonical that was renamed/removed by re-clustering — surfaced, never silently dropped).
import Database from 'better-sqlite3'
import { CAPABILITIES_DB } from '../_paths.mjs'

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')

const canonKeys = new Set(db.prepare('SELECT key FROM canonical_capability').all().map(r => r.key))

// ── 1) re-apply doc/CLI-authored status overrides ─────────────────────────────
const overrides = db.prepare('SELECT * FROM canonical_status_override').all()
const upd = db.prepare(`UPDATE canonical_capability
  SET status = COALESCE(@status, status),
      acceptance_test = COALESCE(@acceptance_test, acceptance_test),
      financial_control_test = COALESCE(@financial_control_test, financial_control_test),
      moves_money = COALESCE(@moves_money, moves_money),
      requires_approval = COALESCE(@requires_approval, requires_approval),
      blocker = COALESCE(@blocker, blocker)
  WHERE key = @canonical_key`)

let applied = 0
const orphanedStatus = []
const moneyViolations = []
const moneyOf = new Map(db.prepare('SELECT key, moves_money, financial_control_test FROM canonical_capability').all().map(r => [r.key, r]))
const txStatus = db.transaction(() => {
  for (const o of overrides) {
    if (!canonKeys.has(o.canonical_key)) { orphanedStatus.push(o.canonical_key); continue }
    // money invariant: a money cap cannot be re-applied as verified without a real fin test.
    // Evaluate against the OVERRIDE's moves_money when set (a survivable file-level proof
    // correction), falling back to the freshly-rebuilt census row otherwise.
    const m = movesMoneyFin(o, moneyOf.get(o.canonical_key))
    if (o.status === 'verified' && m.money && !m.finOk) { moneyViolations.push(o.canonical_key); continue }
    upd.run({
      canonical_key: o.canonical_key,
      status: o.status ?? null,
      acceptance_test: o.acceptance_test ?? null,
      financial_control_test: o.financial_control_test ?? null,
      moves_money: (o.moves_money === 0 || o.moves_money === 1) ? o.moves_money : null,
      requires_approval: (o.requires_approval === 0 || o.requires_approval === 1) ? o.requires_approval : null,
      blocker: o.blocker ?? null,
    })
    applied++
  }
})
txStatus()

function movesMoneyFin(override, row) {
  // an explicit moves_money override (0 or 1) is authoritative; else use the census row.
  const money = (override.moves_money === 0 || override.moves_money === 1)
    ? override.moves_money === 1
    : (row?.moves_money === 1)
  const fin = override.financial_control_test ?? row?.financial_control_test ?? ''
  const finOk = fin && !/^REQUIRED|^—$|^-$/.test(String(fin).trim())
  return { money, finOk }
}

// ── 2) derive canonical_capability.slice from the survivable slice_canonical bridge ──
// (highest-confidence, lowest-slice wins when a canonical links to several slices.)
db.exec('UPDATE canonical_capability SET slice = NULL')
const deriveSlice = db.prepare(`UPDATE canonical_capability SET slice = (
    SELECT sc.slice FROM slice_canonical sc
    WHERE sc.canonical_key = canonical_capability.key
    ORDER BY (sc.confidence='high') DESC, (sc.confidence='medium') DESC, sc.slice ASC
    LIMIT 1)
  WHERE key IN (SELECT canonical_key FROM slice_canonical)`)
const info = deriveSlice.run()
const linkedCanon = db.prepare('SELECT count(*) n FROM canonical_capability WHERE slice IS NOT NULL').get().n
const totalCanon = db.prepare('SELECT count(*) n FROM canonical_capability').get().n

console.log('reapply-overrides:')
console.log(`  status overrides re-applied: ${applied}/${overrides.length}` +
  (orphanedStatus.length ? ` | ${orphanedStatus.length} orphaned (key no longer exists): ${orphanedStatus.slice(0, 5).join(', ')}${orphanedStatus.length > 5 ? '…' : ''}` : '') +
  (moneyViolations.length ? ` | ${moneyViolations.length} BLOCKED money-without-fin-test: ${moneyViolations.slice(0, 5).join(', ')}` : ''))
console.log(`  canonical.slice derived: ${linkedCanon}/${totalCanon} canonicals linked to a slice (${totalCanon - linkedCanon} unassigned -> backlog)`)
db.close()
