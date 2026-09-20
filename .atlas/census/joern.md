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

