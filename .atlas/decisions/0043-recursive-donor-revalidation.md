---
id: atlas.decision.0043.recursive-donor-revalidation
type: decision
status: accepted
canonical: true
---
# ADR 0043 — Historical donor verdicts expire with the capabilities that produced them (G121)

## Context

Every First-50 donor was censused by the Atlas of its own generation, decided, and deleted. Atlas has since gained capabilities that older censuses structurally lacked:

- native path resolution (G75), `self.m()` resolution (G79) and canonical types (G83);
- resolved effects (G77, G91) and concurrency (G117);
- macro-argument recovery (G119, G120).

Treating those old verdicts as permanent would trap Atlas inside the conclusions of its younger, weaker self.

## Decision

1. **The rule, now canonical in `contracts/ESSENTIAL-COMPLEXITY.md`.** A historical donor verdict is valid evidence only for the semantic capabilities available when it was produced. When Atlas gains a materially stronger capability that could change the verdict, the verdict becomes `REVALIDATION_REQUIRED` for uses that depend on that capability.
2. **The mechanism, `roadmap/RECURSIVE-DONOR-REVALIDATION.toml`.** Each of the 11 capability milestones names its generation, the debts it advanced, and the languages it can observe. Every audited donor then carries:
   - its language;
   - its last materialization and deep-census generations;
   - the capabilities examined, and those available at decision time;
   - its semantic delta, status, reason, priority and rank.

   The trigger needs three conditions: the decision depended on capability X, a milestone for X landed after the donor's last deep census, and that milestone observes the donor's language. Age alone never triggers. Priority is the age of each advanced debt the donor depended on, summed over the delta.
3. **Two kinds of donor work, kept apart.** `NEW_DONOR_PROGRESSION` stays `BLOCKED`. `HISTORICAL_DONOR_REVALIDATION` is `ALLOWED_WHEN_TRIGGERED`, only in a `REVALIDATION` generation that executes a recorded `[[revalidation]]`. That record carries:
   - the debt it serves, the question, and the exact historical pin;
   - the materialization mode and scope;
   - evidence for both engines on the same source;
   - an outcome (`REVALIDATED`, `MECHANISM_FOUND` or `DEBT_REOPENED`) and a follow-up.

   A completed revalidation leaves no source behind.

## Consequences

- **R1, `rust`.** It ranked first because its G73 verdict predates five CALL/TYPE milestones, and G73 judged `rustc_hir_typeck` "far too large to absorb" without censusing it.
  - **Materialization:** the exact pin `d287eb7a` as a sparse cone, 40 files and 3.5 MB.
  - **Census:** Atlas rebuilt at the G73 commit and current Atlas ran over the same source.
  - **Result:** the historical engine missed a quarter of the method module's call sites and resolved none.
  - **Outcome: REVALIDATED, with a corrected reason.** The resolution core is bounded, but inseparable from rustc's type inference. The boundary and the REFERENCE_ONLY verdict on the implementation stand. `NA-CALL-TYPE-RESIDUAL` stays a native attack over receiver types that Atlas declares or derives.
  - **Deletion:** the source was deleted again, and G121's post-delete self-recensus is proven.
- **Next candidates:** `rust-analyzer`, `miri` and `egglog`. `clef` and `iris` need their language established first.
- **Guardrail:** revalidation cannot postpone native attacks. The queue head stays planned for the next native generation, with at most one revalidation generation in between.
