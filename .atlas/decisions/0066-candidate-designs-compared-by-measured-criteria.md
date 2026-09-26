---
id: atlas.decision.0066.candidate-designs-compared-by-measured-criteria
type: decision
status: accepted
canonical: true
---
# ADR 0066 — Candidate designs compared by measured criteria (G150)

## Context

DEBT-DESIGN_GENERATION ("candidates over .atlas designs compared and handed to selection") had not advanced since G52. Its only support was the Creator Pareto front over visual recipes. Its success condition is "two candidates compared with explicit criteria"; its falsification condition is "a selection among fewer than two compared candidates".

G148 (ADR 0064) gave designs a `candidate_set`, but only checked that it named the design itself. `contracts/SELECTED-DESIGN.md` forbids a design "'selected' because only one candidate exists" and forbids the "highest model/provider score automatically becoming selected design". `contracts/CREATOR-FABRIC.md` says "when no candidate dominates, multi-objective selection preserves the Pareto alternatives", and `contracts/VERIFICATION-METRICS-PERFORMANCE.md` requires objectives "never collapsed into one scalar".

## Decision

1. **Criteria are measured by Atlas, not supplied** (`core::design::compare`).
   - A `Criterion` names a measure, a direction (the Creator `Direction`) and a stated meaning.
   - The measures:
     - ROOTS: the number of roots;
     - RECORDS of a per-function dimension: distinct record ids whose function is a root. Records of two engines under one id count once.
     - UNRESOLVED_CALLS: call sites in a root function that no engine resolved.
   - A per-function measure is undefined for a design with a root that is not a FUNCTION_IDENTITY. Such a design is recorded as unmeasured and does not compete. A RECORDS measure over a dimension whose records name no function is refused.
2. **A comparison** (`compare_designs`, `DesignComparison`).
   - It requires:
     - at least two distinct designs;
     - one coordinate and one container;
     - each design CANDIDATE or VALIDATED and valid on its own (ADR 0064);
     - each design naming exactly the compared set as its candidate set;
     - declared, distinct criteria.
   - It records every candidate's values, the Pareto front as a set (Creator `pareto_front`), each dominated candidate with the candidates that dominate it, and the unmeasured ones.
   - Its identity is a digest of its canonical form. It ranks nothing and selects nothing.
3. **Selection rests on a comparison.**
   - A SELECTED design must cite a comparison (`comparison`, which is not part of the design identity); without one it gets COMPARISON_MISSING.
   - `validate_selection` refuses:
     - a comparison whose identity does not verify, or one the design does not cite, or one at another coordinate;
     - fewer than two measured candidates;
     - a candidate set that is not the compared set;
     - a design that was not compared, was not measured, or is dominated.
   - `design check --comparison` applies these checks. A cited comparison that was not supplied is COMPARISON_UNAVAILABLE.
4. **CLI.** Three additions:
   - `design candidates` proposes several designs at one coordinate, each naming all of them;
   - `design compare` writes the comparison;
   - `design roots --path` distinguishes functions that share a name by where they are.

## Evidence

`evidence/design/G150-candidate-comparison.json`, over the G149 clean-HEAD container and its self-scope report:
- **Candidates.** Three candidate designs for a container reader:
  - `core::atlas::read`;
  - `runtime::design::read_container`;
  - both.
- **Criteria.** Four declared criteria, all minimized: effects, persistence, unresolved calls, roots.
- **Measurements.**
  - The decoder measures [0, 0, 0, 1].
  - The file reader measures [2, 1, 5, 1].
  - The decoder dominates both other designs.
  - The criteria do not measure capability, and choosing stays the owner's decision.
- **Refusals.**
  - The provider's attempt to select the dominated design is REJECTED: PRINCIPAL_IS_PROVIDER, PRINCIPAL_UNREGISTERED, DESIGN_DOMINATED, and an event it cannot author.
  - A single-design comparison is refused.
- **Mutants.** 16 mutants are killed. Two of them killed only after the tests were strengthened: records of two engines under one id, and a dominated selection checked through `check`.

## Consequences

- DEBT-DESIGN_GENERATION's success condition holds for designs over `.atlas`.
- **What remains open:**
  - Candidates are authored by a pilot, not generated.
  - The measures are counts over function roots, and none measures capability or performance.
  - DecisionProposal and ProviderReceipt lineage are still not recorded.
- The seal gate (M8) can require that a SELECTED design rests on a comparison.
