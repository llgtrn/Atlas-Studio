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

Status: COARSE_CENSUSED (advanced from SKELETON). The core incremental-verification algorithm
(global monotonic revision counter, per-input-class durability, per-memo verified_at/changed_at
split, shallow-then-deep two-tier dependency check) is now understood and evidenced from real
source -- see `.atlas/genome/technology/salsa-durability-gated-incremental-verification.md` for the
full deep-census-grade mechanism record, read directly from `src/revision.rs`, `src/durability.rs`,
`src/zalsa_local.rs`, and `src/function/{memo.rs,maybe_changed_after.rs}`.

Still needing classification (explicitly not yet censused, not silently assumed absent): cycle
detection / fixed-point resolution (`src/cycle.rs` -- likely the concrete mechanism R5's own
"fixed-point closure" naming refers to), tracked structs (`src/tracked_struct.rs`), accumulators
(`src/accumulator.rs`), the macro-generated ingredient/ID system (`components/salsa-macros`),
parallel execution (`src/sync.rs`), storage/persistence, benchmarks, and test suite structure.

## Native Replacement

runtime incremental query cache -- R5's own required capabilities ("dependency-aware query
invalidation", "revision-scoped cached derivation", "recursive/fixed-point semantic derivation",
"incremental recensus of changed source and affected dependents", "deterministic propagation",
"evidence/provenance lineage through derived facts") map directly onto the mechanism now recorded
in the genome document above, though no absorption/implementation decision has been made yet.

