---
id: atlas.blueprint.compiler-roadmap
type: blueprint
status: active
canonical: true
---
# Atlas Compiler Roadmap

The compiler roadmap is cumulative. A later phase may replace a physical backend but may not weaken Genome, graph, temporal, evidence or provenance invariants.

## Phase 0 — Genome, Census and ATLAS Foundation

Deliver:

- `atlas.genome.toml` hard-requirement source plus deterministic genome identity/hash;
- global graph IDs and cross-repository binding grammar;
- adaptive census from federation/repository scope to semantic atom;
- code/test/spec/paper/DeepWiki evidence correlation;
- typed fact/inference/hypothesis/conflict/unknown states;
- real `*.atlas` binary reader/writer with versioning, bounds checks, chunking, dictionaries, deduplication, content addressing, compression, random access and transactional publication;
- design synthesis stored in the same graph as observed reality;
- deterministic selection boundary for ATLASX materialization.

Exit gate: Atlas can explain what exists, where it exists, how state/effects/bindings flow, what evidence supports each important claim, and how a repo connects to other repos.

## Phase 1 — Deterministic ATLASX + Delegated Compiler

```text
*.atlas
  ↓ materialize
*.atlasx/
  ↓ typed lowering
Rust / TypeScript / bounded C ABI
  ↓ rustc / TS toolchain / clang
physical artifact
```

Deliver:

- deterministic repo-shaped `*.atlasx/`;
- typed static lowering without stringly dynamic dispatch as the default;
- ownership/lifetime/effect/concurrency semantics sufficient to generate high-quality Rust;
- TypeScript browser/UI lowering and Rust/WASM escape hatch for heavy compute;
- C only for low-level ABI or explicitly selected device targets;
- compile/test/benchmark/recensus loop;
- differential equivalence tests between Atlas meaning and generated targets.

Exit gate: substantial systems can be generated and maintained from Atlas while Rust/TS remain the trusted physical backend.

## Phase 2 — Typed Atlas HIR/MIR + Semantic Optimizer

Deliver:

- HIR preserving Resource/State/Capability/Binding/Transaction/Effect/Temporal semantics;
- MIR with control flow, SSA-like values, loads/stores, calls, ownership/moves/borrows and effect barriers;
- constant folding, dead-code elimination, inlining, specialization, monomorphization, escape/alias analysis, allocation elimination, loop/dataflow optimization;
- semantic optimizer rules that forbid unsafe reordering across authority/effect/state/evidence boundaries;
- optimized Rust/TS/C emission retained for bootstrap and reference.

Exit gate: optimized IR semantics are proven equivalent to AtlasX semantics and generated target behavior.

## Phase 3 — External Native Backends

```text
*.atlasx/
   ↓
HIR → MIR → LIR
   ├─ LLVM
   ├─ Cranelift
   ├─ WASM
   └─ GPU/accelerator IR where justified
```

Deliver:

- stable Atlas ABI/calling/object layout contracts;
- native code generation through established backends;
- JIT/sandbox path where useful;
- WASM/browser path plus thin TypeScript bindings;
- Rust backend retained as reference/debug/bootstrap implementation;
- differential testing between rustc path and native-backend path.

Exit gate: selected production subsystems no longer require Rust as an intermediate representation while preserving behavior, memory-safety contracts and graph invariants.

## Phase 4 — Atlas Native Compiler Backend

Deliver in order:

1. portable Atlas Machine IR;
2. target descriptions;
3. instruction selection;
4. register allocation/spilling;
5. stack frame and calling convention lowering;
6. instruction scheduling;
7. object emission;
8. system-linker integration, followed later by native linking where justified;
9. x86-64 production backend;
10. ARM64 production backend;
11. RISC-V/embedded targets as evidence requires;
12. SIMD/vector, atomics and hardware-aware specialization;
13. whole-program graph-guided optimization.

Phase 4 does not mean deleting Phase 1–3. `atlas emit rust`, `atlas emit typescript` and external backends remain valuable for debugging, audit, bootstrap and differential proof.

## Terminal direction

```text
engineering reality
      ↓
*.atlas
      ↓
*.atlasx/
      ↓
Atlas HIR/MIR/Machine IR
      ↓
hardware-specialized physical artifact
```

One semantic design can specialize differently for server, browser, robot, embedded gateway or accelerator without becoming separate semantic universes.
