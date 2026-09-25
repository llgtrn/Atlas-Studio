---
id: donor-census-joern
type: reference
status: active
canonical: true
---
# Donor Census: Joern

## Source

- Remote: https://github.com/joernio/joern.git
- Commit: b381922638ae436fcb6862a90241c8f7ce508894
- Branch: master
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: DEEP_CENSUSED.

Deep census evidence: `.atlas/census/donors/source-intelligence-lane-2026-09-20.md`.

Observed mechanisms: CPG generator wrappers, semantic CPG traversal layer, data-flow engine, query packs, language frontend smoke tests and flatgraph columnar performance migration.

Decision: TARGET_MAPPED. Joern principles feed future `runtime/link` and `runtime/query` code-graph projections. Runtime dependency remains `REFERENCE_ONLY`; Atlas must not execute Joern frontend commands during ingestion.

## Native Replacement

core graph projections

## Campaign decision (G69, first-50 #10)

Both hypotheses were falsified (`../evidence/campaign/10-joern.json`):

- **Coverage.** Atlas's CALL/DATA_FLOW/… UNKNOWN is a completeness statement ("absence of unmodeled forms is not proven"). Joern's `rust2cpg` obtains every Rust semantic fact from an external `rust_ast_gen` binary and falls back to `Defines.Any` where that binary gives no type. So the CPG adds structure, not resolution.
- **Scale.** Atlas's graph is 73,848 nodes and 94,446 edges against the donor reference of about 48M nodes and 431M edges, and a full traversal takes about 0.1 s.

REFERENCE_ONLY; re-evaluate columnar layout at 10^6+ nodes. The checkout (2,200 files, 83 MB) was physically deleted.

