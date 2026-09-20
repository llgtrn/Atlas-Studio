---
id: atlas.roadmap
type: blueprint
status: active
canonical: true
---
# Atlas Studio Roadmap

## Now — Phase 0: strict census + logical ATLAS

- keep approved donor batch bulk-staged in `.atlas/temporary/`;
- implement Genome loader/validator/hash in Rust;
- implement universal identity/node/edge/binding/state/event/temporal/evidence primitives;
- implement exhaustive repository inventory;
- implement Rust self-census first;
- guarantee every discovered function is represented;
- implement CFG/call/data/state/effect extraction and explicit dynamic/unknown records;
- implement multi-engine reconciliation, adversarial gap queries and fixed-point closure;
- emit CensusCertificate and seal states;
- implement real typed binary `*.atlas`;
- implement semantic interning/dedup, function/content hashes, revision deltas and compression;
- implement logical root manifests, content-addressed shards, lazy fetch and partial materialization;
- implement federated cross-repo references without competing truth.

## Phase 1 — deterministic AtlasX + delegated product compiler

- `*.atlas → *.atlasx/` selected-design materializer;
- stable executable repo-shaped AtlasX;
- Rust backend / TypeScript frontend / bounded C boundary lowering;
- product lineage, compile/test/benchmark/recensus.

## Phase 2 — HIR/MIR semantic optimizer

- graph/binding specialization;
- devirtualization and policy partial evaluation;
- ownership/lifetime/region/escape/alias analysis;
- data layout/locality;
- concurrency/conflict scheduling;
- classic scalar/control/loop optimization;
- semantics-preserving barriers for authority/state/evidence/temporal/safety.

## Phase 3 — external native backends

- LIR;
- LLVM/Cranelift/WASM and accelerator paths;
- stable Atlas ABI/object layout;
- vectorization/SIMD and target specialization;
- Rust reference path retained.

## Phase 4 — Atlas native backend

- Machine IR;
- instruction selection;
- register allocation/spilling;
- stack/calling convention;
- scheduling;
- object emission;
- x86-64, ARM64, then justified targets;
- native linker/object tooling only when sequencing supports it.

## Production optimization lane

Across mature phases:

```text
AtlasX
+ DeploymentProfile
+ HardwareProfile
+ WorkloadProfile
→ world/graph optimize
→ IR optimize
→ target codegen
→ LTO
→ link
→ post-link optimize
→ product
→ runtime profile
→ PGO / auto-tuning
→ evidence back into Atlas
```

## UI

World Canvas remains later than semantic/compiler foundations. It is a projection over the same graph and supports zoom from federation to semantic atom without owning truth.
