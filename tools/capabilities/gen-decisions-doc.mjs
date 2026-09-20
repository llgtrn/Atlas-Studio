#!/usr/bin/env node
// gen-decisions-doc.mjs — DIM 11 projection: docs/045-decisions.md. The architecture decision records
// (ADR memory) — why we chose what we chose, what we rejected and why — recoverable in 2 years. Pure
// projection of caps.db.architecture_decision; regenerate with `pnpm caps:decisions-doc`.
import Database from 'better-sqlite3'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, GENERATED } from '../_paths.mjs'


mkdirSync(GENERATED, { recursive: true })

const db = new Database(CAPABILITIES_DB, { readonly: true })
const has = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='architecture_decision'").get().n > 0
const rows = has ? db.prepare('SELECT * FROM architecture_decision ORDER BY id').all() : []
db.close()
const esc = (s) => String(s ?? '').replace(/\n/g, ' ')

let md = `# 045 — Architecture Decisions / ADR Memory (DIM 11)

> Why did we choose X? Many projects die because nobody remembers. This is the decision record — each
> entry captures the decision, the context that forced it, the rationale, the **alternatives we rejected
> and why**, and the consequences. A pure projection of \`architecture_decision\` in \`docs/capabilities.db\`;
> durable source \`tools/capabilities/decisions.json\`. Regenerate with \`pnpm caps:decisions-doc\`; live view
> \`pnpm caps:track decisions [<id>]\`.

## Decisions

${rows.length} recorded.

| id | status | decision |
|---|---|---|
${rows.map(r => `| [\`${r.id}\`](#${r.id.toLowerCase()}) | ${r.status} | ${esc(r.title)} |`).join('\n')}

${rows.map(r => {
  let alts = []
  try { alts = JSON.parse(r.alternatives || '[]') } catch { alts = [] }
  return `## ${r.id}
**${esc(r.title)}** — _${r.status}${r.decided_at ? ` · ${r.decided_at}` : ''}_

**Decision:** ${esc(r.decision)}

${r.context ? `**Context:** ${esc(r.context)}\n` : ''}${r.rationale ? `**Rationale:** ${esc(r.rationale)}\n` : ''}${alts.length ? `**Alternatives rejected:**\n${alts.map(a => `- _${esc(a.option)}_ — ${esc(a.rejected_because)}`).join('\n')}\n` : ''}${r.consequences ? `**Consequences:** ${esc(r.consequences)}\n` : ''}${r.related_assumptions ? `**Rests on assumptions:** ${esc(r.related_assumptions)}\n` : ''}${r.related_capabilities ? `**Touches:** ${esc(r.related_capabilities)}\n` : ''}`
}).join('\n')}

---
_Add new ADRs to \`tools/capabilities/decisions.json\` (durable seed) then \`pnpm caps:rebuild\`. Supersede rather than delete: set \`status: superseded\` + a new record with \`supersedes\`. Decisions cross-reference [044-assumptions](044-assumptions.md) (the bets they rest on)._
`
const dest = join(GENERATED, '045-decisions.md')
writeFileSync(dest, md)
console.log(`wrote 045-decisions.md — ${rows.length} ADRs`)
