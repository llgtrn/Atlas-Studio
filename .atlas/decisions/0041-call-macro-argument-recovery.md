---
id: atlas.decision.0041.call-macro-argument-recovery
type: decision
status: accepted
canonical: true
---
# ADR 0041 — CALL recovers standard macro arguments and proves per-file coverage (G119)

## Context

The G118 hard-stop audit (ADR 0040) required one real native attack before the audit could complete. The pressure map selected `NA-MACRO-ARGUMENTS`, the bounded attack with the largest summed staleness. Its eight debts include CONTROL_FLOW, DATA_FLOW, STATE, OWNERSHIP and PERSISTENCE, none advanced since G35.

All eight expression walkers shared one permanent gap: a macro's arguments are an opaque `TokenStream`. So no file could ever claim coverage. Of the eight, only CALL has an executable exhaustiveness oracle (G74), so CALL is the bounded slice.

## Decision

1. **Recovery (`adapter::semantic::rust::macros`).** Macros are re-parsed, never expanded, and nothing is guessed.
   - **Expression macros.** The 20 standard macros whose documented input is a list of evaluated expressions are re-parsed as that list:
     - format, print, write, panic and assert families;
     - `dbg`, `todo`, `unimplemented`, `unreachable`;
     - `vec`, including its repeat form.
     A named format argument contributes its value.
   - **Inert macros.** 12 standard macros evaluate nothing (`stringify!`, `env!`, `concat!`, `include_str!`, ...) and contribute no expression.
   - **Shadowing.** A file-local `macro_rules!` with a standard name shadows it.
2. **Opaque sites.** These are:
   - every other macro invocation, and any recovered macro whose arguments do not parse;
   - every attribute on a function, impl, trait or module that is not built-in or a tool attribute, because an attribute macro can rewrite a body.

   Opaque sites are counted outside the CALL profile's exclusions (closures, async blocks, const/static initializers).
3. **Coverage.** The CALL walker walks recovered arguments. A file with no opaque site claims CALL coverage OBSERVED. Inside macro arguments, CALL builds no `Resolved` DATA_FLOW `PlaceRef`, because DATA_FLOW does not walk them.
4. **Scope coverage.** Scope coverage becomes the worst per-artifact coverage, where each artifact takes the best engine covering it. Previously it was the best status of any obligation, so one covered artifact would have covered the whole scope. That was latent while no partial dimension could claim coverage anywhere. G119 would have turned it into a false CALL closure; the self-scope certificate test caught it.

## Consequences

- **Obligations.** Syntactic CALL obligations went from 97 UNKNOWN to 75 OBSERVED and 23 UNKNOWN (98 Rust artifacts).
- **Records.** Syntactic CALL records went from 17,768 to 22,562, which includes 4,745 recovered call sites.
- **Scope.** CALL coverage stays UNKNOWN (23 files, and the resolution engine), so the certificate is still honest.
- **Proof.** `call_sites_equal_an_independent_syntax_enumeration_on_real_sources` now runs over every workspace source instead of 4 files. It re-parses macro arguments independently and requires, for each file:
  - exact call-site equality with the walker;
  - coverage OBSERVED if and only if nothing is opaque;
  - every resolved DATA_FLOW `PlaceRef` to name an existing record.
- **Falsification.** Eight mutants, all killed:
  - macro arguments not walked;
  - opaque sites never counted;
  - shadowing ignored;
  - attribute macros treated as built-in;
  - `stringify!` parsed as expressions;
  - coverage claimed whenever the file parsed;
  - the PlaceRef gate removed, which produced a real dangling reference at `adapter/src/browser/mod.rs:249`;
  - best-of scope aggregation restored.
- **Not done.** The other seven walkers need macro-specific semantics: format arguments are borrows, and implicit `{x}` captures are reads. They remain queued as `NA-MACRO-ARGUMENTS-WALKERS`.
