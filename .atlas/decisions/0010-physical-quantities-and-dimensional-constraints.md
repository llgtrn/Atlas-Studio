---
id: atlas.decision.0010.physical-quantities-and-dimensional-constraints
type: decision
status: accepted
canonical: true
---
# ADR 0010 — Physical quantities and dimensional constraints

## Context

The physical-engineering directive (G42) names units and dimensional correctness as the first foundational primitive. Dimensional validation must come before any probabilistic, optimization or simulation reasoning, so that `3 kg + 2 V` can never become engineering state.

No units concept existed anywhere in the contracts or the code. ADL attribute values were opaque strings, so `width = 0.12 m` failed `require p.width == 120 mm`. Engineering constraints such as `wall_thickness >= 2 mm` could not be expressed at all.

## Decision

1. **`core::quantity`** — the value model.
   - A `Dimension` is an exponent vector over 8 bases: the 7 SI bases plus **plane angle**. Angle is its own base, unlike SI's dimensionless radian, so an angle can never be silently added to a length ratio.
   - A `Quantity` is an exact `Rational` in coherent SI base units. This makes `0.12 m == 120 mm` hold exactly, and `0.1 m + 0.2 m == 0.3 m` too.
   - Unit expressions are parsed, including `m/s^2`, `N*m`, `kΩ`, `µF` and `mL`. There are 13 SI prefixes. `min` and `h` refuse prefixes.
   - Addition, subtraction and ordering across different dimensions return `DimensionMismatch`.
   - Only units with an exact rational SI factor are admitted. `deg` and `rpm` (π factors) and `degC`/`degF` (affine) return `UnsupportedUnit`; they are never approximated. Overflow is reported, not wrapped.
2. **ADL feature: quantity comparison** — the production consumer.
   - `require x.attr <op> value` with `op ∈ {==, >=, <=, >, <}` becomes `ConstraintCheck::AttributeEquals.comparison`, which defaults to `==` for older IR.
   - When both sides are quantity-shaped (`<decimal> <unit>`), they are compared by dimension and exact value.
   - Verdicts use ADR 0007's three values:
     - relation holds → `SATISFIED`;
     - relation fails → `VIOLATED` (`ATLAS-E050`);
     - different dimensions → `VIOLATED` (`ATLAS-E056`), because the declaration contradicts the requirement's dimension;
     - unsupported unit or overflow → `UNKNOWN` (`ATLAS-E057`);
     - an ordering operator over a non-quantity → `UNKNOWN` (`ATLAS-E058`).
   - Plain strings and bare numbers keep their literal `==` meaning.

### Feature-admission gate (ATLAS-DEVELOPMENT-LANGUAGE.md)

1. **Typed definition:** `Quantity`, `Dimension`, `Rational` and `Comparison`.
2. **Identity:** a quantity is identified by (exact SI value, dimension); its spelling is not identity.
3. **Status:** values stay `DECLARED`. A comparison is a deterministic derivation carrying the existing constraint derivation provenance (ADR 0003).
4. **Normalization:** equal quantities in different units are equal; no rounding happens.
5. **Lowering:** none yet. Units are carried in values for future geometry and electronics lowering.
6. **Failure/UNKNOWN:** unsupported units and overflow give `UNKNOWN`, never `SATISFIED`.
7. **Compatibility:** `comparison` defaults to `EQ`, and non-quantity `==` is unchanged. This repository's own four constraints still evaluate `SATISFIED`.
8. **Fixtures:** unit tests and a mounting-plate ADL fixture covering 12 comparison cases.
9. **Donor dependency:** none; no donor runtime is involved.

## Evidence

- **Independent differential oracle:** pint 0.25.3 (BSD-3), installed only in a scratch virtual environment. It agrees with Atlas on the SI factor (relative tolerance 1e-12) and base-dimension exponents for **75/75** unit expressions.
- The first differential run found a real defect: the litre was wrongly non-prefixable, so `mL` was rejected. It was fixed before commit.
- Pint confirms `deg` and `rpm` need π factors, and that `1 degC` is the absolute 274.15 K. Both are correctly refused.
- **11 mutants killed:** strict `>=`, dimension mismatch demoted to Unknown, unsupported promoted to Violated, quantity path bypassed, operator priority, wrong milli factor, angle folded into dimensionless, addition ignoring dimension, wrapping decimal overflow, and an approximate `deg` admitted.

## Known limits (recorded, not hidden)

- An exponent vector cannot tell torque (N·m) from energy (J), or Hz from Bq. That needs quantity kinds (mp-units `quantity_spec`, a frontier reference).
- Tolerances and uncertainty are not yet part of `Quantity`.
- `where` filters still compare literal strings.

## Donors

- **uom** (Apache-2.0 OR MIT) and **mp-units** (MIT) are the best design references (physical frontier TSV). Atlas's runtime, string-valued ADL needs a runtime exponent vector rather than uom's compile-time `typenum` dimensions, so no mechanism was absorbed and neither is admitted.
- **pint** served as an oracle only. It is not a dependency.
