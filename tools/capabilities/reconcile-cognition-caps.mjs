// Seed the 7 design-provenance canonical capabilities for the Game-Aware Cognition Core
// (chronica-strategy::cognition). These are architecture/design-driven (not donor-sourced); they enter the
// registry as `unimplemented`, then `record-verified` flips them to `verified` with impl evidence.
// Idempotent (ON CONFLICT DO UPDATE). Run: node tools/capabilities/seed-cognition-caps.mjs
import Database from 'better-sqlite3'
import { CAPABILITIES_DB } from '../_paths.mjs'

const db = new Database(CAPABILITIES_DB)

const CAPS = [
  {
    key: 'cognition.assess_commoditization',
    side_effect_class: 'pure',
    canonical_name: 'assess commoditization',
    acceptance_criteria:
      'Scores 6 signals into Baseline | DurableEdge | Decaying: high saturation/copyability is Baseline; margin-compression/crowding/short-runway is Decaying; else DurableEdge. Deterministic.',
    required_tests:
      'high_saturation_or_copyability_is_baseline,decaying_on_margin_compression_or_crowded_trade_or_short_runway,durable_when_low_saturation_long_runway',
    acceptance_test: 'high_saturation_or_copyability_is_baseline',
  },
  {
    key: 'cognition.classify_game',
    side_effect_class: 'pure',
    canonical_name: 'classify game',
    acceptance_criteria:
      'Maps opportunity features to one of 9 games (zero_sum/auction/prisoners_dilemma/signaling/screening/bayesian/repeated/network/real_option) by deterministic first-match precedence; each game exposes a core question; defaults to bayesian.',
    required_tests: 'classifier_precedence_is_deterministic,defaults_to_bayesian,each_game_class_exposes_a_core_question',
    acceptance_test: 'classifier_precedence_is_deterministic',
  },
  {
    key: 'cognition.build_moat_profile',
    side_effect_class: 'pure',
    canonical_name: 'build moat profile',
    acceptance_criteria:
      'Computes a weighted durable-advantage composite from 7 dimensions. Enforces Law 1 (commoditized public signal contributes 0; private data survives only with owned evidence), Law 3 (trust needs verifiable evidence), Law 7 (network/trust/control weighted heaviest).',
    required_tests:
      'law1_public_intelligence_zero_but_private_data_survives,law3_only_verifiable_evidence_raises_trust,law7_network_trust_control_dominate_composite,moat_weights_sum_to_bps_full',
    acceptance_test: 'law7_network_trust_control_dominate_composite',
  },
  {
    key: 'cognition.estimate_edge_decay',
    side_effect_class: 'pure',
    canonical_name: 'estimate edge decay',
    acceptance_criteria:
      'Estimates periods-to-decay + the triggers that shorten it (saturation/copyability/margin/crowding/auction), saturating at 0; decays_faster_than compares execution speed (Law 5 helper). Deterministic.',
    required_tests: 'triggers_shorten_runway,decays_faster_than_compares_execution_speed',
    acceptance_test: 'triggers_shorten_runway',
  },
  {
    key: 'cognition.plan_real_option',
    side_effect_class: 'pure',
    canonical_name: 'plan real option',
    acceptance_criteria:
      'A test-small option: EV via chronica-simulation ScenarioBranch (no forked math), with kill/scale thresholds and a cheap-test fraction check. loop_decision yields Kill | KeepTesting | Scale.',
    required_tests: 'ev_delegates_to_simulation_engine,loop_decision_kill_keep_scale_ladder,cheap_test_fraction_check',
    acceptance_test: 'ev_delegates_to_simulation_engine',
  },
  {
    key: 'cognition.assess_opportunity',
    side_effect_class: 'pure',
    canonical_name: 'assess opportunity',
    acceptance_criteria:
      'Composes commoditization + game + moat + edge-decay + real-option + reused EnterpriseValueVector into one advisory verdict (Go/NoGo/TestSmall/Hedge/Scale/Kill). Enforces Laws 2/4/5/6 (business path) as hard gates before the positive path. NEVER authorizes money (authorizes_money()==false).',
    required_tests:
      'cognition_verdict_never_authorizes_money,law2_saturation_caps_public_edge_not_durable_moat,law4_trend_only_no_moat_no_ev_cannot_go,law5_go_requires_execution_faster_than_decay,law6_negative_unit_economics_forces_no_go,law_gates_beat_high_scores,reuses_enterprise_value_vector,strong_durable_opportunity_goes',
    acceptance_test: 'cognition_verdict_never_authorizes_money',
  },
  {
    key: 'cognition.score_trade_quality',
    side_effect_class: 'pure',
    canonical_name: 'score trade quality',
    acceptance_criteria:
      'Trade-Quality scorer: prediction != position (missing entry/exit/stop/sizing/invalidation blocks the trade); Law 6 trading path (risk/reward, drawdown, liquidity, regime, negative-EV) — NOT ROIC/WACC; crowded public signal is hedged not sized up. Advisory: places_order()==false.',
    required_tests:
      'trading_missing_discipline_blocks_even_strong_prediction,law6_trading_bad_risk_reward_forces_no_go,trading_crowded_signal_caps_size,trading_quality_penalizes_crowdedness_and_slippage,trading_verdict_never_places_order',
    acceptance_test: 'trading_verdict_never_places_order',
  },
  {
    key: 'cognition.compose_opportunity_brief',
    side_effect_class: 'pure',
    canonical_name: 'compose opportunity brief',
    acceptance_criteria:
      'Composes a CognitionVerdict + capital shape into a NON-EXECUTING OpportunityBrief: decision + next_action (RouteToGovernance via route_ma_review | RunSmallTest | Hedge | Stop) + trust_tier + evidence_required + moral_hazard + headline. authorizes_money()==false and is_executing()==false; a Go/Scale ROUTES the money decision to the competent governance layer (the gate/board still commits).',
    required_tests:
      'go_verdict_routes_to_governance_and_never_executes,brief_never_authorizes_money_for_any_decision',
    acceptance_test: 'brief_never_authorizes_money_for_any_decision',
  },
  {
    key: 'strategy.score_player_calibration',
    side_effect_class: 'pure',
    domain: 'strategy',
    target_module: 'calibration',
    canonical_name: 'score player calibration',
    acceptance_criteria:
      'Scores an actor track record into a calibration_confidence_bps that weights the EnterpriseValueVector. Cold-start = lowest trust via a sample-driven ceiling (autonomy is earned); hidden-failure rate penalized more than late-correction rate; bounded-multiplicative positives. Deterministic; bridges into valuation::ConfidenceWeights. No money.',
    required_tests:
      'cold_start_is_lowest_trust,earned_autonomy_rises_with_track_record,hidden_failure_penalized_more_than_late_correction,bridges_into_confidence_weights,calibration_is_deterministic',
    acceptance_test: 'cold_start_is_lowest_trust',
  },
  {
    key: 'strategy.analyze_actor_incentives',
    side_effect_class: 'pure',
    domain: 'strategy',
    target_module: 'game_state',
    canonical_name: 'analyze actor incentives',
    acceptance_criteria:
      'The inner-game model: a reporting agent (not principal-aligned) holding a motive served by distorting a GAMEABLE signal is flagged as a moral hazard whose claim requires verifiable evidence (anti-self-report mechanism). External/observable signals are not gameable; principal-aligned roles (auditor/investor/creditor) are not hazards. Deterministic, no money.',
    required_tests:
      'moral_hazard_flagged_for_self_serving_gameable_signal,principal_aligned_role_has_no_moral_hazard,external_signal_is_not_gameable,no_tempted_motive_means_no_benefit,analysis_is_deterministic',
    acceptance_test: 'moral_hazard_flagged_for_self_serving_gameable_signal',
  },
  {
    key: 'strategy.derive_track_record_from_evidence',
    side_effect_class: 'pure',
    domain: 'strategy',
    target_module: 'calibration',
    canonical_name: 'derive track record from evidence',
    acceptance_criteria:
      'Builds a PlayerTrackRecord from observed internal evidence: EvidenceLedger::from_observations aggregates PlayerObservations (reusing chronica-simulation Calibration for prediction-vs-actual + screening EvidenceRef for audit-backed evidence) → bps metrics. Calibration is EARNED from real behavior: accurate audited forecasts → high; repeated overconfident failures → low; self-report-only → ~zero; evidence-backed → improves; deterministic. No money, no live/external ingestion (per-source readers live in the orchestrator).',
    required_tests:
      'accurate_audited_forecasts_yield_high_calibration,repeated_overconfident_failures_yield_low_calibration,self_report_only_claims_do_not_improve_calibration,evidence_backed_outcomes_improve_calibration,calibration_is_deterministic_from_audit_records',
    acceptance_test: 'accurate_audited_forecasts_yield_high_calibration',
  },
  {
    key: 'cognition.derive_calibration_from_engine_history',
    side_effect_class: 'read_only',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'player_history',
    canonical_name: 'derive calibration from engine history',
    acceptance_criteria:
      'Read-only per-source adapters project existing internal records (AuditEntry / ToolCallLog / WorkflowRunResult / ExecutionResult-S39 / WorkflowRunRecord) into chronica-strategy PlayerObservations; derive_player_calibration_from_engine_history builds an EvidenceLedger → PlayerTrackRecord → PlayerCalibration → TrustTier for an actor within a scope. Actor-mapped (ambiguous=Unknown, excluded); scope-isolated (siblings excluded); no self-report inflation (only audit/tool/outcome-backed records count); secret-safe (no raw I/O copied); deterministic. No money, no execution, no live ingestion, no workflow mutation.',
    required_tests:
      'audit_backed_success_improves_calibration,failed_workflow_lowers_calibration,self_report_only_does_not_improve_calibration,sibling_workspace_data_is_not_included,secret_like_content_is_redacted,unknown_actor_does_not_gain_trust,same_input_produces_same_calibration',
    acceptance_test: 'audit_backed_success_improves_calibration',
  },
  {
    key: 'cognition.compose_brief_from_engine_history',
    side_effect_class: 'read_only',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'cognition_bridge',
    canonical_name: 'compose brief from engine history',
    acceptance_criteria:
      'Read-only bridge: derive_player_calibration_from_engine_history projects an actor\'s real internal engine history (audit / tool / run / S39-learning / workflow records) into a chronica-strategy PlayerTrackRecord, injects it as the opportunity\'s actor calibration (overriding any supplied player), then runs assess_opportunity → CognitionVerdict → OpportunityBrief — so the actionable, NON-EXECUTING brief inherits EARNED trust (trust_tier) from observed behavior. A proven actor (audit-backed in-scope successes) routes a Go/Scale to governance; a cold-start / sibling-scope actor (no in-scope evidence) is discounted to validate-first. Scope-isolated (sibling evidence excluded), deterministic. authorizes_money()==false AND is_executing()==false. No money, no execution, no live ingestion, no workflow mutation, no new dependency.',
    required_tests:
      'engine_derived_trust_flows_into_the_brief,proven_history_routes_to_governance_cold_start_validates_first,bridge_never_authorizes_money_or_executes,sibling_scope_history_does_not_earn_trust,same_input_produces_same_brief',
    acceptance_test: 'bridge_never_authorizes_money_or_executes',
  },
  {
    key: 'cognition.rank_actor_calibrations',
    side_effect_class: 'read_only',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'player_history',
    canonical_name: 'rank actor calibrations',
    acceptance_criteria:
      'Read-only fleet ranking: rank_actor_calibrations projects one EngineHistory into a deterministic ranking of every in-scope, KNOWN actor\'s EARNED PlayerCalibration — reusing derive_player_calibration_from_engine_history per discovered actor (no forked math) — ordered by calibration_confidence_bps descending with a stable actor-key tiebreak. Unknown-actor observations (e.g. runtime traces, which carry no actor) and out-of-scope (sibling) observations are excluded; a proven (audit-backed) actor out-ranks a cold-start one; secret-safe (no raw tool I/O copied into the ranking); deterministic. Pure MEASUREMENT — grants no money/execution authority. No money, no execution, no live ingestion, no workflow mutation, no new dependency.',
    required_tests:
      'ranks_proven_actor_above_cold_start,unknown_actor_is_never_ranked,sibling_scope_actor_is_excluded,fleet_ranking_is_deterministic,fleet_ranking_redacts_secrets',
    acceptance_test: 'ranks_proven_actor_above_cold_start',
  },
  {
    key: 'cognition.compose_portfolio_from_engine_history',
    side_effect_class: 'read_only',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'cognition_bridge',
    canonical_name: 'compose portfolio from engine history',
    acceptance_criteria:
      'Read-only portfolio aggregator: compose_portfolio_from_engine_history assesses a slate of PortfolioCandidates (each = a claimant actor + opportunity features + capital shape) via compose_brief_from_engine_history — so every opportunity inherits its claimant\'s EARNED, engine-derived calibration — then ranks the resulting NON-EXECUTING briefs by pursue-worthiness (decision desirability Scale>Go>TestSmall>Hedge>NoGo>Kill, then EV composite desc, with a stable label tiebreak). A proven claimant\'s opportunity out-ranks the same opportunity claimed by a cold-start actor; a NoGo (e.g. negative unit economics) sinks below a Go regardless of calibration. Deterministic. OpportunityPortfolio.authorizes_money()==false AND is_executing()==false (no entry routes money or executes; a top Go/Scale still routes to governance). No money, no execution, no live ingestion, no workflow mutation, no new dependency.',
    required_tests:
      'portfolio_ranks_pursue_worthy_first,portfolio_inherits_earned_trust_per_candidate,portfolio_never_authorizes_money_or_executes,portfolio_is_deterministic_with_stable_tiebreak,empty_portfolio_is_advisory_and_empty',
    acceptance_test: 'portfolio_never_authorizes_money_or_executes',
  },
  {
    key: 'cognition.measure_calibration_drift',
    side_effect_class: 'read_only',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'player_history',
    canonical_name: 'measure calibration drift',
    acceptance_criteria:
      'Read-only governance signal: measure_calibration_drift compares an actor\'s EARNED PlayerCalibration across two engine-history snapshots (earlier `before` vs later `after`) within a scope — reusing derive_player_calibration_from_engine_history for BOTH (no forked math, identical scope-isolation + no-self-report rules) — and classifies the change as Improving / Stable / Regressing relative to a ±stable_band_bps band, reporting delta_bps and whether the trust_tier changed. A Regressing trend (earned trust decaying, e.g. a wave of failures lowering the realized success rate) is the governance signal. Sibling-scope evidence is excluded (no false improvement); deterministic. Pure measurement — no money/execution authority. No money, no execution, no live ingestion, no workflow mutation, no new dependency.',
    required_tests:
      'improving_calibration_is_detected,regressing_calibration_is_detected,stable_calibration_within_band,drift_respects_scope_isolation,drift_is_deterministic',
    acceptance_test: 'regressing_calibration_is_detected',
  },
  {
    key: 'strategy.recommend_oversight_posture',
    side_effect_class: 'pure',
    domain: 'strategy',
    target_module: 'calibration',
    canonical_name: 'recommend oversight posture',
    acceptance_criteria:
      'Advisory governance primitive: recommend_oversight translates an actor\'s EARNED TrustTier (+ OversightContext risk flags) into a recommended OversightPosture (PreApproveEach | ReviewBeforeCommit | PostAudit | SpotAudit). Earned autonomy LOOSENS oversight (ColdStart→PreApproveEach … Autonomous→SpotAudit); a moral hazard or high-risk action TIGHTENS it — the recommendation is the more restrictive of the tier-base and the risk-floor, so a high-risk or conflicted actor can never be looser than ReviewBeforeCommit regardless of tier, and both together force PreApproveEach; risk never loosens a tighter base. Deterministic. authorizes_money()==false — recommending oversight never authorizes money or executes. No money, no execution, no live ingestion, no new dependency.',
    required_tests:
      'cold_start_requires_pre_approval,autonomous_earns_lightest_oversight,risk_or_hazard_tightens_oversight,recommendation_never_authorizes_money,oversight_is_deterministic',
    acceptance_test: 'recommendation_never_authorizes_money',
  },
  {
    key: 'cognition.annotate_run_cognition_context',
    side_effect_class: 'read_only',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'run_cognition',
    canonical_name: 'annotate run with cognition context',
    acceptance_criteria:
      'Read-only run-loop annotation (observability only): execute_workflow_with_cognition / run_with_controls_with_cognition run the existing gated runners UNCHANGED, then attach a RunCognitionContext computed from the actor\'s internal engine history — actor_id, trust_tier, calibration_score_bps, oversight_posture, evidence_required, moral_hazard, rationale, source_evidence_ids — by composing derive_player_calibration_from_engine_history + IncentiveContext analysis + recommend_oversight (no forked math). NO execution change, NO money block/allow, NO auto-approval, NO money movement, NO external API, NO second gate. Fails safe: no/unknown/unmatched in-scope evidence → ColdStart + PreApproveEach; self-report-only does not raise trust; sibling-scope evidence excluded; secret-safe (ids/hashes only); deterministic. No new dependency.',
    required_tests:
      'workflow_run_gets_cognition_context,known_high_trust_actor_gets_higher_trust_tier,cold_start_actor_gets_conservative_oversight,moral_hazard_flag_appears_with_conflicted_incentive,self_report_only_history_does_not_raise_trust,sibling_workspace_evidence_is_excluded,secret_like_evidence_is_redacted,no_money_or_cost_records_created,no_workflow_behavior_changes,missing_cognition_data_fails_safe_not_open,controlled_run_carries_cognition_context',
    acceptance_test: 'no_workflow_behavior_changes',
  },
  {
    key: 'cognition.enforce_cognition_aware_money_gate',
    side_effect_class: 'policy_check',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'cognition_gate',
    canonical_name: 'enforce cognition-aware money gate',
    acceptance_criteria:
      'Cognition-aware money-gate enforcement: run_with_cognition_gate connects a run\'s RunCognitionContext to the Slice-4 money gate so earned trust can TIGHTEN gating — but it NEVER approves, spends, trades, transfers, or authorizes money. screen_money_step (reusing chronica-strategy OversightPosture; no forked EV math) decides, per money-impacting step: PreApproveEach / ReviewBeforeCommit / evidence_required-unattested / moral_hazard → Escalate (route to approval BEFORE the gate; the handler never runs → no execution, no money); PostAudit/SpotAudit → defer to the gate UNCHANGED with an audit obligation attached. A non-money step runs through the gate behavior-identically. THE INVARIANT: cognition only ADDS requirements — it never bypasses or loosens the money gate (a gate denial/approval-required always stands; PostAudit/SpotAudit allow execution ONLY where the gate already permits), and the money gate inside invoke_tool remains the only capital boundary. Fails safe: missing/unknown/cold-start context, sibling-scope/self-report-only/redacted evidence → Escalate. Deterministic. moves_money=0 — cognition authorizes no money. No new crate, no new dependency, no second gate.',
    required_tests:
      'screen_non_money_step_is_no_change,screen_money_without_context_fails_safe_to_escalate,screen_pre_approve_each_and_review_escalate,screen_evidence_required_escalates_unless_attested,screen_moral_hazard_escalates,screen_post_and_spot_audit_defer_with_audit_obligation,pre_approve_each_routes_money_step_to_approval,evidence_required_routes_money_step_to_approval,post_audit_does_not_bypass_gate_approval,post_audit_allows_only_where_gate_permits,missing_context_fails_safe_even_when_board_approved,cognition_never_loosens_the_money_gate,non_money_steps_are_behavior_identical,cold_start_money_step_escalates_end_to_end',
    acceptance_test: 'cognition_never_loosens_the_money_gate',
  },
  {
    key: 'cognition.run_money_workflow_default',
    side_effect_class: 'policy_check',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'production',
    canonical_name: 'default cognition-gated money-run path',
    acceptance_criteria:
      'run_money_workflow is THE default production entrypoint for any money/cost/capital-impacting workflow run: it routes through the cognition-aware tighten-only gate (run_with_cognition_gate) so earned trust escalates / requires evidence / attaches audit — but NEVER authorizes money (the Slice-4 gate stays the only capital boundary; cognition only ADDS requirements). evidence_attested defaults to false (conservative). A missing cognition context fails safe to pre-approval escalation; a non-money run is behavior-identical to the raw runner (only audit metadata may differ). The raw execute_workflow / run_with_controls remain LOW-LEVEL primitives (documented as such). moves_money=0. No new crate, no forked EV, no second gate, no Buy/Sell/Spend/AuthorizeMoney verdict.',
    required_tests:
      'default_money_run_invokes_cognition_gating,default_non_money_run_is_behavior_identical,default_money_run_missing_context_escalates',
    acceptance_test: 'default_money_run_invokes_cognition_gating',
  },
  {
    key: 'cognition.auto_source_run_history',
    side_effect_class: 'read_only',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'production',
    canonical_name: 'auto-source scoped engine history for a run',
    acceptance_criteria:
      'run_money_workflow_auto is the production money-run path with AUTO-SOURCED history: instead of caller-supplied history it retrieves the actor\'s SCOPED OwnedEngineHistory from an approved HistoryProvider, derives the RunCognitionContext from it, then routes through the cognition gate. Auto-sourcing is read-only — it creates NO money/cost record and authorizes NO money. Enforced by the reused derivation: scope isolation (sibling-scoped evidence excluded even if the source over-returns), redaction (secret-like input never reaches the context), self-report-only history cannot raise trust, missing/unavailable history fails safe to ColdStart + PreApproveEach (→ escalation), and source_evidence_ids carry provenance ids/hashes only when in-scope and evidence-backed. Caller-supplied history remains available for tests. moves_money=0. No new crate, no forked EV, no second gate, no live external ingestion (the provider is the approved internal source).',
    required_tests:
      'auto_sourced_proven_actor_drives_context,auto_sourced_missing_history_fails_safe,auto_sourced_scope_isolation_excludes_sibling,auto_sourced_self_report_only_does_not_raise_trust,auto_sourced_redaction_keeps_secrets_out_of_context,auto_sourcing_creates_no_money_record_for_read_only_run',
    acceptance_test: 'auto_sourced_missing_history_fails_safe',
  },
  {
    key: 'cognition.scoped_history_provider',
    side_effect_class: 'internal_write',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'history_store',
    canonical_name: 'real scoped history provider',
    acceptance_criteria:
      'ScopedHistoryStore is the REAL production HistoryProvider: a read-only, scope-tagged index of the kernel\'s approved internal record types (AuditEntry / ToolCallLog / ExecutionResult / WorkflowRunRecord). record_* append already-happened internal events (read-model writes — no money, no cost record); history_for(actor, scope) returns ONLY records whose recorded scope is contained in the requested scope — so sibling-workspace evidence is excluded AT THE SOURCE (necessary because the audit/tool-call/workflow-outcome observation adapters otherwise adopt the requested scope). Per-actor attribution + no-self-report-inflation + redaction remain the reused derivation\'s job, so self-report-only history cannot raise trust, secret-like content never reaches the context (and never relaxes gating), and a missing/empty store fails safe to ColdStart + PreApproveEach. Read-only; no money authorization; no new crate; no forked calibration.',
    required_tests:
      'real_provider_returns_only_in_scope_evidence,sibling_evidence_excluded_even_when_in_store,secret_like_evidence_redacted_and_does_not_relax_gating,self_report_only_does_not_raise_trust,missing_history_fails_safe_and_does_not_execute,provider_recording_and_sourcing_create_no_money_record',
    acceptance_test: 'sibling_evidence_excluded_even_when_in_store',
  },
  {
    key: 'cognition.restrict_raw_runners_from_production',
    side_effect_class: 'policy_check',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'production',
    canonical_name: 'restrict raw runners from the production money path',
    acceptance_criteria:
      'The production/default money execution path is the cognition-gated path, and the raw runners cannot be used for it. Steps B+C: chronica-engine is a leaf crate (no external dependent), so the low-level raw runners execute_workflow and run_with_controls are demoted to pub(crate) and REMOVED from the public re-exports — external/production code can no longer call them; they remain only for the low-level cognition wrappers and in-crate tests, documented as unsafe for production money workflows. The public default money entrypoint is run_money_workflow / run_money_workflow_auto (which route through run_with_cognition_gate). A guard test proves the production money path ESCALATES (routes through the cognition gate) a money step that the raw runner would have executed — i.e. production money execution uses the gate, not the raw runner. No behavior change for non-money runs; no money authorized; no new crate; no forked EV.',
    required_tests: 'production_money_path_uses_cognition_gate_not_raw_runner',
    acceptance_test: 'production_money_path_uses_cognition_gate_not_raw_runner',
  },
  {
    key: 'cognition.run_controlled_with_cognition_gate',
    side_effect_class: 'policy_check',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'cognition_gate',
    canonical_name: 'controlled cognition-gated runner',
    acceptance_criteria:
      'run_controlled_with_cognition_gate is the controlled (cancellable / resumable / idempotent) runner WITH the cognition-aware money gate. It preserves every controlled-run semantic of run_with_controls — cancellation is checked BEFORE each step, each dispatched step is idempotent under the same per-step key so a resume sharing the store dedupes the committed prefix, and the run reaches exactly one terminal status — AND applies the same tighten-only rules as run_with_cognition_gate: a money step cognition escalates is routed to approval BEFORE the gate AND before the idempotent dispatch (handler never runs → no execution, no money, no commit, so a resume re-screens cleanly); otherwise it dispatches idempotently through the existing gate UNCHANGED with any audit obligation attached. Cognition never authorizes money; missing context fails safe to escalation; non-money behavior is preserved. moves_money=0. No new crate, no forked EV, no second gate.',
    required_tests:
      'controlled_gated_preserves_cancellation,controlled_gated_resume_dedupes_executed_prefix,controlled_money_step_escalates_under_cold_start,controlled_gated_missing_context_escalates_money,controlled_gated_non_money_behavior_preserved',
    acceptance_test: 'controlled_money_step_escalates_under_cold_start',
  },
  {
    key: 'cognition.run_money_workflow_controlled_auto',
    side_effect_class: 'policy_check',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'production',
    canonical_name: 'controlled auto-sourced production money path',
    acceptance_criteria:
      'run_money_workflow_controlled_auto is the production money-run path with AUTO-SOURCED history AND controlled-run semantics: it retrieves the actor\'s scoped history from a HistoryProvider, derives the RunCognitionContext, then runs through run_controlled_with_cognition_gate — preserving cancellation + idempotent resume while applying the same tighten-only money-gate rules. Read-only sourcing (no money/cost record); cognition never authorizes money; missing/unavailable history fails safe to ColdStart + PreApproveEach (controlled money step escalates, handler never runs); non-money completes normally. moves_money=0. No new crate, no forked EV, no second gate.',
    required_tests:
      'controlled_production_auto_proven_actor_completes_non_money,controlled_production_auto_missing_history_escalates_money',
    acceptance_test: 'controlled_production_auto_missing_history_escalates_money',
  },
  {
    key: 'cognition.ground_evidence_attestation',
    side_effect_class: 'pure',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'cognition_gate',
    canonical_name: 'provenance-backed evidence attestation',
    acceptance_criteria:
      'Replaces the ungrounded evidence_attested bool with a provenance-backed attestation. EvidenceAttestation carries a competent AttestationAuthority (Board / Governance / Auditor — the acting agent and cognition are NOT representable, so evidence cannot be self-certified), a non-empty evidence_ref (provenance id/hash of a real evidence record), and a scope. is_proven(scope) is true ONLY when the authority is competent AND the evidence_ref is non-empty AND the attestation scope CONTAINS the run scope; a missing / empty-ref / out-of-scope attestation is NOT proven. evidence_attested_for(attestation, scope) is the ONLY bridge from an attestation to the gate flag (cognition never sets it) and defaults to false (fail safe) when absent or unproven. A proven attestation only lets the evidence requirement DEFER to the gate; it never authorizes money, bypasses the Slice-4 gate, or relaxes the oversight posture. moves_money=0. No new crate, no forked EV.',
    required_tests: 'attestation_must_be_provenance_backed,unproven_attestation_fails_safe',
    acceptance_test: 'attestation_must_be_provenance_backed',
  },
  {
    key: 'cognition.run_money_workflow_auto_attested',
    side_effect_class: 'policy_check',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'production',
    canonical_name: 'attested auto-sourced production money path',
    acceptance_criteria:
      'run_money_workflow_auto_attested is the production money-run path that GROUNDS the gate evidence flag in a provenance-backed EvidenceAttestation (resolved via evidence_attested_for) instead of a raw bool: auto-sources scoped history, derives the context, computes evidence_attested = is_proven, then gates. None/unproven attestation fails safe (evidence_required money steps escalate; handler never runs). A proven attestation satisfies only the evidence requirement — it NEVER authorizes money, bypasses the Slice-4 gate, or relaxes the oversight posture, so a high-risk / cold-start / conflicted actor still escalates regardless of attestation. moves_money=0. No new crate, no forked EV, no second gate.',
    required_tests:
      'attested_path_unproven_attestation_fails_safe,attested_path_proven_attestation_never_authorizes_money',
    acceptance_test: 'attested_path_proven_attestation_never_authorizes_money',
  },
  {
    key: 'cognition.record_run_into_history',
    side_effect_class: 'internal_write',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'history_store',
    canonical_name: 'record a completed run into scoped history',
    acceptance_criteria:
      'ScopedHistoryStore::record_gated_run / record_controlled_run populate the store from a completed run: for each EXECUTED step they record SANITIZED, actor-attributed TRUST evidence (a SYNTHESIZED minimal record — never the run payload/secret — keyed run:{run_id}:{idx}:{tool}, attributed via the actor kind: Agent→tool_call, Workspace→audit, Capability→learning; other kinds record no trust evidence), and for each cognition-escalated / gate-denied / blocked step they append a GovernanceRecord to a separate governance_log that history_for NEVER returns — so escalated/denied steps are logged for audit but can raise NOR lower trust. The controlled variant records evidence only for steps COMMITTED THIS run (deduped steps already recorded by the prior run are skipped; cancelled steps never ran and are absent). Scope-tagged; read-only w.r.t. money (no money/cost record, no money authorization). moves_money=0. No new crate, no forked calibration.',
    required_tests:
      'record_gated_run_records_executed_evidence_and_governance,recorded_evidence_is_sanitized,record_controlled_run_records_committed_only',
    acceptance_test: 'record_gated_run_records_executed_evidence_and_governance',
  },
  {
    key: 'cognition.run_money_workflow_recording',
    side_effect_class: 'internal_write',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'production',
    canonical_name: 'self-populating production money-run path',
    acceptance_criteria:
      'run_money_workflow_recording / run_money_workflow_controlled_recording are the self-populating production money-run paths: they derive the cognition context from the ScopedHistoryStore\'s prior records, run through the (controlled) cognition gate, then RECORD this run\'s sanitized, actor/scope-tagged evidence (and governance records for escalated/denied steps) back into the SAME store — so future run_money_workflow_auto / recording calls derive trust from accumulated internal behavior WITHOUT the caller manually record_*-ing. A later auto-sourced run uses the recorded history (round-trip); sibling-scope records are excluded; secrets never surface in source_evidence_ids/context; escalated/denied money steps are governance-logged but never raise trust and create no money/cost record; controlled recording records committed steps only (cancelled/deduped skipped); missing history fails safe to ColdStart + PreApproveEach; non-money behavior is behavior-identical. moves_money=0. No new crate, no forked EV, no second gate, no live ingestion.',
    required_tests:
      'completed_run_records_then_later_run_uses_it,sibling_scope_records_excluded_across_runs,recorded_history_surfaces_no_secret,escalated_money_step_recorded_as_governance_no_trust_no_money,controlled_recording_records_committed_only_and_skips_cancelled,recording_path_missing_history_fails_safe,recording_non_money_behavior_identical_and_no_money_record',
    acceptance_test: 'completed_run_records_then_later_run_uses_it',
  },
  {
    key: 'cognition.default_production_money_path_records',
    side_effect_class: 'internal_write',
    domain: 'cognition',
    target_crate: 'chronica-engine',
    target_module: 'production',
    canonical_name: 'self-recording production money path is the default',
    acceptance_criteria:
      'The self-recording entrypoints run_money_workflow_recording / run_money_workflow_controlled_recording are THE DEFAULT production money path (derive from prior scoped history → cognition-gated execution → Slice-4 money gate → record sanitized scoped history/governance outcome → next run uses accumulated evidence), documented as such and grouped first in the public re-exports. The auto entrypoints (run_money_workflow_auto / _auto_attested / _controlled_auto) are documented + grouped as READ-ONLY, NON-RECORDING variants — explicitly NOT the default production money path (a read-only run never mutates the store, proven by record_count() before==after). The raw runners stay pub(crate) internal/test-only. Guard tests prove the default path both derives cognition from ScopedHistoryStore BEFORE execution and records sanitized scoped evidence back AFTER, and that the read-only auto path leaves the store untouched. All money-safety invariants preserved (cognition never authorizes money; Slice-4 gate is the only capital boundary; recording writes history/audit records only; missing history fails safe). moves_money=0. No new crate, no forked EV.',
    required_tests:
      'default_production_path_derives_then_records,readonly_auto_path_does_not_mutate_store',
    acceptance_test: 'default_production_path_derives_then_records',
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
  for (const c of CAPS) upsert.run({ domain: 'cognition', target_crate: 'chronica-strategy', target_module: 'cognition', ...c })
})
tx()

const total = db.prepare('SELECT count(*) n FROM canonical_capability').get().n
db.prepare('INSERT OR REPLACE INTO meta (k,v) VALUES (?,?)').run('canonical_capability_count', String(total))
console.log(`seeded ${CAPS.length} design-provenance caps (cognition.* + strategy.score_player_calibration); canonical total ${total}`)
