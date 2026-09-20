#!/usr/bin/env node
// gen-donor-util-doc.mjs — DIM 6 projection: docs/029-donor-utilization.md. Per-donor exploitation so
// scouted repos don't get forgotten. Pure projection; regenerate with `pnpm caps:donor-doc`.
import Database from 'better-sqlite3'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, GENERATED } from '../_paths.mjs'

import { donorUtil } from './tracking-lib.mjs'

mkdirSync(GENERATED, { recursive: true })

const db = new Database(CAPABILITIES_DB, { readonly: true })
const rows = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='donor_scope'").get().n ? donorUtil(db) : []
db.close()
const esc = (s) => String(s ?? '').replace(/\|/g, '\\|')

const totalTrue = rows.reduce((a, r) => a + r.tc, 0)
const totalVerified = rows.reduce((a, r) => a + r.verified, 0)
const exhausted = rows.filter(r => r.exploited_pct >= 70)
const under = rows.filter(r => r.exploited_pct < 20)
const untouched = rows.filter(r => r.extracted_pct === 0)
const byExploit = rows.slice().sort((a, b) => a.exploited_pct - b.exploited_pct || b.tc - a.tc)

let md = `# 029 — Donor Utilization Intelligence (DIM 6)

> The ${rows.length} donor repos under \`Temporary/\` are **knowledge bases**, not one-shot code dumps.
> This tracks how much of each has actually been mined + built, so a scouted repo never gets forgotten.
> A pure projection (derived from \`donor_scope\` + \`source_capability\` + \`canonical_capability\` in
> \`docs/capabilities.db\`); regenerate with \`pnpm caps:donor-doc\`; live view \`pnpm caps:track donors\`.
>
> - **scouted%** = distinct canonical caps this donor contributed / its true_capabilities (mined into the census)
> - **built%** = verified canonicals / mined canonicals (of what's mined, how much is implemented)
> - **exploited%** = verified canonicals / true_capabilities (the bottom line)
> - **remaining** = true_capabilities − verified (rough untapped value)

## Summary

- Total donor capability surface: **${totalTrue}** · verified into Chronica: **${totalVerified}** (≈ ${Math.round(100 * totalVerified / (totalTrue || 1))}% overall).
- **${exhausted.length}** donors ≥70% exploited (nearly exhausted). **${under.length}** under 20% (under-utilized — re-mine via discovery). **${untouched.length}** never scouted (0%).

## Under-utilized — biggest untapped value first

| donor | true | scouted% | built% | exploited% | remaining | disposition |
|---|--:|--:|--:|--:|--:|---|
${byExploit.filter(r => r.exploited_pct < 30).slice(0, 30).map(r => `| ${esc(r.donor)} | ${r.tc} | ${r.extracted_pct}% | ${r.built_pct}% | ${r.exploited_pct}% | ${r.remaining} | ${esc(r.disposition || '')} |`).join('\n')}

## Nearly exhausted (≥70% exploited)

${exhausted.length ? exhausted.sort((a, b) => b.exploited_pct - a.exploited_pct).map(r => `- ${esc(r.donor)} — ${r.exploited_pct}% (${r.verified}/${r.tc})`).join('\n') : '_none yet_'}

---
_Under-utilized high-true donors are prime discovery targets — feed them to \`Workflow({scriptPath:'tools/capabilities/discover-opportunities.workflow.mjs'})\`. \`preserve_as_reference_only\` donors are intentionally not ported (adopt the PATTERN, not the code)._
`
const dest = join(GENERATED, '029-donor-utilization.md')
writeFileSync(dest, md)
console.log(`wrote 029-donor-utilization.md — ${rows.length} donors, ${under.length} under-utilized, ${untouched.length} untouched`)
