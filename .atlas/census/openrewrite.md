---
id: donor-census-openrewrite
type: reference
status: active
canonical: true
---
# Donor Census: OpenRewrite

## Source

- Remote: https://github.com/openrewrite/rewrite.git
- Commit: f7372139f62f62a66e5b78d70bbad50aaea7d8ee
- Branch: main
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: SKELETON. Source is cloned and pinned; implementation inspection still needs to classify modules, algorithms, storage, execution, query, incremental behavior, tests, benchmarks, assumptions, accepted ideas, rejected ideas, and Atlas-native replacement gaps.

## Native Replacement

runtime/refactor recipe and ChangeSet model


## G113 — bounded census; terminal REFERENCE_ONLY; source extinct

Surfaces inspected:
- lossless semantic trees with visitors and cursors;
- recipe composition, with preconditions and options;
- data tables;
- Changeset and Result diffs;
- provenance markers.

Atlas never writes source back, and its construction change model is TARGET, so neither the lossless tree nor the recipe engine has a consumer. The census → declared-change path that exists is `adl derive` (G63), which emits declarations, not source edits. The checkout was physically deleted. Evidence: `../evidence/campaign/49-openrewrite.json`.
