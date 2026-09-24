---
id: atlas.genome.technology.buck2-dice-branching-incrementality
type: technology-genome
status: active
canonical: true
---
# Technology Genome: branching, cheaply-forkable incremental computation (Buck2's Dice)

Donor: Buck2 (`facebook/buck2`, commit `78ec517305d8214d6160f6c99b1d0a3554879878`), the fifth W3
donor censused this session and the last one named in `donor-corpus.toml`'s W3 lane (souffle,
censused the prior generation, was the fourth). This record scopes specifically to Dice's core-state
crate (`dice/dice_core/`) and its own primary design document
(`dice/docs/incrementality.md`, read substantially -- Introduction, §3.3 Soundness, §4.2-4.4
Branches/Resolution, and §5.4 fork -- 808 lines total, not read to full completion). It does not
attempt to census Dice's execution/scheduling layer (`dice/dice/`, the async task-running
environment built on top of this core), the value/interning layer (Appendix A), the series-parallel
dependency structure (Appendix B), or the cycle-handling appendix (Appendix C) -- see Decision for
the explicit remaining scope.

## Capability / problem

Every W3 donor censused so far treats "the current state of the computation" as a SINGLE lineage:
Salsa's `Revision` is one global monotonic counter; datafrog's round counter is one linear sequence;
differential-dataflow generalizes time to a lattice but still models ONE evolving collection;
souffle's provenance mechanism explains a single derivation, not competing derivations. None of them
model MULTIPLE, CONCURRENTLY-EVOLVING, POSSIBLY-DIVERGING lineages of the same computation state as a
first-class object. Dice's own stated driving design goal (`incrementality.md` §1, read directly)
is exactly this: "correct answers while doing as little recompute as possible -- under concurrent
access to multiple historical states, and, THE DRIVING CASE FOR THIS DESIGN, under multiple
concurrently evolving lineages of state." This maps directly onto Atlas's own AIF1 (Multi-AI
Construction Fabric) ambition, already named in this repository's own roadmap: "concurrent
speculative CandidateAtlas branches with separate CandidateChangeSet/evidence lineage" -- Dice is a
real, production-scale (it powers Meta's own build system), rigorously specified (with a stated and
proven soundness theorem) working answer to exactly that requirement.

## Semantic mechanism (as observed in the donor)

- **State is a TREE of branches, not a single line of revisions** (`dice_core/src/branch.rs`,
  `incrementality.md` §4.2, both read directly, confirmed matching): `Branch { parent: Option<Version>,
  children: [(BranchId, Seq)], first/head: Seq, rdeps: RdepMap, closed_index: KeySet }`. A branch
  forks from a specific `(parent_branch, seq)` point; any number of branches can coexist and be
  resolved against independently and concurrently.
- **Forking is CHEAP (no upfront state copy) because resolution INHERITS from the parent by default**
  (§4.4, the document's own words: "The inheritance mechanism is in many ways the crux of this
  algorithm; it allows branches to be relatively cheap while making common cases in which reuse is
  allowed easy to express."): `resolve(K, (branch, seq))` checks the branch's OWN claim for K first;
  if the branch has no claim for K at all, resolution transparently DELEGATES to
  `resolve(K, parent(branch))`, evaluated pinned at the exact fork-point version -- the branch's own
  world is defined as "its parent's fork-point state plus whatever this branch has itself since
  diverged." A newly-forked branch with zero claims of its own is therefore immediately, correctly
  resolvable for every key its parent already had, at zero marginal storage cost per key.
- **The price of cheap inheritance is a nontrivial, invariant-preserving fork-time reconciliation of
  the new branch's own reverse-dependency map** (§5.4, read directly, including its own stated Lemma
  and a topological-sort algorithm sketch): the new branch's `rdeps` map must be COMPLETE and
  SUPPORTED (closed downward: every key resolving as "attached" at the new branch must have its own
  dependency edges present too) from the moment the branch exists, so a later invalidation at the
  new branch propagates correctly -- but this cannot be achieved either by copying the parent's
  entire rdep map (too expensive, defeats the point of cheap forking) or by leaving it empty (would
  silently make future invalidations incomplete, a soundness violation). Dice resolves this by
  partitioning candidate keys (drawn only from the parent's own rdep map and `closed_index`, never a
  full scan) into an "attached" set (topologically closed under their own dependencies, inductively
  acyclic) and a "residual" set (given an explicit empty, invalid claim so they cannot silently
  resolve attached with missing edges).
- **Lookup soundness is one-directional and unconditional by design** (§3.3, the theorem quoted
  directly: "if `lookup(k, v)` returns `Valid(r)`, then [it is justified]... `Unknown` is always
  permitted, making every form of forgetting trivially sound"). This is the same discipline Atlas's
  own `EpistemicStatus`/evidence model already enforces for its own facts (never fabricate Observed;
  Unknown is always an acceptable, honest answer) -- confirmed here as a REAL, PROVEN property of a
  production incremental-computation engine, not merely Atlas's own convention.
- **Injected (asserted) keys are stored and resolved completely differently from computed
  (certified) keys** (§4.3, read directly): an injected key's full history (`Seq -> Revision`) is
  kept in full per branch, because -- unlike a computed key, which can always be recomputed from
  scratch if forgotten -- an asserted value that is forgotten is gone; the document states this
  distinction is relied on "for soundness of the algorithm itself." Directly analogous to Atlas's own
  Symbol/Type-observed-from-source vs. derived/normalized-fact distinction, though Dice's version is
  specifically about which facts the SYSTEM ITSELF, not an external source, must never silently drop.

## Required invariants

- The rdep map's own "support" invariant (every attached key's dependency edges are present) must be
  exactly maintained by every state-mutating operation (write, fork, commit) -- the document states
  this explicitly as Invariant 3 and treats fork-time reconstruction of it as the hardest part of the
  whole algorithm, not an afterthought.
- The attached-set computation at fork time must be INDUCTIVE (acyclic) -- the document states this
  is required "for the reasons given above and in the Appendix" (cycles), directly connecting the
  branching mechanism's own correctness to the cycle-handling appendix this record does not evaluate.
- An installation (a claim-changing operation) at one branch can trigger "cascade obligations" at
  descendant branches to preserve their own correctness -- the exact mechanism for this was not read
  in this record (out of scope; flagged explicitly as a real, load-bearing part of the algorithm this
  record does not fully cover).

## Identity/scope model

A value's full identity is `(Key, Branch, Seq)`, generalizing Salsa's `(query, Revision)` and
datafrog's `(Tuple, round)` with an explicit branch dimension -- the same key can be simultaneously,
independently valid at different revisions across different, concurrently-live branches, something
none of the other four W3 donors censused so far model at all.

## State/effect/resource model

Each branch owns its own `rdeps`/`closed_index` (real, branch-local state, not shared or copied from
the parent) but resolves against its ancestor chain lazily for everything it hasn't itself diverged
on -- a genuinely different resource-commitment shape from every other donor censused in this lane:
Salsa/datafrog/differential-dataflow each model exactly one live computation state at a time (even
differential-dataflow's own incremental updates apply to one evolving collection, not a branching
family of them).

## Failure and recovery behavior

Not evaluated in this record (out of scope -- Appendix C, cycles, and the execution/scheduling layer
built on top of this core state, are both real, separate topics this record explicitly does not
cover).

## Concurrency/temporal behavior

The core state crate's own module doc comment states its central concern is "sound reuse of computed
values across versions, branches of versions, and CONCURRENT ACCESS to any of them" -- concurrency
and branching are treated as one unified concern in this design, not two separate features bolted
together. The execution/scheduling layer that actually runs computations concurrently
(`dice/dice/`, async, built on top of this core) was not read in this record.

## Performance characteristics

Not benchmarked in this record. The document's own stated complexity bound for the core read
operation is explicit and load-bearing: `resolve` is O(chain length) (branch-ancestor-chain depth),
touches exactly one node per level, and "enumerates no branches" -- a real, specific, checkable
performance claim from the donor's own design document, not merely asserted informally.

## Portability/ABI constraints

Pure Rust (`dice_core` has no unsafe/FFI surface observed in what was read); the mechanism itself
(a tree of branches with lazy parent-delegated resolution, fork-time reconciliation of local
reverse-dependency state) is a data-structure-and-algorithm design, not tied to any Rust-specific
capability -- directly portable as a design pattern.

## Evidence references

- `.atlas/temporary/donors/buck2/dice/docs/incrementality.md` (Introduction/§1, Soundness/§3.3-3.4,
  Branches/§4.2, Injected keys/§4.3, Resolution/§4.4, and fork/§5.4 read directly; the remaining
  sections -- §2 interface, §5.1-5.3 the other operations, §6 completeness, and Appendices A-C --
  were not read in this pass)
- `.atlas/temporary/donors/buck2/dice/dice_core/src/lib.rs` (module doc comment, read in full: "sound
  reuse of computed values across versions, branches of versions, and concurrent access to any of
  them")
- `.atlas/temporary/donors/buck2/dice/dice_core/src/branch.rs` (the real `Branch` struct and its
  `root`/`forked`/`add_edge`/`take_rdeps` methods, read directly, confirming the design document's
  own §4.2 data-structure description matches the real implementation)
- `.atlas/temporary/donors/buck2/dice/dice/src/lib.rs` (top-level crate doc comment and worked
  `InjectedKey`/`Key` example, confirming the injected-vs-computed distinction is a real, public API
  surface, not only an internal core-state concept)
- `.atlas/roadmap/SELF-BUILDING-R4-R8.md` R5 section and the `MULTI-AI-CONSTRUCTION-FABRIC.md`
  contract's own "concurrent speculative CandidateAtlas branches with separate CandidateChangeSet/
  evidence lineage" language -- the specific Atlas-native concept this donor's own mechanism is most
  directly relevant to
- `.atlas/genome/technology/salsa-durability-gated-incremental-verification.md`,
  `salsa-fixpoint-cycle-iteration.md`, `datafrog-semi-naive-fixpoint-evaluation.md`,
  `differential-dataflow-algebraic-retraction-and-lattice-time.md`, and
  `souffle-provenance-backed-derivation.md` -- the four companion W3-lane records this one extends
  the body of evidence alongside

## Donor revisions/licenses

facebook/buck2, commit `78ec517305d8214d6160f6c99b1d0a3554879878`. Not independently re-verified in
this record (license files not read this pass; out of scope for a mechanism-only census).

## Known trade-offs

- Fork-time rdep-map reconciliation is real, nontrivial algorithmic work (a topological partition
  over candidate keys, not O(1)) -- forking is cheap relative to copying the WHOLE state, but is not
  literally free; the document is explicit that this is "in many ways the crux of this algorithm,"
  not an incidental detail.
- The core-state crate deliberately knows nothing about values, execution, or scheduling (confirmed
  by its own module doc comment) -- absorbing this mechanism means absorbing a STATE-TRACKING
  algorithm only; an Atlas-native adoption would still need its own execution/scheduling layer on
  top, exactly as Dice itself has a separate `dice/dice/` crate for that.

## Rejected alternatives (for this pass)

Not evaluated in this record: the execution/scheduling layer (`dice/dice/`, async task running,
in-flight-computation deduplication), the value/interning/garbage-collection layer (Appendix A), the
series-parallel dependency structure (Appendix B), and the cycle-handling appendix (Appendix C) --
each a substantial, separate census target, and Appendix C in particular is directly relevant to a
future comparison against Salsa's own `CycleRecoveryStrategy` already censused in this lane.

## Dependency/extinction status

`PENDING` per `donor-corpus.toml`, unchanged by this record.

## Decision

**No absorption decision yet.** This record establishes: (1) Dice's branching/forking mechanism --
a tree of branches with lazy, parent-delegated resolution keeping forks cheap, and a nontrivial but
well-specified fork-time reconciliation algorithm keeping each branch's own reverse-dependency state
sound -- is now genuinely understood and evidenced from the donor's own rigorously-specified design
document (which states and proves an explicit soundness theorem, an unusually strong evidence
standard among the donors censused so far in this lane); (2) this is the first W3-lane mechanism
directly addressing MULTIPLE CONCURRENT LINEAGES of computation state as a first-class object, the
exact shape `MULTI-AI-CONSTRUCTION-FABRIC.md`'s own "concurrent speculative CandidateAtlas branches"
language already names as a target Atlas-native capability -- making this donor's mechanism plausibly
the single most directly relevant piece of real, working prior art for that specific future
capability, among everything censused in the W3 lane so far; (3) with all five W3-lane donors named
in `donor-corpus.toml` now having at least one real, evidenced mechanism record (Salsa's
durability-gated verification and fixpoint-cycle-iteration, datafrog's semi-naive evaluation,
differential-dataflow's algebraic-retraction-and-lattice-time, souffle's provenance-backed
derivation, and now Dice's branching incrementality), the W3 donor lane's own named roster is
complete at the coarse-census level -- an actual R5 design-synthesis document (unifying these five
real mechanisms into one Atlas-native design) is now a well-evidenced, plausible next step, though a
materially different, larger kind of work than any single further donor deep-dive, and named
explicitly here as a decision point for a future generation rather than defaulted into.

`census_status` for buck2 in `donor-corpus.toml` is advanced from `SKELETON` to `COARSE_CENSUSED`
(the branching-incrementality mechanism is evidenced; the execution/scheduling layer, value/
interning layer, series-parallel dep structure, and cycle-handling appendix all remain explicitly,
substantially uncensused -- and buck2 as a whole build system, beyond Dice specifically, remains
essentially entirely uncensused). `decision_status` remains `PENDING`.
