---
id: atlas.architecture.system
type: architecture
status: canonical
canonical: true
---
# Atlas Studio System Architecture

## Responsibilities

- `core/` owns global identity, scope, universal graph primitives, state/event/temporal semantics, bindings, evidence/provenance, claim status, constraints/invariants, Atlas Genome semantics, Organism Genome semantics, ATLAS/ATLASX contracts and compiler IR types.
- `runtime/` owns secure admission, exhaustive census accounting, transitive dependency closure orchestration, reconciliation/fixed-point closure, corpus/design construction, research orchestration, typed decision orchestration, candidate synthesis admission, generated-code census, invention, organism-genome synthesis, SelectedDesign admission, ATLAS publication, AtlasX materialization, compiler passes, optimization, verification, profiling, recensus and incremental invalidation.
- `adapter/` owns Git/filesystem/parsers/package-manager/build-system/compiler metadata/storage/research-provider/decision-provider/synthesis-provider/verification-provider/model API/self-hosted inference/benchmark/OS/toolchain/hardware/environment mechanics. Adapters never become semantic authority.
- `apps/studio/` owns TypeScript/TSX projections only. UI state is not engineering truth.
- `.atlas/` owns authored control knowledge, Genome sources/contracts, architecture, provenance/license references and durable evidence.
- `.atlas/artifacts/` owns durable compiled Genome/Atlas/product manifests as implemented.

## End-to-end dataflow

Atlas has separate observation, authoring and external-intelligence ingress paths that converge before canonical seal.

~~~text
OBSERVATION PATH
pinned roots / build metadata / tests / traces / binaries
        ↓
secure admission → inventory → dependency closure → census
        ↓
typed OBSERVED world
        │
        ├────────────────────────────────────────────┐
        │                                            │
AUTHORING PATH                                       │
Human intent / ADL / Studio                          │
        ↓                                            │
typed DECLARED intent                                │
        │                                            │
        ├───────────────────┐                        │
        │                   │                        │
EXTERNAL INTELLIGENCE       │                        │
constraint envelope         │                        │
        ↓                   │                        │
research provider           │                        │
        ↓                   │                        │
ResearchClaim / candidate mechanisms                 │
        ↓                                            │
typed decision provider → DecisionProposal           │
        ↓                                            │
synthesis provider → CandidateChangeSet              │
        ↓                                            │
generated source/artifacts                           │
        ↓                                            │
UNTRUSTED inventory → census → observed generated semantics
        └──────────────────────┬─────────────────────┘
                               ↓
               reconcile / validation / security
               dependency / license / tests / proof
                               ↓
                    authorized SelectedDesign
                               ↓
                    SEALED logical Atlas
                               ↓
             mechanical provider-independent compaction
                               ↓
                  content-addressed *.atlas
                               ↓
       recursive selected-system/material closure
                               ↓
           closed-world *.atlasx binary capsule
                               ↓
                 HIR → MIR → LIR → Machine IR
                               ↓
                codegen / verify / link / product
                               ↓
                   profile evidence + recensus
                               ↺
~~~

A paper, page, search result or model analysis may create a `ResearchClaim`; it never directly creates `ObservedEvidence`.

A decision-provider score is not selection authority.

A synthesis provider may write real candidate code, but that code becomes implementation evidence only after Atlas inventories/censuses/validates it.

### Human-AI ADL authoring ingress

Atlas Development Language is a Human+AI collaborative authoring ingress into the same semantic world, not a second universe:

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

The current `.atlas/declared/*.adl` parser is ADL0, a bootstrap architectural declaration subset. Full ADL must preserve function/type/control/data/state/effect/resource semantics as typed records.

Conversation, Studio edits and external provider proposals are projections/candidates, not alternate truth systems.

See `../contracts/ARTIFACT-LAYERING.md`, `../contracts/ATLAS-DEVELOPMENT-LANGUAGE.md`, `../contracts/HUMAN-AI-ADL-AUTHORING.md`, `../contracts/ADL-TO-ATLAS.md`, `../contracts/ATLAS-CREATION-PIPELINE.md` and `../contracts/EXTERNAL-PROVIDER-TRUST.md`.

## Trust boundary

Donor/source corpus and external-provider output are untrusted by default.

Instruction-looking files under donor/corpus roots — including CLAUDE/AGENTS files, `.claude/`, `.codex/`, skills, hooks and tool configs — remain census data and MUST NOT become agent/tool authority.

External providers receive least privilege and submit typed candidates/receipts. Canonical policy/admission is Atlas-owned.

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

A logical `*.atlas` is dense binary engineering/design knowledge and may span immutable content-addressed shards. It is semantic compression rather than a source archive: source text may be embedded as FAT evidence, but typed semantic records remain authoritative.

When AI-assisted synthesis is used, the selected implementation has already been generated, censused and validated before logical seal.

Post-seal compaction is mechanical/provider-independent and governed by `../contracts/ATLAS-SEMANTIC-COMPACTION.md`; physical wire-v1 structure is governed by `../contracts/ATLAS-BINARY-WIRE-FORMAT.md`.

`*.atlasx` is the deterministic closed-world binary capsule for one selected system closure. It binds selected Atlas semantics to the transitive dependency/runtime/resource/build/reproduction material required by its declared capsule profile. An unpacked AtlasX directory is a noncanonical tooling projection only. For `target_kind = digital_organism`, AtlasX additionally closes the organism profile/genome plus the selected organ/circuit/body/brain/memory/lifecycle payloads required by that capsule profile.

## Compiler architecture

Compilation optimizes the selected semantic world before machine-local code generation. General products and organism phenotypes share the same compiler substrate.

For organism targets the compiler may additionally specialize model-provider bindings, model placement, batching, memory consolidation paths, organ placement and metabolic resource policy while preserving Organism Genome, authority, learning-admission and lifecycle constraints.

Bootstrap uses Rust/TypeScript/C boundaries. Later phases lower through Atlas HIR/MIR/LIR/Machine IR.

Core performs no filesystem, network, subprocess, provider or UI work. Ingestion is not execution. Model inference is not truth. Training completion is not model activation.
