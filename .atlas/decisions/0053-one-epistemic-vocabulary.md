---
id: atlas.decision.0053.one-epistemic-vocabulary
type: decision
status: accepted
canonical: true
---
# ADR 0053 — One epistemic vocabulary, enforced over the workspace (G134, NA-EPISTEMIC-UNIFICATION)

## Context

The G116 ontology lane found parallel carriers, each added per domain and never mapped:

- **Graph.** The engineering graph carried `f32` confidence derived from each fact's status, a second vocabulary that UNIVERSAL-GRAPH forbids.
- **Product.** A hypothesis carried a free-string status.
- **Physical.** A report carried its evidence level as a free string.
- **Contract.** SIMULATED was missing from the canonical list in SEMANTIC-FACTS.
- **Overloaded terms.** VALIDATED had several meanings.

G126–G133 then added more carriers, each typed or stringly typed ad hoc:

- verification states;
- sandbox enforcement;
- verdicts for closures, deltas and traces;
- hypothesis outcomes;
- dependency verdicts.

`DEBT-EPISTEMIC_UNIFICATION` asked that a test reject any status carrier outside a vocabulary map.

## Decision

1. **`core::vocabulary` is the map.**
   - **`CARRIERS`** lists every type allowed to carry a status, verdict, outcome, state or level, each with its role:
     - the vocabulary itself;
     - a verdict;
     - a lifecycle;
     - an enforcement;
     - an evidence ladder with a declared mapping;
     - a record outcome.
   - **`BOUNDARY_TEXT`** lists the fields that hold text at a boundary, with where it is decoded or produced: container records, snapshots, a donor ledger, document front matter.
   - **`NOT_A_STATUS`** lists fields that only share the name (program-state keys, a hash's chunk state).
   - **`admits(struct, field, type)`** decides a field.
2. **The graph carries the fact's `EpistemicStatus`.**
   - The `f32` confidence and its status-to-number table are removed.
   - A documentation binding matched by text is `INFERRED`; a declared materialization matched by path is `DERIVED`.
3. **The domain carriers become typed:**
   - a product hypothesis's status is `EpistemicStatus::Hypothesis`;
   - the physical evidence level is `PhysicalEvidenceLevel`, whose `epistemic()` maps a semantic model to `DERIVED`, a simulation to `SIMULATED`, and loop, bounded-test and field rungs to `OBSERVED`;
   - the report, trace, delta, closure and dependency verdicts and the hypothesis outcome are typed enums (`vocabulary_enum!`) that serialize to exactly the strings they replace, so every model and report stays byte-compatible.
4. **Container boundary.** `EpistemicStatus::from_name` decodes boundary text. The `.atlas` writer refuses, and the reader rejects, a record status outside the vocabulary.
5. **Futures states are decided** (recorded in `SEMANTIC-FACTS.md`):
   - PREDICTED is a SIMULATED record.
   - VALIDATED, FALSIFIED and STILL_HYPOTHESIZED are hypothesis-record outcomes.
   - COUNTERFACTUAL is a record kind with no consumer yet.
   - MEASURED and CALIBRATED wait for telemetry.
   - SIMULATED joins the canonical list.

## Evidence

**The workspace test.** A test parses every workspace source, finds over 500 struct fields, and requires each status-like one to be admitted by the map.

- Its first run found eleven unmapped fields. Seven only shared the name, and four were boundary text; each is now listed with its reason.
- The float confidence and the free-string statuses were already removed.

**The container test.** A container record with a status outside the vocabulary is refused by the writer, and a hostile container carrying one is rejected by the reader.

Six mutants were each caught by a test:

- the map admitting everything;
- a float confidence reintroduced;
- the reader trusting record statuses;
- the writer trusting them;
- any name decoding as a status;
- a simulation rung mapped to `OBSERVED`.

## What stays open

`DEBT-EPISTEMIC_UNIFICATION` rises to `M3_SINGLE_ENGINE_BOUNDED`:

- **Donor ledger.** The donor campaign ledger's three statuses are boundary text rather than typed lifecycles.
- **Visual tolerance.** The visual `f64` tolerance is `DEBT-UNCERTAINTY`'s.
- **COUNTERFACTUAL** has no consumer.
- **Scope of the test.** The test sees struct fields, not function returns or enum payloads.
