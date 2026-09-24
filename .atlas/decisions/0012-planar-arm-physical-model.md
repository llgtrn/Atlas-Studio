---
id: atlas.decision.0012.planar-arm-physical-model
type: decision
status: accepted
canonical: true
---
# ADR 0012 — Physical milestone 1: a declared planar arm

## Context

The physical-engineering directive's first bounded milestone is a digital-only 2-link arm:
- the ADL declares link lengths, payload and joint limits;
- Atlas turns that into a Physical IR, a kinematic model and constraint verdicts.

The directive asks that the schema grow from real consumers. This milestone is the first consumer, and ADR 0010 already provides exact, dimension-checked quantities.

## Decision

1. **`core::physical`**
   - **Assembly:** `assemble_arms` builds a `PlanarArm` from ADL entities:
     - `Arm`: `payload`, plus optional `required_reach` and `shoulder_torque_limit`;
     - `Link`: `arm`, `index`, `length`, `mass`;
     - `Joint`: `arm`, `index`, `lower`, `upper`.
   - **Validation before any model exists:**
     - every parameter is a `Quantity` with a checked dimension (length, mass or angle);
     - indices must run 1..n, with links and joints in equal number;
     - lengths must be positive and joint limits ordered.

     Any failure is a `VIOLATED` or `UNKNOWN` finding, and no partial model is trusted.
   - **Exact `DERIVED` quantities**, each with its assumptions stated:
     - reach, Σl;
     - inner radius, |l1 − l2|;
     - worst-case static shoulder torque: Σ m·g·r, fully extended horizontally, uniform links, point payload, with standard gravity 9.80665 m/s² (exact by definition).
   - **Requirements** are checked against the derived quantities: `required_reach` ≤ reach, and static torque ≤ `shoulder_torque_limit`.
   - **Forward kinematics** is computed in `f64`, because the trigonometry is transcendental.
2. **Production path.** `runtime::physical::analyze_root` compiles the root's declared ADL and runs the analysis. `atlas-systemizer physical --root R [--out O]` exits non-zero (`PHYSICAL_REQUIREMENTS_NOT_SATISFIED`) on any non-SATISFIED finding or requirement.
3. **Contract.** `.atlas/contracts/PHYSICAL-ENGINEERING.md` fixes the evidence levels, units-first rule, epistemic states and the hard physical-execution boundary.

## Evidence

- **Fixture** (`runtime/tests/fixtures/physical/two-link-arm`):
  - 300 mm + 0.25 m gives reach **exactly 550 mm** and inner radius 50 mm;
  - torque is g·(0.4·0.15 + 0.3·0.425 + 0.5·0.55) = **4.535575625 N·m** exactly;
  - requirements (≥ 500 mm, ≤ 5 N·m) are SATISFIED, and the ADL invariant `l.mass <= 0.5 kg` is SATISFIED;
  - a 3 N·m limit is VIOLATED and the command exits 2.
- **Kinematics:** forward kinematics matches the closed form to 1e-12 over 1000 random in-limit poses and never exceeds reach.
- **Mutation testing:** 7 mutants killed — the centre of mass not halved, payload ignored, dimensions unchecked, joint limits unordered, boundary equality failing a minimum, non-cumulative angles, and gravity rounded to 9.81.

## Limits (recorded)

- Only the planar serial arm with revolute joints is modelled.
- The inner radius ignores elbow limits.
- Torque is static only: no dynamics, inertia or acceleration.
- There is no simulation, so the evidence level is `SEMANTIC_MODEL`.
- **Next:**
  - trajectory and simulation with run identity (MuJoCo's model/data split is the reference);
  - milestone 2's cross-domain chain: motor, gear ratio, current, power rail.
