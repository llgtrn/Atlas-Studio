---
id: atlas.decision.0007.three-valued-constraint-verdict
type: decision
status: accepted
canonical: true
---
# ADR 0007 — Three-valued constraint verdict

## Context

`.atlas/contracts/ARCHITECTURAL-INTEGRITY.md` says: "If Atlas cannot evaluate a required hard invariant, the result is UNKNOWN/INCOMPLETE, not PASS."

`ConstraintResult.passed: bool` could not express that third state:

- "no evaluable checks" (`ATLAS-E053`) was folded into `passed: false`;
- "the required attribute is not declared at all" was reported as a violation (`ATLAS-E050`), even though no counterexample exists.

As a result, `coding_admission` could not distinguish "a counterexample exists" from "Atlas cannot decide". The G37 architecture lane named this the formal-constraint primitive with a real production consumer. No SAT/SMT consumer exists yet: every current check is ground evaluation over finite declared facts.

## Decision

1. `core::constraint::ConstraintVerdict { Satisfied, Violated, Unknown }`.
   - Conjunction is strong Kleene: a definite counterexample decides; otherwise any undecidable conjunct leaves the result undecided.
   - `admits()` holds only for `Satisfied`.
   - `Default` is `Unknown`, so a missing verdict is never a pass.
2. `ConstraintResult.verdict` is added. `passed` is kept as `verdict.admits()`.
3. Evaluation rules:
   - An attribute declared with a different value is `Violated` (`ATLAS-E050`).
   - An attribute not declared on a relevant node is `Unknown` (new `ATLAS-E055`).
   - A missing materialization is `Violated` (`ATLAS-E051`). The declared graph is closed-world for materializations.
   - A declaration with zero checks is `Unknown` (`ATLAS-E053`).
   - Observed-materialization deltas are `Violated`.
4. Admission raises `ADL_CONSTRAINT_VIOLATED` for any `Violated` result and a new `ADL_CONSTRAINT_UNKNOWN` for any `Unknown` result. Both block admission; neither is ever promoted to a pass. The engineering-graph projection carries a `verdict` attribute.

## Consequences

- Reports now separate "refuted" from "undecided", which is the verdict shape any future solver must return: SAT/UNSAT/UNKNOWN maps onto Satisfied/Violated/Unknown.
- A native SAT/SMT core is `ABSORB_LATER`. Z3 and cvc5 are its frontier donors (`.atlas/roadmap/FRONTIER.toml`, lane `formal_reasoning`). It stays there until a consumer needs search rather than ground evaluation: ADL constraints with disjunction or implication, or `DecisionProposal.hard_constraint`.
- On this repository, all 4 declared constraints still evaluate `SATISFIED` and admission is unchanged.
