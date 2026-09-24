---
id: donor-census-datafrog
type: reference
status: active
canonical: true
---
# Donor Census: Datafrog

## Source

- Remote: https://github.com/rust-lang/datafrog.git
- Commit: edd7cfc0f5ad3b93ce5bbe6a17848db9376b13c0
- Branch: master
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: COARSE_CENSUSED (advanced from SKELETON). The core semi-naive bottom-up Datalog evaluation
strategy (three-stage tuple lifecycle: to_add -> recent -> stable; geometric/doubling batch merging;
adaptive galloping-vs-linear deduplication) is now understood and evidenced from real source -- see
`.atlas/genome/technology/datafrog-semi-naive-fixpoint-evaluation.md`, read directly from `src/lib.rs`,
`src/iteration.rs`, and `src/variable.rs` (read in full).

Still needing classification: the treefrog/leaper "worst-case optimal join" machinery
(`src/treefrog.rs`, the crate's single largest file at 795 lines), and the `join.rs`/`merge.rs`/
`map.rs` operator implementations.

## Native Replacement

runtime inference engine -- explicitly complementary to, not redundant with, Salsa's own mechanism
(see the genome record above): Salsa answers "what does one changed input invalidate", datafrog
answers "what is the full closure of a fixed fact set". R5's own requirements plausibly need both,
composed. `differential-dataflow` (the third W3 donor) is flagged as the natural next census target,
since it exists specifically to combine the two.

