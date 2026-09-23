---
id: atlas.census.donors.mlir
type: donor-census
status: active
canonical: true
---
# MLIR Donor Census

## Source

- Donor: `MLIR` (lane-scoped subtree of `llvm/llvm-project`)
- Remote: `https://github.com/llvm/llvm-project.git`
- Commit: `4790b21292d10b2cab1037ebef100e7f595e4209`
- Clone path: `.atlas/temporary/donors/mlir` (flattened `mlir/` subtree root, not the llvm-project monorepo root)
- License: `Apache-2.0 WITH LLVM-exception`, verified by reading `.atlas/temporary/donors/mlir/LICENSE.TXT` directly (header: "The LLVM Project is under the Apache License v2.0 with LLVM Exceptions"), cross-checked against SPDX headers on individual source files.
- Staging mode: `PARTIAL_SPARSE_SUBTREE_MLIR_ONLY` — partial+sparse clone (`--filter=blob:none --sparse --depth 1`, then `git sparse-checkout set mlir`), NOT a full monorepo checkout. A separate, pre-existing `llvm-project` donor (`.atlas/provenance/donors/llvm-project.json`, full monorepo, pinned at a different commit `0bd330675f9eb08126e467505a0800f167084473`) already exists in this repo for the compiler_backend/codegen lane; this `mlir` donor is independent of it and exists specifically for the ASIR (Atlas Semantic IR) design-education lane. Do not conflate the two — this census only covers the `mlir/` subtree.
- Donor gap: `ATLAS_NATIVE_SEMANTIC_IR_DIALECT_MECHANISM`

## Census State

This is a **purely architecture-education census**. Atlas is not going to depend on MLIR/LLVM as a runtime or build dependency for its own IR (ASIR). The goal is to understand the real shape of MLIR's IR object model, dialect-definition mechanism, and supporting infrastructure closely enough to design a typed, from-scratch Rust equivalent, not to reuse MLIR code or vendor MLIR as a library. All findings below come from reading actual header/source files in the staged tree (not README summaries), citations are file paths under `.atlas/temporary/donors/mlir/`.

## Core IR Object Model

Read from `include/mlir/IR/{Operation,Value,Block,Region,BlockSupport,Types,Attributes,Dialect,SymbolTable,Location}.h` and `include/mlir/IR/OperationSupport.h`.

### Operation (`include/mlir/IR/Operation.h`)

`Operation` is `final`, heap-allocated only (never stack/value type), and inherits `llvm::ilist_node_with_parent<Operation, Block>` plus `llvm::TrailingObjects<Operation, OperandStorage, OpProperties, BlockOperand, Region, OpOperand>`. Its actual private fields (from the class body, `Operation.h:1097-1128`):

```
Block *block = nullptr;
Location location;
mutable unsigned orderIndex = 0;
const unsigned numResults;
const unsigned numSuccs;
const unsigned numRegions : 23;   // bitfield
bool hasOperandStorage : 1;       // bitfield
unsigned char propertiesStorageSize : 8;
OperationName name;
DictionaryAttr attrs;
```

Key structural invariants actually observed:
- **Results are stored *before* the `Operation*` in memory** (reverse order), as trailing objects distinct from the operand/region/successor trailing arrays — `Operation*` always points at the fixed offset after the result array. This is a deliberate single-allocation layout trick for cache locality that has no direct Rust analog (Rust would use a normal owned `Vec`/arena index rather than manual trailing-object layout).
- Up to 5 results are packed inline into the pointer-sized `ValueImpl` (`InlineOpResult`); beyond that, `OutOfLineOpResult` is used, trading a few bits of packing for a fallback path. This is a memory-micro-optimization not worth porting into Rust as-is; a plain `SmallVec`-style small-vector-of-results is the natural Rust equivalent.
- Operands are optionally tail-allocated (`OperandStorage`) but can move to a dynamic (heap) allocation when resized — a mutable/resizable operand list is explicitly a non-default-path optimization.
- **Properties**: a per-operation, dialect-owned, fixed-size, non-Attribute-backed C++ object (`OpProperties` trailing storage of up to `256*8` bytes) distinguished from the always-present generic `DictionaryAttr attrs`. Properties are Attribute-convertible on demand but are the actual canonical storage for inherent operation data (e.g. segment-size arrays) — this two-tier storage (typed Properties vs generic string-keyed Attribute dictionary) is directly relevant to Atlas: an ASIR op needs both a fast typed-field path and a generic/introspectable attribute-bag path.
- `Region[]` and `BlockOperand[]` (for successors, i.e. branch targets) are also trailing-allocated arrays sized at construction.

### Value / OpResult / OpOperand (`include/mlir/IR/Value.h`, `OperationSupport.h`)

`Value` is a thin value-semantic wrapper (single pointer, `detail::ValueImpl *impl`) around one of two backing kinds: `OpResult` (owned/allocated by the defining `Operation`, as above) or `BlockArgument` (owned by a `Block`). `ValueImpl` stores `llvm::PointerIntPair<Type, 3, Kind> typeAndKind` — i.e. the Value's *only* stored data is its `Type` plus a 3-bit kind tag; the use-list (`IRObjectWithUseList<OpOperand>`) is inherited separately. `OpOperand` is the actual use record: an intrusive doubly-linked-list node pointing back to its owning `Operation` and forward/back to sibling uses of the same `Value`. This use-list-as-intrusive-linked-list is the backbone of MLIR's O(1) "replace all uses" and def-use walking — a classic SSA-IR design choice. In Rust, the natural equivalent is an arena of `Value`s each holding a `Vec<UseId>` or a generational-index-based use-list rather than an intrusive pointer list, since intrusive linked lists fight the borrow checker; the *design goal* (cheap def-use enumeration, cheap RAUW) is the real donor concept, not the pointer plumbing.

### Block (`include/mlir/IR/Block.h`)

Fields (`Block.h:421-433`):
```
llvm::PointerIntPair<Region *, 1, bool> parentValidOpOrderPair;
unsigned blockID = -1u;             // stable ID within parent region, reassigned on move
OpListType operations;              // intrusive linked list of Operation
std::vector<BlockArgument> arguments;
```
A `Block` is an ordered list of `Operation`s (via `llvm::ilist`) plus its own SSA block arguments (used for both function-entry parameters and structured-control-flow "phi" values — MLIR has no separate phi-node concept; block arguments subsume it). `getBlockID()` is explicitly O(1) and stable (not reused) specifically so generic graph algorithms (dominance, loop info) can index blocks cheaply — a good concrete pattern for Atlas: a stable per-block dense ID assigned at insertion, independent of any print-order numbering.

### Region (`include/mlir/IR/Region.h`)

Fields (`Region.h:342-349`): `BlockListType blocks` (intrusive list of `Block`), `Operation *container` (owning op, may be null), `unsigned nextBlockID` (monotonic counter feeding `Block::blockID`). A `Region` is a list of `Block`s, i.e. the standard "list-of-lists-of-instructions" CFG shape, nested arbitrarily inside any `Operation` (regions are how MLIR expresses structured control flow, function bodies, and nested scopes uniformly, rather than a single flat function-of-basic-blocks model like LLVM IR).

### Type / Attribute (`include/mlir/IR/Types.h`, `Attributes.h`)

Both `Type` and `Attribute` are value-semantic, immutable, uniqued, interned handles into an `MLIRContext`-owned storage arena — thin wrappers around a pointer to a `TypeStorage`/`AttributeStorage` object keyed by a `TypeID` (RTTI-like tag) plus storage-specific "key" data, hash-consed so structurally-equal types/attributes are pointer-equal. This uniquing-table design (parametric, hash-consed, context-owned interning) is one of the most directly portable ideas to Rust: an arena/interner keyed by `(TypeId, key-hash)` returning a cheap `Copy` handle is very natural in Rust (e.g. via `string-interner`-style or a custom `Interner<T>`).

### Dialect (`include/mlir/IR/Dialect.h`)

A `Dialect` is a named (`StringRef name` namespace), per-`MLIRContext`-instance (`MLIRContext *context`) registry object identified by a `TypeID dialectID`. It owns: the set of `Operation`/`Type`/`Attribute` kinds it defines (registered via `addOperations<...>()`/`addTypes<...>()`/`addAttributes<...>()` in its `initialize()`), an `allowUnknownOperations`/`allowUnknownTypes` policy flag, optional `DialectInterface`s (see Interfaces below), and hooks for parsing/printing/materializing constants and for participating in canonicalization/folding at the dialect level. Loading dialects into a context is an explicit, opt-in step — unregistered dialects/ops are representable but degrade to "opaque" generic-syntax-only handling. This "explicit registry object per namespace, instantiated once per context" pattern maps directly onto an Atlas `Dialect` trait/registry keyed by a namespace string plus a `TypeId`-equivalent.

### Symbol / SymbolTable (`include/mlir/IR/SymbolTable.h`, `SymbolInterfaces.td`)

`Symbol` is an *operation interface* (see Interfaces below), not a separate base class — any op can opt into being a named, referenceable entity (functions, globals) by implementing the `Symbol` interface (providing a `StringAttr` name and reference-counting/rename hooks). `SymbolTable` is a side-table (not intrusive) built lazily over a region's top-level ops that have the `Symbol` trait, providing name uniquing and symbol-use-walking (including verifying that `SymbolRefAttr`-typed attributes on other ops resolve). This "symbols are just an interface any op can implement, plus a derived index" pattern is a strong candidate for Atlas: it avoids a hard-wired "declaration" op kind and instead makes name-ability a composable capability.

### Location (`include/mlir/IR/Location.h`)

`Location` is itself just an `Attribute` (a `LocationAttr` subclass) — i.e. provenance/source-position tracking is not a separate side-channel, it is interned/uniqued exactly like any other attribute and is *mandatory* on every `Operation` (there is always a `Location`, defaulting to `UnknownLoc` — never optional/nullable). Composite location kinds exist (`FusedLoc`, `CallSiteLoc`, `NameLoc`) allowing a single `Operation` to carry a location that is itself a small tree recording e.g. an inlining call chain. Making location a first-class, always-present, uniqued, composable Attribute (rather than an optional debug side-table) is directly relevant to Atlas provenance tracking, which already has a strong "everything has provenance" ethos.

## Interfaces / Traits Mechanism

Two distinct, deliberately different mechanisms coexist (`include/mlir/IR/OpDefinition.h`, `include/mlir/Support/InterfaceSupport.h`, `docs/Interfaces.md`):

- **Traits** (`OpDefinition.h:378` `TraitBase<ConcreteType, TraitType>`): plain CRTP mixins. A trait is a template class the concrete `Op<...>` type multiply-inherits from at compile time (e.g. `ZeroOperands`, `OneOperand`, `OpInvariants`). Traits can contribute a static `verifyTrait(Operation*)` hook that the verifier infrastructure calls. This is compile-time, zero-cost, but *closed*: a trait must be known and inherited at the point the op type is defined; it cannot be retrofitted onto an existing op type from elsewhere.
- **Interfaces** (`Support/InterfaceSupport.h:68` `class Interface`, `docs/Interfaces.md`): a **type-erased Concept/Model ("external polymorphism") pattern**, not virtual inheritance on `Operation` itself (which has none). Each interface (`OpInterface`, `AttrInterface`, `TypeInterface`, `DialectInterface`) defines a `Concept` (an abstract vtable-shaped struct) and a templated `Model<T>` (implements the `Concept` by dispatching to `T`'s own methods). A concrete op/type/attribute either provides a `Model` at definition time, **or** — critically — a dialect can register an `ExternalModel<T,U>` for a type it does not own, entirely decoupling "does X implement interface Y" from X's own source location. Resolution at a call site (`OpInterface::getInterfaceFor`, `OpDefinition.h:2168`) goes through the operation's `RegisteredOperationName`'s interface map (keyed by the interface's own `TypeID`), falling back to the owning `Dialect`'s interface registry. **This external-model / decoupled-registration mechanism is the single most important MLIR idea for Atlas**: it is effectively a runtime, TypeID-keyed vtable registry that lets behavior be attached to a data type from a completely separate compilation/registration unit — solving in C++ (without Rust's orphan rule at all, since there is no such rule) the same problem Rust's orphan rule makes awkward. Atlas's own equivalent would likely be an explicit `TypeId -> Box<dyn Any>`-keyed capability registry rather than attempting compile-time trait coherence tricks.
- Dialect-level interfaces (`DialectInterfaceBase::Base<>`, CRTP) are the same Concept/Model idea but scoped to an entire dialect rather than one op, used for cross-cutting concerns like inlining legality/cost that shouldn't be implemented per-op.

## Verifier Infrastructure

`lib/IR/Verifier.cpp` implements an `OperationVerifier` that walks the IR (optionally recursively into regions, `verifyOpAndDominance`) checking: op-level invariants generated from ODS (`OpInvariants::verifyTrait`, itself calling the op's generated `verifyInvariantsImpl()`), then each attached trait's static `verifyTrait`, then each attached interface's `verifyTrait` if present, then dominance (`Dominance.h`) for SSA use-before-def violations, then dialect-supplied custom verification hooks. Verification is explicitly **not** run continuously/automatically on every mutation — MLIR *allows* transformations to pass through transiently-invalid IR states (e.g., unlinking an op before reinserting) and only requires validity at defined checkpoints (parse, before/after passes in debug builds, explicit `verify()` calls). This "verification is a checkpoint operation, not an always-on invariant" design is a deliberate performance/ergonomics tradeoff worth carrying into Atlas: mutation-heavy IR construction/rewriting needs a way to be transiently invalid.

## Rewrite / Pattern-Matching Infrastructure

`docs/PatternRewriter.md`, `lib/IR/PatternMatch.cpp`. A `RewritePattern` declares a static `PatternBenefit` (cost, resolved statically so patterns can be compiled into an efficient dispatch/state-machine rather than dynamically re-costed per application) and, usually, a specific root operation name it matches against (or an explicit "any op" opt-in tag). `matchAndRewrite` must not mutate IR before the match is confirmed successful (match/rewrite are conceptually — though not always physically — separated). The `PatternRewriter` is the only sanctioned mutation surface during a rewrite (it wraps `OpBuilder` with additional bookkeeping/listener hooks so a driver can track what changed). The **greedy pattern rewrite driver** applies the full pattern set to a worklist of ops repeatedly until fixpoint or an iteration cap, which is exactly what the canonicalizer (below) runs. This benefit-cost-based, worklist/fixpoint-driven rewrite engine is a strong conceptual donor for an Atlas rewrite pass, though the actual scheduling data structure (worklist + dirty-marking on mutation) is a well-known, independently-inventable design, not MLIR-specific IP.

## Pass Infrastructure

`include/mlir/Pass/Pass.h`, `PassManager.h`. `Pass` is an abstract base (`TypeID passID`, virtual `getName()`, `getDependentDialects(DialectRegistry&)` so a pass declares up front which dialects it may newly introduce ops/types/attrs from, `getArgument()`/`getDescription()` for CLI registration, an optional fixed `opName` restricting which operation type the pass runs on). `OpPassManager` groups passes into a pipeline scoped to a specific operation-type "anchor" (nested pass managers can be scoped to nested regions/ops, e.g. running a function-level pipeline inside a module-level one) with `PreservedAnalyses` tracking to avoid invalidating/recomputing analyses a pass didn't touch. `PassManager` is the top-level driver. The "declare which dialects you may introduce, run scoped to an operation-type anchor, track preserved analyses" trio is the reusable conceptual core; Atlas's own pass/pipeline design should study this shape rather than a flat unordered pass list.

## ODS (TableGen Operation Definition Specification)

`docs/DefiningDialects/Operations.md` states the motivation directly: without ODS, MLIR's "stringly typed IR" (repetitive string comparisons, error-prone positional `getOperand(3)` accessors, verbose constructors, verbose textual dumps) becomes unmanageable at scale. ODS lets an operation's shape (operands, results, attributes, regions, successors, traits, verifier, printer/parser, canonicalization patterns, builders) be declared once in TableGen syntax and expanded at **build time** into a C++ `mlir::Op<...>` specialization plus generated accessors, verifier, and (optionally) parser/printer. This is a compile-time code-generation solution to the same problem IRDL and xDSL's Python IRDL solve at *runtime* — see below. **Atlas does not need or want a TableGen-equivalent build step**: a Rust proc-macro or a runtime-introspectable typed-dialect-registration API (much closer to IRDL/xDSL than to ODS) is the right target. ODS itself was not deep-censused beyond its stated purpose and observed `.td` file shapes (`include/mlir/IR/OpBase.td`, `CommonAttrConstraints.td` etc.) — the TableGen backend implementation itself (`mlir-tblgen`) was not read.

## IRDL (highest-value section for Atlas's own dialect-definition design)

`lib/Dialect/IRDL/{IR/IRDL.cpp,IRDLLoading.cpp,IRDLVerifiers.cpp,IRDLSymbols.cpp}`, `docs/Dialects/IRDL.md`, `test/Dialect/IRDL/*.irdl.mlir`. IRDL is MLIR's own **dialect defined as an MLIR dialect** for declaring *other* dialects, operations, types, and attributes **as ordinary MLIR IR**, then loading them into a live `MLIRContext` **at runtime** via `IRDLLoading.cpp` — no TableGen, no C++ compilation step. Concretely (read from `test/Dialect/IRDL/cmath.irdl.mlir`): an `irdl.dialect @cmath { ... }` op contains `irdl.type @complex { ... }` and `irdl.operation @norm { ... }` ops; inside those, a small constraint-expression sublanguage (`irdl.is`, `irdl.any_of`, `irdl.any`, `irdl.parametric @cmath::@complex<...>`) builds up an operand/result/parameter type-constraint graph, terminated by `irdl.operands(...)`/`irdl.results(...)`/`irdl.parameters(...)`. `IRDLLoading.cpp` walks this IR and dynamically registers a real `ExtensibleDialect` (`IR/ExtensibleDialect.h`) with runtime-constructed op/type/attr descriptors backed by a `ConstraintVerifier` built from the declared constraint graph (`irdlAttrOrTypeVerifier`, `IRDLLoading.cpp:29`), including variadic-operand segment-size handling (`getSegmentSizesFromAttr`) equivalent to ODS's `AttrSizedOperandSegments`. **This is the closest existing prior art to what Atlas needs**: a dialect definition mechanism that is itself data (IR, or a Rust-native equivalent serializable structure), loadable and introspectable at runtime without a separate code-generation/build pass, with the same operand/result/attribute-constraint expressiveness ODS gets from TableGen. `mlir-irdl-to-cpp`/`tblgen-to-irdl` tools (bridging IRDL and ODS/TableGen) exist but were not deep-censused — only their existence and stated purpose were observed.

## Bytecode Format and Dialect Extension Mechanism

`docs/BytecodeFormat.md`, `include/mlir/Bytecode/{BytecodeImplementation.h,BytecodeReader.h,BytecodeWriter.h,Encoding.h}`, `lib/Bytecode/`. Key facts read directly from the format spec:
- Magic number `MLïR` (bytes `4D 4C EF 52`), explicitly versioned, with a documented compatibility promise: **older bytecode must always remain readable by newer tooling**, and "back-deployment" (reading *newer*-than-tool bytecode where possible) is also supported.
- Integers are encoded with a "PrefixVarInt" LEB128 variant: the number of leading zero bits in the first byte's low bits encodes how many additional bytes follow, giving 1-byte encoding for values under 2^7 up to 9-byte encoding for full 64-bit values — a compact, alignment-free variable-width integer scheme, directly reusable as a design pattern for an Atlas bytecode/serialization format.
- **Dialects own their own versioning**: a dialect opts in via `BytecodeDialectInterface`, which exposes hooks to read/write a dialect-specific version blob into the bytecode file and an `upgradeFromVersion` hook invoked lazily during parsing so a dialect can upgrade its own encoded IR forward, post-parse, when reading older bytecode. There is no global schema-version gate — versioning is per-dialect and additive. This is the key mechanism note for Atlas: bytecode forward/backward compatibility is explicitly **not** solved by a single monolithic format version, but by each dialect being individually responsible for its own wire-format evolution, discovered and dispatched through the same dialect-registry mechanism used everywhere else in MLIR (Dialects, Interfaces). A comparably-scoped per-dialect (per-namespace) versioning hook is a strong, concrete idea to carry into ASIR's own binary serialization design.

## Canonicalization Infrastructure

`docs/Canonicalization.md`. A single canonicalization *pass* (not per-dialect passes) that iteratively applies canonicalization patterns **registered by every loaded dialect on its own operations** in a greedy, best-effort, non-fixpoint-guaranteed way (bounded by a max-iteration option to avoid infinite loops from a faulty pattern). Explicitly documented design stance: canonicalization exists to make *later* analyses/transforms simpler and more effective, not for its own performance win, and pass pipelines must not rely on it for *correctness* (it can legally be skipped/bounded). This "one shared driver, patterns supplied per-op via the same rewrite-pattern registration surface used everywhere else" design (rather than a bespoke canonicalization-specific mechanism) is the reusable idea — canonicalization is just "the patterns every op author registers as canonical, run through the generic greedy rewrite driver."

## Parsing / Printing / Textual vs In-Memory vs Bytecode

Three representations of the same IR are explicitly distinguished in this tree: (1) the in-memory `Operation`/`Value`/`Block`/`Region` graph described above; (2) a generic + optionally custom-per-op **textual assembly format** (`OpAsmParser`/`OpAsmPrinter`, `include/mlir/IR/OpImplementation.h`, `AsmPrinter.cpp`/parser under `lib/AsmParser` outside the staged `mlir/lib/IR`) which every op supports at minimum generically (`opname(...) : (types) -> (types) attrs`) and may customize; (3) the versioned binary bytecode format above. Parsing and printing are strictly separated interfaces (`OpAsmParser` vs `OpAsmPrinter`) rather than one bidirectional codec object, and both are driven generically off the same op-name/attribute/operand/region structure the in-memory model exposes — i.e. there is no separate "AST" layer between text and the in-memory IR; the parser builds `Operation`s directly. This three-representation split (in-memory graph / human-authorable text / compact versioned binary), with the in-memory graph as the single source of truth that both text and bytecode serialize from and deserialize to, is a directly reusable top-level architecture shape for ASIR.

## Location / Provenance Tracking

Already covered under Core IR Object Model — `Location` is a mandatory, uniqued, composable `Attribute`, not an optional side-channel. Worth restating here because it's a cross-cutting concept: because `Location` is just another interned `Attribute`, it participates in bytecode serialization, printing, and structural equality/hash-consing for free, with no special-cased plumbing anywhere else in the IR. Atlas provenance tracking should take the "provenance is a first-class, uniqued, always-present value, not a bolted-on side-table" lesson directly.

## Disposition (per concept)

- **Operation/Value/Block/Region memory layout (trailing-objects, inline-result packing, intrusive ilists)**: REJECT as implementation, STUDY as design rationale. C++-specific memory tricks; Rust should use arenas/`Vec`/generational indices for the equivalent structural relationships (op has N results/operands/regions/successors; block is op list + arg list; region is block list).
- **Value/OpOperand def-use tracking via intrusive linked use-lists**: STUDY the goal (O(1) RAUW, cheap def-use walk), REIMPLEMENT via an arena-friendly Rust structure (e.g. per-value `Vec<UseHandle>` or a generational slot map), not the pointer-chasing mechanism itself.
- **Type/Attribute interning in a context-owned uniquing table**: ADAPT directly — a `TypeId`/hash-keyed interner returning cheap `Copy` handles is very natural in Rust and should be a core ASIR primitive.
- **Dialect as a namespaced, per-context registry object owning op/type/attr kinds**: ADAPT — matches Atlas's own need for a `Dialect` trait/registry keyed by namespace.
- **Symbol as an interface, not a base class, plus a derived SymbolTable index**: ADAPT — avoids hard-wiring a "declaration" concept into the core op model; strong fit for ASIR.
- **Location as a first-class, mandatory, uniqued, composable Attribute**: ADAPT directly into ASIR provenance.
- **Traits (CRTP mixins, compile-time, closed)**: REJECT as a literal port (C++ template mechanism has no clean Rust equivalent at the same call sites); the *idea* of static, compile-time-checked op capabilities maps better to Rust's own trait system directly (a real `trait` bound), so no separate "trait mechanism" needs inventing — REIMPLEMENT-not-needed, use native Rust traits.
- **Interfaces (type-erased Concept/Model, external-model registration decoupled from the type's own definition site)**: ADAPT — this is the highest-value single idea in the whole donor for Atlas's extensibility story. Design a `TypeId`-keyed capability registry (external-model pattern) so dialect authors, including ones compiled/loaded after core ASIR, can attach behavior to op/type/attribute kinds they don't own.
- **Verifier as a checkpoint operation (not always-on), trait+interface+dialect-hook composition**: ADAPT — design ASIR verification as an explicit, composable, checkpoint-triggered pass over trait/interface-contributed checks, and allow IR to be transiently invalid during construction/rewriting.
- **Greedy worklist/fixpoint pattern-rewrite driver with static pattern benefit**: STUDY/ADAPT — well-understood, independently-inventable rewrite-engine shape; worth deliberately re-deriving in Rust rather than copying MLIR's specific driver code.
- **Pass declares dependent dialects + operation-type-scoped nested pass managers + preserved-analyses tracking**: ADAPT — concrete, reusable pipeline shape for an Atlas pass manager.
- **ODS / TableGen compile-time codegen**: REJECT as a mechanism (Atlas has no reason to add a TableGen-equivalent build step); STUDY only the *problem statement* (avoid stringly-typed, positional, unverified op access) which IRDL/xDSL's runtime-typed approach and/or a Rust proc-macro should solve instead.
- **IRDL (dialects-as-data, runtime-loadable dialect/op/type/attr definitions with a constraint sublanguage)**: ADAPT — the primary architectural reference for Atlas's own typed, runtime-introspectable dialect-registration API. Highest-value section of this census alongside Interfaces and Bytecode.
- **Bytecode wire format (varint scheme, magic number, per-dialect opt-in versioning + upgrade hooks)**: ADAPT — the per-dialect versioning/upgrade-hook mechanism and the compact varint encoding are both concretely reusable design patterns for an ASIR binary serialization format.
- **Canonicalization as "patterns registered per-op, run through the shared greedy driver," not a bespoke mechanism**: ADAPT — avoid building a second, parallel rewrite mechanism just for canonicalization.
- **Textual / in-memory / bytecode three-representation split with in-memory graph as source of truth, strict parser/printer separation**: ADAPT — directly reusable top-level shape for ASIR's own text format, in-memory graph, and binary format.

## Things NOT to Copy

- Never copy any MLIR/LLVM C++ source (headers, `.cpp`, `.td` TableGen files) verbatim into Atlas, even though Apache-2.0-WITH-LLVM-exception is a permissive, Atlas-license-compatible license. This census is a clean-room architecture study: study the idea, then write Atlas's own Rust implementation from scratch against Atlas's own requirements.
- Do not port MLIR's specific memory-layout tricks (trailing-objects allocation, inline-result packing, intrusive `ilist`) as literal Rust code; they are C++-idiom-specific and fight Rust's ownership model. Reimplement the *structural relationships* they encode, not the pointer arithmetic.
- Do not adopt TableGen or any external code-generation build step as Atlas's dialect-definition mechanism; IRDL and xDSL both show a runtime/data-driven alternative is viable and is the better fit for Atlas.
- Do not treat MLIR's specific bytecode byte-for-byte encoding as something Atlas must be binary-compatible with; only the *design patterns* (varint scheme, per-dialect versioning hook) are donor material, not the encoding itself.

## Known Risks / Gaps in This Census

- Did not census MLIR's ODS/TableGen backend implementation (`mlir-tblgen`) itself — only its stated purpose (`docs/DefiningDialects/Operations.md`) and the shape of representative `.td` files were read. No `.td`-to-C++ codegen logic was read.
- Did not census the `Traits/` doc directory or individual trait implementations beyond `OpDefinition.h`'s `TraitBase`/a handful of example traits (`ZeroOperands`, `OneOperand`, `OpInvariants`).
- Did not census `mlir-irdl-to-cpp`/`tblgen-to-irdl` (the IRDL<->ODS bridging tools) beyond noting their existence and file locations — only `IRDLLoading.cpp` (the runtime-loading path, the part actually relevant to Atlas) was read in depth.
- Did not census `DialectConversion.md` / the dialect conversion framework (legalization, type conversion across dialects during lowering) at all — this is a plausible follow-up census topic for Atlas's own dialect-lowering story, flagged here as a gap, not covered above.
- Did not census the Python bindings (`python/`) or the C API (`lib/CAPI`) surfaces.
- Did not run or build any MLIR code — this census is 100% static reading, per the donor-workbench isolation contract; some runtime behavior claims above (e.g. exact greedy-driver fixpoint semantics) are taken from `docs/*.md` design documentation cross-checked against the matching source files, not from execution traces.
- Bytecode `Reader`/`Writer` implementation directories (`lib/Bytecode/Reader`, `lib/Bytecode/Writer`) were located but not read line-by-line; findings on the wire format come from `docs/BytecodeFormat.md` plus the interface header `BytecodeImplementation.h`, not the full reader/writer implementation.
