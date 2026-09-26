---
id: atlas.decision.0065.scoped-certificate-and-seal-policy
type: decision
status: accepted
canonical: true
---
# ADR 0065 — Scoped certificate policy and SealPolicy (G149)

## Context

Construction nodes M3 ("scoped certificate policy") and M4 ("SealPolicy record and schema") belong to DEBT-SEAL_GATE and precede the seal eligibility gate M8.

`contracts/CENSUS-CERTIFICATE.md` says "Some scopes may permit UNKNOWN, UNSUPPORTED or IGNORED. The permission is policy/scoped and must itself be recorded; those states never disappear". `contracts/VERIFICATION-METRICS-PERFORMANCE.md` adds that "no provider may weaken the seal policy to make its own candidate pass".

The certificate emitted blockers as `CODE: detail` text, and no code was typed. No contract lists the fields of a seal policy.

DEBT-DEPENDENCY's nine dynamic dependency obligations were to be "resolved or permitted by a recorded scope policy".

## Decision

1. **Typed blocker kinds** (`core::seal::BlockerKind`).
   - Every code the certificate module emits is typed. `parse_blocker` refuses an unknown code rather than skipping it.
   - A test checks that each kind is a code the certificate writes.
2. **Scoped certificate policy (M3).**
   - A `ScopePolicy` gives every kind one of three dispositions:
     - REQUIRED_CLEAR;
     - ACCEPTED_BOUNDED_RESIDUAL, with an enumerated set of details or a count bound and a reason;
     - OUT_OF_SCOPE, with a reason.
   - `evaluate_certificate` refuses an unknown code, an unclassified kind, a required blocker and a residual beyond its bound.
   - A permitted blocker is listed in the verdict with its reason and never dropped.
3. **SealPolicy (M4).**
   - The record holds the scope, the verification policy a sealed candidate must be admitted under, the certificate scope policy, the HARD dimensions and the required integrity verdict (ELIGIBLE). Its identity is a digest of the canonical form.
   - `validate_policy` requires a disposition for every kind. It refuses any permission for REVISION_DIRTY, DEPENDENCY_UNRESOLVED, PROVENANCE_INCOMPLETE, NORMALIZATION_CONFLICTS, REPLAY_NOT_CONVERGED or COVERAGE_CONFLICT. It also refuses permitting UNKNOWN or UNSUPPORTED coverage for a HARD dimension. So a policy can be relaxed only within these bounds, whoever edits it.
4. **The declared self-scope policy.** `.atlas/declared/seal-policy.json` holds the strict default, and `atlas-systemizer seal policy [--check|--out]` checks or writes it.
   - Every blocker must be clear, and all twelve semantic dimensions are HARD.
   - The nine dynamic dependency classes, which the Cargo census cannot observe, are a residual bounded at a count of nine.
   - An unsealed root is out of scope, because the seal is what seals it.
   - Relaxing the policy is the owner's decision.
5. **Evaluation.** `atlas-systemizer seal evaluate --certificate` exits SEAL_NOT_ELIGIBLE unless every blocker is permitted.

## Evidence

`evidence/seal/G149-seal-policy.json`:
- **Self census.** The self census (G148 certificate) is NOT_ELIGIBLE. Its 13 refused blockers are:
  - the dirty revision;
  - no packaged root;
  - UNKNOWN coverage in 8 HARD dimensions;
  - unknown and unsupported facts;
  - a single engine where the Genome requires two.

  The nine dynamic dependency classes are listed as the permitted residual.
- **Tests.** Core tests type every code, refuse an unknown one, reject every forbidden permission and every missing disposition, and reject a permitted HARD dimension and an unstamped identity. They show bounds, enumerations, out-of-scope rules and residual listing working. The runtime tests check the declared policy and the self verdict.
- **Mutants.** 9 mutants are killed.

## Consequences

- **What M3 and M4 provide.** They exist, and the seal gate M8 has a policy to decide under.
- **DEBT-DEPENDENCY.** Its success condition is met in the "policy-permitted" form. The nine obligations stay open residuals and are never erased.
- **Certificate schema.** The certificate schema still forces a SEALED certificate to have no blockers. M9 therefore needs the certificate to carry permitted residuals separately, or SEALED to be decided by the gate.
- **Next on the critical path:** M8 (NA-SEAL-GATE), which consumes the policy, the certificate verdict, the verification report, the integrity report and a SELECTED design.
