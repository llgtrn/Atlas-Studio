---
id: atlas.decision.0014.cross-domain-drivetrain-power
type: decision
status: accepted
canonical: true
---
# ADR 0014 — Physical milestone 2: cross-domain drivetrain and power

## Context

Milestone 2 of the physical-engineering directive is cross-domain engineering. Atlas must detect contradictions that no single domain can see. For example, a mechanically valid arm may be one whose motors, currents and power rail cannot actually hold it.

## Decision

`core::physical::analyze_drivetrains` extends ADR 0012's report. The ADL declares:

- `Motor` entities with `joint`, `gear_ratio`, `efficiency`, `torque_constant` (N·m/A), `rated_torque`, `max_current`, an optional `winding_resistance`, and `rail`;
- `Rail` entities with `voltage` and `max_current`.

### The chain

Every step is exact and dimension-checked. Each derived value lists the declared parameters or derived records it came from in `inputs`, and every input resolves.

1. **Static torque per joint.** Computed from the links beyond that joint plus the payload: `static_joint_torques()`, which generalizes the shoulder-only torque.
2. **Motor torque** = joint torque ÷ (gear ratio × efficiency). Mechanics becomes drivetrain.
3. **Holding current** = motor torque ÷ torque constant. Drivetrain becomes electrical.
4. **Holding heat** = I² × R. Electrical becomes thermal.
5. **Rail totals.** Each rail's total current is the sum of the holding currents of its motors, and its electrical power is V × I.

### Checks

- Each motor: `rated_torque ≥ motor torque` and `max_current ≥ holding current`.
- Each rail: `max_current ≥ total current`.

### Parameter validation

- Dimensionless parameters (`gear_ratio`, `efficiency`) are bare decimals and must be positive.
- `efficiency > 1` is VIOLATED (energy conservation).
- A dimensionally wrong parameter yields a finding and no derived values for that motor.

## Evidence

- **Exact values on the fixture.** Shoulder: 4.535575625 N·m → 0.113389390625 N·m at the motor → 2.2677878125 A. Elbow: 2549729/1920000 A. Rail: 3.5958 A (43.149 W at 12 V). All requirements are SATISFIED.
- **Independent check.** Python `fractions` reproduces every value exactly.
- **Cross-domain contradiction.** With a 1.5 kg payload, `required_reach` is still SATISFIED but the 5 A rail is VIOLATED at 8.34 A. The mechanics alone would not reveal this.
- **Production path.** `atlas-systemizer physical` on the ADL fixture prints the whole chain with provenance.
- **Mutation testing.** Five mutants killed:
  - efficiency ignored;
  - every joint summing all links;
  - efficiency above 1 allowed;
  - capacity comparison inverted;
  - a motor dropped from the rail sum.

  One mutant is equivalent (recorded): the current-dimension filter cannot fire while `torque_constant` is typed.

## Limits

- Static holding only, with all joints at worst case simultaneously. There is no acceleration, speed, back-EMF or voltage headroom.
- Efficiency is taken as constant.
- Evidence level remains `SEMANTIC_MODEL`.
- **Next:** dynamics (inertia along a trajectory), then a simulated plant with run identity.
