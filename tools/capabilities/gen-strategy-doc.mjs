#!/usr/bin/env node
// gen-strategy-doc.mjs — DIM 5/8 projection: docs/028-strategy-graph.md. The opportunity graph ranked
// by architectural leverage (downstream unlocks), the unlock chains, dead-ends, blocked nodes, and the
// execution-simulation view. Pure projection; regenerate with `pnpm caps:strategy-doc`.
import Database from 'better-sqlite3'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, GENERATED } from '../_paths.mjs'

import { graphStats, loadEdges } from './tracking-lib.mjs'

mkdirSync(GENERATED, { recursive: true })

const db = new Database(CAPABILITIES_DB, { readonly: true })
const has = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='strategy_edge'").get().n > 0
const stats = has ? graphStats(db) : new Map()
const edges = has ? loadEdges(db) : []
const titles = Object.fromEntries(db.prepare('SELECT id,title FROM opportunity').all().map(r => [r.id, r.title]))
db.close()
const esc = (s) => String(s ?? '').replace(/\|/g, '\\|').replace(/\n/g, ' ')
const rows = [...stats.entries()]
const byLeverage = rows.slice().sort((a, b) => b[1].transitive - a[1].transitive || b[1].graph_priority - a[1].graph_priority)
const deadEnds = rows.filter(([, s]) => s.dead_end).map(([i]) => i)
const blocked = rows.filter(([, s]) => s.blockedBy.length)
const unlockEdges = edges.filter(e => e.relation === 'unlocks')

let md = `# 028 — Strategy Graph & Execution Simulation (DIM 5 + 8)

> The backlog as a **graph**, not a list. Directed edges (\`unlocks\`/\`depends_on\`/\`blocks\`/\`obsoletes\`/
> \`contributes_to\`) between opportunities, issues, and end-state pillars. A pure projection of
> \`strategy_edge\` in \`docs/capabilities.db\`; durable source \`tools/capabilities/strategy-edges.json\`.
> Regenerate with \`pnpm caps:strategy-doc\`; live view \`pnpm caps:track graph\` / \`caps:track sim <id>\`.
> **graph_priority = base priority × (1 + 0.15·downstream-unlocks + 0.1·issues-obsoleted), halved if
> blocked** — so a node's rank reflects what completing it unlocks (a finished node re-scores the rest).

## Highest architectural leverage (unlocks the most downstream)

| unlocks↓ | obsoletes | base→graph priority | opportunity | status |
|--:|--:|--:|---|---|
${byLeverage.slice(0, 15).map(([id, s]) => `| ${s.transitive} | ${s.obsoletes.length} | ${s.priority} → **${s.graph_priority}** | \`${id}\`${s.blockedBy.length ? ' ⛔' : ''} | ${esc(titles[id] || '')} |`).join('\n')}

## Unlock chains (build upstream first)

${unlockEdges.length ? unlockEdges.map(e => `- \`${e.from_id}\` → **unlocks** → \`${e.to_id}\`${e.rationale ? `  _(${esc(e.rationale)})_` : ''}`).join('\n') : '_none_'}

## Blocked (an open issue stops these)

${blocked.length ? blocked.map(([id, s]) => `- \`${id}\` ⛔ blocked by open issue(s): ${s.blockedBy.map(b => `\`${b}\``).join(', ')}`).join('\n') : '_none blocked_'}

## Dead-ends (no downstream unlock / issue / pillar — low strategic multiplier)

${deadEnds.length ? deadEnds.map(i => `\`${i}\``).join(', ') : '_none_'}

---
_To simulate building one: \`pnpm caps:track sim <opportunity-id>\` → "+N unlocked, +M issues resolved, +K pillars, ROI". Add edges in \`strategy-edges.json\` as new dependencies are discovered (the execution feedback loop re-runs after each implementation)._
`
const dest = join(GENERATED, '028-strategy-graph.md')
writeFileSync(dest, md)
console.log(`wrote 028-strategy-graph.md — ${edges.length} edges; top leverage ${byLeverage[0]?.[0] ?? 'none'} (unlocks ${byLeverage[0]?.[1].transitive ?? 0})`)
