---
id: atlas.decision.0048.verification-obligations-and-evidence
type: decision
status: accepted
canonical: true
---
# ADR 0048 — Verification obligations, evidence and reports (G127, NA-VERIFICATION-EVIDENCE)

## Context

`contracts/VERIFICATION-METRICS-PERFORMANCE.md` has defined `VerificationObligation`, `VerificationEvidence`, `VerificationReport` and `VerificationFailure` since G38, but no code implemented them.

- **Why nothing moved.** Kani, Verus and TLA+ were each judged `EXTERNAL_BOUNDARY` (`ORACLE`) partly because nothing could consume their evidence, a circular absence that left `DEBT-VERIFICATION_EVIDENCE` stale for 88 generations.
- **What it blocks.** Construction node M2 blocks SelectedDesign (M7) and the seal gate (M8).

## Decision

1. **`core::verification`** implements the contract's verification vocabulary.
   - **Types:** eleven verification classes; obligations; evidence (producer, run, the exact candidate identity, environment, inputs, result, content hash, counterexample, status); outcomes; failures; policies; plans; reports.
   - **Obligation state is a function of the admissible evidence set.** Evidence is admissible only when it is `OBSERVED` and names the exact candidate. No admissible evidence means `UNVERIFIED`, never `SATISFIED`. Any `VIOLATED` means `FAILED`, with a typed failure carrying the counterexample. All `SATISFIED` means `SATISFIED`. Anything else is `UNRESOLVED`.
   - **Library-package policy:** SEMANTIC, UNIT and COMPATIBILITY.
   - **Blocking.** A required class without an obligation blocks, because a required obligation may not disappear. So does any required obligation that is not `SATISFIED`. Coverage is reported per class.
2. **`runtime::verification`** evaluates the self scope, whose candidate identity is its census digest:
   - **SEMANTIC:** the generation's `recensus prove` report, admissible only for the tree it proved (`after_census_digest`);
   - **UNIT:** `cargo test --workspace`;
   - **COMPATIBILITY:** the `.atlas` schema-history and older-container tests (`cargo test -p core atlas::`).

   Command evidence records that it ran with ambient authority (not sandboxed). `atlas-systemizer verification self` writes the report and its evidence, and exits non-zero unless the report is `ADMISSIBLE`.
3. **Construction node M2 is `EXISTS`.** SelectedDesign and the seal gate now wait only on typed `.atlas` sections (M1); transformation waits only on the sandbox.

## Evidence

`evidence/verification/G127-self-scope.json` is the self scope evaluated on the committed G127 tree. The debt's success condition was that a SEMANTIC/UNIT/COMPATIBILITY obligation set be evaluated for the self scope.

Six mutants were each caught by a test:

- no evidence counted as satisfied;
- evidence for any candidate admitted;
- violations ignored;
- a missing required class not blocking;
- hypothetical evidence admitted;
- recensus evidence bound to the predecessor tree.

## What stays open

`DEBT-VERIFICATION_EVIDENCE` stays open:

- the report has no consumer until the seal gate exists (`NA-SEAL-GATE`);
- verification runs outside the sandbox;
- only the library-package policy is defined.
