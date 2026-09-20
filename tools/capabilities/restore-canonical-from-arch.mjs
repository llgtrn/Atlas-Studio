#!/usr/bin/env node
// restore-canonical-from-arch.mjs — recover the canonical capability NAMESPACE from the committed
// docs/architecture.db into the (gitignored, regenerated) docs/capabilities.db.
//
// WHY: the full ~894MB capabilities.db census is gitignored and is lost whenever the container is
// reprovisioned. But architecture.db IS committed and preserves every capability KEY (+ target_crate,
// target_module, status) in capability_architecture_link. This restores those keys as canonical_capability
// rows so the namespace (and the prior count) survives a fresh checkout — donor source-reading then
// re-attaches EVIDENCE (source_capability) to these EXISTING keys (reuse-not-fork) rather than minting
// parallel duplicates. Names/behaviours are placeholders (`recovered`) until re-evidenced.
//
// MONEY SAFETY: architecture.db carries no moves_money flag. Under-flagging money is dangerous, so any
// capability whose domain/target_crate looks financial is CONSERVATIVELY flagged moves_money=1 (which
// forces a financial-control test before it can ever be marked verified). Over-flagging is safe.
import Database from 'better-sqlite3'
import { CAPABILITIES_DB, ARCHITECTURE_DB } from '../_paths.mjs'

const MONEY_DOMAIN = new Set(['erp', 'commerce', 'finance', 'billing', 'payments', 'accounting', 'trading-markets'])
const MONEY_CRATE = /payment|account|billing|erp|ledger|money|finance|invoice|payout|settle/i
const moneyish = (domain, crate) => MONEY_DOMAIN.has(domain) || (crate ? MONEY_CRATE.test(crate) : false)

// status priority when a key links to several arch nodes with different statuses
const STATUS_RANK = { verified: 3, unimplemented: 2, excluded: 1 }
const humanize = (key) => {
  const suffix = key.includes('.') ? key.slice(key.indexOf('.') + 1) : key
  const s = suffix.replace(/_/g, ' ').trim()
  return s.charAt(0).toUpperCase() + s.slice(1)
}

const arch = new Database(ARCHITECTURE_DB, { readonly: true })
// one representative row per capability_key: prefer the highest-rank status + a non-null target.
const rows = arch.prepare(`
  SELECT capability_key AS key, target_crate, target_module, status
  FROM capability_architecture_link
  WHERE capability_key IS NOT NULL AND capability_key <> ''
`).all()
arch.close()

const byKey = new Map()
for (const r of rows) {
  const cur = byKey.get(r.key)
  if (!cur) { byKey.set(r.key, r); continue }
  // keep the better status; backfill a missing target from any row
  if ((STATUS_RANK[r.status] || 0) > (STATUS_RANK[cur.status] || 0)) byKey.set(r.key, { ...r, target_crate: r.target_crate || cur.target_crate, target_module: r.target_module || cur.target_module })
  else { cur.target_crate ||= r.target_crate; cur.target_module ||= r.target_module }
}

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
// only fill keys that do NOT already exist (never clobber freshly-extracted, evidence-backed rows).
const exists = new Set(db.prepare('SELECT key FROM canonical_capability').all().map(r => r.key))
const ins = db.prepare(`INSERT OR IGNORE INTO canonical_capability
  (key,canonical_name,domain,target_crate,target_module,side_effect_class,moves_money,requires_approval,acceptance_criteria,required_tests,financial_control_test,status,donor_count)
  VALUES (@key,@name,@domain,@crate,@module,@sec,@money,0,@ac,NULL,@fct,@status,0)`)

let inserted = 0, money = 0, skipped = 0
const tx = db.transaction(() => {
  for (const [key, r] of byKey) {
    if (exists.has(key)) { skipped++; continue }
    const domain = key.includes('.') ? key.split('.')[0] : 'unclustered'
    const isMoney = moneyish(domain, r.target_crate) ? 1 : 0
    if (isMoney) money++
    ins.run({
      key, name: humanize(key), domain,
      crate: r.target_crate || null, module: r.target_module || null,
      sec: isMoney ? 'money' : 'internal_write', money: isMoney,
      ac: `recovered from architecture.db (evidence pending re-extraction)`,
      fct: isMoney ? 'REQUIRED: policy->approval->CostRecord->SHA256 audit; no external charge when the gate denies' : null,
      status: r.status === 'verified' ? 'unimplemented' : (r.status || 'unimplemented'), // never claim verified without present evidence
    })
    inserted++
  }
})
tx()
const total = db.prepare('SELECT COUNT(*) n FROM canonical_capability').get().n
db.close()
console.log(JSON.stringify({ recovered_keys: byKey.size, inserted, money_flagged: money, skipped_already_present: skipped, canonical_total_now: total }))
