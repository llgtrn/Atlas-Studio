#!/usr/bin/env node
// Reconcile the Company/Group runtime roadmap slices with the canonical capability DB.
//
// This keeps Claude's DB-facing execution surface aligned with the Rust code:
// ProjectTemplate -> Project, CompanyTemplate -> Company, GroupTemplate -> Group,
// plus the allocation/governance/ERP/events/template anchors added in slices 143-195.

import Database from 'better-sqlite3'
import { CAPABILITIES_DB } from '../_paths.mjs'

const db = new Database(CAPABILITIES_DB)
const now = new Date().toISOString()

const slug = (s) =>
  s
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .slice(0, 80)

const canonicalStatus = (sliceStatus) => {
  if (sliceStatus === 'verified') return 'verified'
  if (sliceStatus === 'green' || sliceStatus === 'implemented_unverified') return 'implemented_unverified'
  return 'unimplemented'
}

const domainFor = (slice, title, crate = '') => {
  const t = `${title} ${crate}`.toLowerCase()
  if (t.includes('erp') || t.includes('finance') || t.includes('allocation') || t.includes('capital')) return 'erp-finance'
  if (t.includes('approval') || t.includes('governance') || t.includes('council') || t.includes('audit') || t.includes('board')) return 'policy-approval-audit'
  if (t.includes('memory') || t.includes('rag') || t.includes('learning')) return 'agent-knowledge'
  if (t.includes('event') || t.includes('analytics') || t.includes('olap') || t.includes('kpi') || t.includes('observability')) return 'observability-analytics'
  if (t.includes('commerce') || t.includes('marketplace') || t.includes('seller') || t.includes('listing')) return 'commerce'
  if (t.includes('template') || t.includes('workflow') || t.includes('project') || t.includes('company') || t.includes('group')) return 'workflow-runtime'
  if (slice >= 156 && slice <= 182) return 'agent-knowledge'
  return 'workflow-runtime'
}

const sliceUpdates = new Map([
  [121, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-company --lib model::tests::company_and_model_roundtrip_json',
    next_action: 'verified: Company/CompanyModel/CompanyTemplate runtime core is backed by chronica-company tests',
  }],
  [122, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-company --lib workspace::tests::one_company_template_creates_many_companies',
    next_action: 'verified: CompanyTemplate creates Company via chronica-company runtime constructor',
  }],
  [123, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-template-store --lib runtime_instantiation_creates_project_company_and_group_objects',
    next_action: 'verified: TemplateStore runtime bridge creates Project from ProjectTemplate and preserves attachment semantics',
  }],
  [124, {
    status: 'green',
    percent: 70,
    acceptance_test: 'cargo test -p chronica-company --lib business_map::tests allocation::tests',
    next_action: 'partial: business map and allocation primitives exist; finish DB/UI portfolio runtime before marking verified',
  }],
  [125, {
    status: 'green',
    percent: 70,
    acceptance_test: 'cargo test -p chronica-company --lib inter_company::tests',
    next_action: 'partial: CompanyContract/inter-company primitives exist; SharedWorkflow runtime still needs final verification',
  }],
  [126, {
    status: 'green',
    percent: 70,
    acceptance_test: 'cargo test -p chronica-memory --lib memory_scope::tests',
    next_action: 'partial: contract-gated shared memory exists; shared artifact/dataset persistence still needs final verification',
  }],
  [127, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-company --lib inter_company::tests::invoice_produces_balanced_offsetting_legs',
    fin_test: 'chronica_company::inter_company::financial_control::inter_company_transfer_full_chain',
    next_action: 'verified: CompanyContract/inter-company audit and financial-control chain are implemented',
  }],
  [128, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-template-store --lib runtime_instantiation_creates_project_company_and_group_objects',
    next_action: 'verified: GroupTemplate creates/configures Group through TemplateStore runtime bridge',
  }],
  [129, {
    status: 'green',
    percent: 70,
    acceptance_test: 'cargo test -p chronica-company --lib group::tests::group_template_creates_a_group_with_planned_members',
    next_action: 'partial: Group carries consolidated budget ceiling; hard-stop policy cascade still needs final verification',
  }],
  [130, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-company --lib governance::tests',
    target_crate: 'chronica-company, chronica-approvals, chronica-policy',
    side_effect_class: 'policy_check',
    next_action: 'verified: hierarchical routing and board-only approval semantics are backed by governance and approval tests; this routes/checks authority but does not itself move money',
  }],
  [131, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-memory --lib memory_scope::tests::group_layer_reads_descendant_workspace_memory',
    next_action: 'verified: hierarchical memory visibility blocks peers and allows group descendant reads',
  }],
  [132, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-simulation --lib group_portfolio_variants_rank_by_consolidated_ev',
    target_module: 'lib',
    side_effect_class: 'pure',
    next_action: 'verified: Group portfolio simulation variants rank by consolidated evidence-weighted EV',
  }],
  [133, {
    status: 'green',
    percent: 70,
    acceptance_test: 'cargo test -p chronica-core --lib scope::tests',
    next_action: 'partial: ScopePath enforces hierarchy; API persistence coverage still needs final verification',
  }],
  [134, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-company --lib workspace::tests::project_template_sync_updates_live_bindings_across_companies',
    next_action: 'verified: ProjectTemplate version synchronization updates live bindings across Companies while preserving detached/history pins',
  }],
  [135, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-company --lib allocation::tests::cost_attribution_records_usage_without_an_invoice',
    fin_test: 'chronica_company::inter_company::financial_control::inter_company_transfer_full_chain',
    next_action: 'verified: default allocation attribution and optional legal inter-company billing are both covered',
  }],
  [136, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-company --lib shared_agent_pool_leases_group_agents_to_allowed_companies_only',
    next_action: 'verified: Group shared agent pool leases only to allowed member Companies',
  }],
  [137, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-company --lib group_replication_emit_requires_approved_council_decision_and_targets_each_company',
    next_action: 'verified: Group replication emits per-Company targets only after approved Council decision',
  }],
  [138, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-memory --lib rag::tests::scoped_rag_retrieve_filters_before_ranking',
    next_action: 'verified: multi-scope RAG applies memory visibility before hybrid retrieval/ranking',
  }],
  [139, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-memory --lib learning::tests::group_learning_rollup_creates_shared_summary_without_exposing_private_records',
    next_action: 'verified: group learning rollup creates shareable summary memory without exposing private Company records',
  }],
  [140, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-workflows --lib group_scope_workflow_change_routes_to_council_and_targets_member_companies',
    next_action: 'verified: Group-scope self-evolving workflow proposals target member Companies and route to Council',
  }],
  [141, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-evidence-debate --lib group_simulation_debate_selects_best_variant_and_requires_council_view',
    next_action: 'verified: Group-scale simulation debate selects a portfolio variant with Council dissent',
  }],
  [142, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-company --lib governance::tests::new_subsidiary_routes_to_council',
    next_action: 'verified: CompanyCeo/GroupCouncil/GroupPresident role routing is implemented in governance layer',
  }],
  [143, {
    status: 'green',
    percent: 70,
    acceptance_test: 'cargo test -p chronica-core --lib scope::tests',
    target_module: 'scope',
    side_effect_class: 'pure',
    blocker: 'ScopePath/AccountId core is tested, but full S143 also requires Account { id, root_group_id }, Group.account_id, and a company-layer one-account-one-group bootstrap test.',
    next_action: 'partial: ScopePath is the shared AccountId/Group/Company/Project hierarchy token and tests prove prefix containment + cross-account/cross-group isolation; finish Account/root_group bootstrap before marking whole slice verified',
  }],
  [147, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-simulation --lib allocation_scenarios_rank_through_the_one_ev_engine',
    target_module: 'lib',
    side_effect_class: 'pure',
    next_action: 'verified: AllocationScenario maps onto ScenarioBranch and ranks through the one evidence-weighted EV engine; no forked ranking and no write/money side effect',
  }],
  [150, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-company --lib governance::tests::a_layer_viewpoint_carries_vision_constitution_kpi_authority_memory',
    target_module: 'governance',
    side_effect_class: 'pure',
    next_action: 'verified: GovernanceLayerKind is the canonical six-layer viewpoint taxonomy and each layer carries scope, vision, constitution, KPI, authority, memory visibility, and audit responsibility',
  }],
  [155, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-company --lib governance::tests::governance_audit_records_which_layer_decided_and_why',
    target_module: 'governance',
    side_effect_class: 'pure',
    next_action: 'verified: Governance audit event records the deciding layer, escalation origin, reason, scope, and outcome without writing money or bypassing the gate',
  }],
  [156, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-world-model --lib signal_carries_source_evidence_confidence_timestamp_scope',
    target_module: 'lib',
    side_effect_class: 'pure',
    next_action: 'verified: External world signals carry evidence, confidence, timestamp, and ScopePath scope as pure advisory primitives',
  }],
  [157, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-world-model --lib scenario::tests::fifteen_canonical_scenarios_exist',
    target_module: 'scenario',
    side_effect_class: 'pure',
    next_action: 'verified: The canonical 15 macro scenario set exists and maps distinct external signals without side effects',
  }],
  [158, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-world-model --lib viewpoint::tests::same_signal_yields_six_distinct_layer_interpretations',
    target_module: 'viewpoint',
    side_effect_class: 'pure',
    next_action: 'verified: Same external signal is interpreted through distinct layer viewpoints and decision context refuses missing required maps',
  }],
  [159, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-simulation --lib macro_shock_maps_to_branch_and_raises_risk',
    target_module: 'lib',
    side_effect_class: 'pure',
    next_action: 'verified: MacroScenario maps onto ScenarioBranch and macro downside feeds RiskClass for the pre-execution gate; advisory only, no write/money side effect',
  }],
  [161, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-memory --lib memory_scope::tests',
    target_module: 'memory_scope',
    side_effect_class: 'pure',
    next_action: 'verified: MemoryScopeSet enforces ScopePath-based memory visibility, blocks sibling raw reads without a contract, allows group rollup/shared reads, and blocks cross-group access',
  }],
  [181, {
    status: 'green',
    percent: 70,
    acceptance_test: 'cargo test -p chronica-world-model --lib template_assumptions::tests',
    target_module: 'template_assumptions',
    side_effect_class: 'pure',
    blocker: 'External world assumptions and triggers exist, but full S181 still needs the macro-to-MiroFish bridge and macro-aware M&A/template consumer before verification.',
    next_action: 'partial: Template external assumptions and threshold triggers are implemented; finish bridge/consumer wiring before marking the slice verified',
  }],
  [178, {
    status: 'verified',
    percent: 100,
    acceptance_test: 'cargo test -p chronica-strategy ma_lens',
    target_module: 'ma_lens',
    side_effect_class: 'pure',
    next_action: 'verified: M&A Go/No-Go, kill/scale criteria, EV reuse, and governance routing are backed by chronica-strategy ma_lens tests; advisory only, no capital commit',
  }],
])

const evidenceBySlice = new Map([
  [121, ['crates/chronica-company/src/model.rs', 'Company,CompanyModel,CompanyTemplate', 'crates/chronica-company/src/model.rs', 'company_and_model_roundtrip_json']],
  [122, ['crates/chronica-company/src/workspace.rs', 'create_company_from_company_template,InstantiatedCompany', 'crates/chronica-company/src/workspace.rs', 'one_company_template_creates_many_companies']],
  [123, ['crates/chronica-template-store/src/lib.rs', 'instantiate_runtime,RuntimeInstantiationRequest,RuntimeInstantiation', 'crates/chronica-template-store/src/tests.rs', 'runtime_instantiation_creates_project_company_and_group_objects']],
  [127, ['crates/chronica-company/src/inter_company.rs', 'CompanyContract,InterCompanyTransfer,invoice_finance_legs', 'crates/chronica-company/src/inter_company.rs', 'inter_company_transfer_full_chain']],
  [128, ['crates/chronica-company/src/group.rs', 'Group,GroupTemplate,create_group_from_group_template', 'crates/chronica-company/src/group.rs', 'group_template_creates_a_group_with_planned_members']],
  [130, ['crates/chronica-company/src/governance.rs', 'GovernanceLayerKind,DecisionContext,route_decision', 'crates/chronica-company/src/governance.rs', 'irreversible_money_escalates_to_board_at_any_amount']],
  [131, ['crates/chronica-memory/src/memory_scope.rs', 'MemoryItem,MemoryScopeSet,can_read', 'crates/chronica-memory/src/memory_scope.rs', 'group_layer_reads_descendant_workspace_memory']],
  [132, ['crates/chronica-simulation/src/lib.rs', 'GroupPortfolioVariant,CompanyPortfolioBranch,rank_group_portfolio_variants', 'crates/chronica-simulation/src/lib.rs', 'group_portfolio_variants_rank_by_consolidated_ev']],
  [134, ['crates/chronica-company/src/workspace.rs', 'ProjectTemplateSynchronizationReport,synchronize_project_template_bindings', 'crates/chronica-company/src/workspace.rs', 'project_template_sync_updates_live_bindings_across_companies']],
  [135, ['crates/chronica-company/src/allocation/mod.rs', 'CostAttribution,attribute_cost,ResourceUsage', 'crates/chronica-company/src/allocation/tests.rs', 'cost_attribution_records_usage_without_an_invoice']],
  [136, ['crates/chronica-company/src/group.rs', 'SharedAgentPool,SharedAgentLease,lease_shared_agent', 'crates/chronica-company/src/group.rs', 'shared_agent_pool_leases_group_agents_to_allowed_companies_only']],
  [137, ['crates/chronica-company/src/group.rs', 'GroupReplicationRequest,GroupReplicationEmit,emit_group_replication', 'crates/chronica-company/src/group.rs', 'group_replication_emit_requires_approved_council_decision_and_targets_each_company']],
  [138, ['crates/chronica-memory/src/rag.rs', 'ScopedRagCandidate,scoped_rag_retrieve,hybrid_score', 'crates/chronica-memory/src/rag.rs', 'scoped_rag_retrieve_filters_before_ranking']],
  [139, ['crates/chronica-memory/src/learning.rs', 'GroupLearningRollup,roll_up_group_learning,LearningRecord', 'crates/chronica-memory/src/learning.rs', 'group_learning_rollup_creates_shared_summary_without_exposing_private_records']],
  [140, ['crates/chronica-workflows/src/self_evolving/mod.rs', 'GroupScopeWorkflowChange,group_scope_workflow_change,WorkflowChangeProposal', 'crates/chronica-workflows/src/self_evolving/tests.rs', 'group_scope_workflow_change_routes_to_council_and_targets_member_companies']],
  [141, ['crates/chronica-evidence-debate/src/lib.rs', 'GroupSimulationDebateReport,debate_group_portfolio_simulation,GroupSimulationDebateError', 'crates/chronica-evidence-debate/src/lib.rs', 'group_simulation_debate_selects_best_variant_and_requires_council_view']],
  [142, ['crates/chronica-company/src/governance.rs', 'GovernanceLayerKind,DecisionContext,route_decision', 'crates/chronica-company/src/governance.rs', 'new_subsidiary_routes_to_council']],
  [143, ['crates/chronica-core/src/scope.rs', 'ScopePath,ScopeType,ScopePath::contains,ScopePath::canonical_key', 'crates/chronica-core/src/scope.rs', 'cross_account_and_cross_group_are_structurally_isolated']],
  [147, ['crates/chronica-simulation/src/lib.rs', 'allocation_scenario_to_branch,rank_allocation_scenarios,ScenarioBranch', 'crates/chronica-simulation/src/lib.rs', 'allocation_scenarios_rank_through_the_one_ev_engine']],
  [150, ['crates/chronica-company/src/governance.rs', 'GovernanceLayerKind,GovernanceLayer,VisionStatement,ConstitutionRule,default_authority_limit', 'crates/chronica-company/src/governance.rs', 'a_layer_viewpoint_carries_vision_constitution_kpi_authority_memory']],
  [155, ['crates/chronica-company/src/governance.rs', 'governance_audit_event,GovernanceAuditEvent,DecisionRoute,EscalationReason', 'crates/chronica-company/src/governance.rs', 'governance_audit_records_which_layer_decided_and_why']],
  [156, ['crates/chronica-world-model/src/lib.rs', 'SignalKind,SignalCommon,ExternalSignal,ExternalWorldMap', 'crates/chronica-world-model/src/lib.rs', 'signal_carries_source_evidence_confidence_timestamp_scope']],
  [157, ['crates/chronica-world-model/src/scenario.rs', 'MacroScenarioKind,MacroScenario,macro_scenario_branches,scenario_for_signal', 'crates/chronica-world-model/src/scenario.rs', 'fifteen_canonical_scenarios_exist']],
  [158, ['crates/chronica-world-model/src/viewpoint.rs', 'LayerWorldview,interpret_signal,interpret_all_layers', 'crates/chronica-world-model/src/viewpoint.rs', 'same_signal_yields_six_distinct_layer_interpretations']],
  [149, ['crates/chronica-company/src/allocation/mod.rs', 'PortfolioAllocationDecision,capital_allocation_action', 'crates/chronica-company/src/allocation/tests.rs', 'portfolio_decision_money_gates_only_major_capital_legs']],
  [154, ['crates/chronica-company/src/group.rs', 'GroupCouncil,GroupPresident,CapitalAllocationMandate', 'crates/chronica-company/src/group.rs', 'capital_mandate_links_to_a_council_decision']],
  [159, ['crates/chronica-simulation/src/lib.rs', 'macro_scenario_to_branch,macro_scenario_risk,ScenarioBranch,RiskClass', 'crates/chronica-simulation/src/lib.rs', 'macro_shock_maps_to_branch_and_raises_risk']],
  [161, ['crates/chronica-memory/src/memory_scope.rs', 'MemoryScopeSet,MemoryShareGrant,MemoryItem,can_read', 'crates/chronica-memory/src/memory_scope.rs', 'cross_group_contract_grant_still_cannot_read_private_memory']],
  [165, ['crates/chronica-company/src/field_execution.rs', 'contractor_payment_action,ContractorAssignment', 'crates/chronica-company/src/field_execution.rs', 'contractor_payment_is_irreversible_and_money_gated']],
  [169, ['crates/chronica-erp/src/order_flow.rs', 'JournalEntry,PurchaseOrder,SalesOrder', 'crates/chronica-erp/src/order_flow.rs', 'a_balanced_journal_posts']],
  [170, ['crates/chronica-erp/src/order_flow.rs', 'JournalEntry,post_journal_entry', 'crates/chronica-erp/src/order_flow.rs', 'an_unbalanced_journal_is_rejected']],
  [177, ['crates/chronica-strategy/src/ma_lens.rs', 'DueDiligenceReport,InvestmentThesis,UnitEconomicsAssessment', 'crates/chronica-strategy/src/ma_lens.rs', 'ma_lens_reuses_the_simulation_ev_engine']],
  [178, ['crates/chronica-strategy/src/ma_lens.rs', 'decide_go_no_go,route_ma_review,KillCriteria,ScaleCriteria,GoNoGoDecision', 'crates/chronica-strategy/src/ma_lens.rs', 'ma_review_routes_by_scale_not_to_president_by_default']],
  [179, ['crates/chronica-company/src/pm_lens.rs', 'ProjectCharter,WorkBreakdownStructure,critical_path_days', 'crates/chronica-company/src/pm_lens.rs', 'critical_path_is_the_longest_dependency_chain']],
  [180, ['crates/chronica-company/src/pm_lens.rs', 'ProjectCharter,budget_commit_action,RaciAssignment', 'crates/chronica-company/src/pm_lens.rs', 'budget_commit_is_money_gated_zero_is_not']],
  [183, ['crates/chronica-template/src/lib.rs', 'TemplateProgram,verify_completeness,TemplateTier', 'crates/chronica-template/src/lib.rs', 'a_fully_specified_template_passes_the_15_point_gate']],
  [190, ['crates/chronica-template-store/src/lib.rs', 'TemplateStore,instantiate_runtime,RuntimeInstantiation', 'crates/chronica-template-store/src/tests.rs', 'runtime_instantiation_creates_project_company_and_group_objects']],
  [193, ['crates/chronica-commerce/src/lib.rs', 'CommerceErpBinding,commerce_money_action', 'crates/chronica-commerce/src/lib.rs', 'commerce_erp_binding_maps_listing_to_an_item']],
])

const nonMoneyAnchorSlices = new Set([
  // Admin/board role routing decides who may approve money, but defining the roles
  // does not itself move money.
  142,
  // Hierarchical approval/routing decides who must approve. It is a policy check
  // around the money gate, not the money movement itself.
  130,
  // M&A Go/No-Go decides and routes through governance; it never commits capital or
  // writes a CostRecord. Actual allocation remains behind the one money gate.
  178,
])

const updateSlice = db.prepare(`
  UPDATE capability
     SET status=@status,
         percent=@percent,
         acceptance_test=COALESCE(@acceptance_test, acceptance_test),
         fin_test=COALESCE(@fin_test, fin_test),
         next_action=@next_action,
         blocker=@blocker
   WHERE kind='slice' AND slice=@slice
`)

const updateSliceMoney = db.prepare(`
  UPDATE capability
     SET moves_money=@moves_money,
         fin_test=CASE WHEN @moves_money=0 THEN '' ELSE fin_test END
   WHERE kind='slice' AND slice=@slice
`)

const upsertCanon = db.prepare(`
  INSERT INTO canonical_capability (
    key, canonical_name, domain, target_crate, target_module, side_effect_class,
    moves_money, requires_approval, acceptance_criteria, required_tests,
    financial_control_test, acceptance_test, status, exclusion_note, blocker,
    slice, donor_count
  ) VALUES (
    @key, @canonical_name, @domain, @target_crate, @target_module, @side_effect_class,
    @moves_money, @requires_approval, @acceptance_criteria, @required_tests,
    @financial_control_test, @acceptance_test, @status, NULL, @blocker,
    @slice, 0
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
    status=excluded.status,
    blocker=excluded.blocker,
    slice=excluded.slice
`)

const upsertLink = db.prepare(`
  INSERT OR REPLACE INTO slice_canonical (slice, canonical_key, match_method, confidence, reviewed, note)
  VALUES (@slice, @canonical_key, 'runtime-slice-anchor', 'high', 1, @note)
`)

const upsertOverride = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,financial_control_test,blocker,set_at,set_by,moves_money)
  VALUES (@canonical_key,@status,@acceptance_test,@financial_control_test,@blocker,@set_at,'codex',@moves_money)
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=excluded.status,
    acceptance_test=excluded.acceptance_test,
    financial_control_test=excluded.financial_control_test,
    blocker=excluded.blocker,
    set_at=excluded.set_at,
    set_by=excluded.set_by,
    moves_money=excluded.moves_money
`)

const upsertEvidence = db.prepare(`
  INSERT INTO impl_evidence (canonical_key,impl_file,impl_symbols,test_file,test_symbol,donor_source,verified_at,verified_by,notes)
  VALUES (@canonical_key,@impl_file,@impl_symbols,@test_file,@test_symbol,'chronica-runtime-slice-anchor',@verified_at,'codex',@notes)
  ON CONFLICT(canonical_key) DO UPDATE SET
    impl_file=excluded.impl_file,
    impl_symbols=excluded.impl_symbols,
    test_file=excluded.test_file,
    test_symbol=excluded.test_symbol,
    donor_source=excluded.donor_source,
    verified_at=excluded.verified_at,
    verified_by=excluded.verified_by,
    notes=excluded.notes
`)

const deleteEvidence = db.prepare('DELETE FROM impl_evidence WHERE canonical_key=@canonical_key')

const setMeta = db.prepare('INSERT OR REPLACE INTO meta (k,v) VALUES (?,?)')

const tx = db.transaction(() => {
  for (const [slice, patch] of sliceUpdates) {
    updateSlice.run({
      slice,
      status: patch.status,
      percent: patch.percent,
      acceptance_test: patch.acceptance_test ?? null,
      fin_test: patch.fin_test ?? null,
      next_action: patch.next_action,
      blocker: patch.status === 'verified' ? null : (patch.blocker ?? null),
    })
  }
  for (const slice of nonMoneyAnchorSlices) {
    updateSliceMoney.run({ slice, moves_money: 0 })
  }

  const rows = db.prepare(`
    SELECT slice,title,status,percent,target_crate,acceptance_test,fin_test,moves_money,next_action,blocker
      FROM capability
     WHERE kind='slice' AND slice BETWEEN 121 AND 195
     ORDER BY slice
  `).all()

  for (const row of rows) {
    const key = `slice.${row.slice}.${slug(row.title)}`
    const status = canonicalStatus(row.status)
    const blocker = status === 'verified' ? null : (row.blocker || null)
    const movesMoney = nonMoneyAnchorSlices.has(row.slice)
      ? 0
      : row.fin_test || row.moves_money
        ? 1
        : 0
    upsertCanon.run({
      key,
      canonical_name: row.title,
      domain: domainFor(row.slice, row.title, row.target_crate),
      target_crate: sliceUpdates.get(row.slice)?.target_crate ?? row.target_crate,
      target_module: sliceUpdates.get(row.slice)?.target_module ?? null,
      side_effect_class: sliceUpdates.get(row.slice)?.side_effect_class ?? (movesMoney ? 'money_write' : 'internal_write'),
      moves_money: movesMoney,
      requires_approval: movesMoney,
      acceptance_criteria: `Runtime slice anchor for S${row.slice}: ${row.title}`,
      required_tests: row.acceptance_test || null,
      financial_control_test: row.fin_test || null,
      acceptance_test: row.acceptance_test || null,
      status,
      blocker,
      slice: row.slice,
    })
    upsertLink.run({
      slice: row.slice,
      canonical_key: key,
      note: `S${row.slice} canonical runtime anchor: ${row.title}`,
    })
    upsertOverride.run({
      canonical_key: key,
      status,
      acceptance_test: row.acceptance_test || null,
      financial_control_test: row.fin_test || null,
      blocker,
      set_at: now,
      moves_money: movesMoney,
    })
    if (status === 'verified') {
      const evidence = evidenceBySlice.get(row.slice)
      if (!evidence) throw new Error(`verified slice S${row.slice} needs impl evidence mapping`)
      const [impl_file, impl_symbols, test_file, test_symbol] = evidence
      upsertEvidence.run({
        canonical_key: key,
        impl_file,
        impl_symbols,
        test_file,
        test_symbol,
        verified_at: now,
        notes: `Verified runtime slice anchor S${row.slice}: ${row.title}`,
      })
    } else {
      deleteEvidence.run({ canonical_key: key })
    }
  }

  const canonicalCount = db.prepare('SELECT count(*) n FROM canonical_capability').get().n
  const moneyCount = db.prepare('SELECT count(*) n FROM canonical_capability WHERE moves_money=1').get().n
  setMeta.run('canonical_capability_count', String(canonicalCount))
  setMeta.run('money_canonical_count', String(moneyCount))
  setMeta.run('company_group_runtime_reconciled_at', now)
})

tx()

const linked = db.prepare('SELECT count(*) n FROM slice_canonical WHERE slice BETWEEN 121 AND 195').get().n
const verified = db.prepare("SELECT count(*) n FROM canonical_capability WHERE key LIKE 'slice.%' AND status='verified'").get().n
console.log(`reconcile-company-group-runtime: linked ${linked} slice anchors; ${verified} verified slice anchors`)
db.close()
