---
id: atlas.decision.0038.essential-complexity-audit
type: decision
status: accepted
canonical: true
---
# ADR 0038 — Essential complexity is debt, not a donor verdict (G116 retrospective audit)

## Context

The first-50 campaign reached its gate at G115: every donor is terminal. The retrospective audit at G116 read the whole ledger (G35–G115), every donor decision and the declared end-state (NORTH-STAR canonical chain, CAPABILITY-ARCHITECTURE, INVENTION-PIPELINE). It found a systematic failure mode.

- **The failure run.** Native attacks on open P0 dimensions came at G59–G62, G74, G75, G77, G79 and G83. The longest healthy gap was 12 generations. G84–G115 is a 32-generation run with no native attack on any open P0 dimension. Its priorities were P3 = 45, P0 = 5, P4 = 1 and P2 = 1 from G64 onward. It includes 15 consecutive compiler/construction donors (G89–G103) that built no construction component.
- **Conflation.** 27 of the 50 re-audited decisions used NOT_NOW evidence (no consumer, small graph, fast enough, the repository has no case) as if it proved the capability unnecessary.
- **Circular "no consumer".** VerificationEvidence (kani, verus, tlaplus), the `.atlas` container (zstd, arrow, agentir), the seal gate (capnproto, containers-image) and SandboxBackend (wasmtime, podman) were each declared consumerless because the consumer was itself an unbuilt essential component.
- **Exit E.** Exit criterion E was recorded as BLOCKED_ON_PREREQUISITES at G65. After that, no generation attacked any of its prerequisites.
- **A falsified zero.** CONCURRENCY has zero records although Atlas itself runs a scoped worker thread (`std::thread::scope` and `Builder::spawn_scoped` in `adapter/src/semantic/rust/resolve.rs`). The zero was a measurement gap, not a result.
- **Parallel carriers.** Several domains carry epistemic status and quantities outside the core: product `EvidenceBasis` and a String status, physical `evidence_level` as a String, graph f32 confidence, and physical and product side reports that never enter the graph. This is a second architecture in embryo.

## Decision

1. **Contract.** `contracts/ESSENTIAL-COMPLEXITY.md` separates three questions: whether Atlas needs this donor's mechanism, whether it needs the underlying capability, and whether current scale justifies building it now. It classifies every capability as `ESSENTIAL`, `SCALE_TRIGGERED` or `OPTIONAL`. It makes three invariants canonical:
   - DONOR_REJECTED does not imply CAPABILITY_CLOSED;
   - REFERENCE_ONLY does not imply CAPABILITY_UNNECESSARY;
   - CURRENT_SCALE_DOES_NOT_JUSTIFY does not imply FUTURE_ARCHITECTURE_DOES_NOT_REQUIRE.
   It also adds the anti-avoidance rule, Rule A (no complexity without evidence) and Rule B (no deferral of established essential complexity).
2. **Debt ledger.** `roadmap/ESSENTIAL-COMPLEXITY-DEBT.toml` holds:
   - 39 debts, each with first observation, last advance, maturity (M0–M5), evidence, unknowns, donors examined, next attack and deadline gate;
   - 7 numeric scale triggers;
   - exit E as a finite 28-node dependency graph. The first missing node on the critical path is M1, typed-record sections in `.atlas`;
   - the ordered `native_attack_queue` of 23 attacks;
   - where each futures epistemic state lives;
   - the core physical primitives;
   - the Chronica counterfactual blockers.
3. **Alarms, derived from the ledger.**
   - REVIEW when `stale_generations > 12`. That is the healthy gap.
   - PRIORITY_ESCALATION when `stale_generations > 24`.
   - Skip budget N = 3, derived from the donor attempts that preceded past native attacks (1, 2 and 5, median 2).
   - An escalated or skip-budget debt must be queued, be blocked by a queued debt, or be frozen with an unfreeze gate.
   - At G116, 36 debts are escalated and 9 have exhausted their skip budget.
4. **Re-audit.** The G116 re-audit (superseded at G118 by `roadmap/DONOR-CAPABILITY-AUDIT.toml`, which covers all 63 donors) re-audits all 50 decisions with Q1 (mechanism) and Q2 (capability). It adds `reference_class` (FOREVER or UNTIL_TRIGGER, with a numeric or milestone trigger), DEFERRED fields, a `boundary_type` for each EXTERNAL_BOUNDARY, and counterfactuals. Terminal donor states are not reverted, and no donor is re-absorbed.
5. **Routes.** `roadmap/FUTURISM-ENGINEERING-END-STATE.toml` maps each of the 18 end-state capabilities and 17 domains to debts. `roadmap/FRONTIER-CAPABILITY-CLUSTERS.toml` routes all 129 frontier families into capability clusters.
6. **Gates.** `FIRST_50_CAMPAIGN_COMPLETE` (MET_G115) and `FOUNDATIONAL_ATLAS_READY` (NOT_MET, 9 criteria) are separate gates. While the latter is NOT_MET, frontier expansion stays frozen and generation selection draws from the queue head before any donor.

## Consequences

- G117 is the queue head: NA-CONCURRENCY-RESOLVED, a P0 native attack.
- The campaign resumes as native attacks. A native attack may study donor evidence (Rule A), but it may not end as a donor decision without a native change.
- Runtime tests (`essential_complexity`) enforce the alarms, the queue, the reaudit's classes and triggers, cluster routing, end-state ownership, the construction graph's closure and the gate/freeze coupling.
