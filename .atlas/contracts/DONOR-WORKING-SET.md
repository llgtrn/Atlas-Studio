---
id: atlas.contract.donor-working-set
type: contract
status: active
canonical: true
---
# Donor Working Set

Donor source is temporary working material. Durable Atlas-native code, technology genome, decisions, provenance and verification evidence are the retained result. Atlas may know about far more repositories than it can hold locally; the working set stays bounded by measured disk.

~~~text
DISCOVER MANY   (remote: URL, head, tree, license, size -- RECOMMENDED-OSS-FRONTIER.toml)
   ↓
QUEUE MANY      (capability-driven: DONOR-WORKING-SET.toml [[queue]])
   ↓
MATERIALIZE FEW (cheapest sufficient mode, admitted only inside the budget)
   ↓
UNDERSTAND DEEPLY → ABSORB NATIVELY → VERIFY
   ↓
DELETE SOURCE → MEASURE RECLAIMED → RECOMPUTE BUDGET
   ↓
NEXT DONOR
~~~

This contract supersedes the clone-the-approved-batch-up-front rule formerly in `../blueprints/BULK-DONOR-ABSORPTION.md`.

## Discovery is not materialization

- Remote Git access (`ls-remote`, blobless/treeless metadata, single files) is enough for discovery, revision/tree identity, license, rough sizing, locating candidate mechanisms and pre-admission evidence. That state is `REMOTE_DISCOVERED`.
- It is **not** a materialized donor. Execution-backed census — compilation, execution, generated files, mutation, profiling, whole-repository resolution, local dependency closure — requires local materialization.

## Source-backed

`SOURCE_BACKED` means a claim is traceable to an exact source revision, path and blob, plus evidence. It does not mean the source permanently exists in Atlas's workspace. Provenance must answer, after physical deletion:
- which repository;
- which revision;
- which path or blob;
- what license;
- which mechanism;
- what evidence;
- why it was absorbed;
- how it was verified.

## Storage state (orthogonal to decision state)

Each donor-corpus record carries `storage_state` and `materialization_mode`, independent of `decision_status`. For example, `ABSORB_NOW` with `MATERIALIZED_SLICE` is ordinary, and so is `REFERENCE_ONLY` with `SOURCE_DELETED`.

| state | local source |
|---|---|
| REMOTE_DISCOVERED, QUEUED | must be absent |
| MATERIALIZING | may be partial |
| MATERIALIZED_SLICE, MATERIALIZED_CHECKOUT, CENSUSING, ABSORBING, VERIFIED_NATIVE, SOURCE_DELETABLE | must be present |
| SOURCE_DELETED | must be absent (it was materialized and physically deleted) |
| SOURCE_NEVER_MATERIALIZED | must be absent (remote census only; no deletion happened or is owed) |

- Atlas never clones a donor only so that it can delete it and call that extinction. A donor that was never materialized records `SOURCE_NEVER_MATERIALIZED`.
- Materialization modes, cheapest first: `REMOTE_METADATA` → `FETCHED_FILE_SET` → `SPARSE_CHECKOUT` → `SHALLOW_CHECKOUT` → `FULL_CHECKOUT`.
  - `cheapest_sufficient_mode` picks the cheapest mode that can produce the required evidence.
  - A full checkout is justified only by history (blame, bisect, cross-revision provenance). That reason is recorded in the ledger.
  - Legacy `FULL_SOURCE_TREE` staging (one pinned revision, no history) is `SHALLOW_CHECKOUT`. Legacy `PARTIAL_SPARSE_SUBTREE` staging is `SPARSE_CHECKOUT`.

## Where source lives

- **New materializations** go under `.atlas/.cache/donors/<id>/`. This is untracked, git-ignored scratch. Giant donor checkouts are never committed just to preserve provenance.
- **Legacy tracked trees** (`.atlas/temporary/donors/`, committed before this contract) are drained by extinction and never extended.
- Build outputs, generated files, profiler artifacts and benchmark binaries stay in scratch.

## Budget and watermarks

- **Free fraction** = available / (used + available). This is the `df` Use% basis, so reserved blocks and per-session allowances do not inflate capacity.
- **Watermarks** are configurable permille values in `../roadmap/DONOR-WORKING-SET.toml`. The defaults are 400 / 200 / 100:
  - **HEALTHY:** new materialization is allowed.
  - **PRESSURE:** finish in-progress donors (MATERIALIZING / CENSUSING / ABSORBING), then delete `SOURCE_DELETABLE` source, before expanding.
  - **HIGH_PRESSURE:** no new large donor (footprint at or above `large_donor_bytes`); prioritize extinction and cleanup.
  - **CRITICAL:** materialization hard stop.
- **Donor budget** = available − build headroom − safety reserve. Headroom and reserve are ratios with absolute floors. A request's footprint is its source plus its expected build output (build amplification) and must fit the budget.
- **Working-set cap:** all local donor source together may not exceed `max_working_set_permille` of capacity. Deleted source does not count.
- **Remote-only work** (`REMOTE_METADATA`) occupies no disk and is never refused on storage grounds.
- **Disk capacity is never a reason to admit a donor.** Admission stays capability-driven: a concrete Atlas gap, a named mechanism, and a consumer.

## Dependencies: accounted is not cloned

`TRANSITIVE_CLOSURE_REQUIRED` still holds, so every dependency node gets a terminal classification. Only `SOURCE_REQUIRED` (its source implements part of the mechanism being absorbed) is ever materialized. These terminals are accounted but not cloned:
- `SYSTEM_BOUNDARY`
- `TOOLCHAIN`
- `SERVICE`
- `EXTERNAL_PROVIDER`
- `ALREADY_NATIVE`
- `REFERENCE_ONLY`

Large repositories (llvm-project, browser engines, ROS 2 families, FreeCAD, OCCT, KiCad) expand progressively: remote tree census, then a sparse slice, then only the further slices the census proves necessary.

## Extinction and reclaim

The extinction contract is unchanged: physical deletion, then a post-deletion verify, then `EXTINCT`, with scope-level extinction for mechanism slices. After deletion:
1. record `SOURCE_DELETED`;
2. measure the reclaimed bytes;
3. recompute the budget;
4. consider the next queued donor.

Storage pressure is an execution signal, not a loop failure. Under pressure, stop new admission, delete verified-deletable source and safe build artifacts, remeasure, and resume. Required evidence is never deleted to recover space.

## Integrity (enforced)

`runtime::donor_storage` together with `atlas_core::donor::check_integrity` fails on:
- a record whose state requires absence while its source exists (EXTINCT or never-materialized source that reappeared);
- a record whose state requires presence while no source exists (deleted but still recorded as materialized);
- a local donor directory that no source-holding record owns (unadmitted, orphan or stale checkout).

Known violations whose remedy is blocked are recorded in `DONOR-WORKING-SET.toml [[unadmitted_checkout]]` with that remedy. That list can only shrink truthfully: a listed entry that no longer violates also fails.

## Independence boundaries

Deleting donor source does not remove external platforms. These are distinguished from donor source dependency:
- OS
- ISA
- browser engine
- toolchain
- hardware
- service
- provider

Independence is claimed only at the boundary actually crossed.
