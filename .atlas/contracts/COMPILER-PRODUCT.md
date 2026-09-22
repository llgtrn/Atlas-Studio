---
id: atlas.contract.compiler-product
type: contract
status: active
canonical: true
---
# Compiler Product Contract

Atlas compilation transforms one validated canonical AtlasX root into a verified physical product while preserving graph, temporal, evidence, authority and target-specific invariants.

The normative compiler-stage semantics and lowering boundaries are defined by `COMPILER-IR-PIPELINE.md`; the minimum v1 logical record/op schemas are defined by `COMPILER-IR-SCHEMAS.md`. This product contract defines the end-to-end product obligation; it does not permit implementation-specific reinterpretation of HIR/MIR/LIR/Machine IR.

## General product pipeline

~~~text
validated *.atlasx root
 + DeploymentProfile
 + HardwareProfile
 + WorkloadProfile
 + Target / ABI profile
       ↓
HIR
       ↓
MIR
       ↓
LIR
       ↓
Machine IR
       ↓
codegen / object emission
       ↓
LTO → link → post-link
       ↓
physical product
       ↓
runtime profile / PGO / auto-tuning
       ↺
Atlas evidence / candidate blueprint revision
~~~

Every arrow above is governed by an explicit lowering/equivalence contract. A compiler may optimize internally, but it may not silently move semantic responsibilities between stages.

## Digital Organism product pipeline

```text
Digital-Organism AtlasX
 + Organism Genome
 + Environment Profile
 + admitted Model/Provider bindings
       ↓
organ/circuit/world optimization
       ↓
runtime + body + memory + learning substrate
       ↓
brain/model integration
       ↓
compiled phenotype repository/artifacts
       ↓
birth / instantiate
       ↓
observe / remember / act
       ↓
candidate learning/adaptation
       ↓
evaluation / simulation / regression
       ↓
authority/policy admission
       ↓
activation / rollback evidence
```

Compilation may emit Rust runtime, TypeScript UI, GPU kernels, model graphs/training code, weights/checkpoint manifests, storage layout and environment adapters.

Training weights is not required for compilation completion: external model APIs, preexisting admitted checkpoints, deterministic cognition and hybrid systems are valid phenotype bindings.

## Semantic optimization

Atlas may resolve bindings, devirtualize calls, remove unnecessary abstraction/serialization boundaries, specialize policies, fuse capabilities/organs, place state/memory/models, choose data layouts/regions, partition concurrency and specialize deployment topology.

For organism targets it may also optimize model placement, batching/context budgets, local-vs-provider routing, memory consolidation paths and metabolic resource use.

## Safety and authority barriers

Optimization may not weaken required authority, safety, temporal, evidence, transaction, recovery, lifecycle, model-admission or externally observable binding semantics.

An organism cannot gain authority merely because a model proposes an action or learning produces new weights.

## Product evidence

A production artifact records lineage to Atlas root, AtlasX root, Atlas Genome, Organism Genome where applicable, compiler version, target profiles, model/provider/checkpoint bindings, optimization configuration, tests, benchmarks and artifact hashes.

## Blueprint evolution

Compiler/product architecture is allowed to improve from census evidence.

If donor/dependency census, proof, differential testing or benchmark evidence reveals a materially better compiler mechanism, IR representation, backend boundary or optimization strategy, Atlas MAY revise the compiler blueprint through `BLUEPRINT-EVOLUTION.md`.

A compiler optimization that changes only physical implementation while preserving the selected design may remain an optimization.

A discovery that changes stage semantics, identity, ABI, selected architecture or required invariants is a blueprint/contract revision and MUST NOT be hidden inside an optimization pass.

