---
id: donor-census-rust-analyzer
type: reference
status: active
canonical: true
---
# Donor Census: rust-analyzer

## Source

- Remote: https://github.com/rust-lang/rust-analyzer.git
- Commit: aaddfb73fd95f2c0bf001b474dca91ae28bcce3a
- Branch: master
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: DEEP_CENSUSED.

Deep census evidence: `.atlas/census/donors/source-intelligence-lane-2026-09-20.md`.

Observed mechanisms: VFS file identity, source-root partitioning, change packs, salsa-backed source database, crate graph ingestion, Rust HIR/semantic API and diagnostics over `RootDatabase`.

Decision: TARGET_MAPPED. rust-analyzer principles feed `adapter/source/rust`, `core/identity`, `runtime/ingest` and future incremental source analysis. Runtime dependency remains `REFERENCE_ONLY`; Atlas must not import rust-analyzer as its backend.

## Native Replacement

adapter/parser rust semantic boundary


## G75 — name resolution absorbed; terminal ABSORBED; source extinct

The recorded blocking question was whether a bounded slice of hir/hir-def could move CALL observations off UNRESOLVED without rustc. The answer is **yes, for path calls**. `hir-def`'s `nameres` covers:
- module files;
- item scopes (types and values);
- the `use` import fixed point with glob shadowing and visibility;
- block DefMaps;
- the extern prelude.

`hir-ty`'s inherent-impl index covers `Type::f`. This is absorbed natively as `adapter::semantic::rust::resolve`, a second CALL engine (`atlas.resolution.rust-paths`) that observes the syntactic extractor's claims (ADR 0031).

- **Measured:** 3,731 of 6,165 path calls resolved. All 3,731 are confirmed by the pinned rust-analyzer 1.90.0 SCIP index. Recall is 95.1%, and every miss is a deliberate refusal.
- **Reference only:** method calls (11,704) need `hir-ty` inference, which stays REFERENCE_ONLY.
- **Extinct:** the checkout (2,345 files) is physically deleted. The DC1 real-donor test keeps 11 of 15 Cargo donors, and the `(source)`-suffix shape is covered by a synthetic fixture.

Evidence: `../evidence/campaign/04-rust-analyzer.json`.
