---
id: atlas.decision.0023.product-semantic-ir-and-unit-economics
type: decision
status: accepted
canonical: true
---
# ADR 0023 — Product Semantic IR and unit economics

## Context

The PRODUCT_FOUNDRY directive opens a new capability family, from need to manufacturable, sellable product.

The highest-leverage foundational primitive is a typed product model. It must satisfy three conditions:
- every commercial number carries its evidence level and uncertainty;
- costs roll up through a variant-aware BOM with exact, dimension-checked quantities;
- nothing claims more than its evidence supports.

Every later lane depends on this. CAD and print estimates feed BOM quantities, supplier quotes raise evidence levels, and pricing and experiments update hypotheses.

## Decision

1. **`core::product`** analyzes ADL-declared entities (contract `PRODUCT-FOUNDRY.md`):
   - `parse_money` and `parse_rate` give exact rational intervals; the currency is ISO 4217 and a per-unit price is dimensioned.
   - `EvidenceBasis` orders the evidence levels, and derived values take the weakest.
   - The BOM explodes through assemblies and parts to priced leaves. It refuses cycles, dimension mismatches and undeclared items.
   - Unit economics use dependency-aware closed forms.
   - Requirements are decided over whole intervals.
   - Hypotheses stay HYPOTHESIZED, and market uncertainty stays UNKNOWN without evidence.
2. **`runtime::product::analyze_root`** and **`atlas-systemizer product --root R`**. The command exits `PRODUCT_REQUIREMENTS_NOT_SATISFIED` on any finding or on an undecided requirement.
3. **Machine-readable state:**
   - 29 `PRODUCT_FOUNDRY` lanes in `FRONTIER.toml`;
   - the `PRODUCT-FOUNDRY.toml` product ledger;
   - donor candidates in the repo-exact frontier, with remote heads;
   - the next donor slices queued in `DONOR-WORKING-SET.toml`.

## Evidence

- **Desk organizer fixture** (two variants, every input an explicit assumption with a range and a source):
  - All 7 metrics per variant, plus break-even, equal an independent Python `fractions` oracle exactly. The oracle was written from the economic definitions, not from this code.
  - Small: landed 3.008–7.439 USD; margin ratio 0.690–0.896; break-even 2.6–18.1 units.
  - Every margin rests on a HYPOTHESIZED price.
- **The oracle caught two design errors before commit:**
  - naive interval subtraction produced a margin ratio above 1;
  - break-even double-counted fixed costs through amortization.

  Both were replaced with dependency-aware forms.
- **Mutation testing: 17 of 17 killed.** A first-battery survivor (currency mixing) exposed a silent-UNKNOWN path. Every value entry point now emits a no-silent-conversion finding.

## Limits and next

- BOM quantities are declared. Geometry does not yet derive them.
- There is no print-time model, and no quote, prototype or market evidence exists.
- Duty, warranty and payment fees are only generic cost items and rates.
- **Next (G57):**
  - native parametric CSG for the organizer (exact volume → DERIVED mass → material cost) with STL export;
  - a trapezoidal print-time estimate. CuraEngine and PrusaSlicer independently use the same Marlin-derived model; it would be reimplemented from principles, because both are AGPL.
