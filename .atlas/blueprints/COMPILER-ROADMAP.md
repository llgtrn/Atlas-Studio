---
id: atlas.blueprint.compiler-roadmap
type: blueprint
status: active
canonical: true
---
# Atlas Compiler Roadmap

The roadmap is cumulative.

Later phases may replace physical backends but may not weaken Genome, census completeness, graph, temporal, evidence, provenance, artifact-layering, or closed-world dependency invariants.

Normative artifact roles are fixed first by `../contracts/ARTIFACT-LAYERING.md`.

Normative design/capsule/compiler semantics are defined by:

- `../contracts/SELECTED-DESIGN.md`;
- `../contracts/ATLAS-TO-ATLASX.md`;
- `../contracts/ATLASX-FORMAT.md`;
- `../contracts/ATLASX-BINARY-WIRE-FORMAT.md`;
- `../contracts/COMPILER-IR-PIPELINE.md`;
- `../contracts/COMPILER-IR-SCHEMAS.md`.

This roadmap is canonical for the current evidence state but MAY be revised under `../contracts/BLUEPRINT-EVOLUTION.md` when donor/dependency census or verification/benchmark evidence proves a materially better architecture.

Such revision must be explicit.

Implementation code may not silently redefine artifact roles or the compiler pipeline.

## Artifact model prerequisite

The compiler roadmap assumes:

~~~text
ADL      = natural-language-first Intent Artifact
*.atlas  = canonical semantic binary
*.atlasx = closed-world single-file system capsule
~~~

The compiler does not consume ADL as canonical truth.

The compiler does not establish canonical input by scanning an unpacked AtlasX directory.

## Phase 0 — Genome, Strict Census, ADL Resolution, and Logical ATLAS

Deliver:

- deterministic Genome identity/hash and machine enforcement path;
- universal graph/binding/evidence/temporal primitives;
- exhaustive artifact inventory;
- S0→S10 scope lattice with every required function accounted for;
- CFG/call/data/state/effect/binding semantics and explicit UNKNOWN/UNSUPPORTED records;
- multi-engine reconciliation and adversarial gap queries;
- fixed-point closure and CensusCertificate;
- Human+AI ADL authoring with natural-language-first intent/constraints;
- deterministic ADL → typed semantic elaboration;
- research/donor/candidate/selection pipeline;
- generated-code census before admission;
- explicit SelectedDesign;
- provider-free logical seal;
- real `*.atlas` binary reader/writer with typed records, dictionaries, semantic dedup, compression, chunk hashes, bounded random access, and transactional publication;
- logical Atlas root/shard manifests, content addressing, lazy fetch, and partial semantic materialization where contracts permit.

Exit gate:

- no silent omissions in the admitted corpus;
- required scopes reach closure;
- publication-critical ADL ambiguity is resolved;
- a SEALED logical Atlas root can be independently verified without original ADL or provider interpretation.

## Atlas Development Language lane

Full Atlas Development Language work begins only after typed census semantics can survive losslessly through Census/Normalization.

The current ADL0 declaration parser remains a bootstrap subset.

~~~text
Human + AI natural-language intent
        +
deep donor/dependency census
        ↓
Technology Genomes
        ↓
mechanism/invariant comparison
        ↓
Atlas-native semantic primitives
        ↓
typed ADL elaboration
        ↓
candidate synthesis / validation / selection
        ↓
resolved canonical semantic world
        ↓
*.atlas
~~~

This lane is governed by:

- `../contracts/ATLAS-DEVELOPMENT-LANGUAGE.md`;
- `../contracts/HUMAN-AI-ADL-AUTHORING.md`;
- `../contracts/ADL-TO-ATLAS.md`;
- `../contracts/DONOR-TO-LANGUAGE-GENESIS.md`.

Donor syntax/APIs may not become language authority by convenience.

Natural language may remain the primary human authoring surface while exact semantics lower into typed Atlas records.

## Phase 1 — Deterministic ATLASX Closure + Delegated Compiler

Phase 1 no longer treats an AtlasX directory as canonical input.

The canonical path is:

~~~text
SEALED *.atlas
  ↓ validate root / Genome / certificate
SelectedDesign
  ↓ compute selected semantic closure
  ↓ recursively close dependencies
  ↓ close build/runtime/assets/models/toolchains according to profile
  ↓ bind explicit external boundaries
  ↓ legal/security/provenance gates
  ↓ deterministic AtlasX packaging
*.atlasx binary capsule (wire v2)
  ↓ validate capsule root + complete closure
  ↓ delegated typed lowering
Rust / TypeScript / bounded C ABI
  ↓ pinned toolchain/backend
physical product
~~~

The Atlas→AtlasX transition MUST follow `../contracts/ATLAS-TO-ATLASX.md`.

### Phase 1 deliverables

Deliver:

- AtlasX wire v2 single-file packer/reader/validator;
- VERIFY_ONLY capsule profile;
- BUILD_REPRODUCIBLE capsule profile;
- EXECUTABLE_PORTABLE profile for supported targets;
- DEPLOYMENT_TARGETED profile where evidence justifies it;
- recursive dependency closure to fixed point;
- EMBEDDED/PINNED_FETCH/external-boundary classification;
- SBOM/license/provenance integration;
- runtime/model/asset entry support;
- toolchain/build-recipe closure for reproducible builds;
- secure bounded decompression and entry validation;
- AtlasX root identity/reproducibility tests;
- migration reader for legacy AtlasX wire v1 directory/object roots;
- high-quality static Rust lowering;
- browser/UI TypeScript projection;
- bounded C ABI/device boundaries;
- compile/test/benchmark/recensus;
- differential semantic verification.

The delegated compiler remains a backend over a validated AtlasX capsule.

Generated source is a noncanonical physical projection and may not become a new semantic authority.

Exit gate:

- substantial systems can be packed into independently verified AtlasX v2 capsules;
- required transitive dependencies are closed according to capsule profile;
- delegated backends produce verified products without host-ambient semantic dependencies;
- Rust/TS remain trusted physical backends, not semantic authority.

## Phase 2 — Atlas HIR/MIR + Whole-Semantic Optimizer

HIR and MIR are not implementation-defined names.

Their normative responsibilities are fixed by `../contracts/COMPILER-IR-PIPELINE.md`.

Their record/op schemas are fixed by `../contracts/COMPILER-IR-SCHEMAS.md`.

Deliver:

- HIR preserving Resource/State/Capability/Binding/Transaction/Effect/Temporal semantics and AtlasX lineage;
- MIR with explicit CFG, SSA-like/block-argument values, calls, loads/stores, ownership/moves/borrows, failure paths, and typed semantic barriers;
- graph/binding specialization and devirtualization;
- policy partial evaluation;
- state placement;
- ownership/lifetime/region selection;
- escape/alias analysis and allocation elimination;
- data-layout/locality optimization;
- concurrency/conflict analysis;
- inlining, constant folding, DCE, CSE, specialization, monomorphization, and loop/dataflow optimization.

Exit gate: optimized IR is semantically equivalent to the validated AtlasX input and measurably improves selected workloads without breaking required invariants.

## Phase 3 — External Native Backends

~~~text
validated *.atlasx capsule
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

LIR, target model, and ABI obligations are governed by `../contracts/COMPILER-IR-PIPELINE.md`.

Deliver stable Atlas ABI/object-layout contracts, AOT/JIT/sandbox paths, WASM/browser path, target vectorization/SIMD, and differential testing against the delegated reference path.

Exit gate: selected production systems no longer require Rust source as an intermediate representation.

## Phase 4 — Atlas Native Machine Backend

Machine IR semantics, target instructions/register classes, ABI/call lowering, object emission, and verification obligations are governed by `../contracts/COMPILER-IR-PIPELINE.md`.

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

Before training new weights, Atlas must be able to construct, seal, close, and compile the organism substrate:

~~~text
Organism intent / Genome
  ↓
organs / circuits / body / brain bindings
  ↓
memory / world / learning / homeostasis / metabolism
  ↓
capability + authority + lifecycle substrate
  ↓
SEALED Atlas semantics
  ↓
closed AtlasX organism capsule
  ↓
phenotype repository/product
~~~

Cognition may bind to an external API, self-hosted checkpoint, deterministic implementation, or hybrid set.

External model/provider boundaries must be explicit in SelectedDesign and AtlasX closure.

Later compiler phases optimize model placement, GPU kernels, and provider routing without changing durable organism identity or authority semantics.

Training/evolution remains downstream of substrate maturity:

~~~text
experience → memory → dataset → candidate model/weights/rules
→ evaluation/simulation/regression → admission → activation
~~~

## Production Optimization Lane

All mature backends support:

~~~text
validated AtlasX capsule
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
Atlas evidence / recensus
~~~

The compiler may specialize separately for server, browser, robot, embedded, realtime, or accelerator targets without creating new semantic universes.

Rust/TypeScript emitters and external backends remain useful for bootstrap, audit, debugging, and differential proof even after native codegen exists.

## Rust Parity and Surpass Destination

The detailed maturity roadmap is `RUST-PARITY-AND-SURPASS-ROADMAP.md`.

Native code generation alone is not parity.

Atlas must close:

~~~text
borrow/ownership soundness
diagnostics
codegen correctness
LLVM/external backend quality
ABI/platform edge cases
debug/profiling information
fuzz/regression maturity
production evidence
~~~

Only after those gates are green may Atlas establish any scoped performance/capability claim on a locked target/workload profile.

## Evidence-driven compiler blueprint revision

Compiler architecture is expected to improve as Atlas censuses compiler, optimizer, storage, linker, packaging, and verification donors and their dependency closures.

Examples of valid discoveries include a better:

- HIR/MIR structure;
- SSA/value representation;
- ownership/resource model;
- effect/barrier representation;
- devirtualization strategy;
- Atlas semantic encoding;
- AtlasX capsule entry structure;
- deterministic closure algorithm;
- register allocator;
- instruction selector;
- object layout;
- linker;
- incremental compilation strategy;
- translation-validation method.

A discovered mechanism may revise this roadmap or a stage blueprint when the evidence-backed decision passes `../contracts/BLUEPRINT-EVOLUTION.md`.

Artifact-role changes require explicit contract migration and cannot be smuggled in as implementation optimizations.

The required loop is:

~~~text
census discovery
→ deep census + dependency closure + provider attribution
→ compare against current compiler/artifact blueprint
→ benchmark/prove/falsify
→ BlueprintRevisionDecision
→ update canonical docs/contracts if selected
→ implement
→ differential verification
→ recensus Atlas
~~~

Do not preserve a weaker first design merely because it is already documented.

Do not silently replace a documented stage in code either.
