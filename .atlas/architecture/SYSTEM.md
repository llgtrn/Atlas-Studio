---
id: atlas.architecture.system
type: architecture
status: canonical
canonical: true
---
# Atlas Studio System Architecture

## Responsibilities

- `core/` owns global identity, scope, universal graph primitives, state/event/temporal semantics, bindings, evidence/provenance, claim status, constraints/invariants, Genome semantics, ATLAS/ATLASX contracts and compiler IR types.
- `runtime/` owns secure admission, exhaustive census accounting, reconciliation/fixed-point closure, corpus/design construction, invention, ATLAS publication, AtlasX materialization, compiler passes, optimization, verification, profiling, recensus and incremental invalidation.
- `adapter/` owns Git/filesystem/parsers/compiler metadata/storage/provider/research/benchmark/OS/toolchain/hardware mechanics. Adapters never become semantic authority.
- `apps/ui/` owns TypeScript/TSX projections only. UI state is not engineering truth.
- `.atlas/` owns authored control knowledge, Genome source, architecture, provenance/license references and contracts.
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
                  world/graph optimization
                              ↓
           HIR → MIR → LIR → Machine IR
                              ↓
       codegen → LTO → link → post-link optimization
                              ↓
          binary / library / WASM / UI product
                              ↓
               workload profile / benchmark
                              ↓
                     evidence + recensus
                              ↺
```

## Universal graph substrate

Every repo, donor, paper claim, source function, semantic atom, design candidate, compiler unit and generated product is represented by the same semantic spine:

```text
Identity / Scope / Node / Edge / Binding
State / Event / Temporal
Evidence / Provenance / Claim
Constraint / Invariant
Interface / Capability / Effect
Materialization
```

Repositories remain sovereign graph partitions. Cross-repository linkage references stable global identities instead of copying foreign canonical state.

## Census architecture

Every admitted artifact is accounted for. Every discovered function/method is represented. Adaptive census controls semantic depth, not whether a function exists in Atlas.

Independent extractors may contribute syntax, symbols, compiler facts, build graphs, runtime traces and tests. Conflict is preserved and triggers deeper census. High-level claims must decompose to evidence; low-level facts must aggregate upward.

Only Genome-eligible CLOSED/SEALED census roots may feed production materialization.

## ATLAS architecture

A logical `*.atlas` is dense binary engineering/design knowledge. It may be physically one file or many immutable content-addressed shards across storage locations/repositories. The logical root manifest commits to all required shard hashes.

Size is not a semantic constraint. Atlas may be larger than source because it preserves function semantics, graph relations, evidence, history, donor knowledge, research, alternatives, conflicts and selected design.

## ATLASX architecture

`*.atlasx/` is the deterministic selected executable representation derived from one pinned Atlas root, Genome and compiler version. It is repo-shaped for bounded addressing but is not documentation.

## Compiler architecture

Compilation optimizes the whole selected semantic world before machine-local code generation. It may resolve bindings, devirtualize, specialize policies, choose state placement, memory regions, data layout and concurrency topology, provided graph/authority/temporal/evidence invariants remain valid.

Bootstrap uses Rust/TypeScript/C boundaries. Later phases lower through Atlas HIR/MIR/LIR/Machine IR to external and native backends.

Production compilation includes explicit DeploymentProfile, HardwareProfile and WorkloadProfile, plus LTO/whole-program optimization, link/post-link optimization, profile-guided recompilation and evidence-backed auto-tuning where justified.

Core performs no filesystem, network, subprocess, provider or UI work. Ingestion is not execution. Mutation and integration remain explicit authorized repository operations.
