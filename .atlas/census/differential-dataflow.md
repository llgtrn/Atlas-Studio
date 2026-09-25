---
id: donor-census-differential-dataflow
type: reference
status: active
canonical: true
---
# Donor Census: Differential Dataflow

## Source

- Remote: https://github.com/TimelyDataflow/differential-dataflow.git
- Commit: aa8745f93ea8abe131104fc7885ba4fd47e63902
- Branch: master
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: COARSE_CENSUSED (advanced from SKELETON). Two essential conceptual generalizations are now
understood and evidenced -- see
`.atlas/genome/technology/differential-dataflow-algebraic-retraction-and-lattice-time.md`:

- diffs as elements of an Abelian group (Semigroup/Monoid/Abelian, `src/difference.rs`), not
  hardcoded signed counts -- enabling true retraction, not just monotone insertion (the gap
  identified against datafrog's own insertion-only model);
- time as a lattice (`src/trace/mod.rs`), not a scalar counter -- unifying Salsa's revision-scalar
  and datafrog's round-scalar into one representation general enough for both nested fixed-point
  iteration and streaming incremental updates together.

This donor is far larger than Salsa or datafrog (207 Rust files, built on the separate
`timely-dataflow` framework). Still needing classification, substantially more so than the other two
W3 donors given the size difference: the operator library (`src/operators/`), the
arrangement/index system, `timely-dataflow` itself, distributed execution, and fault tolerance.

## Native Replacement

runtime incremental graph maintenance -- with all three of Salsa/datafrog/differential-dataflow now
having at least one real mechanism record, an actual R5 design synthesis (not a further donor
deep-dive) is now plausibly the highest-value next step in this thread.


## G80 — terminal REFERENCE_ONLY; source extinct

The recorded question asked whether Atlas's edit-to-recensus workload needs incremental view maintenance. The self-recensus proof chain now answers it. Across 23 proven generations (G57–G79):
- **Edits are small:** 1–19 of about 117 artifacts per generation (median 3).
- **Full recompute is cheap:** about 7.8 s at self scale, of which extraction is about 3.8 s and is memoizable per artifact (ADR 0008). The rest is linear passes plus a 0.4 s name-resolution fixpoint.
- **Large invalidations are program changes:** they happen only when Atlas's own analyzer changes, and differential dataflow cannot incrementalize a program change either.
- **Corpus scale was report-limited** (G41).

Collections, arrangements, traces and lattice time stay REFERENCE_ONLY. The P4 input is that self-recensus does not use the extraction cache. The checkout (360 files) was physically deleted. Evidence: `../evidence/campaign/17-differential-dataflow.json`.
