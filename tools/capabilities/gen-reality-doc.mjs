#!/usr/bin/env node
// gen-reality-doc.mjs — THE REALITY ENGINE (DIM 16) projection: docs/052-reality-engine.md. Did it
// actually HAPPEN? Reality events + the Reality Ratio + outcome graph + prediction market. Pure projection
// of caps.db.reality_event/outcome/prediction; regenerate with `pnpm caps:reality-doc`.
import Database from 'better-sqlite3'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, GENERATED } from '../_paths.mjs'

import { realityEngine, predictionMarket } from './tracking-lib.mjs'

mkdirSync(GENERATED, { recursive: true })

const db = new Database(CAPABILITIES_DB, { readonly: true })
const has = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='reality_event'").get().n > 0
const r = has ? realityEngine(db) : null
const p = has ? predictionMarket(db) : null
const events = has ? db.prepare('SELECT * FROM reality_event ORDER BY occurred_at DESC').all() : []
const outcomes = has ? db.prepare('SELECT * FROM outcome ORDER BY status, id').all() : []
const preds = has ? db.prepare('SELECT * FROM prediction ORDER BY expiry').all() : []
db.close()
const esc = (s) => String(s ?? '').replace(/\|/g, '\\|').replace(/\n/g, ' ')

let md = `# 052 — Reality Engine (DIM 16): did it actually HAPPEN?

> The layer the whole stack ultimately answers to. Everything above asks "is it implemented?"; this asks
> "did it actually **happen**?". L3 = code exists + tests pass. **L4 = a real request flowed through it,
> real (non-synthetic) data passed, a real user touched it, the expected outcome occurred.** L4 is
> **NON-HAND-ASSIGNABLE** — it is earned only by a recorded reality_event with \`user_real\` + \`data_real\`,
> never claimed in \`evidence.json\` (reconcile-evidence caps any unbacked L4 down to L3). That is the
> structural answer to *"who evidences the evidencer?"*: the top rung cannot be authored, only lived. A
> pure projection of \`reality_event\` / \`outcome\` / \`prediction\` in \`docs/capabilities.db\`; durable sources
> \`tools/capabilities/{reality-events,outcomes,predictions}.json\`. Live view \`pnpm caps:track reality\`.

## ★ Reality Ratio

${r ? `**${r.realityRatio}%** — L4 production nodes (**${r.l4Nodes}**) / strategic claims (**${r.strategic}**). Company-wide incl. capabilities: **${r.companyRatio}%**.

This is **PRODUCTION reality** — real user + real data. **0% does NOT mean nothing happened**; it means no real user has touched a slice yet. The full evidence **ladder** over the same ${r.strategic} strategic nodes:

| ratio | definition | value |
|---|---|---|
| **Reality (production)** | L4 / strategic | **${r.realityRatio}%** (${r.l4Nodes}) |
| **Runtime (engineering)** | (L3+L4) / strategic | **${r.runtimeRatio}%** (${r.runtimeNodes}) |
| **Code (built+verified)** | (L2+L3+L4) / strategic | **${r.codeRatio}%** (${r.codeNodes}) |
| **Human-usage** | real-user nodes / strategic | **${r.humanUsageRatio}%** (${r.humanUsageNodes}) |

Runtime + code reality are **real and substantial** (capabilities separately: hundreds code+test, dozens runtime-gated). The one gap is **production reality**, which only a real user closes.

The bottleneck of the entire Company OS is no longer architecture, capability, or planning — it is getting
**one complete slice (real request → money gate → approval → execution → audit) to L4** with a real user.
Objective: primary = raise production reality (needs a real user) · secondary = raise runtime+code reality · tertiary = donor exploitation. **Do not idle waiting for L4.**` : '_reality engine not built_'}

## Reality events — what has actually run

${r ? `**${r.l3events}** L3_synthetic (real runtime, test user + test data) · **${r.l4events}** L4_production (real user + real data).` : ''}

| level | when | type | real user | real data | what happened | evidence |
|---|---|---|:-:|:-:|---|---|
${events.map(e => `| ${e.reality_level === 'L4_production' ? '**L4**' : 'L3'} | ${e.occurred_at || '?'} | ${e.event_type || ''} | ${e.user_real ? '✓' : '·'} | ${e.data_real ? '✓' : '·'} | ${esc(e.description)} | ${esc(e.evidence_ref)} |`).join('\n')}

## Outcome graph — results, not builds

> "Built X" is not the point; "X produced result Y" is. \`actual\` stays empty until reality yields a number.

${r ? `**${r.outcomes.measured}** measured / **${r.outcomes.total}** — the system has shipped capability, not yet outcome.` : ''}

| status | subject | metric | baseline → target | actual |
|---|---|---|---|---|
${outcomes.map(o => `| ${o.status} | \`${esc(o.subject_id)}\` | ${esc(o.metric)} | ${esc(o.baseline)} → ${esc(o.target)} | ${o.actual ? esc(o.actual) : '—'} |`).join('\n')}

## Prediction market — the brain grading itself

> Every load-bearing bet becomes a falsifiable prediction with an expiry. As they resolve, the Brier score
> (mean (confidence − outcome)², lower is better) tells the brain how well-calibrated its own forecasts are.

${p ? `**${p.resolved}/${p.total}** resolved${p.brierMean != null ? ` · Brier **${p.brierMean}**` : ' — never tested against reality yet'}. ${p.overdue.length ? `⏰ **${p.overdue.length}** past expiry (reality is due — resolve them).` : 'None past expiry yet.'}` : ''}

| resolution | expiry | conf | subject | prediction |
|---|---|--:|---|---|
${preds.map(pr => `| ${pr.resolution} | ${pr.expiry || '?'} | ${pr.confidence} | \`${esc(pr.subject_id)}\` | ${esc(pr.prediction)} |`).join('\n')}

---
_Record a real occurrence in \`reality-events.json\` (only L4 with user_real+data_real counts toward the Reality Ratio). Resolve a prediction by setting its \`resolution\` in \`predictions.json\` then \`pnpm caps:rebuild\` (Brier recomputes). Fill an \`outcome.actual\` when reality produces the number. See [050-truth-engine](050-truth-engine.md) — reality is the only thing that can lift a node to L4._
`
const dest = join(GENERATED, '052-reality-engine.md')
writeFileSync(dest, md)
console.log(`wrote 052-reality-engine.md — Reality Ratio ${r ? r.realityRatio : '?'}% (${r ? r.l4events : 0} L4 events); ${preds.length} predictions, ${outcomes.length} outcomes`)
