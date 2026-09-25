---
id: atlas.decision.0046.persistence-from-resolved-std-paths
type: decision
status: accepted
canonical: true
---
# ADR 0046 — PERSISTENCE from resolved std paths (G125, NA-PERSISTENCE-RESOLVED)

## Context

Every PERSISTENCE record was a callee-spelling guess (`INFERRED`, `UNRESOLVED`): a function named `commit` was persistence by spelling alone. `DEBT-PERSISTENCE` had not advanced since the ledger began, and after G124's tightened selection it was the pressure head of the native attack queue.

The attack was worked through the Agent-Worn interface. `agent explain` showed the template: `std_path_effects` (G77) and `std_path_concurrency` (G117) are declared tables with a single consumer, `resolve_rust_path_calls`. `agent understand` bounded the PERSISTENCE extractor, whose one entry point is `build_persistence`.

## Decision

1. **A declared std-path persistence table** (`core::semantic::persistence::std_path_persistence`), sorted and exact, following documented std contracts:
   - `std::fs::write` and `std::fs::copy` are `DURABLE_WRITE`;
   - `std::fs::read` and `std::fs::read_to_string` are `DURABLE_READ`;
   - `std::fs::File::sync_all` and `sync_data` are `SYNC`.

   Mutations without a fitting kind (`rename`, `remove_file`, `create_dir`) are not declared. A path absent from the table declares nothing.
2. **The path-resolution engine derives PERSISTENCE.** A call it resolves to a declared path becomes a `DERIVED` persistence site of the calling function, anchored at the call. `resolution` is `RESOLVED` (the resolved API is the evidence) and `place` is `UNRESOLVED` (the file it touches is not claimed). The engine's PERSISTENCE obligation stays `UNKNOWN` and names what is outside it: method calls (`sync_all`, `flush`, `commit`), closure bodies, macro arguments and non-std storage.
3. **The spelling engine is kept.** Its `INFERRED` guesses remain, marked as guesses. PERSISTENCE now has two engines and leaves the certificate's `MULTI_ENGINE_RECONCILIATION_ABSENT` blocker.

## Evidence

- **SCIP oracle.** The pinned rust-analyzer SCIP index (offline, working tree of the candidate) confirms 129 of 129 derived sites at their anchors with the same kind (`evidence/census/G125/persistence-scip-differential.json`). There are no disagreements. SCIP sees five further occurrences: four inside closure bodies and one inside `assert_eq!` arguments, all outside the engine's declared profile.
- **Falsification.** Five mutants were each caught by a test:
  - `write` declared a durable read;
  - the resolved API recorded as a spelling guess;
  - any `std::fs` path treated as a durable write;
  - the persistence obligation dropped;
  - derived sites claimed `OBSERVED`.

## Consequences

- **Debts.** `DEBT-PERSISTENCE` and `DEBT-MULTI_ENGINE` advance at G125. `DEBT-STATE`, listed on the attack, is not advanced: no STATE record changes.
- **Revalidation.** A `PERSISTENCE_RESOLVED_STD_PATHS` capability milestone joins the revalidation mechanism. No audited donor verdict depended on these debts.
