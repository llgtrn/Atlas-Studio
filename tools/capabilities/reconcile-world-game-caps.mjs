// Seed the 5 design-provenance canonical capabilities for the World Game advisory brief
// (chronica-strategy::world_game). These are architecture/design-driven (not donor-sourced); they enter the
// registry as `unimplemented`, then `record-verified --config` flips them to `verified` (moves_money=0) with
// impl evidence. Idempotent (ON CONFLICT DO UPDATE). Run: node tools/capabilities/reconcile-world-game-caps.mjs
import Database from 'better-sqlite3'
import { CAPABILITIES_DB } from '../_paths.mjs'

const db = new Database(CAPABILITIES_DB)

const CAPS = [
  {
    key: 'strategy.world_game.compose_brief',
    side_effect_class: 'pure',
    canonical_name: 'compose world game brief',
    acceptance_criteria:
      'Composes a deterministic 23-section game-theoretic ADVISORY brief from an opportunity + BI data quality (+ optional payoff matrix + open position), reusing cognition::assess_opportunity and the chronica-analytics data-quality gate (no forked engine, no money call-site). Deny-by-default on data (weak data caps Go at TestSmall). NEVER authorizes money / executes / places an order (authorizes_money()==is_executing()==places_order()==false). Deterministic.',
    required_tests:
      'compose_report_strong_input_clean_data_is_not_capped_and_is_money_safe,compose_report_weak_data_caps_strong_opportunity_to_test_small,compose_report_with_payoff_matrix_surfaces_the_equilibrium_and_trap,compose_report_with_position_surfaces_take_profit_and_renders_sections_15_to_22,compose_report_is_deterministic,section_taxonomy_has_23_sections_in_order',
    acceptance_test: 'compose_report_strong_input_clean_data_is_not_capped_and_is_money_safe',
  },
  {
    key: 'strategy.world_game.decide',
    side_effect_class: 'pure',
    canonical_name: 'decide world game action',
    acceptance_criteria:
      'Maps the cognition decision + the BI data-quality gate into the World-Game decision ladder (Go/NoGo/TestSmall/Hedge/Scale/Kill, plus the position moves TakeProfit/StopLoss). DENY-BY-DEFAULT: weak data quality caps Go/Scale at TestSmall (validate before committing); all other outcomes pass through. Pure + deterministic; never authorizes money.',
    required_tests: 'weak_data_quality_caps_go_to_test_small,no_go_and_kill_pass_through_regardless_of_data',
    acceptance_test: 'weak_data_quality_caps_go_to_test_small',
  },
  {
    key: 'strategy.world_game.solve_equilibrium',
    side_effect_class: 'pure',
    canonical_name: 'solve payoff-matrix equilibrium',
    acceptance_criteria:
      'Solves a finite 2-player normal-form payoff matrix: best response, ALL pure-strategy Nash equilibria (index-ordered), dominant strategy, Pareto-dominated trap (prisoners-dilemma) detection, social optimum, and the intervention lever (welfare-superior target + the deviation incentive a mechanism must neutralize to make it stable). Integer payoffs, finite, no RNG, byte-stable (same matrix => same analysis). Validated input (empty/ragged matrices rejected). Pure data — moves no money.',
    required_tests:
      'prisoners_dilemma_has_a_single_dominated_nash_trap,intervention_lever_targets_the_optimum_and_quantifies_the_deviation_incentive,coordination_game_has_two_untapped_equilibria,matching_pennies_has_no_pure_nash,validation_rejects_empty_and_ragged_matrices,analysis_is_deterministic_and_byte_stable,dominant_strategy_is_a_best_response_to_every_opponent_move',
    acceptance_test: 'prisoners_dilemma_has_a_single_dominated_nash_trap',
  },
  {
    key: 'strategy.world_game.evaluate_position',
    side_effect_class: 'pure',
    canonical_name: 'evaluate take-profit / stop-loss',
    acceptance_criteria:
      'Take-profit (利確) / stop-loss (損切り) rules for an open position. Risk-first precedence: thesis-break -> hard stop-loss floor -> take-profit ceiling -> trailing give-back -> hold; produces the TakeProfit/StopLoss World-Game decisions. Integer cents/bps, div-by-zero + overflow safe, deterministic. ADVISORY: evaluating a position never places an order (places_order()==false).',
    required_tests:
      'take_profit_when_gain_exceeds_ceiling,stop_loss_when_loss_exceeds_floor,thesis_invalidation_forces_stop_loss_even_in_profit,trailing_give_back_locks_profit_while_still_up,hold_within_bounds,verdict_never_places_order,evaluation_is_deterministic_and_div_by_zero_safe',
    acceptance_test: 'take_profit_when_gain_exceeds_ceiling',
  },
  {
    key: 'strategy.world_game.render_mermaid',
    side_effect_class: 'pure',
    canonical_name: 'render world game mermaid diagrams',
    acceptance_criteria:
      'Renders three typed, deterministic Mermaid diagrams for the brief — static battlefield graph (§⑳), game-theory flow strategies->Nash->trap->target->lever (§㉑), and BI/action flow data->cognition->World-Game->governance (§㉒). Node labels are sanitized so arbitrary scope/strategy text cannot break the Mermaid grammar; byte-stable (same brief => identical Mermaid). Pure strings — no money, no side effect.',
    required_tests:
      'static_graph_starts_with_graph_td_and_has_core_nodes,game_theory_flow_renders_the_trap_and_lever,game_theory_flow_handles_no_matrix,bi_action_flow_shows_the_full_pipeline_and_position,render_is_deterministic,label_neutralizes_mermaid_breaking_characters',
    acceptance_test: 'game_theory_flow_renders_the_trap_and_lever',
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
    upsert.run({ domain: 'strategy', target_crate: 'chronica-strategy', target_module: 'world_game', ...c })
})
tx()

const total = db.prepare('SELECT count(*) n FROM canonical_capability').get().n
db.prepare('INSERT OR REPLACE INTO meta (k,v) VALUES (?,?)').run('canonical_capability_count', String(total))
console.log(`seeded ${CAPS.length} design-provenance caps (strategy.world_game.*); canonical total ${total}`)
