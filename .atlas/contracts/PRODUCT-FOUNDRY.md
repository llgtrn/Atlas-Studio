---
id: atlas.contract.product-foundry
type: contract
status: active
canonical: true
---
# Product Foundry

Product Foundry compiles a commercial opportunity into an engineered, costed, manufacturable product package:

need → hypothesis → requirements → engineering (Physical Engineering) → BOM → cost → manufacturing → packaging → SKU → listing (Creator Fabric) → market evidence → next product.

It is a loop, not a generator. It is not a business-plan writer.

## Product Semantic IR (ADL-declared, `core::product`)

| entity | meaning |
|---|---|
| `ProductHypothesis` | problem, target customer, cited `MarketEvidence`; always `HYPOTHESIZED` |
| `MarketEvidence` | an attributed market signal (never invented) |
| `Product` | currency (ISO 4217), planned volume (a hypothesis) |
| `Variant`, `Sku` | variant-scoped BOM lines, costs and prices; SKU code |
| `Assembly`, `Part` | counted structure; `Part.process` |
| `Material`, `Component` | priced leaves: `price = low..high CCY[/unit]`, `basis`, `source` |
| `BomLine` | parent, item, dimensioned quantity, optional variant scope |
| `CostItem` | `stage = factory \| landing \| channel \| fixed`, amount |
| `CostRate` | `kind = scrap \| platform_fee \| returns_reserve \| …`, rate `low..high%` |
| `Price` | variant or product price hypothesis |
| `Quote` | required for any QUOTED value |
| `ProductRequirement` | metric, `<=`/`>=`, limit |

Physical concepts are not duplicated. Parts will reference Physical Engineering geometry and material (G57+). Marketing assets are Creator Fabric outputs, and every claim in them must trace to product or physical state.

## Evidence levels and anti-hype rules (enforced)

- Every commercial value carries a basis: HYPOTHESIZED < ESTIMATED < QUOTED < PROTOTYPED < VALIDATED < PRODUCTION_MEASURED.
- A derived value carries the weakest basis of its inputs.
- An ESTIMATED or HYPOTHESIZED point value is refused, because uncertainty must be declared as a range. So is any value without a `source`.
- QUOTED and above require a declared `Quote`. Supplier pricing is never assumed.
- Missing or unusable inputs make every dependent metric UNKNOWN, never zero.
- Currencies are never converted silently.
- Requirements are decided over the whole interval: SATISFIED only if every value satisfies the requirement, VIOLATED only if none does, UNKNOWN otherwise.
- Market demand is never asserted. Citing evidence does not validate a hypothesis.
- Atlas never claims any of the following without evidence at the matching level:
  - market fit
  - profitability
  - manufacturability
  - certification
  - production readiness

## Unit economics (exact rational intervals, dependency-aware)

- `bom_cost` = Σ exploded leaf quantity × unit price (dimension-checked)
- `variable_factory_cost` = (bom + factory items) / (1 − scrap)  *(yield loss; ERPNext's byproduct netting is a different mechanism)*
- `tooling_amortization` = fixed costs / planned volume (basis ≤ HYPOTHESIZED)
- `factory_cost` = variable factory + amortization
- `landed_cost` = factory + landing items
- `variable_landed_cost` = variable factory + landing items
- `gross_margin` = price − landed
- `gross_margin_ratio` = 1 − landed / price
- `contribution_margin` = price·(1 − channel rates) − variable landed − channel items
- `break_even_units` = fixed costs / contribution, derived only when contribution > 0 across its range

Each form uses each uncertain input once, so the intervals are exact rather than widened.

## Donors

Donors enter through the bounded working set (`DONOR-WORKING-SET.md`) and only for a concrete lane blocker. The first wave (G56) was censused remotely:
- ERPNext, Medusa and Saleor are reference only.
- CadQuery (the parametric slice) is the G57 target.
- CuraEngine and PrusaSlicer are AGPL, so principles only. Both use the same Marlin-derived trapezoidal time model.
- OpenSCAD and KiCad are reference only.

## Lanes

The 29 `PRODUCT_FOUNDRY` lanes in `../roadmap/FRONTIER.toml` each record capability, state, implementation, donors, blocker, evidence, falsification and next action. The per-generation product ledger is `../roadmap/PRODUCT-FOUNDRY.toml`.
