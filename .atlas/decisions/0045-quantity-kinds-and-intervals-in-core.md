---
id: atlas.decision.0045.quantity-kinds-and-intervals-in-core
type: decision
status: accepted
canonical: true
---
# ADR 0045 — Quantity kinds and exact intervals enter the core (G124, NA-QUANTITY-IN-CORE)

## Context

`core::quantity` (ADR 0010) gave Atlas exact, dimension-checked quantities. Two essential debts stayed open at the head of the native attack queue.

- **`DEBT-QUANTITY_UNITS`.** Torque and energy share the dimension kg·m²/s² and compared equal (`1 J == 1 N*m`). Quantities never entered typed records or the graph; physical and product results were side JSON.
- **`DEBT-UNCERTAINTY`.** No uncertainty carrier existed on `Quantity`. Physical values dropped from `Rational` to `f64` without any marker.

The attack was worked through the Agent-Worn interface. `agent understand path:core/src/quantity` named the consumers: physical (16 calls), product (6), and the ADL constraint comparator (3). `agent explain` located `compare_attribute`, `quantity_of` and `static_joint_torques` as the edit points. `agent verify` held every invariant after the change.

## Decision

1. **`QuantityKind`** is one of `UNSPECIFIED`, `ENERGY`, `TORQUE`, `FREQUENCY` or `ACTIVITY`.
   - **How a kind is set.** The unit names it (`J` names energy, `Hz` frequency, `Bq` activity, under any SI prefix). Otherwise a trailing word declares it (`5 N*m torque`). A kind must have its dimension (`3 kg torque` is refused), and a joule never becomes a torque.
   - **How kinds combine.** Two different declared kinds never add, subtract or compare (`KindMismatch`). An undeclared kind joins any kind of its dimension. A product or quotient declares no kind.
2. **Exact interval uncertainty.** `Quantity.uncertainty` holds exact rational bounds, written `120 ± 0.5 mm`.
   - **Propagation.** Addition, subtraction, multiplication and division propagate the bounds exactly. A property test checks that every derived interval contains every exact result of operands drawn from the operands' bounds.
   - **Undecided cases.** Comparing overlapping intervals is `Undecided`, never guessed. A divisor interval containing zero is refused.
   - **Leaving exact arithmetic.** `Rational::approximate` returns an `Approximation`, an `f64` marked exact or not.
3. **ADL comparisons.** A kind mismatch is `VIOLATED` (`ATLAS-E059`). An undecided comparison is `UNKNOWN` (`ATLAS-E057`).
4. **Census and graph.**
   - **Census.** Every quantity-shaped declared attribute becomes a `QUANTITY` census fact. It is `DECLARED` with its canonical SI form, interval and kind, or `UNSUPPORTED` with the reason (`90 deg` is never approximated).
   - **Graph.** Each quantity is a typed `Quantity` node linked `HAS_QUANTITY` from its declared entity, which keeps its own kind.
5. **Physical model.** Every torque-dimension attribute must be a torque, so a declared energy is a `VIOLATED` finding. Computed static joint torques carry the `TORQUE` kind. The two-link-arm fixture declares its torques.

## Falsification

Eight mutants were each caught by a test:

- kinds that never conflict;
- interval addition pairing the wrong bounds;
- overlapping intervals ordered by nominal value;
- every approximation marked exact;
- unadmitted quantity attributes dropped;
- the physical model ignoring the torque kind;
- `with_kind` overriding a declared kind;
- an ADL kind mismatch downgraded to `UNKNOWN`.

## What stays open

- **`DEBT-QUANTITY_UNITS`** (advanced G124): derived physical and product quantities still leave through side reports (`NA-QUANTITY-SIDE-REPORTS`). The kind lattice has four kinds.
- **`DEBT-UNCERTAINTY`** (advanced G124): product money, simulation inputs and predictions still use their own carriers, and the simulation does not yet use the approximation marker (`NA-UNCERTAINTY-PREDICTIONS`).
- **Revalidation.** A `QUANTITY_KINDS_AND_INTERVALS` capability milestone was added to the revalidation mechanism. No historical donor verdict depended on these debts.
