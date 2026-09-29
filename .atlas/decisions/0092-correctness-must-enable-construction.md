---
id: atlas.decision.0092.correctness-must-enable-construction
type: decision
status: accepted
canonical: true
---
# ADR 0092 — Correctness must enable construction, not permanently replace it (G179)

## Context

For many generations the loop has hardened the same Rust semantic layer: call and type resolver residuals, found by replays and fixed natively. Each generation removes real errors, but it adds less capability and costs more to prove.

G178 (replay R15, egglog) was measured, not estimated (`evidence/verification/G178-verification-cost.json`):
- 21,705 s of wall clock (6.0 h);
- 294 s of machine replay stages (1.4%);
- 11,843 s of investigation and fix rounds and 4,090 s of review rounds (73% together);
- 3,209 s lost to a restart and stale runs (15%);
- 2,269 s of gates and proofs (10%).

It delivered one capability epoch (E24), no construction node, and removed four classes of WRONG claims. Under the rubric below that is 0.33 capability delta per hour.

The ledgers also carried stale truth. The construction graph still listed M8 (seal eligibility gate) and M9 (SEALED encoder and reader) as MISSING, although both exist since G161 (ADR 0076, `core/src/seal/gate.rs`, `core/src/atlas`). No rule forced the graph to be checked, and no lane ever needed it.

Correctness stays mandatory: UNKNOWN is better than an unjustified claim. This decision adds that correctness must enable construction, not permanently replace it.

## Decision

1. **Two lanes.** From G179 every generation carries `lane = "HARDENING"` or `lane = "CREATION"`.
   - HARDENING: donor and replay falsification of existing capabilities, native hardening attacks, revalidations.
   - CREATION: one new vertical capability toward ADL, design, `.atlas`, construction, `.atlasx`, an executable or product, and self-reconstruction. Its kind is `CREATION`, which is not an interleaving class.
   - No more than one HARDENING generation in a row (`max_consecutive_hardening = 1`). The only exception is a HARDENING generation with `correctness_blocker = "<residual>"`, naming an OPEN residual that can create a false claim. It takes the slot CREATION would have had: REPLAY, NATIVE (blocker), REPLAY, CREATION.
   - The replay cadence (ADR 0067, one non-replay generation at most between replays) is an owner gate and is unchanged. CREATION and a correctness blocker count as non-replay generations. Together the two cadences give REPLAY, CREATION, REPLAY, CREATION.
   - **The tradeoff.** Replays keep every other generation, and native resolver hardening pauses unless a false-claim residual blocks correctness. Reopening regular native slots needs a new policy decision, not silent drift.
   - Rule B keeps pressure selection (Rule A is unchanged) and `next_native_attack` still names the head. Its `planned_generation` is `LANE_DEFERRED` unless an OPEN false-claim residual names it (`attack = "<id>"` in RESIDUALS.toml). Then it is planned in the next CREATION slot as `CORRECTNESS_BLOCKER <residual>`, with only replays or interleaving generations planned before it.
   - Policy: PRIORITY.toml `[generation_lanes]`. Tests: `creation_lane_is_never_starved`, `replay_cadence_is_machine_enforced`, `native_attack_queue_is_ordered_and_heads_selection`.
2. **Vertical value.** A construction node's vertical value is 1 plus the number of MISSING nodes that transitively require it. A debt's vertical value is the largest over its MISSING nodes, or 0.
   - Pressure becomes stale generations × (1 + debts it blocks) + 10 × vertical value. The map records `vertical_weight` and each entry's `vertical_value`, and the test recomputes the formula and the order.
   - CREATION attacks name their `construction_node`. Among those whose node is MISSING and reachable, the score is vertical value × 10 plus the summed staleness of the attack's debts. The winner is recorded in `[creation_selection]` and must be named by the next planned CREATION generation (`creation_selection_follows_vertical_value`).
   - The HARDENING selection (`[selection]`) is unchanged, restricted to HARDENING attacks.
3. **The construction graph, checked against the code.**
   - M8 and M9 are EXISTS since G161.
   - M15 (phase-1 delegated backend) now requires M14 (executable body semantics). The first scope is code-bearing (the host Rust target), the delegated backend closes its loop by compile and test (COMPILER-IR-PIPELINE), and the HIR verifier rejects a missing body for a required executable function (COMPILER-IR-SCHEMAS). An architecture-only first scope would remove the edge.
   - M16 stays MISSING. The restricted-subprocess backend exists, but toolchain identity and filesystem confinement do not.
   - The reachable MISSING nodes are M10, M12 and M14. Their CREATION attacks are NA-ATLASX-PRECONDITION (new, M10), NA-ATLASX-CODEC (M12) and NA-SELF-RECONSTRUCTION-BODIES (M14). Each objective is the smallest slice that moves its node to EXISTS, with a falsification test.
4. **Residual ledger.** `roadmap/RESIDUALS.toml` (`atlas.residuals.v1`) holds each deferred gap:
   - its exact reproduction, severity and subsystem;
   - whether it can create a false claim, and if so the exact condition and why it does not hold on the current tree;
   - its reopen trigger, its status, and a pressure of severity weight (1, 3, 9) × 3 when it can create a false claim.

   G178's twelve deferred findings are recorded. Three can create a false claim: a registry or git dependency whose `[lib] name` differs from its key, crates passed by `--extern` outside Cargo, and a non-normalized registry `[lib] name` misread by `locked_traits`. None holds on the current tree.
5. **Complexity budget.** A donor exposes A, A exposes B, B exposes C, D, E and F. A generation fixes what the current tree's known-wrong claims require and records the rest as residuals. It may expand only when the new issue can falsify the tree's claimed truth or invalidate the generation's own evidence. Each expansion is a `[[generation.expansion]]` entry with justification `FALSIFIES_CURRENT_TRUTH` or `INVALIDATES_GENERATION_EVIDENCE`.
6. **Value metric.** Every generation from G179 carries `[generation.value]`:
   - `capability_delta` = 3 × construction nodes completed + 2 × new capabilities (a capability epoch, or a new native user-visible command or record) + 1 × debt maturity steps;
   - wall-clock, machine, proof and review seconds;
   - wrong claims removed, residuals deferred, and whether the generation was correctness-critical;
   - `frontier_rate` = capability delta per wall-clock hour. It is LOW_FRONTIER_PROGRESS below 0.5 per hour unless correctness-critical.

   The test recomputes the metric. The G178 baseline lives in PRIORITY.toml, not in G178's ledger entry: delta 2, 0.33 per hour, FRONTIER_PROGRESS only because it removed WRONG claims.
7. **Schedule.**
   - G179: CREATION, NA-ATLASX-PRECONDITION (M10, score 230).
   - G180: replay of rust-lang/datafrog.
   - G181: CREATION, re-selected after G179 (forecast NA-ATLASX-CODEC, 220).
   - G182: FULL_OSS_REPLAY of the replay ledger's next target after datafrog (differential-dataflow by today's revalidation ranking).
   - G183: CREATION, the selection winner at that time.
   - NA-CALL-TYPE-RESIDUAL stays the HARDENING head (pressure 97), `LANE_DEFERRED`: no false-claim residual names it.

## Consequences

- Construction work now has a guaranteed share: every other generation. Hardening keeps its replay stream, which still fixes natively what a replay falsifies, and a known-wrong claim can always take a CREATION slot. The native attack queue does not advance on its own until a policy change reopens its slots.
- The selected CREATION work is the AtlasX precondition gate. It can be built and falsified on the G161 sealed fixture, although the self scope is not sealable: no SELECTED design exists, and the certificate is RECONCILED.
- A stale node in the construction graph now distorts both pressures, so graph truth has a consumer.
- Construction work without a node in the graph (target profile, the `.atlas` DESIGN section, weights, transformation) stays HARDENING-labelled until its node is added.
