---
id: atlas.decision.0056.architectural-integrity-envelope-and-report
type: decision
status: accepted
canonical: true
---
# ADR 0056 — The architectural integrity envelope and its evaluated report (G138, NA-INTEGRITY-ENVELOPE)

## Context

`ARCHITECTURAL-INTEGRITY.md` and its two schemas had been CONTRACT only since G63 (`DEBT-INTEGRITY_ENVELOPE` at `M1_CONTRACT_ONLY`).

Since then, G130 added census-quantified layering invariants and G137 added effect envelopes, both decided over the composed census. The remaining gap was the envelope record that pins such invariants as selected architecture, and the report that evaluates a candidate against the pin (construction milestones M5/M6). Without them, nothing stopped an invariant from being weakened to make a candidate pass.

## Decision

1. **`core::integrity::derive_envelope`** lowers the declared architecture that Atlas can falsify with its census into typed invariants that follow `architectural-integrity-envelope.schema.json`:

   | Declaration | Invariant |
   |---|---|
   | `forbid call to` | HARD `DEPENDENCY_DIRECTION` |
   | `forbid effect`, `observed effect within` | HARD `AUTHORITY_BOUNDARY` |
   | `depends_on` between two Rust materializations | HARD `DEPENDENCY_DIRECTION` |
   | `depends_on` between entities the dependency census cannot observe | SOFT, `DIAGNOSTIC_ONLY`, with a note |

   - **Invariant fields.** Every invariant carries its identity (`AIE:<name>`), subjects, statement, evaluator (`rule_ref = adl:<result name>`), a falsification condition, its violation action and its declaration's location.
   - **Elements.** Materialized entities become elements: `LOAD_BEARING` when a HARD invariant names them, else `STRUCTURAL`.
   - **Unpinned references.** `genome_ref` says `UNPINNED`, and `selected_design_ref` is null: no Genome or SelectedDesign record exists yet (`DEBT-SELECTED_DESIGN`). Neither is invented.
2. **The pin.** The envelope's identity is a BLAKE3 digest of its invariants' canonical text. The repository pins its envelope at `.atlas/declared/integrity-envelope.json`. `integrity envelope --check` and a test fail when the declared envelope drifts from the pin, so any change to an invariant is an explicit, reviewable re-pin.
3. **`core::integrity::evaluate`** evaluates the pinned envelope against one candidate, following `architectural-integrity-report.schema.json`.
   - **Per invariant.** Each pinned invariant is `PASS`, `FAIL` or `UNKNOWN`, from the decided constraint result it cites.
   - **Missing or changed invariants.** A pinned invariant that the candidate's ADL no longer declares identically is `FAIL` with `ARCHITECTURE_UNSELECTED_REVISION`, so deleting a violated invariant to pass is rejected.
   - **Diagnostics.** A HARD `FAIL` carries `ARCHITECTURE_HARD_VIOLATION`, and a HARD `UNKNOWN` carries `ARCHITECTURE_REQUIRED_UNKNOWN`. A SOFT `UNKNOWN` never blocks.
   - **Verdict.** `REJECTED` on any hard violation, `INCOMPLETE` on any required unknown, else `ELIGIBLE`.
   - **Consistency.** `check_report` re-derives the counts, the verdict, the pinned identity and the coverage of every pinned invariant (the contract's internal-consistency rule).
   - **Impact closure.** The report states it as closed by `FULL_RECOMPUTE`: every pinned invariant is evaluated over the full census.
4. **`atlas-systemizer integrity report`** exits `INTEGRITY_NOT_ELIGIBLE` unless the verdict is `ELIGIBLE`.

## Evidence

- **Atlas's own envelope.** It has 15 invariants: 14 HARD (6 layering, 4 effect envelopes, 4 Cargo dependencies) and 1 SOFT (WebUI → Runtime, which has no TypeScript dependency census). Four elements are `LOAD_BEARING` and WebUI is `STRUCTURAL`. The report is `ELIGIBLE`, with 0 hard violations and 0 required unknowns.
- **Schema conformance.** Both records conform to their schemas, checked by a recursive conformance check in the test.
- **End to end on a fixture.** The clean tree is `ELIGIBLE`. A seeded `AppNeverCallsCore` is `REJECTED`, with the counterexample `main@app/src/main.rs` in the evaluation. Dropping that invariant from the ADL against the pin is `REJECTED`.

Seven mutants were each caught by a test:

- a HARD failure without a violation;
- pinned and current not compared;
- a HARD unknown not required;
- every dependency HARD;
- an identity blind to statements;
- a verdict not re-derived;
- a renamed report field (caught by schema conformance).

## What stays open

`DEBT-INTEGRITY_ENVELOPE` rises to `M2_PARTIAL`. The remaining work:

- **Two classes only.** Just `DEPENDENCY_DIRECTION` and `AUTHORITY_BOUNDARY` are lowered. State, persistence, concurrency and the other classes need declaration forms first (`NA-ADL-STATE-AND-PACKAGES`, `NA-INTEGRITY-CLASSES`).
- **Full recompute.** The report does not yet use the G129 incremental impact closure.
- **Seal.** No seal binds the envelope or the report (`NA-SEAL-GATE`).
