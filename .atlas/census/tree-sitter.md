---
id: donor-census-tree-sitter
type: reference
status: active
canonical: true
---
# Donor Census: Tree-sitter

## Source

- Remote: https://github.com/tree-sitter/tree-sitter.git
- Commit: 5b951eff4f8b1431e933ed0fe45e48fcd4036a38
- Branch: master
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: DEEP_CENSUSED.

Deep census evidence: `.atlas/census/donors/source-intelligence-lane-2026-09-20.md`.

Observed mechanisms: incremental concrete syntax tree, changed ranges, included ranges, query/capture projection, grammar/runtime language metadata, highlight/tag projections.

Decision: TARGET_MAPPED. Tree-sitter principles feed `adapter/source` syntax extraction and `runtime/ingest` changed-range ingestion. Runtime dependency remains `REFERENCE_ONLY`; Atlas must not make Tree-sitter query captures the canonical semantic graph.

## Native Replacement

adapter/parser SourceParser boundary

