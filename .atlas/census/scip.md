---
id: donor-census-scip
type: reference
status: active
canonical: true
---
# Donor Census: SCIP

## Source

- Remote: https://github.com/sourcegraph/scip.git
- Commit: 4f50fbbd0ed945405cd8a4196af6477ea2a9f8b5
- Branch: main
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: DEEP_CENSUSED.

Deep census evidence: `.atlas/census/donors/source-intelligence-lane-2026-09-20.md`.

Observed mechanisms: streaming `Index`, workspace metadata, canonical document paths, position encoding, symbol grammar, occurrence role bitset, symbol information and symbol relationships.

Decision: TARGET_MAPPED. SCIP principles feed `adapter/exchange/source_index`, `core/identity` symbol identity, and `core/model` symbol occurrence records. Runtime dependency remains `REFERENCE_ONLY`; SCIP protobuf is an exchange adapter, not Atlas canonical storage.

## Native Replacement

adapter/source-index exchange boundary

