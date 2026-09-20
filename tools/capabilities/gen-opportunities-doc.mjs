#!/usr/bin/env node
// gen-opportunities-doc.mjs — project the opportunity table into docs/027-opportunities.md: the
// leverage-ranked STRATEGIC BACKLOG (the Architecture Intelligence dimension). Pure DB projection;
// regenerate with `pnpm caps:opp-doc` (or `pnpm docs:gen`). Durable source: opportunities.json.
import Database from 'better-sqlite3'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, GENERATED } from '../_paths.mjs'


mkdirSync(GENERATED, { recursive: true })

const db = new Database(CAPABILITIES_DB, { readonly: true })
const has = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='opportunity'").get().n > 0
const rows = has ? db.prepare(`SELECT id,title,description,category,impact,reach,leverage,effort,confidence,ev,priority,source_repo,related_capabilities,related_architecture_nodes,status,notes FROM opportunity ORDER BY priority DESC`).all() : []
db.close()

const esc = (s) => String(s ?? '').replace(/\|/g, '\\|').replace(/\n/g, ' ')
const active = rows.filter(o => o.status !== 'implemented' && o.status !== 'rejected')
const done = rows.filter(o => o.status === 'implemented' || o.status === 'rejected')
const byCat = {}
for (const o of active) (byCat[o.category] ??= []).push(o)

let md = `# 027 — Opportunities (the strategic backlog)

> The **Architecture Intelligence** dimension: what SHOULD exist, what creates leverage — not what's
> broken (that's [026-status.md](026-status.md) issues). A pure projection of \`opportunity\` in
> \`docs/capabilities.db\`; durable source \`tools/capabilities/opportunities.json\`. Regenerate with
> \`pnpm caps:opp-doc\` (or \`pnpm docs:gen\`). Discover/record with \`pnpm caps:track opp add\`, advance with
> \`pnpm caps:track opp promote\`. **Ranked by priority = ev · confidence / effort**, where
> **ev (Expected Value) = impact · reach · leverage** (all 1-5; *leverage* = architectural leverage:
> foundational work that unlocks other work). Donor-grounded: \`source\` names the Temporary/ donor(s).

## How to read

- **Do the top of the list first** — highest priority = best value per unit effort, weighted by confidence.
- **leverage** is the multiplier that matters most for sequencing: a high-leverage item unlocks many others.
- Statuses: discovered → validated → planned → executing → implemented (or rejected).

## Ranked backlog (active: ${active.length})

| # | priority | ev | i·r·lev | eff | conf | id | category | status | source | title |
|--:|--:|--:|:--|--:|--:|---|---|---|---|---|
${active.map((o, i) => `| ${i + 1} | ${o.priority} | ${o.ev} | ${o.impact}·${o.reach}·${o.leverage} | ${o.effort} | ${o.confidence} | \`${o.id}\` | ${o.category} | ${o.status} | ${esc(o.source_repo)} | ${esc(o.title)} |`).join('\n')}

`
// top-5 detail
md += `## Top ${Math.min(5, active.length)} — detail\n\n`
for (const o of active.slice(0, 5)) {
  md += `### ${o.priority} · \`${o.id}\` (${o.category}, ev ${o.ev}, leverage ${o.leverage})\n`
  md += `${esc(o.description) || esc(o.title)}\n\n`
  md += `- **scores**: impact ${o.impact} · reach ${o.reach} · leverage ${o.leverage} · effort ${o.effort} · confidence ${o.confidence}\n`
  if (o.source_repo) md += `- **donor source**: ${esc(o.source_repo)}\n`
  if (o.related_capabilities) md += `- **caps**: \`${esc(o.related_capabilities)}\`\n`
  if (o.related_architecture_nodes) md += `- **arch nodes**: \`${esc(o.related_architecture_nodes)}\`\n`
  if (o.notes) md += `- **note**: ${esc(o.notes)}\n`
  md += `\n`
}

// by category
md += `## By category (active)\n\n`
for (const cat of Object.keys(byCat).sort((a, b) => byCat[b].length - byCat[a].length)) {
  md += `**${cat}** (${byCat[cat].length}): ${byCat[cat].map(o => `\`${o.id}\``).join(', ')}\n\n`
}

if (done.length) {
  md += `## Resolved (${done.length})\n\n| id | status | title |\n|---|---|---|\n`
  md += done.map(o => `| \`${o.id}\` | ${o.status} | ${esc(o.title)} |`).join('\n') + '\n'
}

md += `\n---\n_Opportunities live in \`tools/capabilities/opportunities.json\` (committed; survives the gitignored caps.db). Categories: capability·workflow·integration·dedup·architecture·product·revenue·automation·ux·observability·security·testing·synergy. This backlog is re-evaluated after each implementation (the execution feedback loop)._\n`

const dest = join(GENERATED, '027-opportunities.md')
writeFileSync(dest, md)
console.log(`wrote 027-opportunities.md — ${active.length} active opportunities (top: ${active[0]?.id ?? 'none'} @ priority ${active[0]?.priority ?? '-'})`)
