---
id: atlas.blueprint.rust-parity-surpass
type: blueprint
status: active
canonical: true
---
# Rust Parity and Surpass Roadmap

Atlas MUST NOT claim to "beat Rust" merely because it emits native code. The target is staged:

```text
Rust bootstrap
→ semantic parity
→ safety parity or stronger
→ diagnostics/tooling parity
→ codegen correctness parity
→ ABI/platform/debug parity
→ native backend parity
→ whole-system performance advantage
→ sustained maturity evidence
```

"Rust" here means the practical rustc + standard toolchain maturity bar relevant to Atlas-supported targets, not every historical Rust target or ecosystem package.

## Surpass definition

Atlas reaches RUST_SURPASS only when all mandatory parity gates are green and the locked Atlas benchmark corpus demonstrates a repeatable advantage over the reference rustc pipeline under equal semantics, safety, target hardware and workload constraints.

A faster unsafe or semantically weakened artifact does not count.

---

## R0 — Rust as trusted bootstrap/reference

AtlasX lowers to high-quality Rust/TypeScript and uses rustc/LLVM as the trusted backend.

Deliver:
- deterministic validated AtlasX binary capsule → Rust lowering;
- exact source/semantic lineage;
- differential tests between Atlas semantics and generated Rust;
- benchmark harness comparing equivalent implementations;
- compiler evidence records for every materialization.

Exit:
- Atlas can generate substantial systems reproducibly;
- rustc remains the physical reference oracle.

---

## R1 — Atlas type, ownership, effect and lifetime core

Build the semantic foundation required to equal and eventually exceed Rust safety.

Deliver:
- static type system;
- ownership/move/copy semantics;
- borrow/alias model;
- region/lifetime inference;
- mutable/shared exclusivity;
- resource drop/destruction semantics;
- effect/state/authority types;
- concurrency traits/capability constraints equivalent in role to Send/Sync where applicable;
- explicit unsafe/FFI boundary model;
- interior mutability/atomic/lock semantics;
- pinning/self-reference model where needed;
- async/coroutine ownership semantics.

Atlas advantage:
- ownership is integrated with State, Effect, Binding, Authority, Transaction and Temporal semantics, not treated only as memory aliasing.

Exit:
- accepted AtlasX programs cannot express memory-unsafe aliasing through safe semantics;
- compile-fail/compile-pass corpus covers ownership/lifetime edge cases;
- differential behavior matches Rust reference programs for shared semantics.

---

## R2 — Borrow checker maturity gate

A borrow checker prototype is not enough.

Deliver:
- path-sensitive borrow analysis;
- non-lexical lifetimes or stronger region reasoning;
- reborrows;
- partial moves;
- pattern/destructuring ownership;
- closures/captures;
- generators/async suspension points;
- trait/generic interactions;
- higher-ranked lifetime-like constructs where Atlas semantics require them;
- unsafe escape accounting;
- FFI alias contracts;
- concurrency race/conflict analysis integrated with state graph;
- precise diagnostics for borrow conflicts.

Verification:
- large generated compile-test matrix;
- mutation testing of borrow rules;
- differential Rust corpus where semantics overlap;
- Miri-like execution validation for reference materializations;
- model/property checking for core alias invariants.

Exit:
- no known safe-language memory-unsoundness in supported semantic surface;
- regression suite blocks previously found soundness bugs;
- critical compiler scopes require UNKNOWN = 0.

---

## R3 — Diagnostics and developer observability parity

Atlas compiler errors must be at least as actionable as the generated Rust errors it replaces.

Deliver:
- exact AtlasX/source/span/semantic-node mapping;
- multi-span diagnostics;
- causal error chains;
- "why this borrow/effect/binding is invalid" traces;
- suggested fixes with proof obligations;
- graph-aware diagnostics across repositories;
- type/effect/lifetime visualization;
- incremental IDE diagnostics;
- deterministic diagnostic IDs;
- diagnostic regression snapshots.

Atlas advantage:
- an error may be explained from semantic atom → function → module → binding → system invariant.

Exit:
- official error corpus has no fallback requirement to inspect generated Rust for root cause;
- user can trace every compiler error to Atlas semantic evidence.

---

## R4 — Codegen correctness and compiler trust

Performance work cannot outrun correctness.

Deliver:
- HIR/MIR verifier;
- IR well-formedness proofs/checkers;
- undefined-behavior contract;
- deterministic lowering;
- differential codegen against rustc/LLVM reference;
- property-based tests;
- compiler fuzzing;
- miscompilation minimization;
- randomized optimization-order testing;
- sanitizer/reference-runtime validation;
- reproducible-build mode;
- artifact/root hashes;
- crash/miscompile corpus;
- translation validation for high-risk optimization passes.

Exit:
- zero known silent miscompilations in supported release corpus;
- every discovered miscompile becomes permanent regression evidence;
- release requires compiler conformance/fuzz gates.

---

## R5 — LLVM-class external backend integration

Before replacing mature backend technology, Atlas must exploit it fully.

Deliver:
- first-class LLVM IR/backend integration;
- Cranelift fast/JIT path;
- ThinLTO/LTO;
- PGO;
- BOLT/post-link or equivalent evidence path;
- vector/SIMD lowering;
- target feature detection;
- sanitizer hooks where applicable;
- object emission and linker integration;
- cross-language C ABI;
- WASM backend;
- accelerator/GPU IR where justified.

Exit:
- Atlas IR can reach production native quality without Rust source as an intermediate;
- reference Rust and direct Atlas→LLVM paths pass differential correctness tests.

---

## R6 — ABI, object, unwind and platform maturity

A compiler is not production-grade if it only works on one happy-path Linux binary.

Initial mandatory target matrix:
1. x86-64 Linux;
2. AArch64 Linux;
3. x86-64 Windows;
4. AArch64 macOS;
5. WebAssembly;
6. RISC-V/embedded only after evidence justifies expansion.

Deliver per supported target:
- calling conventions;
- data/type layout;
- alignment/packing;
- symbol visibility;
- TLS;
- atomics;
- exception/panic/unwind model;
- stack unwinding;
- varargs where required;
- dynamic/static linking;
- shared libraries;
- relocations;
- ELF/PE-COFF/Mach-O/Wasm object support;
- C ABI conformance;
- OS/runtime startup and termination;
- thread-local/runtime hooks.

Exit:
- ABI conformance suites pass;
- cross-language round-trip tests pass;
- platform-specific edge cases are in regression corpus.

---

## R7 — Debug info, profiling and production observability parity

Deliver:
- DWARF for ELF/Mach-O targets;
- CodeView/PDB for Windows targets;
- source mapping from machine instruction → Machine IR → MIR/HIR → AtlasX → original evidence/source span;
- stack traces;
- variable/location info;
- async/coroutine debug mapping;
- profiler/perf integration;
- coverage;
- flame graphs/sample attribution;
- crash dump/symbolication;
- optimized-build debuggability.

Atlas advantage:
- profiler samples can map back not only to functions but to semantic capabilities, bindings, effects and organism organs/circuits.

Exit:
- production crash/performance investigation does not require generated Rust artifacts.

---

## R8 — Atlas native backend parity

Deliver:
- Machine IR;
- x86-64 and AArch64 instruction selectors;
- register allocation/spilling;
- frame lowering;
- instruction scheduling;
- peephole/machine optimization;
- atomics/SIMD;
- object emission;
- linker integration;
- native backend differential tests against LLVM;
- compile-speed and code-quality dashboards.

Do NOT remove LLVM/Rust paths. They remain independent oracles and fallback/reference backends.

Exit:
- native backend passes R4/R6/R7 gates on its declared target subset;
- runtime performance/code size is competitive with Atlas→LLVM on locked corpus.

---

## R9 — Whole-world optimizer: the actual path to surpass Rust

This is where Atlas can exceed a conventional source compiler.

Use knowledge unavailable or only partially available after ordinary language lowering:

- global Binding Graph;
- State/Effect/Authority Graph;
- cross-repository call/dataflow;
- deployment topology;
- hardware profile;
- workload profile;
- temporal/evidence constraints.

Transformations may include:
- compile-time binding resolution;
- whole-world devirtualization;
- removal/fusion of unnecessary service/interface boundaries;
- serialization elimination;
- state placement/partitioning;
- hot/cold data splitting;
- AoS↔SoA specialization;
- arena/region placement;
- ownership transfer simplification;
- cross-component inlining/fusion;
- concurrency topology selection;
- lock/shard/actor specialization;
- SIMD/GPU offload;
- algorithm selection;
- model/provider placement for organism targets.

Exit:
- semantic-equivalence proof/validation passes;
- advantage comes from system-level transformation, not relaxed correctness.

---

## R10 — Empirical PGO and auto-tuning

Deliver:
- representative workload capture;
- profile lineage and temporal validity;
- multi-version compilation;
- parameter/algorithm/layout search;
- benchmark isolation/noise control;
- Pareto optimization;
- target-specific binary selection;
- rollback if runtime evidence regresses.

Candidate dimensions:
- algorithm;
- layout;
- inline threshold;
- chunk size;
- arena strategy;
- thread count;
- scheduling strategy;
- SIMD width;
- GPU/CPU split;
- local/external model routing.

Exit:
- selected artifact beats static Atlas and rustc reference under repeated blinded runs on locked hardware/workload.

---

## R11 — Sustained compiler maturity

"Years of compiler testing" is replaced by measurable accumulated evidence, not calendar age alone.

Required maturity systems:
- always-on fuzzing;
- compiler crash/miscompile database;
- regression minimization;
- ecosystem-scale rebuild testing analogous in purpose to crater;
- nightly/canary/stable release channels;
- reproducible bootstrap;
- self-hosting where justified;
- backward/forward format compatibility tests;
- long-running stress/concurrency tests;
- hardware farm across supported architectures;
- corpus of real production systems;
- security response process;
- compiler change risk classification;
- rollback/bisect tooling.

Maturity metrics include:
- compiler-hours fuzzed;
- unique semantic programs tested;
- target/hardware matrix coverage;
- release count without critical miscompile;
- production artifact-hours;
- ABI/debugger conformance coverage;
- known soundness defect count;
- mean detection/minimization time.

Exit:
- reliability is demonstrated by accumulated evidence, not project age.

---

# Rust Parity Gate

Atlas may declare RUST_PARITY only when all of the following are true for the declared supported subset:

```text
Memory/type safety          PASS
Borrow/ownership soundness  PASS
Diagnostics                 PASS
Codegen correctness         PASS
LLVM/native integration     PASS
ABI/platform matrix         PASS
Debug/profiling             PASS
Fuzz/regression maturity    PASS
Production evidence         PASS
```

Unsupported Rust features/targets must be explicit. Atlas must not claim global Rust parity from a narrow benchmark.

# Rust Surpass Gate

Atlas may declare RUST_SURPASS_<TARGET_PROFILE> only when:

1. Rust Parity Gate is already green for that target profile.
2. Same functional/semantic workload is compared.
3. Same hard safety/authority constraints apply.
4. Same hardware/OS/toolchain conditions are pinned.
5. Benchmarks are repeated and statistically stable.
6. Atlas wins the declared objective without unacceptable regression in constrained secondary objectives.
7. The win reproduces after clean rebuild.
8. Results and profiles are stored as Evidence in Atlas.

Suggested first claim scope:

```text
RUST_SURPASS_X86_64_LINUX_SERVER
```

not:

```text
ATLAS_IS_FASTER_THAN_RUST_EVERYWHERE
```

# Strategic rule

The fastest route to surpassing Rust is NOT to prematurely replace rustc.

```text
use Rust as oracle
→ absorb its compiler principles
→ build stronger Atlas semantics
→ exploit LLVM/Cranelift
→ prove correctness
→ build native backend
→ use whole-world knowledge Rust source compilers do not normally possess
→ empirically prove the advantage
```
