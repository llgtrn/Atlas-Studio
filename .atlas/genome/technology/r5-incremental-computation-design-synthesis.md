---
id: atlas.genome.technology.r5-incremental-computation-design-synthesis
type: technology-genome
status: active
canonical: true
---
# Technology Genome: R5 design synthesis — incremental query and fixed-point closure

Synthesis across the five real, evidenced W3-lane mechanism records now complete in this directory:
`salsa-durability-gated-incremental-verification.md`, `salsa-fixpoint-cycle-iteration.md`,
`datafrog-semi-naive-fixpoint-evaluation.md`,
`differential-dataflow-algebraic-retraction-and-lattice-time.md`, and
`souffle-provenance-backed-derivation.md`. The fifth record's own Decision explicitly named this
document as a plausible, evidenced next step ("an actual R5 design-synthesis document ... unifying
five real, evidenced mechanisms into one Atlas-native design") and equally explicitly did not
attempt it. This record attempts it.

All five donors remain `census_status = COARSE_CENSUSED`, `decision_status = PENDING` in
`.atlas/references/donor-corpus.toml`, confirmed by direct read of the corpus file for this record
(`salsa`, `datafrog`, `differential-dataflow`, `souffle`, `buck2` entries). **This document does not
change any donor's `census_status`/`decision_status` and does not itself constitute an absorption
decision for any of the five mechanisms.** It is a cross-donor reasoning pass grounded in (a) the
five genome records' own text, read in full, and (b) Atlas's own current, real N0
normalization/graph-projection layer (`runtime/src/normalize/mod.rs`, `core/src/graph/
engineering_graph.rs`, `core/src/semantic/mod.rs`, `core/src/evidence/mod.rs`,
`core/src/census/dependency.rs`), read directly for this record specifically to ground the synthesis
in what Atlas already has rather than free-floating theory. R5's own required-capabilities list is
quoted verbatim from `.atlas/roadmap/SELF-BUILDING-R4-R8.md` (`## R5 — incremental query and
fixed-point closure`, lines 476–491) throughout.

## Capability / problem

R5's required capabilities, as literally stated in the roadmap, are six:

1. dependency-aware query invalidation
2. revision-scoped cached derivation
3. recursive/fixed-point semantic derivation
4. incremental recensus of changed source and affected dependents
5. deterministic propagation
6. evidence/provenance lineage through derived facts

No branching/concurrent-lineage capability appears in this list. That absence is doctrinally
significant and is addressed directly in this record's §(c) below — it is not an oversight this
document is correcting, it is a finding this document confirms by checking the roadmap text against
what buck2/Dice's own genome record actually claims to address.

## What Atlas already has (the real substrate this design must sit on)

Read directly for this record, not assumed:

- **`normalize()` (`runtime/src/normalize/mod.rs`) is a pure, batch, from-scratch function**: it takes
  one whole `CensusReport` and returns one whole `NormalizationReport`. There is no notion anywhere in
  this file of "given the previous `NormalizationReport` and a delta, produce the next one" — every
  call recomputes every fact's canonicalization, re-sorts everything, and re-runs
  `detect_conflict_candidates` over the ENTIRE typed-record set. This is confirmed, not inferred, by
  the function signature (`fn normalize(census: &CensusReport) -> NormalizationReport`) and by the
  absence of any cached/previous-report parameter.
- **`build_system_graph` (`core/src/graph/engineering_graph.rs`) is likewise a full, from-scratch
  projection**: it calls `build_repository_graph` (itself calling `build_source_graph`), then walks
  `normalization.facts` in full, appending nodes/edges for the whole set. `ensure_node` deduplicates
  within one build pass (`graph.nodes.iter().any(|node| node.id == id)` — an O(n) linear scan per
  insertion, not an incremental index) but nothing here reuses a PRIOR graph across calls.
- **Identity is content-addressed and revision-independent**: `SemanticRecordId::new(dimension,
  identity_key)` (`core/src/semantic/mod.rs:96-107`) is `stable_id(format!("semantic:{dimension}"),
  identity_key)` — a deterministic hash of the dimension and the family's own `identity_key()`, with
  NO revision, timestamp, or sequence number folded in. Two calls with the same dimension and identity
  key always produce the same `SemanticRecordId`, regardless of when or how many times they are
  computed. `SemanticRecordHeader<Subject>` (`mod.rs:154-165`) carries `record_id` (identity),
  `status: EpistemicStatus` (per ADR 0002's canonical vocabulary — `OBSERVED, DECLARED, DERIVED,
  INFERRED, HYPOTHESIS, CONFLICT, UNKNOWN, UNSUPPORTED, IGNORED`), `revision: RevisionRef` (a
  `{kind: "git", value: <sha>}` pair — an EXTERNAL, whole-repository-scoped coordinate, not an
  Atlas-internal monotonic counter), and `evidence_refs: Vec<EvidenceId>` — a set of references into a
  separate `Evidence` ledger (`core/src/evidence/mod.rs`: `id, kind, path, summary, revision`).
- **There is no temporal/verification state on a record beyond `EpistemicStatus` and `RevisionRef`.**
  No field anywhere in `SemanticRecordHeader`, `Evidence`, or `NormalizationReport` distinguishes "when
  this was last confirmed still valid" from "when this last actually changed" the way Salsa's
  `verified_at`/`changed_at` split does. No field records which OTHER `record_id`s were read to derive
  a given record (`evidence_refs` records evidence-ledger entries — raw observation provenance — not
  a dependency-on-other-derived-records relation). No field carries a derivation-rule identity or a
  proof-height/level analogous to Souffle's `@rule_num`/`@level_num`.
- **Nothing is retracted; nothing persists live across runs.** `normalize()`/`build_system_graph`
  compute forward from whatever `CensusReport` they are handed; a fact absent from the current
  `CensusReport` is simply absent from the next `NormalizationReport`/graph — there is no explicit
  negative fact, tombstone, or `RETRACTED` epistemic state (ADR 0002's nine-value vocabulary has none).
  Confirmed by grep: no `incremental`, `recompute`, `cache`, or `memoi[ze]` term appears anywhere in
  `core/src`, and the only two hits in `runtime/src` are unrelated (a persistence-kind doc comment in
  `core/src/semantic/persistence.rs`, and a test name asserting epistemic status is "preserved
  verbatim not recomputed" in `runtime/src/census/extraction.rs`). **R5 is a genuine greenfield gap:
  Atlas today has a batch-recompute-everything model with no incremental-computation infrastructure
  at all**, not a partial one that needs extending.
- **`DependencyClosureReport` (`core/src/census/dependency.rs:343-367`) is the one existing
  "closure"-shaped computation in the codebase**, but it is a flat, one-shot resolution of a lockfile's
  own already-materialized edge list (`edges: Vec<DependencyEdge>`), not an iterative fixed-point
  computed by Atlas's own rule application — its acyclicity comes from `Cargo.lock`'s own DAG
  guarantee, not from any cycle-safety property Atlas's code establishes. This matches the open
  question `salsa-fixpoint-cycle-iteration.md` itself flagged and leaves unresolved (see Open
  questions below).
- Atlas's own `schema::mod` already has a "closure" DISCIPLINE — `census_closure_fails_if_a_typed_
  observation_disappears`-style checks (`core/src/schema/mod.rs`) verify that every reference (evidence,
  diagnostic, obligation) resolves within a report before accepting it as closed. This is a
  **referential-completeness check on a fixed snapshot**, not incremental re-verification across
  snapshots — a real, existing discipline R5 should extend, not a substitute for it.

## Cross-donor synthesis

### (a) Complementary mechanisms that compose

- **Souffle's provenance pattern is evaluation-strategy-agnostic, and is the cleanest fit onto
  Atlas's own EXISTING identity/evidence split.** Souffle's own record explicitly frames provenance as
  "layered onto the existing semi-naive evaluation, not a competing evaluation algorithm" — the same
  `provenance::UnitTranslator` subclasses `seminaive::UnitTranslator`, changing only what metadata
  rides along, never what is computed. This generalizes cleanly: provenance-as-inline-metadata is
  orthogonal to WHICH of Salsa/datafrog/differential-dataflow's evaluation strategies Atlas eventually
  picks, because its essential shape (identity stays primary-columns-only; `@rule_num`/`@level_num`
  ride as auxiliary, non-identity-affecting fields) is exactly the shape Atlas's OWN
  `SemanticRecordId` (identity) vs. `SemanticRecordHeader` (identity plus attached temporal/evidentiary
  state) split already has for LEAF/observed facts. The souffle genome record itself notices this
  structural match ("souffle's contribution is showing that split can be pushed all the way down to
  the DERIVED-fact storage layer itself, not only the leaf-observation layer Atlas currently applies
  it to") — this record confirms that finding still holds after actually reading Atlas's current
  header type: `SemanticRecordHeader<Subject>` has no derived-fact-specific fields today, so extending
  provenance to derived facts is additive (new fields on a derived-record's header or a parallel
  derived-record type), not a redesign of the identity model.
- **Salsa's recorded-dependency-read list (`QueryOrigin`) and Souffle's subproof mechanism answer two
  different questions from the same kind of underlying data.** Salsa records "which inputs were read"
  to decide WHAT TO RECHECK when something upstream changes (an invalidation consumer). Souffle
  records "which rule and which supporting tuples" to answer WHY a fact holds (an explanation
  consumer). Both need a record-level list of "what this derivation actually depended on" — Atlas has
  no such list today (`evidence_refs` is raw-observation provenance, not derived-record-to-derived-
  record dependency). A single Atlas-native "dependency edges recorded at derivation time" structure
  could plausibly serve BOTH consumers if it records enough (which upstream `record_id`s, at which
  rule/derivation identity) — this is a real design opportunity, not confirmed as trivial: Salsa's own
  genome record notes `QueryOrigin`'s exact shape was read only at the level of "records reads," not
  benchmarked or fully censused for cost, and Souffle's own subproof mechanism re-executes a rule body
  rather than merely replaying a stored dependency list, which is a different (and possibly more
  robust, since it never desyncs from a stale list) implementation strategy this record does not
  reconcile with Salsa's approach.
- **datafrog's semi-naive three-stage lifecycle is a plausible internal execution strategy for
  `DependencyClosureReport`-shaped computations specifically, if and when they need repeated
  incremental recomputation rather than one-shot resolution.** Today `DependencyClosureReport` is
  computed once per census pass directly from a parsed lockfile — there is no evidence in
  `core/src/census/dependency.rs` of a rule-based, multi-round derivation datafrog's staging would
  actually help with. This composition is currently HYPOTHETICAL, flagged as such by datafrog's own
  genome record ("Atlas's own future incremental-recensus needs are likely to need BOTH") and not
  independently confirmed by this record's own read of `dependency.rs`.

### (b) Genuinely competing alternatives, and the actual tradeoff for Atlas's problem shape

Salsa, datafrog, and differential-dataflow are not three redundant answers to one question; the
datafrog genome record's own Decision states this precisely: Salsa answers "what does one changed
input invalidate" (demand-driven, top-down, one query at a time); datafrog answers "what is the full
closure of a fixed base fact set" (eager, bottom-up, whole computation at once); differential-dataflow
generalizes BOTH into one representation (lattice-valued time unifying Salsa's revision-scalar and
datafrog's round-scalar; Abelian-group-valued diffs generalizing insertion-only facts to include
retraction). The real choice for Atlas is not "which of the three is better" in the abstract — it is:

**does Atlas's actual R5 workload need genuine incremental VIEW MAINTENANCE across many small edits to
an already-large derivation graph (differential-dataflow's specific value proposition), or does it
need two SEPARATE, simpler mechanisms wired together manually (a Salsa-shaped shallow-invalidation
gate in front of a datafrog-shaped from-scratch closure recompute)?**

This record can ground three concrete points toward that tradeoff, without resolving it:

1. **Identity-model fit.** Of the three, Salsa's identity/temporal-state split
   (`DatabaseKeyIndex` = identity; `verified_at`/`changed_at`/`durability` = attached temporal state on
   that same identity) is the closest structural match to Atlas's ALREADY-EXISTING
   `SemanticRecordId`/`SemanticRecordHeader` split — confirmed directly by this record's own read of
   `core/src/semantic/mod.rs`, not merely by the Salsa genome record's own (unverified-at-the-time)
   claim of analogy. datafrog's identity model, by contrast, collapses identity and ordering into one
   `Ord`-bound `Tuple` type (its own genome record's "Identity/scope model" section states this
   directly: "there is no separate identity-vs-content distinction the way Atlas's own
   `SemanticRecordId` ... maintains" — a real, evidenced mismatch, not a superficial one). Adopting
   datafrog's shape would require choosing, for every fixed-point computation, some `Ord`-implementing
   surrogate key standing in for `SemanticRecordId` (which is not itself naturally sortable in a
   semantically meaningful way — it is a stable hash). differential-dataflow's identity model
   (`(key, val)` plus a time-indexed diff history) is a further generalization again, and additionally
   requires Atlas to define a LATTICE type for "time" where today `RevisionRef` is a flat
   `{kind, value}` pair with no join operation defined anywhere in `core/src/temporal` (not read in
   full this record, but no lattice/partial-order machinery appears anywhere in the grepped surface of
   `core/src`). **This is the single most concrete, evidenced finding of this cross-donor comparison:
   Salsa's algorithm fits Atlas's EXISTING data model with the least structural rework; datafrog and
   differential-dataflow each require Atlas to introduce a new supporting concept (a sortable surrogate
   key, or a lattice-typed time domain, respectively) that does not exist in the codebase today.**
   This is a fit argument, not a power argument — it says nothing about which mechanism is more
   CAPABLE for R5's eventual needs, only which is cheapest to graft onto what Atlas already has.
2. **Atlas's revision coordinate is not what any of the three donors assume.** All three donors assume
   a revision/round/time coordinate that is cheap, frequent, and finer-grained than "one whole
   repository state" — Salsa's `Revision` increments on every input mutation (one atomic counter,
   process-global); datafrog's round counter increments once per fixed-point iteration; differential-
   dataflow's lattice time can encode both. Atlas's actual `RevisionRef` is `{kind: "git", value: sha}`
   — ONE value per whole census run, sourced externally from git, not an Atlas-internal counter that
   advances per recomputation. None of the three donors' mechanisms can be adopted by literally reusing
   `RevisionRef` as their revision/time coordinate; each would need Atlas to introduce a SEPARATE,
   internal, more-finely-grained counter (a `Revision`-equivalent, a round-equivalent, or a lattice
   coordinate) alongside the existing git-SHA-shaped `RevisionRef`, which would continue to serve its
   current, different purpose (recording which on-disk state a record came from) rather than double as
   an invalidation clock. This is a real, concrete implementation gap none of the five genome records
   individually surfaced, because each was read against the donor's own source, not against Atlas's
   `RevisionRef` type specifically.
3. **Retraction is currently unmodeled, and only differential-dataflow's mechanism gives it a
   principled, general treatment.** `normalize()`/`build_system_graph` handle a fact's disappearance
   only by omission from the next full rebuild — there is no `RETRACTED` `EpistemicStatus` (ADR 0002's
   nine-value vocabulary has none) and no negative-fact representation anywhere in the schema.
   differential-dataflow's `Abelian::negate` is the one mechanism among the five that makes "this used
   to hold and no longer does" a first-class, uniformly-handled value rather than an absence to infer.
   Whether Atlas's R5 needs this FULL generality (an arbitrary Abelian-group-valued diff, per the
   donor's own `(count, sum)`-pair example for numeric aggregates) or merely a simple boolean
   tombstone/`RETRACTED` status addition to the existing `EpistemicStatus` enum is exactly the kind of
   choice this record flags as open (see Open questions) rather than resolves — Atlas's facts today are
   discrete typed records (symbols, calls, types, ...), not numeric aggregates, so the donor's own
   headline generalization (tracking a running sum incrementally) has no obvious Atlas analog; the
   PART of the mechanism actually motivated by Atlas's real needs is retraction specifically, which is
   a much narrower slice of what differential-dataflow's full algebra provides.

### (c) Dice's branching as a genuinely orthogonal axis, not a competing evaluation strategy

Every one of the first four donors models exactly ONE evolving computation state at a time — Salsa's
single global `Revision`, datafrog's single round sequence, differential-dataflow's single (if
lattice-timed) evolving collection, Souffle's single derivation whose justification is explained.
Buck2's Dice is the only one of the five that makes MULTIPLE, concurrently-live, possibly-diverging
lineages of the SAME computation state a first-class object (`Branch { parent, children, ... }`,
identity generalized to `(Key, Branch, Seq)`). This is not a fourth alternative to Salsa/datafrog/
differential-dataflow for R5's own evaluation-strategy question — it is answerable independently of
which of those three (or which composition of them) Atlas eventually picks for its core evaluation,
because Dice's own core-state crate "deliberately knows nothing about values, execution, or
scheduling" (confirmed directly in the buck2 genome record's own Known trade-offs section) — it is
purely a state-tracking/branch-resolution algorithm layered ABOVE whatever evaluation mechanism
actually computes values.

Checked directly against the roadmap for this record: **Dice's mechanism maps to AIF1 (Multi-AI
Construction Fabric)'s "concurrent speculative CandidateAtlas branches with separate
CandidateChangeSet/evidence lineage" language, not to any of R5's six required capabilities**, which
say nothing about branching or concurrent lineages. AIF1 is itself named, in its own section heading in
`SELF-BUILDING-R4-R8.md`, as "cross-cutting R6→R8, load-bearing in R7" — a separate roadmap item from
R5, not a capability folded into it. (The `R4-R8` in the roadmap FILE's own name spans the whole
document's wave range; it is not AIF1's own specific cross-cutting scope, which the heading states
narrower, as R6→R8 — corrected here after checking the file's own heading text directly, not merely
its filename.) The buck2/Dice genome record's
own Capability/problem section states this mapping explicitly. This record's contribution is
confirming, by checking the roadmap text directly, that this separation is doctrinally correct and
not merely the donor record's own framing: **buck2 sharing R5's W3 donor lane does not mean Dice's
mechanism is required for R5's baseline scope** — it means Dice was censused alongside the other four
because the roadmap's donor-lane grouping is organized by TECHNOLOGY AREA ("incremental query, fixed
point, build/dependency reasoning" per `DONOR-ABSORPTION-ROADMAP.md:311`), not by which roadmap
capability each donor's own mechanism actually satisfies. If and when AIF1 work begins, Dice's
branch-tree/fork/lazy-parent-delegated-resolution pattern would compose with whichever evaluation
strategy R5 settles on (each branch would need its own copy of R5's invalidation/closure state,
resolved lazily against its parent per Dice's own algorithm) — but this record finds no basis in
either the roadmap text or the five genome records for treating Dice's mechanism as something R5
itself must design around now.

## Atlas-native design sketch (deliberately not a final design)

Grounded in the above, an eventual R5 incremental engine for Atlas would plausibly need to track, in
addition to what `SemanticRecordHeader` already carries:

- **An Atlas-internal generation/revision counter**, separate from `RevisionRef`'s git-SHA coordinate,
  that advances once per recensus pass (or, if Atlas's census granularity ever gets finer than "one
  whole run," once per unit of work finer than that) — the cheap, coarse "has anything changed at all"
  signal Salsa's shallow check depends on, which nothing in Atlas's current schema provides.
- **A durability-like classification for Atlas's own input classes**, borrowing the SHAPE (not the
  exact four levels) of Salsa's `Durability` — the Salsa genome record's own "Known trade-offs" section
  already sketches a plausible Atlas-specific mapping (source file content = LOW; `Cargo.lock`/manifest
  = MEDIUM; donor pinned commit/admitted dependency version = HIGH; canonical contract/schema version =
  closer to NEVER_CHANGE within one Atlas revision) — this record does not improve on that sketch, only
  confirms it remains the best-evidenced starting point after checking it against Atlas's real
  `Evidence`/`Provenance` types, which have no analogous classification field today.
- **A two-stamp (verified-at / changed-at) split attached to derived records**, so a record re-verified
  without its value actually changing does not force its own dependents to re-run — new state, not
  present on `SemanticRecordHeader` today.
- **A recorded set of upstream `record_id`s (and, plausibly, a derivation-rule identity) per derived
  record** — the Salsa-`QueryOrigin`-shaped list this record's §(a) above identifies as potentially
  serving both invalidation and (Souffle-shaped) explanation, if designed carefully; not confirmed as a
  single unified structure, only as a plausible shared foundation.
- **Souffle-shaped auxiliary provenance metadata on DERIVED records specifically** (an Atlas analog of
  `@rule_num`/`@level_num`) — additive to the existing identity/header split per §(a), directly
  answering R5's own "evidence/provenance lineage through derived facts" requirement and pre-loading
  the ground R6's reconciliation requirements ("top-down claim decomposition," "bottom-up aggregation")
  will need for DERIVED, not just observed, facts.
- **Some retraction primitive** — at minimum a `RETRACTED` (or similarly named) addition to
  `EpistemicStatus`'s existing nine-value vocabulary, or, if Atlas's needs turn out to require more than
  a tombstone, a narrower-than-differential-dataflow's-full-generality negation mechanism scoped to
  "this specific derived record no longer holds," motivated directly by the concrete gap identified in
  §(b).3 above (today a disappeared fact is invisible-by-omission, not explicitly negated).

**What this design sketch would explicitly NOT need, and why, grounded in what was actually read:**

- **No distributed execution substrate** (differential-dataflow's `timely-dataflow`) — every genome
  record in this lane independently confirms Atlas's census pipeline is single-threaded today; this is
  not a new finding, but this record confirms it remains true by grep (`core/src`, `runtime/src`: no
  concurrency-primitive usage found in the normalize/graph layer read this pass).
- **No general unbounded fixed-point cycle-recovery machinery** (Salsa's `Fixpoint`/
  `CycleRecoveryStrategy`, `MAX_ITERATIONS`) as a DEFAULT — `salsa-fixpoint-cycle-iteration.md`'s own
  panic-by-default finding ("the correct, honest default Atlas should also adopt, not iterate-by-
  default") applies directly, and this record's own read of `DependencyClosureReport` finds Atlas's
  one existing closure-shaped computation is acyclic BY CONSTRUCTION (a lockfile's own DAG), with no
  evidence yet of a genuinely cyclic Atlas-native recursive derivation that would require this
  machinery. If R5 implementation work later identifies a real cyclic derivation, this machinery
  becomes relevant; it is not needed to START R5 design work.
- **No Dice-shaped branch tree** — per §(c), out of R5's own required-capabilities scope; relevant to
  AIF1, a separate roadmap item.
- **No full Abelian-group-typed diff algebra for arbitrary aggregates** — per §(b).3, Atlas's facts are
  discrete typed records, not numeric aggregates; the part of differential-dataflow's generalization
  actually motivated by evidence is retraction specifically, not the broader algebra.
- **No persistent, live, cross-run `Trace`-equivalent** (differential-dataflow's own genome record
  flags this as "a genuinely different resource commitment": keeping a computation's derivation graph
  ALIVE across many rounds, vs. Atlas's current model of computing `NormalizationReport`/
  `EngineeringGraph` fresh from a `CensusReport` every call) — introducing this is a substantial new
  state/persistence commitment this record does not recommend taking on before establishing that
  Atlas's actual edit-to-recensus cadence needs it rather than a cheaper "recompute the whole thing,
  but cheaply, thanks to the durability-gated shallow check" approach.

## Required invariants

Carried forward, unresolved, from the underlying mechanisms and now stated against Atlas's own
concepts specifically:

- Any Atlas-native durability classification must be conservative (never overstated) or the
  shallow-check optimization becomes unsound — exactly Salsa's own invariant, restated against
  Atlas's proposed source/manifest/donor-commit/contract classification above.
- Any Atlas-internal revision/generation counter must be strictly monotonic and globally consistent
  for a `<=`-style shallow comparison to mean anything — ruling out, as in Salsa, independent
  per-shard counters without a single serialization point, which matters only once/if Atlas's census
  pipeline stops being single-threaded.
- Any semi-naive-shaped staging (if datafrog's pattern is adopted for a specific closure computation)
  requires every contributing derivation rule to be monotone, and at least one de-duplicating stage
  per derivation cycle — both explicitly documented, unenforced invariants in the donor itself, and
  neither currently checkable against any Atlas derivation rule because none exist yet to check.
- Any provenance auxiliary metadata (rule/level-equivalent) must be assigned consistently and never
  reused across genuinely different derivation rules for the same record, and any level/height field
  must be a true strict-descent measure — exactly Souffle's own invariants, restated.

## Identity/scope model

No new identity model is proposed here. The synthesis in §(b).1 concludes Atlas's existing
`SemanticRecordId`/`SemanticRecordHeader` split is the best-fitting foundation of the five donors'
identity shapes; this record recommends extending it (new fields, possibly a parallel
derived-record-specific header) rather than replacing it with any donor's own identity type wholesale.

## State/effect/resource model

Atlas's current model (pure, in-memory, recomputed fresh per call, nothing persisted across
`normalize()`/`build_system_graph` invocations beyond the reports/graphs themselves) is closest to
datafrog's resource shape among the five (transient, discarded-after-use working state) and furthest
from differential-dataflow's (a `Trace` kept alive across many rounds) and Dice's (branch-local state
retained indefinitely). Any R5 design that introduces persistent cross-run incremental state is a
genuinely new resource commitment for Atlas, not a natural extension of what exists — flagged, not
resolved, above.

## Failure and recovery behavior

Atlas's own R4 discipline (never fabricate a result; report inability honestly) already matches the
loud-failure posture both Salsa (panic on unhandled cycle or non-convergence) and Dice (soundness
theorem: `Unknown` is always permitted, "making every form of forgetting trivially sound") independently
converge on. datafrog's complete absence of a termination safety net (no `MAX_ITERATIONS`-equivalent)
is flagged by its own genome record as a real gap; any Atlas-native semi-naive-shaped computation
should not silently inherit that absence.

## Concurrency/temporal behavior

Not applicable beyond what is stated in §(c) and the design sketch's "not needed" list — Atlas's
census pipeline remains single-threaded, confirmed again by this record's own grep pass.

## Performance characteristics

Not benchmarked or estimated in this record. Each of the five source genome records is itself explicit
that it did not benchmark its donor's mechanism; this synthesis inherits that limitation and adds no
new performance evidence of its own.

## Portability/ABI constraints

Not applicable; this record proposes no specific implementation, only a design-space characterization.

## Evidence references

- `.atlas/genome/technology/salsa-durability-gated-incremental-verification.md` (read in full)
- `.atlas/genome/technology/salsa-fixpoint-cycle-iteration.md` (read in full)
- `.atlas/genome/technology/datafrog-semi-naive-fixpoint-evaluation.md` (read in full)
- `.atlas/genome/technology/differential-dataflow-algebraic-retraction-and-lattice-time.md` (read in
  full)
- `.atlas/genome/technology/souffle-provenance-backed-derivation.md` (read in full)
- `.atlas/genome/technology/buck2-dice-branching-incrementality.md` (read in full)
- `.atlas/references/donor-corpus.toml` (`salsa`/`datafrog`/`differential-dataflow`/`souffle`/`buck2`
  entries: `census_status = "COARSE_CENSUSED"`, `decision_status = "PENDING"` for all five, confirmed
  directly)
- `.atlas/roadmap/SELF-BUILDING-R4-R8.md` (R5 section, lines 476-491, quoted verbatim for the six
  required capabilities; AIF1 section for the branching-capability mapping check in §(c))
- `.atlas/roadmap/DONOR-ABSORPTION-ROADMAP.md:311` (the W3 donor-lane row: "incremental query, fixed
  point, build/dependency reasoning" — confirming the lane groups by technology area, not by
  per-capability mapping)
- `.atlas/decisions/0001-one-normalized-semantic-path.md` and
  `.atlas/decisions/0002-epistemic-status-model.md` (read in full, for this repository's own
  design-document precedent/rigor and for `EpistemicStatus`'s canonical nine-value vocabulary, checked
  directly for the absence of a `RETRACTED`-equivalent state)
- `runtime/src/normalize/mod.rs` (read in full: confirms `normalize()` is a pure, batch,
  from-scratch function with no incremental/cached-input parameter)
- `core/src/graph/engineering_graph.rs` (read: `build_source_graph`, `build_repository_graph`,
  `build_system_graph`, `ensure_node` — confirms full-rebuild graph projection with no cross-call
  reuse)
- `core/src/semantic/mod.rs` (read in full: `SemanticRecordId`, `SemanticRecordHeader`,
  `SemanticDimension`, `ExtractorIdentity` — the identity/header split this record's §(b).1 compares
  against all five donors' own identity models)
- `core/src/evidence/mod.rs` (read in full: `Evidence { id, kind, path, summary, revision }`)
- `core/src/census/dependency.rs` (read: `DependencyClosureReport`, confirming it is a one-shot
  lockfile-edge resolution, not an iterative rule-based fixed point)
- `core/src/schema/mod.rs` (grepped for existing closure-checking test names, confirming a
  referential-completeness-on-a-snapshot discipline already exists, distinct from incremental
  re-verification across snapshots)
- Grep of `core/src` and `runtime/src` for `incremental|recompute|cache|memoi` (two incidental,
  unrelated hits only — confirms no incremental-computation infrastructure exists anywhere in the
  codebase today)

## Donor revisions/licenses

Unchanged from the six source genome records; this record pins no new donor commit and introduces no
new licensing consideration. See each source record's own "Donor revisions/licenses" section.

## Known trade-offs

- This synthesis is necessarily bounded by what the five source records themselves censused (each
  explicitly scoped to ONE mechanism per donor, with substantial remaining surface — cycle handling in
  Dice's own Appendix C, the RAM IR/interpreter/synthesiser in Souffle, the operator/arrangement layer
  in differential-dataflow, the macro/ingredient/parallel-execution system in Salsa, the
  treefrog/leaper join machinery in datafrog — explicitly uncensused in every case). A future,
  implementation-grade R5 design will very likely need to return to several of these donors for a
  DEEPER census of their operational machinery, not just their conceptual mechanism, before writing
  real Atlas code.
- This record's own read of Atlas's current source was itself scoped to two files plus targeted greps
  (`runtime/src/normalize/mod.rs` in full; `core/src/graph/engineering_graph.rs` partially — roughly
  the first 400 of 3792 lines, focused on `build_source_graph`/`build_repository_graph`/
  `build_system_graph`'s structure — plus `core/src/semantic/mod.rs`, `core/src/evidence/mod.rs`, and
  `core/src/census/dependency.rs`'s `DependencyClosureReport` type). `engineering_graph.rs`'s remaining
  ~3400 lines (per-dimension observation-to-node/edge projection logic for Call/DataFlow/State/
  Ownership/Concurrency/Persistence, and its own extensive test suite) were not read for this record
  and could contain additional load-bearing detail relevant to a future, more implementation-grounded
  R5 design pass.
- The identity-model-fit argument in §(b).1 is a genuine, evidenced finding, but it is a FIT argument
  (least structural rework), not a CAPABILITY argument (most powerful for R5's eventual full scope) —
  conflating the two would be a real mistake a future reader of this record should not make.

## Rejected alternatives (for this pass)

Not evaluated in this record: any concrete Rust type/trait design for the fields and structures
sketched in "Atlas-native design sketch" above; any prototype or benchmark of any of the five
mechanisms against Atlas's actual data; any decision about whether Salsa-shaped, datafrog-shaped, or
differential-dataflow-shaped evaluation is ultimately chosen (deliberately left open, see Open
questions); any design for how R5's incremental engine interacts with R6's reconciliation/
`CensusCertificate` machinery, which the roadmap names as consuming R5's own "fixed-point closure"
and "dependency closure included in closure proof" but which this record does not attempt to design
against.

## Open questions (deliberately unresolved)

Stated plainly, matching this repository's own epistemic-honesty discipline (ADR 0002; never claim
more than the evidence supports):

1. **Does Atlas's actual R5 workload need genuine incremental view maintenance (differential-
   dataflow's value proposition) at all**, or would a simpler Salsa-shaped shallow-invalidation gate
   plus datafrog-shaped from-scratch closure recompute suffice for Atlas's real edit-to-recensus
   cadence? Not measured; no workload characterization exists yet.
2. **Is there a genuinely cyclic Atlas-native recursive derivation** that would require Salsa's
   `Fixpoint`/`CycleRecoveryStrategy` machinery, or does everything Atlas currently computes
   recursively (transitive dependency closure, foreseeably others) remain acyclic by construction the
   way `DependencyClosureReport` currently is? Flagged as open by `salsa-fixpoint-cycle-iteration.md`
   itself and not resolved by this record's own read of `dependency.rs`.
3. **What granularity should an Atlas-internal revision/generation counter actually have** — per
   source-file edit, per whole census run, per something finer or coarser? Not decided; §(b).2 only
   establishes that `RevisionRef` (git-SHA-shaped) cannot itself serve this role unmodified.
4. **Does retraction need full Abelian-group generality, or does a single `RETRACTED`-style
   `EpistemicStatus` addition suffice for Atlas's actual needs?** §(b).3 leans toward the narrower
   option being sufficient for what has actually been evidenced (discrete typed records, not numeric
   aggregates) but does not resolve it.
5. **Can a single "recorded upstream `record_id`s" structure serve both Salsa-shaped invalidation and
   Souffle-shaped explanation**, or do these two consumers need materially different data (a stored
   list vs. a regenerable subroutine, per Souffle's own more robust-against-staleness approach)? Named
   as a real design opportunity in §(a), not resolved.
6. **How does an eventual R5 incremental engine interact with R6's reconciliation/`CensusCertificate`
   machinery** (explicit `CONFLICT` preservation, cross-scope reconciliation, top-down claim
   decomposition, bottom-up aggregation, "fixed-point closure" and "dependency closure included in
   closure proof" as R6's own required capabilities)? Entirely out of this record's scope; the two
   roadmap items are clearly coupled by the roadmap's own text but this record does not attempt to
   design across that boundary.
7. **Is Dice's branch-tree pattern actually the right shape for AIF1**, once AIF1 work begins, or does
   the buck2 genome record's own confidence in that mapping (based on a partial read of one design
   document and one core-state crate, explicitly not covering Dice's execution/scheduling layer or
   Appendix C's cycle handling) hold up under deeper census? Not re-examined by this record; carried
   forward as-is from the source record.
8. **What does `engineering_graph.rs`'s remaining, unread ~3400 lines** (the per-dimension
   observation-to-node/edge projection logic this record did not read) imply for the design sketch
   above? Not established; a real gap in this record's own grounding, named explicitly per this
   repository's own discipline of naming what was not checked rather than implying full coverage.

## Dependency/extinction status

Not applicable to a synthesis record; see each of the five underlying donor entries in
`.atlas/references/donor-corpus.toml` (all `PENDING`, unchanged by this record).

## Decision

**No absorption decision is made or implied by this record.** What this record concretely establishes,
durably, so it is not rediscovered later:

1. The five W3-lane mechanism records, read together against Atlas's own real N0 normalization/graph
   layer (not merely against each donor's own source), yield two findings visible only from the
   comparison, neither stated by any single genome record alone: (i) Atlas's existing
   `SemanticRecordId`/`SemanticRecordHeader` identity split structurally fits Salsa's evaluation shape
   with materially less new supporting machinery than datafrog's or differential-dataflow's shapes
   would each require (a sortable surrogate key, or a lattice-typed time domain, respectively — neither
   of which exists in Atlas today); and (ii) Atlas's actual `RevisionRef` type (an external, whole-
   repository git-SHA coordinate) cannot itself serve as any of the three donors' revision/round/time
   coordinate unmodified — an Atlas-internal, finer-grained counter is a real, previously-unnamed
   prerequisite for adopting any of them.
2. Souffle's provenance pattern is confirmed, against Atlas's real header type specifically, to be
   ADDITIVE (new fields, not a redesign) to Atlas's existing identity/evidence split — directly
   actionable groundwork for R5's "evidence/provenance lineage through derived facts" requirement and
   for R6's later reconciliation needs, independent of which evaluation strategy R5 ultimately adopts.
3. Buck2/Dice's branching mechanism is confirmed, by direct check against the roadmap's own R5
   required-capabilities text, to be OUT OF R5's SCOPE and IN AIF1's scope — the W3 donor lane's shared
   grouping of all five donors is organized by technology area, not by per-roadmap-capability mapping,
   and this record found no basis for treating Dice as competing with or required by R5's own baseline
   design.
4. R5 is confirmed, by direct grep and read of `core/src` and `runtime/src`, to be a genuine
   greenfield gap — Atlas has a batch-recompute-everything model today with zero existing incremental-
   computation infrastructure, not a partial one R5 extends.
5. This record explicitly does NOT propose a final, ready-to-implement R5 design. It narrows the
   design space (identity-model fit; the revision-coordinate gap; retraction's narrow real motivation;
   Dice's out-of-scope status) and names eight concrete open questions (above) that implementation-
   design work should resolve, several with a clear next step (e.g. characterizing Atlas's actual
   edit-to-recensus workload cadence before choosing between differential-dataflow-shaped and
   two-mechanism-composed approaches).

`census_status`/`decision_status` for all five underlying W3-lane donors remain unchanged
(`COARSE_CENSUSED`/`PENDING`) in `.atlas/references/donor-corpus.toml`. This record introduces no new
`type` value and no new donor-corpus entry; it is a synthesis record within the existing
`technology-genome` type, cross-referencing all five without altering any of their own recorded state.
