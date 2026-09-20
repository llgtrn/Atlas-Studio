#!/usr/bin/env node
// gen-competitive-doc.mjs — DIM 13 projection: docs/047-competitive-intel.md. Competitor features mapped
// to our capabilities: gap / parity / advantage / differentiation. Pure projection of caps.db.competitor
// + competitor_feature; regenerate with `pnpm caps:competitive-doc`.
import Database from 'better-sqlite3'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, GENERATED } from '../_paths.mjs'

import { marketGaps } from './tracking-lib.mjs'

mkdirSync(GENERATED, { recursive: true })

const db = new Database(CAPABILITIES_DB, { readonly: true })
const has = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='competitor'").get().n > 0
const competitors = has ? db.prepare('SELECT * FROM competitor ORDER BY market, name').all() : []
const { counts, gaps, differentiations, competitorCount } = has ? marketGaps(db) : { counts: {}, gaps: [], differentiations: [], competitorCount: 0 }
db.close()
const esc = (s) => String(s ?? '').replace(/\|/g, '\\|').replace(/\n/g, ' ')
const markets = [...new Set(competitors.map(c => c.market))]

let md = `# 047 — Competitive Intelligence (DIM 13)

> Chronica does not exist in a vacuum. This maps notable competitor features to our capabilities across the
> markets we play in, classed **gap** (they have it, we don't), **parity**, **advantage** (we have it, they
> don't), and **differentiation** (uniquely ours). A pure projection of \`competitor\`/\`competitor_feature\`
> in \`docs/capabilities.db\`; durable source \`tools/capabilities/competitive.json\`. Regenerate with
> \`pnpm caps:competitive-doc\`; live view \`pnpm caps:track market [--gaps]\`.

## Summary

- **${competitorCount}** competitors across ${markets.length} markets (${markets.join(', ')}).
- Features: ${Object.entries(counts).map(([k, n]) => `**${n}** ${k}`).join(' · ')}.

## Competitors

| market | competitor | notes |
|---|---|---|
${competitors.map(c => `| ${c.market} | ${esc(c.name)} | ${esc(c.notes)} |`).join('\n')}

## Differentiation — uniquely ours (defend + lead with these)

${differentiations.length ? differentiations.map(x => `- ★ **${esc(x.feature)}**${x.our_capability_ref ? ` — \`${esc(x.our_capability_ref)}\`` : ''}${x.notes ? ` · ${esc(x.notes)}` : ''}`).join('\n') : '_none_'}

## Gaps — they have it, we don't (the honest catch-up list)

| market | competitor | feature | our ref |
|---|---|---|---|
${gaps.map(g => `| ${g.market} | ${esc(g.comp)} | ${esc(g.feature)} | ${g.our_capability_ref ? esc(g.our_capability_ref) : '—'} |`).join('\n')}

---
_Add competitors/features to \`tools/capabilities/competitive.json\` then \`pnpm caps:rebuild\`. A persistent gap that matters is a candidate opportunity — promote it into [027-opportunities](027-opportunities.md). Differentiation is the moat: do not casually erode it._
`
const dest = join(GENERATED, '047-competitive-intel.md')
writeFileSync(dest, md)
console.log(`wrote 047-competitive-intel.md — ${competitorCount} competitors, ${counts.gap || 0} gaps / ${counts.differentiation || 0} differentiators`)
