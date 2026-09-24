---
id: donor-census-buck2
type: reference
status: active
canonical: true
---
# Donor Census: Buck2

## Source

- Remote: https://github.com/facebook/buck2.git
- Commit: 78ec517305d8214d6160f6c99b1d0a3554879878
- Branch: main
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: COARSE_CENSUSED (advanced from SKELETON). One essential mechanism -- Dice's branching,
cheaply-forkable incrementality -- is now understood and evidenced; see
`.atlas/genome/technology/buck2-dice-branching-incrementality.md`:

- state is a TREE of branches (`dice/dice_core/src/branch.rs`), not a single revision line; forking
  is cheap because resolution lazily delegates to the parent branch at the fork point (no upfront
  state copy), at the cost of a nontrivial fork-time reconciliation of the new branch's own
  reverse-dependency map (a topological attached/residual partition, per the donor's own
  `dice/docs/incrementality.md` §5.4);
- the donor's own design document states and proves an explicit soundness theorem (§3.3):
  `lookup` returning `Unknown` is always sound by design -- the same "never fabricate, Unknown is
  always honest" discipline Atlas's own `EpistemicStatus` model already applies, confirmed here as a
  proven property of a production incremental-computation engine;
- this is the first W3-lane donor modeling MULTIPLE CONCURRENT LINEAGES of computation state as a
  first-class object -- directly relevant to `MULTI-AI-CONSTRUCTION-FABRIC.md`'s own "concurrent
  speculative CandidateAtlas branches" language.

Still needing classification: Dice's own execution/scheduling layer (`dice/dice/`, async task
running and in-flight-computation deduplication), the value/interning/garbage-collection layer, the
series-parallel dependency structure, the cycle-handling appendix, and buck2 as a build system beyond
Dice specifically (the actual build-graph/target-identity/dependency-scheduling surface this donor
was originally recorded for).

## Native Replacement

adapter/build and runtime build graph -- Dice's branching-incrementality mechanism is also a strong
candidate for a future Atlas-native concurrent-speculative-branch capability (AIF1), a materially
different use than the original build-graph framing this donor was recorded under. With all five
named W3-lane donors (Salsa, datafrog, differential-dataflow, souffle, buck2/Dice) now
coarse-censused with at least one real mechanism record each, an actual R5 design synthesis is a
well-evidenced, plausible next step for this lane.

