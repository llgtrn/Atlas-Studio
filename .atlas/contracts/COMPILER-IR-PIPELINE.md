---
id: atlas.contract.compiler-ir-pipeline
type: contract
status: active
canonical: true
---
# Atlas Compiler IR Pipeline Contract

## Purpose

This contract defines the semantic responsibilities and lowering boundaries from validated ATLASX to physical product.

It exists to prevent compiler implementations from inventing incompatible meanings for HIR, MIR, LIR or Machine IR.

The canonical native pipeline is:

~~~text
validated *.atlasx root
        ↓
HIR
        ↓
MIR
        ↓
LIR
        ↓
Machine IR
        ↓
object/code emission
        ↓
link / post-link
        ↓
physical product
~~~

Delegated bootstrap backends may temporarily emit Rust/TypeScript/bounded C, but they remain compiler backends over validated AtlasX semantics and do not redefine the canonical semantic world.

## Compiler input identity

A compiler invocation MUST identify all inputs that may affect output semantics or physical specialization:

- validated AtlasX root identity;
- parent Atlas root identity through AtlasX lineage;
- Genome identity/hash;
- compiler identity/version;
- compiler pipeline schema version;
- DeploymentProfile identity;
- HardwareProfile identity;
- WorkloadProfile identity when optimization depends on workload;
- target triple/target description;
- ABI profile;
- backend identity/version;
- optimization profile;
- PGO/profile evidence identities where used;
- admitted external toolchain identities;
- feature/policy inputs.

Ambient host state MUST NOT silently affect semantic compilation.

## Global compiler invariants

All compiler stages MUST preserve:

- selected design meaning;
- stable lineage to AtlasX/Atlas;
- authority/security barriers;
- state transition semantics;
- effect semantics;
- ownership/resource requirements;
- concurrency ordering requirements;
- transaction/persistence/recovery requirements;
- temporal requirements;
- required interfaces/bindings;
- failure behavior where externally observable or contractually required;
- evidence/provenance lineage required for verification;
- target/profile constraints;
- declared undefined/unresolved behavior boundaries.

Optimization is invalid if any required invariant is lost.

## Stage principle

Every stage has one responsibility level.

~~~text
AtlasX
= explicit selected executable semantics

HIR
= high-level compiler semantic IR

MIR
= explicit executable control/data/memory/resource IR

LIR
= target-family/ABI/layout-aware low-level IR

Machine IR
= target-instruction-level program before/following register allocation
~~~

A stage MUST NOT depend on information that was silently discarded by an earlier stage.

If a later stage needs semantic information, that information must either:

- survive explicitly in the IR; or
- survive as attached typed semantic barrier/metadata with precise lineage.

## HIR contract

HIR is target-independent or minimally target-constrained high-level executable semantic IR.

HIR MUST retain enough meaning to represent:

- modules/namespaces required for compilation;
- canonical types and type parameters;
- functions/methods;
- interfaces/capabilities;
- bindings;
- calls including dynamic binding where still permitted;
- state objects and state transitions;
- effects;
- resources/ownership requirements;
- concurrency constructs;
- transaction/persistence constructs;
- constraints/invariants that affect executable legality;
- error/failure semantics;
- external boundaries;
- target-independent data representation requirements;
- semantic barriers;
- source/AtlasX lineage.

HIR SHOULD eliminate presentation-only AtlasX organization that has no compiler meaning.

HIR MUST NOT yet commit to machine register allocation, concrete instruction selection or target object layout except where target policy makes a representation semantic requirement.

### HIR identity

Every HIR entity MUST have deterministic identity derived from:

- AtlasX semantic lineage;
- compiler pipeline schema;
- deterministic expansion/specialization coordinate where one AtlasX entity produces multiple HIR entities.

Display names are not identity.

### HIR allowed transformations

HIR may perform semantically proven operations such as:

- explicit binding resolution already permitted by AtlasX;
- generic/template specialization according to compiler policy;
- capability/interface devirtualization when proven;
- high-level constant propagation;
- unreachable selected-design branch elimination;
- state/capability fusion when invariants prove equivalence;
- high-level partial evaluation;
- target-independent normalization.

Every transformation must preserve required lineage/evidence.

## MIR contract

MIR makes execution order, values, memory/resource operations and control flow explicit.

MIR MUST represent:

- functions;
- deterministic basic blocks;
- block arguments or SSA-like phi semantics;
- typed values;
- constants;
- calls;
- branches/switches;
- returns;
- panic/exception/failure/unwind paths as required;
- loads/stores;
- address/reference creation where applicable;
- allocation/deallocation or abstract resource operations;
- ownership moves/borrows/copies where semantically relevant;
- lifetime/region barriers where required;
- state reads/writes/transitions;
- effect operations;
- lock/channel/atomic/task/thread primitives or lowerable abstract equivalents;
- transaction begin/commit/abort/recovery operations where required;
- external calls/FFI;
- semantic barriers;
- provenance/lineage.

MIR is the primary representation for whole-function and cross-function semantic optimization.

### MIR CFG requirements

Each executable function has:

- exactly one logical entry;
- explicit reachable blocks;
- explicit terminators;
- deterministic block identity/order under canonical serialization;
- explicit failure/unwind edges where required by semantics;
- no hidden fallthrough semantics.

### MIR value model

Values MUST have stable deterministic identities within their function/body revision.

MIR must distinguish:

- value identity;
- storage location;
- reference/borrow;
- resource handle;
- state identity.

These MUST NOT collapse into one generic string or pointer-like bucket.

### MIR memory/resource model

MIR must make enough memory/resource behavior explicit for:

- ownership validation;
- alias/escape analysis;
- allocation elimination;
- state-placement optimization;
- concurrency analysis;
- ABI lowering.

If alias/ownership facts are unresolved, the compiler must conservatively preserve semantics rather than assume freedom to optimize.

### MIR effect barriers

Operations with externally observable or authority-sensitive effects carry typed barriers.

Examples:

- filesystem write;
- network send;
- process spawn;
- FFI;
- authority check;
- persistence commit;
- synchronization;
- model/provider call where ordering matters;
- lifecycle transition.

Passes may move/remove such operations only when the relevant equivalence proof permits it.

## LIR contract

LIR is low-level, target-family-aware IR.

LIR commits to:

- concrete or target-family-constrained scalar/vector widths;
- data layouts;
- alignment;
- address spaces;
- calling convention classes;
- lowered aggregates;
- lowered control flow;
- explicit memory operations;
- concrete atomic ordering classes;
- explicit ABI boundaries;
- exception/failure lowering strategy;
- stack/heap/static placement classes;
- relocation/symbol requirements;
- vector/SIMD operations where selected;
- accelerator/GPU boundary representation where selected.

LIR MUST retain enough semantic barrier metadata to prevent machine-level optimization from violating higher-level invariants.

### LIR is not Machine IR

LIR uses low-level operations independent of one exact instruction set where possible.

Examples:

~~~text
i64.add
load align=8
atomic_cmpxchg ordering=acq_rel
call abi=sysv64 ...
vector_add <8 x f32>
~~~

Machine IR chooses exact target instructions/register classes.

## Machine IR contract

Machine IR represents the target program at instruction-selection/register-allocation/scheduling level.

Machine IR MUST represent where applicable:

- target opcode;
- virtual registers before allocation;
- physical registers after allocation where phase-appropriate;
- register classes;
- immediates;
- memory operands/addressing modes;
- stack slots;
- calling-convention operands;
- clobbers;
- flags/condition codes;
- branch targets;
- relocatable symbols;
- unwind/debug/profiling locations;
- scheduling constraints;
- atomic/fence requirements;
- semantic barrier metadata that still constrains legal scheduling.

Machine IR identity remains lineage-linked to LIR/MIR operations.

## Lowering contracts

Each transition is explicit:

~~~text
AtlasX → HIR
HIR → MIR
MIR → LIR
LIR → Machine IR
Machine IR → object/code
~~~

For each transition, implementation MUST define:

- supported input schema/version;
- supported semantic constructs;
- output schema/version;
- deterministic ordering;
- unsupported construct behavior;
- legality checks;
- semantic-equivalence obligations;
- lineage mapping;
- diagnostic model;
- verification tests.

Unsupported required semantics MUST produce a compiler diagnostic/failure, not silent omission.

## AtlasX → HIR

This lowering:

- consumes only validated AtlasX semantic objects;
- converts executable selected-design semantics into compiler-oriented high-level IR;
- expands compiler-owned generic/specialization structures;
- retains dynamic bindings that remain semantically dynamic;
- retains state/effect/resource/concurrency/persistence meaning;
- attaches semantic barriers;
- preserves AtlasX lineage.

It MUST NOT perform target machine lowering.

## HIR → MIR

This lowering:

- constructs explicit CFG;
- introduces explicit value flow;
- lowers structured control constructs;
- lowers high-level state/resource operations into explicit MIR operations;
- makes failure paths explicit;
- introduces memory/storage abstractions;
- preserves effect/authority/transaction barriers;
- records mapping from HIR entities to MIR functions/blocks/values.

HIR constructs with no legal MIR lowering MUST fail explicitly.

## MIR → LIR

This lowering commits to target/ABI/layout policy.

It:

- selects concrete layouts;
- lowers aggregates;
- lowers ownership/resource abstractions to concrete memory/resource operations;
- resolves calling conventions;
- lowers concurrency primitives to target/runtime mechanisms;
- lowers persistence/runtime interfaces;
- chooses vector widths/intrinsics when permitted;
- represents relocations/symbol boundaries;
- preserves semantic barriers required for codegen scheduling.

Target facts MUST be explicit compiler inputs.

## LIR → Machine IR

This lowering performs:

- instruction selection;
- target operand formation;
- address-mode selection;
- virtual register creation;
- target branch selection;
- target call sequence formation;
- target-specific atomic/fence lowering;
- machine-level barrier annotation.

Register allocation/scheduling may be subsequent Machine IR passes.

## Machine IR → object/code

This stage includes as applicable:

- register allocation/spilling;
- stack frame layout;
- prologue/epilogue generation;
- instruction scheduling;
- branch relaxation;
- constant pools;
- object section emission;
- relocations;
- symbol tables;
- unwind info;
- debug/profiling info;
- target metadata.

The object writer/linker must preserve declared ABI and externally observable behavior.

## Target model

A target description MUST explicitly define:

- architecture;
- endianness;
- pointer width;
- scalar/vector legal types;
- register classes;
- instruction features;
- alignment rules;
- address spaces;
- atomic capabilities;
- calling-convention availability;
- object format;
- relocation model;
- TLS model where applicable;
- exception/unwind model;
- debug format;
- platform/runtime requirements.

Host defaults are insufficient for canonical production compilation.

## ABI model

ABI is typed compiler policy, not ad hoc backend convention.

An ABI profile MUST define where applicable:

- symbol naming/mangling;
- calling convention;
- parameter/return classification;
- aggregate passing;
- stack alignment;
- register preservation;
- variadic rules;
- exception/unwind boundary;
- object layout requirements;
- FFI ownership/resource transfer rules;
- versioning/compatibility.

FFI/C boundaries MUST refer to explicit ABI profiles.

## Optimization placement

Optimization responsibilities are ordered:

### Atlas/SelectedDesign

Chooses intended engineering semantics.

### Materialization

Expands selected semantics deterministically; no performance invention.

### HIR

High-level semantic/world optimization:

- binding specialization;
- capability devirtualization;
- policy partial evaluation;
- state/capability fusion where proven.

### MIR

Execution/data/memory optimization:

- inlining;
- constant folding;
- DCE;
- CSE;
- escape/alias optimization;
- allocation elimination;
- loop/data-flow optimization;
- ownership/lifetime simplification;
- concurrency conflict optimization.

### LIR

Target-aware optimization:

- layout;
- vectorization;
- target intrinsics;
- calling convention simplification;
- low-level memory optimization.

### Machine IR

Instruction-level optimization:

- instruction selection combines;
- register allocation;
- scheduling;
- peephole optimization;
- branch/layout tuning.

### Link/post-link

- LTO/whole-program;
- dead section elimination;
- symbol/internalization;
- code/data layout;
- profile-guided ordering.

No stage may optimize across a semantic barrier without proof.

## Semantic barrier model

Barrier categories include at least:

- authority;
- safety;
- effect;
- transaction;
- persistence;
- recovery;
- concurrency/order;
- temporal/lifecycle;
- external interface;
- model/provider admission;
- evidence-sensitive behavior where required.

Barrier metadata MUST be typed.

Generic "do not optimize" strings are insufficient.

## Compiler evidence

Each material compiler run MUST be able to produce lineage/evidence containing:

- input AtlasX root;
- compiler/pipeline versions;
- profiles/target/ABI;
- pass pipeline;
- transformations applied;
- diagnostics;
- verification checks;
- backend/toolchain versions;
- output hashes;
- tests/benchmarks where part of admission.

High-impact optimization passes SHOULD provide queryable transformation evidence linking input IR entities to output entities.

## Semantic equivalence

Compiler correctness is scoped to the selected design and declared profiles.

For every stage transition, Atlas must establish equivalence appropriate to the semantics involved.

Evidence may include:

- structural invariant checking;
- differential execution;
- property tests;
- translation validation;
- SMT/proof checks;
- model checking;
- reference backend comparison;
- runtime trace comparison.

No one method is universally mandatory, but high-risk transformations require stronger evidence.

## Delegated Phase 1 backend

Before native HIR/MIR/LIR production is mature, Atlas may lower validated AtlasX to:

- Rust;
- TypeScript;
- bounded C ABI surfaces.

This is a delegated compiler backend.

Rules:

- generated source is a physical projection, not canonical semantic truth;
- source lowering is deterministic for pinned inputs;
- lineage to AtlasX entities is preserved;
- rustc/TS/clang toolchain identities are recorded;
- compile/test/benchmark/recensus closes the loop;
- delegated source generation MUST NOT bypass AtlasX validation;
- delegated output behavior is differential evidence for later native backends.

## External native backends

LLVM, Cranelift, WASM or accelerator backends may consume LIR or another explicitly contracted boundary.

A backend adapter MUST declare:

- accepted IR version;
- supported semantics;
- target coverage;
- ABI coverage;
- unsupported operations;
- verification strategy.

Backend-specific IR MUST NOT become a competing canonical semantic world.

## PGO and auto-tuning

Profiles are evidence.

PGO/auto-tuning may propose:

- layout;
- inlining thresholds;
- specialization;
- code placement;
- vector widths;
- scheduling;
- backend parameter choices.

A tuned candidate must preserve semantic invariants.

Benchmark improvement alone does not permit changing selected design meaning.

If tuning discovers a genuinely superior architectural mechanism, that becomes a BlueprintRevisionDecision under `BLUEPRINT-EVOLUTION.md`, not a hidden compiler optimization.

## Determinism

Given identical:

- AtlasX root;
- compiler version;
- pipeline schema;
- target/ABI;
- profiles;
- backend/toolchain;
- optimization inputs;

the compiler MUST produce equivalent canonical IR stage identities and reproducible output identity under the declared reproducibility policy.

Parallel pass scheduling, host path, locale, clock time and hash-map iteration MUST NOT alter semantic output.

## Diagnostics

Compiler diagnostics MUST identify:

- failing stage;
- semantic identity involved;
- source AtlasX/Atlas lineage where available;
- invariant/contract violated;
- whether failure is unsupported, invalid, unresolved or internal compiler defect.

Do not reduce semantic compiler errors to unstructured strings when a typed diagnostic category exists.

## Incremental compilation

Incremental caches MAY exist per stage.

Cache keys MUST include every semantic/compiler/profile input that affects the cached output.

A cache hit MUST be semantically equivalent to rerunning the stage.

Caches are regenerable and noncanonical.

## Blueprint evolution

Compiler-stage boundaries and mechanisms are blueprints, not immutable dogma.

Donor/dependency census may reveal better:

- IR structures;
- SSA forms;
- ownership representations;
- effect systems;
- instruction-selection methods;
- register allocators;
- layout algorithms;
- linkers;
- verification strategies.

Atlas MAY revise the compiler blueprint when evidence demonstrates improvement.

Revision MUST follow `BLUEPRINT-EVOLUTION.md`.

A selected revision may change stage internals or, with explicit contract migration, stage boundaries. It MUST NOT silently reinterpret old IR or bypass semantic-equivalence obligations.

## Required maturity gates

### HIR mature

- typed schema exists;
- all selected high-level semantics have explicit representation or explicit unsupported diagnostic;
- deterministic serialization/identity;
- AtlasX lineage tests;
- semantic barrier tests.

### MIR mature

- CFG/value/memory/resource semantics explicit;
- deterministic block/value identity;
- failure paths explicit;
- ownership/effect/concurrency/persistence barriers represented;
- verifier catches malformed MIR;
- optimization regression corpus.

### LIR mature

- target/layout/ABI semantics explicit;
- cross-target validation;
- deterministic lowering;
- backend-independent verifier;
- ABI differential tests.

### Machine IR mature

- target instruction semantics defined;
- register/stack/call lowering verified;
- object emission stable;
- debug/unwind evidence;
- differential tests against reference backend.

## Forbidden shortcuts

Forbidden:

- compiling unvalidated arbitrary AtlasX directory files;
- skipping a stage while silently moving its responsibilities elsewhere;
- losing effect/authority/transaction barriers during lowering;
- assuming unknown alias/ownership freedom;
- implicit host ABI;
- implicit target features;
- compiler pass changing selected design semantics without blueprint/design revision;
- benchmark-only acceptance of semantically different output;
- using backend quirks as canonical language/Atlas semantics;
- calling native codegen "Rust parity" without the separate maturity gates.

## Final invariant

The compiler pipeline is a chain of explicit semantic contracts:

~~~text
AtlasX
--validated lowering-->
HIR
--explicit execution lowering-->
MIR
--target/layout/ABI lowering-->
LIR
--instruction lowering-->
Machine IR
--verified emission-->
Product
~~~

Every arrow preserves meaning, lineage and required barriers.

If census later discovers a better compiler mechanism, Atlas changes the blueprint explicitly and revalidates the chain rather than freezing an inferior first design or silently redesigning it in code.
