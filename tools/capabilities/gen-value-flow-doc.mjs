#!/usr/bin/env node
// gen-value-flow-doc.mjs — DIM 12 projection: docs/046-value-flow.md. Traces capability → user value →
// business value → revenue, and flags ORPHANS (capabilities with no revenue path). Pure projection of
// caps.db.value_node/value_edge; regenerate with `pnpm caps:value-doc`.
import Database from 'better-sqlite3'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, GENERATED } from '../_paths.mjs'

import { valueFlow } from './tracking-lib.mjs'

mkdirSync(GENERATED, { recursive: true })

const db = new Database(CAPABILITIES_DB, { readonly: true })
const has = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='value_node'").get().n > 0
const vf = has ? valueFlow(db) : { nodes: [], edges: [], flagged: [], untraced: [], revenueCount: 0 }
const edgesFull = has ? db.prepare('SELECT from_id, to_id, rationale FROM value_edge').all() : []
db.close()
const label = Object.fromEntries(vf.nodes.map(n => [n.id, n.label]))
const kind = Object.fromEntries(vf.nodes.map(n => [n.id, n.kind]))
const esc = (s) => String(s ?? '').replace(/\|/g, '\\|').replace(/\n/g, ' ')
const byKind = {}; for (const n of vf.nodes) byKind[n.kind] = (byKind[n.kind] || 0) + 1
const revenue = vf.nodes.filter(n => n.kind === 'revenue')

let md = `# 046 — Value Flow (DIM 12)

> Chronica is ultimately a product, not a tech demo. This traces how capability domains turn into value:
> **capability → user value → business value → revenue**. A capability with no honest path to revenue is an
> ORPHAN — the warning this dimension exists to raise. A pure projection of \`value_node\`/\`value_edge\` in
> \`docs/capabilities.db\`; durable source \`tools/capabilities/value-flow.json\`. Regenerate with
> \`pnpm caps:value-doc\`; live view \`pnpm caps:track value [--orphans]\`.

## Summary

- **${vf.nodes.length}** value nodes (${Object.entries(byKind).map(([k, n]) => `${k}: ${n}`).join(', ')}) · **${vf.edges.length}** edges · **${vf.revenueCount}** revenue sinks.
- **${vf.flagged.length}** grounded orphan(s) (no revenue path identified) · **${vf.untraced.length}** untraced branch(es) (dead-end before revenue — trust-infra or not yet wired).

## Revenue sinks (where value terminates)

${revenue.map(r => `- 💰 ${esc(r.label)}`).join('\n') || '_none_'}

## Grounded orphans — capabilities with NO revenue path identified

${vf.flagged.length ? vf.flagged.map(o => `- ⚠ ${esc(o.label)}`).join('\n') : '_none_'}

## Untraced branches — value chain dead-ends before revenue (wire or accept as trust-infra)

${vf.untraced.length ? vf.untraced.map(o => `- · ${esc(o.label)}`).join('\n') : '_none_'}

## Flow edges (capability → … → revenue)

| from | kind | → | to | kind | why |
|---|---|:-:|---|---|---|
${edgesFull.map(e => `| ${esc(label[e.from_id] || e.from_id)} | ${kind[e.from_id] || ''} | → | ${esc(label[e.to_id] || e.to_id)} | ${kind[e.to_id] || ''} | ${esc(e.rationale)} |`).join('\n')}

---
_Add flows + orphans to \`tools/capabilities/value-flow.json\` then \`pnpm caps:rebuild\`. A capability that stays an orphan after honest tracing is a candidate to **descope, reframe, or connect to a revenue path** — see [027-opportunities](027-opportunities.md)._
`
const dest = join(GENERATED, '046-value-flow.md')
writeFileSync(dest, md)
console.log(`wrote 046-value-flow.md — ${vf.nodes.length} nodes, ${vf.flagged.length} grounded orphans (+${vf.untraced.length} untraced)`)
