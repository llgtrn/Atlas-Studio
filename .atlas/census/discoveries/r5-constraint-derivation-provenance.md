---
id: atlas.discovery.r5-constraint-derivation-provenance
type: discovery-record
status: absorbed
canonical: true
---
# Discovery record: constraint-derivation provenance (R5 slice)

Fields below follow `.atlas/roadmap/DONOR-ABSORPTION-PLAN.toml`'s `[discovery_record]` schema. This is
the first discovery record this repository has ever filed under that schema — no prior discovery
reached an explicit `ABSORB_NOW`/`ABSORB_LATER`/`REJECT` disposition in `.atlas/references/donor-corpus.toml`
before this one; every W3-lane donor sat at `decision_status = "PENDING"` indefinitely.

## discovery_identity

`r5-constraint-derivation-provenance` — Souffle's provenance-backed-derivation pattern, applied to
Atlas's own `core::language::adl::evaluate_constraints`.

## provider_identity

`souffle` (`souffle-lang/souffle`), per `.atlas/references/donor-corpus.toml`.

## provider_revision_or_version

`a1303be3c0166400dee3d1f36f0d96abe03e6901` (pinned commit, per `.atlas/references/donor-corpus.toml`
and `.atlas/provenance/donors/souffle.json`).

## provider_scope

The provenance sub-mechanism only: `@rule_num`/`@level_num`-shaped auxiliary metadata layered onto an
existing Datalog evaluator, as censused in
`.atlas/genome/technology/souffle-provenance-backed-derivation.md`. Explicitly NOT the RAM IR,
interpreter, synthesiser, semi-naive evaluation core, or any other Souffle subsystem — those remain
uncensused; see `donor_source_remaining_surface` on the `souffle` donor-corpus entry.

## evidence_refs

- `.atlas/genome/technology/souffle-provenance-backed-derivation.md`
- `.atlas/genome/technology/r5-incremental-computation-design-synthesis.md`
- `.atlas/decisions/0003-constraint-derivation-provenance.md`
- `core/src/language/adl/mod.rs` (`ConstraintCheckDerivation`, `ConstraintCheckKind`,
  `evaluate_constraints`, and their test module: 4 new tests, each falsified against the real unfixed
  code before being trusted)
- `core/src/graph/engineering_graph.rs` (`add_constraint_derivations`; 4 new tests, likewise falsified)
- Real production verification: `cargo run -p atlas-cli -- graph --root .` against this repository's
  own `.atlas/declared/system.adl` produced 4 `ConstraintResult` nodes and 3 `SUPPORTED_BY` edges

## root_donor_or_dependency_attribution

Direct donor, admitted per `.atlas/references/donor-corpus.toml`'s existing `souffle` entry (no new
promotion needed — already `ingestion_status = "CLONED"`, coarse-censused, evidenced).

## atlas_capability_target

R5 ("incremental query and fixed-point closure", `.atlas/roadmap/SELF-BUILDING-R4-R8.md`), specifically
its sixth required capability: "evidence/provenance lineage through derived facts". Bounded to the one
derived-fact computation that exists in Atlas today (`ConstraintResult`); does not claim the other five
R5 capabilities (dependency-aware query invalidation, revision-scoped cached derivation, recursive
fixed-point derivation, incremental recensus, deterministic propagation).

## atlas_native_owner

`core` (`core::language::adl`, `core::graph::engineering_graph`) — within
`allowed_native_owners = ["core", "runtime", "adapter", "apps"]`
(`.atlas/roadmap/DONOR-ABSORPTION-PLAN.toml`). `runtime` (`runtime::systemize`, `runtime::graph`,
`runtime::code_analyze`) is the wiring/consumer layer.

## disposition

`ABSORB_NOW`, per `.atlas/roadmap/DONOR-ABSORPTION-ROADMAP.md`'s own criteria, checked explicitly:

- actual provider scope identified: yes (provenance pattern only, see `provider_scope` above)
- sufficient evidence available: yes (existing genome record plus this record's own direct reads of
  `core::language::adl::evaluate_constraints`)
- Atlas-native owner known: yes (`core::language::adl`, `core::graph::engineering_graph`)
- required semantic depth achievable: yes — additive fields on an existing evaluator and an existing
  graph-projection convention (`add_dependency_closure`'s own shape), no new identity model
- verification definable: yes, and done (see `evidence_refs`)
- dependency-removal/extinction path credible: yes — `souffle`'s `runtime_dependency_status` was
  already `REFERENCE_ONLY` before this record; nothing in Atlas's build/test/runtime ever executed or
  imported the souffle checkout

## decision_rationale

Of the five W3-lane donors, souffle's provenance pattern is the only one whose absorption does not
depend on an unresolved Atlas-workload question. `.atlas/genome/technology/
r5-incremental-computation-design-synthesis.md` (read in full before this decision) establishes that
salsa/datafrog/differential-dataflow each require a new supporting concept Atlas does not have yet (an
internal revision counter, a sortable surrogate key, or a lattice-typed time domain, respectively) and
that buck2/Dice's mechanism belongs to AIF1 (a separate, later roadmap item), not R5's own baseline
scope. Souffle's mechanism, by contrast, is additive to Atlas's EXISTING `SemanticRecordId`/
`SemanticRecordHeader` identity/evidence split (confirmed directly against `core/src/semantic/mod.rs`
by the synthesis record) and has a real, bounded substrate to attach to today: `evaluate_constraints`,
Atlas's only existing derived-fact computation. No other roadmap item, gap, or blocker was more directly
actionable with current evidence.

## required_semantic_depth

Shallow-to-moderate: record which rule and which supporting declared facts a derivation consulted;
project that into the graph as real nodes/edges. Does not require resolving how derivation provenance
composes with R6's reconciliation machinery (`CensusCertificate`, cross-scope reconciliation) — named
as an open question in the R5 synthesis record and left open here too.

## prerequisites_or_blockers

None for this bounded slice (confirmed by completing it in one generation with full falsification at
every layer). The BROADER R5 capability has real prerequisites, recorded per-donor in
`.atlas/references/donor-corpus.toml` (`blocking_question`/`evidence_needed`/`cheapest_falsification`
fields on `salsa`, `datafrog`, `differential-dataflow`, `buck2`).

## verification_plan

Executed, not merely planned:

1. `core::language::adl` layer: 4 new tests (`attribute_equals_derivation_names_every_supporting_node_when_the_check_passes`,
   `materialization_exists_derivation_names_its_target`,
   `a_synthesized_observed_materialization_delta_result_carries_its_own_derivation`, plus a derivation
   assertion added to the existing `invariants_are_evaluated_just_like_constraints_not_silently_skipped`
   test for the FAILED-verdict case). Each falsified: the population logic was temporarily broken
   (supporting-node/materialization-target fields forced empty), confirmed every test caught it, then
   restored from a byte-for-byte backup.
2. `core::graph::engineering_graph` layer: 4 new tests
   (`constraint_derivation_projects_a_supported_by_edge_to_the_consulted_declared_node`,
   `a_failed_constraint_derivation_still_projects_its_supporting_node_edge`,
   `a_materialization_exists_derivation_records_its_target_as_a_node_attribute`,
   `constraint_derivations_with_no_results_is_a_no_op`). Falsified the same way: the edge-target
   computation was temporarily broken, confirmed the test caught it, restored.
3. Production wiring: `runtime::systemize`, `runtime::graph`, `runtime::code_analyze` all updated to
   pass real `adl.constraint_results` through; confirmed by full-workspace compilation (no mocks) and
   by running the REAL `atlas-cli graph --root .` against this repository's own real
   `.atlas/declared/system.adl`, observing 4 real `ConstraintResult` nodes and 3 real `SUPPORTED_BY`
   edges in the output.
4. Full workspace suite: 581/581 passing (up from 574), `cargo fmt --all --check` clean, `cargo clippy
   --workspace --all-targets -- -D warnings` clean, `docs audit` gate_ready true, real self-census
   (`systemize --root .`) reports zero blockers.

## dependency_removal_plan

Souffle was never a runtime/build/test dependency (`runtime_dependency_status = "REFERENCE_ONLY"` from
admission). No removal is needed at the dependency level — only the donor SOURCE CHECKOUT under
`.atlas/temporary/donors/souffle` is a removal candidate, addressed under `extinction_implications`.

## extinction_implications

Souffle's donor checkout is eligible for scoped physical extinction (per `.atlas/roadmap/
DONOR-ABSORPTION-PLAN.toml`'s `extinction_gate`, `scope_level_extinction_supported = true`):

- `transitive_dependency_closure_accounted`: souffle has no Atlas-side dependency edges (reference-only,
  never built/linked/imported)
- `technology_genome_durable`: yes (`souffle-provenance-backed-derivation.md`,
  `r5-incremental-computation-design-synthesis.md`, this record)
- `provenance_and_license_durable`: yes (`.atlas/provenance/donors/souffle.json`,
  `.atlas/licenses/souffle/LICENSE`, both independent of the source checkout)
- `atlas_native_replacement_exists`: yes (`ConstraintCheckDerivation`/`add_constraint_derivations`,
  in production use)
- `donor_runtime_build_test_source_dependency_zero`: yes, confirmed by grep (no `souffle` reference in
  `core/Cargo.toml`, `adapter/Cargo.toml`, `runtime/Cargo.toml`, `apps/cli/Cargo.toml`, or any build
  script) and by `runtime_dependency_status` having been `REFERENCE_ONLY` since admission
- `donor_source_files_physically_deleted` / `donor_source_path_absent`: pending execution as the
  immediately following action in this same generation cycle (see the commit that follows this one)
- `no_atlas_controlled_source_archive_cache_snapshot`: to be confirmed at deletion time (no separate
  archive/cache copy of the souffle checkout exists outside `.atlas/temporary/donors/souffle` itself)
- `post_delete_recensus_and_verification_pass`: to be executed and recorded as its own evidence record
  after physical deletion

`remaining_donor_only_knowledge_captured_deferred_externalized_or_rejected`: the RAM IR, interpreter,
and synthesiser subsystems remain uncensused. No current roadmap item (R5 through R8, as currently
planned) requires them. This is recorded as an explicit DEFERRAL, not a silent drop: if a future roadmap
item needs them, souffle would need to be re-cloned/re-admitted for that specific further census — this
record does not claim souffle's full surface is exhausted, only that the one capability selected for
absorption here is fully and durably captured without the source checkout.
