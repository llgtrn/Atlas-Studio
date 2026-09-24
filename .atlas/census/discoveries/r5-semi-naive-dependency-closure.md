---
id: atlas.discovery.r5-semi-naive-dependency-closure
type: discovery-record
status: absorbed
canonical: true
---
# Discovery record: semi-naive fixed-point dependency closure (R5 slice)

Fields follow `.atlas/roadmap/DONOR-ABSORPTION-PLAN.toml`'s `[discovery_record]` schema. Second record
filed under it (first: `r5-constraint-derivation-provenance.md`).

## discovery_identity

`r5-semi-naive-dependency-closure` — datafrog's semi-naive, delta-driven fixed-point evaluation,
applied to origin-labelled transitive dependency reachability in `adapter::census_cargo_workspace`.

## provider_identity

`datafrog` (`rust-lang/datafrog`), per `.atlas/references/donor-corpus.toml`.

## provider_revision_or_version

`edd7cfc0f5ad3b93ce5bbe6a17848db9376b13c0` (pinned commit, per `.atlas/references/donor-corpus.toml`
and `.atlas/provenance/donors/datafrog.json`).

## provider_scope

The `Variable` stable/recent/to_add delta discipline (`src/variable.rs` `VariableTrait::changed`) and
the `Iteration` round loop (`src/iteration.rs` `Iteration::changed`). Explicitly NOT leapjoin/treefrog,
antijoin, `Relation` merge/gallop storage or `map` helpers — dispositioned `REFERENCE_ONLY` on the donor
entry (`donor_source_remaining_surface`).

## evidence_refs

- `.atlas/genome/technology/datafrog-semi-naive-fixpoint-evaluation.md`
- `.atlas/genome/technology/r5-incremental-computation-design-synthesis.md` (open question 2, now resolved)
- `.atlas/decisions/0004-semi-naive-dependency-closure.md`
- `.atlas/evidence/verification/r5-semi-naive-dependency-closure-absorption.json` (full lane outputs,
  join, falsification battery, recensus)
- `core/src/closure/mod.rs` (6 tests; a mutation removing deduplication against `stable` is caught by 3)
- `adapter/src/dependency/cargo.rs` (`dependency_reachability` + 9 new tests; five independent
  mutations each caught by the test built for that condition; the real-donor census test now requires
  a converged fixed point for all 15 real workspaces)

## root_donor_or_dependency_attribution

Direct donor, admitted per the existing `datafrog` donor-corpus entry. The cycles that make the
mechanism necessary are attributed to the real donor workspaces that exhibit them (crubit,
rust-analyzer, wasm-tools, wasmtime, zed), not to datafrog.

## atlas_capability_target

R5 "recursive/fixed-point semantic derivation" (`.atlas/roadmap/SELF-BUILDING-R4-R8.md`), bounded to
the dependency-closure requirements of `.atlas/contracts/DEPENDENCY-CENSUS.md` (transitive accounting
per admitted build/runtime mode; "no new dependency nodes appear on another closure iteration").

## atlas_native_owner

`core::closure` (driver), `adapter::dependency::cargo::dependency_reachability` (first caller),
`core::census::dependency::DependencyReachability` (report shape).

## disposition

`ABSORB_NOW` → absorbed (mechanism-level). Remaining datafrog mechanisms: `REFERENCE_ONLY`.

## decision_rationale

The recorded blocking question's premise ("acyclic by Cargo.lock's own DAG guarantee") was falsified
by executable evidence (Tarjan SCC at exact name+version granularity: genuine cycles in 5 of 15 real
workspaces), and a canonical contract already required the recursive derivation. Adversarial and
architecture lanes returned SURVIVES_WITH_CONDITIONS / no layering violation; every condition was
implemented and tested.

## required_semantic_depth

Mechanism-level: the delta invariant and round accounting, not datafrog's storage layout or join
algorithms.

## prerequisites_or_blockers

None outstanding. A narrow blocker was fixed in the same change (member manifest roles no longer
attributed to same-named registry crates).

## verification_plan

Executed: core driver tests + mutation; adapter adversarial tests (dev-only self-loop, dev leakage
through an intermediate member, multi-table roles, version separation, name clash, unattributed edge,
dangling reference, permutation invariance) + five-mutation battery; real-donor recensus of 15
workspaces; full workspace gates; real `atlas-cli systemize` on this repository.

## dependency_removal_plan

datafrog was never a build/runtime/test dependency (zero references in any Cargo.toml, Cargo.lock or
build.rs). Only the materialized checkout remains; see extinction.

## extinction_implications

The absorbed mechanism needs no donor source. The remaining mechanisms are `REFERENCE_ONLY` with their
knowledge retained in the genome record, so the checkout is extinction-eligible once post-deletion
verification passes.
