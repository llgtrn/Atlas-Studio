---
id: atlas.blueprint.compiler-roadmap
type: blueprint
status: active
canonical: true
---
# Atlas Compiler Roadmap

The roadmap is cumulative. Later phases may replace physical backends but may not weaken Genome, census completeness, graph, temporal, evidence or provenance invariants.

Normative design/materialization/compiler semantics are defined by `../contracts/SELECTED-DESIGN.md`, `../contracts/ATLAS-TO-ATLASX.md`, `../contracts/ATLASX-FORMAT.md`, `../contracts/ATLASX-BINARY-WIRE-FORMAT.md`, `../contracts/COMPILER-IR-PIPELINE.md` and `../contracts/COMPILER-IR-SCHEMAS.md`.

This roadmap is a blueprint. It is canonical for the current evidence state but MAY be revised under `../contracts/BLUEPRINT-EVOLUTION.md` when donor/dependency census or verification/benchmark evidence proves a materially better architecture. Such revision must be explicit; implementation code may not silently redefine the pipeline.

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

## Atlas Development Language lane

Full Atlas Development Language work begins only after typed census semantics can survive losslessly through Census/normalization. The current ADL0 declaration parser remains a bootstrap subset.

```text
deep donor census
→ Technology Genomes
→ mechanism/invariant comparison
→ Atlas-native semantic primitives
→ ADL feature admission
→ typed ADL lowering
→ canonical semantic world
→ *.atlas
```

This lane is governed by `../contracts/ATLAS-DEVELOPMENT-LANGUAGE.md`, `../contracts/ADL-TO-ATLAS.md` and `../contracts/DONOR-TO-LANGUAGE-GENESIS.md`. Donor syntax or APIs may not become language authority by convenience.

## Phase 1 — Deterministic ATLASX + Delegated Compiler

~~~text
SEALED *.atlas
  ↓ validate root / Genome / certificate
SelectedDesign
  ↓ compute selection closure
  ↓ resolve only design-authorized bindings
  ↓ deterministic expansion
validated *.atlasx root
  ↓ delegated typed lowering
Rust / TypeScript / bounded C ABI
  ↓ pinned rustc / TS toolchain / clang
physical product
~~~

The Atlas→AtlasX transition MUST follow `../contracts/ATLAS-TO-ATLASX.md`.

The delegated compiler remains a compiler backend over validated AtlasX. Generated source is noncanonical physical projection and may not become a new semantic authority.

Deliver deterministic AtlasX root identities, validated object/reference closure, high-quality static Rust lowering, browser/UI TypeScript projection, bounded C ABI/device boundaries, compile/test/benchmark/recensus and differential semantic verification.

Exit gate: substantial AtlasX systems produce verified products while Rust/TS remain trusted physical backends.

## Phase 2 — Atlas HIR/MIR + Whole-Semantic Optimizer

HIR and MIR are not implementation-defined names. Their normative responsibilities are fixed by `../contracts/COMPILER-IR-PIPELINE.md`; their v1 logical record/op schemas are fixed by `../contracts/COMPILER-IR-SCHEMAS.md`.

Deliver:

- HIR preserving Resource/State/Capability/Binding/Transaction/Effect/Temporal semantics and AtlasX lineage;
- MIR with explicit CFG, SSA-like/block-argument values, calls, loads/stores, ownership/moves/borrows, failure paths and typed semantic barriers;
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

~~~text
validated *.atlasx
  ↓
HIR
  ↓
MIR
  ↓ target/layout/ABI lowering
LIR
  ├─ LLVM adapter
  ├─ Cranelift adapter
  ├─ WASM adapter
  └─ GPU/accelerator adapter
~~~

LIR, target model and ABI obligations are governed by `../contracts/COMPILER-IR-PIPELINE.md`.

Deliver stable Atlas ABI/object-layout contracts, AOT/JIT/sandbox paths, WASM/browser path, target vectorization/SIMD and differential testing against the Rust reference path.

Exit gate: selected production systems no longer require Rust source as an intermediate representation.

## Phase 4 — Atlas Native Machine Backend

Machine IR semantics, target instructions/register classes, ABI/call lowering, object emission and verification obligations are governed by `../contracts/COMPILER-IR-PIPELINE.md`.

Deliver:

1. portable-to-targeted Machine IR family with explicit versioned target semantics;
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

## Evidence-driven compiler blueprint revision

Compiler architecture is expected to improve as Atlas censuses compiler, optimizer, storage, linker and verification donors and their dependency closures.

Examples of valid discoveries include a better:

- HIR/MIR structure;
- SSA/value representation;
- ownership/resource model;
- effect/barrier representation;
- devirtualization strategy;
- register allocator;
- instruction selector;
- object layout;
- linker;
- incremental compilation strategy;
- translation-validation method.

A discovered mechanism may revise this roadmap or a stage blueprint when the evidence-backed decision passes `../contracts/BLUEPRINT-EVOLUTION.md`.

Do not preserve a weaker first design merely because it is already documented.

Do not silently replace a documented stage in code either.

The required loop is:

~~~text
census discovery
→ deep census + provider attribution
→ compare against current compiler blueprint
→ benchmark/prove/falsify
→ BlueprintRevisionDecision
→ update canonical docs/contracts if selected
→ implement
→ differential verification
→ recensus Atlas
~~~
