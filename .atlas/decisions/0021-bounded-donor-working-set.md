---
id: atlas.decision.0021.bounded-donor-working-set
type: decision
status: accepted
canonical: true
---
# ADR 0021 — Bounded donor working set

## Context

Donor ingestion had drifted between two extremes, neither of them intended:
- **Clone first.** `BULK-DONOR-ABSORPTION.md` said to "clone the approved batch up front", and its compiler-lane batch did exactly that.
- **Remote only.** Other lanes treated remote Git reads as sufficient for everything.

Measured state at the base commit `fd39dcd3d4`:
- **Tracked donor source:** 436k tracked files under `.atlas/temporary/donors/`, totalling 4.82 GB of apparent size across 54 donors that hold source.
- **Unowned checkouts:** 7 checkouts owned by no record, 529 MB in total.
- **Disk:** 24.3% free, in the PRESSURE band.
- **Stale records:** three extinct donors whose provenance records still read `CLONED`.

## Decision

1. **`atlas_core::donor`** (pure, integer-exact) provides:
   - `StorageState`: 11 states, orthogonal to decision state, each with a presence rule.
   - `MaterializationMode` and `cheapest_sufficient_mode`.
   - `DiskMeasurement`, `StoragePolicy` (watermarks, and ratio-with-floor reserves), `Pressure`.
   - `decide_admission`: working set, budget, cap, and per-pressure gates.
   - `DependencyTerminal`: only `SOURCE_REQUIRED` is materialized.
   - `check_integrity`.
2. **`runtime::donor_storage`** provides:
   - `measure_disk`, which uses POSIX `df -P -k` with capacity = used + available;
   - `directory_bytes`;
   - `load_donor_storage`, which reads `storage_state` from `donor-corpus.toml`;
   - `DONOR-WORKING-SET.toml`, which holds the policy and the recorded blockers;
   - `working_set_report`.
3. **CLI.** `atlas-systemizer donors working-set [--request ID --mode M --source-bytes N --build-bytes N]` exits with:
   - `DONOR_STORAGE_INTEGRITY_VIOLATION` on any unrecorded violation;
   - `DONOR_MATERIALIZATION_REFUSED` when the gate refuses.
4. **Records.**
   - Every donor-corpus record gains `storage_state` and `materialization_mode`: 53 `MATERIALIZED_CHECKOUT`, mlir `MATERIALIZED_SLICE`, and 3 `SOURCE_DELETED`.
   - The stale provenance of blake3, datafrog and souffle is corrected.
   - New materialization goes to git-ignored scratch (`.atlas/.cache/donors/`).
   - The clone-first rule is superseded in every canonical document (`contracts/DONOR-WORKING-SET.md`).
5. **The 7 unadmitted checkouts** are classified in `DONOR-WORKING-SET.toml`: six are tracked checkouts without provenance, and one (xz) is a dangling gitlink. Their deletion stays `BLOCKED_ON_USER`, because the permission boundary that denied it is preserved. They are now enforced integrity violations with a recorded remedy rather than a hard-coded allow-list.

## Evidence

- **Real measurement.**
  - Capacity 39.78 GB, 9.69 GB available: 243‰ free, so PRESSURE.
  - Donor budget 5.28 GB; working-set cap 9.95 GB.
  - Working set 4.82 GB across 54 donors; unowned 529 MB.
  - A 100 MB sparse request is admitted, because no donor is in progress.
- **Mutation testing: 17 of 17 scheduler mutants killed.**
  - Critical disk admitting.
  - Size ignored under high pressure.
  - In-progress donors ignored.
  - Deletable source ignored.
  - Build amplification dropped.
  - Cap ignored.
  - Deleted source counted in the working set.
  - Safety reserve dropped.
  - Headroom floor dropped.
  - Remote counted as local.
  - A watermark off by one.
  - Never-materialized source tolerated.
  - Unowned directories ignored.
  - History served by a shallow checkout.
  - Toolchain dependencies cloned.
  - Policy left unvalidated.
  - Re-admission allowed.
- **Real-tree falsification.** Each of these fails the repository integrity test:
  - an orphan directory in scratch;
  - souffle re-marked `MATERIALIZED_CHECKOUT`;
  - a recorded blocker dropped from the list.

## Limits

- Sizes are apparent bytes, not allocated blocks.
- Build amplification is an estimate supplied with the request. It is measured once a donor has been materialized.
- The legacy tracked trees stay in git history after deletion; the extinction contract concerns the working tree.
