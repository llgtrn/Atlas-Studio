---
id: atlas.genome.technology.salsa-fixpoint-cycle-iteration
type: technology-genome
status: active
canonical: true
---
# Technology Genome: query-graph fixed-point cycle iteration (Salsa)

Donor: Salsa (`salsa-rs/salsa`, commit `5d8eaf1fa372a73d44f9b717407de41f19ef59d7`). Direct follow-up
to `salsa-durability-gated-incremental-verification.md`, which explicitly flagged this mechanism as
the natural next slice of Salsa's own census and the likely actual referent of R5's own
"fixed-point closure" naming (`.atlas/roadmap/SELF-BUILDING-R4-R8.md`). Confirmed correct: `cycle.rs`
opens with an unusually thorough module doc comment (read in full, lines 1-42) that names exactly
this mechanism "fixed-point iteration."

## Capability / problem

R5 requires "recursive/fixed-point semantic derivation" over the same query graph R5's other
requirements already assume. A derivation graph over a real, growing donor/dependency corpus can
have genuine cycles (mutually recursive facts -- e.g. two donors whose admitted capability sets each
depend on facts about the other, or a recursive dependency-closure computation that must reach a
stable answer rather than looping forever). Salsa's default behavior for a cycle is simply to PANIC
(`cycle.rs:4-5`, confirmed) -- fixed-point iteration is an explicit OPT-IN a query author requests,
not automatic magic; this record captures both the mechanism and the fact that panicking-by-default
is the correct, honest default Atlas should also adopt, not iterate-by-default.

## Semantic mechanism (as observed in the donor)

- **Cycle detection is call-stack membership, not graph analysis.** A cycle is recognized the moment
  a query attempts to execute while already on the active execution stack (module doc comment,
  confirmed) -- purely a runtime property of the current derivation attempt, requiring no
  precomputed dependency graph or static analysis.
- **The "cycle head"**: whichever query was ALREADY on the stack when re-entered is responsible for
  managing the whole cycle's iteration. It calls a query-author-supplied `cycle_initial` function to
  produce a bottom/starting value (classic Kleene fixed-point iteration: start from the lattice's
  bottom element), which is returned to whichever caller triggered the cycle.
- **Provisional values, explicitly marked as such.** Every value computed while inside an
  unconverged cycle is tagged with the set of cycle head(s) it transitively depends on
  (`CycleHeads`, `cycle.rs:222` -- a `ThinVec<CycleHead>`, each `CycleHead` pairing a
  `DatabaseKeyIndex` with an `IterationStamp`, `cycle.rs:88,133`). A memoized result carrying a
  non-empty `CycleHeads` is explicitly a PROVISIONAL value, not a final one -- this is a real,
  first-class distinct state (not merely "possibly stale"), confirmed as a distinguishable case
  throughout `function/execute.rs` and `function/maybe_changed_after.rs`.
- **Convergence is TWO conditions, not one -- a detail easy to get wrong by only checking the value.**
  `execute.rs:792`: `this_converged = value_converged && metadata_converged`. `value_converged`
  (`execute.rs:255-271`) is the query-author-supplied `values_equal` comparison between the
  previous iteration's provisional value and the newly recomputed one -- the standard Kleene
  "has the value stopped changing" check. But `metadata_converged` is checked SEPARATELY
  (`execute.rs:786`, comparing `durability`/other revision metadata across iterations) --
  because a query's VALUE could stabilize while its computed DURABILITY has not yet, which would
  make the shallow-verification optimization from `salsa-durability-gated-incremental-verification.md`
  unsound if trusted prematurely. This is the concrete point where this record's two Salsa genome
  documents connect: fixed-point convergence must be judged against BOTH mechanisms this donor
  documents, not either alone.
- **`CycleRecoveryStrategy` (`cycle.rs:63`, three variants, confirmed):** `Panic` (the default --
  correct, honest failure for a query that never opted in), `Fixpoint` (the full iterate-to-
  convergence mechanism described above), and `FallbackImmediate` (a cheaper third option: skip
  iteration entirely, immediately substitute a fixed fallback value for every query in the cycle,
  "as if they were not computed" -- confirmed at `execute.rs:250-254`, `value_converged` is
  unconditionally `true` for this strategy since no iteration ever happens). `FallbackImmediate` is
  the right choice for a cycle where SOME sound-but-imprecise default answer is acceptable and
  paying for iteration is not justified; `Fixpoint` is for cycles where the actual converged value
  matters.
- **A hard iteration bound, not an unbounded loop:** `MAX_ITERATIONS: u8 = 200` (`cycle.rs:52`,
  confirmed), explicitly documented as a safety net against "a badly configured cycle recovery"
  function that never actually converges -- the mechanism does not trust that any user-supplied
  `values_equal`/`recover_from_cycle` pair is well-behaved; it panics rather than hanging forever
  past this bound.
- **Lazy finality propagation, an explicit efficiency choice, not an oversight (module doc comment,
  confirmed):** when the cycle head converges, it alone is immediately marked final. Every OTHER
  query in the cycle keeps its provisional marking -- the mechanism does NOT eagerly walk the whole
  cycle re-marking everything final in one pass. Instead, the NEXT time any such provisional value
  is actually read, the reader checks whether all of ITS OWN recorded cycle heads are now final, and
  only then promotes it. This defers work to exactly the queries that are still actually being
  consulted, rather than paying for a cycle-wide walk whether or not the rest of the cycle's values
  are ever read again in this revision.
- **Nested cycles transfer lock ownership to the outermost cycle head** (module doc comment,
  confirmed) specifically to avoid a documented real failure mode: without this, independent threads
  iterating different nested cycle heads would compete for each other's locks, producing "potential
  hangs (but not deadlocks)" -- the donor's own words, naming a subtle concurrency hazard this
  mechanism was specifically built to avoid, not merely correctness alone.

## Required invariants

- The query-author-supplied `cycle_initial`/`values_equal`/`recover_from_cycle` functions must form
  a MONOTONE step function over a well-founded lattice for iteration to be guaranteed to terminate in
  principle (standard Kleene fixed-point theory) -- Salsa itself does not, and cannot, verify this;
  `MAX_ITERATIONS` is the mechanism's own explicit acknowledgment that it cannot prove termination
  and instead bounds the damage of a caller that violates this invariant.
- Convergence must check BOTH value equality AND metadata (durability) stability, per
  `execute.rs:792` -- checking value equality alone is an easy, plausible-looking, INCORRECT
  simplification this record specifically flags, since a value-converged-but-metadata-unconverged
  query could otherwise be wrongly treated as safe to shallow-verify in a later revision.

## Identity/scope model

`CycleHeads` are keyed by `DatabaseKeyIndex` (the same query identity used throughout Salsa, per the
prior genome record) plus an `IterationStamp` -- so the SAME query participating in a cycle at
different iteration counts is distinguishable, letting the mechanism tell "this is the same
provisional value from iteration 3" apart from "iteration 4" without needing a separate identity
scheme.

## State/effect/resource model

Purely in-memory, process-local iteration state layered on top of the memo-table mechanism the prior
genome record already covers; no new I/O or persistence surface.

## Failure and recovery behavior

Two distinct, deliberate failure modes, not one: (1) a cycle with no opt-in recovery strategy panics
immediately at cycle-detection time (`CycleRecoveryStrategy::Panic`, the default) -- the mechanism
never silently produces a wrong answer for an unhandled cycle; (2) a cycle WITH `Fixpoint` recovery
that fails to converge within `MAX_ITERATIONS` also panics, rather than hanging -- both failure
paths are loud, not silent, matching this session's own R4 epistemic discipline of never fabricating
a result rather than reporting that one cannot be produced.

## Concurrency/temporal behavior

The lock-transfer-to-outer-cycle-head design (above) is concurrency-specific machinery this record
notes but does not evaluate further, per the prior genome record's own finding that Atlas's census
pipeline is currently single-threaded.

## Performance characteristics

Not benchmarked. `FallbackImmediate` existing as a distinct, cheaper strategy alongside `Fixpoint`
is itself evidence that the donor's own authors consider full iteration measurably more expensive
than a single fallback substitution -- a real design trade-off Atlas's own eventual R5 design should
expect to face for its own recursive derivations (e.g., an unbounded transitive-dependency-closure
computation might reasonably use a `FallbackImmediate`-shaped strategy rather than always iterating).

## Portability/ABI constraints

Not applicable; pure in-process Rust mechanism.

## Evidence references

- `.atlas/temporary/donors/salsa/src/cycle.rs` (module doc comment read in full, lines 1-42;
  `MAX_ITERATIONS` at line 52; `CycleRecoveryStrategy` at line 63; `CycleHead`/`IterationStamp` at
  lines 88/133; `CycleHeads` at line 222)
- `.atlas/temporary/donors/salsa/src/function/execute.rs` (convergence logic read directly: lines
  215-295 for `value_converged` computation including the `FallbackImmediate` short-circuit at
  250-254 and the `Fixpoint` path's `values_equal` check at line 271; line 792 for the combined
  `this_converged = value_converged && metadata_converged` check; lines 764-833 for
  `try_complete_cycle_head` and the lazy inner-cycle convergence check)
- `.atlas/genome/technology/salsa-durability-gated-incremental-verification.md` (this record's own
  companion -- the metadata/durability half of the convergence check depends on that record's own
  mechanism)
- `.atlas/roadmap/SELF-BUILDING-R4-R8.md` R5 section (the "recursive/fixed-point semantic
  derivation" requirement this record directly answers)

## Donor revisions/licenses

salsa-rs/salsa, commit `5d8eaf1fa372a73d44f9b717407de41f19ef59d7`. Dual Apache-2.0/MIT, unchanged
from the companion record.

## Known trade-offs

- Fixed-point iteration requires the query author to supply a correct `values_equal` and a
  monotone `recover_from_cycle` -- real implementation burden shifted to whoever writes the
  recursive query, not something the framework can automatically derive. Any Atlas-native
  absorption of this mechanism inherits the same burden for whichever R5 queries need it.
- `MAX_ITERATIONS = 200` is a magic-number safety net, not a proof of termination -- Atlas's own
  eventual choice of bound (if it adopts this pattern) should be a deliberate, documented decision,
  not copied verbatim without considering its own workload's expected iteration depth.

## Rejected alternatives (for this pass)

- Not evaluated in this record: whether Atlas's own eventual recursive derivations (e.g. transitive
  dependency closure, which `core::census::dependency::DependencyClosureReport` already computes
  today via a presumably iterative-but-not-cycle-safe algorithm -- not re-examined here) would
  actually need general fixed-point iteration, or whether their specific recursive structure (e.g.
  already acyclic by construction, per `Cargo.lock`'s own DAG guarantee) makes this mechanism
  unnecessary for R5's FIRST concrete use case even if valuable for others. Flagged as an open
  design question for R5 implementation, not resolved here.

## Dependency/extinction status

`PENDING` per `donor-corpus.toml`, unchanged by this record.

## Decision

**No absorption decision yet**, consistent with the companion record. This record establishes,
durably: (1) Salsa's fixed-point cycle-iteration mechanism is now genuinely understood and evidenced
from real source, confirming the hypothesis that it is the concrete mechanism behind R5's own
"fixed-point closure" roadmap naming; (2) the mechanism's two most easily-missed subtleties --
convergence requiring BOTH value and metadata stability, and lazy (not eager) finality propagation
through the rest of the cycle -- are recorded explicitly so a future R5 implementer does not have to
rediscover them by trial and error; (3) whether Atlas's own concrete R5 use cases actually need
general cycle recovery (vs. being acyclic by construction) is an open question this record does not
resolve.

`census_status` for Salsa remains `COARSE_CENSUSED` in `donor-corpus.toml` (this record adds a
second core-mechanism deep-dive, but tracked structs, accumulators, the macro-generated ingredient
system, and parallel execution remain explicitly uncensused, so `DEEP_CENSUSED` -- which would imply
comprehensive surface coverage -- remains premature). `decision_status` remains `PENDING`.
