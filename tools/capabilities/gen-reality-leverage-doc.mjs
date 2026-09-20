#!/usr/bin/env node
// gen-reality-leverage-doc.mjs — REALITY LEVERAGE projection: docs/053-reality-leverage.md. The objective-
// function flip: the roadmap re-scored by expected_new_L4 / effort, the L4 dependency path (what unlocks
// the FIRST L4), and Time-to-Reality. Pure projection of caps.db.reality_milestone + opportunity.expected_l4;
// regenerate with `pnpm caps:reality-leverage-doc`.
import Database from 'better-sqlite3'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, GENERATED } from '../_paths.mjs'

import { realityLeverage, timeToReality } from './tracking-lib.mjs'

mkdirSync(GENERATED, { recursive: true })

const db = new Database(CAPABILITIES_DB, { readonly: true })
const has = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='reality_milestone'").get().n > 0
const rl = has ? realityLeverage(db) : null
const ttr = has ? timeToReality(db) : null
db.close()
const esc = (s) => String(s ?? '').replace(/\|/g, '\\|').replace(/\n/g, ' ')
const mk = (s) => s === 'done' ? '✅' : s === 'in_progress' ? '◐' : s === 'blocked' ? '⛔' : '○'

let md = `# 053 — Reality Leverage: the objective-function flip

> Once a Reality Engine exists, the system's objective must change. The old objective — "build capability →
> gain progress" — now mis-aligns with reality (the tracker measures reality at 0% but still ranks roadmap
> by build-leverage). The new dominant objective is **create L4 evidence → gain progress**, scored by
> **Reality Leverage = expected_new_L4 / effort**. Under it, "build 100 capabilities" loses to "drive one
> real invoice through the gate". A pure projection of \`reality_milestone\` + \`opportunity.expected_l4\` in
> \`docs/capabilities.db\`; durable source \`tools/capabilities/reality-path.json\`. Live view
> \`pnpm caps:track realize\`.

## ⏱ Time-to-Reality

${ttr ? `- **Last L4: ${ttr.lastL4 || 'NEVER'}**${ttr.daysSinceLastL4 != null ? ` (${ttr.daysSinceLastL4} days ago)` : ' (∞)'} — a stronger strategic signal than any completion %.
- Next milestone: **${ttr.next ? esc(ttr.next.label) : '—'}**${ttr.blocker ? ` — ⛔ ${esc(ttr.blocker)}` : ''}.
- To the **first L4** (${esc(ttr.firstL4Milestone?.label || '?')}): **${ttr.effortToFirstL4}** effort-units. To the **keystone money-L4**: **${ttr.effortToKeystone}** (${ttr.milestonesRemaining}/${ttr.milestonesTotal} milestones remaining).` : '_reality path not built_'}

## The L4 Dependency Graph — what unlocks the FIRST L4?

> Not "what unlocks the workflow engine?" but "what unlocks the first real-world evidence?". The critical
> path, in order. ★L4 = completing it records a real L4 reality event.

| # | status | milestone | effort | ★L4 | delivered by | blocked by |
|--:|:-:|---|--:|:-:|---|---|
${rl ? rl.milestones.map(m => `| ${m.ordinal} | ${mk(m.status)} | ${esc(m.label)} | ${m.effort} | ${m.produces_l4 ? '★' : ''} | ${esc(m.delivered_by)} | ${esc(m.blocked_by)} |`).join('\n') : ''}

## Roadmap re-ranked by Reality Leverage (the dominant objective)

> The minimal path to reality beats the biggest internal build. The long tail is capability work that
> builds the system but creates zero real-world evidence — correctly demoted to 0.00 under this objective.

| reality_leverage | kind | item | effort | +L4 |
|--:|---|---|--:|--:|
${rl ? rl.top.map(x => `| ${x.reality_leverage} | ${x.kind} | \`${esc(x.id)}\` | ${x.effort} | ${x.expected_l4} |`).join('\n') : ''}

${rl ? `\n_…then **${rl.zeroL4}** capability opportunities at reality_leverage **0.00** — including the old build-leverage top picks (CQRS backbone, etc.). They build the system; they do not create reality. Under the new objective they wait until they are on the path to an L4 event._` : ''}

---
_Edit the path + the per-opportunity expected_l4 in \`tools/capabilities/reality-path.json\` then \`pnpm caps:rebuild\`. As a milestone is completed in the REAL world, record its reality_event ([052-reality-engine](052-reality-engine.md)) and set the milestone \`status: done\`. The first \`status: done\` with produces_l4 lifts the Reality Ratio off 0 — the moment the system starts learning from the world, not just from itself._
`
const dest = join(GENERATED, '053-reality-leverage.md')
writeFileSync(dest, md)
console.log(`wrote 053-reality-leverage.md — ${rl ? rl.milestones.length : 0} milestones, ${rl ? rl.zeroL4 : 0} zero-leverage opps; last L4 ${ttr?.lastL4 || 'never'}`)
