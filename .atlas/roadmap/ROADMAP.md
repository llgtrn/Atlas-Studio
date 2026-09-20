---
id: atlas.roadmap
type: blueprint
status: active
canonical: true
---
# Atlas Studio Roadmap

## Phase 0 — strict census + logical ATLAS

- Genome loader/validator/hash in Rust;
- universal graph/binding/state/event/temporal/evidence primitives;
- exhaustive inventory and Rust self-census;
- every discovered function represented;
- CFG/call/data/state/effect extraction and explicit dynamic/unknown records;
- reconciliation, adversarial gaps, fixed point and CensusCertificate;
- typed binary `*.atlas`, semantic dedup/compression, logical root manifests and content-addressed shards;
- federated cross-repo references without competing truth.

## Organism Foundation — after universal semantics are real

Before model training ambitions:

- implement Organism Genome v1 types;
- persistent OrganismId / GenomeId / SpeciesId / Generation / RuntimeInstance lineage;
- organ/circuit/trait semantics;
- body/environment capability model;
- durable memory lineage;
- learning/adaptation candidate model;
- authority constraints and self-modification admission;
- model/provider/checkpoint bindings supporting API, self-hosted, hybrid and none;
- homeostasis and metabolism;
- lifecycle/birth/suspend/retire/recovery;
- species templates as reusable constraints, not product silos.

## Phase 1 — deterministic AtlasX + delegated compiler

- general `*.atlas → *.atlasx/`;
- digital-organism AtlasX profile;
- Rust backend / TypeScript frontend / bounded C boundary lowering;
- phenotype repository/product generation;
- compile/test/benchmark/recensus and product lineage.

## Phase 2 — HIR/MIR semantic optimizer

- graph/binding/organ/circuit specialization;
- devirtualization and policy partial evaluation;
- ownership/lifetime/region/escape/alias analysis;
- state/memory/model placement;
- data layout/locality;
- concurrency scheduling;
- classic scalar/control/loop optimization;
- semantics barriers for authority/evidence/temporal/lifecycle/model admission.

## Phase 3 — external native backends

- LIR;
- LLVM/Cranelift/WASM/accelerator paths;
- stable Atlas ABI/object layout;
- vectorization/SIMD/target specialization;
- GPU/model runtime integration where selected.

## Phase 4 — Atlas native backend

- Machine IR, instruction selection, register allocation, scheduling, object emission;
- x86-64, ARM64 and justified targets;
- native linker/object tooling only after sequencing supports it.

## Organism learning/evolution lane

After organism substrate and authority are mature:

```text
Experience
→ Durable Observation/Event
→ Memory
→ Dataset Compiler
→ Training/Adaptation
→ Candidate Model/Weights/Rule
→ Evaluation
→ Simulation/Regression/Safety
→ Genome Compatibility
→ Admission
→ Activation
→ Evidence/Rollback
```

No learning artifact activates directly.

## UI

World Canvas remains a projection over the same graph. It may visualize organism organs/circuits/memory/model lineage but does not own organism truth.


## Rust parity/surpass maturity lane

The compiler destination is not merely native codegen. Follow `../blueprints/RUST-PARITY-AND-SURPASS-ROADMAP.md` through ownership/borrow safety, diagnostics, codegen correctness, LLVM integration, ABI/platform support, debug info, native backend parity, whole-world optimization, PGO/auto-tuning and sustained compiler maturity.

Atlas may only claim a scoped Rust-surpass result after parity gates are green and the same semantic workload is demonstrably faster/more efficient under locked safety, hardware and workload constraints.
