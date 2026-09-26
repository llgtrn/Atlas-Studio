---
id: atlas.decision.0067.full-oss-replay-and-capability-epochs
type: decision
status: accepted
canonical: true
---
# ADR 0067 — FULL_OSS_REPLAY and Atlas capability epochs (G151)

## Context

The repository owner ordered a full OSS replay campaign. Every canonical donor is to be replayed through successively stronger versions of Atlas: each donor faces the strongest Atlas produced so far, and any new capability may reopen an old conclusion. The campaign also explicitly authorizes one new donor, GitNexus.

Two gates stay in force:
- NEW_DONOR_PROGRESSION stays BLOCKED (FOUNDATIONAL-ATLAS-READY, machine-enforced since G118).
- FRONTIER_EXPANSION stays frozen.

The replay must therefore be a distinct, bounded lane rather than a loophole in either gate.

## Decision

1. **The FULL_OSS_REPLAY lane** (`roadmap/FULL-OSS-REPLAY.toml`).
   - **The replay set.** It is the deduplicated union, by canonical remote, of every repository the donor ledgers name: the donor corpus, the First-50 campaign, the capability audit, the repo-exact frontier, revalidation, the working set, and the campaign and provenance records. Explicitly authorized donors are added to it. At G151 that is 160 repositories: the 153-repository frontier, 6 legacy unadmitted checkouts, and GitNexus.
   - **Eligibility.** Only these repositories are eligible. The lane is distinct from NEW_DONOR_PROGRESSION, FRONTIER_EXPANSION and HISTORICAL_DONOR_REVALIDATION, and neither gate is relaxed.
2. **Capability epochs** (`[[epoch]]`).
   - E0 is Atlas at the campaign start (G150).
   - An epoch advances only when Atlas gains a new or materially stronger capability, never for a docs-only or test-only change.
   - A donor verdict is valid at the epoch that decided it. It is re-checked or reopened (REVALIDATION_REQUIRED) after a later epoch, and newer decisions are appended without rewriting history.
3. **The REPLAY generation kind.** A replay generation must:
   - name its ledger record, an exact pin, and the epochs before and after;
   - record its evidence: the Atlas_N mission (what Atlas knew, the challenge question and Atlas's answer), the differential, the N-versus-N+1 comparison, manual reads, the decision and the extinction;
   - leave no source behind.

   Every replay is an Agent-Worn mission and counts toward the mission cadence.
4. **Cadence and window.**
   - No more than one non-replay generation may pass between replays, unless it names the materialized donor as a `replay_prerequisite`.
   - One donor may be materialized at a time, under `.atlas/.cache/donors`, and nothing else may be there.
5. **Licenses.** A restrictively licensed donor (for example PolyForm Noncommercial) is run and studied for research and testing, and its code is never copied (`code_reuse = NONE`).
6. **Separate outcomes.**
   - A donor decision never closes capability debt.
   - A capability advance must move the epoch and carry the comparison of the donor as seen at N and at N+1.
7. **Machine enforcement.** Runtime tests check:
   - that the ledger accounts for every canonical donor once and restates its counts;
   - pins, epoch-relative verdicts, extinction and license, and the one-donor window;
   - the replay cadence;
   - the REPLAY kind's record and evidence.

## First replay: GitNexus (R1, `evidence/replay/R1-gitnexus.json`)

**Donor:**
- Pinned at 06ce60beb674, PolyForm Noncommercial 1.0.0.
- Run for research and testing only; no implementation copied.

**Atlas_N (E0) on the donor:**
- It saw 3,102 components.
- Its 363 functions are all Rust test fixtures.
- The 2,543 TypeScript files are UNSUPPORTED in every dimension.
- The challenge question — which code realizes impact analysis? — got the answer UNKNOWN, and Atlas invented none.

**Differential:** GitNexus and Atlas were run on Atlas Studio itself and checked against the source:

| Question | Classification |
|---|---|
| Direct callers | BOTH_CORRECT |
| Transitive dependents above a closure region | DONOR_CORRECT |
| Trait-impl methods on concrete receivers | DONOR_CORRECT |
| `main` → `compare_designs` across a renamed package | ATLAS_CORRECT |
| Atlas Studio's own JavaScript tooling | DONOR_CORRECT |
| Hunk-level changed symbols, and execution flows | BOTH_CORRECT_DIFFERENT_ABSTRACTION |

**E1, Atlas-native and derived from observed behavior, not from GitNexus source:**
- **Resolution.** Lifetime parameters are erased from impl shapes and declared types, because a lifetime never decides which method a call reaches. On Atlas Studio this adds 54 resolved call edges and removes 68 unresolved sites.
- **Impact frontier.** The frontier crosses what resolved callers cannot, each step INFERRED and counted by reason:
  - ENCLOSES: a closure region's enclosing function;
  - DISPATCHES_TO: a trait declaration an implementation is dispatched from;
  - UNIQUE_NAME: an unresolved call site spelled with a name exactly one workspace function carries. A name two functions share is refused, never guessed.
- **Frontier shape.** `callers` and `nearest` stay resolved dependents only. INFERRED dependents are counted by reason and listed apart (`inferred_nearest`), never mixed with resolved ones.
- **Effect on `walk_call_like`.** 12 resolved and 77 INFERRED dependents, now reaching `extract`, `systemize` and `main`.
- **Mutants.** 10 mutants are killed.

**N versus N+1 on the donor:** there is no delta, because GitNexus is TypeScript.

**Decision:** REFERENCE_ONLY_UNTIL_TRIGGER at E1. It is re-replayed when a TypeScript extractor lands.

**Queued gaps:** a second language (next replay: tree-sitter), trait-impl methods on concrete receivers, hunk-level impact seeds, and execution flows.

**Extinction:** the source, the analyzed clone and GitNexus's index are all deleted.

## Consequences

- The next donor faces E1.
- The six legacy unadmitted checkouts stay BLOCKED_ON_USER. Their deletion still needs the owner.
- The twelve admitted legacy checkouts are replayed, and then extinguished, in turn.
