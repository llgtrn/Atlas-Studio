---
id: atlas.decision.0057.typed-local-method-resolution
type: decision
status: accepted
canonical: true
---
# ADR 0057 — Method calls on typed locals decided by the probe's first step (G139, NA-CALL-TYPE-RESIDUAL)

## Context

The resolution engine resolved path calls (G75) and `self.m()` (G79). On the self census, 25,245 call sites were unresolved against 3,825 resolved call edges, and most of the residual is method calls. `DEBT-CALL` names receiver types (`NA-CALL-TYPE-RESIDUAL`) as its next step, and `DEBT-PERSISTENCE`'s method-level sites (`sync_all`, `flush`) wait on the same step.

G79's argument does not depend on the receiver being `self`. The method probe's first step tries the receiver expression's own type by value, and inherent methods beat trait methods at that step. So the inherent method whose `self` form equals the receiver's type wins, ahead of every trait, for any receiver whose type is known.

## Decision

A local's type is known without inference when it is declared:

- **Where declared.** A parameter, or a `let` at the top level of the function body.
- **Which spellings.** `T`, `&T` or `&mut T` for a workspace type `T` spelled without generic arguments. Inside an impl of such a type, `Self` counts as `T`.
- **Bound once.** The name must be bound exactly once in the whole function, closures included. Every use then denotes that binding.

For `x.m(..)` on such a local, after its binding, the resolution engine applies the G79 probe:

- **Resolved.** The one inherent method of `T` in an impl without generics, whose receiver form (`self`, `&self`, `&mut self`) equals the declared form.
- **Unresolved, with a reason.** A different form, a trait-only method, a generic impl, or two candidates.

Closures inherit the enclosing function's typed locals.

Never claimed:

- untyped `let`s;
- rebound names;
- uses before the `let`;
- generic parameters;
- types with generic arguments;
- field receivers;
- method chains.

## Evidence

`evidence/census/G139/method-call-scip-differential.json`: the self census gains 61 resolved method calls (5,387 → 5,448 resolved CALL records) and loses none. All 61 agree with rust-analyzer SCIP: the symbol SCIP references at each anchor is defined inside the function Atlas names.

Five mutants were each caught by a test:

- the bound-once check dropped;
- the binding position ignored;
- generic arguments accepted;
- the declared form ignored;
- closures losing typed locals.

## What stays open

`NA-CALL-TYPE-RESIDUAL` stays queued for the inferred receiver types:

- untyped `let`s (`let x = T::new()`);
- fields (`self.field.m()`);
- chains;
- trait-object and generic receivers.

The last of these is where the datafrog mission M4 found `Iteration::changed → Variable::changed` invisible.
