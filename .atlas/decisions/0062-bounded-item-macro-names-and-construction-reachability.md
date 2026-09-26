---
id: atlas.decision.0062.bounded-item-macro-names-and-construction-reachability
type: decision
status: accepted
canonical: true
---
# ADR 0062 — Bounded item-macro names; construction reachability in queue selection (G145)

## Context

**Queue selection.** At G144 the pressure map selected NA-FIRST-ARTIFACT for G145. Its primary debt, DEBT-ARTIFACT_IDENTITY, owns construction nodes M17 and FIRST_ARTIFACT. M17 requires M16, which requires M15, which requires MIN_ATLASX, and so on: ten missing nodes stand in front of it.

The selection rule checked only the debt-level `blocked_by` edges, which say nothing about the construction graph. The graph also still recorded M5 and M6 as MISSING, although G138 (ADR 0056) built both: the ArchitecturalIntegrityEnvelope and its report evaluator.

**Item macros.** The re-selected head is NA-CALL-TYPE-RESIDUAL, chosen by DEBT-TYPE's 61 stale generations. The resolution engine canonicalized no type spelling at all in eight core files. The cause was not the types themselves: any item-position macro invocation (`crate::vocabulary_enum! {..}`, `typed_id!(NodeId)`) marked its whole module *open*, and in an open module every name that is not found directly is uncertain, `String`, `usize` and `str` included.

## Decision

1. **Construction reachability.** An attack is selectable only if its primary debt owns no `construction_node`, or owns at least one MISSING node whose requirements all EXIST (the construction frontier).
   - This is added to the pressure map's selection rule.
   - The runtime test `the_queue_head_is_construction_reachable` enforces it on the ledger head.
   - M5 and M6 become EXISTS (G138).
   - NA-FIRST-ARTIFACT stays queued, blocked on its prerequisites.
2. **Bounded item-macro names.** An item-position invocation of a workspace `macro_rules!` may define only certain names, read from the transcribers without expanding:
   - the identifier after an item keyword (`struct enum union trait type fn mod const static`) spelled in any transcriber;
   - when a transcriber puts a metavariable in such a position, every identifier of the invocation.

   Only lookups of those names stay uncertain in the module. Every other name resolves as if the macro were absent.
3. **Which invocations are bounded.** The invocation must name the macro by textual scope (`name!`, defined earlier in the same module of the same file) or by `crate::name!` (the crate's `#[macro_export]` definitions). The union over every candidate definition and every arm is taken.
4. **Which invocations stay open.** The expansion is unbounded, and the module stays open as before, when:
   - a transcriber imports (`use`, `extern`);
   - a transcriber invokes a macro outside the std expression-macro allowlist, or through a metavariable;
   - a transcriber or the invocation carries an attribute outside the allowlist. The allowlist is `derive` of std or serde derives, `doc`, `serde`, lints, `cfg`, `repr`, `inline`, `must_use`, `non_exhaustive` and `default`.
   - no candidate definition exists.
5. **Propagation.** A glob import carries the source module's uncertain names into the importing module. Every place that trusted a glob entry or a miss in an open module now asks `uncertain(module, name)`: lexical lookup, `resolve_prefix` segments, path-call targets and type canonicalization. Autoref keeps treating a module with any bounded macro as open.
6. **Lifetimes and metavariables are not keywords.** `'static` is `'` followed by the identifier `static`, and `$fn` is a metavariable.

## Evidence

`evidence/census/G145/macro-names-scip-differential.json`, measured on the same tree with the G144 engine and the G145 engine:
- **Types.** 1,250 more type occurrences are canonicalized in 8 files. No occurrence that the G144 engine canonicalized changed its identity.
- **Calls.**
  - 15 more path calls are resolved to workspace functions. All 15 agree with the rust-analyzer SCIP index of the G144 tree.
  - 190 more path calls are resolved to external paths. All 42 standard-library ones are SCIP-confirmed.
  - No resolution was lost.
- **Falsification.** All 10 mutants of the new rules were killed by the adapter tests. Each rule was dropped in turn: the unbounded flag, glob propagation, the lifetime skip, invocation names, textual order, invocation attributes, the `resolve_prefix` glob check, the lexical miss, `use` in a transcriber, and the macro allowlist.
- **Reachability.** Against the G144 ledger, the reachability test fails: NA-FIRST-ARTIFACT owns no construction node whose requirements exist.

## Consequences

- **Macro-defined items are not collected.** A macro-defined type such as `NodeId` or `EnvelopeStatus` stays uncanonical, and calls to its functions stay unresolved. Expanding `macro_rules!` natively would make them workspace items; that is queued under NA-CALL-TYPE-RESIDUAL.
- **Other residuals.** Glob imports of external enums (`use std::cmp::Ordering::*`) still open their scope.
- **Selection.** The G145 selection is NA-CALL-TYPE-RESIDUAL (61) over NA-ATLAS-TYPED-SECTIONS (58). M1, the first missing node on the critical path, is on the frontier and next by pressure once DEBT-TYPE advances.

## Amendment (G147)

Decision 1 said construction nodes M5 and M6 "become EXISTS". The G145 edit went to a generated copy of the construction graph, and the ledger kept both nodes MISSING. The claim was not true when this ADR was written.

G147 records M5 and M6 as EXISTS at the graph's source, together with M1. The G145 selection is unaffected: DEBT-INTEGRITY_ENVELOPE, which owns both nodes, was not a candidate primary debt.
