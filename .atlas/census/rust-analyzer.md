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

