---
id: atlas.blueprint.compiler-roadmap
type: blueprint
status: active
canonical: true
---
# Atlas Compiler Roadmap

The roadmap is cumulative. Later phases may replace physical backends but may not weaken Genome, census completeness, graph, temporal, evidence or provenance invariants.

## Phase 0 — Genome, Strict Census and Logical ATLAS

Deliver:

- deterministic Genome identity/hash and machine enforcement path;
- universal graph/binding/evidence/temporal primitives;
- exhaustive artifact inventory;
- S0→S10 scope lattice with every function accounted;
- CFG/call/data/state/effect/binding semantics and explicit UNKNOWN/UNSUPPORTED records;
- multi-engine reconciliation and adversarial gap queries;
- fixed-point closure and CensusCertificate;
- real `*.atlas` binary reader/writer with typed records, dictionaries, semantic dedup, compression, chunk hashes, bounded random access and transactional publication;
- logical Atlas root/shard manifests, content addressing, lazy fetch and partial materialization;
- research/gap/candidate/selected-design layers.

Exit gate: no silent omissions in the admitted corpus; required scopes reach closure; a SEALED logical Atlas root can be independently verified.

## Phase 1 — Deterministic ATLASX + Delegated Compiler

```text
SEALED *.atlas
  ↓ materialize selected design
*.atlasx/
  ↓ typed lowering
Rust / TypeScript / bounded C ABI
  ↓ rustc / TS toolchain / clang
physical product
```

Deliver deterministic AtlasX semantic hashes, high-quality static Rust lowering, browser/UI TypeScript projection, bounded C ABI/device boundaries, compile/test/benchmark/recensus and differential semantic verification.

Exit gate: substantial AtlasX systems produce verified products while Rust/TS remain trusted physical backends.

## Phase 2 — Atlas HIR/MIR + Whole-Semantic Optimizer

Deliver:

- HIR preserving Resource/State/Capability/Binding/Transaction/Effect/Temporal semantics;
- MIR with CFG, SSA-like values, calls, loads/stores, ownership/moves/borrows and semantic barriers;
- graph/binding specialization and devirtualization;
- policy partial evaluation;
- state placement;
- ownership/lifetime/region selection;
- escape/alias analysis and allocation elimination;
- data-layout/locality optimization;
- concurrency/conflict analysis;
- inlining, constant folding, DCE, CSE, specialization, monomorphization and loop/dataflow optimization.

Exit gate: optimized IR is semantically equivalent to AtlasX and measurably improves selected workloads without breaking required invariants.

## Phase 3 — External Native Backends

```text
*.atlasx/
  ↓
HIR → MIR → LIR
  ├─ LLVM
  ├─ Cranelift
  ├─ WASM
  └─ GPU/accelerator IR
```

Deliver stable Atlas ABI/object-layout contracts, AOT/JIT/sandbox paths, WASM/browser path, target vectorization/SIMD and differential testing against the Rust reference path.

Exit gate: selected production systems no longer require Rust source as an intermediate representation.

## Phase 4 — Atlas Native Machine Backend

Deliver:

1. portable Machine IR;
2. target descriptions;
3. instruction selection;
4. register allocation/spilling;
5. stack/calling convention lowering;
6. instruction scheduling;
7. object emission;
8. system linker integration;
9. x86-64 backend;
10. ARM64 backend;
11. RISC-V/embedded as justified;
12. SIMD/vector/atomics/hardware-aware specialization;
13. whole-program graph-guided optimization.

## Digital Organism Target Lane

Digital Organism is a compiler target profile, not a separate compiler universe.

Before training new weights, Atlas must be able to compile and instantiate the organism substrate:

```text
Organism Genome
  ↓
organs / circuits / body / brain bindings
  ↓
memory / world / learning / homeostasis / metabolism
  ↓
capability + authority + lifecycle substrate
  ↓
phenotype repository/product
```

Cognition may bind to an external API, self-hosted checkpoint, deterministic implementation or hybrid set. Later compiler phases optimize model placement, GPU kernels and provider routing without changing durable organism identity or authority semantics.

Training/evolution is downstream of substrate maturity:

```text
experience → memory → dataset → candidate model/weights/rules
→ evaluation/simulation/regression → admission → activation
```

## Production Optimization Lane

All mature backends support:

```text
AtlasX
 + DeploymentProfile
 + HardwareProfile
 + WorkloadProfile
      ↓
world/graph specialization
      ↓
IR optimization
      ↓
target codegen
      ↓
LTO / whole-program optimization
      ↓
link
      ↓
post-link code/data layout
      ↓
product
      ↓
representative workload
      ↓
PGO + auto-tuning
      ↺
Atlas evidence
```

The compiler may specialize separately for server, browser, robot, embedded, realtime or accelerator targets without creating new semantic universes.

Rust/TypeScript emitters and external backends remain permanently useful for bootstrap, audit, debugging and differential proof even after native codegen exists.


## Rust Parity and Surpass Destination

The detailed maturity roadmap is `RUST-PARITY-AND-SURPASS-ROADMAP.md`.

Native code generation alone is not parity. Atlas must close:

```text
borrow/ownership soundness
diagnostics
codegen correctness
LLVM/external backend quality
ABI/platform edge cases
debug/profiling information
fuzz/regression maturity
production evidence
```

Only after those gates are green may Atlas use whole-world graph optimization, PGO and auto-tuning to establish a scoped Rust-surpass claim on a locked target/workload profile.
