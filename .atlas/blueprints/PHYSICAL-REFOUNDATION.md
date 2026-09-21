---
id: atlas.blueprint.physical-refoundation
type: blueprint
status: active
canonical: true
---
# Physical Refoundation Blueprint

## Purpose

The authored architecture is ahead of current physical code. `main` already has the correct top-level owners (`core`, `runtime`, `adapter`, `apps`), but production behavior is still concentrated in bootstrap files and a large `tools/` forest.

Refoundation closes that gap without a big-bang rewrite and without creating a second Atlas.

## Current bootstrap reality

```text
core/src/
  capability/
  census/
  constraint/
  evidence/
  graph/
  identity/
  language/
  provenance/
  schema/
  state/
  temporal/
  lib.rs

runtime/src/
  inventory/
  lib.rs

adapter/src/
  source/
  vcs/
  lib.rs

apps/
  cli/
  studio/

tools/
  historical/bootstrap subsystems
```

R1 now materializes an inventory ledger before the semantic source projection. Unknown extensions, oversized text, binary content, symlinks and explicit policy boundaries receive typed dispositions instead of disappearing. Deeper semantic frontends remain later waves.

## Refoundation waves

### R0 — Preserve canonical main

Each work run starts from exact `main`, adds one real primitive, proves invariants, merges, retires the temporary bridge, then continues from the new canonical state.

### R1 — Inventory ledger

**Status: materialized on the native path.** Typed artifact identity and disposition now precede deeper parsing.

```text
admitted total
=
parsed
+ binary-described
+ generated
+ explicit-policy-ignore
+ unsupported
+ unknown
+ externalized
```

No extension, size, parser failure or file type may silently remove an artifact from accounting.

### R2 — Typed semantic kernel

**Status: native ownership split materialized; semantic hardening continues.** The generic `core/model` bucket has been removed. Existing production types now live under graph/schema/state/temporal/evidence/provenance/constraint/capability owners, while the crate-root compatibility surface keeps runtime and adapters stable.

New foundational semantics must not expand the generic string-map model. Later R2 work must harden typed identity/relation/state/event/capability semantics only when they gain real runtime callers, durable evidence and verification.

### R3 — Structural source boundary

**Status: typed boundary materialized.** `adapter/source/frontend.rs` now owns the `SourceFrontend` contract, stable frontend identities and the built-in source registry. Inventory recognition routes through that contract instead of a private extension switch.

The bootstrap frontends currently establish structural recognition and language ownership only. Incremental syntax, compiler metadata and independent semantic extractors remain implementation mechanisms behind the contract and will deepen in R4 without becoming canonical truth.

### R4 — Semantic normalization

**Status: normalization spine materialized and semantic contracts locked; deep semantic extractors remain.**

R4 is governed by `../contracts/SEMANTIC-FACTS.md`, `../contracts/SEMANTIC-EXTRACTION.md`, `../contracts/NORMALIZATION.md`, durable decisions 0001/0002, and the R4 acceptance matrix in the extraction contract. `runtime/census` now turns inventory plus declared ADL into provenance-linked typed semantic facts, and `runtime/normalize` deterministically normalizes those facts without changing epistemic status or dropping records. `SystemizeReport v9` carries both stages and coding admission fails if either stage loses accounting closure.

The engineering graph now consumes normalized semantic facts rather than independently reinterpreting ADL, so census/normalization and graph projection form one semantic path instead of parallel truths. Unknown or unsupported inventory artifacts also remain visible to the graph as explicit artifact nodes/facts.

Current source frontends prove artifact/language recognition only. Symbol, type, call, CFG/dataflow, build, state and effect extraction remain explicitly `UNSUPPORTED` in census coverage until independent extractors produce evidence; R4 is not complete until those dimensions become real observed facts rather than inferred placeholders.

### R5 — Incremental query and closure

Create dependency-aware revision invalidation plus fixed-point derivation. Derived facts retain input revision/evidence lineage.

### R6 — Reconciliation and certificate

Implement explicit UNKNOWN/DYNAMIC/UNSUPPORTED/CONFLICT, cross-scope reconciliation, adversarial gaps, fixed point and CensusCertificate.

### R7 — Research boundary

Implement ResearchClaim separately from observed facts. DeepWiki/papers/docs can seed claims; implementation claims about donors require exact-pinned-source corroboration.

### R8 — Real ATLAS and AtlasX

Implement binary records, content addressing, integrity, transactional publication, sharding, deterministic AtlasX and lineage.

### R9 — Compiler and verification

Only after semantic closure: mature HIR/MIR/LIR/Machine IR, delegated backends, verification, sandboxed execution and profile-guided evidence.

### R10 — Studio convergence

Refactor UI toward `apps/studio`. Zed/OpenDesign/XYFlow/Cytoscape/ELK inform interaction/design; UI state remains derived.

## Legacy tool extinction map

This is a migration hypothesis; each path is recensused before relocation.

| Existing family | Candidate owner |
| --- | --- |
| `universal-graph` | `core/graph`, `core/schema` |
| `reconcile` | `runtime/reconcile` |
| `reality-atlas`, `system-atlas`, `docs-atlas` | `runtime/inventory/census/query` plus projections |
| `capabilities` | `core/capability`, `runtime/query` |
| `build` | `adapter/build`, `runtime/compile` |
| `parity`, `testing` | `runtime/verify` |
| `benchmark` | `runtime/profile` |
| `canonical-shards`, `subdb` | `runtime/seal`, `adapter/storage` after census |
| `refoundation` | extinct after native owners absorb remaining behavior |

Other tool families need explicit census before an owner is assigned.

## Donor extinction gate

A donor mechanism may disappear from `.atlas/temporary` only when its revision/license/provenance are durable, its mechanism/invariants are captured, an Atlas-native replacement exists, runtime dependency is zero for native technology, verification passes, recensus agrees, and remaining donor-only knowledge is captured or rejected.

## Completion

Refoundation is complete when there is no silent inventory omission, foundational semantics are typed, research cannot impersonate observation, incremental/fixed-point closure is real runtime behavior, `tools/` is bounded migration compatibility only, and canonical behavior lives under `core/runtime/adapter/apps`.
