// Seed the design-provenance canonical capability for the READ-ONLY workflow PLAN preview
// route `POST /api/companies/:id/workflows/plan` (chronica-api::routes::workflows). This is
// architecture-driven (a new HTTP surface over the EXISTING pure planner
// chronica_workflows::run_workflow), not donor-sourced; it enters the registry as
// `unimplemented`, then `record-verified --config` flips it to `verified` (moves_money=0)
// with impl evidence. The route COMMITS NOTHING: it classifies each money node through the
// ONE §10 gate (optionally TIGHTENED by the oversight-posture floor) and returns a RunPlan —
// no commit_money_node, no money/cost/finance write, no second gate. moves_money=0 by birth.
// Idempotent (ON CONFLICT DO UPDATE). Re-run after a full census rebuild (apply-clusters wipes
// canonical_capability), then `pnpm docs:gen` to project into docs/033.
// Run: node tools/capabilities/reconcile-workflow-plan-cap.mjs
import Database from 'better-sqlite3'
import { CAPABILITIES_DB } from '../_paths.mjs'

const db = new Database(CAPABILITIES_DB)

const CAPS = [
  {
    key: 'workflow-runtime.plan_workflow',
    side_effect_class: 'internal_write',
    canonical_name: 'preview workflow gate routing (read-only plan)',
    acceptance_criteria:
      'POST /api/companies/:id/workflows/plan PREVIEWS how a posted WorkflowGraph + its money nodes would be gated and COMMITS NOTHING. It threads the graph through the PURE chronica_workflows::run_workflow planner, which walks the DAG and classifies each money node through the ONE §10 gate (chronica_policy::evaluate_gate), optionally TIGHTENED by the oversight-posture floor (clamp_verdict_tighter, flag-gated OFF by default ⇒ byte-identical), records a NodeOutcome, pauses on a gated/hard-stopped node, and returns a RunPlan. NEVER dispatches a node, NEVER reaches commit_money_node / execute_run_money_db / commit_money_action, adds NO second gate, writes ZERO money_commitments/cost_events/finance_events rows. Thresholds are SERVER-side (never client). Deny-by-default tenancy (anonymous→403, cross-account→403) and per-money-node cross-company rejection (422) run BEFORE the planner. The posture floor can only TIGHTEN, never widen AutoAllow, and is keyed on the VERIFIED principal actor_id. moves_money=0.',
    required_tests:
      'plan_money_node_returns_awaiting_approval_never_commits,plan_hard_stop_budget_returns_failed_no_effect,plan_is_read_only_no_persistence,plan_posture_floor_off_is_byte_identical,plan_posture_floor_on_only_tightens,plan_cross_company_money_node_rejected_422,plan_route_anonymous_403,plan_route_cross_account_403,plan_workflow_handler_references_no_commit_path',
    acceptance_test: 'plan_money_node_returns_awaiting_approval_never_commits',
  },
]

const upsert = db.prepare(`
  INSERT INTO canonical_capability (
    key, canonical_name, domain, target_crate, target_module, side_effect_class,
    moves_money, requires_approval, acceptance_criteria, required_tests,
    financial_control_test, acceptance_test, status, exclusion_note, blocker, slice, donor_count
  ) VALUES (
    @key, @canonical_name, @domain, @target_crate, @target_module, @side_effect_class,
    0, 0, @acceptance_criteria, @required_tests,
    NULL, @acceptance_test, 'unimplemented', NULL, NULL, NULL, 0
  )
  ON CONFLICT(key) DO UPDATE SET
    canonical_name=excluded.canonical_name,
    domain=excluded.domain,
    target_crate=excluded.target_crate,
    target_module=excluded.target_module,
    side_effect_class=excluded.side_effect_class,
    acceptance_criteria=excluded.acceptance_criteria,
    required_tests=excluded.required_tests,
    acceptance_test=excluded.acceptance_test
`)

const tx = db.transaction(() => {
  for (const c of CAPS)
    upsert.run({ domain: 'workflow-runtime', target_crate: 'chronica-api', target_module: 'routes::workflows', ...c })
})
tx()

const total = db.prepare('SELECT count(*) n FROM canonical_capability').get().n
db.prepare('INSERT OR REPLACE INTO meta (k,v) VALUES (?,?)').run('canonical_capability_count', String(total))
console.log(`seeded ${CAPS.length} design-provenance cap (workflow-runtime.plan_workflow); canonical total ${total}`)
