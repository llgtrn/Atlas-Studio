---
id: atlas.decision.0058.trait-bound-receivers-dynamic-dispatch
type: decision
status: accepted
canonical: true
---
# ADR 0058 — Method calls through trait bounds recorded as dynamic dispatch to the declaration (G140, NA-CALL-TYPE-RESIDUAL step two)

## Context

G139 decided `x.m()` for locals whose declared type is a plain workspace type. Receivers known only by their traits were never claimed:

- `&dyn Tr`;
- `impl Tr`;
- a generic `T: Tr`;
- `self` in a trait's default body.

Agent mission M4 hit exactly this on datafrog. `Iteration::changed` calls each `Variable::changed` through `dyn VariableTrait`, and Atlas could only answer that trace UNKNOWN.

Constructor-inferred `let`s (`let x = T::new()`) were considered first and rejected for now. Their common `&self` method needs the probe's autoref step, where a by-value trait method in scope (a blanket `into`) is tried first, so claiming them would not be sound without enumerating every trait in scope.

## Decision

For a receiver known only by workspace-trait bounds, rustc's method probe assembles the bound traits' methods as inherent-like candidates. These are object candidates for `dyn Tr` and parameter candidates for `T: Tr`. They beat every other trait at the first step.

The resolution engine therefore records:

- **When one method matches.** Exactly one bound trait declares `m` with the caller's receiver form. The call is `DYNAMIC_PARTIAL`, and its one callee is that trait method's declaration (`PathCallOutcome::Dynamic`). The interface is resolved; the implementation is chosen at run time (`dyn`) or instantiation (generic), and no implementation is named.
- **When it is not decided.** Unresolved with a reason:
  - a form mismatch (`receiver-form-differs`);
  - no bound method of that name (`method-not-in-bounds`, which includes supertraits' methods, not followed);
  - two bound methods (`ambiguous-associated`).
- **What counts as bounds.** Workspace traits, plus method-less markers (`Send`, `Sync`, `Unpin`, `Sized`, `Copy`, `?Sized`, lifetimes).
- **Never claimed.**
  - Any other bound (a foreign trait could supply `m`).
  - Impl-level generics.
  - `Box<dyn Tr>`, whose autoderef step through `Box` is not modeled.
  - Any `dyn` receiver when an inherent impl on a trait object exists.
- **Bound once.** G139's rule still holds: the local is bound once, closures included, and is used after its binding.

## Evidence

`evidence/census/G140/dynamic-call-scip-differential.json`: 21 `DYNAMIC_PARTIAL` calls on the self census. Their callees are trait method declarations and default methods of:

- `SemanticExtractor`;
- `SourceFrontend`;
- `StatementWalker`;
- `BrowserInstrument`.

All 21 agree with rust-analyzer SCIP.

Six mutants were each caught by a test:

- a bound method's form ignored;
- foreign bounds ignored;
- an inherent `dyn` impl ignored;
- where-clause bounds ignored;
- a default body's `self` untyped;
- a dynamic call recorded as static.

## What stays open

`NA-CALL-TYPE-RESIDUAL` stays queued for:

- **Inferred receivers.** Untyped `let`s need the autoref argument over the traits in scope.
- **Structural receivers.** Fields, chains and `Box`/`Rc` receivers.
- **Supertrait methods.**

A `DYNAMIC_RESOLVED_SET` (every implementation named) would need a closed-world rule for the trait's visibility.
