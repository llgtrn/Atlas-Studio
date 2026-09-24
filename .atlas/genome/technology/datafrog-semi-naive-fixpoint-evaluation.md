---
id: atlas.genome.technology.datafrog-semi-naive-fixpoint-evaluation
type: technology-genome
status: active
canonical: true
---
# Technology Genome: semi-naive bottom-up Datalog fixed-point evaluation (datafrog)

Donor: datafrog (`rust-lang/datafrog`, commit `edd7cfc0f5ad3b93ce5bbe6a17848db9376b13c0`), the second
W3 donor named in `.atlas/roadmap/SELF-BUILDING-R4-R8.md`'s R5 lane, following
`salsa-durability-gated-incremental-verification.md` and
`salsa-fixpoint-cycle-iteration.md`. Advances datafrog from `SKELETON` to `COARSE_CENSUSED`, read
directly from `.atlas/temporary/donors/datafrog/src/{lib.rs,iteration.rs,variable.rs}` (the whole
crate is small, ~2,000 lines total across 9 files; `variable.rs` alone -- the core mechanism -- was
read in full, not sampled).

## Capability / problem

Given a set of BASE facts and a set of monotonic derivation rules (joins/maps/filters that only ever
ADD facts, never retract them), compute the full closure -- every fact derivable by repeatedly
applying the rules until nothing new is produced -- as efficiently as possible. This is a DIFFERENT
point in the same design space Salsa occupies: Salsa answers "given ONE changed input, what
minimal set of already-computed results need re-checking" (top-down, demand-driven, one query at a
time); datafrog answers "given a WHOLE base fact set, compute the ENTIRE closure" (bottom-up,
eager, the whole computation at once). R5 names both because Atlas's own future incremental-
recensus needs are likely to need BOTH: Salsa's shape for "recheck what a single source-file edit
invalidates", datafrog's shape for "given the current admitted fact set, derive the full
transitive-dependency/capability closure" -- exactly what
`core::census::dependency::DependencyClosureReport` already computes today, currently without this
donor's own semi-naive optimization (not re-examined in this record whether it should be; recorded
as an open question below).

## Semantic mechanism (as observed in the donor)

- **Semi-naive evaluation via a three-stage tuple lifecycle** (module doc comment on `Variable`,
  `variable.rs:22-38`, confirmed): a fact added to a `Variable` first sits in `to_add` (not yet
  visible to any join), is promoted to `recent` for exactly one round (visible to joins, and it is
  the CHANGED set that makes that round's derivations differ from the previous round's), then is
  folded into `stable` (permanent, no longer separately re-considered). This is the textbook
  semi-naive Datalog technique: a join rule only needs to consider `recent x stable` and
  `recent x recent` combinations each round, never `stable x stable` again -- because any fact
  derivable purely from already-stable inputs would already have been derived in an earlier round.
  Without this staging, a naive bottom-up evaluator would re-derive the ENTIRE stable fact set's
  joins every round, making total work quadratic (or worse) in the number of rounds rather than
  proportional to the number of genuinely NEW facts.
- **`Iteration::changed()` as the fixed-point driver** (`iteration.rs`, confirmed): a plain
  `while iteration.changed() { ... }` loop at the call site (the crate's own convention, confirmed
  by every example in `src/test.rs`) -- `changed()` returns `true` if ANY tracked `Variable` still
  had a non-empty `recent` set this round (after performing the stage-rotation below), `false` once
  every variable has stabilized. Compared to Salsa's own convergence check (dual value+metadata
  equality, per iteration, PER QUERY), this is coarser and eager: the WHOLE computation runs to a
  global fixed point in one pass, there is no per-fact "is this specific derivation still needed"
  decision -- appropriate because datafrog assumes it is deriving the closure of a KNOWN, currently-
  fixed base fact set, not reacting to a single incremental edit.
- **A geometric/doubling merge strategy keeps `stable` from becoming one costly-to-resort blob**
  (`variable.rs:340-356`, confirmed, read directly): `stable` is `Vec<Relation<Tuple>>` -- multiple
  separately-sorted batches, not one flat sorted vector. When `recent` is folded in, the code
  repeatedly pops and merges the LAST stable batch into the incoming one WHILE that popped batch's
  length is `<= 2 * incoming length` (`variable.rs:349`) -- the exact "binary counter" amortization
  trick also used in binary-counter-style incremental data structures: this keeps the number of
  separate stable batches at O(log(total facts)) rather than 1, and keeps the AMORTIZED cost of
  merging in N new facts over the whole computation at O(N log N) rather than the O(N * rounds) a
  naive "always re-sort everything" strategy would cost.
- **Adaptive galloping-vs-linear deduplication when merging `to_add` against `stable`**
  (`variable.rs:365-378`, confirmed, read directly): when checking whether an incoming candidate
  fact already exists in a stable batch, the code chooses between two different algorithms based on
  the RELATIVE size of the two sides -- a galloping/exponential-skip search (`join::gallop`) when the
  stable batch is more than 4x larger than the incoming batch, a plain linear merge-scan otherwise.
  This is a real, deliberate, evidenced engineering trade-off (not a hardcoded single algorithm):
  galloping wins when scanning a large sorted set for a few needles, linear merge wins when both
  sides are comparably sized (galloping's own overhead is not worth paying then).
- **Distinctness is an explicit, documented opt-out with a stated correctness obligation, not a
  hidden default** (`variable.rs:35-38`, `Iteration::variable_indistinct`, confirmed): a `Variable`
  normally deduplicates every fact against everything already known (`distinct: true` -- required
  for the semi-naive staging above to terminate: an indistinct variable can advertise the SAME fact
  as "recent" unboundedly many times, since nothing ever confirms it was already seen). The crate's
  own doc comment states the invariant plainly: "it is important that any cycle of derivations have
  at least one de-duplicating variable on it" -- an indistinct variable trades this termination
  guarantee for raw throughput when the caller can prove some OTHER variable in the same derivation
  cycle already deduplicates.

## Required invariants

- Every derivation rule must be MONOTONE (only ever adds facts, never removes/modifies) -- the
  entire semi-naive staging strategy (recent-only reconsideration) is unsound for a non-monotone
  rule, since a later round could need to reconsider a fact that was already "settled" into stable.
- At least one `distinct` (deduplicating) variable must exist on every derivation cycle, or the
  computation may never terminate -- an invariant this crate explicitly documents but, like Salsa's
  own monotone-recovery-function requirement, cannot itself enforce or verify.

## Identity/scope model

A `Tuple` is any `Ord`-implementing type (the crate's own generic bound, `variable.rs:39` and
throughout) -- identity and ordering are the SAME relation (sortedness is how stable/recent/to_add
are merged and deduplicated); there is no separate identity-vs-content distinction the way Atlas's
own `SemanticRecordId` (identity) vs. observation payload (content) split maintains. A direct native
port would need to decide which Atlas identity type plays the role of the sorted `Tuple` key for
each specific fixed-point computation, not a generic decision this record can make once.

## State/effect/resource model

Pure, in-memory, single-threaded (confirmed: no `Arc`/threading primitives anywhere in the crate,
unlike Salsa's own `sync.rs`) -- a plain library dropped into an existing single-threaded Rust
program, computed to completion, then read out as plain `Vec`s (module doc comment, `lib.rs`,
confirmed). No I/O, no persistence, no concurrency at all -- the simplest possible resource model of
the three donors censused in this R5 lane so far.

## Failure and recovery behavior

None -- the crate does not model failure at all; a non-terminating (non-monotone or fully
indistinct-cycle) computation simply loops forever, with no analog to Salsa's `MAX_ITERATIONS` safety
net. This is a real, concrete gap relative to Salsa's own design that any Atlas-native absorption of
this pattern should not silently inherit.

## Concurrency/temporal behavior

None; single-threaded by construction (see State/effect/resource model above).

## Performance characteristics

Not benchmarked directly by this record; the crate's own `benches/` directory (not read this record)
presumably has real numbers. The two evidenced techniques above (geometric batch merging, adaptive
galloping) are both real, non-trivial engineering specifically aimed at keeping per-round cost
proportional to NEW work, not total accumulated work -- the crate's own design intent, confirmed by
its actual code shape, not merely asserted by its README.

## Portability/ABI constraints

Pure Rust, no FFI surface; not relevant.

## Evidence references

- `.atlas/temporary/donors/datafrog/src/lib.rs` (full file read: module doc comment, public API
  surface)
- `.atlas/temporary/donors/datafrog/src/iteration.rs` (full file read: `Iteration::changed()`
  driver loop, `variable`/`variable_indistinct` constructors)
- `.atlas/temporary/donors/datafrog/src/variable.rs` (read in full, 428 lines: `Variable`'s own
  module doc comment at lines 22-38 for the three-stage lifecycle; `changed()` implementation at
  lines 340-390 for the geometric merge at 340-356 and adaptive galloping deduplication at 365-378)
- `.atlas/census/datafrog.md` (existing SKELETON census, superseded by this record for the
  core evaluation-strategy mechanism specifically)
- `.atlas/genome/technology/salsa-durability-gated-incremental-verification.md` and
  `salsa-fixpoint-cycle-iteration.md` (the companion records this donor is explicitly contrasted
  against throughout this record)
- `.atlas/roadmap/SELF-BUILDING-R4-R8.md` R5 section

## Donor revisions/licenses

rust-lang/datafrog, commit `edd7cfc0f5ad3b93ce5bbe6a17848db9376b13c0`. Part of the `rust-lang` GitHub
organization (used internally by rustc's own NLL/borrow-check implementation, per the donor's own
`examples/borrow_check.rs`) -- standard Rust-ecosystem dual Apache-2.0/MIT licensing expected,
not independently re-verified in this record (out of scope; license files not read this pass).

## Known trade-offs

- No termination safety net (unlike Salsa's `MAX_ITERATIONS`) -- a real, evidenced gap this record
  specifically flags, not merely inferred by absence.
- Single-threaded only -- simpler than Salsa, but cannot exploit parallelism the way Salsa's own
  (uncensused) concurrent execution machinery can.
- Whole-computation-at-once: there is no notion of "only recompute what one changed input affects"
  the way Salsa's entire design is built around -- datafrog assumes the base fact set is fixed for
  the duration of one `Iteration`, then discarded. Reusing a datafrog-shaped computation across
  MULTIPLE incremental edits (Atlas's actual R5 scenario) would require re-running the whole
  fixed-point from scratch each time, unless combined with something like Salsa's own mechanism to
  decide WHEN a full recomputation is actually needed -- exactly why R5 plausibly needs both
  donors' mechanisms together, not either alone.

## Rejected alternatives (for this pass)

- Not evaluated in this record: `differential-dataflow` (the third, most complex W3 donor,
  explicitly built to give datafrog-style bottom-up evaluation actual INCREMENTAL VIEW MAINTENANCE
  -- i.e. exactly the "combine datafrog's throughput with Salsa's incrementality" capability this
  record's own Known trade-offs section identifies as the gap between the two donors already
  censused). Recorded as the clear next-highest-value W3 donor to census, given this record's own
  finding that neither Salsa nor datafrog alone covers both needs.

## Dependency/extinction status

Originally `PENDING`. Superseded -- see "Decision update (2026-09-24)" below.

## Decision

**No absorption decision yet.** This record establishes: (1) datafrog's semi-naive bottom-up
evaluation strategy (three-stage tuple lifecycle, geometric batch merging, adaptive galloping
deduplication) is a real, evidenced, DIFFERENT mechanism from Salsa's own, not a redundant second
donor covering the same ground -- confirming R5's own roadmap judgment that both are needed; (2) the
two donors' mechanisms are complementary, not substitutable: Salsa answers "what does one change
invalidate", datafrog answers "what is the full closure of a fixed fact set", and Atlas's own R5
requirements ("incremental recensus of changed source and affected dependents" + "recursive/
fixed-point semantic derivation") plausibly need both, composed, not either alone; (3)
`differential-dataflow` -- explicitly built to close exactly this gap (incremental bottom-up
fixed-point maintenance) -- is now the clear highest-value next W3 donor to census, identified by
this record's own finding rather than merely because the roadmap lists it.

`census_status` for datafrog in `donor-corpus.toml` is advanced from `SKELETON` to
`COARSE_CENSUSED` (core evaluation-strategy mechanism now evidenced; the treefrog/leaper "worst-case
optimal join" machinery in `treefrog.rs` -- the crate's single largest file at 795 lines -- and the
`join.rs`/`merge.rs`/`map.rs` operator implementations remain explicitly uncensused).
`decision_status` remains `PENDING`.

## Decision update (2026-09-24)

The "no absorption decision yet" above is superseded. The semi-naive delta mechanism described in this
record (three-stage tuple lifecycle; `changed()` folding `recent` into `stable` and promoting only
not-yet-stable tuples) is **absorbed** as `core::closure` and drives origin-labelled transitive
dependency reachability in `adapter::census_cargo_workspace` -- `.atlas/decisions/0004-semi-naive-
dependency-closure.md`, discovery record `.atlas/census/discoveries/r5-semi-naive-dependency-closure.md`.
The absorbed invariant is the delta discipline and round accounting only; the geometric batch-merge
layout and galloping deduplication are storage optimisations that `BTreeSet` replaces, and the
treefrog/leaper join plus `join.rs`/`merge.rs`/`map.rs` operators are dispositioned `REFERENCE_ONLY`
(no Atlas caller). This record's own point (2) -- that Salsa and datafrog are complementary, not
substitutable -- still stands: only the closure half is absorbed.
