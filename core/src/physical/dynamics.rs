//! Physical milestone 3: 2R planar arm dynamics, a declared trajectory, and a deterministic
//! simulation with run identity (ADR 0019). Digital only; results are `SIMULATED`.
//!
//! Model: two uniform rods (centre of mass at mid-length, inertia m*l^2/12 about it) and a point
//! payload at the tip, joint 1 angle absolute from the horizontal, joint 2 relative, gravity along
//! -y. Closed-form Lagrangian dynamics `M(q) q'' + c(q, q') + g(q) = tau`. The gravity term at the
//! horizontal pose equals milestone 1's exact static torques -- a cross-check between the
//! independent derivations.

use super::{PlanarArm, standard_gravity};
use crate::quantity::FloatDrop;
use crate::{EpistemicStatus, IntegrityDigest};
use serde::{Deserialize, Serialize};

/// Mass properties in SI `f64` (dynamics is transcendental; exact values stay in `PlanarArm`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ArmDynamics {
    pub l1: f64,
    pub m1: f64,
    pub l2: f64,
    pub m2: f64,
    pub payload: f64,
    pub gravity: f64,
}

impl ArmDynamics {
    /// From a validated 2-link `PlanarArm`; `None` for other chain lengths.
    /// The arm's parameters in `f64` for integration; every conversion is recorded in `drops`
    /// (G131) with its exactness and any declared uncertainty it leaves behind.
    pub fn from_arm(arm: &PlanarArm, drops: &mut Vec<FloatDrop>) -> Option<Self> {
        let [first, second] = arm.links.as_slice() else {
            return None;
        };
        Some(Self {
            l1: first
                .length
                .drop_to_f64(format!("{}.length", first.name), drops),
            m1: first
                .mass
                .drop_to_f64(format!("{}.mass", first.name), drops),
            l2: second
                .length
                .drop_to_f64(format!("{}.length", second.name), drops),
            m2: second
                .mass
                .drop_to_f64(format!("{}.mass", second.name), drops),
            payload: arm
                .payload
                .drop_to_f64(format!("{}.payload", arm.name), drops),
            gravity: standard_gravity().drop_to_f64("standard_gravity", drops),
        })
    }

    /// Link 2 with the payload folded in: (mass, centre-of-mass distance from joint 2, inertia
    /// about that centre of mass).
    fn distal(&self) -> (f64, f64, f64) {
        let mass = self.m2 + self.payload;
        let com = (self.m2 * self.l2 / 2.0 + self.payload * self.l2) / mass;
        let inertia = self.m2 * self.l2 * self.l2 / 12.0
            + self.m2 * (self.l2 / 2.0 - com).powi(2)
            + self.payload * (self.l2 - com).powi(2);
        (mass, com, inertia)
    }

    pub fn mass_matrix(&self, q2: f64) -> [[f64; 2]; 2] {
        let (m2, c2, i2) = self.distal();
        let (lc1, i1) = (self.l1 / 2.0, self.m1 * self.l1 * self.l1 / 12.0);
        let m11 = self.m1 * lc1 * lc1
            + i1
            + m2 * (self.l1 * self.l1 + c2 * c2 + 2.0 * self.l1 * c2 * q2.cos())
            + i2;
        let m12 = m2 * (c2 * c2 + self.l1 * c2 * q2.cos()) + i2;
        let m22 = m2 * c2 * c2 + i2;
        [[m11, m12], [m12, m22]]
    }

    /// Coriolis/centrifugal torques.
    pub fn coriolis(&self, q2: f64, dq: [f64; 2]) -> [f64; 2] {
        let (m2, c2, _) = self.distal();
        let h = -m2 * self.l1 * c2 * q2.sin();
        [h * dq[1] * (2.0 * dq[0] + dq[1]), -h * dq[0] * dq[0]]
    }

    /// Gravity torques.
    pub fn gravity_torque(&self, q: [f64; 2]) -> [f64; 2] {
        let (m2, c2, _) = self.distal();
        let g = self.gravity;
        let outer = m2 * c2 * g * (q[0] + q[1]).cos();
        [
            (self.m1 * self.l1 / 2.0 + m2 * self.l1) * g * q[0].cos() + outer,
            outer,
        ]
    }

    /// Inverse dynamics: the torques that produce acceleration `ddq` at state `(q, dq)`.
    pub fn inverse(&self, q: [f64; 2], dq: [f64; 2], ddq: [f64; 2]) -> [f64; 2] {
        let m = self.mass_matrix(q[1]);
        let c = self.coriolis(q[1], dq);
        let g = self.gravity_torque(q);
        [
            m[0][0] * ddq[0] + m[0][1] * ddq[1] + c[0] + g[0],
            m[1][0] * ddq[0] + m[1][1] * ddq[1] + c[1] + g[1],
        ]
    }

    /// Forward dynamics: acceleration under torques `tau`.
    pub fn forward(&self, q: [f64; 2], dq: [f64; 2], tau: [f64; 2]) -> [f64; 2] {
        let m = self.mass_matrix(q[1]);
        let c = self.coriolis(q[1], dq);
        let g = self.gravity_torque(q);
        let rhs = [tau[0] - c[0] - g[0], tau[1] - c[1] - g[1]];
        let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];
        [
            (m[1][1] * rhs[0] - m[0][1] * rhs[1]) / det,
            (m[0][0] * rhs[1] - m[1][0] * rhs[0]) / det,
        ]
    }

    /// Kinetic + potential energy (zero potential at the joint-1 axis height).
    pub fn energy(&self, q: [f64; 2], dq: [f64; 2]) -> f64 {
        let m = self.mass_matrix(q[1]);
        let kinetic = 0.5
            * (m[0][0] * dq[0] * dq[0] + 2.0 * m[0][1] * dq[0] * dq[1] + m[1][1] * dq[1] * dq[1]);
        let (m2, c2, _) = self.distal();
        let potential = self.gravity
            * (self.m1 * self.l1 / 2.0 * q[0].sin()
                + m2 * (self.l1 * q[0].sin() + c2 * (q[0] + q[1]).sin()));
        kinetic + potential
    }
}

/// Minimum-jerk (quintic) point-to-point joint move from rest to rest.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct JointMove {
    pub from: [f64; 2],
    pub to: [f64; 2],
    pub duration_s: f64,
}

impl JointMove {
    /// Desired (q, dq, ddq) at time `t` (clamped to [0, duration]).
    pub fn at(&self, t: f64) -> ([f64; 2], [f64; 2], [f64; 2]) {
        let t_total = self.duration_s;
        let s = (t / t_total).clamp(0.0, 1.0);
        let shape = 10.0 * s.powi(3) - 15.0 * s.powi(4) + 6.0 * s.powi(5);
        let rate = (30.0 * s.powi(2) - 60.0 * s.powi(3) + 30.0 * s.powi(4)) / t_total;
        let accel = (60.0 * s - 180.0 * s.powi(2) + 120.0 * s.powi(3)) / (t_total * t_total);
        let delta = [self.to[0] - self.from[0], self.to[1] - self.from[1]];
        (
            [
                self.from[0] + delta[0] * shape,
                self.from[1] + delta[1] * shape,
            ],
            [delta[0] * rate, delta[1] * rate],
            [delta[0] * accel, delta[1] * accel],
        )
    }
}

/// Everything that determines a run; its canonical digest is the run identity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulationConfig {
    pub dynamics: ArmDynamics,
    pub trajectory: JointMove,
    /// Joint torque limits (actuator capability after gearing), N*m.
    pub torque_limits: [f64; 2],
    pub kp: [f64; 2],
    pub kd: [f64; 2],
    pub step_s: f64,
    /// Settling time simulated after the move ends.
    pub hold_s: f64,
    pub integrator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulationRun {
    pub run_identity: IntegrityDigest,
    pub status: EpistemicStatus,
    pub steps: usize,
    /// Largest |commanded torque| per joint before saturation (controller-dependent).
    pub peak_demand: [f64; 2],
    /// Largest |torque| the desired trajectory itself requires (inverse dynamics along it) --
    /// controller-independent, the basis for actuator-capability verdicts.
    pub ideal_peak: [f64; 2],
    /// Steps at which a joint's command was clipped to its limit.
    pub saturated_steps: [usize; 2],
    pub max_tracking_error_rad: f64,
    pub final_error_rad: f64,
    /// The integration left the finite numbers: every result of this run is meaningless and
    /// must be reported `UNKNOWN`, never as a value.
    pub diverged: bool,
}

impl SimulationConfig {
    /// Canonical, exact serialization: every `f64` as its IEEE-754 bit pattern, fields in a fixed
    /// order under a versioned tag -- no float-formatting ambiguity.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let d = &self.dynamics;
        let t = &self.trajectory;
        let fields: [(&str, f64); 18] = [
            ("l1", d.l1),
            ("m1", d.m1),
            ("l2", d.l2),
            ("m2", d.m2),
            ("payload", d.payload),
            ("gravity", d.gravity),
            ("from0", t.from[0]),
            ("from1", t.from[1]),
            ("to0", t.to[0]),
            ("to1", t.to[1]),
            ("duration", t.duration_s),
            ("limit0", self.torque_limits[0]),
            ("limit1", self.torque_limits[1]),
            ("kp0", self.kp[0]),
            ("kp1", self.kp[1]),
            ("kd0", self.kd[0]),
            ("kd1", self.kd[1]),
            ("step", self.step_s),
        ];
        let mut text = String::from("atlas.simulation-config.v1");
        for (name, value) in fields {
            text.push_str(&format!("|{name}={:016x}", value.to_bits()));
        }
        text.push_str(&format!(
            "|hold={:016x}|integrator={}",
            self.hold_s.to_bits(),
            self.integrator
        ));
        text.into_bytes()
    }

    /// Run identity: BLAKE3 over `canonical_bytes`.
    pub fn run_identity(&self) -> IntegrityDigest {
        IntegrityDigest::of_bytes(&self.canonical_bytes())
    }
}

fn rk4(
    dynamics: &ArmDynamics,
    state: ([f64; 2], [f64; 2]),
    tau: [f64; 2],
    h: f64,
) -> ([f64; 2], [f64; 2]) {
    let deriv = |q: [f64; 2], dq: [f64; 2]| (dq, dynamics.forward(q, dq, tau));
    let add = |a: [f64; 2], b: [f64; 2], k: f64| [a[0] + k * b[0], a[1] + k * b[1]];
    let (q, dq) = state;
    let (k1q, k1v) = deriv(q, dq);
    let (k2q, k2v) = deriv(add(q, k1q, h / 2.0), add(dq, k1v, h / 2.0));
    let (k3q, k3v) = deriv(add(q, k2q, h / 2.0), add(dq, k2v, h / 2.0));
    let (k4q, k4v) = deriv(add(q, k3q, h), add(dq, k3v, h));
    let combine = |a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2], x: [f64; 2]| {
        [
            x[0] + h / 6.0 * (a[0] + 2.0 * b[0] + 2.0 * c[0] + d[0]),
            x[1] + h / 6.0 * (a[1] + 2.0 * b[1] + 2.0 * c[1] + d[1]),
        ]
    };
    (
        combine(k1q, k2q, k3q, k4q, q),
        combine(k1v, k2v, k3v, k4v, dq),
    )
}

/// Simulates the move under computed-torque control -- tau = M(q)(q''_d + Kp e + Kd e') + c + g,
/// whose error dynamics e'' + Kd e' + Kp e = 0 are independent of the arm's inertia -- saturated
/// at the torque limits, integrated with RK4 at a fixed step. Deterministic: identical
/// configurations give identical runs. (A first version applied Kp/Kd directly as torques; its
/// effective elbow inertia of ~0.0035 kg*m^2 made the discrete loop unstable and the run diverged
/// -- found by comparing against the ideal inverse-dynamics demand, ADR 0019.)
pub fn simulate(config: &SimulationConfig) -> SimulationRun {
    let identity = config.run_identity();
    let d = &config.dynamics;
    let steps = ((config.trajectory.duration_s + config.hold_s) / config.step_s).round() as usize;
    let (mut q, mut dq) = (config.trajectory.from, [0.0; 2]);
    let mut run = SimulationRun {
        run_identity: identity,
        status: EpistemicStatus::Simulated,
        steps,
        peak_demand: [0.0; 2],
        ideal_peak: [0.0; 2],
        saturated_steps: [0; 2],
        max_tracking_error_rad: 0.0,
        final_error_rad: 0.0,
        diverged: false,
    };
    for step in 0..steps {
        let t = step as f64 * config.step_s;
        let (qd, dqd, ddqd) = config.trajectory.at(t);
        let ideal = d.inverse(qd, dqd, ddqd);
        for (peak, torque) in run.ideal_peak.iter_mut().zip(ideal) {
            *peak = peak.max(torque.abs());
        }
        let reference = [
            ddqd[0] + config.kp[0] * (qd[0] - q[0]) + config.kd[0] * (dqd[0] - dq[0]),
            ddqd[1] + config.kp[1] * (qd[1] - q[1]) + config.kd[1] * (dqd[1] - dq[1]),
        ];
        let commanded = d.inverse(q, dq, reference);
        let mut tau = [0.0; 2];
        for j in 0..2 {
            let command = commanded[j];
            run.peak_demand[j] = run.peak_demand[j].max(command.abs());
            let limit = config.torque_limits[j];
            if command.abs() > limit {
                run.saturated_steps[j] += 1;
            }
            tau[j] = command.clamp(-limit, limit);
        }
        (q, dq) = rk4(d, (q, dq), tau, config.step_s);
        if !(q.iter().chain(dq.iter()).all(|v| v.is_finite())) {
            run.diverged = true;
            return run;
        }
        let error = (qd[0] - q[0]).abs().max((qd[1] - q[1]).abs());
        run.max_tracking_error_rad = run.max_tracking_error_rad.max(error);
    }
    let end = config.trajectory.to;
    run.final_error_rad = (end[0] - q[0]).abs().max((end[1] - q[1]).abs());
    run
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arm() -> ArmDynamics {
        ArmDynamics {
            l1: 0.3,
            m1: 0.4,
            l2: 0.25,
            m2: 0.3,
            payload: 0.5,
            gravity: 9.80665,
        }
    }

    #[test]
    fn gravity_at_horizontal_equals_the_exact_static_torques() {
        // Milestone 1/2's exact values: 4.535575625 and 1.593580625 N*m.
        let g = arm().gravity_torque([0.0, 0.0]);
        assert!((g[0] - 4.535575625).abs() < 1e-12, "{}", g[0]);
        assert!((g[1] - 1.593580625).abs() < 1e-12, "{}", g[1]);
        let hanging = arm().gravity_torque([-std::f64::consts::FRAC_PI_2, 0.0]);
        assert!(
            hanging[0].abs() < 1e-12 && hanging[1].abs() < 1e-12,
            "no torque hanging down"
        );
    }

    #[test]
    fn mass_matrix_is_symmetric_positive_definite_and_inverse_forward_round_trip() {
        let d = arm();
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % 20_000) as f64 / 1000.0 - 10.0
        };
        for _ in 0..500 {
            let (q, dq, ddq) = ([next(), next()], [next(), next()], [next(), next()]);
            let m = d.mass_matrix(q[1]);
            assert_eq!(m[0][1], m[1][0]);
            assert!(m[0][0] > 0.0 && m[0][0] * m[1][1] - m[0][1] * m[1][0] > 0.0);
            let tau = d.inverse(q, dq, ddq);
            let back = d.forward(q, dq, tau);
            assert!((back[0] - ddq[0]).abs() < 1e-9 && (back[1] - ddq[1]).abs() < 1e-9);
        }
    }

    #[test]
    fn mass_matrix_matches_kinetic_energy_of_discretized_rods() {
        // Independent oracle for M(q): split each uniform rod into many point masses, compute
        // every element's velocity from planar kinematics, and sum 1/2 m v^2.
        let d = arm();
        let segments = 2000;
        for (q, dq) in [
            ([0.3, 0.8], [1.5, -2.0]),
            ([-1.0, 2.2], [0.7, 3.1]),
            ([2.0, -0.4], [-2.5, 0.9]),
        ] {
            let (q, dq): ([f64; 2], [f64; 2]) = (q, dq);
            let point_velocity = |r1: f64, r2: f64| {
                // Position = r1 * u(q1) + r2 * u(q1 + q2).
                let (a, b) = (q[0], q[0] + q[1]);
                let vx = -r1 * a.sin() * dq[0] - r2 * b.sin() * (dq[0] + dq[1]);
                let vy = r1 * a.cos() * dq[0] + r2 * b.cos() * (dq[0] + dq[1]);
                vx * vx + vy * vy
            };
            let mut kinetic = 0.0;
            for k in 0..segments {
                let x = (f64::from(k) + 0.5) / f64::from(segments);
                kinetic += 0.5 * d.m1 / f64::from(segments) * point_velocity(x * d.l1, 0.0);
                kinetic += 0.5 * d.m2 / f64::from(segments) * point_velocity(d.l1, x * d.l2);
            }
            kinetic += 0.5 * d.payload * point_velocity(d.l1, d.l2);
            let m = d.mass_matrix(q[1]);
            let from_matrix = 0.5
                * (m[0][0] * dq[0] * dq[0]
                    + 2.0 * m[0][1] * dq[0] * dq[1]
                    + m[1][1] * dq[1] * dq[1]);
            assert!(
                (kinetic - from_matrix).abs() < 1e-6 * kinetic,
                "discretized {kinetic} vs M(q) {from_matrix}"
            );
        }
    }

    #[test]
    fn an_unactuated_arm_conserves_energy_under_rk4() {
        // Independent physics check of M, c and g together: with tau = 0 the Lagrangian system
        // is conservative, so total energy may drift only by integration error.
        let d = arm();
        let (mut q, mut dq) = ([0.3, 0.8], [0.0, 0.0]);
        let e0 = d.energy(q, dq);
        for _ in 0..2000 {
            (q, dq) = rk4(&d, (q, dq), [0.0, 0.0], 5e-4);
        }
        let drift = (d.energy(q, dq) - e0).abs() / e0.abs();
        assert!(drift < 1e-6, "relative energy drift {drift}");
    }

    fn slow_move() -> SimulationConfig {
        SimulationConfig {
            dynamics: arm(),
            trajectory: JointMove {
                from: [0.0, 0.0],
                to: [1.2, -0.8],
                duration_s: 0.8,
            },
            torque_limits: [1e9, 1e9],
            kp: [900.0, 900.0],
            kd: [60.0, 60.0],
            step_s: 1e-3,
            hold_s: 0.3,
            integrator: "rk4".into(),
        }
    }

    #[test]
    fn unsaturated_tracking_demand_matches_ideal_inverse_dynamics() {
        // Oracle: the torque the ideal trajectory needs, from inverse dynamics alone.
        let config = slow_move();
        let mut ideal = [0.0_f64; 2];
        for k in 0..=800 {
            let (q, dq, ddq) = config.trajectory.at(f64::from(k) * 1e-3);
            let tau = config.dynamics.inverse(q, dq, ddq);
            ideal = [ideal[0].max(tau[0].abs()), ideal[1].max(tau[1].abs())];
        }
        let run = simulate(&config);
        assert!(!run.diverged);
        // The controller is sampled once per step (zero-order hold), so tracking lags by about
        // v_max * dt = 2.8 rad/s * 1 ms; the error must be of that order and must shrink with dt.
        assert!(
            run.max_tracking_error_rad < 5e-3,
            "{}",
            run.max_tracking_error_rad
        );
        let mut fine = config.clone();
        fine.step_s = 1e-4;
        let fine_run = simulate(&fine);
        assert!(
            fine_run.max_tracking_error_rad < run.max_tracking_error_rad / 5.0,
            "error must converge with the step: {} (1 ms) vs {} (0.1 ms)",
            run.max_tracking_error_rad,
            fine_run.max_tracking_error_rad
        );
        assert!(run.final_error_rad < 1e-4, "{}", run.final_error_rad);
        for (joint, (simulated, required)) in run.peak_demand.iter().zip(ideal).enumerate() {
            assert!(
                (simulated - required).abs() < 0.05 * required,
                "joint {joint}: simulated {simulated} vs ideal {required}"
            );
        }
    }

    #[test]
    fn a_diverging_integration_is_flagged_not_reported_as_values() {
        let mut config = slow_move();
        config.step_s = 0.2; // far beyond the loop's stable step
        config.kp = [9e6, 9e6];
        config.kd = [6e3, 6e3];
        assert!(simulate(&config).diverged);
    }

    #[test]
    fn minimum_jerk_move_starts_and_ends_at_rest() {
        let m = JointMove {
            from: [0.0, 0.0],
            to: [1.0, -0.5],
            duration_s: 0.8,
        };
        let (q0, dq0, ddq0) = m.at(0.0);
        let (q1, dq1, ddq1) = m.at(0.8);
        assert_eq!(q0, [0.0, 0.0]);
        assert!((q1[0] - 1.0).abs() < 1e-12 && (q1[1] + 0.5).abs() < 1e-12);
        for v in [dq0, ddq0, dq1, ddq1] {
            assert!(v[0].abs() < 1e-9 && v[1].abs() < 1e-9);
        }
    }
}
