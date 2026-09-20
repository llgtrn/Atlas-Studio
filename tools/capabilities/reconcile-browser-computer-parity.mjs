#!/usr/bin/env node
// Reconcile the underweighted browser donor group into a governed Chronica browser-computer surface.
//
// This script is intentionally conservative: it records the verified classifier seed that exists in
// chronica-internet-hand, and records the larger browser-computer pipeline as unimplemented until the
// Rust executor/profile/evidence path exists.

import Database from 'better-sqlite3'
import { CAPABILITIES_DB } from '../_paths.mjs'

const db = new Database(CAPABILITIES_DB)

const rows = [
  {
    key: 'browser.classify_action_risk',
    canonical_name: 'Classify browser actions as read-only or gated side effects',
    domain: 'browser-internet-hand',
    target_crate: 'chronica-internet-hand',
    target_module: 'internet',
    side_effect_class: 'pure',
    moves_money: 0,
    requires_approval: 0,
    acceptance_criteria: 'InternetAction separates read-only browse/extract/screenshot from SideEffectAction; InternetTask lists gated actions.',
    required_tests: 'cargo test -p chronica-internet-hand --lib internet::tests::task_separates_free_and_gated',
    financial_control_test: null,
    acceptance_test: 'task_separates_free_and_gated',
    status: 'implemented_unverified',
    blocker: null,
  },
  {
    key: 'browser.governed_computer_pipeline',
    canonical_name: 'Governed browser computer pipeline',
    domain: 'browser-internet-hand',
    target_crate: 'chronica-internet-hand, chronica-runtime, chronica-engine, chronica-policy, chronica-approvals, chronica-observability',
    target_module: 'browser_computer',
    side_effect_class: 'external_read',
    moves_money: 0,
    requires_approval: 1,
    acceptance_criteria: 'VerifiedPrincipal -> ScopePath -> Capability -> Policy/Risk -> SecretReference -> BrowserProfile/SiteProfile -> action plan -> dry-run/preview -> armed execution -> trace/evidence/audit/learning.',
    required_tests: 'cargo test -p chronica-internet-hand --test governed_browser_computer',
    financial_control_test: null,
    acceptance_test: 'governed_browser_pipeline_routes_all_live_actions_through_policy_and_audit',
    status: 'unimplemented',
    blocker: 'Design ledger only; BrowserProfile/SiteProfile, action-plan model, runtime arming, screenshot evidence, AuditEvent, and scoped learning still need Rust implementation.',
  },
  {
    key: 'browser.profile_site_policy',
    canonical_name: 'BrowserProfile and SiteProfile policy model',
    domain: 'browser-internet-hand',
    target_crate: 'chronica-internet-hand',
    target_module: 'browser_profile',
    side_effect_class: 'internal_write',
    moves_money: 0,
    requires_approval: 1,
    acceptance_criteria: 'Profiles bind cookies, storage, proxy/secret references, site permissions, ScopePath, and allowed capabilities without leaking across companies.',
    required_tests: 'cargo test -p chronica-internet-hand --test browser_profile_scope',
    financial_control_test: null,
    acceptance_test: 'browser_profiles_do_not_cross_scopepath_boundaries',
    status: 'unimplemented',
    blocker: 'No BrowserProfile/SiteProfile Rust model exists yet.',
  },
  {
    key: 'browser.plan_dry_run_preview',
    canonical_name: 'Browser action plan dry-run and preview',
    domain: 'browser-internet-hand',
    target_crate: 'chronica-engine',
    target_module: 'execution_mode',
    side_effect_class: 'policy_check',
    moves_money: 0,
    requires_approval: 1,
    acceptance_criteria: 'BrowserActionPlan can preview target URLs, selectors, credentials, side effects, irreversibility, estimated cost, and required approval tier before live execution.',
    required_tests: 'cargo test -p chronica-engine --test browser_preview',
    financial_control_test: null,
    acceptance_test: 'browser_preview_never_executes_live_actions',
    status: 'unimplemented',
    blocker: 'ExecutionMode and preview primitives exist, but browser-specific plan/dry-run wiring is not implemented.',
  },
  {
    key: 'browser.live_execution_armed_only',
    canonical_name: 'Live browser execution only when armed',
    domain: 'browser-internet-hand',
    target_crate: 'chronica-runtime',
    target_module: 'invoke',
    side_effect_class: 'external_read',
    moves_money: 0,
    requires_approval: 1,
    acceptance_criteria: 'Runtime refuses live browser execution unless the call is scoped, policy-cleared, approved if irreversible/money-touching, and explicitly armed.',
    required_tests: 'cargo test -p chronica-runtime --test governed_browser_invoke',
    financial_control_test: null,
    acceptance_test: 'browser_live_execution_requires_armed_policy_approval',
    status: 'unimplemented',
    blocker: 'Runtime invoke gate exists for tools, but no browser executor adapter is wired through the governed arming path.',
  },
  {
    key: 'browser.evidence_trace_audit_learning',
    canonical_name: 'Browser runtime trace, evidence, audit, and scoped learning',
    domain: 'browser-internet-hand',
    target_crate: 'chronica-observability, chronica-core, chronica-memory',
    target_module: 'browser_evidence',
    side_effect_class: 'internal_write',
    moves_money: 0,
    requires_approval: 0,
    acceptance_criteria: 'Every live browser run emits RuntimeTrace, screenshot/content evidence, Merkle-linked AuditEvent, and scoped non-authorizing learning updates.',
    required_tests: 'cargo test -p chronica-observability --test browser_evidence',
    financial_control_test: null,
    acceptance_test: 'browser_live_run_records_trace_evidence_audit_and_scoped_learning',
    status: 'unimplemented',
    blocker: 'EvidenceCard seed exists in chronica-internet-hand; integrated screenshot/audit/learning consumer is not implemented.',
  },
  {
    key: 'browser.commerce_payment_click_records_finance',
    canonical_name: 'Approved browser payment click records finance and cost evidence',
    domain: 'browser-internet-hand',
    target_crate: 'chronica-internet-hand',
    target_module: 'commerce',
    side_effect_class: 'money',
    moves_money: 1,
    requires_approval: 1,
    acceptance_criteria: 'A browser checkout payment click executes only after approval and arming, then records CostRecord, finance handoff, evidence hash, and Merkle audit entries.',
    required_tests: 'cargo test -p chronica-internet-hand --test browser_commerce approved_payment_click_records_cost_finance_evidence_and_blocks_replay',
    financial_control_test: 'cargo test -p chronica-internet-hand --test browser_commerce approved_payment_click_records_cost_finance_evidence_and_blocks_replay',
    acceptance_test: 'approved_payment_click_records_cost_finance_evidence_and_blocks_replay',
    status: 'implemented_unverified',
    blocker: null,
  },
  {
    key: 'browser.commerce_requires_approval_and_arming',
    canonical_name: 'Browser commerce requires approval and explicit arming',
    domain: 'browser-internet-hand',
    target_crate: 'chronica-internet-hand',
    target_module: 'commerce',
    side_effect_class: 'policy_check',
    moves_money: 0,
    requires_approval: 1,
    acceptance_criteria: 'Unapproved payment clicks are refused, and approved clicks still refuse execution until the live browser run is explicitly armed.',
    required_tests: 'cargo test -p chronica-internet-hand --test browser_commerce unapproved_or_unarmed_checkout_cannot_click',
    financial_control_test: null,
    acceptance_test: 'unapproved_or_unarmed_checkout_cannot_click',
    status: 'implemented_unverified',
    blocker: null,
  },
  {
    key: 'browser.commerce_approval_snapshot_guard',
    canonical_name: 'Browser commerce approval snapshot guard',
    domain: 'browser-internet-hand',
    target_crate: 'chronica-internet-hand',
    target_module: 'commerce',
    side_effect_class: 'policy_check',
    moves_money: 0,
    requires_approval: 1,
    acceptance_criteria: 'The live cart/domain/payment scope must match the approved snapshot, and charged amount cannot exceed the approved ceiling.',
    required_tests: 'cargo test -p chronica-internet-hand --test browser_commerce cart_domain_or_amount_mismatch_blocks_after_approval',
    financial_control_test: null,
    acceptance_test: 'cart_domain_or_amount_mismatch_blocks_after_approval',
    status: 'implemented_unverified',
    blocker: null,
  },
  {
    key: 'browser.commerce_approval_separation_of_duties',
    canonical_name: 'Browser commerce approval separation of duties',
    domain: 'browser-internet-hand',
    target_crate: 'chronica-internet-hand',
    target_module: 'commerce',
    side_effect_class: 'policy_check',
    moves_money: 0,
    requires_approval: 1,
    acceptance_criteria: 'Self-approval and non-board approval cannot open or arm a browser payment run.',
    required_tests: 'cargo test -p chronica-internet-hand --test browser_commerce self_or_non_board_approval_cannot_arm_payment',
    financial_control_test: null,
    acceptance_test: 'self_or_non_board_approval_cannot_arm_payment',
    status: 'implemented_unverified',
    blocker: null,
  },
  {
    key: 'browser.commerce_evidence_redacts_secrets',
    canonical_name: 'Browser commerce evidence rejects secret material',
    domain: 'browser-internet-hand',
    target_crate: 'chronica-internet-hand',
    target_module: 'commerce',
    side_effect_class: 'internal_write',
    moves_money: 0,
    requires_approval: 0,
    acceptance_criteria: 'Payment evidence stores hashes and redacted summaries only; obvious card/secret material is rejected before finance records are written.',
    required_tests: 'cargo test -p chronica-internet-hand --test browser_commerce evidence_rejects_secret_material_before_finance_record',
    financial_control_test: null,
    acceptance_test: 'evidence_rejects_secret_material_before_finance_record',
    status: 'implemented_unverified',
    blocker: null,
  },
  {
    key: 'erp.browser_purchase_handoff_record',
    canonical_name: 'Browser purchase finance handoff record',
    domain: 'erp-finance',
    target_crate: 'chronica-internet-hand',
    target_module: 'commerce',
    side_effect_class: 'money',
    moves_money: 1,
    requires_approval: 1,
    acceptance_criteria: 'An approved browser payment produces a scoped finance handoff record carrying the CostRecord, receipt identifiers, evidence hash, and ScopePath.',
    required_tests: 'cargo test -p chronica-internet-hand --test browser_commerce approved_payment_click_records_cost_finance_evidence_and_blocks_replay',
    financial_control_test: 'cargo test -p chronica-internet-hand --test browser_commerce approved_payment_click_records_cost_finance_evidence_and_blocks_replay',
    acceptance_test: 'approved_payment_click_records_cost_finance_evidence_and_blocks_replay',
    status: 'implemented_unverified',
    blocker: null,
  },
]

const update = db.prepare(`
  INSERT INTO canonical_capability (
    key, canonical_name, domain, target_crate, target_module, side_effect_class,
    moves_money, requires_approval, acceptance_criteria, required_tests,
    financial_control_test, acceptance_test, status, exclusion_note, blocker, slice, donor_count
  ) VALUES (
    @key, @canonical_name, @domain, @target_crate, @target_module, @side_effect_class,
    @moves_money, @requires_approval, @acceptance_criteria, @required_tests,
    @financial_control_test, @acceptance_test, @status, NULL, @blocker, NULL, NULL
  )
  ON CONFLICT(key) DO UPDATE SET
    canonical_name=excluded.canonical_name,
    domain=excluded.domain,
    target_crate=excluded.target_crate,
    target_module=excluded.target_module,
    side_effect_class=excluded.side_effect_class,
    moves_money=excluded.moves_money,
    requires_approval=excluded.requires_approval,
    acceptance_criteria=excluded.acceptance_criteria,
    required_tests=excluded.required_tests,
    financial_control_test=excluded.financial_control_test,
    acceptance_test=excluded.acceptance_test,
    status=CASE
      WHEN canonical_capability.status='verified' THEN canonical_capability.status
      ELSE excluded.status
    END,
    blocker=excluded.blocker
`)

const tx = db.transaction(() => {
  for (const row of rows) update.run(row)
})

tx()
db.close()

console.log(`reconciled ${rows.length} governed browser-computer capabilities`)
