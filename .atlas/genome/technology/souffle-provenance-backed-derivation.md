---
id: atlas.genome.technology.souffle-provenance-backed-derivation
type: technology-genome
status: active
canonical: true
---
# Technology Genome: provenance-backed derivation (Souffle)

Donor: Souffle (`souffle-lang/souffle`, commit `a1303be3c0166400dee3d1f36f0d96abe03e6901`), the
fourth W3 donor censused this session, and the first from the `--provenance` translation strategy
rather than the plain `seminaive` one. Souffle is a large, AOT-compiling C++ Datalog engine (AST ->
RAM relational-algebra IR -> interpreted or C++-synthesized execution); this record deliberately
scopes to ONE specific mechanism -- how a derived fact's own justification becomes a first-class,
reconstructable, queryable artifact -- read directly from `src/ast2ram/provenance/UnitTranslator.{h,cpp}`,
`src/ast2ram/provenance/ClauseTranslator.cpp`, and `src/include/souffle/SouffleInterface.h`'s own
documented `getAuxiliaryArity()` contract. It does not attempt to census the RAM IR itself, the
interpreter, the C++ synthesiser, the parser/AST layer, or the plain `seminaive` (non-provenance)
translation strategy -- see Decision for the explicit remaining scope.

## Capability / problem

Every donor censused so far in the W3 lane (Salsa, datafrog, differential-dataflow) answers "what is
the current, correct result" -- none of them make "WHY does this specific derived fact hold, and
which specific supporting facts and rule justify it" a first-class, queryable property of the
computation itself. Atlas's own entire semantic model is built on the opposite discipline
(`EpistemicStatus`, `Evidence`, `Provenance`, "never claim Observed without syntax-determined
evidence") for LEAF observations -- but nothing in Atlas today extends that same discipline to a
DERIVED fact (e.g. a future R5/R6 recursive/fixed-point derivation, which the roadmap's own R5
section explicitly names: "evidence-preserving query dependencies"). Souffle's provenance mode is a
real, shipped, working answer to exactly this gap for Datalog-shaped derivation.

## Semantic mechanism (as observed in the donor)

- **Every provenance-enabled relation's tuple carries two extra, HIDDEN trailing columns, stored
  inline as part of the same tuple** (`UnitTranslator.cpp:146-149`, `addAuxiliaryArity` returns
  `auxArity = 2`; documented at the public API level in `SouffleInterface.h:365-377`:
  "Auxiliary attributes are used for provenance ... stored as the last attributes of a tuple").
  Confirmed by `ClauseTranslator.cpp:92-97`: the two values are `@rule_num` (which of the clause's
  own alternative rule bodies derived this specific tuple) and `@level_num` (the derivation's own
  proof height -- how many derivation steps deep this tuple's justification chain goes). This is the
  single essential design choice the whole mechanism rests on: provenance metadata is NOT a separate
  side-table or external log keyed by tuple identity -- it travels WITH the tuple, in the same
  relation, at the storage layer other donors treat as pure application data.
- **A "subproof subroutine" is auto-generated per clause, translating the SAME rule body into a
  reusable existence-check** (`UnitTranslator.h`: `makeSubproofSubroutine`/
  `makeNegationSubproofSubroutine`; `addProvenanceClauseSubroutines` visits every non-fact clause in
  the program and generates one). Given a candidate tuple's own bound argument values plus its
  `@rule_num`, the subroutine re-executes exactly that one rule body as a RAM existence check
  (`ExistenceCheck`) against the current relation state, returning which specific subgoal atoms
  matched which specific supporting tuples -- each of which carries its own `@rule_num`/`@level_num`
  and can be recursively explained the same way. This is a real, derived-on-demand explanation, never
  a fabricated or approximated one: the subroutine is compiled from the identical AST the original
  derivation rule was compiled from, so a "yes" answer is exactly as trustworthy as the original
  derivation itself.
- **`@level_num` is what makes recursive proof reconstruction provably terminate, and gives proofs a
  natural minimality ordering**: since every derivation step strictly increases `@level_num` from its
  supporting facts' own levels, walking backward through a chain of subproof calls strictly decreases
  it, guaranteeing the reconstruction is finite even for deeply recursive Datalog programs (e.g.
  transitive closure) -- the same finite-descent argument a well-founded ranking gives any recursive
  proof search, made concrete and mechanical here as one extra integer column.
- **Negation is handled by an existence check in the OPPOSITE direction**
  (`makeNegationSubproofSubroutine`, `UnitTranslator.cpp:394+`, own comment shows the generated
  shape: `IF (arg0,arg1,_,_) IN rel_1: return 1` / `IF ... NOT IN rel_1: return 0` per subgoal):
  explaining why a NEGATED subgoal held (i.e. why some fact does NOT exist) is answered by checking
  each of the clause's own positive alternatives failed to match, not by a symmetric "positive"
  proof -- an intrinsically different, asymmetric justification shape from the positive case, which
  the donor's own code keeps as a clearly separate generated subroutine rather than unifying the two.
- **Provenance mode changes the TRANSLATION STRATEGY, not the source Datalog program**: `provenance::
  UnitTranslator` is a subclass of `seminaive::UnitTranslator` (`UnitTranslator.h:41`), overriding
  only relation-creation/auxiliary-arity/subroutine-generation hooks. The same `.dl` source program
  compiles under either strategy; provenance is a cross-cutting instrumentation layered onto the
  existing semi-naive evaluation, not a competing evaluation algorithm or a change to what the
  program computes.

## Required invariants

- `@rule_num` must be assigned consistently at derivation time and never reused across genuinely
  different rule bodies for the same relation, or a subproof lookup would re-execute the WRONG rule
  body against a tuple, producing an explanation for facts the program never actually derived that
  way -- a soundness bug in the explanation layer, not merely an incomplete one.
- `@level_num` must be a true strict-descent measure (every derivation's level exceeds every one of
  its own direct supporting facts' levels) or the finite-termination argument for recursive
  reconstruction no longer holds.
- The subproof subroutine's own re-execution must be against the SAME relation state the original
  derivation saw (or an equivalent fixed point) -- re-deriving against a mutated/incremental relation
  state could certify a different justification than the one that actually produced the stored tuple.

## Identity/scope model

A provenance-mode tuple's identity is still `(primary columns)`, exactly as in plain semi-naive mode
-- `@rule_num`/`@level_num` are metadata ABOUT that identity's derivation, not part of what makes two
tuples the same or different fact. This is a clean separation Atlas's own `SemanticObservation` vs.
`Evidence`/`Provenance` split already mirrors structurally (a fact's own identity is independent of
which evidence backs it) -- souffle's contribution is showing that split can be pushed all the way
down to the DERIVED-fact storage layer itself, not only the leaf-observation layer Atlas currently
applies it to.

## State/effect/resource model

Provenance mode's storage cost is exactly 2 extra integer columns per tuple in every relation that
opts in (`auxArity = 2`, a compile-time constant for this translation strategy) -- a small, fixed,
predictable overhead, not a separate parallel data structure whose size could grow independently of
the relation itself. Subproof reconstruction is on-demand (a subroutine call per explanation
request), not maintained eagerly for every derived tuple -- explanation cost is paid only when
something is actually asked to justify itself.

## Failure and recovery behavior

Not evaluated in this record (out of scope -- this record covers the provenance-instrumentation
mechanism specifically, not Souffle's broader error handling/diagnostics).

## Concurrency/temporal behavior

Not evaluated in this record. Souffle's plain (non-provenance) evaluation can synthesize
parallelized C++ execution (confirmed present in `src/synthesiser/`, not read this pass); whether
provenance mode's auxiliary-column/subproof mechanism is compatible with that parallel execution
path was not investigated here.

## Performance characteristics

Not benchmarked in this record. The two-column overhead itself is small and bounded (see State/
effect/resource model above); the cost of the generated subproof subroutines' own re-execution at
explanation time was not measured.

## Portability/ABI constraints

C++17, AOT-compiled (either interpreted RAM or synthesized-and-compiled C++, per `src/synthesiser/`,
not read this pass); no FFI surface relevant to this record's scope. The MECHANISM (auxiliary
metadata columns + auto-generated per-rule subproof subroutines) is language-agnostic and does not
depend on any C++-specific capability -- directly portable as a design pattern to an Atlas-native
Rust implementation.

## Evidence references

- `.atlas/temporary/donors/souffle/src/ast2ram/provenance/UnitTranslator.h` (read in full: the
  `provenance::UnitTranslator` class declaration, its override surface, and `auxArity`)
- `.atlas/temporary/donors/souffle/src/ast2ram/provenance/UnitTranslator.cpp` (read: `addAuxiliaryArity`,
  `addProvenanceClauseSubroutines`, `makeNegationSubproofSubroutine`'s own doc comment showing the
  generated shape)
- `.atlas/temporary/donors/souffle/src/ast2ram/provenance/ClauseTranslator.cpp` (read: `@rule_num`/
  `@level_num` variable assignment, confirming the two auxiliary columns' real meaning)
- `.atlas/temporary/donors/souffle/src/include/souffle/SouffleInterface.h` (read: the public
  `getAuxiliaryArity()`/`getPrimaryArity()` API contract and its own doc comment naming provenance
  as the reason auxiliary attributes exist)
- `.atlas/roadmap/SELF-BUILDING-R4-R8.md` R5 section ("evidence-preserving query dependencies") and
  R6 section (reconciliation/CensusCertificate) -- the roadmap sections this mechanism is most
  directly relevant to

## Donor revisions/licenses

souffle-lang/souffle, commit `a1303be3c0166400dee3d1f36f0d96abe03e6901`. Not independently
re-verified in this record (license file not read this pass; out of scope for a mechanism-only
census).

## Known trade-offs

- Provenance mode is a distinct translation strategy the query author must opt into (`--provenance`
  at the CLI level, confirmed by the existence of the separate `provenance::` translator hierarchy
  alongside `seminaive::`) -- it is not free by default, and every relation opting in pays the fixed
  2-column storage cost even for facts nobody ever asks to explain.
- The subproof mechanism as observed handles atoms and negation; the donor's own code contains an
  explicit `TODO` (`ClauseTranslator.cpp`-adjacent comment, `makeNegationSubproofSubroutine`: "we
  only deal with atoms (no constraints or negations or aggregates or anything else...)") acknowledging
  incomplete coverage of the full Datalog constraint language as of the censused commit -- not
  evaluated further whether this has since been extended, since this record pins to one commit.

## Rejected alternatives (for this pass)

Not evaluated in this record: the RAM IR itself, the interpreter, the C++ synthesiser
(`src/synthesiser/`), the AST/parser layer, and the plain `seminaive` (non-provenance) translation
strategy that provenance mode extends. Each is a substantial, separate census target if R5/R6
implementation work ever needs Souffle's OTHER mechanisms (e.g. AOT compilation of a declarative
query to specialized imperative code, directly relevant to Atlas's own eventual compiler-backend
ambitions), not merely the provenance-instrumentation pattern this record covers.

## Dependency/extinction status

`PENDING` per `donor-corpus.toml`, unchanged by this record.

## Decision

**No absorption decision yet.** This record establishes: (1) Souffle's provenance mechanism -- fixed,
small, inline auxiliary metadata (`rule_num`, `level_num`) carried on every derived tuple, plus an
auto-generated, on-demand "subproof" subroutine per rule that re-executes the SAME rule body as a
targeted existence check to reconstruct a specific tuple's justification, recursively, with
termination guaranteed by the level metadata's own strict-descent property -- is now genuinely
understood and evidenced from real donor source, not merely from its public-facing name; (2) this is
the first W3-lane mechanism directly addressing R5's own named "evidence-preserving query
dependencies" goal for DERIVED (not merely observed) facts, complementing rather than competing with
Salsa's revision-gated verification, datafrog's semi-naive evaluation, and differential-dataflow's
algebraic retraction -- an eventual Atlas-native R5/R6 design could layer souffle-style provenance
metadata onto whichever of those three incremental-evaluation mechanisms is ultimately selected,
since the provenance pattern (metadata-on-tuple + regenerable subproof) is evaluation-strategy-
agnostic; (3) the donor's much larger surface (RAM IR, interpreter, C++ synthesiser, parser) remains
substantially uncensused and is a separate, later target if Atlas's own eventual compiler-backend
work (Phase 1-4 in the roadmap) ever wants to study AOT specialization of a declarative query to
imperative code, a materially different mechanism than provenance tracking.

`census_status` for souffle in `donor-corpus.toml` is advanced from `SKELETON` to `COARSE_CENSUSED`
(one real, evidenced mechanism -- provenance-backed derivation -- is now documented; the RAM IR,
interpreter, synthesiser, and parser remain explicitly, substantially uncensused). `decision_status`
remains `PENDING`.
