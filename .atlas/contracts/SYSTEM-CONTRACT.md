---
id: atlas.contract.system
type: contract
status: active
canonical: true
---
# Atlas System Contract

## Hard Invariants

NO_COMPLETE_DOCS_NO_CODING. GRAPH_BEFORE_CODE. ONE_CANONICAL_TARGET_PER_SESSION. MIRRORS_ARE_NON_CANONICAL. MIRROR_STARTS_AT_EXACT_TARGET_SHA. OSS_RUNTIME_DEPENDENCY_FORBIDDEN_FOR_NATIVE_TECHNOLOGY. TRANSLATION_IS_CANDIDATE_ONLY. ATLAS_HAS_NO_MERGE_AUTHORITY. GENERATED_GRAPHS_ARE_REBUILDABLE.

## Interfaces

atlas.systemizer.cli.v1 remains the stable external boundary. Target repositories need no Atlas runtime dependency. Fleet membership, optional worker pools and mirror lineage are centrally managed by Atlas.

## State and Durability

Owning repositories store durable source/docs. Atlas stores derived engineering metadata, graphs, mirror/work allocations and proof indexes. Native technology claims require source/tests/evidence independent from donor runtime availability.

## Authority

Atlas may propose repository creation, mirror creation, OSS allocation, translations and bounded work. It cannot silently create canonical truth, bypass docs/graph gates, or merge target branches.

## Evidence

Required evidence includes documentation admission, exact target SHA, donor provenance, technology/design graph, bounded scope, mirror lineage when used, translation provenance, tests/CI and integration SHA.

## Recovery

Discard/rebuild derived state. Regenerate stale mirrors from target SHA. Re-run translation from source evidence. Reopen technology as transitional if zero-dependency proof fails.

## Verification

Tests cover docs gate, graph-before-code, mirror exactness, single canonical target, OSS allocation uniqueness, candidate-only translation, distributed CI and zero-dependency admission.
