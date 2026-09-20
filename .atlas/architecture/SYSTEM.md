---
id: atlas.architecture.system
type: architecture
status: canonical
canonical: true
---
# Atlas System Architecture

## System Model

Atlas manages an engineering graph over repositories, declared ADL source, docs, observed source, donors, technology primitives, work plans, verification and proof evidence. A target repository need not contain Atlas-specific metadata; Atlas can compile its understanding from the checkout plus Atlas-owned knowledge.

## Responsibilities

Core owns product-neutral facts, nodes, edges, bindings, evidence, manifests, ADL syntax/AST/IR, declared-vs-observed comparison and graph construction. Adapter owns filesystem, repository manifest, ADL source discovery, documentation and source observation mechanics. Runtime owns repository compilation, CLI orchestration, coding admission, ADL checks and evidence emission. UI code under `apps/ui` may visualize graph state, but Rust remains authoritative for backend semantics.

## Boundaries

Atlas may observe many repositories simultaneously. Each invention session has one canonical target lineage. Parallel workers operate only in exact-SHA mirrors or isolated branches and must reconverge. Target repositories do not need Atlas runtime libraries, services or metadata files.

## Runtime Ownership

The stable external surface is the atlas-systemizer CLI/API. Atlas implementation lives in Atlas-Systemizer under `core/`, `runtime/` and `adapter/`; UI code lives under `apps/ui/`. Systems built or inspected by Atlas continue to run when Atlas is absent.

## Data and Effect Flow

Target repo -> manifest -> declared ADL -> repository observation -> facts -> declared graph + observed graph -> documentation graph -> constraint evaluation -> deltas/problems -> work plan -> coding -> verification -> evidence -> updated repository state -> refreshed graph.

## Source References

- `core/src/lib.rs`
- `.atlas/declared/system.atlas`
- `adapter/src/lib.rs`
- `runtime/src/lib.rs`
- `runtime/src/main.rs`

## Failure and Recovery

Missing docs blocks coding. Stale target SHA invalidates mirror/work plans. Mirror divergence requires rebase/regeneration. Failed translation remains a candidate. Failed proof blocks reconvergence. Derived graphs can be rebuilt.

## Evidence

North Star, blueprint, contracts, exact target SHA, donor provenance, technology graph, target design graph, mirror lineage, translation lineage, CI shards, differential tests and final integration SHA form the engineering evidence chain.

## Verification

Machine tests must prove docs admission, graph-before-code, exact mirror lineage, one canonical target, donor runtime-dependency prohibition, candidate-only translation, distributed CI aggregation and reconvergence safety.
