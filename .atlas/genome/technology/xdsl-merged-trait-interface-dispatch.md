---
id: atlas.genome.technology.xdsl-merged-trait-interface-dispatch
type: technology-genome
status: active
canonical: true
---
# Technology Genome: merged trait/interface dispatch via ordinary dynamic typing (xDSL)

Donor: xDSL (`xdslproject/xdsl`, commit `00703508e37a2b4ef06687b3bbbe4cd78c41c740`),
compiler-IR-design-education lane.

This record follows the mandatory schema in `.atlas/contracts/DONOR-TO-LANGUAGE-GENESIS.md`. It
restructures the existing, exceptionally thorough deep census (`.atlas/census/donors/xdsl.md`, 148
lines) into the canonical genome shape, bounded to the single mechanism the census's own comparison
section names as "the strongest single 'pick xDSL's simpler answer over MLIR's' signal in this
census" (line 111): merging MLIR's separate Traits and Interfaces mechanisms into one ordinary,
dynamically-dispatched hierarchy. Re-verified directly against primary source in this pass
(`xdsl/traits.py` lines 1-70, `xdsl/ir/core.py` lines 1325-1358) rather than relying on the existing
census's citations alone. This record forms a deliberate pair with
`.atlas/genome/technology/mlir-external-model-interface-registration.md` (this session's immediately
preceding genome record) -- both study the same underlying capability-attachment problem from two
independently-implemented donors, and this record's Decision section states the synthesis directly.

## Capability / problem

Given the same problem MLIR's Interfaces solve (attach a cross-cutting capability -- "can this be
folded," "does this satisfy invariant X" -- to an IR operation kind, queryable generically), determine
whether a language with native runtime type introspection (Python; by direct extension, Rust via `dyn
Trait`/`Any`) actually needs MLIR's full type-erased Concept/Model/ExternalModel machinery, or whether
ordinary subclassing plus `isinstance`-style dispatch is sufficient.

## Semantic mechanism (as observed in the donor, evidence re-verified directly in this pass)

- **Traits and interfaces are explicitly, deliberately unified into one hierarchy** -- confirmed
  verbatim against the donor's own docstring (`xdsl/traits.py` lines 20-27, read directly):
  `OpTrait`'s class docstring states plainly, *"Note that traits are the merge of traits and
  interfaces in MLIR."* A single `@dataclass(frozen=True)` base (`OpTrait`) with one `verify(self, op:
  Operation) -> None` hook (default no-op) is both what MLIR would call a Trait (a marker/behavior
  mixin) and what MLIR would call an Interface (a capability with its own extra methods, e.g.
  `ConstantLike`/`HasFolder`, lines 36-77, read directly) -- there is no second, separate mechanism.
- **Lookup is an ordinary linear scan with `isinstance`, not a `TypeID`-keyed registry** -- confirmed
  directly against `Operation.get_trait`/`has_trait`/`get_traits_of_type` (`xdsl/ir/core.py` lines
  1325-1358, read directly): `get_trait` iterates `cls.traits` (the op class's own fixed collection,
  see `traits: ClassVar[OpTraits]` in the existing census's own reading of `Operation`'s fields) and
  returns the first entry satisfying `isinstance(t, trait)`. No vtable, no per-context registry, no
  separate `Concept`/`Model` split -- the host language's own runtime type system does the entire job
  MLIR needed a hand-built type-erasure mechanism to simulate.
- **Attachment is therefore also simpler by construction**: a trait/interface is just listed in the op
  class's own `traits` collection at definition time (populated by the IRDL machinery per the existing
  census's reading) -- there is no `attachInterface`-equivalent runtime registration call, because
  there is nothing decoupled to register; the capability is a plain part of the class's own static
  data.
- **The one real capability this simpler model gives up, named explicitly by the existing census's own
  "Things NOT to Copy" section (line 137)**: cross-compilation-unit attachment. MLIR's `ExternalModel`
  lets a dialect attach an interface implementation to an operation kind it does not own, from a
  separate translation unit, without touching the original type's source. xDSL's merged model has no
  equivalent -- a trait must be part of the op class's own `traits` list, which means (in the
  Rust-analog framing this record adopts) whoever defines the op type controls what capabilities it
  can ever have, unless a foreign-attachment mechanism is separately added.

## Required invariants

- A trait instance's `isinstance`/equality-based identity must remain stable and distinguishable from
  every other trait type for `get_trait`'s scan to resolve correctly -- in the Rust-analog framing,
  this is exactly what `TypeId`-based downcasting (`dyn Any::downcast_ref`) or an enum-tagged trait
  object would need to guarantee.
- `verify(self, op)`'s default no-op means a trait that adds no invariant of its own is free to attach
  purely as a capability marker (matching `ConstantLike`, which contributes no `verify` override in
  the excerpt read, only a `get_constant_value` helper) -- the mechanism does not force every attached
  capability to also carry a verification obligation.

## Identity/scope model

Directly complementary to the MLIR genome record's Identity/scope model section: where MLIR's
mechanism is identified by a `TypeID` resolved through a per-`MLIRContext` registry, xDSL's mechanism
has no separate identity concept at all -- a trait's "identity" is just its Python class object,
resolved through the host language's own type system. Neither Atlas concept exists in code today
(confirmed by the same search the MLIR record already performed: no `Dialect`/`ASIR`/trait-registry
code anywhere in `core/src`/`runtime/src`/`adapter/src`).

## State/effect/resource model

Stateless by the same discipline as MLIR's `Concept`/`Model` (a trait instance typically carries no
per-operation-instance data of its own -- `OpTrait` is `frozen=True`, i.e. immutable after
construction); all real per-instance state lives on the `Operation` the trait is attached to, exactly
mirroring the MLIR record's own finding for `Concept`/`Model`.

## Failure and recovery behavior

`get_trait`/`has_trait` return `None`/`False` on a missing trait rather than raising -- a clean,
already-idiomatic "optional capability" query, directly more amenable to a native Rust `Option<&dyn
OpTrait>` return than MLIR's assert-guarded `Concept*` pointer (see the MLIR record's own Failure and
recovery behavior section, which recommends exactly this typed-`Option` shape as the Rust target
regardless of which donor's model Atlas ultimately follows more closely).

One donor-code observation worth recording precisely rather than glossing over: `has_trait`'s own
signature declares a `value_if_unregistered: bool = True` parameter (`xdsl/ir/core.py` line 1330,
read directly) that the method body (`return cls.get_trait(trait) is not None`) never actually
references -- an apparent unused-parameter smell in the donor's own code, noted here as a fidelity
check on this record's own evidence (this record reads the donor precisely, including its rough
edges, rather than only its clean parts) and NOT proposed for replication in any Atlas-native
equivalent.

## Concurrency/temporal behavior

Not evaluated -- same reasoning as the MLIR record: trait attachment happens at op-class-definition
time (effectively static, not runtime-mutable per this reading), so no concurrent-mutation concern
was identified.

## Performance characteristics

Not benchmarked. The existing census's own framing (line 65, about `Dialect`'s similarly simple linear
scan) applies here too: a linear scan over a small, per-op-class trait list is a legitimate, simpler
starting point than a hashed/keyed registry, appropriate until a concrete measured cost justifies the
more complex alternative -- explicitly not assumed to already be a problem.

## Portability/ABI constraints

Pure-Python, relying on the language's own `isinstance`/ABC machinery; the direct Rust analog the
existing census itself proposes (line 94) is `dyn Trait` downcasting via `Any`, or a small `TypeId`-
keyed map for `has_trait`/`get_trait` -- i.e. Rust sits architecturally *between* xDSL's free
reflection and MLIR's hand-built type erasure: Rust has no built-in `isinstance` over arbitrary trait
objects, but `std::any::Any` plus `TypeId` gives equivalent capability with a small, well-understood
amount of machinery, closer in spirit to xDSL's simplicity than to MLIR's bespoke Concept/Model
apparatus.

## Evidence references

- `.atlas/census/donors/xdsl.md` (existing deep census, "Traits / Interfaces" section lines 92-94 and
  its Comparison Summary line 111 and Disposition entry line 126 -- primary prior evidentiary source)
- `.atlas/temporary/donors/xdsl/xdsl/traits.py` (880 lines total; this record directly reads lines
  1-77, `OpTrait`'s docstring and the `ConstantLike`/`HasFolder` trait definitions, rather than relying
  on the census's citations alone)
- `.atlas/temporary/donors/xdsl/xdsl/ir/core.py` (this record directly reads lines 1325-1358,
  `Operation.has_trait`/`get_trait`/`get_traits_of_type`, confirming the linear-scan-not-registry claim
  precisely, including the `value_if_unregistered` dead-parameter observation)
- `.atlas/genome/technology/mlir-external-model-interface-registration.md` -- this session's
  immediately preceding genome record, the direct comparison point this record is written against;
  the two together are the intended reading unit for this specific design decision
- Confirmed via `grep` that no `Dialect`/`ASIR`/trait-registry concept exists anywhere in `core/src`,
  `runtime/src`, or `adapter/src` today

## Donor revisions/licenses

xdslproject/xdsl, commit `00703508e37a2b4ef06687b3bbbe4cd78c41c740`, Apache License 2.0 WITH
LLVM-exception, verified by the existing census directly against
`.atlas/temporary/donors/xdsl/LICENSE` and cross-checked against `pyproject.toml`.

## Known trade-offs

- The merged model is strictly simpler to implement and reason about (no separate registry, no
  `TypeID` bookkeeping, attachment is just "list it in the class definition") at the direct cost of
  MLIR's one real extra capability: attaching a capability to a type from a separate compilation unit
  that does not own it. The existing census's own "Things NOT to Copy" section (line 137) states this
  precisely: xDSL's simplicity is evidence the simpler model is a *reasonable default*, not proof the
  more complex mechanism is never needed.
- A linear scan over a small per-class trait list is asymptotically worse than a hashed lookup, but
  only matters once a single op kind accumulates enough traits for the difference to be measurable --
  not assumed to be a real cost without evidence, consistent with the Dialect-registry finding the
  existing census already made about xDSL's similarly simple design elsewhere.

## Rejected alternatives (for this pass)

- Full-fidelity direct adoption of xDSL's dynamic, ABC-based dispatch as literal Rust code -- not
  applicable; Python's `isinstance`/duck-typing has no direct Rust translation (already stated by the
  existing census's own "Things NOT to Copy," line 135), so the *shape* (unified hierarchy, no separate
  registry) is the transferable asset, mapped onto `dyn Trait`/`Any`-based Rust idioms, not the
  Python mechanism itself.
- Building Atlas's own MLIR-style `ExternalModel` cross-unit attachment mechanism now, speculatively --
  explicitly rejected, matching the existing census's own disposition (line 126: "DEFER that decision,
  don't build it speculatively") and this record's own Decision below.
- IRDL, the parser/printer split, and the rewrite/pattern-matching infrastructure the same xDSL census
  covers -- all real, independently valuable findings, deliberately left out of this record's scope
  (bounded to the trait/interface-unification mechanism specifically, per this session's established
  one-mechanism-per-record convention). They remain queued in the existing census's own disposition
  table.

## Dependency/extinction status

No `runtime_dependency_status`/`census_status` change is made by this record. xDSL's own
`donor-corpus.toml` entry (`census_status = "DEEP_CENSUS_ACTIVE"`) is unchanged -- the existing
census's own "Known Risks/Gaps" section (the transforms library, the interpreter, the backend/targets
code, the frontend, and the declarative-assembly-format mechanism) lists real, still-open census areas.
No runtime/build dependency on xDSL source exists today, nor is one proposed by this record.

## Decision

**`ABSORB_LATER`**, and specifically: **this record's real contribution is a decision rule, not an
implementation to schedule** -- exactly the same shape of finding the Cap'n Proto genome record
produced for the binary-format cluster earlier this session. Given both mechanisms are now
genome-captured (this record and `mlir-external-model-interface-registration.md`), the synthesis is:
when Atlas's own ASIR/dialect extensibility design work eventually starts, **default to xDSL's merged,
ordinary-dispatch model** (a `Vec<Box<dyn OpTrait>>` per op-kind, or a small `TypeId`-keyed map for
`has_trait`/`get_trait`-equivalent queries) rather than building MLIR's full external-model apparatus
up front. Only adopt the MLIR record's `TypeId -> Box<dyn Any>`-keyed capability-registry design
(cross-compilation-unit attachment) once a **concrete, named** requirement for it exists -- the
clearest one either census names is a plugin/external dialect needing to attach a capability to a core
ASIR op or type it does not itself define. Until that requirement is real, building the more complex
mechanism is speculative generality this session's own engineering-quality standard forbids
introducing ahead of need.

Not `ABSORB_NOW`: no ASIR/dialect extensibility design work has started (confirmed by search, same as
the MLIR record) -- there is no current consumer for either mechanism yet.

This record does not change `xdsl`'s `census_status`; the donor's own census remains honestly
`DEEP_CENSUS_ACTIVE`, with this genome record added as new evidence covering its census's own
strongest-named comparison finding, deliberately paired with the MLIR genome record to make the
cross-donor decision rule explicit rather than leaving it implicit across two separate documents.

## G90 update

The campaign cycle (first-50 #26) resolved this record's open conditions. The merged model needs no registry (G89), and the verification fork is settled by the ASIR admission pipeline. The donor is REFERENCE_ONLY and its checkout is physically deleted, so `.atlas/temporary/donors/xdsl` paths above now name the pinned commit `00703508e37a2b4ef06687b3bbbe4cd78c41c740`.
