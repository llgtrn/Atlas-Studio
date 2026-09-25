---
id: atlas.decision.0049.affected-impact-closure-with-oracle
type: decision
status: accepted
canonical: true
---
# ADR 0049 — Affected-impact closure with a full-recompute oracle (G129, NA-IMPACT-CLOSURE)

## Context

`contracts/ARCHITECTURAL-INTEGRITY.md` ("Impact closure and continuous revalidation") requires every semantic change to be mapped to the semantic identities it affects and, through architecture-relevant relations, to the invariants it can change. Incremental work may skip a whole-world rerun only when it can prove that unaffected invariants remain unaffected.

- **Why nothing moved.** ADR 0009 chose full recompute for speed, and that speed argument was also used to close the closure question. `DEBT-IMPACT_CLOSURE` stayed `M1_CONTRACT_ONLY` for 88 generations.
- **What was missing.** Atlas never computed the transitive dependents of a change.

## Decision

1. **`core::composition::closure`** computes the closure of a set of changed paths.
   - **Inputs.** It reads the world model before the change in full. Of the model after it, it reads only the functions and components in the changed paths, which is what an incremental re-extraction of those paths would produce.
   - **Why function impact is one hop.** A function's composed behavior is local to its own records and its resolved call edges, so function impact propagates by rules:
     - **R1 seed:** every function in a changed path, before or after;
     - **R2 callees:** a seed's callees gain or lose a caller;
     - **R3 callers of removed functions:** their resolved call no longer resolves;
     - **R4 name candidates of added functions:** a call spelled with an added function's name may now resolve to it. This rule exists because resolution is name-based (G123's callee spelling), so an unchanged file can change.
   - **Components and subsystems** are those of the affected functions and the changed paths.
   - **Invariants are selected by kind:**
     - dependency: the calling subsystem's outgoing calls may change;
     - safety: the changed paths' subsystems;
     - authority: the changed or calling subsystems, plus any category the seed gains or loses;
     - state: keys the seed touches, or the changed paths' subsystems;
     - declared constraints: the file set changed.
   - **Global changes.** The workspace manifests, the root lockfile and the system's `.atlas/declared` ADL make the closure global. A fixture's manifest or ADL nested inside a subsystem does not.
   - **Semantic frontier.** Resolved transitive callers are reported separately as the frontier an agent should re-read, not as facts that change.
2. **`closure::oracle`** checks a closure against the full-recompute diff of the two models, level by level: functions, components, subsystems, state, relations and invariants. Its verdict is `SOUND` when every difference lies inside the closure; otherwise it lists each miss.
3. **`runtime::agent::normalize`** makes two revisions comparable.
   - **Why it is needed.** Record ids carry the revision, so the same function has a different id in every commit.
   - **What it does.** Normalization maps function ids to `path#scope#name` (with `@line` only to break a tie), reduces every other record id to its dimension, and re-sorts id-ordered lists. The same code at two revisions normalizes equal (tested).
4. **`atlas-systemizer agent closure --before <model> --model <model> (--changed <paths> | --git <from>..<to>)`** prints the closure and its oracle.

## Evidence

`evidence/census/G129/impact-closure-oracle.json` records seven changes: the main commits of G123 through G128, and G129's own change. Each was composed before and after by the same Atlas.

- **Soundness:** all seven are `SOUND`, with no missed difference at any level.
- **Function precision:** changed/affected ranges from 38/60 to 325/855.
- **A refinement the first run exposed:** a fixture's ADL made G124 global. G124's affected functions fell from 2,200 to 728 once only the workspace's declarations were global, still with no miss.

Seven mutants were each caught by a test:

- no callee propagation;
- no callers of removed functions;
- no name candidates;
- manifests not global;
- function ids not normalized;
- an oracle blind to function differences;
- a closure that reads the whole after model.

## What stays open

`DEBT-IMPACT_CLOSURE` rises to `M3_SINGLE_ENGINE_BOUNDED` and stays open:

- **No consumer.** Recensus and admission do not yet skip work outside the closure.
- **A named blind spot.** A changed `pub use` re-export or `mod` item can re-route path resolution in unchanged files by a name the model does not carry. Such changes occurred in the evidence without a miss, but no rule covers them: soundness there is observed, not derived.
- **Limited oracle.** Soundness is observed on recorded changes, not proven for every change.

## Amendment (G136, agent mission M4 on the datafrog donor)

The closure was first run outside Atlas on the 94 non-merge commits of `rust-lang/datafrog` that touch Rust (`evidence/revalidation/R2-datafrog.json`). Seven of the 94 were `UNSOUND`, and the misses fell into two defect classes that Atlas's own seven changes never exercised.

- **Renames.** `git diff --name-only` reports a renamed file only by its new path, so the old path's functions left the model unseen. This caused two misses (`src/bin/*.rs` moved to `examples/`, and `leapfrog.rs` renamed to `treefrog.rs`). `changed_paths` now passes `--no-renames`, so a rename yields both of its paths.
- **Introduced invariants.** The invariant rules walked only the invariants of the model before the change. An invariant that the change introduces was therefore never named, for example the `INV-STATE` of a field the change adds and writes. This caused five misses. The rules now also walk invariants that exist only after the change, under the same rules derived from the seed: an introduced invariant enters the closure by a rule, never wholesale.

Both fixes only add to the closure, so every change that was `SOUND` before stays `SOUND`. After the fixes, all 94 donor changes are `SOUND`. Function precision over the donor's history is 2,906 changed out of 5,101 affected function-revisions, and the closure covers 5,101 of the 11,483 function-revisions of the before models.

Three mutants were each caught by a test:

- rename detection restored;
- introduced invariants dropped;
- introduced invariants included wholesale.
