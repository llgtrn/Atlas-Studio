---
id: atlas.decision.0068.typescript-javascript-bounded-profile
type: decision
status: accepted
canonical: true
---
# ADR 0068 — The bounded first TypeScript/JavaScript semantic profile (G152)

## Context

Replay R1 (ADR 0067) showed that Atlas was blind outside Rust:
- GitNexus is 2,543 TypeScript files, and every dimension of them was UNSUPPORTED.
- GitNexus, run as an oracle, found JavaScript in Atlas Studio that Atlas itself could not see.

DEBT-MULTILANGUAGE had stood at M1 (contract only) since G35, and three earlier donors had left it unchanged.

The owner directed that the same GitNexus pin be the falsification target of the second-language attack, with tree-sitter as a syntax substrate only: a parse is never semantic support.

## Decision

1. **Syntax substrate.** tree-sitter 0.25 with tree-sitter-typescript 0.23 and tree-sitter-javascript 0.23, all MIT-licensed, parse the source. The grammar follows the extension: TSX for `.tsx`, TypeScript for `.ts`, `.mts` and `.cts`, and JavaScript (JSX included) otherwise.
2. **Atlas's own extractor.** `atlas.typescript.source-semantic.v1` is registered for `typescript` and `javascript`. It emits typed records under the Rust contract. The declared profile is:
   - **FUNCTION_IDENTITY and FUNCTION_SIGNATURE:**
     - function declarations;
     - arrow functions and function expressions bound to a `const`;
     - class methods, with static methods as associated functions and the class as owner;
     - anonymous functions and object-literal methods as `{closure@line:column}` regions under `fn <enclosing>`, as in Rust;
     - a generated `{module}` region for top-level calls.

     Signatures carry spelled parameter and return types, `async`, visibility (`export`, `module`, `local`, or a class accessibility) and a comment-free body fingerprint. Overload, interface and abstract signatures have no body and are not functions.
   - **SYMBOL:** definitions of functions, classes, interfaces, type aliases, enums and module variables; imported names as declarations under `import <source>`.
   - **CALL:**
     - Every call and `new` site is recorded with its spelling and attributed to its innermost region.
     - A bare identifier resolves only to a module-level function of the same file whose name is bound exactly once in the file and never assigned; lexical scoping then fixes the callee, so the record is DERIVED.
     - `new Name` resolves to the class's constructor under the same rule.
     - Resolution waits until the whole file is walked, because declarations are hoisted.
     - Member, imported, callback, `this`, dynamic, JSX, accessor, decorator and tagged-template callees stay unresolved.
   - **Obligations:**
     - An error-free parse makes FUNCTION_IDENTITY, FUNCTION_SIGNATURE and SYMBOL exhaustive; a syntax error or excessive nesting leaves them UNKNOWN with their observations.
     - CALL is always UNKNOWN with its observations.
     - TYPE, CONTROL_FLOW, DATA_FLOW, STATE, EFFECT, OWNERSHIP, CONCURRENCY and PERSISTENCE are UNSUPPORTED, never guessed.
3. **Resource bounds.** Extraction runs on a 256 MB stack, and nesting deeper than 1,000 is not walked; such a file stays UNKNOWN.

## Evidence (`evidence/replay/R2-gitnexus.json`)

The same GitNexus pin (06ce60beb674) was measured at E1 and at E2:

| Measure | E1 | E2 |
|---|---|---|
| Functions | 363, all Rust fixtures | 65,414 (65,051 TypeScript or JavaScript) |
| CALL records | 228 | 249,271 |
| SYMBOL records | 751 | 42,424 |
| Resolved call edges | 6 | 21,045 |

The challenge question, which code realizes impact analysis, now resolves to functions. `LocalBackend::_impactImpl` (`src/mcp/local/local-backend.ts:7098`) calls `ambiguityReport`, `logQueryError` and `nonBlankUid`, which the source confirms are module-level functions at lines 243, 611 and 311. Its caller is `this._impactImpl` in `impact` (line 7062), which is correctly left unresolved and reached only as INFERRED.

On Atlas Studio itself, 48 JavaScript functions (`adapter/src/browser/*.js`) are now censused.

7 mutants are killed; the hoisting mutant was rewritten to resolve each call while walking before it could be killed. Re-running the donor itself found closures without an enclosing region (module-level callbacks, class fields of anonymous classes); they are now scoped under their real region, and a test covers it.

## Consequences

- DEBT-MULTILANGUAGE moves from M1 to M3_SINGLE_ENGINE_BOUNDED for this profile.
- **What remains open:**
  - cross-file import and module resolution;
  - member and `this` calls;
  - types and data flow;
  - the self census's scope, which excludes `tools/`.
- Donors with TypeScript or JavaScript become revalidation candidates.
- The re-materialized GitNexus source was deleted after the measurement. A first attempt was refused by a safety check because the working directory had been left inside the checkout; the removal was re-issued from the repository root on the owner's directive. The replay is PROCESSED with the source extinct, and the verdict REFERENCE_ONLY_UNTIL_TRIGGER is decided at E2.
