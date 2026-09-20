#!/usr/bin/env node
// gen-arch-debt-doc.mjs — DIM 14 projection: docs/048-arch-debt.md. Architectural debt: things that WORK
// but are expensive to keep or will cost to migrate — tracked separately from issues (which are broken).
// Pure projection of caps.db.arch_debt; regenerate with `pnpm caps:debt-doc`.
import Database from 'better-sqlite3'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, GENERATED } from '../_paths.mjs'

import { debtLoad } from './tracking-lib.mjs'

mkdirSync(GENERATED, { recursive: true })

const db = new Database(CAPABILITIES_DB, { readonly: true })
const has = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='arch_debt'").get().n > 0
const { total, byKind, items } = has ? debtLoad(db) : { total: 0, byKind: {}, items: [] }
const allRows = has ? db.prepare('SELECT * FROM arch_debt ORDER BY status, (maintenance_cost+migration_cost) DESC').all() : []
db.close()
const esc = (s) => String(s ?? '').replace(/\|/g, '\\|').replace(/\n/g, ' ')

let md = `# 048 — Architectural Debt (DIM 14)

> Debt is **not** a bug (those live in [026-status](026-status.md) / the issue tracker). Debt is something
> that **works today but is expensive to keep, or will cost to migrate** — coupling, complexity, temporary
> scaffolding, duplication, deferred migrations, operational friction. Tracked separately so it cannot hide
> behind green tests. A pure projection of \`arch_debt\` in \`docs/capabilities.db\`; durable source
> \`tools/capabilities/arch-debt.json\`. Regenerate with \`pnpm caps:debt-doc\`; live view \`pnpm caps:track debt\`.
>
> **cost-load = maintenance_cost + migration_cost** (2–10). Higher = pay it down sooner.

## Summary

- **${items.length}** active debt items · total active cost-load **${total}**.
- By kind: ${Object.entries(byKind).sort((a, b) => b[1] - a[1]).map(([k, n]) => `**${k}** ${n}`).join(' · ') || '—'}.

## Active debt (by cost-load)

| load | maint | migr | kind | id | title | location |
|--:|--:|--:|---|---|---|---|
${items.map(r => `| ${r.cost_load} | ${r.maintenance_cost} | ${r.migration_cost} | ${r.kind} | \`${r.id}\` | ${esc(r.title)} | ${esc(r.location)} |`).join('\n')}

## Detail

${items.map(r => `### ${r.id} — load ${r.cost_load} (${r.kind})
**${esc(r.title)}**${r.location ? ` · _${esc(r.location)}_` : ''}
${esc(r.description)}${r.related_capabilities ? `\n_related: ${esc(r.related_capabilities)}_` : ''}`).join('\n\n')}

${allRows.some(r => r.status !== 'active') ? `## Accepted / scheduled / paid\n\n${allRows.filter(r => r.status !== 'active').map(r => `- [${r.status}] \`${r.id}\` — ${esc(r.title)}`).join('\n')}\n` : ''}
---
_Add debt to \`tools/capabilities/arch-debt.json\` then \`pnpm caps:rebuild\`. When debt is paid down, set \`status: paid\`; when consciously tolerated, \`status: accepted\` (records the decision rather than pretending it is gone). High-load coupling/migration debt is a candidate for [027-opportunities](027-opportunities.md)._
`
const dest = join(GENERATED, '048-arch-debt.md')
writeFileSync(dest, md)
console.log(`wrote 048-arch-debt.md — ${items.length} active items, cost-load ${total}`)
