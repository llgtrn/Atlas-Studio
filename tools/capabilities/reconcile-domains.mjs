#!/usr/bin/env node
// reconcile-domains.mjs — fix mis-clustered canonical capabilities whose semantic-dedupe
// cluster landed them in the wrong domain. The clearest case: 29 `noise.<area>.*` keys
// clustered into `erp-finance` that are plainly NOT ERP (osint/auth/social/media/etc).
// Their own key carries the true area (`noise.osint.*`, `noise.media.*`, …), so we
// re-domain deterministically by that prefix. Runs in the census rebuild order AFTER
// apply-clusters, so the fix survives every rebuild.
//
// Idempotent. Read-only-safe except the targeted domain UPDATEs. Reports every move.
import Database from 'better-sqlite3'
import { CAPABILITIES_DB } from '../_paths.mjs'

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')

// noise.<area>.* -> true canonical domain
const NOISE_AREA_DOMAIN = {
  osint: 'osint',
  social: 'other', auth: 'other', web: 'other', platform: 'other', config: 'other', infra: 'infra-execution',
  security: 'other',
  media: 'media-generation',
  finance: 'trading-markets',
  browser: 'browser-internet-hand',
  observability: 'observability-analytics',
}

const mis = db.prepare("SELECT key, domain FROM canonical_capability WHERE key LIKE 'noise.%'").all()
const upd = db.prepare('UPDATE canonical_capability SET domain=@domain WHERE key=@key')
let moved = 0
const moves = {}
const tx = db.transaction(() => {
  for (const r of mis) {
    const area = r.key.split('.')[1]
    const target = NOISE_AREA_DOMAIN[area] || 'other'
    if (target !== r.domain) { upd.run({ key: r.key, domain: target }); moved++; (moves[`${r.domain}->${target}`] = (moves[`${r.domain}->${target}`] || 0) + 1) }
  }
})
tx()

console.log(`reconcile-domains: re-domained ${moved} mis-clustered noise.* capabilities.`)
for (const [k, n] of Object.entries(moves)) console.log(`  ${k}: ${n}`)
const erpNoise = db.prepare("SELECT count(*) n FROM canonical_capability WHERE domain='erp-finance' AND key LIKE 'noise.%'").get().n
console.log(`  erp-finance now holds ${erpNoise} noise.* rows (target: 0).`)
db.close()
