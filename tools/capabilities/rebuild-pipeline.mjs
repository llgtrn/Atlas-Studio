#!/usr/bin/env node
// rebuild-pipeline.mjs — the canonical census REBUILD ORDER, in one place.
//
// The capability DB is rebuilt by several steps that WIPE and recreate tables
// (apply-clusters DELETEs canonical_capability; build-db DROPs capability). Doc-authored
// status and the slice<->canonical link live in SURVIVABLE side-tables and must be
// re-applied AFTER the wipe, in a fixed order. This script encodes that order so the
// "both surfaces stay in sync" guarantee can never be broken by running steps out of order.
//
// It does NOT re-run the expensive census EXTRACTION (the donor source-reading agent passes)
// — those produce source_capability rows and are run manually with a workflow journal. This
// orchestrates the deterministic DB-shaping steps that run on the already-ingested data.
//
// Order (each idempotent):
//   1. census-schema        ensure all tables incl. survivable side-tables exist
//   2. reconcile-domains    fix mis-clustered noise.* -> true domain
//   3. link-slices          rebuild slice_canonical (the bridge) + derive canonical.slice
//   4. reapply-overrides    re-apply canonical_status_override + re-derive .slice
//   (then run `pnpm docs:gen` to project to docs, `pnpm docs:verify` to confirm.)
//
// NOTE: apply-clusters (the cluster wipe) and build-db (the slice-table rebuild) are run
// from their own workflow context; after either, run THIS to restore the derived state.
import { execFileSync } from 'node:child_process'
import { join } from 'node:path'

const steps = [
  ['census-schema.mjs', 'ensure schema + side-tables'],
  ['import-data-ledgers.mjs', 'fold _machine ledgers (absorption/retirement/exec-state) into DB'],
  ['reconcile-domains.mjs', 'fix mis-clustered noise.* domains'],
  ['link-slices.mjs', 'rebuild slice<->canonical bridge'],
  ['reapply-overrides.mjs', 're-apply survivable doc/CLI status + derive .slice'],
  ['reconcile-w9-target-keys.mjs', 're-apply w9-target-reconciliation.json (issue #713 Wave 9 target-key reconciliation, never verified)'],
  ['reconcile-helpdesk-evidence.mjs', 're-point live + downgrade unbuilt helpdesk caps (de-fork inbox/* deleted)'],
  ['reconcile-test-as-impl-evidence.mjs', 're-point test-as-impl rows to their real sibling impl modules (money-safe)'],
  ['reconcile-report-evidence.mjs', 're-derive verified caps from committed crate reconcile reports with real Rust tests'],
  ['reconcile-meta.mjs', 'recompute meta headline counts from live tables (verified/money/denominator/source/slices)'],
  ['reconcile-tracking-issues.mjs', 're-apply the committed tracking-issues.json into caps.db.tracking_issue (state/progress/issue tracker)'],
  ['reconcile-opportunities.mjs', 're-apply opportunities.json into caps.db.opportunity + recompute ev/priority (strategic backlog)'],
  ['reconcile-end-state.mjs', 're-apply end-state.json into caps.db.end_state_pillar (DIM 7 — Company-OS target)'],
  ['reconcile-strategy-edges.mjs', 're-apply strategy-edges.json into caps.db.strategy_edge (DIM 5 — the strategy graph)'],
  ['reconcile-assumptions.mjs', 're-apply assumptions.json into caps.db.assumption (DIM 10 — are the bets still true)'],
  ['reconcile-decisions.mjs', 're-apply decisions.json into caps.db.architecture_decision (DIM 11 — ADR memory)'],
  ['reconcile-value-flow.mjs', 're-apply value-flow.json into caps.db.value_node/value_edge (DIM 12 — capability→revenue)'],
  ['reconcile-competitive.mjs', 're-apply competitive.json into caps.db.competitor/competitor_feature (DIM 13 — market)'],
  ['reconcile-arch-debt.mjs', 're-apply arch-debt.json into caps.db.arch_debt (DIM 14 — works-but-expensive)'],
  ['reconcile-evolution.mjs', 're-apply evolution-scores.json into caps.db.evolution_score (DIM 15 — organism mode)'],
  ['reconcile-reality.mjs', 'THE REALITY ENGINE (DIM 16) — reality events + outcomes + predictions; refuses hand-claimed L4 (must run BEFORE reconcile-evidence so its L4 guard sees real events)'],
  ['reconcile-reality-path.mjs', 'REALITY LEVERAGE — the L4 dependency path + per-opportunity expected_l4 (re-points the objective to creating real-world evidence)'],
  ['reconcile-evidence.mjs', 'THE TRUTH LAYER — evidence-level every meta-node + cap+propagate confidence (must run after all dimension reconciles)'],
  ['reconcile-external.mjs', 're-apply external-signals.json into caps.db.external_signal (External Reality Loop)'],
  ['verify-impl-evidence.mjs', 'audit that every verified cap has real present Chronica code'],
]

const here = join(process.cwd(), 'tools', 'capabilities')
for (const [script, desc] of steps) {
  console.log(`\n── ${script} — ${desc}`)
  execFileSync(process.execPath, [join(here, script)], { stdio: 'inherit' })
}
console.log('\n✓ rebuild-pipeline complete. Next: `pnpm docs:gen` then `pnpm docs:verify`.')
