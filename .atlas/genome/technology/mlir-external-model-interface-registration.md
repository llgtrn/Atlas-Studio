---
id: atlas.genome.technology.mlir-external-model-interface-registration
type: technology-genome
status: active
canonical: true
---
# Technology Genome: type-erased interfaces with decoupled external-model registration (MLIR)

Donor: MLIR (`llvm/llvm-project`, `mlir/` subtree, commit `4790b21292d10b2cab1037ebef100e7f595e4209`),
compiler-IR-design-education lane.

This record follows the mandatory schema in `.atlas/contracts/DONOR-TO-LANGUAGE-GENESIS.md`. It
restructures the existing, exceptionally thorough deep census (`.atlas/census/donors/mlir.md`, 167
lines, itself reading `include/mlir/IR/{Operation,Value,Block,Region,Dialect,SymbolTable,Location}.h`,
`Support/InterfaceSupport.h`, and `docs/Interfaces.md` directly with file/line citations) into the
canonical genome shape, bounded to the single mechanism the census's own text names twice as the
donor's highest-value idea for Atlas ("the single most important MLIR idea for Atlas" -- census line
91). This record re-verifies the core claim directly against primary source in this pass
(`Support/InterfaceSupport.h` lines 1-120, `IR/OpDefinition.h` lines 1770-1787 and 2168) rather than
relying on the existing census's citations alone.

## Capability / problem

Given a core IR data type (an operation, type, or attribute kind) and a cross-cutting capability
("can this be inlined," "can this be folded," "does this implement interface X"), let capability be
attached to the data type **after the fact, from a separate compilation/registration unit that does
not own the data type's own definition** -- without runtime type information built into the data type
itself, without editing the original type's source, and without hitting the kind of orphan-rule-style
coherence restriction Rust's own trait system imposes on implementing a foreign trait for a foreign
type.

## Semantic mechanism (as observed in the donor, evidence re-verified directly in this pass)

- **Concept/Model is a type-erased "external polymorphism" pattern, not virtual inheritance on the
  core IR type itself** (`Support/InterfaceSupport.h` lines 27-96, read directly): `Operation` itself
  has no virtual interface dispatch built in. Instead, each interface (`OpInterface`, `AttrInterface`,
  `TypeInterface`, `DialectInterface`) defines its own `Concept` (an abstract, virtual-method struct)
  and a templated `Model<T>` that implements `Concept` by forwarding to `T`'s own methods --
  confirmed verbatim against the header's own worked example (`ExampleInterfaceTraits` with a
  `Concept`/`Model<DerivedT>` pair, lines 44-56).
- **`ExternalModel<T, U>` is the decoupling mechanism**: `Interface`'s public API exposes
  `ExternalModel` as a template alias alongside `Model` and `Concept` (`InterfaceSupport.h` line 78,
  `using ExternalModel = typename Traits::template ExternalModel<T, U>;`) -- a dialect that does not
  own type `T` can still supply a `Model`-shaped implementation of the interface FOR `T`.
- **Attachment is a runtime, context-scoped registration call, confirmed directly against
  `OpDefinition.h`**: `ConcreteOp::attachInterface<Models...>(MLIRContext &context)` (lines 1775-1787)
  looks up the target operation's `RegisteredOperationName` in the given context and calls
  `info->attachInterface<Models...>()` on it -- a plain static method callable from any translation
  unit that has visibility of the op type and the context, not something that must happen at the op's
  own definition site. Resolution at a use site (`getInterfaceFor`, `OpDefinition.h` line 2168, and
  the class's own accompanying comment "Allow access to `getInterfaceFor`") goes through the
  operation's registered-name interface map, keyed by the interface's own `TypeID`, falling back to
  the owning `Dialect`'s own interface registry when the op itself has none attached.
- **The interface's identity is itself a `TypeID`** (`InterfaceSupport.h` lines 82-87, the nested
  `Trait<ConcreteT>` struct's `getInterfaceID()` returning `TypeID::get<ConcreteType>()`), i.e. the
  registry this mechanism is built on is exactly a `TypeID -> Concept-vtable` map, scoped per
  `MLIRContext` instance -- not a single process-wide table.

## Required invariants

- A `Model`/`ExternalModel`'s implementation must be attachable and resolvable without requiring the
  attaching code to have compile-time access to modify the target type's own source -- this is the
  entire point; a mechanism that required editing the original type would not solve the "orphan"
  problem at all.
- Interface identity (`TypeID`) must be stable and unique per interface across the whole program for a
  given `MLIRContext`, since resolution is a lookup keyed by that ID; two distinct interfaces
  colliding on the same `TypeID` would silently resolve to the wrong `Concept`.
- Registration is explicit (`attachInterface` must actually be called, typically during dialect
  initialization) -- there is no implicit/automatic discovery of `ExternalModel` implementations
  scanning the binary; the donor's own design requires an explicit registration step, not "magic"
  auto-registration.

## Identity/scope model

Directly relevant to a future Atlas dialect/extensibility design (no such system exists in Atlas
today -- confirmed by search, no `Dialect`/`ASIR` type anywhere in `core/src`/`runtime/src`/
`adapter/src`). The mechanism's identity model (an interface identified by its own `TypeID`, resolved
per-value through a registered-name's interface map with dialect-level fallback) is a direct precedent
for whatever `TypeId`-keyed capability lookup a future Atlas op/dialect registry would need --
consistent with `core::identity`'s own existing `stable_id`-based typed-ID conventions, though this
record does not propose a specific Atlas type for it (see Decision).

## State/effect/resource model

The `Concept`/`Model` vtable objects themselves are stateless (the header's own doc comment states
"Both of these classes *must* not contain non-static data," `InterfaceSupport.h` lines 41-43) -- all
real state lives on the `ValueT` (the `Operation*`/`Attribute`/`Type` instance) the interface is
constructed around. The registry mapping `TypeID -> Concept*` is the one piece of mutable state,
scoped per `MLIRContext`, populated by explicit `attachInterface` calls during dialect
initialization.

## Failure and recovery behavior

Not deeply evaluated in this pass; the existing census (line 91) notes `Interface`'s constructor
`assert`s that a value claiming to implement a trait actually resolves to a real `Concept` instance
(`InterfaceSupport.h`'s own `assert((!t || conceptImpl) && "expected value to provide interface
instance")`, confirmed present in the excerpt read directly this pass at lines 96-98/104-106) -- a
debug-build invariant check, not a typed recoverable error; a native Atlas reimplementation should
prefer a typed `Option`/`Result` over an assert for the equivalent "does this value implement this
capability" query, consistent with this session's own established preference for typed, recoverable
outcomes over panics on adversarial or merely-absent data.

## Concurrency/temporal behavior

Not evaluated -- registration happens at dialect-initialization time (effectively single-threaded
setup, per the donor's own dialect-loading model), and the resulting registry is read-only afterward
for the lifetime of the `MLIRContext`; no concurrent-mutation concern was identified in the read
portions of this mechanism.

## Performance characteristics

Not benchmarked. The mechanism trades one indirection (a `TypeID`-keyed map lookup, with a
dialect-level fallback lookup on miss) for O(1)-ish capability dispatch, versus C++ virtual dispatch
built directly into the base class -- not quantified here, out of scope for an architecture-study
pass.

## Portability/ABI constraints

C++-template/RTTI-specific implementation (`TypeID`, CRTP `Trait<ConcreteT>`, template alias
machinery); the *pattern* (a capability keyed by a stable type identifier, resolved through a
context-scoped registry, attachable by any code with visibility of the target type and the registry)
is directly portable to Rust as an explicit `TypeId -> Box<dyn Any>`-or-similar capability registry --
this is exactly the census's own stated conclusion (line 91: "Atlas's own equivalent would likely be
an explicit `TypeId -> Box<dyn Any>`-keyed capability registry rather than attempting compile-time
trait coherence tricks"), which this record adopts unchanged as the concrete Rust-shape recommendation.

## Evidence references

- `.atlas/census/donors/mlir.md` (existing deep census, "Interfaces / Traits Mechanism" section, lines
  86-93, and its Disposition entry, line 142 -- primary prior evidentiary source)
- `.atlas/temporary/donors/mlir/include/mlir/Support/InterfaceSupport.h` (this record directly reads
  lines 1-120, the `Interface` class template and its `Concept`/`Model`/`ExternalModel`/`Trait`
  members, rather than relying on the census's citations alone)
- `.atlas/temporary/donors/mlir/include/mlir/IR/OpDefinition.h` (this record directly reads lines
  1770-1799, `attachInterface`, and confirms `getInterfaceFor` exists at line 2168 as the census
  states)
- Confirmed via `grep` that no `Dialect`/`ASIR`/interface-registry concept exists anywhere in
  `core/src`, `runtime/src`, or `adapter/src` today -- this record captures genome ahead of any
  consumer, consistent with every other `ABSORB_LATER` record this session.

## Donor revisions/licenses

llvm/llvm-project (`mlir/` subtree), commit `4790b21292d10b2cab1037ebef100e7f595e4209`, Apache
License 2.0 WITH LLVM-exception, verified by the existing census directly against
`.atlas/temporary/donors/mlir/LICENSE.TXT`.

## Known trade-offs

- Decoupled external-model registration costs one explicit registration step (`attachInterface` must
  actually be called, typically during dialect init) versus a capability that is simply always present
  on a type by construction -- the donor's own design treats this as worthwhile because it is the
  price of letting a dialect attach behavior to a type it does not own at all, which a
  compile-time-only mechanism (C++ virtual inheritance, or Rust's own coherence-restricted trait impls)
  cannot do for a genuinely foreign type without a wrapper/newtype.
- A `TypeID`-keyed runtime registry is strictly more indirection than a directly-inherited virtual
  method table, and its correctness depends on `TypeID` stability/uniqueness holding -- the donor's
  own invariant (see Required invariants) makes this an explicit design requirement, not an incidental
  detail.

## Rejected alternatives (for this pass)

- MLIR's compile-time CRTP `Trait` mechanism (as opposed to `Interface`) -- explicitly rejected by the
  existing census (Disposition, line 141) as a literal port target: "the C++ template mechanism has no
  clean Rust equivalent at the same call sites," and the underlying need (static, compile-time-checked
  op capabilities) already has a native, better-fitting Rust answer (ordinary Rust `trait` bounds) that
  needs no separate mechanism invented for it. This record does not revisit that conclusion.
- IRDL (dialects-as-data, runtime-loadable dialect/op/type/attr definitions) -- a separate, real,
  independently high-value mechanism the same census names (lines 110-112, 147) as tied with
  Interfaces for highest value; deliberately left out of this record's scope so this genome record
  stays bounded to one coherent mechanism, per this session's established convention. It remains a
  durably queued, real finding in the existing census's own disposition table, a natural candidate for
  a sibling genome record in a future generation.
- Taking any MLIR/LLVM source (headers, `.cpp`, `.td` files) as a dependency, or porting the specific
  memory-layout/pointer-chasing implementation -- already explicitly rejected by the existing census's
  own "Things NOT to Copy" section (lines 152-157); this record adds no new dependency conclusion.

## Dependency/extinction status

No `runtime_dependency_status`/`census_status` change is made by this record. The MLIR donor's own
`donor-corpus.toml` entry (`census_status = "DEEP_CENSUS_ACTIVE"`) is unchanged -- this genome record
covers one mechanism from an already-extensive census; the census's own "Known Risks/Gaps" section
(ODS/TableGen backend, individual trait implementations beyond a handful of examples, the IRDL<->ODS
bridging tools, `DialectConversion.md`, Python/C API bindings, and the Bytecode Reader/Writer
implementation directories) remain genuinely un-censused, so `DEEP_CENSUS_ACTIVE` -- not
`DEEP_CENSUSED` -- remains the honest status. No runtime/build dependency on MLIR/LLVM source exists
today, nor is one proposed by this record.

## Decision

**`ABSORB_LATER`**, native-implementation-only, as an explicit `TypeId -> Box<dyn Any>`-keyed
capability registry (the census's own stated Rust-shape conclusion, adopted unchanged): when Atlas
eventually builds its own dialect/extensibility story for ASIR (the compiler-IR work named across
R7/R8, not yet started -- confirmed by search, zero `Dialect`/`ASIR` code exists anywhere today), this
record's practical output is the specific design precedent to follow: (1) a capability/interface is
identified by a stable type identifier (Atlas already has `stable_id`/typed-ID conventions in
`core::identity` to build this on); (2) attaching an implementation for a type is an explicit runtime
registration call, not automatic discovery; (3) resolution at a use site is a registry lookup keyed by
that identifier, with an explicit fallback tier (MLIR falls back from op-level to dialect-level; a
future Atlas registry would need to decide its own analogous fallback chain, not designed here); (4)
prefer a typed `Option`/`Result` outcome for "does this value implement this capability" over an
assert-style invariant, unlike the donor's own debug-only assert.

Not `ABSORB_NOW`: the real trigger is Atlas's own dialect/extensibility design work, which has not
started -- there is no current ASIR consumer for this mechanism, the same reasoning every other
`ABSORB_LATER` genome record this session has used for its own not-yet-triggered mechanism.

This record does not change `mlir`'s `census_status`; the donor's own census remains honestly
`DEEP_CENSUS_ACTIVE`, with this genome record added as new evidence covering its single
highest-cited mechanism.
