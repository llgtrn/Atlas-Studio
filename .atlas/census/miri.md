---
id: donor-census-miri
type: reference
status: active
canonical: true
---
# Donor Census: Miri

## Source

- Remote: https://github.com/rust-lang/miri.git
- Commit: c63eee41f54d6a4b414af7b22ab717063da0c50a
- Branch: master
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: SKELETON. Source is cloned and pinned; implementation inspection still needs to classify modules, algorithms, storage, execution, query, incremental behavior, tests, benchmarks, assumptions, accepted ideas, rejected ideas, and Atlas-native replacement gaps.

## Native Replacement

runtime verification evidence adapters


## G76 — terminal REFERENCE_ONLY; source extinct

The hypothesis was interpreter-backed UB/effect evidence for EFFECT/STATE. Three measurements settle it:
- **No target for the interpreter:** Atlas has 0 unsafe blocks, fns, impls or extern blocks (a syn enumeration of every workspace file). Safe Rust has no UB, so the interpreter and its UB checkers (Stacked/Tree Borrows, alignment, uninit, data races) have nothing to check.
- **Not runnable:** Miri is nightly-only and absent from the pinned 1.90.0 toolchain.
- **Wrong level for effects:** the shims (`src/shims/unix/foreign_items.rs`) model OS effects at the libc boundary. Atlas's EFFECT gap sits at the std API: 504 call sites reach effectful std modules (fs 352, process 63, env 50, io 20, time 17, thread 2), and none is observed.

That P0 gap is planned natively (G77): external path resolution on top of G75, plus a declared std-path effect table. The checkout (2,593 files) was physically deleted, and the DC1 real-donor test keeps 10 of 15 Cargo donors.

Evidence: `../evidence/campaign/15-miri.json`.
