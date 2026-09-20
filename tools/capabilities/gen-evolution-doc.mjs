#!/usr/bin/env node
// gen-evolution-doc.mjs — DIM 15 projection: docs/049-evolution.md. Organism mode — rank opportunities by
// future-EVOLUTION potential (learning/optionality/adaptability/knowledge), not near-term ROI, and surface
// the SLEEPERS the priority sort buries. Pure projection of caps.db.evolution_score + the strategy graph;
// regenerate with `pnpm caps:evolution-doc`.
import Database from 'better-sqlite3'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, GENERATED } from '../_paths.mjs'

import { organismRanking } from './tracking-lib.mjs'

mkdirSync(GENERATED, { recursive: true })

const db = new Database(CAPABILITIES_DB, { readonly: true })
const has = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='evolution_score'").get().n > 0
const rows = has ? organismRanking(db) : []
const titles = has ? Object.fromEntries(db.prepare('SELECT id,title FROM opportunity').all().map(r => [r.id, r.title])) : {}
db.close()
const esc = (s) => String(s ?? '').replace(/\|/g, '\\|').replace(/\n/g, ' ')
const sleepers = rows.filter(r => r.riser >= 8)

let md = `# 049 — Organism Mode / Evolution Scoring (DIM 15)

> The final lens. Not "what should be built next?" but **"if the Company OS were trying to maximize its own
> evolution, what would it build next?"** Here scoring is not priority/effort/ROI — it adds **learning value,
> future optionality, adaptability, and knowledge gain**. The point is to surface opportunities with low ROI
> today but high future-optionality (a capability that opens 30 later directions), and to flag high-priority
> point features that add little to what the organism can become. A pure projection of \`evolution_score\` +
> the strategy graph in \`docs/capabilities.db\`; durable source \`tools/capabilities/evolution-scores.json\`.
> Regenerate with \`pnpm caps:evolution-doc\`; live view \`pnpm caps:track evolve\`.
>
> **evolution_score = learning × optionality × adaptability × knowledge_gain** (each 1–5).
> **organism_score = evolution_score × (1 + 0.1 × transitive-unlocks)** (folds in the strategy graph).
> **riser = priority_rank − organism_rank** — how many ranks ABOVE its ROI rank organism mode places it.
> A large positive riser = a **sleeper** the near-term ROI sort buries.

## Sleepers — build for what they unlock later (organism mode ranks them far above ROI)

${sleepers.length ? sleepers.map(s => `- ↑${s.riser} \`${s.id}\` — ${esc(s.rationale || titles[s.id] || '')}`).join('\n') : '_none_'}

## Ranked by organism score (future-evolution potential)

| organism | evo | unlocks | riser | learn | opt | adapt | know | opportunity |
|--:|--:|--:|--:|--:|--:|--:|--:|---|
${rows.map(r => `| ${r.organism_score} | ${r.evolution_score} | ${r.transitive} | ${r.riser > 0 ? '+' + r.riser : r.riser} | ${r.learning_value ?? '—'} | ${r.optionality ?? '—'} | ${r.adaptability ?? '—'} | ${r.knowledge_gain ?? '—'} | \`${r.id}\` |`).join('\n')}

---
_Score opportunities in \`tools/capabilities/evolution-scores.json\` then \`pnpm caps:rebuild\`. Use this alongside [027-opportunities](027-opportunities.md) (ROI) and [028-strategy-graph](028-strategy-graph.md) (leverage): ROI says what pays now, organism mode says what compounds. The healthiest backlog spends on both._
`
const dest = join(GENERATED, '049-evolution.md')
writeFileSync(dest, md)
console.log(`wrote 049-evolution.md — ${rows.length} scored, ${sleepers.length} sleepers; top ${rows[0]?.id ?? 'none'}`)
