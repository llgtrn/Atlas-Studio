---
id: atlas.decision.0024.self-recensus-proof-of-nextgen
type: decision
status: accepted
canonical: true
---
# ADR 0024 — Priority reset: self-recensus is the proof of NEXTGEN

## Context

Generations G43–G56 added real capabilities: Creator, Physical, browser instruments, the donor working set and Product Foundry. But the roadmap diffused, and no generation proved, through Atlas's own census, that the new Atlas understood itself better than the old one. Generation numbering had become the only evidence of "next".

## Decision

1. **Priority reset.** `.atlas/roadmap/PRIORITY.toml` makes the order canonical:
   - P0 census kernel closure
   - P1 self-recensus N→N+1
   - P2 ADL→.atlas→.atlasx
   - P3 first-50 donor campaign
   - P4 incremental recensus
   - P5 absorption/extinction/recycling
   - P6 construction substrate

   Creator, Physical, Product Foundry, Futures, Ladybird, media, robotics and commerce are frozen secondary workloads, usable only as census evidence. The file also defines the selection rule (all answers false means DEFER), the census-closure elements, the Atlas self-scope baseline against them, and the exit criteria.
2. **Self-recensus.**
   - `atlas_core::recensus` provides `CensusSnapshot`, a revision-independent projection with a verifiable digest, and `prove`, which applies intent against observation and checks forbidden regressions and replay.
   - `runtime::recensus` provides the entry points.
   - `atlas-systemizer recensus snapshot|prove` exits `GENERATION_NOT_PROVEN`.
   - Contract: `contracts/SELF-RECENSUS.md`.
3. **Mandatory and chained.** From G57, every generation records `priority`, `serves` and `self_recensus`. A ledger test requires a PROVEN report with matching snapshot digests, and each pre digest must equal the previous post digest.

## Evidence

- **Baseline.** The census of `4fcb0d6819` gives `census_digest = blake3-256:81c7931c…`, identical across two independent runs (deterministic replay). The self-scope measures:
  - 98 artifacts;
  - 10,482 facts, 54,229 typed records and 1,056 obligations;
  - 61,739 graph nodes and 78,759 edges;
  - cargo closure CLOSED;
  - 632 UNKNOWN and 120 UNSUPPORTED facts;
  - CALL/CONTROL_FLOW/DATA_FLOW/STATE/EFFECT/OWNERSHIP/CONCURRENCY/PERSISTENCE coverage UNKNOWN;
  - verdict NOT_CLOSED.
- **G57 proves itself.** Its own change is proven by `recensus prove` against that baseline (`.atlas/evidence/census/G57/`).
- **Tests.** Nine unit tests pin the proof rules:
  - revision independence;
  - intended/unexpected/unobserved changes;
  - every forbidden regression;
  - explained unknowns;
  - replay divergence;
  - tampering;
  - a missing objective;
  - coverage intent.

  The mutation battery results are recorded in the generation evidence.

## Limits

- The census does not yet see `apps/cli`. G57's own CLI change is therefore invisible to the proof, which the baseline demonstrates. This is the G59 target.
- Typed-record projection is span-level.
- Docs are counted, not content-identified.
- No reconciliation certificate exists yet (G58).
