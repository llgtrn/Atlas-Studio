---
id: atlas.decision.0051.one-uncertainty-carrier-and-marked-float-drops
type: decision
status: accepted
canonical: true
---
# ADR 0051 — One uncertainty carrier, marked float drops, uncertainty-honest physical verdicts (G131, NA-UNCERTAINTY-PREDICTIONS)

## Context

G124 (ADR 0045) gave quantities exact interval uncertainty, but other carriers stayed separate and conversions to floating point went unmarked:

- **Two interval types.** Product money kept a structurally identical but separate `Interval`.
- **The physical simulation converted every declared value with `si_value.to_f64()`.** That path never used the G124 `Approximation` marker, a known unsoundness recorded in `DEBT-UNCERTAINTY`, and it dropped declared uncertainty too:
  - a rated torque of `0.3 ± 0.01 N*m` fed the actuator-capability verdict as `0.3`;
  - an uncertain link mass fed the dynamics as its nominal value;
  - either way the verdict could be `SATISFIED` although part of the declared interval fails.

The simulation's outputs are Atlas's predictions (`SIMULATED`), so this is where uncertainty and predictions meet.

## Decision

1. **`quantity::Interval` is the one exact uncertainty carrier.**
   - **Arithmetic:** `point`, `hull`, `checked_add`, `checked_sub`, `checked_mul` (the hull of the four corner products, sign-aware), `scale` (a negative factor swaps the bounds), `is_nonnegative` and `locate`.
   - **Quantities** compute their bounds through it (`Quantity::interval`).
   - **Product money** is this type (`product::Interval` re-exports it). Its non-negative product became the general one, and serialized reports are unchanged.
2. **`quantity::FloatDrop` records every exit from exact arithmetic.** `Quantity::drop_to_f64(subject, drops)` returns the `f64` and records:
   - the subject;
   - the value;
   - whether the `f64` is exact;
   - the declared bounds it left behind.
3. **The physical verdict path:**
   - **Every value a simulation takes as `f64`** is recorded in `SimulationRecord.inputs` (13 drops for a 2-link arm and one trajectory).
   - **Joint limits are decided exactly** on the declared angles and limits, and an uncertain angle overlapping a limit is `UNKNOWN`.
   - **Actuator capability is exact:** the interval of rated torque × gear ratio × efficiency. It is `SATISFIED` only when its low bound covers the required peak torque, `VIOLATED` only when its high bound falls short, and `UNKNOWN` between.
   - **A model or trajectory input with declared uncertainty** (a link length or mass, the payload, an angle, the duration) makes every dynamic and tracking verdict drawn from the nominal simulation `UNKNOWN`, naming the input.

## Evidence

Core tests cover the following:

- all 13 drops are recorded, with 0.3 m marked inexact and 0.25 m exact;
- an uncertain link mass makes both joints' capability and tracking `UNKNOWN`, while the exact joint-limit verdict stays `SATISFIED`;
- an uncertain actuator is `SATISFIED`, `UNKNOWN` and `VIOLATED` for an interval above, around and below the requirement;
- an uncertain angle overlapping a limit is `UNKNOWN`, and one wholly beyond it is `VIOLATED`;
- the interval arithmetic is exact and sign-aware.

Six mutants were each caught by a test:

- dropped bounds not recorded;
- a nominal simulation deciding uncertain inputs;
- the actuator compared at its nominal value;
- joint limits compared at nominal values;
- a product that is not sign-aware;
- actuator uncertainty treated as model uncertainty.

## What stays open

`DEBT-UNCERTAINTY` stays open:

- there is no distribution representation (interval only);
- a simulation cannot propagate input bounds (it answers `UNKNOWN` instead of bounding);
- visual `f64` and graph `f32` carriers remain.

`DEBT-SIMULATION` stays open: bounded simulation (interval or sampled runs) would turn those `UNKNOWN` verdicts into decisions.
