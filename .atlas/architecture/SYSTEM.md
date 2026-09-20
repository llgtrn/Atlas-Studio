---
id: atlas.architecture.system
type: architecture
status: canonical
canonical: true
---
# Atlas Studio System Architecture

## Responsibilities

- `core/` owns global identity, scope, universal graph primitives, state/event/temporal semantics, bindings, evidence/provenance, claim status, constraints/invariants, Atlas Genome semantics, Organism Genome semantics, ATLAS/ATLASX contracts and compiler IR types.
- `runtime/` owns secure admission, exhaustive census accounting, reconciliation/fixed-point closure, corpus/design construction, invention, organism-genome synthesis, ATLAS publication, AtlasX materialization, compiler passes, optimization, verification, profiling, recensus and incremental invalidation.
- `adapter/` owns Git/filesystem/parsers/compiler metadata/storage/provider/model API/self-hosted inference/research/benchmark/OS/toolchain/hardware/environment mechanics. Adapters never become semantic authority.
- `apps/ui/` owns TypeScript/TSX projections only. UI state is not engineering truth.
- `.atlas/` owns authored control knowledge, Genome sources/contracts, architecture, provenance/license references and durable evidence.
- `.atlas/artifacts/` owns durable compiled Genome/Atlas/product manifests as implemented.

## End-to-end dataflow

```text
admitted repositories / OSS / tests / builds / specs / papers / DeepWiki
                              ↓
                        secure admission
                              ↓
                     exhaustive inventory
                              ↓
                    multi-engine census
                              ↓
             S0→S10 semantic accounting
                              ↓
          reconcile / adversarial gaps / fixed point
                              ↓
                    CensusCertificate
                              ↓
                    observed world graph
                              ↓
              research + invention + selection
                              ↓
                    SEALED logical *.atlas
                              ↓
         one or many content-addressed physical shards
                              ↓
              deterministic *.atlasx/ projection
                              ↓
                   target-kind expansion
                      ├─ general software
                      └─ digital organism
                          ├─ Genome
                          ├─ organs/circuits
                          ├─ body/brain
                          ├─ memory/world
                          ├─ learning
                          ├─ homeostasis/metabolism
                          └─ lifecycle/capabilities
                              ↓
                  world/graph optimization
                              ↓
           HIR → MIR → LIR → Machine IR
                              ↓
       codegen → LTO → link → post-link optimization
                              ↓
                     physical product
                              ↓
      runtime evidence / learning candidates / profiles
                              ↓
          evaluation/admission/recensus
                              ↺
```

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

Every admitted artifact is accounted for. Every discovered function/method is represented. Adaptive census controls semantic depth, not existence. Independent extractors may disagree; conflict triggers deeper census.

Only Genome-eligible CLOSED/SEALED census roots may feed production materialization.

## ATLAS / ATLASX

A logical `*.atlas` is dense binary engineering/design knowledge and may span immutable content-addressed shards.

`*.atlasx/` is deterministic selected executable representation. For `target_kind = digital_organism`, AtlasX includes an organism profile/genome plus the executable organ/circuit/body/brain/memory/lifecycle semantics required to create a phenotype.

## Compiler architecture

Compilation optimizes the selected semantic world before machine-local code generation. General products and organism phenotypes share the same compiler substrate.

For organism targets the compiler may additionally specialize model-provider bindings, model placement, batching, memory consolidation paths, organ placement and metabolic resource policy while preserving Organism Genome, authority, learning-admission and lifecycle constraints.

Bootstrap uses Rust/TypeScript/C boundaries. Later phases lower through Atlas HIR/MIR/LIR/Machine IR.

Core performs no filesystem, network, subprocess, provider or UI work. Ingestion is not execution. Model inference is not truth. Training completion is not model activation.
