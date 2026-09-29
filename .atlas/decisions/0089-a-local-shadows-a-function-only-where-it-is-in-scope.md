---
id: atlas.decision.0089.a-local-shadows-a-function-only-where-it-is-in-scope
type: decision
status: accepted
canonical: true
---
# ADR 0089 — A local shadows a function only where it is in scope (G175)

## Context

G175 took NA-CALL-TYPE-RESIDUAL, the native queue head (pressure 81). The residual was measured on Atlas itself first: the resolver's outcomes over a snapshot of HEAD `6a3e8984`, joined to rust-analyzer 1.90.0's SCIP index of the same snapshot. 796 calls the index resolves to a workspace function were left unresolved or withheld:

- **457 `receiver-form-differs`.** 431 of them are `&self` methods whose autoref step the G142 guard refused. Nearly every core module imports `serde::{Deserialize, Serialize}`, and serde's lock closure is UNKNOWN (G173: its root splices build-script output).
- **169 `local-binding`.** 147 of them are `value(rest, "--out")` calls in `apps/cli/src/main.rs`.
- **150 `method-not-inherent`.** Trait-impl methods called on `self`.
- **20 others.** Open scopes, unresolved imports, external globs.

A single-segment call `f(..)` was refused as `local-binding` whenever any binding named `f` existed anywhere in the enclosing function (the G75 rule). Rust scopes bindings lexically ("Scopes", the Rust reference). A `let` binding is in scope only after its statement, to the end of its block. So a local named `value` bound somewhere else in a 1,000-line function hid every call of the module function `value` in it.

Attacking the rule also found three older holes where Atlas claimed a function the call does not name:
- a `macro_rules!` invocation in statement position that binds a spelled identifier (`m!(value);` expanding to `let $v = ..`);
- a raw binding (`let r#value = ..` hid nothing, because the name was stored with its `r#`);
- a binding in a pattern macro (`let same!(value) = ..`).

## Decision

1. **Scope ranges.** A function's locals are ranges, not a set: for each name (`r#` stripped), the source ranges `[from, until)` where one of its bindings is in scope. A single-segment path call names a local exactly when its position lies in such a range. Otherwise the item lookup decides, as before. The ranges follow the reference:
   - **fn parameters:** the whole body.
   - **`let` statement:** from the end of its `;` to the closing brace of its block. The initializer and the let-else block do not see it.
   - **`if let` / `while let` (let chains included):** from the end of the `let` through the guarded block. The `else` branch and the code after do not see it.
   - **`match` arm:** from the end of the pattern over the guard and the arm.
   - **`for` pattern:** the loop body only.
   - **closure parameters:** the closure, from its closing `|`.
   - **blocks inside types:** a pattern's type, a parameter's or return type, or a closure's input or output type (`[u8; { let v = ..; v() }]`) scope their own `let`s like any block.
   - **anything else:** a binding not placed otherwise is in scope from its start to the end of the function (the old rule, which is always sound).
2. **Macros.**
   - **Statement position.** An opaque invocation (one the G174 expansion left in place) in statement position may expand to `let $v = ..`. Every identifier its tokens spell is bound from the invocation to the end of its block.
     - An invocation `macros::recover` still reads as a standard macro binds nothing. With the crate's shadowing set kept for the walk, these are the invocations left opaque only for their implicit format captures.
   - **Expression position.** rustc rejects a `let` there, so an invocation in expression position binds nothing outside itself.
   - **Item position.** Only a `macro_rules!` definition is accepted there, and the locals its expansions introduce are hygienic, so it binds nothing.
3. **Every caller shares the ranges.** Closures and async blocks keep their own context: the enclosing ranges plus their own. The G142 typed lets, the G157 and G169 holder and temporary analyses, and the method probe read the same ranges through `resolve_call`. So a closure parameter shadowing a function now hides it from those passes too; the old function-level set left closure parameters out.
4. **Marker spans are real.** The G174 rewrite's marker attributes (`#[atlas_expanded_macro]`, `#[atlas_conditional]`) take their macro's or argument's source span. A span joined over rewritten code then starts where the source does. The scope bounds are also taken from tokens that are never synthesized: the arm itself, and the closure's `|`.

## Evidence

- **Atlas.** The G174 resolver and this one ran over one snapshot of HEAD `6a3e8984`, checked against rust-analyzer's SCIP index of that snapshot.
  - Resolved path calls: 9,214 → 9,383 (+169).
  - Withdrawn: 0. Retargeted: 0.
  - 9,379 of 9,383 are confirmed. The other 4 are the known `±` lines, read correct.
  - No oracle-confirmed call is left `local-binding`. The remaining refusals of that kind are real locals (closures named `run_git`, `target`, `systemize`).
- **Review.** An independent adversarial review of the change found two regressions, both reproduced on rustc 1.90.0 and closed here:
  - **Collapsed ranges.** A closure or arm body opening with an expanded standard macro had an empty scope range. The G174 marker attribute carried a call-site span, so `span()` over the body collapsed to (1,0). `|value| format!("{}", value())` was claimed as a call of the module function.
  - **Pattern types.** A `let` inside a block in a pattern's type (`let _a: [u8; { let value2 = other; value2() }] = []`) was placed at the enclosing binding's range.

  The review also recorded one older hole the change does not close (Consequences). It found the ranges, let-else, let chains, labeled, `unsafe` and `const` blocks, `@` and or-patterns, async closures, the syn statement-macro classification and the other `resolve_call` callers sound.
- **Tests:**
  - `a_local_binding_shadows_a_function_only_where_it_is_in_scope`: 37 expectations, each checked against rustc 1.90.0 edition 2024.
  - `a_standard_macro_left_opaque_for_its_captures_binds_no_local`.
  - `scopes_hold_over_expanded_macro_bodies_and_blocks_in_pattern_types`: the review's fixtures, the closure output type, and a call after a block-scoped `let` in a type.
  - Two expectations of `a_standard_macro_name_the_crate_may_rebind_stays_opaque` now read `local-binding`: an opaque statement macro spelling the name may bind it.
- **Mutants: 32 killed, 1 equivalent.**
  - 25 against the first version:
    - `let` scope start and block end;
    - the else branch; the condition scrutinee; let chains; `while let`;
    - the `for` iterator and the loop's end; arm leakage and guards;
    - closure parameters and their end; the closure rescan; fn parameters;
    - the statement-macro rule and its reach; expression and pattern macros;
    - raw names both ways; an exclusive start; any binding anywhere.
  - 1 for the standard-macro exemption.
  - 6 for the review fixes:
    - the expanded marker's span, paired with each of the two scope bounds;
    - pattern types bound in place, and pattern types not walked;
    - signature types and closure output types not walked.
  - Equivalent: the conditional marker's span. A conditional argument always sits inside the expanded tuple, whose own marker opens every span joined over it.

## Consequences

- **DEBT-CALL:** a local shadows a function only where it is in scope. Capability epoch E22. It fires no processed replay's trigger.
- **Still open** (older than this change, now recorded):
  - **A function-like procedural macro in statement position** may emit `let value = ..` with call-site hygiene without spelling `value`. Telling procedural macros from `macro_rules!` needs the dependency's declarations (NA-DEPENDENCY-DECLARATIONS). Binding every name after every statement macro would withhold the calls after each `debug!` or `info!`.
  - **A statement or tail macro expanding to a block item** (`def!();` defining `fn value`) is visible in its whole block. `collect_block` skips statement macros; queued as NA-BLOCK-MACRO-ITEMS.
- **Queued (measured here):** NA-DEPENDENCY-DECLARATIONS step two, the largest remaining residual on Atlas.
  - 431 autoref refusals beside `use serde::{Deserialize, Serialize}`.
  - serde's root binds both names by a named `pub use serde_core::{..}`, inside its zero-argument local `crate_root!()`. A same-named item from the unseen include would be a compile error (E0255), so the names are certain.
  - Reading them needs that expansion and the per-item path to serde_core's trait declarations.
