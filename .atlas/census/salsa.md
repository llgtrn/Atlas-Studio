---
id: donor-census-salsa
type: reference
status: active
canonical: true
---
# Donor Census: Salsa

## Source

- Remote: https://github.com/salsa-rs/salsa.git
- Commit: 5d8eaf1fa372a73d44f9b717407de41f19ef59d7
- Branch: master
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: COARSE_CENSUSED. Two core mechanisms are now understood and evidenced from real source:

- the incremental-verification algorithm (global monotonic revision counter, per-input-class
  durability, per-memo verified_at/changed_at split, shallow-then-deep two-tier dependency check)
  -- see `.atlas/genome/technology/salsa-durability-gated-incremental-verification.md`, read from
  `src/revision.rs`, `src/durability.rs`, `src/zalsa_local.rs`, and
  `src/function/{memo.rs,maybe_changed_after.rs}`.
- the fixed-point cycle-iteration mechanism (cycle-head-managed Kleene iteration, provisional
  values, dual value+metadata convergence, lazy finality propagation, a hard 200-iteration safety
  bound) -- confirmed as the concrete mechanism behind R5's own "fixed-point closure" roadmap
  naming -- see `.atlas/genome/technology/salsa-fixpoint-cycle-iteration.md`, read from
  `src/cycle.rs` and `src/function/execute.rs`.

Still needing classification (explicitly not yet censused, not silently assumed absent): tracked
structs (`src/tracked_struct.rs`), accumulators (`src/accumulator.rs`), the macro-generated
ingredient/ID system (`components/salsa-macros`), parallel execution (`src/sync.rs`),
storage/persistence, benchmarks, and test suite structure.

## Native Replacement

runtime incremental query cache -- R5's own required capabilities ("dependency-aware query
invalidation", "revision-scoped cached derivation", "recursive/fixed-point semantic derivation",
"incremental recensus of changed source and affected dependents", "deterministic propagation",
"evidence/provenance lineage through derived facts") map directly onto the mechanism now recorded
in the genome document above, though no absorption/implementation decision has been made yet.

