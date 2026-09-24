---
id: atlas.contract.physical-engineering
type: contract
status: active
canonical: true
---
# Physical Engineering Contract

Physical Engineering is the Atlas capability family that carries intent through a formal engineering model toward verified physical systems: mechanics, electronics, firmware, control and manufacturing, connected by one semantic and provenance graph.

Machine-readable lane state lives in `.atlas/roadmap/FRONTIER.toml` (family `PHYSICAL_ENGINEERING`).

## Evidence levels (MUST be claimed honestly)

Evidence progresses through these levels, in order:

`SEMANTIC_MODEL` → `SIMULATED` → `SIL_VERIFIED` → `HIL_VERIFIED` → `BOUNDED_PHYSICAL_TESTED` → `FIELD_VALIDATED`

Every physical report states its level, and a report never claims a level above what it demonstrated.

- A simulation is not reality.
- A CAD model is not a manufactured part.
- A controller that works in simulation is not verified hardware control.
- HIL is never simulated and then labelled HIL.
- Certification and external validation are separate evidence domains. They are never implied by tests or simulation.

## Units first (MUST)

- **Quantities are typed.** Every physical parameter is a `core::quantity::Quantity`: exact SI value plus dimension (ADR 0010).
- **Dimensional validation comes first.** It precedes every constraint, optimization, probabilistic or simulation step.
  - A dimensionally wrong parameter is a finding, and the model is not assembled.
  - A unit Atlas cannot represent exactly is `UNKNOWN`, never approximated.
- **Verdicts are three-valued.** Constraints over physical parameters use `SATISFIED`/`VIOLATED`/`UNKNOWN` (ADR 0007). `UNKNOWN` is never merged into success or failure.

## Epistemic discipline (MUST)

- Declared parameters are `DECLARED`.
- Algebraic consequences are `DERIVED`, with their rule and modelling assumptions stated (e.g. "uniform-density links", "static, horizontal worst case", "standard gravity 9.80665 m/s²").
- `SIMULATED` was added by an ADR 0002 amendment on 2026-09-24. It marks the result of executing a model, and every simulation run carries a BLAKE3 run identity over its exact configuration.
- `MEASURED` and `CALIBRATED` still require an amendment before any record may carry them. A sensor reading is not automatically ground truth.

## Physical execution boundary (MUST)

- Atlas contains no code path that actuates hardware. A candidate physical command and an authorized physical execution are different things, separated by an explicit policy/capability gate.
- Synthesized control logic is never deployed to real machinery automatically.
- New mechanisms progress through these stages; none is skipped:

  `STATIC CHECK → DIMENSION CHECK → HARD CONSTRAINT CHECK → MODEL CHECK → SIMULATION → SCENARIO VARIATION → FAULT INJECTION → SIL → HIL → bounded physical test`

- There are no uncontrolled physical experiments:
  - no live vehicles;
  - no energizing high-power systems;
  - no bypassed interlocks or emergency stops;
  - no unknown toolpaths;
  - no experimental flight firmware.

## Donors

- **Mechanism slices only.** A donor is admitted only as a mechanism slice with pinned commit and durable license.
- **Copyleft donors are principles-only:** KiCad, FreeCAD, OCCT, OpenFOAM, LinuxCNC, OpenModelica.
- **Mature kernels and solvers may be EXTERNAL_BOUNDARY.** When justified (e.g. BREP kernels, CFD solvers, certified toolchains), they may remain external; extinction is never forced.

## Implementation status

- **G42 (ADR 0010):** `core::quantity`, and dimension-aware ADL quantity constraints.
- **G44 (ADR 0012):** physical milestone 1, at level `SEMANTIC_MODEL`. `core::physical::PlanarArm` is assembled from ADL `Arm`/`Link`/`Joint` entities. It provides:
  - exact reach, inner radius and worst-case static shoulder torque;
  - verdicts on `required_reach` and `shoulder_torque_limit`;
  - planar forward kinematics verified against closed form.

  Exposed as `atlas-systemizer physical --root R [--out O]`.
- **G51 (ADR 0019):** milestone 3, at level `SIMULATED`. It covers:
  - 2R Lagrangian dynamics;
  - declared minimum-jerk `Trajectory` entities;
  - computed-torque RK4 simulation saturated at actuator capability;
  - a divergence guard;
  - joint-limit, actuator-capability and tracking verdicts.
