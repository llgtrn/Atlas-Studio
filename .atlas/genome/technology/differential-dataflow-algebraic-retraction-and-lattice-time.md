---
id: atlas.genome.technology.differential-dataflow-algebraic-retraction-and-lattice-time
type: technology-genome
status: active
canonical: true
---
# Technology Genome: algebraic retraction and lattice-valued logical time (differential dataflow)

Donor: differential-dataflow (`TimelyDataflow/differential-dataflow`, commit
`aa8745f93ea8abe131104fc7885ba4fd47e63902`), the third W3 donor for R5, following the two Salsa
records and the datafrog record. This donor is dramatically larger than the previous two (207 Rust
files, built on top of an entire separate framework, `timely-dataflow`, for distributed data-
parallel execution) -- this record deliberately scopes to the TWO essential generalizations
identified as the actual gap between Salsa and datafrog in the prior genome record, read directly
from `src/lib.rs`, `src/difference.rs` (full file), and `src/trace/mod.rs` (module documentation and
type signature). It does not attempt to census the operator library, the distributed execution
model, arrangements/indices, or the `timely-dataflow` substrate itself -- see Decision for the
explicit remaining scope.

## Capability / problem

The prior genome record (`datafrog-semi-naive-fixpoint-evaluation.md`) identified a concrete gap:
Salsa answers "what does one changed input invalidate" but cannot compute a from-scratch closure the
way datafrog does; datafrog computes a full closure efficiently but is insertion-only (monotone) and
must re-run entirely from scratch for each new base fact set, with no notion of reacting to a
single incremental edit against ALREADY-computed results. Differential dataflow's own stated purpose
(`lib.rs`, confirmed) closes exactly this gap: "add records to or remove records from its inputs;
the system will automatically update the computation's outputs with the appropriate corresponding
additions and removals" -- i.e., real incremental view maintenance over a computation that can also
be understood as a full Datalog-style derivation graph (`iterate`, joins, `map`/`filter`/`reduce`,
confirmed in `lib.rs`'s own vocabulary, directly recognizable as datafrog's own operator vocabulary
generalized).

## Semantic mechanism (as observed in the donor)

- **Diffs are elements of an ABELIAN GROUP, not hardcoded signed integers** (`difference.rs`, read
  in full, confirmed): a layered trait hierarchy -- `IsZero` (can this value be recognized as the
  additive identity, letting an update be safely discarded once it nets to nothing) ->
  `Semigroup` (addition, `plus_equals`) -> `Monoid` (has an explicit zero) -> `Abelian` (has
  negation). The module's own doc comment states the generalization directly: "the most common
  generalization is when we maintain both a count and another accumulation, for example height ...
  which allows us to track something like the average" -- a `(count, sum)` pair is itself a valid
  Abelian group under component-wise addition/negation, so the SAME incremental machinery that
  tracks simple insert/delete multiplicities can, with no change to the core algorithm, track a
  running sum or other associative aggregate incrementally too. This is the essential generalization
  beyond datafrog's implicit assumption of monotone (insertion-only, "diff" = "always present, never
  negative") facts: `Abelian::negate` is precisely what makes RETRACTION -- "this fact used to hold
  and no longer does" -- a first-class, uniformly-handled case rather than something requiring a
  from-scratch recomputation, exactly the gap this record set out to investigate.
- **Time is a LATTICE (a partial order with joins), not a scalar counter** (`trace/mod.rs`, module
  doc comment, confirmed): a trace is "a set of updates of the form `(key, val, time, diff)`, which
  determine the contents of a collection at given times by accumulating updates whose time field is
  less than or equal to the target field." Unlike Salsa's single global `Revision` (a totally-ordered
  scalar) or datafrog's single integer round counter, `time` here is any type implementing `Lattice`
  (imported from `timely::progress`) -- this is precisely what unifies Salsa's "which revision is
  this valid as of" concept and datafrog's "which round was this derived in" concept into ONE
  representation general enough to express BOTH simultaneously: nested loop iteration (datafrog's
  own round-by-round model) AND real streaming wall-clock-ish progress (Salsa's own revision-by-
  revision model) are both just different concrete lattices plugged into the same generic mechanism.
  A computation with an `iterate` operator nested inside an outer streaming input uses a PRODUCT
  lattice (outer time, inner iteration count) -- this composability (nest a fixed-point loop inside
  an incrementally-updated outer computation, or vice versa) is not something either Salsa or
  datafrog's own mechanism, read in isolation, obviously supports; it falls out for free once time is
  generalized to a lattice.
- **`Antichain`/frontier tracking answers "which times are done" -- required to know when
  accumulated diffs can be safely compacted** (`trace/mod.rs`, `use timely::progress::{Antichain,
  frontier::AntichainRef}`, confirmed as a core import, not evaluated to full implementation depth
  in this record). Because time is only a PARTIAL order, "is round N complete" cannot be answered by
  a simple `>=` comparison the way Salsa's totally-ordered `Revision` allows (`revision.rs`'s own
  `<=` comparison, per the companion record); the frontier (an antichain -- a set of pairwise-
  incomparable times marking the current "leading edge" of possible future updates) is the
  generalized mechanism answering the same question a scalar counter answers for free in the
  simpler donors. This added complexity is the direct, necessary cost of the lattice-time
  generalization -- not incidental implementation detail.

## Required invariants

- Diff addition must actually be COMMUTATIVE and ASSOCIATIVE (a true Abelian group), or accumulating
  updates in a different order (which the system is free to do, since consolidation/compaction
  reorders updates by key rather than preserving arrival order) could produce different final
  results -- the module's own doc comment acknowledges this directly ("there is a light presumption
  of commutativity ... non-commutative semigroups should be used with care").
- `IsZero::is_zero()` must correctly recognize every case where accumulated diffs truly cancel to
  nothing, or the system will retain updates that have no real effect (a correctness-preserving but
  space/performance cost, not a soundness bug) -- conversely it must NEVER return true for a
  non-zero value, which WOULD be a soundness bug (silently discarding a real, still-live update).

## Identity/scope model

A record's identity in one collection is `(key, val)`; its full history is the multiset of
`(time, diff)` pairs accumulated for that `(key, val)` across all updates ever applied -- directly
generalizing both Salsa's per-query `DatabaseKeyIndex` identity and datafrog's per-`Tuple` identity
into a shared `(identity, time, algebraic-value)` shape.

## State/effect/resource model

A `Trace` is the accumulated, indexed history of a collection's updates -- real, potentially large,
persistent-within-the-computation state (unlike datafrog's transient stable/recent/to_add vectors,
which are discarded once the `Iteration` completes). This is a genuinely different resource
commitment: differential dataflow is designed to keep a computation's derivation graph ALIVE across
many rounds of incremental updates, not to run once to a fixed point and discard its intermediate
state.

## Failure and recovery behavior

Not evaluated in this record (out of scope -- this record covers the diff-algebra and time-lattice
mechanisms specifically, not the framework's operational/fault-tolerance model, which in a
distributed `timely-dataflow` deployment is a substantial separate topic).

## Concurrency/temporal behavior

Built for genuine distributed, multi-worker execution via `timely-dataflow` (confirmed by `lib.rs`'s
own framing: "automatically parallelizes across multiple threads, processes, and computers") --
categorically more ambitious than either Salsa's (single-process, optionally multi-threaded) or
datafrog's (single-threaded) concurrency model. Not evaluated further; Atlas's own census pipeline
remains single-threaded (per prior genome records' repeated finding), so this is the least
immediately relevant part of this donor for Atlas's CURRENT scale, though potentially relevant if
Atlas's own census ever needs to scale beyond one process.

## Performance characteristics

Not benchmarked in this record. The `benches/` directory exists in the donor's own source
(confirmed present, not read) but was not examined.

## Portability/ABI constraints

Pure Rust, built on `timely-dataflow`; no FFI surface relevant to this record's scope.

## Evidence references

- `.atlas/temporary/donors/differential-dataflow/differential-dataflow/src/lib.rs` (module doc
  comment and worked example, read in full -- 147 lines)
- `.atlas/temporary/donors/differential-dataflow/differential-dataflow/src/difference.rs` (read in
  full: `IsZero`/`Semigroup`/`Monoid`/`Abelian` trait hierarchy and their own doc comments)
- `.atlas/temporary/donors/differential-dataflow/differential-dataflow/src/trace/mod.rs` (module doc
  comment and `TraceReader` trait's opening definition, confirmed `(key, val, time, diff)` shape and
  the `Antichain`/`Lattice` imports)
- `.atlas/genome/technology/salsa-durability-gated-incremental-verification.md`,
  `salsa-fixpoint-cycle-iteration.md`, and `datafrog-semi-naive-fixpoint-evaluation.md` (the three
  companion records this record directly extends and whose identified gap motivated this
  investigation)
- `.atlas/roadmap/SELF-BUILDING-R4-R8.md` R5 section

## Donor revisions/licenses

TimelyDataflow/differential-dataflow, commit `aa8745f93ea8abe131104fc7885ba4fd47e63902`. Not
independently re-verified in this record (license files not read this pass; out of scope for a
mechanism-only census).

## Known trade-offs

- Substantially more implementation complexity than either Salsa or datafrog alone: a lattice-valued
  time type, an antichain/frontier tracking mechanism, and an algebraic diff type are all real
  machinery a query author (or an Atlas-native absorption) must correctly instantiate, versus
  Salsa's scalar revision or datafrog's scalar round counter.
- Built on top of an entire separate distributed-execution framework (`timely-dataflow`) -- absorbing
  this mechanism's ALGORITHM (lattice time + algebraic diffs) does not require absorbing timely-
  dataflow's own distributed-execution architecture; these are separable, and this record's own
  Decision treats them as such.

## Rejected alternatives (for this pass)

- Not evaluated in this record: the operator library (`src/operators/`, joins/reduces/iterate
  implementations), the `arrangement`/index machinery that makes joins efficient over a `Trace`, and
  `timely-dataflow` itself. Each is a substantial, separate census target if R5 implementation work
  ever needs the OPERATIONAL machinery, not merely the conceptual generalization this record covers.

## Dependency/extinction status

`PENDING` per `donor-corpus.toml`, unchanged by this record.

## Decision

**No absorption decision yet.** This record establishes: (1) differential dataflow's core conceptual
contribution -- generalizing "diff" from a signed count to an arbitrary Abelian group element
(enabling uniform retraction, not just insertion) and generalizing "time" from a scalar to a lattice
(unifying Salsa's revision-scalar and datafrog's round-scalar into one representation general enough
to express both nested fixed-point iteration and streaming incremental updates together) -- is now
genuinely understood and evidenced, closing the exact gap the prior datafrog record identified
between Salsa's and datafrog's mechanisms; (2) this conceptual generalization is SEPARABLE from the
donor's own much larger operational machinery (the `timely-dataflow` distributed execution
substrate, the arrangement/index system, the operator library) -- an eventual Atlas-native R5
absorption should evaluate adopting the ALGEBRA (lattice time + Abelian diffs) as a REWRITE against
Atlas's own existing types (`SemanticRecordId`, `Provenance`, the census/normalization pipeline),
not the donor's own distributed-systems architecture, which is disproportionate to Atlas's current
single-process scale; (3) with all three of Salsa/datafrog/differential-dataflow now having at least
one real mechanism record, R5's own conceptual foundation (durability-gated verification,
semi-naive evaluation, and now algebraic-retraction-plus-lattice-time) is well-evidenced enough to
support an actual R5 DESIGN document being attempted in a future generation -- a materially different
kind of work (synthesis across three donors' mechanisms into one Atlas-native design) than any single
further donor deep-dive would be, and arguably now the highest-value next step in this thread.

`census_status` for differential-dataflow in `donor-corpus.toml` is advanced from `SKELETON` to
`COARSE_CENSUSED` (the two essential conceptual generalizations are evidenced; the vast remaining
operational surface -- operators, arrangements, timely-dataflow itself, distributed execution,
fault tolerance -- remains explicitly, substantially uncensused, more so proportionally than for
either Salsa or datafrog given this donor's much larger size). `decision_status` remains `PENDING`.
