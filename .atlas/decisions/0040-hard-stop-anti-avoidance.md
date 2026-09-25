---
id: atlas.decision.0040.hard-stop-anti-avoidance
type: decision
status: accepted
canonical: true
---
# ADR 0040 — The anti-avoidance discipline is a CI property (G118 hard stop)

## Context

ADR 0038 (G116) recorded essential complexity as debt, but the stop it described was prose. Donor work could still resume, and its accounting was wrong in three places:

- **Donor corpus:** it holds 58 records, not 57. 45 overlap the First-50 and 13 lie outside it.
- **First-50:** 5 of the First-50 donors (git, in-toto, nix, tlaplus, z3) were never admitted to the corpus.
- **Frontier:** the repo-exact frontier has 153 repositories. A campaign comment still said 146.

Completing the First-50 campaign said nothing about the corpus. Neither says anything about whether Atlas is foundationally ready.

The G118 audit classified all 63 distinct donors by capability. Some capabilities drew many donors and nothing native:

| Capability cluster | Donors | Absorbed |
| --- | --- | --- |
| Compiler IR | 13 | 0 |
| Construction | 8 | 0 |
| Distributed execution, simulation, physical engineering, AI reasoning, causal/probabilistic reasoning, real-world observation | 0 each | 0 |

33 historical decisions treated not-needed-now evidence as not-architecturally-required. They include glean's query IR ("no census-query consumer") and ADR 0009's "no measured justification".

## Decision

1. **REMOVE ACCIDENTAL COMPLEXITY AGGRESSIVELY. CONQUER ESSENTIAL COMPLEXITY DELIBERATELY. NEVER CONFUSE THE TWO.** This is canonical in `contracts/ESSENTIAL-COMPLEXITY.md`.
2. **Donor capability audit.** `roadmap/DONOR-CAPABILITY-AUDIT.toml` gives every donor two independent decisions:
   - a mechanism decision: `ABSORBED`, `REFERENCE_ONLY`, `REFERENCE_ONLY_UNTIL_TRIGGER`, `EXTERNAL_BOUNDARY`, `DEFERRED` or `REJECTED`;
   - a capability decision: `CAPABILITY_CLOSED`, `CAPABILITY_PARTIAL`, `CAPABILITY_OPEN_ESSENTIAL`, `CAPABILITY_OPEN_SCALE_TRIGGERED` or `CAPABILITY_NOT_REQUIRED`.

   The file also records capability clusters and their holes, and answers the 22 suspicious cases. The 13 non-First-50 corpus records are `DEFERRED` behind the hard stop, except Ladybird, an `EXTERNAL_BOUNDARY`. The file supersedes `DONOR-DECISION-REAUDIT.toml`.
3. **Debt ledger v2.** `roadmap/ESSENTIAL-COMPLEXITY-DEBT.toml` now has:
   - 41 debts carrying the full field set: evidence, age, unsoundness, blind spots, mechanisms rejected and absorbed, blocking priorities and exits, attack type, success and falsification conditions, deadline and escalation state. RESOURCE and CONSTRUCTION_TRANSACTION are new;
   - the 15 fundamental dimensions, measured;
   - the compiler/construction ownership table, where "rustc may stay external" never means "no compiler architecture";
   - the 29-node `.atlasx` DAG, now ending in recensus of the materialized output;
   - numeric P4 thresholds, evaluated mechanically against the post-change census;
   - 23 physical primitives, 9 epistemic states and the drift records.
4. **Escalation.** It is derived from the ledger: the healthy native-attack gap is 12 generations and the documented failure run is 32.
   - `TRACK` below 13 stale generations;
   - `REVIEW_REQUIRED` from 13;
   - `PRIORITY_ESCALATION` from 25;
   - `BLOCK_NEW_DONOR_PROGRESS` from 32.

   The skip budget stays at 3. A debt past it also blocks donor progression.
5. **Pressure map.** `roadmap/ARCHITECTURE-PRESSURE-MAP.toml` ranks every debt by pressure, which is staleness × (1 + the number of debts it blocks). It records the selection rule for the mandatory native attack: the bounded attack with the largest summed staleness. The rule selects `NA-MACRO-ARGUMENTS`.
6. **Foundational gate and hard stop.** `roadmap/FOUNDATIONAL-ATLAS-READY.toml` separates `FIRST_50_CAMPAIGN_COMPLETE` (true) from `FOUNDATIONAL_ATLAS_READY` (false, 3 of 10 criteria). It holds the hard stop:
   - `[audit]` becomes `COMPLETE` only when a native attack generation is proven.
   - `[donor_progression]` is `BLOCKED`. It pins the materialized donor checkouts.
   - `[frontier_expansion]` is `BLOCKED`. It pins the repository and lifecycle counts.
7. **Enforcement.** The runtime module `essential_complexity` has 19 tests that fail CI when:
   - a pinned corpus record changes, a checkout appears or the frontier grows while blocked;
   - a `DONOR` generation or a P3 generation appears after the audit;
   - a terminal donor closes a capability;
   - `REFERENCE_ONLY` is read as not required outside an OPTIONAL cluster;
   - "no consumer" is used against essential complexity without a drift record;
   - an essential debt's attack or deadline is vague ("later", "when needed");
   - a scale trigger is not numeric, or a crossed threshold is not FIRED;
   - an external boundary is untyped;
   - an escalation, skip-budget or progression state differs from its recomputation.

## Consequences

- Ten mutations of the enforcement were all caught:
  - a corpus record progressing;
  - frontier growth;
  - "later" as a next attack;
  - joern closing DATA_FLOW;
  - mlir read as not required;
  - a new checkout;
  - FOUNDATIONAL read as MET;
  - a DONOR generation;
  - an untyped boundary;
  - a stale debt downgraded to TRACK.
- At G118, 34 OPEN debts block donor progression. Donor work can resume only after native attacks advance them below the block threshold and past their skip budgets. First, the audit must be completed by the native attack generation G119.
