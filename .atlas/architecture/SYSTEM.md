---
id: atlas.architecture.system
type: architecture
status: canonical
canonical: true
---
# Atlas Studio System Architecture

## Responsibilities

- `core/` owns global identity, scope, universal graph primitives, state/event/temporal semantics, bindings, evidence/provenance, claim status, constraints/invariants, Atlas Genome semantics, Organism Genome semantics, ATLAS/ATLASX contracts and compiler IR types.
- `runtime/` owns secure admission, exhaustive census accounting, transitive dependency closure orchestration, reconciliation/fixed-point closure, corpus/design construction, invention, organism-genome synthesis, ATLAS publication, AtlasX materialization, compiler passes, optimization, verification, profiling, recensus and incremental invalidation.
- `adapter/` owns Git/filesystem/parsers/package-manager/build-system/compiler metadata/storage/provider/model API/self-hosted inference/research/benchmark/OS/toolchain/hardware/environment mechanics. Adapters never become semantic authority.
- `apps/studio/` owns TypeScript/TSX projections only. UI state is not engineering truth.
- `.atlas/` owns authored control knowledge, Genome sources/contracts, architecture, provenance/license references and durable evidence.
- `.atlas/artifacts/` owns durable compiled Genome/Atlas/product manifests as implemented.

## End-to-end dataflow

Atlas has separate observation and research ingress paths.

```text
OBSERVATION PATH
pinned root repositories @ exact revisions
build metadata / tests / runtime traces / binary metadata
        ↓
secure admission
        ↓
root inventory ledger
        ↓
transitive dependency resolution / closure
        ↓
expanded federated inventory
        ↓
multi-engine census
        ↓
typed semantic facts
        ↓
reconcile / adversarial gaps / fixed point
        ↓
CensusCertificate
        ↓
Observed World ────────────────┐
                               │
RESEARCH PATH                  │
DeepWiki / papers / RFCs / docs│
        ↓                      │
ResearchClaim                  │
        ↓ corroboration ───────┘
                 ↓
comparison / gap graph
        ↓
invention + validation
        ↓
selected design
        ↓
SEALED logical *.atlas
        ↓
content-addressed shards
        ↓
deterministic *.atlasx/
        ↓
HIR → MIR → LIR → Machine IR
        ↓
codegen / verify / link / product
        ↓
profile evidence + recensus
        ↺
```

A DeepWiki page, paper or model analysis may create a `ResearchClaim`; it never directly creates `ObservedEvidence`. Donor implementation claims require corroboration against the exact pinned donor revision.

### ADL authoring ingress

Atlas Development Language is a second authoring ingress into the same semantic world, not a second universe:

```text
existing implementation                  ADL source
        ↓ census                            ↓ parse/elaborate
typed OBSERVED records                  typed DECLARED records
        └───────────────┬───────────────────┘
                        ↓
               canonical Census
                        ↓
                normalize/reconcile
                        ↓
            selected semantic world
                        ↓
                    *.atlas
```

The current `.atlas/declared/*.adl` parser is ADL0, a bootstrap architectural declaration subset. Full ADL must preserve function/type/control/data/state/effect/resource semantics as typed records. See `../contracts/ATLAS-DEVELOPMENT-LANGUAGE.md` and `../contracts/ADL-TO-ATLAS.md`.

## Native capability ownership

Production architecture converges on four owners:

- `core/`: typed semantic primitives and contracts;
- `runtime/`: inventory, census, query, closure, reconciliation, research correlation, invention, sealing, materialization, compilation, verification and recensus;
- `adapter/`: filesystem/VCS/source/build/binary/storage/security/research/provider/toolchain mechanics;
- `apps/`: Studio, CLI and MCP projections/invocation surfaces.

`tools/` is bootstrap/migration-only. Mature behavior migrates into its native owner and the superseded path is retired. Donor names may describe provenance/research lanes, not permanent runtime ownership.

See `CAPABILITY-ARCHITECTURE.md` and `../blueprints/PHYSICAL-REFOUNDATION.md`.

## Typed semantic kernel

The bootstrap `core/model` bucket is extinct. Current native ownership is split across `graph`, `schema`, `state`, `temporal`, `evidence`, `provenance`, `constraint` and `capability`, with crate-root re-exports preserving stable callers. This is an ownership refoundation, not a claim that every universal state/event/capability primitive is mature.

## Universal graph substrate

Every repo, donor, research claim, source function, semantic atom, design candidate, organism organ/circuit/model/memory lineage, compiler unit and generated product uses the same semantic spine:

```text
Identity / Scope / Node / Edge / Binding
State / Event / Temporal
Evidence / Provenance / Claim
Constraint / Invariant
Interface / Capability / Effect
Materialization
```

Organism-specific semantics extend this graph; they do not create a parallel universe.

## Digital Organism substrate

Organism targets add typed primitives for persistent organism identity, Organism Genome, species/traits, organs, circuits, body, brain/model bindings, world model, durable memory lineage, learning/adaptation, homeostasis, metabolism and lifecycle.

External LLM/model providers and self-hosted models are interchangeable capability providers only where the Organism Genome declares compatible semantics. Provider sessions never own organism identity or durable memory.

## Census architecture

Census starts from a root corpus but expands through the transitive dependency graph for every admitted resolution context. Source-backed dependencies become federated census scopes; binary/toolchain/system/service dependencies become explicit terminal boundaries. The same mechanism is used for Atlas self-census and as a general Atlas feature. See `../contracts/DEPENDENCY-CENSUS.md`.

Every admitted artifact is accounted for. Every discovered function/method is represented. Adaptive census controls semantic depth, not existence. Independent extractors may disagree; conflict triggers deeper census.

Only Genome-eligible CLOSED/SEALED census roots may feed production materialization.

## ATLAS / ATLASX

A logical `*.atlas` is dense binary engineering/design knowledge and may span immutable content-addressed shards. It is semantic compression rather than a source archive: source text may be embedded as FAT evidence, but typed semantic records remain authoritative. Physical wire-v1 structure is governed by `../contracts/ATLAS-BINARY-WIRE-FORMAT.md`.

`*.atlasx/` is deterministic selected executable representation. For `target_kind = digital_organism`, AtlasX includes an organism profile/genome plus the executable organ/circuit/body/brain/memory/lifecycle semantics required to create a phenotype.

## Compiler architecture

Compilation optimizes the selected semantic world before machine-local code generation. General products and organism phenotypes share the same compiler substrate.

For organism targets the compiler may additionally specialize model-provider bindings, model placement, batching, memory consolidation paths, organ placement and metabolic resource policy while preserving Organism Genome, authority, learning-admission and lifecycle constraints.

Bootstrap uses Rust/TypeScript/C boundaries. Later phases lower through Atlas HIR/MIR/LIR/Machine IR.

Core performs no filesystem, network, subprocess, provider or UI work. Ingestion is not execution. Model inference is not truth. Training completion is not model activation.
