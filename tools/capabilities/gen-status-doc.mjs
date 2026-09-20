#!/usr/bin/env node
// gen-status-doc.mjs — project docs/capabilities.db into docs/026-status.md: the ONE human-facing
// STATE + PROGRESS + ISSUES surface. A pure DB projection (like 025-build-priority-queue): regenerate
// with `pnpm caps:status-doc` (or `pnpm docs:gen`). The durable issue source is tracking-issues.json.
import Database from 'better-sqlite3'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, GENERATED } from '../_paths.mjs'

import { attachCanonicalCapabilityFallback } from './canonical-fallback.mjs'

mkdirSync(GENERATED, { recursive: true })

const db = new Database(CAPABILITIES_DB, { readonly: true })
attachCanonicalCapabilityFallback(db)
const meta = Object.fromEntries(db.prepare('SELECT k,v FROM meta').all().map(r => [r.k, r.v]))
const byStatus = Object.fromEntries(db.prepare('SELECT status, count(*) n FROM canonical_capability GROUP BY status').all().map(r => [r.status, r.n]))
const money = db.prepare('SELECT count(*) n FROM canonical_capability WHERE moves_money=1').get().n
const moneyVerified = db.prepare("SELECT count(*) n FROM canonical_capability WHERE moves_money=1 AND status='verified'").get().n
const moneyUnguarded = db.prepare('SELECT count(*) n FROM canonical_capability WHERE moves_money=1 AND requires_approval=0').get().n
const verifiedMoneyNoFin = db.prepare("SELECT count(*) n FROM canonical_capability WHERE moves_money=1 AND status='verified' AND COALESCE(financial_control_test,'')=''").get().n
const hasIssues = db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name='tracking_issue'").get().n > 0
const issues = hasIssues ? db.prepare('SELECT id, severity, area, title, status, source, autonomous_safe, evidence_ref, fix_ref FROM tracking_issue').all() : []
db.close()

const SEV = ['P0', 'P1', 'P2', 'P3', 'P4']
const open = issues.filter(i => i.status === 'open' || i.status === 'in_progress')
const resolved = issues.filter(i => !(i.status === 'open' || i.status === 'in_progress'))
const esc = (s) => String(s ?? '').replace(/\|/g, '\\|').replace(/\n/g, ' ')

let md = `# 026 — Status (state · progress · issues)

> The single human-facing tracking surface. A **pure projection** of \`docs/capabilities.db\` (state +
> progress) and \`tools/capabilities/tracking-issues.json\` (issues) — regenerate with \`pnpm caps:status-doc\`
> (or \`pnpm docs:gen\`). For a live terminal view use \`pnpm caps:track snapshot\`; record/resolve issues with
> \`pnpm caps:track add|resolve\`. Numbers are kept true by \`reconcile-meta.mjs\`; issues by
> \`reconcile-tracking-issues.mjs\` (both run in \`caps:rebuild\`).

## State (caps.db meta)

| Surface | Value |
|---|---|
| Canonical capabilities | ${meta.canonical_capability_count ?? '?'} |
| Verified | ${meta.canonical_verified_count ?? '?'} |
| Money-moving | ${money} (${moneyVerified} verified) |
| Source capabilities (74 donors) | ${meta.source_capability_count ?? '?'} |
| Meta last reconciled | ${meta.meta_reconciled_at ?? '(unset)'} |

## Progress (canonical_capability.status)

| Status | Count |
|---|---|
${Object.entries(byStatus).map(([s, n]) => `| ${s} | ${n} |`).join('\n')}

Build queue (what's next): [025-build-priority-queue.md](025-build-priority-queue.md). Per-domain roadmaps: 030-043, 156-158.

## Money safety (invariants)

| Check | Result |
|---|---|
| \`moves_money=1 ∧ requires_approval=0\` | ${moneyUnguarded} ${moneyUnguarded === 0 ? '✅' : '❌ UNGUARDED'} |
| verified money caps without a financial_control_test | ${verifiedMoneyNoFin} ${verifiedMoneyNoFin === 0 ? '✅' : '❌'} |

## Issues — OPEN (${open.length})

`
if (open.length) {
  md += `| sev | area | id | fix-mode | title | evidence |\n|---|---|---|---|---|---|\n`
  for (const s of SEV) for (const i of open.filter(x => x.severity === s)) {
    md += `| ${i.severity} | ${i.area} | \`${i.id}\` | ${i.autonomous_safe ? 'auto-safe' : 'human' } | ${esc(i.title)} | ${esc(i.evidence_ref)} |\n`
  }
} else {
  md += `_none open._\n`
}

md += `\n## Issues — RESOLVED (${resolved.length})

| sev | area | id | status | title | fix |
|---|---|---|---|---|---|
${resolved.map(i => `| ${i.severity} | ${i.area} | \`${i.id}\` | ${i.status} | ${esc(i.title)} | ${esc(i.fix_ref)} |`).join('\n')}

---
_Issues live in \`tools/capabilities/tracking-issues.json\` (committed; survives the gitignored caps.db). Areas: money · architecture · evidence · product · tenancy · security · docs · tests · process · meta._
`

const dest = join(GENERATED, '026-status.md')
writeFileSync(dest, md)
console.log(`wrote 026-status.md — state(${meta.canonical_verified_count}/${meta.canonical_capability_count} verified) · issues(${open.length} open, ${resolved.length} resolved)`)
