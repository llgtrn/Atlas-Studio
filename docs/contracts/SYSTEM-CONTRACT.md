---
id: atlas.contract.system
type: contract
status: active
canonical: true
---
# Atlas System Contract

## Hard Invariants

NO_COMPLETE_DOCS_NO_CODING. ONE_REPOSITORY_PER_CODING_SESSION. EXACT_BASE_SHA_REQUIRED. ATLAS_HAS_NO_MERGE_AUTHORITY. GENERATED_STATE_IS_REBUILDABLE. CHRONICA_RUNTIME_MUST_NOT_IMPORT_ATLAS.

## Interfaces

Stable interface is atlas.systemizer.cli.v1. Additive commands may extend v1 without breaking existing commands; incompatible semantics require a new explicit CLI version.

## State and Durability

Owning Git repositories hold durable source/docs/evidence. Atlas indexes and graph projections are derived and must be reconstructible from repository inputs.

## Authority

Atlas may produce analysis and bounded plans. It cannot approve its own documentation gap, bypass coding admission, mutate multiple repositories in one coding session, or merge canonical branches.

## Evidence

Coding admission evidence includes repo gate, docs gate, docs standard, exact selected repo SHA, scope and verification plan.

## Recovery

On failed analysis rebuild derived state. On failed implementation preserve/discard branch without altering canonical main. On stale base SHA refresh fleet and regenerate the work plan.

## Verification

Machine tests must cover documentation hard gate, exact SHA, registered fleet connection, one-repository planning, repository manifests, runtime isolation and CLI contract compatibility.
