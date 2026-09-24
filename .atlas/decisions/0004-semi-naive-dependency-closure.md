---
id: atlas.decision.0004.semi-naive-dependency-closure
type: decision
status: accepted
canonical: true
---
# ADR 0004 — Semi-naive fixed-point dependency closure

## Context

`.atlas/roadmap/SELF-BUILDING-R4-R8.md` names "recursive/fixed-point semantic derivation" as one of
R5's six required capabilities; no native code for it existed anywhere in Atlas. ADR 0003 left the
datafrog donor `ABSORB_LATER` on one recorded blocking question: is there any genuinely cyclic or
multi-round Atlas derivation, or is everything acyclic by construction "the way
`DependencyClosureReport` currently is" (by Cargo.lock's own DAG guarantee)?

This generation's parallel investigation wave answered it, and the answer came from the coordinator's
join rather than from any single lane (two lanes independently concluded "no such derivation
exists"; both had checked only existing code and the R5 roadmap text, not the dependency contracts):

- `.atlas/contracts/DEPENDENCY-CENSUS.md` already requires one: census "MUST account for the resolved
  dependency graph transitively for each admitted build/runtime context" (§ intro), every edge
  active in an admitted context must be accounted (§ resolution context), and closure requires that
  "no new dependency nodes appear on another closure iteration" (§ closure). Atlas emitted only a
  flat lockfile edge list; transitive edges carry `role: None`, so which packages are reachable, and
  through which root dependency table, was never computed at all.
- The blocking question's premise is false on real input. A Tarjan-SCC pass at exact (name, version)
  granularity over every Cargo.lock in this repository found genuine cycles in 5 of the 15 real donor
  workspaces Atlas already censuses: crubit and rust-analyzer (a member listing itself in its own
  `[dev-dependencies]`), wasm-tools, wasmtime (`wasmtime` dev-depends on `wasmtime-test-util`, which
  depends on `wasmtime`) and zed. (A first, name-only pass also reported cycles in buck2/miri/rust;
  those were artifacts of collapsing distinct versions and were discarded.)

Transitive closure over a cyclic relation is exactly the workload datafrog's semi-naive evaluation
exists for (`rust-lang/datafrog@edd7cfc`: `src/variable.rs` `VariableTrait::changed`,
`src/iteration.rs` `Iteration::changed`).

## Decision

1. **`core::closure`** — `DeltaRelation` and `semi_naive_fixed_point`: datafrog's `Variable`/`Iteration`
   mechanism compressed to its invariant. A relation holds disjoint `stable` / `recent` / `to_add`
   tuple sets; each round folds `recent` into `stable`, promotes only not-yet-stable queued tuples to
   `recent`, and feeds only that delta to the derivation step. Every run yields a
   `FixedPointRecord { iterations, converged, delta_remaining }`. A defensive round bound makes
   non-convergence representable (`converged: false`, never silently truncated) even though a monotone
   loop over a finite domain always converges.
2. **`adapter::dependency::cargo::dependency_reachability`**, called by `census_cargo_workspace` on every
   real census: seeds one `(package, ReachOrigin)` tuple per role a workspace member declares for an
   edge (all roles, not just the first match; `Unattributed` when none was evidenced), and propagates
   origins along resolved edges keyed by exact lockfile package (name + version + source) -- except a
   workspace member's dev-only edges, which Cargo never activates for a package used as a dependency.
3. **`DependencyClosureReport.reachability: Option<DependencyReachability>`** (schema v4):
   origin-labelled reached instances, unreached non-member instances, the dependency fixed-point
   record, an explicit `approximation` (`OriginUpperBound`, or `IncompleteGraph` when the closure is
   `Partial`) and `epistemic_status: Inferred`. `None` for `NotApplicable`/`Blocked` -- never a
   fabricated zero-round record. A non-converged fixed point keeps the report out of `Closed`.
4. **Narrow blocker fix, same change**: manifest roles and evidence paths are now attributed only to
   source-less lockfile entries. Keying them by name alone had handed a member's manifest roles to any
   registry crate sharing its name (real in this corpus: rust-analyzer, zed), which would have seeded
   false origins.

### Why `core::closure`, not the plan's named `runtime/closure`

`DONOR-ABSORPTION-PLAN.toml` names `runtime/closure` as a W3 target directory, but also admits `core`
as an owner for pure mechanisms. The driver has no I/O, no state and only `serde` as a dependency --
the same shape as `core::graph` and `core::language::adl::evaluate_constraints` -- and its first
caller lives in `adapter`, which may depend on `core` but not on `runtime`.

### Why a generic driver rather than a specialised breadth-first loop

The adversarial lane correctly observed that a layered BFS is semi-naive evaluation for a single linear
rule. The generic form is kept because it is the same size as a specialised loop, because it is the
direct, testable encoding of the absorbed invariant (dedup against `stable`, delta-only expansion,
round accounting), and because the contracts name a second consumer: the census-wide fixed point of
`.atlas/contracts/CENSUS-COMPLETENESS.md`.

## What this explicitly does not do

- It is not resolution-context modelling. Feature-selected `optional` activation and target-selector
  evaluation remain TARGET work; the result is an explicit upper bound and says so.
- It does not read manifests of source-less path packages that are not declared workspace members, so
  their own dev-dependency edges cannot be excluded (upper bound, again explicit).
- It does not absorb datafrog's leapjoin/treefrog multi-way join, antijoin, `Relation` merge/gallop
  storage or `map` helpers: none has an Atlas caller. Recorded as `REFERENCE_ONLY` in the donor entry.
- It does not change `DependencyEdge.role`, which still records only the first matching declaration;
  reachability seeds from every declaration. Widening the edge field is a separate schema change.
- It is not the census-wide reconciliation fixed point of `CENSUS-CERTIFICATE.md`.

## Consequences

- R5 now has native code for two of its six capabilities (ADR 0003: provenance lineage; this ADR:
  recursive/fixed-point derivation).
- Recensus of the 15 real donor workspaces with the improved Atlas: all converge (5-12 rounds) and
  stay `Closed`, and the new capability immediately surfaced facts the previous Atlas could not observe
  -- unreached lockfile packages in rust-analyzer (5), rust (1) and wasm-tools (39). The first two
  expose a real pre-existing defect: Cargo.lock's `"name version (source)"` disambiguation suffix is
  not parsed, so an edge to a registry crate sharing a name and version with an in-tree package resolves
  to the in-tree one. Recorded as the next generation's first candidate, not fixed here.
- Supersedes ADR 0003's statement that datafrog "remains ABSORB_LATER".
- datafrog's donor checkout becomes an extinction candidate for the absorbed mechanism; its remaining
  mechanisms are dispositioned `REFERENCE_ONLY` before any deletion.
