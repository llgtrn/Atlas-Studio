---
id: atlas.census.donors.xdsl
type: donor-census
status: active
canonical: true
---
# xDSL Donor Census

## Source

- Donor: `xDSL`
- Remote: `https://github.com/xdslproject/xdsl.git`
- Commit: `00703508e37a2b4ef06687b3bbbe4cd78c41c740`
- Clone path: `.atlas/temporary/donors/xdsl`
- License: `Apache-2.0 WITH LLVM-exception`, verified by reading `.atlas/temporary/donors/xdsl/LICENSE` directly (header: "The xDSL project is under the Apache License v2.0 with LLVM Exceptions"), cross-checked against `pyproject.toml`'s `project.license = { text = "Apache License v2.0 with LLVM Exceptions" }`.
- Staging mode: `FULL_SOURCE_TREE` (normal-sized pure-Python repo, full shallow clone).
- Donor gap: `ATLAS_NATIVE_SEMANTIC_IR_DIALECT_MECHANISM`

## Census State

Purely an architecture-education census, staged and read alongside the `mlir` donor (`.atlas/census/donors/mlir.md`). xDSL is a **from-scratch pure-Python reimplementation** of MLIR's IR/dialect/pattern-rewrite model — much smaller (~1400 files, ~13.7MB vs MLIR's ~7800 files/78.5MB `mlir/` subtree alone) and, because Python has no separate TableGen/codegen step, its "ODS equivalent" (IRDL, `xdsl/irdl/`) is plain, readable, runtime-executed Python rather than a build-time C++ generator. This makes xDSL structurally the closer analog to what a from-scratch Rust ASIR implementation would look like: typed data classes with runtime-registered capabilities, no separate code-generation pass required to get a working dialect. All findings below are from reading actual source under `xdsl/` in the staged tree, not documentation summaries.

## Core IR Object Model

Read from `xdsl/ir/core.py` (2653 lines total).

### Operation (`xdsl/ir/core.py:849`)

`Operation` is a plain `@dataclass(eq=False, unsafe_hash=False, repr=False)`, not a hand-rolled memory layout. Its actual fields:

```python
name: ClassVar[str]
_operands: SSAValues = field(default=SSAValues())
_operand_uses: tuple[Use, ...] = field(default=())
results: SSAValues[OpResult] = field(default=SSAValues())
_successors: tuple[Block, ...] = field(default=())
_successor_uses: tuple[Use, ...] = field(default=())
properties: dict[str, Attribute] = field(default_factory=dict)
attributes: dict[str, Attribute] = field(default_factory=dict)
location: LocationAttr = field(default_factory=_unknown_loc)
regions: tuple[Region, ...] = field(default=())
parent: Block | None = field(default=None)
_next_op: Operation | None = field(default=None)
_prev_op: Operation | None = field(default=None)
traits: ClassVar[OpTraits]
```

Notable design choices vs. MLIR's C++ `Operation` (`.atlas/temporary/donors/mlir/include/mlir/IR/Operation.h`), all directly relevant since Atlas is a from-scratch Rust implementation, not a C++ one:
- No trailing-objects/single-allocation memory trick — operands/results/regions/successors are ordinary Python tuples/dataclass fields. This confirms MLIR's memory layout is a C++-specific optimization, not an inherent requirement of the IR *model*; a Rust `Operation` struct with `Vec`/`SmallVec`/`Box<[T]>` fields for operands/results/regions is the equivalent-fidelity, idiomatic choice.
- **Same two-tier `properties` vs `attributes` split as MLIR** (both present as separate dict fields), confirming this is a load-bearing IR-model idea (inherent typed/fast data vs. generic discardable metadata), not an MLIR C++ implementation detail — strengthens the case for ADAPT on this specific idea.
- Sibling linkage (`_next_op`/`_prev_op`) is an **explicit doubly-linked list of plain optional references**, not an intrusive `llvm::ilist` template. `_insert_next_op`/`_insert_prev_op` manually splice these pointers. This is a much more direct model for what a Rust arena-based op list would look like (e.g. `Option<OpId>` next/prev fields into a slab/arena) than MLIR's intrusive-list C++ idiom.
- `location: LocationAttr` is mandatory with a default factory (`_unknown_loc`) — same "location is always present, never optional" design as MLIR, independently confirming this is a genuine cross-implementation IR-design consensus worth adopting in ASIR, not an MLIR-only quirk.
- `traits: ClassVar[OpTraits]` — traits are a **class-level, not instance-level**, iterable collection (see Traits/Interfaces below), populated by the IRDL definition machinery.

### SSAValue / OpResult / BlockArgument (`xdsl/ir/core.py:538-694`)

`SSAValue` is an abstract dataclass (`@dataclass(eq=False)`, `ABC`, `Generic[AttributeCovT]`) mixing in `IRWithUses` (owns an `IRUses` collection of `Use` records, `core.py:344-414`) and `IRWithName`. `OpResult(SSAValue)` adds `op: Operation` (owner) and `index: int`; `BlockArgument(SSAValue)` adds `block: Block` and `index: int`. `Use` is a plain `@dataclass(repr=False)` record `(operation: Operation, index: int)` stored in an ordinary Python list-backed `IRUses`/`OpOperands` collection (`core.py:764` `OpOperands(Sequence[SSAValue])`) rather than an intrusive linked list — the def-use relationship is modeled as **ordinary collections you can enumerate and mutate directly**, not a pointer-chasing structure. This is direct, concrete evidence that MLIR's intrusive-use-list is an optimization choice, not a semantic requirement — Atlas's own def-use tracking can use a plain `Vec<Use>` per value (or a secondary index) with no loss of IR-model fidelity.

### Block / Region (`xdsl/ir/core.py:1570`, `2159`)

`Block` and `Region` are both dataclasses (`_IRNode` subclasses) holding, respectively, an ordered op sequence (exposed via `BlockOps`, a custom `Reversible[Operation]` wrapper over the `_next_op`/`_prev_op` chain — same "doubly linked list of plain refs" pattern as `Operation` siblings) plus block arguments, and an ordered `RegionBlocks` sequence. Structurally identical model to MLIR (region = list of blocks, block = op list + block args), independently confirming this shape as the real donor idea rather than an MLIR-only design.

### Dialect (`xdsl/ir/core.py:47`)

`Dialect` is a plain `@dataclass` holding `_name: str`, `_operations: list[type[Operation]]`, `_attributes: list[type[Attribute]]`, `_interfaces: list[DialectInterface]`, with `get_interface(interface_type)`/`has_interface(...)` doing a linear `isinstance` scan over the small `_interfaces` list. Much simpler than MLIR's `TypeID`-keyed interface maps — acceptable in xDSL because dialects have at most a handful of dialect-level interfaces, and directly shows that a naive linear-scan registry is a legitimate, simpler starting point for Atlas before any TypeID-map optimization is needed.

### Attribute / ParametrizedAttribute / Data (`xdsl/ir/core.py:102-330`)

`Attribute` is an `ABC` `@dataclass(frozen=True)` with a `verify()` hook called from `__post_init__` (i.e. **attributes self-verify at construction time**, unlike MLIR where attribute "verification" is more diffuse) and a hard constraint enforced in `__post_init__`: every `Attribute` must be either `Data` (an opaque wrapped Python value, e.g. wrapping an `int`/`str`) or `ParametrizedAttribute` (a named attribute with typed sub-attribute parameters) — a clean, closed two-way split that MLIR's C++ `Attribute`/`AttributeStorage` hierarchy expresses far less legibly. `BuiltinAttribute` is a separate marker ABC for attributes the parser/printer must special-case built-in syntax for. This frozen-dataclass-with-self-verifying-`__post_init__` pattern is a strong concrete template for Atlas: an `Attribute` in Rust as an enum or trait-object with a `verify(&self) -> Result<(), VerifyError>` called at construction.

## IRDL (xDSL's declarative operation-definition system — directly usable, unlike MLIR's TableGen)

`xdsl/irdl/operations.py` (defines `IRDLOperation`, `operand_def`/`result_def`/`prop_def`/`attr_def`/`region_def`/`successor_def` field-declaration helpers, and option classes like `AttrSizedOperandSegments`) plus `xdsl/irdl/attributes.py` and `xdsl/irdl/constraints.py`. Concrete usage pattern observed in `xdsl/dialects/arith.py`:

```python
@irdl_op_definition
class ConstantOp(IRDLOperation, HasFolderInterface):
    result = result_def(_T)
    ...

@irdl_op_definition
class AddOp(...):
    lhs = operand_def(T)
    rhs = operand_def(T)
    result = result_def(T)
```

`@irdl_op_definition` is a class decorator that inspects the class body's `operand_def(...)`/`result_def(...)`/`prop_def(...)`/`region_def(...)` class attributes (each producing an `OperandDef`/`ResultDef`/`PropertyDef`/`RegionDef` marker instance, `xdsl/irdl/operations.py:323-472`) plus Python type-hint-driven generic constraints (`T`, `_T` type vars bound to `AttrConstraint`s), and synthesizes `IRDLOperation.__init__`/`build`/`get_irdl_definition` machinery (`operations.py:86-205`) — an `OpDef` schema object introspectable at runtime (`get_irdl_definition() -> OpDef`). Variadic operand/result/region/successor handling mirrors MLIR's ODS exactly in vocabulary (`VarOperandDef`, `OptOperandDef`, `AttrSizedOperandSegments`, `SameVariadicOperandSize`) but is plain Python metaprogramming over dataclass-like field declarations — **no code generation, no build step, fully introspectable at runtime via `get_irdl_definition()`**. This is the single closest prior-art match to what an Atlas Rust proc-macro (`#[derive(Operation)]` style) or a runtime dialect-registration API should produce: a typed, statically-checkable-where-possible, but also runtime-introspectable operation schema.

Separately, `xdsl/dialects/irdl/irdl.py` implements the **MLIR-IRDL-textual-format-compatible** dialect (parsing/emitting the same `irdl.dialect`/`irdl.operation`/`irdl.type` IR MLIR's own IRDL uses, see `xdsl/dialects/irdl/pyrdl_to_irdl.py` and `irdl_to_pyrdl.py`, and the `.irdl.mlir` fixtures under `tests/filecheck/dialects/irdl/`), i.e. xDSL supports **both** its own native Python IRDL (`xdsl/irdl/`, always used to define xDSL's own dialects) **and** interop with MLIR's data-as-IR IRDL dialect (conversion both directions, `pyrdl_to_irdl.py` / `irdl_to_pyrdl.py`) — direct evidence that "dialect defined as IR, convertible to/from a native in-language typed schema" is a workable, already-implemented round-trip, which is exactly the shape an Atlas ASIR dialect definition (native Rust API) plus an optional data/IR-based dialect-description format (for cross-tool interop or runtime dialect loading) would need.

## Traits / Interfaces (`xdsl/traits.py`, 880 lines)

xDSL **deliberately merges MLIR's traits and interfaces into one hierarchy** — stated explicitly in the source: `OpTrait` docstring reads "Note that traits are the merge of traits and interfaces in MLIR." `OpTrait` is a `@dataclass(frozen=True)` base with a `verify(self, op: Operation) -> None` hook (default no-op); a "trait" is just a plain Python subclass instance attached to an op's `traits: ClassVar[OpTraits]` collection, with capability-specific traits (e.g. `ConstantLike`, `HasFolder`) contributing extra abstract/static methods beyond just `verify`. There is no separate type-erased Concept/Model machinery — ordinary Python subclassing, `isinstance`/duck-typed dispatch (`op.has_trait(SomeTrait)`, `op.get_trait(SomeTrait)`) does the job, because Python has native runtime type introspection MLIR's C++ has to simulate via the Concept/Model pattern. **This is the most important divergence point from MLIR for Atlas's own design choice**: MLIR needed type erasure because C++ has no reflection; Rust's `dyn Trait` + `Any`/downcasting gives much of that same runtime introspection for comparatively little machinery. So Atlas can plausibly get away with something closer to xDSL's simpler single-hierarchy model (a `Vec<Box<dyn OpTrait>>` per op-kind, or a `TypeId`-keyed small map for `has_trait`/`get_trait` lookups) instead of needing MLIR's full external-model Concept/Model apparatus, unless Atlas specifically needs the "attach a capability to a type from a separate compilation unit" property MLIR's ExternalModel gives — which is still a real, separate consideration (see disposition below).

## Parser / Printer (`xdsl/parser/`, `xdsl/printer.py`)

`xdsl/parser/` is split into `base_parser.py` (350 lines, token/cursor-level primitives), `generic_parser.py` (331 lines, generic op/region/block syntax any op supports), `attribute_parser.py` (1656 lines — the largest parser file, one parse function per builtin/dialect attribute syntax), `affine_parser.py`, and `core.py` (1059 lines, the top-level `Parser` tying it together) — i.e. the same generic-vs-custom-per-op-syntax split MLIR has (`OpAsmParser`), just organized as ordinary Python modules instead of a C++ interface hierarchy. `Printer` (`xdsl/printer.py`, 703 lines) is the dual, single-direction (IR to text) emitter. No separate bytecode/binary format was found in xDSL (`find ... -iname "*bytecode*"` returned nothing) — xDSL is textual-IR-only; the bytecode format and its per-dialect versioning story is MLIR-only donor material (see `mlir` census).

## Rewrite / Pattern-Matching Infrastructure (`xdsl/pattern_rewriter.py`, 823 lines)

`RewritePattern` (`pattern_rewriter.py:352`) is an ABC with one abstract method `match_and_rewrite(self, op: Operation, rewriter: PatternRewriter)` — match and rewrite are **not** separated into two calls (unlike MLIR's conceptual match/rewrite split); a helper decorator `op_type_rewrite_pattern` (`pattern_rewriter.py:371`) uses **Python type-hint introspection** (`inspect.signature`) on the decorated method's second parameter to auto-filter by operation type before invoking it — a lightweight, no-macro way to get MLIR's "root operation name" pattern-matching optimization using only Python's own type system, worth noting as a pattern for a Rust equivalent (e.g. a proc-macro or generic `impl<Op: SomeOpType> RewritePattern<Op>` bound achieving the same filtering at compile time, for free, with no runtime type dispatch needed at all). `GreedyRewritePatternApplier` (`pattern_rewriter.py:581`) is xDSL's greedy-driver-equivalent: tries dead-code-elimination first (`is_trivially_dead`), then optional constant folding via a separate `Folder` (`xdsl/folder.py`) before falling through to the ordered pattern list, applying the first pattern that performs any rewriter action (`rewriter.has_done_action`) — i.e. **DCE and constant-folding are explicitly layered in front of the general pattern-rewrite loop**, not expressed as just more patterns in the same list. `PatternRewriteWalker` (`pattern_rewriter.py:639`) is the actual worklist/fixpoint driver, structurally parallel to MLIR's greedy driver.

## Verifier / `verify()`

Operation-level verification in xDSL is a method the generated `IRDLOperation` machinery attaches (calling into `OpDef`-derived operand/result/attribute/region arity and type-constraint checks) plus each attached trait's `verify(op)` hook (`OpTrait.verify`, default no-op, overridden per trait). Unlike MLIR's separately-staged `OperationVerifier` walk (`Verifier.cpp`), xDSL verification is simpler and more directly invoked (attribute self-verification at construction via `Attribute.__post_init__` calling `verify()`, operation verification via IRDL-generated checks plus trait `verify` calls) — consistent with xDSL generally choosing "verify eagerly, in the language's own object-construction hooks" over MLIR's "verification is an explicit, deferred, opt-in checkpoint pass." Both strategies were independently observed; Atlas should pick deliberately rather than default to either.

## Comparison Summary: Where xDSL Diverges from MLIR (and why that's more relevant to Atlas)

- **No intrusive linked lists / trailing-object memory layout** — plain dataclass fields and explicit optional-reference doubly-linked lists throughout. Confirms these are C++-only concerns; a Rust ASIR should use arenas/`Vec`/`Option<Id>`, matching xDSL's model far more closely than MLIR's.
- **Traits and interfaces merged into one hierarchy**, using ordinary subclassing/`isinstance` instead of MLIR's Concept/Model type-erasure machinery — because the host language (Python, and by extension Rust via `dyn Trait`/`Any`) already has the runtime introspection C++ lacks. This is the strongest single "pick xDSL's simpler answer over MLIR's" signal in this census.
- **IRDL is the *only* and *native* way to define operations** (no TableGen alternative exists or is needed) — proof by existence that a single, runtchecked declarative-field mechanism (Atlas's proc-macro/derive equivalent) is sufficient; MLIR's ODS/IRDL duality is a historical-compatibility artifact (ODS came first, IRDL added later for runtime use cases), not evidence Atlas needs two mechanisms.
- **match_and_rewrite as one combined call**, with type-filtering achieved via a thin decorator over Python's own type hints, instead of MLIR's benefit-scored, separately-negotiated match/rewrite protocol — simpler, and given Rust's compile-time generics, likely replaceable with zero-runtime-cost generic trait bounds rather than even needing xDSL's decorator trick.
- **Eager, construction-time attribute self-verification** vs. MLIR's deferred checkpoint-based verification — a genuine design fork worth Atlas deciding on explicitly (this census recommends studying both, not defaulting to either without a decision).

## Disposition (per concept)

- **Operation/Block/Region as plain typed structs/dataclasses with tuple/Vec-backed operand/result/region/successor storage**: ADAPT directly — confirms a straightforward Rust struct-of-`Vec`/`Box<[T]>` fields (no trailing-object tricks) is the right-fidelity ASIR `Operation` representation.
- **Two-tier `properties` (typed/inherent) vs `attributes` (generic dict) storage**: ADAPT — independently confirmed by both MLIR and xDSL as load-bearing, not MLIR-specific.
- **Def-use tracking via plain `Use` records in ordinary collections rather than intrusive linked lists**: ADAPT directly — this is the more Rust-idiomatic version of the same MLIR concept; prefer this shape over MLIR's intrusive-list mechanism.
- **`_next_op`/`_prev_op` explicit optional-reference doubly-linked list for block op ordering**: STUDY/ADAPT the intent (stable ordering, O(1) insert/remove given a neighbor), REIMPLEMENT via an arena/slotmap-friendly Rust structure (e.g. `Option<OpId>` next/prev into an arena, or an `indexmap`/`Vec`-backed ordered list) rather than literal optional-reference chains, which are awkward under Rust ownership without `Rc`/arena indirection.
- **Dialect as a small linear-scan registry of ops/attrs/interfaces**: ADAPT as the *starting point* — simpler than MLIR's TypeID-map, appropriate until/unless Atlas's dialect count or interface count makes linear scan a real cost.
- **Attribute as a closed `Data | ParametrizedAttribute` split with construction-time self-verification (`__post_init__` -> `verify()`)**: ADAPT — a clean, directly portable pattern for a Rust `Attribute` enum/trait with a `verify(&self)` invoked at construction (or at a deliberate builder-finalize step).
- **IRDL (native Python declarative op/attr/type definition, runtime-introspectable via `get_irdl_definition()`, no code-generation step)**: ADAPT — together with MLIR's IRDL, this is the primary architectural reference for Atlas's own dialect-definition mechanism (a Rust proc-macro producing a runtime-introspectable `OpDef`-equivalent schema is the target shape).
- **xDSL-native-IRDL <-> MLIR-textual-IRDL bidirectional conversion (`pyrdl_to_irdl.py`/`irdl_to_pyrdl.py`)**: STUDY — evidence that a native typed schema and a data/IR-based dialect description can round-trip; DEFER an actual Atlas equivalent (data-as-IR dialect description format) until there's a concrete cross-tool interop need.
- **Merged trait+interface hierarchy via ordinary subclassing/`isinstance`, no Concept/Model type erasure**: ADAPT as the default — prefer this simpler model in ASIR (`Box<dyn OpTrait>` / `TypeId`-keyed lookup) over porting MLIR's full external-model machinery, UNLESS Atlas specifically needs cross-compilation-unit capability attachment (MLIR's `ExternalModel`), which should be evaluated as a separate, deliberate extension if/when a concrete need for it (e.g. plugin dialects attaching behavior to core ASIR types) arises — DEFER that decision, don't build it speculatively.
- **`op_type_rewrite_pattern` type-hint-based auto-filtering decorator**: REIMPLEMENT-worthy only as inspiration — Rust's generics/trait bounds can likely achieve the same filtering at compile time with zero runtime cost, so don't literally port the reflection-based decorator approach.
- **GreedyRewritePatternApplier layering DCE and folding in front of the general pattern list rather than expressing them as ordinary patterns**: ADAPT — a concrete, sensible layering to reuse in an ASIR rewrite driver.
- **Eager construction-time verification (attributes) vs. deferred checkpoint verification (MLIR)**: STUDY both, DEFER an explicit Atlas decision to whoever designs ASIR's verification pass — flagged here as a real fork this census surfaced but does not resolve.
- **Parser/printer module split mirroring MLIR's generic-vs-custom-per-op-syntax distinction, but as plain Python modules instead of a C++ interface hierarchy**: STUDY as organizational reference only — low novelty, standard parser architecture.

## Things NOT to Copy

- Never copy xDSL's Python source verbatim into Atlas, even though Apache-2.0-WITH-LLVM-exception is a permissive, Atlas-license-compatible license — this is a clean-room architecture study; write Atlas's own Rust implementation.
- Do not port Python-specific mechanisms as-is: `inspect.signature`-based runtime type-hint introspection (`op_type_rewrite_pattern`), Python `ABC`/duck-typing dispatch, or dataclass `__post_init__` magic have no direct Rust translation and should be redesigned using Rust's own type system (generics, traits, derive macros) rather than simulated.
- Do not assume xDSL's simplicity (linear-scan dialect registry, no intrusive lists, no trailing-object layout) is "free" — some of it is only cheap because xDSL is un-optimized reference-quality Python; Atlas should validate its own Rust equivalents against real workload sizes rather than assuming xDSL's choices are performance-adequate at Atlas's scale.
- Do not treat xDSL's merged trait/interface model as proof that Atlas will never need MLIR-style external-model/cross-unit capability attachment — it is evidence the simpler model is a *reasonable default*, not proof the more complex mechanism is unnecessary for Atlas's actual plugin/extensibility requirements.

## Known Risks / Gaps in This Census

- Did not census `xdsl/transforms/` (the actual library of built-in rewrite/optimization passes) beyond incidentally reading `dead_code_elimination.is_trivially_dead` by reference from `pattern_rewriter.py` — no transform pass was read in full.
- Did not census `xdsl/interpreters/` or `xdsl/interpreter.py` (xDSL's IR interpreter/execution engine) at all — flagged as a plausible follow-up census topic if Atlas wants an ASIR reference-interpreter design, not covered here.
- Did not census `xdsl/backend/` or `xdsl/targets/` (lowering/codegen-adjacent code) at all.
- Did not census `xdsl/frontend/` (Python-to-xDSL frontend) at all.
- Did not read `xdsl/irdl/declarative_assembly_format.py`/`declarative_assembly_format_parser.py` (xDSL's declarative custom-assembly-syntax mechanism, an MLIR-ODS-assembly-format analog) beyond noting their existence — this is a real gap since custom textual syntax declaration is closely related to ODS/IRDL and was called out as in-scope territory; flagged for a follow-up pass, not covered above.
- Did not run or execute any xDSL code (including its own test suite) — per the donor-workbench isolation contract, this census is 100% static reading.
- `xdsl/irdl/constraints.py` and `xdsl/irdl/attributes.py` (the actual `AttrConstraint`/`RangeConstraint` implementations underlying IRDL's type-constraint expressiveness) were referenced via imports but not read in depth — only their role, not their internals, is characterized above.

## G90 — terminal REFERENCE_ONLY; source extinct

Both recorded forks are settled.

- **Dispatch default.** G89 showed neither registry precondition holds in Atlas, so the merged trait model is ordinary Rust traits, with no registry.
- **Verification timing.** The ASIR admission pipeline (ASIR-CONSTRUCTION-MODEL.md, CONTRACT) verifies every ACP transaction as an isolated post-transaction candidate before it applies, and again at seal. Value-level checks (xDSL's attribute self-verification) are its schema and type stages, run eagerly per payload. Structural checks (MLIR's verifier walk) run at the transaction checkpoint. States inside a transaction are never observable.

The ADAPT dispositions above remain design references for the TARGET ASIR. The checkout was physically deleted. Evidence: `../../evidence/campaign/26-xdsl.json`.
