#!/usr/bin/env node
// gen-truth-doc.mjs — THE TRUTH LAYER projection: docs/050-truth-engine.md. Evidence quality (L0-L4) +
// confidence propagation. The lens that separates fact / inference / assumption / validated reality, so
// the planning brain cannot believe its own reasoning as if it were evidence. Pure projection of
// caps.db.node_evidence; regenerate with `pnpm caps:truth-doc`.
import Database from 'better-sqlite3'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, GENERATED } from '../_paths.mjs'

import { evidenceLedger, confidencePropagation } from './tracking-lib.mjs'

mkdirSync(GENERATED, { recursive: true })

const db = new Database(CAPABILITIES_DB, { readonly: true })
const has = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='node_evidence'").get().n > 0
const led = has ? evidenceLedger(db) : null
const prop = has ? confidencePropagation(db) : []
const byKindLevel = has ? db.prepare('SELECT node_kind, evidence_level, count(*) n FROM node_evidence GROUP BY node_kind, evidence_level').all() : []
db.close()
const esc = (s) => String(s ?? '').replace(/\|/g, '\\|').replace(/\n/g, ' ')
const lv = led?.byLevel || {}
const proved = (lv.L3 || 0) + (lv.L4 || 0), code = lv.L2 || 0, think = (lv.L0 || 0) + (lv.L1 || 0)
const kinds = [...new Set(byKindLevel.map(r => r.node_kind))]
const cell = (k, l) => byKindLevel.find(r => r.node_kind === k && r.evidence_level === l)?.n || ''

let md = `# 050 — Truth Engine: Evidence Quality + Confidence Propagation

> The layer that sits UNDER all 15 dimensions. A planning brain's worst failure mode is trusting its own
> inference as if it were evidence — a self-authored opinion and a runtime-proven fact looking identical in
> the graph. This separates them. Every meta-node carries an **evidence_level**, and the cardinal rule is
> **EVIDENCE CAPS CONFIDENCE**. A pure projection of \`node_evidence\` in \`docs/capabilities.db\`; durable
> source \`tools/capabilities/evidence.json\`. Regenerate with \`pnpm caps:truth-doc\`; live view
> \`pnpm caps:track truth\` / \`pnpm caps:track confidence\`.

## The evidence ladder (and its confidence ceiling)

| level | meaning | confidence ceiling |
|---|---|--:|
| **L0** | opinion — author judgment, no external support | 0.40 |
| **L1** | inferred — read from code / git / memory / web, but not verified | 0.60 |
| **L2** | code-verified — a real code or test reference exists | 0.80 |
| **L3** | runtime-verified — a passing live / smoke run | 0.95 |
| **L4** | production-verified — proven in a deployed production run | 1.00 |

\`intrinsic_confidence = min(stated_confidence, ceiling(level))\` — you cannot believe an L0 opinion at 90%.

## What do I KNOW vs THINK vs have PROVEN?

${led ? `- **KNOW** (runtime/production, L3-L4): **${proved}** meta-nodes${led.cap ? ` + **${led.cap.L3_money_runtime}** money caps runtime-gated` : ''}.
- **CODE-VERIFIED** (L2): **${code}** meta-nodes${led.cap ? ` + **${led.cap.L2_code_verified}** capabilities (code+test)` : ''}.
- **THINK** (opinion/inference, L0-L1): **${think}** meta-nodes${led.cap ? ` + **${led.cap.L0_unimplemented}** unproven capability claims` : ''}.
- **PRODUCTION (L4): ${led.cap ? led.cap.L4_production : 0}** — nothing is production-proven yet. The honest ceiling on every claim.
- **Self-authored opinion/inference: ${led.selfAuthored}** — the part most at risk of the brain believing its own reasoning.` : '_evidence layer not built_'}

## Evidence by node kind

| kind | L0 | L1 | L2 | L3 | L4 |
|---|--:|--:|--:|--:|--:|
${kinds.map(k => `| ${k} | ${cell(k, 'L0')} | ${cell(k, 'L1')} | ${cell(k, 'L2')} | ${cell(k, 'L3')} | ${cell(k, 'L4')} |`).join('\n')}

## ⚠ Fragile opinions load-bearing the roadmap

L0/L1 assumptions that underpin active opportunities — validate these before building further on what rests on them:

${led && led.riskyLoadBearing.length ? led.riskyLoadBearing.map(a => `- ⚠ \`${a}\``).join('\n') : '_none_'}

## Confidence propagation (weakest-link)

> \`effective_confidence = min(intrinsic, min over the nodes it rests on)\`. A roadmap is at most as
> trustworthy as the weakest bet it rests on. These opportunities carry **less real confidence than they
> appear to** — each names the support dragging it down.

| intrinsic | → effective | opportunity | capped by |
|--:|--:|---|---|
${prop.length ? prop.map(r => `| ${r.intrinsic.toFixed(2)} | ${r.effective.toFixed(2)} | \`${r.id}\` | ${r.culprit ? `\`${esc(r.culprit)}\` (${(r.culprit_conf ?? 0).toFixed(2)})` : '—'} |`).join('\n') : '| | | _none dragged_ | |'}

---
_Set evidence levels in \`tools/capabilities/evidence.json\` (defaults are honest-by-construction: self-authored = L0/L1; overrides pin high-stakes nodes against real evidence refs) then \`pnpm caps:rebuild\`. As a bet is validated, raise its level and its ceiling rises with it. See [044-assumptions](044-assumptions.md) (the bets) and [051-external-reality](051-external-reality.md) (what the outside world says about them)._
`
const dest = join(GENERATED, '050-truth-engine.md')
writeFileSync(dest, md)
console.log(`wrote 050-truth-engine.md — ${led?.total ?? 0} nodes leveled (KNOW ${proved}/L2 ${code}/THINK ${think}); ${prop.length} confidence-dragged`)
