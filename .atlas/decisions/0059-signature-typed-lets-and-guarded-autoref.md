---
id: atlas.decision.0059.signature-typed-lets-and-guarded-autoref
type: decision
status: accepted
canonical: true
---
# ADR 0059 — Let bindings typed by their callee's signature, and the probe's autoref step behind guards (G142, NA-CALL-TYPE-RESIDUAL step three)

## Context

G139 and G140 decide `x.m()` only when the receiver's declared type and the method's receiver form agree at the method probe's first step. Two large classes stayed unresolved:

- **Receivers the author never annotates.** `let model = compose(&report); model.explain(..)`.
- **Methods whose form differs.** A value calling a `&self` method (`let s = S::new(); s.look()`), and a `&mut T` calling a `&self` method.

The second class is decided by the probe's autoref step. That step comes after every by-value candidate, so it is claimable only when no by-value method of the same name can exist.

## Decision

1. **Signature-typed lets.** A `let x = ..` at the top level of a function body, bound once in the whole function (closures included), holds a value of a plain workspace type when its initializer is one of:
   - a path call resolving to a workspace function whose declared output is such a type (`Self` of an impl without generics included, a generic name in force excluded);
   - a struct literal;
   - a tuple-struct or enum-variant constructor.

   Paths with generic arguments are never typed.
2. **Guarded autoref.** For the equal-shaped inherent method of a plain type, a value calling `&self`/`&mut self`, or a `&mut T` calling `&self`, is resolved only when no by-value method of that name can come first. Any one of these withholds the claim:
   - a workspace trait declares a by-value method of that name;
   - the name is `into`, `try_into` or `into_iter` (std blanket or IntoIterator by-value methods);
   - the scope chain has an unseen name or a non-std import (it could bring a trait with blanket impls);
   - the type has a workspace impl of `Iterator`, `IntoIterator`, `DoubleEndedIterator`, `Read`, `BufRead`, `Write`, `Future` or `Stream`.

   The same rule applies to `self` (G79).

## Evidence

`evidence/census/G142/method-call-scip-differential.json`: the G141 and G142 engines on the same tree give 5,504 → 5,678 resolved CALL records: 174 new, none lost, none changed. All 174 agree with rust-analyzer SCIP.

Six mutants were each caught by a test:

- autoref unguarded;
- the blanket names ignored;
- by-value trait methods ignored;
- Iterator-like impls ignored;
- non-std imports ignored;
- the `let` bound-once rule dropped.

## What stays open

`NA-CALL-TYPE-RESIDUAL` stays queued for:

- **Method-call results and chains.** The receiver's type would come from a resolved method's output.
- **Fields.** `self.field.m()` needs struct field types.
- **For-loop bindings.** `GAP-LOOP-BINDING-RECEIVERS`, which is datafrog's `Iteration::changed`.
- **`Box`/`Rc` receivers and supertraits.**

The autoref guards are conservative: a non-std import anywhere in scope withholds every autoref claim there.
