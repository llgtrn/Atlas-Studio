#!/usr/bin/env node
// gen-assumptions-doc.mjs — DIM 10 projection: docs/044-assumptions.md. The load-bearing architectural
// bets + the strongest honest counter-evidence, ranked by fragility, so we never build months of work on
// a wrong premise. Pure projection of caps.db.assumption; regenerate with `pnpm caps:assumptions-doc`.
import Database from 'better-sqlite3'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, GENERATED } from '../_paths.mjs'

import { assumptionRisk } from './tracking-lib.mjs'

mkdirSync(GENERATED, { recursive: true })

const db = new Database(CAPABILITIES_DB, { readonly: true })
const has = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='assumption'").get().n > 0
const { all, fragile } = has ? assumptionRisk(db) : { all: [], fragile: [] }
db.close()
const esc = (s) => String(s ?? '').replace(/\|/g, '\\|').replace(/\n/g, ' ')
const stIcon = (s) => s === 'validated' ? '✓ validated' : s === 'invalidated' ? '✗ invalidated' : s === 'validating' ? '◐ validating' : '○ unvalidated'

let md = `# 044 — Assumptions (DIM 10)

> Every large architecture rests on a hypothesis treated as fact. This is the register of Chronica's
> **load-bearing bets** and — for each — the strongest honest counter-evidence, so a wrong premise gets
> caught before months of work ride on it. A pure projection of \`assumption\` in \`docs/capabilities.db\`;
> durable source \`tools/capabilities/assumptions.json\`. Regenerate with \`pnpm caps:assumptions-doc\`;
> live view \`pnpm caps:track assumptions [--fragile]\`.
>
> **fragility = risk_if_wrong × (6 − confidence)** (1–25). A bet that is fragile (≥12) **and** not yet
> validated is the kind most worth pressure-testing now. Status moves unvalidated → validating → validated
> / invalidated; an invalidated high-fragility assumption should trigger a re-think of everything it underpins.

## Summary

- **${all.length}** assumptions tracked · **${fragile.length}** fragile + unvalidated (pressure-test first).

## Fragile + unvalidated — test these before building on them

${fragile.length ? fragile.map(a => `### ⚠ ${a.id} — fragility ${a.fragility} (confidence ${a.confidence}/5, risk ${a.risk_if_wrong}/5)
**${esc(a.statement)}**
- Underpins: ${esc(a.underpins) || '—'}
- For: ${esc(a.evidence_for)}
- **Against:** ${esc(a.evidence_against)}
- Status: ${stIcon(a.validation_status)} · source: ${esc(a.source)}`).join('\n\n') : '_none_'}

## All assumptions (by fragility)

| frag | conf | risk | status | id | statement |
|--:|--:|--:|---|---|---|
${all.map(a => `| ${a.fragility} | ${a.confidence} | ${a.risk_if_wrong} | ${stIcon(a.validation_status)} | \`${a.id}\` | ${esc(a.statement)} |`).join('\n')}

---
_Add/update bets in \`tools/capabilities/assumptions.json\` (the durable seed) then \`pnpm caps:rebuild\`. When evidence resolves a bet, flip \`validation_status\`; when an assumption is invalidated, open issues for whatever it underpinned._
`
const dest = join(GENERATED, '044-assumptions.md')
writeFileSync(dest, md)
console.log(`wrote 044-assumptions.md — ${all.length} assumptions, ${fragile.length} fragile+unvalidated`)
