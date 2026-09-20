#!/usr/bin/env node
// gen-build-queue.mjs — generate docs/025-build-priority-queue.md: a ranked "what to build
// first" view projected from capabilities.db, so there's ONE ordered entry point into the
// 1,699 unimplemented capabilities instead of a flat list.
//
// Ranking (highest first):
//   1. MONEY caps that are partially built (implemented_unverified) — closest to a verified
//      money path, each needs a financial-control test. Highest leverage + highest risk.
//   2. MONEY caps unimplemented, by donor_count (broad provenance = foundational).
//   3. Non-money caps implemented_unverified, by donor_count — quick verification wins.
//   4. Non-money foundational caps (high donor_count) unimplemented.
// The doc is fully generated (a DB projection); regenerate with the docs:gen pipeline.
import Database from 'better-sqlite3'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, GENERATED } from '../_paths.mjs'

import { attachCanonicalCapabilityFallback } from './canonical-fallback.mjs'

mkdirSync(GENERATED, { recursive: true })

const db = new Database(CAPABILITIES_DB, { readonly: true })
attachCanonicalCapabilityFallback(db)
const NUM = '025'

const line = (c, n) => {
  const tgt = c.target_module ? `${c.target_crate}::${c.target_module}` : (c.target_crate || '(crate tbd)')
  const sl = c.slice ? ` · S${c.slice}` : ' · (backlog)'
  const fin = c.moves_money ? ' · 💰 needs financial-control test' : ''
  return `${String(n).padStart(3)}. \`${c.key}\` — ${c.canonical_name} → ${tgt}${sl} · ${c.donor_count} donor(s)${fin}`
}

const Q = (where, order, lim) => db.prepare(
  `SELECT key,canonical_name,target_crate,target_module,moves_money,donor_count,slice,status
   FROM canonical_capability WHERE ${where} ORDER BY ${order} LIMIT ${lim}`).all()

const moneyPartial = Q("moves_money=1 AND status='implemented_unverified'", 'donor_count DESC, key', 100)
const moneyUnimpl = Q("moves_money=1 AND status='unimplemented'", 'donor_count DESC, key', 100)
const nonMoneyPartial = Q("moves_money=0 AND status='implemented_unverified'", 'donor_count DESC, key', 60)
const foundational = Q("moves_money=0 AND status='unimplemented'", 'donor_count DESC, key', 40)

const counts = db.prepare(`SELECT
  sum(status='verified') v, sum(status='implemented_unverified') iu, sum(status='unimplemented') u,
  sum(status='excluded') x,
  sum(moves_money=1 AND status!='excluded') money,
  sum(moves_money=1 AND status='verified') money_v FROM canonical_capability`).get()

let md = `# ${NUM} — Build-Priority Queue (the ordered entry point)\n\n`
md += `> Generated from \`docs/capabilities.db\` — a ranked "what to build first" view over the ${counts.u} unimplemented + ${counts.iu} partially-built capabilities. This whole doc is a DB projection; regenerate with \`pnpm docs:gen\`. The per-domain roadmaps (030-043) hold the full lists; this is the ordered cut. Index: [000-INDEX.md](000-INDEX.md).\n\n`
md += `| Field | Value |\n|---|---|\n`
md += `| Verified | ${counts.v} / ${counts.v + counts.iu + counts.u} active (${counts.x || 0} excluded) |\n`
md += `| Money capabilities | ${counts.money} (${counts.money_v} verified) — each needs a gate→approval→CostRecord→audit test |\n`
md += `| Tracking surface | this doc ⇄ \`canonical_capability\` ranked |\n\n`

md += `## How to use this queue\n`
md += `1. Take the top item. \`pnpm caps:query get-canonical <key>\` for its full spec (provenance, source files, acceptance criteria).\n`
md += `2. Build it natively in the target \`chronica-*\` crate with a real test. Money caps ALSO need a financial-control test.\n`
md += `3. In the capability's domain doc (030-043), flip its line to \`✅ verified\` + fill the 🧪 test id, then \`pnpm docs:sync\`.\n\n`

md += `<!-- chronica:buildqueue ${NUM} -->\n`
md += `## 1 · Money capabilities — partially built (verify next: highest leverage)\n`
md += (moneyPartial.length ? moneyPartial.map((c, i) => line(c, i + 1)).join('\n') : '_(none)_') + '\n\n'
md += `## 2 · Money capabilities — unimplemented (by provenance breadth)\n`
md += (moneyUnimpl.length ? moneyUnimpl.map((c, i) => line(c, i + 1)).join('\n') : '_(none)_') + '\n\n'
md += `## 3 · Foundational non-money — partially built (quick verification wins)\n`
md += (nonMoneyPartial.length ? nonMoneyPartial.map((c, i) => line(c, i + 1)).join('\n') : '_(none)_') + '\n\n'
md += `## 4 · Foundational non-money — unimplemented (highest donor count first)\n`
md += (foundational.length ? foundational.map((c, i) => line(c, i + 1)).join('\n') : '_(none)_') + '\n'
md += `<!-- /chronica:buildqueue -->\n`

writeFileSync(join(GENERATED, `${NUM}-build-priority-queue.md`), md)
console.log(`wrote ${NUM}-build-priority-queue.md (money partial ${moneyPartial.length}, money unimpl ${moneyUnimpl.length}, nonmoney partial ${nonMoneyPartial.length}, foundational ${foundational.length})`)
db.close()
