---
id: atlas.decision.0019.arm-dynamics-and-simulation
type: decision
status: accepted
canonical: true
---
# ADR 0019 — Physical milestone 3: arm dynamics, declared trajectories, simulation with run identity

## Context

The physical-engineering directive's next milestones need three things:
- a kinematic and dynamic model;
- a trajectory;
- a simulation that produces verification evidence.

That evidence has to be honestly labelled: a simulation is not reality.

Milestones 1 and 2 (ADR 0012, ADR 0014) are static. A move that is slow enough and one that is too fast look identical to them.

## Decision

1. **Epistemic amendment.** ADR 0002 gains `SIMULATED`: a result of executing a model. It is never promoted to observed, measured or verified without a separate evidence path.
2. **`core::physical::dynamics`.**
   - **Model.** A closed-form Lagrangian 2R planar model built from uniform rods plus a point payload: M(q), Coriolis terms c(q, q̇), gravity g(q), inverse and forward dynamics, and total energy.
   - **Trajectory.** A minimum-jerk joint move from rest to rest.
   - **Simulator.** Computed-torque control, τ = M(q)(q̈d + Kp·e + Kd·ė) + c + g, with ω = 30 rad/s and critical damping. Torque is saturated at each joint's actuator capability (rated torque × gear ratio × efficiency, from the ADR 0014 motors). Integration is RK4 at 1 ms.
   - **Divergence guard.** A run that goes non-finite yields `UNKNOWN`, never values.
   - **Run identity.** Every run is identified by BLAKE3 over a canonical, exact encoding of its full configuration: each f64 by its bit pattern, plus the model, trajectory, limits, gains, step and integrator.
3. **Declared trajectories.** `Trajectory` entities in ADL declare joint endpoints (rad), duration (s) and tracking tolerance (rad), all dimension-checked. Three verdicts come from them:
   - **Within joint limits:** checked at the endpoints, which suffice because a minimum-jerk move is monotone per joint.
   - **Torque within actuator capability:** decided on the `DERIVED`, controller-independent peak torque that inverse dynamics says the trajectory requires. The `SIMULATED` controller peak and the saturated steps are reported alongside.
   - **Tracking error ≤ tolerance:** decided on `SIMULATED` evidence.

   When any simulation runs, the report's evidence level becomes `SIMULATED`.

## Evidence

The model is checked against independent oracles:
- Gravity at the horizontal pose equals milestone 1/2's *exact* static torques to 1e-12.
- Kinetic energy of the rods, discretized into 2000 point masses, equals ½q̇ᵀM(q)q̇ to 1e-6.
- An unactuated arm conserves energy under RK4, with relative drift below 1e-6.
- Inverse and forward dynamics round-trip over 500 random states.
- The simulated unsaturated demand matches ideal inverse dynamics within 5%.
- Tracking error is about v_max·dt (2.9 mrad at 1 ms) and shrinks by more than 5× at 0.1 ms.

The fixture (`atlas-systemizer physical`) exercises the two cases this milestone exists to separate:

| Move | Shoulder torque required | Available | Tracking error | Verdict |
|---|---|---|---|---|
| 1.2 rad in 0.8 s | 6.27 N·m | 12 N·m | 2.9 mrad | every requirement SATISFIED |
| same move in 0.25 s | 22.38 N·m | 12 N·m | 1.24 rad under saturation | VIOLATED |

The 0.25 s move passes every static check.

Two honest findings along the way:
- **Controller.** The first controller applied Kp and Kd directly as torques. Because of the elbow's small effective inertia, the discrete loop diverged (to about 1e280). The divergence was found by comparing against ideal inverse dynamics, and it was fixed by computed-torque control.
- **Oracle gap.** The mutant that drops the rods' own inertia first survived: it is a self-consistent model, so energy conservation cannot see it. The discretized kinetic-energy oracle now kills it.

Mutation testing killed 7 mutants: Coriolis sign, gravity sin/cos, rod inertia dropped, missing M scaling, divergence unguarded, RK4 degraded to Euler, and limit without efficiency.

## Limits

- **Model scope:** a 2-link planar arm; rigid bodies; no joint friction, motor dynamics, current limits or voltage headroom.
- **Evidence level:** `SIMULATED`. This is not SIL: no Atlas-produced controller binary runs against the plant.
- **Next steps:**
  - fault injection (sensor bias/dropout, actuator saturation);
  - SIL, once a controller binary exists;
  - the safety state machine.
