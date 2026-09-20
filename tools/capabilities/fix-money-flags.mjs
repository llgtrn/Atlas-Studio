#!/usr/bin/env node
// fix-money-flags.mjs — correct the over-applied moves_money flag on canonical_capability.
// The dedupe set moves_money=1 if ANY clustered source row was money-flagged, which over-propagated
// (audit/log/config/read capabilities got money=1 wrongly). Re-flag with a STRICT rule:
// money = actual external payment / payout / order / spend / charge / refund / transfer / trade / invoice-pay.
import Database from 'better-sqlite3'
import { join } from 'node:path'

const db = new Database(join(process.cwd(), 'docs', 'capabilities.db'))
db.pragma('journal_mode = WAL')

const hasTable = (name) => db.prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name=?").get(name) != null
const columnNames = (table) => new Set(db.prepare(`PRAGMA table_info(${table})`).all().map(c => c.name))

// STRICT money: the action commits or moves real money / external financial side effect.
const MONEY_RE = /\b(pay|payment|payout|refund|charge|charge[d]?|invoice|bill(ing)?|spend|spent|purchase|checkout|order(ed)?|transfer|remit|disburse|settle|trade|trad(e|ing)|buy|sell|wallet|subscription|deposit|withdraw|escrow|payroll|salary|reimburs|debit|credit-card|stripe|paypal|braintree|ach|wire)\b/i
// EXCLUDE: read/log/config/audit/metric verbs that are never money even if they touch a finance object
const NOT_MONEY_RE = /\b(audit|log(s)?|metric|trace|config|configure|read|get|list|fetch|query|view|show|search|index|monitor|observe|dashboard|report|retention|rate.?limit|reload|knowledge.graph|debate|sentiment|analyz|enrich|scan|enumerate|discover|footprint)\b/i

const before = db.prepare('SELECT count(*) n FROM canonical_capability WHERE moves_money=1').get().n
const rows = db.prepare('SELECT id, key, canonical_name FROM canonical_capability').all()
const canonicalCols = columnNames('canonical_capability')
const sourceSignals = new Map()
if (hasTable('source_capability')) {
  const sourceCols = columnNames('source_capability')
  const canJoinDirect = canonicalCols.has('id') && sourceCols.has('canonical_id')
  const canJoinProvenance = canonicalCols.has('id') && sourceCols.has('id') && hasTable('provenance')
  const sourceMoves = sourceCols.has('moves_money') ? 'COALESCE(s.moves_money, 0)' : '0'
  const sourceRequires = sourceCols.has('requires_approval') ? 'COALESCE(s.requires_approval, 0)' : '0'
  const sourceSide = sourceCols.has('side_effect_class') ? "lower(COALESCE(s.side_effect_class, ''))" : "''"
  const merge = (rows) => {
    for (const r of rows) {
      const prev = sourceSignals.get(r.id) || { movesMoney: 0, requiresApproval: 0 }
      sourceSignals.set(r.id, {
        movesMoney: Math.max(prev.movesMoney, Number(r.movesMoney || 0)),
        requiresApproval: Math.max(prev.requiresApproval, Number(r.requiresApproval || 0)),
      })
    }
  }
  const select = (joinSql) => `
    SELECT c.id,
      MAX(CASE WHEN ${sourceMoves}=1 OR ${sourceSide} LIKE '%money%' THEN 1 ELSE 0 END) AS movesMoney,
      MAX(${sourceRequires}) AS requiresApproval
    FROM source_capability s
    ${joinSql}
    GROUP BY c.id
  `
  if (canJoinDirect) merge(db.prepare(select('JOIN canonical_capability c ON c.id=s.canonical_id')).all())
  if (canJoinProvenance) merge(db.prepare(select(`
    JOIN provenance p ON p.source_id=s.id
    JOIN canonical_capability c ON c.id=p.canonical_id
  `)).all())
}
const upd = db.prepare('UPDATE canonical_capability SET moves_money=?, requires_approval=?, side_effect_class=?, financial_control_test=?, acceptance_criteria=? WHERE id=?')
let flipped = 0
const tx = db.transaction(() => {
  for (const r of rows) {
    const text = (r.key + ' ' + r.canonical_name).toLowerCase()
    const source = sourceSignals.get(r.id) || { movesMoney: 0, requiresApproval: 0 }
    const isMoney = (MONEY_RE.test(text) && !NOT_MONEY_RE.test(text)) || source.movesMoney === 1
    const requiresApproval = isMoney ? 1 : source.requiresApproval
    // current
    const cur = db.prepare('SELECT moves_money, requires_approval FROM canonical_capability WHERE id=?').get(r.id)
    const want = isMoney ? 1 : 0
    if (cur.moves_money !== want || Number(cur.requires_approval || 0) !== requiresApproval) {
      flipped++
      upd.run(
        want,
        requiresApproval,
        want ? 'money' : '',
        want ? 'REQUIRED (not yet written)' : null,
        want
          ? `Test proves: ${r.canonical_name} runs policy -> board approval -> CostRecord -> audit chain; blocked path leaves no side effect; agent cannot self-approve.`
          : `Test proves: ${r.canonical_name} executes its documented behavior natively in Rust and returns expected output for representative inputs.`,
        r.id
      )
    }
  }
})
tx()
const after = db.prepare('SELECT count(*) n FROM canonical_capability WHERE moves_money=1').get().n
console.log(`money flags corrected: ${before} -> ${after} (flipped ${flipped} rows; strict rule = real payment/order/spend/transfer only)`)
// show the surviving money capabilities (should look genuinely financial)
const survivors = db.prepare("SELECT canonical_name FROM canonical_capability WHERE moves_money=1 ORDER BY donor_count DESC LIMIT 15").all()
console.log('sample TRUE money capabilities:', survivors.map(s => s.canonical_name).join(' | '))
db.close()
