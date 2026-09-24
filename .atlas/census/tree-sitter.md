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

## Campaign decision (G65, first-50 #6)

The blocking question was stale on its first clause. ADR 0008's per-artifact `ExtractionCache` (`systemize --cache`) already re-extracts only changed artifacts.

The cheapest falsification was run on the clean tree at 7e4e88e76e (release build, `systemize --cache`):

| run | wall time |
|---|---|
| cold, 89 misses | 10.0 s |
| warm, 89 hits | 5.9–7.1 s |
| warm after one edit to the largest non-test file (4,324 lines), 1 miss | 6.4–7.2 s |

Re-extracting the whole file is noise-level, about 40 ms per artifact on average. So incremental parsing is **falsified** for Atlas.

All four mechanisms are **REFERENCE_ONLY**:
- incremental parsing;
- grammar runtime. The modular-frontend principle is already native in the `SourceFrontend` registry, and third-party grammars would be a runtime dependency that needs a parser sandbox;
- error recovery;
- the query language, which belongs to the ast-grep and semgrep cycles.

The tracked checkout (618 files) was physically deleted. Nothing in build, runtime or tests used it. The commit sha, license and provenance remain recorded.

Evidence: `../evidence/campaign/06-tree-sitter.json`.

