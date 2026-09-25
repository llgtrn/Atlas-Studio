---
id: donor-census-c2rust
type: reference
status: active
canonical: true
---
# Donor Census: C2Rust

## Source

- Remote: https://github.com/immunant/c2rust.git
- Commit: 1d37ebbf4d505c947f20947edc264f47e33ed14b
- Branch: master
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: SKELETON. Source is cloned and pinned; implementation inspection still needs to classify modules, algorithms, storage, execution, query, incremental behavior, tests, benchmarks, assumptions, accepted ideas, rejected ideas, and Atlas-native replacement gaps.

## Native Replacement

runtime migration planning and semantic preservation


## G114 — bounded census; terminal REFERENCE_ONLY; source extinct

Surfaces inspected:
- clang AST export into unsafe-Rust transpilation;
- the refactoring tool;
- static pointer-permission analysis (`c2rust-analyze`);
- the dynamic pointer derivation graph (`pdg`).

Atlas has no C semantic frontend and no migration pipeline, so none of these has a consumer. clang is the donor's own boundary. The checkout was a DC1 Cargo test input; it was physically deleted, and the ledger-driven tests pass without it. Evidence: `../evidence/campaign/50-c2rust.json`.
