---
id: atlas.decision.0003.constraint-derivation-provenance
type: decision
status: accepted
canonical: true
---
# ADR 0003 — Constraint derivation provenance

## Context

`.atlas/roadmap/DONOR-ABSORPTION-PLAN.toml` requires `atlas_native_owner_required_before_absorption`
and `deep_census_required_before_absorption`, and `.atlas/roadmap/SELF-BUILDING-R4-R8.md`'s R5 wave
names "evidence/provenance lineage through derived facts" as one of six required capabilities. Five
W3-lane donors (`salsa`, `datafrog`, `differential-dataflow`, `souffle`, `buck2`) were coarse-censused
and cross-synthesized (`.atlas/genome/technology/r5-incremental-computation-design-synthesis.md`), but
every one remained `decision_status = "PENDING"` and no native R5 capability existed anywhere in
`runtime/src` (`runtime/query`, `runtime/closure` — R5's own named target directories — did not exist).
Census, genome documentation and verification-hardening work continued indefinitely without ever
producing an absorbed capability.

`core::language::adl::evaluate_constraints` is Atlas's only existing derived-fact computation today
(`ConstraintResult`, `EpistemicStatus::Derived`): for each declared `ConstraintCheck`, it consults
declared facts (`DeclaredNode`s of a matching kind, or a `MaterializationDecl` target) and derives a
`passed: bool` verdict plus human-readable diagnostics — but discarded, before this decision, exactly
which facts it consulted. Souffle's provenance mechanism
(`.atlas/genome/technology/souffle-provenance-backed-derivation.md`) — auxiliary `@rule_num`/
`@level_num`-shaped metadata layered onto an *existing* evaluator, changing what is recorded about a
derivation, never what is computed — maps directly onto this gap, is additive to Atlas's existing
`SemanticRecordId`/`SemanticRecordHeader` identity/evidence split (confirmed by the R5 synthesis
record's own read of `core/src/semantic/mod.rs`), and requires none of R5's other four donors'
genuinely unresolved prerequisites (an Atlas-internal revision counter, a lattice-typed time domain, a
sortable surrogate key, or evidence of a workload that actually needs incremental view maintenance).

## Decision

Absorb Souffle's provenance pattern as a bounded, Atlas-native mechanism:

1. `core::language::adl::ConstraintCheckDerivation` — a new type recording, per evaluated
   `ConstraintCheck` (in the same order as the declaring constraint's own `checks`): which rule ran
   (`ConstraintCheckKind::{AttributeEquals, MaterializationExists, ObservedMaterializationDelta}`),
   which `DeclaredNode.name`s it consulted (`AttributeEquals`), and which materialization target it
   consulted (`MaterializationExists`/`ObservedMaterializationDelta`). `ConstraintResult` gains a
   `derivation: Vec<ConstraintCheckDerivation>` field, populated by `evaluate_constraints` itself and
   by the `compare_declared_observed`-delta synthesis path — both real, existing production code, not
   a new parallel evaluator.
2. `core::graph::engineering_graph::add_constraint_derivations` — projects this provenance into the
   engineering graph: one `ConstraintResult` node per result, and a `SUPPORTED_BY` edge to every
   `DeclaredNode` graph node an `AttributeEquals` check consulted (reusing the exact `declared_node_id`
   scheme `SemanticFactKind::DeclaredNode`'s own projection already uses, so both converge on the same
   node). Mirrors `add_dependency_closure`'s established shape and calling convention exactly.
3. Wired into real production entry points: `runtime::systemize`, `runtime::graph`, and
   `runtime::code_analyze` all call it on every real invocation — not merely available, but exercised
   on this repository's own real `.atlas/declared/system.adl` constraints (confirmed via
   `atlas-cli graph --root .`: 4 `ConstraintResult` nodes, 3 `SUPPORTED_BY` edges, on real data).

A failed check's derivation is recorded identically to a passed one's: provenance's value is
explaining *why* a derivation failed, not only justifying successes.

## What this explicitly does not do

This is a bounded slice, not R5's full incremental engine. It does not introduce an Atlas-internal
revision/generation counter, a durability classification, a verified-at/changed-at split, a retraction
primitive, or any evaluation-strategy choice among Salsa/datafrog/differential-dataflow — those remain
`ABSORB_LATER` in `.atlas/references/donor-corpus.toml`, each with an explicit, still-unresolved
blocking question recorded there (not a vague `PENDING`). (datafrog's blocking question was later
resolved and its semi-naive mechanism absorbed: see ADR 0004.) This decision closes exactly the one R5
sub-capability ("evidence/provenance lineage through derived facts") for which sufficient evidence,
a bounded scope, and a credible verification path already existed.

## Consequences

- `ConstraintResult`'s `derivation` field is additive: every existing consumer of `ConstraintResult`
  (5 pre-existing construction sites, all updated) is unaffected in behavior; only new data is added.
- `summarize_system_graph_with_dependencies`'s signature gained a `constraint_results: &[ConstraintResult]`
  parameter; all three real call sites (`systemize`, `code_analyze`, plus `runtime::graph`'s own direct
  `add_constraint_derivations` call) were updated to pass real data, not a placeholder.
- No `Materialization` graph node kind exists yet; `MaterializationExists`/`ObservedMaterializationDelta`
  derivations record their target as a `ConstraintResult` node attribute rather than a fabricated edge
  to a node kind this graph does not project. A future materialization-graph-projection slice could
  upgrade this to a real edge without changing `ConstraintCheckDerivation`'s own shape.
- Souffle's donor checkout becomes a real extinction candidate for this specific absorbed capability:
  its `runtime_dependency_status` was already `REFERENCE_ONLY` (never executed by Atlas at
  build/test/runtime), its genome record durably captures what was learned, and its remaining uncensused
  surface (RAM IR, interpreter, synthesiser) is explicitly recorded as deferred, not silently dropped.
  See `.atlas/census/discoveries/r5-constraint-derivation-provenance.md` for the extinction evaluation.
