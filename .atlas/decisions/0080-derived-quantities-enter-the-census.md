---
id: atlas.decision.0080.derived-quantities-enter-the-census
type: decision
status: accepted
canonical: true
---
# ADR 0080 — Derived quantities enter the census with their lineage, and scaling keeps a kind (G165)

## Context

G124 (ADR 0045) brought declared quantities into the census and the graph, typed with their kind and interval. What the physical and product models derive from those declarations did not follow:

- motor torques, currents, heat and reach;
- bills of materials, costs and margins.

Those values lived only in the `physical` and `product` reports. They were side JSON outside the census, the graph, the certificate's counts and the census container. DEBT-QUANTITY_UNITS named this blind spot, and NA-QUANTITY-SIDE-REPORTS headed the native queue.

Atlas answered the Atlas-first question itself. The world model shows `runtime::physical` and `runtime::product` each exposing one `analyze_root`, used only by the CLI.

Bringing the values into the census exposed a second defect. The motor torque the physical model derives as joint torque / (gear ratio × efficiency) had no kind. Every quotient dropped the kind, so an undeclared N·m could join either torque or energy: exactly the debt's named unsoundness, now on a derived value.

## Decision

1. **Every derived quantity is a census fact.** The census runs the physical and product models over the declared nodes it already compiles. Each value becomes a `QUANTITY` fact beside the declared ones:
   - a physical derived quantity carries the status its model gives it: DERIVED, or SIMULATED behind a simulation;
   - a product metric per variant is DERIVED as `[low, high] CCY BASIS`, or UNKNOWN when an input was missing or unusable.

   The extractor names the model: `atlas.physical.derivation.v1` or `atlas.product.derivation.v1`. The provenance is the subject entity's ADL span.
2. **Each input is a `QUANTITY_DERIVATION` fact** (subject the entity, predicate the derived name, object the input). This answers why the value exists.
3. **The graph gives a derived quantity a node of its own identity,** owned by its entity through `HAS_QUANTITY`, so a metric named like a declared attribute never merges with it. `DERIVED_FROM` (Semantic, many-to-many, not owning, not causal: computation is not causation) links it to the first input that has a node:
   - a declared quantity;
   - another derived quantity (`Entity.name`, or a metric of the same entity);
   - a declared entity.

   An input with no node stays a fact only, and no node is ever invented for it.
4. **Scaling by a pure number keeps the kind.** A product or quotient still has no kind (torque × angle is not a torque; an angle has its own base dimension). Multiplying or dividing by a dimensionless number keeps the other operand's kind. A dimensionless number over a torque has none.

## Consequences

- **The two fixtures, Atlas_N → Atlas_N+1:**

  | Fixture | Facts | Graph nodes | Graph edges |
  |---|---|---|---|
  | two-link arm | 38 → 83 | 40 → 52 | 29 → 70 |
  | desk organizer | 29 → 109 | 30 → 54 | 4 → 76 |

  - Two-link arm: 12 derived quantities and their 33 inputs.
  - Desk organizer: 24 metrics across two variants.
- **The side reports are the oracle.** Every derived value equals the report's and every input appears once. The derived motor torque reaches the graph `DERIVED_FROM` the derived joint torque, which is `DERIVED_FROM` the declared link lengths. The motor torque is a torque.
- **Falsification.** 11 mutants were written, and 10 were killed:
  - emission dropped;
  - lineage dropped;
  - derived nodes merged into declared ones;
  - edges dropped or left dangling;
  - UNKNOWN metrics claimed DERIVED;
  - the kind rule dropped or widened in three places.

  The eleventh was equivalent: `with_kind` never lets a dimensionless quantity carry a kind, so the redundant check was removed.
- **Atlas's own census does not change in facts.** Its declared ADL has no physical or product entities.
- **What stays in the reports:** requirement verdicts, findings, simulations and market hypotheses.
- **Still unknown:** the kind lattice beyond four kinds, and the kinds of derived products such as power or heat.
