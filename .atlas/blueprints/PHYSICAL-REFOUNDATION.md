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

The canonical R4→R8 self-building execution map is `../roadmap/SELF-BUILDING-R4-R8.md`.

## Current bootstrap reality

At the R4.4 baseline:

~~~text
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
  semantic/
  state/
  temporal/
  lib.rs

runtime/src/
  census/
  inventory/
  normalize/
  lib.rs

adapter/src/
  semantic/
  source/
  vcs/
  lib.rs

apps/
  cli/
  studio/

tools/
  historical/bootstrap subsystems
~~~

Dependency-census runtime ownership is specified but `runtime/src/dependency/` and `adapter/src/dependency/` are not yet materialized at this baseline.

## Refoundation waves

### R0 — Preserve canonical main

Each work run starts from exact `main`, adds one real primitive, proves invariants, merges, retires the temporary bridge, then continues from the new canonical state.

### R1 — Inventory ledger

**Status: materialized on the native path.** Typed artifact identity and disposition now precede deeper parsing.

~~~text
admitted total
=
parsed
+ binary-described
+ generated
+ explicit-policy-ignore
+ unsupported
+ unknown
+ externalized
~~~

No extension, size, parser failure or file type may silently remove an artifact from accounting.

### R2 — Typed semantic kernel

**Status: native ownership split materialized; semantic hardening continues.** The generic `core/model` bucket has been removed. Existing production types now live under graph/schema/state/temporal/evidence/provenance/constraint/capability/semantic owners, while crate-root compatibility re-exports keep runtime and adapters stable.

New foundational semantics must not expand the generic string-map model. Later hardening must add typed identity/relation/state/event/capability semantics only when they gain real runtime callers, durable evidence and verification.

### R3 — Structural source boundary

**Status: typed boundary materialized.** `adapter/source/frontend.rs` owns the `SourceFrontend` contract, stable frontend identities and the built-in source registry. Inventory recognition routes through that contract instead of a private extension switch.

Structural recognition is not canonical semantic truth. Deeper parser/compiler/index mechanisms remain evidence-producing implementations behind the semantic extraction boundary.

### R4 — Semantic census and normalization

**Status: R4.4 materialized; CALL and deeper semantic dimensions remain.**

R4 is governed by `../contracts/SEMANTIC-FACTS.md`, `../contracts/SEMANTIC-EXTRACTION.md`, `../contracts/NORMALIZATION.md`, durable decisions 0001/0002, and the R4 acceptance matrix in the extraction contract.

Materialized through R4.4:

- a real Rust semantic extractor exists;
- SYMBOL observations are real;
- TYPE observations are real;
- FUNCTION_IDENTITY observations are real;
- FUNCTION_SIGNATURE observations are real;
- extraction is folded into canonical `build_census`;
- typed observations are preserved losslessly through Census and normalization;
- independent extractor observations are separated by raw observation identity rather than collapsed by semantic claim identity;
- typed obligation/evidence/diagnostic lineage survives in canonical reports;
- typed closure accounting is machine-enforced;
- the engineering graph projects currently-real semantic node families directly from normalized typed semantic records;
- compatibility `SemanticFact` records remain projection/bootstrap compatibility rather than semantic authority;
- FunctionIdentity now carries typed declaration kind, owner/trait context and function generics sufficient to distinguish free functions, inherent/associated methods, trait declarations/defaults and trait-implementation methods at source-evidence level.

Still pending in R4:

- CALL (R4.5);
- CONTROL_FLOW;
- DATA_FLOW;
- STATE;
- EFFECT;
- OWNERSHIP;
- CONCURRENCY;
- PERSISTENCE;
- deeper deterministic normalization/equivalence rules;
- declared-profile R4 closure/reference corpus.

Do not describe currently-real Symbol/Type/FunctionIdentity/FunctionSignature extraction as UNSUPPORTED.

Do not claim the remaining dimensions are implemented merely because their typed kernels/contracts exist.

The prospective R4.5→R4.12 sequence is canonical in `../roadmap/SELF-BUILDING-R4-R8.md`.

### DC1 — Dependency Census Runtime — cross-cutting gate

**Status: contract-locked, not production-materialized at the R4.4 baseline.**

DC1 is not an R4 semantic dimension. It expands census breadth:

~~~text
root corpus
→ admitted resolution contexts
→ direct dependencies
→ transitive dependencies
→ source-backed dependency admission
→ explicit non-source terminals
→ dependency fixed point
→ expanded federated inventory
~~~

DC1 must be production-real before W0 may claim full `COARSE_CENSUSED` status for the donor corpus.

The normative contract is `../contracts/DEPENDENCY-CENSUS.md`.

### R5 — Incremental query and closure

Create dependency-aware revision invalidation plus fixed-point derivation and incremental recensus. Derived facts retain input revision/evidence lineage.

R5 operates on the donor/dependency corpus already being censused; it does not start census for the first time.

### R6 — Reconciliation and certificate

Implement explicit UNKNOWN/DYNAMIC/UNSUPPORTED/CONFLICT handling, cross-scope reconciliation, adversarial gaps, fixed point and CensusCertificate.

R6 must include dependency closure in the proof boundary.

### R7 — Research correlation and selection

Implement ResearchClaim separately from observed facts and connect observed donor mechanisms, Technology Genomes, capability gaps and candidate/selected Atlas-native designs.

Research/model output cannot impersonate observed implementation evidence.

### R8 — Real ATLAS and AtlasX

Implement binary records, content addressing, integrity, transactional publication, sharding, deterministic AtlasX and lineage.

R8 provides the durable canonical carrier needed for census-derived donor knowledge to survive physical source deletion at scale. It does not end the census/recensus loop.

### R9 — Compiler and verification

Only after semantic closure: mature HIR/MIR/LIR/Machine IR, delegated backends, verification, sandboxed execution and profile-guided evidence.

### R10 — Studio convergence

Refactor UI toward `apps/studio`. Studio/editor donors inform interaction/design through the donor absorption process; UI state remains derived.

## Self-building rule

Atlas does not finish R4→R8 and then begin learning from donors.

The required loop throughout refoundation is:

~~~text
current native capability
→ census donors + admitted dependency closure
→ discover/attribute mechanism
→ explicit disposition
→ deep census selected scope
→ Atlas-native implementation
→ verify
→ recensus Atlas + affected donors/dependencies
→ absorb
→ physical extinction when gates close
→ stronger native capability
↺
~~~

Discovery and extinction semantics are canonical in `../roadmap/SELF-BUILDING-R4-R8.md` and `../roadmap/DONOR-ABSORPTION-ROADMAP.md`.

## Evidence-driven refoundation blueprint evolution

This blueprint is authoritative for the current evidence state, but it is intentionally revisable.

If census or implementation evidence demonstrates a better native ownership split, migration sequence, storage/materialization mechanism or compiler boundary, Atlas MAY revise this blueprint under `../contracts/BLUEPRINT-EVOLUTION.md`.

The revision must be explicit, evidence-backed and migration-aware. It must not create a second Atlas or silently violate higher-level contracts.

Refoundation therefore preserves hard invariants while allowing better mechanisms discovered during census to replace weaker earlier design choices.

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

A donor scope may disappear from active Atlas-controlled source only when its revision/license/provenance are durable, relevant dependency closure is accounted, its mechanism/invariants are captured, an Atlas-native replacement exists, runtime/build/test source dependency is zero, verification passes, recensus agrees, required durable knowledge survives deletion, physical source deletion is performed, path absence is verified, and post-delete recensus remains valid.

`ABSORBED`, `EXTINCTION_READY`, `SOURCE_DELETED` and `EXTINCT` are distinct states.

## Completion

Refoundation is complete when there is no silent inventory omission, foundational semantics are typed, research cannot impersonate observation, dependency census is first-class, incremental/fixed-point closure is real runtime behavior, `tools/` is bounded migration compatibility only, and canonical behavior lives under `core/runtime/adapter/apps`.
