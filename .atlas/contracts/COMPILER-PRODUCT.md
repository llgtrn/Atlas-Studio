---
id: atlas.contract.compiler-product
type: contract
status: active
canonical: true
---
# Compiler Product Contract

Atlas compilation transforms a selected AtlasX executable world into a verified physical product while preserving required graph, temporal, evidence, authority and effect semantics.

## Canonical product pipeline

```text
*.atlasx/
  + DeploymentProfile
  + HardwareProfile
  + WorkloadProfile
        ↓
World/Graph Optimizer
        ↓
HIR
        ↓
Semantic Optimizer
        ↓
MIR
        ↓
Ownership/Alias/Escape/Memory/Concurrency Optimization
        ↓
LIR
        ↓
Target Specialization
        ↓
Machine IR
        ↓
Instruction Selection
        ↓
Register Allocation
        ↓
Instruction Scheduling
        ↓
Object Code
        ↓
LTO / whole-program optimization
        ↓
Link
        ↓
Post-link optimization
        ↓
Product binary / library / WASM / UI bundle
        ↓
Representative workload + runtime profile
        ↓
PGO / auto-tuning evidence
        ↺
*.atlas
```

## Semantic optimization precedes machine optimization

Atlas may use the universal graph to resolve bindings, devirtualize calls, remove unnecessary abstraction/serialization boundaries, specialize policies, fuse capabilities, place state, choose data layouts, choose memory regions, partition concurrency and specialize deployment topology before lowering to machine code.

## Safety barriers

Optimization may not weaken required authority, tenant, safety, temporal, evidence, transaction, recovery or externally observable binding semantics. Such semantics create compiler barriers unless the Genome proves a semantics-preserving transformation.

## Deployment specialization

One AtlasX may compile differently for server, browser, robot, embedded gateway or accelerator. Semantic identity remains stable while physical representation changes.

## Profile-guided loop

Runtime profiles are Evidence with revision, hardware, workload and temporal scope. Profiles may drive inlining, code layout, branch ordering, allocation, data placement, scheduling and specialization. A profile from one target/workload may not be silently generalized to another.

## Auto-tuning

Atlas may compile and benchmark multiple candidate algorithms/layouts/chunk sizes/thresholds and select a Genome-approved objective or Pareto set. Candidate generation is ANALYZE; selected production materialization must retain benchmark evidence.

## Product evidence

A production artifact MUST retain lineage to Atlas root, AtlasX root, Genome hash, compiler version, deployment/hardware/workload profiles, optimization configuration, tests, benchmarks and artifact hash.
