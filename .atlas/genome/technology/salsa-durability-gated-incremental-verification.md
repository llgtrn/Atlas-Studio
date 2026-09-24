---
id: atlas.genome.technology.salsa-durability-gated-incremental-verification
type: technology-genome
status: active
canonical: true
---
# Technology Genome: durability-gated incremental query verification (Salsa)

Donor: Salsa (`salsa-rs/salsa`, commit `5d8eaf1fa372a73d44f9b717407de41f19ef59d7`), the primary W3
donor named for R5 ("incremental query and fixed-point closure") in
`.atlas/roadmap/SELF-BUILDING-R4-R8.md`. Advances Salsa from `SKELETON` (the existing
`.atlas/census/salsa.md`, source cloned and pinned but not yet inspected) to a real deep-census-
grade mechanism record for its CORE incremental-recomputation-avoidance algorithm specifically,
read directly from `.atlas/temporary/donors/salsa/src/{revision.rs,durability.rs,zalsa_local.rs,
function/{memo.rs,maybe_changed_after.rs}}` -- not from the project's own book/documentation.
This record covers the single mechanism most directly load-bearing for R5's declared required
capabilities; it does not claim full coverage of Salsa's surface (tracked structs, accumulators,
cycle resolution, parallel execution, and the macro-generated ingredient system are all real,
substantial subsystems this record does not evidence -- see Decision for exactly what remains
uncensused).

## Capability / problem

Given a large, mutable corpus of source facts and a graph of derived queries computed from them
(exactly Atlas's own census -> normalization -> graph pipeline), after ONE input changes, determine
which already-computed derived results are still valid and which must be recomputed -- without
either (a) recomputing everything from scratch (defeats the point of incremental computation) or
(b) walking every derived result's full transitive dependency chain on every check (asymptotically
as expensive as recomputation for a deep dependency graph). This is exactly R5's own declared
requirement: "dependency-aware query invalidation" + "incremental recensus of changed source and
affected dependents" without a full recensus.

## Semantic mechanism (as observed in the donor)

- **A single global monotonic `Revision` counter** (`revision.rs`, `NonZeroUsize`-backed,
  `Revision::next` increments by exactly 1). Every input mutation advances this counter by one.
  This is the coarsest possible "has anything at all changed" signal, deliberately cheap (one
  atomic load) to check before anything more expensive is attempted.
- **`Durability` (`durability.rs`): a 4-level (`LOW`/`MEDIUM`/`HIGH`/`NEVER_CHANGE`), explicitly
  user- or system-assigned "how likely is this to change" classification**, independent of the
  Revision counter. The donor's own doc comment states the exact payoff directly: "if we know that
  the only changes were to inputs of low durability ... and we know that the query only used inputs
  of medium durability or higher, then we can skip \[walking every input\]." This is the single
  most important, most directly reusable idea in this record -- see Atlas applicability below.
  `Zalsa` tracks, per durability LEVEL (not per individual input), the most recent `Revision` at
  which ANY input of that level or lower last changed (`last_changed_revision(durability)`,
  confirmed at `function/maybe_changed_after.rs:377`) -- an O(1) lookup, not a graph walk.
  `NEVER_CHANGE` needs no revision slot at all (`Durability::LEN` excludes it, `durability.rs`).
- **Per-memoized-result state, NOT per-input state, is what actually gets checked.** Every memoized
  query result (`Memo`/`MemoHeader`, `function/memo.rs:183-199`) carries two SEPARATE revision
  stamps, not one -- this two-stamp split is the second most important idea in this record:
  - `verified_at`: the revision at which this result was last CONFIRMED still valid (whether or not
    it was actually recomputed).
  - `changed_at` (on `QueryRevisions`, `zalsa_local.rs:502-504`): the revision at which this
    result's OWN VALUE last actually changed -- which can be, and typically is, OLDER than
    `verified_at`. A query can be re-verified (its inputs re-walked) at revision N+5 while its
    output value has not changed since revision N, because the recomputed value happened to equal
    the previous one ("backdating", `function/backdate.rs`). Separating these two facts is exactly
    what lets a query's OWN DEPENDENTS avoid re-running merely because an ancestor was re-verified
    without its value actually changing -- collapsing them into one timestamp (as a naive "last
    touched" design would) would force every dependent to re-run on every re-verification anywhere
    upstream, defeating incrementality at the second layer of the graph.
  - **The verification algorithm itself (`function/maybe_changed_after.rs`,
    `shallow_verify_memo_cold`, confirmed lines 368-388): a SHALLOW check runs first --
    `last_changed_revision(memo's own minimum input durability) <= memo.verified_at`. If true, the
    ENTIRE deep dependency walk is skipped outright (`ShallowUpdate::HigherDurability`) for this
    memo AND, transitively, everything reachable only through it: nothing at or below that
    durability level changed since this was last checked, full stop.** Only when the shallow check
    fails does the algorithm fall back to the expensive path: recursively calling
    `maybe_changed_after` on each individually-tracked dependency read during the query's last
    execution (`QueryOrigin`'s recorded reads), taking the whole thing as changed the moment any one
    dependency reports changed. This two-tier (cheap durability-bucket check, expensive per-edge
    walk only on demand) structure is the actual "incremental" part of incremental computation here
    -- not memoization alone (a plain hashmap cache is not incremental in the invalidation sense;
    Salsa's contribution is a CHEAP, SOUND way to decide when the cache can be trusted without
    re-deriving from scratch).
- **Provenance is a first-class, recorded fact, not reconstructed after the fact.** `QueryOrigin`
  (referenced via `QueryRevisions::origin_and_extra`) records exactly which inputs were read during
  a query's LAST execution, at that execution's own durability -- directly analogous to R5's own
  "evidence/provenance lineage through derived facts" requirement, and to Atlas's own existing
  `Provenance`/`EvidenceId` discipline (already load-bearing in every `SemanticRecordHeader` this
  session's own R4 work produces) -- the concepts are already compatible in spirit, not merely
  superficially similar.

## Required invariants

- **Durability assignment must be conservative (never overstated), or the shallow check becomes
  UNSOUND.** If a genuinely LOW-durability input (frequently changes) were mis-classified as HIGH,
  the shallow check could wrongly report "nothing changed" and skip real invalidation -- a false
  negative, the one failure mode this whole mechanism must never produce. The donor's own doc
  comment frames this directly: durability is an OPTIMIZATION hint about *likelihood*, and the
  correctness of the deep-walk fallback path (which is always sound, just possibly slower) is what
  actually guarantees correctness; the shallow path is a strict, provable OVER-APPROXIMATION of "no
  need to recheck", never an under-approximation.
- Revision numbers must be strictly monotonically increasing and globally consistent across the
  whole database for the `<=` comparison in `shallow_verify_memo_cold` to mean anything -- this rules
  out, e.g., per-shard or per-thread independent revision counters without a single global
  serialization point (confirmed: `Revision`/`AtomicRevision` are process-global, `runtime.rs`).

## Identity/scope model

A memoized query result's identity is keyed by `(query function, input key)` (`DatabaseKeyIndex`);
its TEMPORAL state (`verified_at`/`changed_at`/`durability`) is attached to that same identity, not
tracked separately -- directly analogous to Atlas's own `SemanticRecordId` (identity) vs.
`EpistemicStatus`/`Provenance` (temporal/evidentiary state attached to that identity) split already
established in `core::semantic`.

## State/effect/resource model

Purely in-process, in-memory state (the `Zalsa` database + its memo tables); no I/O, no persistence
by default (`persistence` is an optional, separately-gated feature for serializing the whole memo
table, not evaluated further here -- out of this record's scope).

## Failure and recovery behavior

Cycle detection exists (`cycle.rs`, `ProvisionalStatus`, `IterationStamp`) for queries that
transitively depend on themselves -- real, substantial machinery this record does NOT evidence in
depth (out of scope: this record is about the linear-dependency verification algorithm, not fixed-
point cycle resolution, even though R5's own name references "fixed-point closure" and cycle
resolution is very likely the concrete mechanism that phrase is asking for -- flagged as the natural
next slice of THIS donor's own census, not attempted this record).

## Concurrency/temporal behavior

`AtomicRevision`/`AtomicUsize` throughout confirm the design is built for concurrent readers against
a single writer-serialized revision counter (`sync.rs`, `Cancelled` -- a running query is cancelled
if the database is mutated concurrently, rather than allowed to observe a torn revision). Not
evaluated further; Atlas's own census pipeline is confirmed single-threaded today (per the prior
BLAKE3 genome record's own finding), so this concurrency machinery is not yet a consumable mechanism
for Atlas, only a design constraint to keep in mind if/when R5 work parallelizes recensus.

## Performance characteristics

Not benchmarked directly. The mechanism's own stated performance claim (durability-bucket shallow
check is O(1) vs. O(dependency-subgraph size) for the deep walk) is a structural property evident
from the code shape itself (an atomic load-and-compare vs. a recursive walk), not something this
record independently measured.

## Portability/ABI constraints

Pure Rust library, no FFI/native surface; not relevant to this mechanism.

## Evidence references

- `.atlas/temporary/donors/salsa/src/revision.rs` (full file read: `Revision`/`AtomicRevision`
  structure, `next()`)
- `.atlas/temporary/donors/salsa/src/durability.rs` (full file read: 4-level enum, doc comment
  stating the exact optimization rationale, `LEN`/`MIN`/`MAX`)
- `.atlas/temporary/donors/salsa/src/function/memo.rs` (`MemoHeader` at lines 183-199,
  `verified_at` field and its `AtomicRevision` type)
- `.atlas/temporary/donors/salsa/src/zalsa_local.rs` (`QueryRevisions` struct at lines 502-526:
  `changed_at`/`durability`/`origin_and_extra` fields)
- `.atlas/temporary/donors/salsa/src/function/maybe_changed_after.rs` (`shallow_verify_memo_cold`
  at lines 368-388, confirmed exact shallow-check logic:
  `last_changed_revision(durability) <= verified_at`)
- `.atlas/census/salsa.md` (existing SKELETON census -- this record supersedes it for the core
  verification-algorithm mechanism specifically, not Salsa's full surface)
- `.atlas/roadmap/SELF-BUILDING-R4-R8.md` R5 section (the six required capabilities this record
  cross-references directly)

## Donor revisions/licenses

salsa-rs/salsa, commit `5d8eaf1fa372a73d44f9b717407de41f19ef59d7`. Dual Apache-2.0/MIT (standard
Rust-ecosystem permissive licensing) per the donor's own repository root -- not a constraint on
either dependency or reference-reimplementation absorption.

## Known trade-offs

- Durability is a manually-assigned classification, not automatically inferred -- Atlas would need
  its own policy for classifying census inputs by durability (e.g.: a source file's OWN content is
  LOW; a `Cargo.lock`/manifest is MEDIUM; a donor's pinned commit/admitted dependency version is
  HIGH; a canonical contract/schema version is closer to NEVER_CHANGE within one Atlas revision).
  Getting this classification wrong in the unsafe direction (over-stating durability) is the one
  correctness risk this whole mechanism carries, per Required invariants above.
- The two-tier verification algorithm is more complex than a plain "hash the inputs, compare"
  memoization scheme; the complexity is the whole point (it is what makes the deep-walk case avoidable
  most of the time), but it is real implementation surface, not a trivial reuse.

## Rejected alternatives (for this pass)

- **Vendoring/depending on the `salsa` crate directly** -- not evaluated in this record; Atlas's
  census/normalization pipeline has its own existing typed data model (`CensusReport`,
  `EngineeringGraph`, `SemanticRecordHeader`) that Salsa's own macro-generated `#[salsa::query]`
  ingredient system was not designed around, and retrofitting Atlas's data model onto Salsa's
  database/ingredient framework (rather than the reverse: absorbing Salsa's ALGORITHM into Atlas's
  own existing types) is a materially different, larger design decision this record does not make.
  Recorded as an open question for whoever designs R5's actual implementation, not resolved here.

## Dependency/extinction status

`PENDING` per `donor-corpus.toml`, unchanged by this record (this record advances census depth, not
the absorption decision itself -- see Decision).

## Decision

**No absorption decision yet -- this record's scope is deep-census of the core mechanism only,**
per the SKELETON census's own stated next steps ("classify modules, algorithms, storage, execution,
query, incremental behavior ..."). What this record concretely establishes, durably, so it is not
rediscovered later:

1. Salsa's core incremental-verification algorithm (global revision counter + per-input-class
   durability + per-memo verified_at/changed_at split + shallow-then-deep two-tier check) is a real,
   evidenced, well-understood mechanism directly answering R5's "dependency-aware query invalidation"
   and "revision-scoped cached derivation" requirements -- not a black box Atlas would need to
   reverse-engineer from behavior alone when R5 implementation work actually begins.
2. The ESSENTIAL mechanism (durability-gated shallow verification before a deep dependency walk) is
   separable from the ACCIDENTAL implementation detail (Salsa's own macro-generated ingredient/
   database architecture, `unsafe` pointer-based memo storage for `dyn Any`-style type erasure,
   parallel/concurrent execution machinery) -- Atlas's own eventual R5 implementation should absorb
   the ALGORITHM (REWRITE disposition against Atlas's own existing `SemanticRecordId`/`Provenance`
   types), not the donor's own storage/concurrency architecture, matching
   `DONOR-TO-LANGUAGE-GENESIS.md`'s general REWRITE-by-default posture (no cryptographic-primitive-
   style carve-out applies here, unlike the BLAKE3 record).
3. Explicitly NOT yet censused, and recorded as the natural next slice of THIS donor's own deep
   census (not attempted this record, to keep this record's own verification bounded and reviewable):
   cycle detection/fixed-point resolution (`cycle.rs`) -- likely the concrete mechanism behind R5's
   own "fixed-point" naming, and probably the single most important remaining gap in this census;
   tracked structs (`tracked_struct.rs`); accumulators; the macro-generated ingredient/ID system
   (`components/salsa-macros`); parallel execution (`sync.rs`, the `parallel` test suite).

`census_status` for Salsa in `donor-corpus.toml` is advanced from `SKELETON` to
`COARSE_CENSUSED` (a real, evidenced understanding of the core mechanism exists now, superseding
"source cloned and pinned, not yet inspected") -- not `DEEP_CENSUSED`, since the substantial
remaining subsystems named above (especially cycle/fixed-point resolution) are still unexamined.
`decision_status` remains `PENDING`, correctly: R5 implementation work itself has not started, and
an absorption disposition (REWRITE/ABSORB_NOW/etc.) for a mechanism, as opposed to a concrete API
surface, is premature before that work's own design is underway.
