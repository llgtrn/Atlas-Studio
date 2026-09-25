---
id: atlas.contract.essential-complexity
type: contract
status: active
canonical: true
---
# Essential Complexity Contract

~~~text
REMOVE ACCIDENTAL COMPLEXITY AGGRESSIVELY.
CONQUER ESSENTIAL COMPLEXITY DELIBERATELY.
NEVER CONFUSE THE TWO.
~~~

## Purpose

Atlas falsifies mechanisms before absorbing them and refuses machinery that has no measured need. That discipline has a failure mode:

~~~text
current Atlas is small → mechanism not needed today → REFERENCE_ONLY / DEFERRED → source deleted → repeat
~~~

Repeat that often enough and Atlas becomes excellent at explaining why difficult machinery is unnecessary, while never building the machinery its declared end-state requires (`../architecture/constitution/NORTH-STAR.md`, `../architecture/CAPABILITY-ARCHITECTURE.md`, `../roadmap/FUTURISM-ENGINEERING-END-STATE.toml`).

This contract separates three questions that earlier decisions sometimes merged:

- Does Atlas need **this donor's mechanism**?
- Does Atlas need **the underlying capability**?
- Does the **current scale** justify building it **now**?

It was established by the G116 retrospective audit (`../decisions/0038-essential-complexity-audit.md`) and made machine-enforced by the G118 hard stop (`../decisions/0040-hard-stop-anti-avoidance.md`).

**Accounting.** Three quantities, never interchangeable: the First-50 campaign (50 donors, 5 of them never admitted to the corpus), the donor corpus (58 records in `../references/donor-corpus.toml`, 13 outside the First-50), and the repo-exact OSS frontier (153 repositories). `../roadmap/DONOR-CAPABILITY-AUDIT.toml` classifies all 63 distinct donors.

## Complexity classes

Every capability that a donor decision, contract or roadmap item touches belongs to exactly one class.

| Class | Meaning | Allowed outcome while unsolved |
| --- | --- | --- |
| `ESSENTIAL` | The declared end-state cannot be reached without it. | Stays `OPEN` in the debt ledger with an attack plan, whatever happens to individual donors. |
| `SCALE_TRIGGERED` | Not justified at current scale, but required once a measurable threshold is crossed. | Parked behind an explicit numeric or milestone trigger, re-checked every generation. |
| `OPTIONAL` | Its absence does not block the end-state, because the underlying capability is solved another way or is not required. | May remain `REFERENCE_ONLY` or `REJECTED` indefinitely. |

A capability's class is derived from the canonical documents, never from what the current repository happens to exercise.

## Canonical rules

These rules have equal force. Tests enforce them (`runtime` tests, module `essential_complexity`); a regression fails CI.

1. **DONOR_REJECTED does not imply CAPABILITY_CLOSED.**
2. **REFERENCE_ONLY does not imply CAPABILITY_UNNECESSARY.**
3. **CURRENT_SCALE_DOES_NOT_JUSTIFY does not imply FUTURE_ARCHITECTURE_DOES_NOT_REQUIRE.**
4. **No current consumer does not imply never build the consumer.** "No current consumer" is a valid reason only for an `OPTIONAL` capability, or for a `SCALE_TRIGGERED` one with an explicit trigger. For `ESSENTIAL` complexity it means the consumer itself may be missing, and that missing consumer is audited as debt.
5. **Anti-avoidance rule.** A donor may be terminal. An essential capability may not become terminal merely because a donor is terminal. It stays `OPEN` until Atlas has one of:
   - a native solution (`NATIVE_SOLUTION`),
   - an explicit permanent external boundary (`PERMANENT_EXTERNAL_BOUNDARY`), or
   - a proven alternative architecture (`PROVEN_ALTERNATIVE`).
   Each closure cites evidence that is not a non-absorbing donor decision.
6. **Rule A: do not implement complexity without evidence.** Every mechanism is falsified before it is absorbed.
7. **Rule B: do not defer essential complexity once necessity is established.** An `ESSENTIAL` capability with no executable attack plan blocks generation selection until it has one.

## Essential complexity debt

`../roadmap/ESSENTIAL-COMPLEXITY-DEBT.toml` is the ledger. It is also the **architecture pressure map**. Each `[[debt]]` records:

- capability, class and priority;
- first observed generation, and the generation of the last material advance;
- current state, current and required maturity;
- evidence, native support and known unknowns;
- donors examined, native mechanisms and external boundaries;
- why the debt is still open and the required end-state;
- blocking milestones, the next attack and a deadline gate;
- the risk if deferred.

**Maturity ladder**, shared by every debt:

| Level | Meaning |
| --- | --- |
| `M0_ABSENT` | Nothing in contracts or code. |
| `M1_CONTRACT_ONLY` | Specified, with no production code. |
| `M2_PARTIAL` | Production code covers part of the capability; obligations stay UNKNOWN. |
| `M3_SINGLE_ENGINE_BOUNDED` | One engine, with residuals bounded and enumerated. |
| `M4_MULTI_ENGINE_RECONCILED` | Independent engines agree, and disagreement is explicit CONFLICT. |
| `M5_CLOSED_WITH_ORACLE` | Closure proven against an oracle and enforced by tests. |

## Age alarm

- `age_generations` is the current generation minus the generation that first observed the debt.
- `stale_generations` is the current generation minus the last material advance.

The thresholds are derived from the ledger, not chosen blindly:

- **Healthy gap.** While the generation loop was working, native P0 attacks came at G59–G62, G74, G75, G77, G79 and G83. The longest gap between two of them was 12 generations (G62→G74, spanning exit criteria C and D and seven donor cycles).
- **The failure run.** After G83 came 32 generations (G84–G115) with no native attack on any open P0 dimension. They included 15 consecutive compiler/construction donors (G89–G103) that built no construction component.

| `escalation_state` | `stale_generations` | Consequence |
| --- | --- | --- |
| `TRACK` | `< 13` | Tracked; within the healthy gap. |
| `REVIEW_REQUIRED` | `>= 13` | The debt carries a review within the last 12 generations, with a concrete next attack. |
| `PRIORITY_ESCALATION` | `>= 25` | The debt holds a place in the ordered `native_attack_queue` (directly or through a queued blocker), which preempts donor progression. The queue head is the next generation planned in `PRIORITY.toml`. |
| `BLOCK_NEW_DONOR_PROGRESS` | `>= 32` | The length of the documented failure run. While any `OPEN` debt is here, donor progression is blocked (`../roadmap/FOUNDATIONAL-ATLAS-READY.toml` `[donor_progression]`). |

## Skip budget

When a native attack did follow donor study, it came after 1 to 5 donor attempts (EFFECT 1 at G76→G77, TYPE 2 at G78/G82→G83, CALL 5 at G69–G73→G74; median 2). The skip budget is therefore **N = 3**.

If 3 consecutive donor decisions link to the same `ESSENTIAL` debt and leave its maturity unchanged, donor progression stops. The next generation is a mandatory **native attack generation** on that debt. The ledger records the trigger, the test enforces that a native attack is planned, and the debt blocks donor progression until a native advance resets it.

## Decision classes

Every donor decision is re-audited in `../roadmap/DONOR-CAPABILITY-AUDIT.toml` (63 donors) with two independent decisions, the mechanism decision (`ABSORBED`, `REFERENCE_ONLY`, `REFERENCE_ONLY_UNTIL_TRIGGER`, `EXTERNAL_BOUNDARY`, `DEFERRED`, `REJECTED`) and the capability decision (`CAPABILITY_CLOSED`, `CAPABILITY_PARTIAL`, `CAPABILITY_OPEN_ESSENTIAL`, `CAPABILITY_OPEN_SCALE_TRIGGERED`, `CAPABILITY_NOT_REQUIRED`), from two questions:

- **Q1, donor-specific:** does Atlas need this donor's mechanism? `YES`, `NO`, `NOT_YET` or `EXTERNAL_BOUNDARY`.
- **Q2, capability-level:** is the underlying capability `ESSENTIAL_OPEN`, `ESSENTIAL_CLOSED`, `SCALE_TRIGGERED` or `OPTIONAL`?

A decision whose recorded reasoning treated `NOT_NOW` evidence as `NOT_ARCHITECTURALLY_REQUIRED` is marked `conflated = true`. Such evidence includes: no consumer, small graph, fast enough, the repository has no case, and similar.

Terminal decisions also carry the following classifications:

- **`REFERENCE_ONLY`** carries a `reference_class`:
  - `REFERENCE_ONLY_FOREVER`, when the capability is solved another way or is optional; or
  - `REFERENCE_ONLY_UNTIL_TRIGGER`, with a trigger that names a number or a named milestone. "Later" is not a trigger.
- **`DEFERRED`** carries a trigger, an owner capability, a blocking milestone and a re-admission condition.
- **`EXTERNAL_BOUNDARY`** carries a `boundary_type`:
  - `PERMANENT_REALITY`: the external system is part of the world Atlas works in, such as the host VCS or a certified solver;
  - `BOOTSTRAP`: used until Atlas's own stage exists, such as rustc/cargo;
  - `ORACLE`: an independent differential check, such as rust-analyzer SCIP;
  - `TEMPORARY_UNTIL_NATIVE`: the capability is essential, and Atlas must build its own layer above the boundary.
- **Every major deferred or rejected family** carries a future-scale counterfactual:
  - `ALREADY_SCALES_CONCEPTUALLY`;
  - `EXPLICIT_TRIGGER`; or
  - `REQUIRES_REDESIGN`.

## Gates

`FIRST_50_CAMPAIGN_COMPLETE` and `FOUNDATIONAL_ATLAS_READY` are separate gates and never synonyms; both live in `../roadmap/FOUNDATIONAL-ATLAS-READY.toml`. `FOUNDATIONAL_ATLAS_READY` requires all of (the file carries the ten evaluated criteria):

1. self-recensus operational;
2. P0 essential semantic dimensions closed, or with bounded, enumerated residuals;
3. ADL consuming census truth beyond package dependencies;
4. a sealed `.atlas`;
5. a `SelectedDesign`;
6. a minimum `.atlasx`;
7. a construction path to at least one artifact;
8. an explicit incremental/scaling strategy with numeric, mechanically evaluated triggers;
9. an executable construction substrate (P6);
10. no essential debt that is unbounded, meaning without an attack plan or past escalation without a planned generation.

Until `FOUNDATIONAL_ATLAS_READY` is `MET`, frontier expansion stays frozen, and generation selection draws from the debt ledger before the donor frontier.

## Hard stop

The G118 audit made the stop machine-enforced. `../roadmap/FOUNDATIONAL-ATLAS-READY.toml` records:

- `[audit]`: `COMPLETE` only after a native attack generation proven by self-recensus;
- `[donor_progression]`: `BLOCKED` while the audit is incomplete, and afterwards while any `OPEN` debt is at `BLOCK_NEW_DONOR_PROGRESS` or past its skip budget. It pins the materialized donor checkouts;
- `[frontier_expansion]`: `BLOCKED`, pinning the repository count and lifecycle counts.

While blocked, the runtime tests fail on any change to a pinned donor-corpus record (`DONOR-CAPABILITY-AUDIT.toml` `corpus_*` fields), any new donor checkout, any frontier growth, and any generation of `kind = "DONOR"`. `../roadmap/ARCHITECTURE-PRESSURE-MAP.toml` ranks every debt by pressure, independent of donor counts, and records the selection of the next native attack.

## Frontier

`../roadmap/FRONTIER-CAPABILITY-CLUSTERS.toml` maps every family in the repository-exact frontier to a capability cluster. It maps every cluster to the debts it could attack, or marks it `OPTIONAL`. The frontier is a set of routes into debts, not a flat list of interesting repositories.

## Historical donor evidence

~~~text
A historical donor verdict is valid evidence only for the semantic capabilities available at the
time it was produced. When Atlas gains a materially stronger capability that could change that
verdict, the verdict becomes REVALIDATION_REQUIRED for uses depending on that capability.
~~~

- SOURCE_EXTINCT does not imply EVIDENCE_ETERNALLY_CURRENT.
- REFERENCE_ONLY at one generation does not imply REFERENCE_ONLY at a later one.
- ABSORBED at one generation does not imply that no further mechanism can be discovered.

`../roadmap/RECURSIVE-DONOR-REVALIDATION.toml` evaluates every audited donor against capability milestones. A historical First-50 donor becomes `REVALIDATION_REQUIRED` only when all three conditions hold. Generation age alone never triggers it.

1. Its decision depended on capability X.
2. A milestone for X landed after its last deep census.
3. That milestone can observe the donor's language.

Revalidation is **historical recensus**, not new-donor progression. `NEW_DONOR_PROGRESSION` stays blocked, and `HISTORICAL_DONOR_REVALIDATION` is allowed only when triggered. A revalidation:

1. re-materializes the exact pinned commit, cheapest sufficient scope first;
2. censuses it with current Atlas, next to the historical engine where one can be rebuilt;
3. records a machine-readable semantic delta and one outcome: `REVALIDATED`, `MECHANISM_FOUND` or `DEBT_REOPENED`;
4. deletes the source again, then proves a post-delete self-recensus.

It adds a decision layer and never rewrites the historical decision. It is not a way to postpone native attacks: at most one revalidation generation may sit between two native attacks, and a found mechanism must be resolved in a bounded follow-up (native implementation, permanent boundary or proven alternative).

