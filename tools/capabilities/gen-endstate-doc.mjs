#!/usr/bin/env node
// gen-endstate-doc.mjs — DIM 7 projection: docs/024-end-state.md. The formalized Company-OS target +
// distance-to-end-state per pillar. Pure projection; regenerate with `pnpm caps:endstate-doc`.
import Database from 'better-sqlite3'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, GENERATED } from '../_paths.mjs'

import { endStatePillars } from './tracking-lib.mjs'

mkdirSync(GENERATED, { recursive: true })

const db = new Database(CAPABILITIES_DB, { readonly: true })
const has = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='end_state_pillar'").get().n > 0
const { pillars, overall } = has ? endStatePillars(db) : { pillars: [], overall: 0 }
db.close()
const esc = (s) => String(s ?? '').replace(/\|/g, '\\|').replace(/\n/g, ' ')
const bar = (p) => '█'.repeat(Math.round(p / 5)).padEnd(20, '░')

let md = `# 024 — Company OS End-State & Gap (DIM 7)

> The formalized **target** the platform is converging toward, and the measured **distance** to it.
> A pure projection of \`end_state_pillar\` in \`docs/capabilities.db\`; durable source
> \`tools/capabilities/end-state.json\`. Regenerate with \`pnpm caps:endstate-doc\` (or \`pnpm docs:gen\`);
> live view \`pnpm caps:track endstate\`. Per-pillar % = verified caps / total caps in the pillar's
> domains (or by crate; the feed pillar is a manual estimate — UI is TS, not in caps.db). Overall =
> importance-weighted average.

## Company OS Completion — **${overall}%**

| % | progress | pillar | weight | basis |
|--:|---|---|--:|---|
${pillars.map(p => `| ${p.pct}% | \`${bar(p.pct)}\` | **${esc(p.name)}** | ${p.weight} | ${p.basis === 'manual' ? 'manual' : `${p.verified}/${p.total}`} |`).join('\n')}

## Pillars (the target subsystems)

${pillars.map(p => `### ${p.pct}% · ${esc(p.name)} (weight ${p.weight})\n${esc(p.description)}\n\n- domains/crates: \`${esc(p.domains || p.crates || '—')}\` · verified ${p.verified}/${p.total}${p.basis === 'manual' ? ' (manual estimate)' : ''}\n`).join('\n')}

---
_Lowest-completion + highest-weight pillars are where end-state progress is most leveraged. Cross-reference [027-opportunities.md](027-opportunities.md): an opportunity's \`contributes_to\` edges (DIM 5) say which pillar it advances; \`caps:track sim <id>\` shows how far. Edit \`end-state.json\` to refine the vision._
`
const dest = join(GENERATED, '024-end-state.md')
writeFileSync(dest, md)
console.log(`wrote 024-end-state.md — Company OS ${overall}% across ${pillars.length} pillars`)
