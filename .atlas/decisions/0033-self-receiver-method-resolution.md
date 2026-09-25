---
id: atlas.decision.0033.self-receiver-method-resolution
type: decision
status: accepted
canonical: true
---
# ADR 0033 — `self.m()` resolved by the method probe's first step

## Context

After G75 (ADR 0031), the residual CALL gap is method calls, which need the receiver's type.

The clef cycle (G78, `../evidence/campaign/16-clef.json`) measured the gap: 10,776 method calls. Of these, 595 are `self.m()` calls to a workspace inherent method. Their receiver's type needs no inference: inside an impl method, `self` has the impl's self type.

## Decision

Rust's method probe tries the receiver's type by value first, and at that step tries inherent methods before trait methods. So `self.m()` in a method of `impl T` (or `impl Trait for T`) is resolved when all of these hold:
- the caller's receiver is `self`, `&self` or `&mut self` (a typed receiver such as `self: Box<Self>` is never claimed);
- `T` has exactly one inherent method named `m` with a receiver;
- that method's receiver form equals the caller's, so it matches at the first step;
- its impl has the same generics, where clause and self-type arguments as the caller's impl, so an `impl G<u8>` method is never claimed for a `G<u16>` caller.

Otherwise the call is unresolved with a reason: `receiver-form-differs`, `generic-impl`, `method-not-inherent` or `ambiguous-associated`. A receiver other than `self`, and `self` in a trait default body, are left to type inference.

## Consequences

- **Resolved:** 496 `self.m()` calls on this repository, all agreeing with rust-analyzer 1.90.0 SCIP at the same anchor.
- **Refused:** 85 oracle-resolvable calls, each soundly:
  - 73 differ in receiver form. At the first step, an in-scope blanket trait such as `Into` could win, and this pass does not enumerate trait method names.
  - 6 sit on a differently shaped generic impl.
  - 6 are methods this pass cannot see.
- **Falsification:** 7 mutants, all killed.
