#!/usr/bin/env node
// gen-external-doc.mjs — External Reality Loop projection: docs/051-external-reality.md. Outward signals
// (markets + OSS/AI/browser/agent ecosystems) that should re-score the roadmap — because the best roadmap
// for 2025 is not the best for 2026. Pure projection of caps.db.external_signal; durable source
// tools/capabilities/external-signals.json (refreshed by the scan-external-reality workflow).
import Database from 'better-sqlite3'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, GENERATED } from '../_paths.mjs'

import { externalSignals } from './tracking-lib.mjs'

mkdirSync(GENERATED, { recursive: true })

const db = new Database(CAPABILITIES_DB, { readonly: true })
const has = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='external_signal'").get().n > 0
const x = has ? externalSignals(db) : { count: 0, byCategory: {}, all: [], lastObserved: null }
db.close()
const esc = (s) => String(s ?? '').replace(/\|/g, '\\|').replace(/\n/g, ' ')
// group by the roadmap node each signal re-scores
const byAffects = {}
for (const s of x.all) { (byAffects[s.affects_id] = byAffects[s.affects_id] || []).push(s) }
const ranked = Object.entries(byAffects).sort((a, b) => b[1].length - a[1].length)

let md = `# 051 — External Reality Loop

> Everything else in this tracker looks INWARD (caps.db, donors, the internal graph). This is the outward
> loop: dated developments in the markets + OSS/AI/browser/agent ecosystems that should re-score the
> roadmap — because the best roadmap for 2025 is not the best for 2026. Each signal names what it AFFECTS
> and a recommended action. Signals are evidence-level **L1** (web-scan, inferred from the open web). A pure
> projection of \`external_signal\` in \`docs/capabilities.db\`; durable source
> \`tools/capabilities/external-signals.json\`, refreshed by running the \`scan-external-reality\` workflow.
> Live view \`pnpm caps:track external [--all]\`.

## Summary

- **${x.count}** external signals; latest observed **${x.lastObserved || '—'}**.
- By category: ${Object.entries(x.byCategory).sort((a, b) => b[1] - a[1]).map(([k, n]) => `**${n}** ${k}`).join(' · ') || '—'}.

## Roadmap nodes the outside world is re-scoring (most signal first)

| signals | re-scores | what to do |
|--:|---|---|
${ranked.map(([aff, sigs]) => `| ${sigs.length} | \`${esc(aff)}\` | ${esc(sigs[0].recommended_action || '').slice(0, 160)} |`).join('\n')}

## Signals (by what they affect)

${ranked.map(([aff, sigs]) => `### Re-scores: ${esc(aff)}
${sigs.map(s => `- **[${esc(s.observed_at || '?')}]** _(${s.category}, confidence ${s.confidence}/5)_ ${esc(s.headline)}
  - ⇒ ${esc(s.recommended_action)}${s.url ? `\n  - ${s.url}` : ''}`).join('\n')}`).join('\n\n')}

---
_Refresh by re-running the outward scan: \`Workflow({scriptPath:'tools/capabilities/scan-external-reality.workflow.mjs'})\`, then transform its output into \`external-signals.json\` and \`pnpm caps:rebuild\`. A signal that points at an UNTRACKED bet (e.g. "MCP/agent-standards matter") is a prompt to add an [044-assumptions](044-assumptions.md) entry. A signal that challenges a load-bearing assumption (see [050-truth-engine](050-truth-engine.md)) should move that assumption's validation_status._
`
const dest = join(GENERATED, '051-external-reality.md')
writeFileSync(dest, md)
console.log(`wrote 051-external-reality.md — ${x.count} signals across ${ranked.length} re-scored roadmap nodes`)
