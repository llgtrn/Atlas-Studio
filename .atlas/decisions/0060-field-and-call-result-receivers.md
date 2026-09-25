---
id: atlas.decision.0060.field-and-call-result-receivers
type: decision
status: accepted
canonical: true
---
# ADR 0060 — Receivers typed forward through fields and call results (G143, NA-CALL-TYPE-RESIDUAL step four)

## Context

G139–G142 type a receiver only when it is `self` or a named local. Much of the residual is expressions:

- a field (`self.store.save()`, `n.leaf.poke()`);
- a call result (`self.leaf().touch()`, `make_node().walk()`);
- a `let` bound to either (`let l = self.leaf();`).

Each of these has a type its declarations already state.

## Decision

One function, `receiver_type`, gives a receiver expression's type as the probe first sees it, with the impl shape its method must match. It covers:

- `self` (G79, and `Self: Trait` in a default body, G140);
- a typed local after its binding (G139, G140, G142);
- a named field of a struct declared without generics, when the field is `T`, `&T` or `&mut T` for a plain workspace type;
- the result of a resolved method or path call whose callee declares a plain output;
- any of these in parentheses.

`method_outcome` applies the G79–G142 probe rules to it, including the equal form, the bounds and the guarded autoref step. The visitor uses it for every method call, which preserves every earlier outcome.

A top-level `let` bound once to a field or a call result is typed as well. Bindings are typed in order, so a later `let` sees an earlier one.

Never typed:

- generic structs' fields;
- types with generic arguments;
- reference expressions;
- untyped bases.

## Evidence

`evidence/census/G143/method-call-scip-differential.json`: the G142 and G143 engines on the same tree give 5,687 → 5,763 resolved CALL records: 76 new, none lost, none changed. All 76 agree with rust-analyzer SCIP.

Five mutants were each caught by a test:

- generic structs' fields recorded;
- parentheses opaque;
- the binding position ignored;
- a field's generic arguments accepted;
- call results untyped.

## What stays open

`NA-CALL-TYPE-RESIDUAL` stays queued for:

- **Loop bindings.** `for` bindings over typed collections (`GAP-LOOP-BINDING-RECEIVERS`).
- **Pattern bindings.** Bindings from `match` and `if let`.
- **Wrapped receivers.** Generic instantiations, `Box`/`Rc` receivers and std receivers.
