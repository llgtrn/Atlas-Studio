//! Physical Engineering: the first Physical Semantic IR slice -- a planar serial arm (ADR 0012).
//!
//! Grown from its first consumer (physical milestone 1: a 2-link arm), not designed as a
//! speculative schema. A `PlanarArm` is assembled from ADL-declared entities:
//!
//! ```text
//! entity Arm   A  { payload = 0.5 kg  required_reach = 450 mm  shoulder_torque_limit = 3 N*m }
//! entity Link  L1 { arm = A  index = 1  length = 300 mm  mass = 0.4 kg }
//! entity Joint J1 { arm = A  index = 1  lower = -1.5 rad  upper = 1.5 rad }
//! ```
//!
//! Every parameter is parsed as a `core::quantity::Quantity` and dimension-checked (a link length
//! in kilograms is a finding, not a value). Derived quantities are exact where the physics is
//! algebraic -- reach, inner workspace radius, worst-case static shoulder torque with the defined
//! standard gravity -- and `DERIVED` with their modelling assumptions stated. Forward kinematics
//! needs trigonometry, so it is evaluated in `f64` and verified against closed form. Nothing here
//! is simulated, measured or actuated: this is the SEMANTIC_MODEL evidence level.

pub mod dynamics;

use crate::{
    ConstraintVerdict, EpistemicStatus,
    language::adl::DeclaredNode,
    quantity::{Dimension, Quantity, QuantityKind, Rational},
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

const LENGTH: Dimension = Dimension {
    exponents: [1, 0, 0, 0, 0, 0, 0, 0],
};
const MASS: Dimension = Dimension {
    exponents: [0, 1, 0, 0, 0, 0, 0, 0],
};
const ANGLE: Dimension = Dimension {
    exponents: [0, 0, 0, 0, 0, 0, 0, 1],
};
const TORQUE: Dimension = Dimension {
    exponents: [2, 1, -2, 0, 0, 0, 0, 0],
};

/// Standard acceleration of gravity, exact by definition (CGPM 1901): 9.80665 m/s^2.
pub fn standard_gravity() -> Quantity {
    Quantity::parse("9.80665 m/s^2").expect("defined constant")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanarLink {
    pub name: String,
    pub length: Quantity,
    pub mass: Quantity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevoluteJoint {
    pub name: String,
    pub lower: Quantity,
    pub upper: Quantity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanarArm {
    pub name: String,
    pub payload: Quantity,
    /// `links[i]` hangs from `joints[i]`, base first.
    pub links: Vec<PlanarLink>,
    pub joints: Vec<RevoluteJoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhysicalFinding {
    pub subject: String,
    pub verdict: ConstraintVerdict,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DerivedQuantity {
    pub subject: String,
    pub name: String,
    pub value: Quantity,
    /// `value` rendered in the display unit, for humans; `value` is authoritative.
    pub display: String,
    pub status: EpistemicStatus,
    pub rule: String,
    pub assumptions: Vec<String>,
    /// The declared parameters and derived quantities this value was computed from
    /// (`Entity.attribute` or `Entity.derived_name`) -- the answer to "why does this value exist".
    #[serde(default)]
    pub inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequirementCheck {
    pub subject: String,
    pub requirement: String,
    pub verdict: ConstraintVerdict,
    pub detail: String,
}

/// The physical evidence ladder (`contracts/PHYSICAL-ENGINEERING.md`), typed (G134): each rung
/// is a kind of evidence, and `epistemic` says what it makes a claim in the one vocabulary.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PhysicalEvidenceLevel {
    #[default]
    SemanticModel,
    Simulated,
    SilVerified,
    HilVerified,
    BoundedPhysicalTested,
    FieldValidated,
}

impl PhysicalEvidenceLevel {
    /// A semantic model derives from declarations; a simulation is `SIMULATED`; software- and
    /// hardware-in-the-loop, bounded physical tests and field validation observe the artifact.
    pub fn epistemic(self) -> EpistemicStatus {
        match self {
            Self::SemanticModel => EpistemicStatus::Derived,
            Self::Simulated => EpistemicStatus::Simulated,
            Self::SilVerified
            | Self::HilVerified
            | Self::BoundedPhysicalTested
            | Self::FieldValidated => EpistemicStatus::Observed,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PhysicalReport {
    pub schema: String,
    pub evidence_level: PhysicalEvidenceLevel,
    pub arms: Vec<PlanarArm>,
    pub derived: Vec<DerivedQuantity>,
    pub requirements: Vec<RequirementCheck>,
    pub findings: Vec<PhysicalFinding>,
    /// Trajectory simulations (ADR 0019); their results are `SIMULATED`.
    #[serde(default)]
    pub simulations: Vec<SimulationRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulationRecord {
    pub trajectory: String,
    pub arm: String,
    pub config: dynamics::SimulationConfig,
    pub run: dynamics::SimulationRun,
    /// G131: every declared value the simulation took as `f64`, marked exact or not, with the
    /// declared uncertainty it did not carry.
    #[serde(default)]
    pub inputs: Vec<crate::quantity::FloatDrop>,
}

fn quantity_of(
    node: &DeclaredNode,
    attribute: &str,
    dimension: Dimension,
    findings: &mut Vec<PhysicalFinding>,
) -> Option<Quantity> {
    let finding = |verdict, message: String| PhysicalFinding {
        subject: node.name.clone(),
        verdict,
        message,
    };
    let Some(text) = node.attributes.get(attribute) else {
        findings.push(finding(
            ConstraintVerdict::Unknown,
            format!("`{attribute}` is not declared"),
        ));
        return None;
    };
    match Quantity::parse(text) {
        // G124: every TORQUE-dimension attribute of the physical model is a torque; a declared
        // energy (`0.3 J`) has the dimension but is refused as a torque.
        Ok(quantity) if quantity.dimension == dimension && dimension == TORQUE => {
            match quantity.with_kind(QuantityKind::Torque) {
                Ok(torque) => Some(torque),
                Err(error) => {
                    findings.push(finding(
                        ConstraintVerdict::Violated,
                        format!("`{attribute} = {text}` is not a torque: {error}"),
                    ));
                    None
                }
            }
        }
        Ok(quantity) if quantity.dimension == dimension => Some(quantity),
        Ok(quantity) => {
            findings.push(finding(
                ConstraintVerdict::Violated,
                format!(
                    "`{attribute} = {text}` has dimension {} but {} is required",
                    quantity.dimension, dimension
                ),
            ));
            None
        }
        Err(error) => {
            findings.push(finding(
                ConstraintVerdict::Unknown,
                format!("`{attribute} = {text}`: {error}"),
            ));
            None
        }
    }
}

fn index_of(node: &DeclaredNode) -> Option<u32> {
    node.attributes.get("index")?.parse().ok()
}

/// Assembles every `Arm` entity with its `Link`/`Joint` entities (matched by `arm = <name>`,
/// ordered by `index = 1..n`). A structurally or dimensionally invalid arm yields findings and no
/// model -- never a partially trusted one.
pub fn assemble_arms(nodes: &[DeclaredNode]) -> (Vec<PlanarArm>, Vec<PhysicalFinding>) {
    let mut findings = Vec::new();
    let mut arms = Vec::new();
    for arm in nodes.iter().filter(|node| node.node_kind == "Arm") {
        let before = findings.len();
        let member = |kind: &str| {
            let mut members: Vec<&DeclaredNode> = nodes
                .iter()
                .filter(|node| {
                    node.node_kind == kind
                        && node.attributes.get("arm").map(String::as_str) == Some(arm.name.as_str())
                })
                .collect();
            members.sort_by_key(|node| index_of(node));
            members
        };
        let links = member("Link");
        let joints = member("Joint");
        let expected: Vec<Option<u32>> = (1..=links.len() as u32).map(Some).collect();
        if links.is_empty()
            || links.len() != joints.len()
            || links.iter().map(|n| index_of(n)).collect::<Vec<_>>() != expected
            || joints.iter().map(|n| index_of(n)).collect::<Vec<_>>() != expected
        {
            findings.push(PhysicalFinding {
                subject: arm.name.clone(),
                verdict: ConstraintVerdict::Violated,
                message: format!(
                    "a serial arm needs links and joints indexed 1..n in equal number (links {}, joints {})",
                    links.len(),
                    joints.len()
                ),
            });
            continue;
        }
        let payload = quantity_of(arm, "payload", MASS, &mut findings);
        let links: Vec<PlanarLink> = links
            .iter()
            .filter_map(|node| {
                let length = quantity_of(node, "length", LENGTH, &mut findings);
                let mass = quantity_of(node, "mass", MASS, &mut findings);
                Some(PlanarLink {
                    name: node.name.clone(),
                    length: length?,
                    mass: mass?,
                })
            })
            .collect();
        let joints: Vec<RevoluteJoint> = joints
            .iter()
            .filter_map(|node| {
                let lower = quantity_of(node, "lower", ANGLE, &mut findings);
                let upper = quantity_of(node, "upper", ANGLE, &mut findings);
                let (lower, upper) = (lower?, upper?);
                if lower.checked_cmp(&upper) != Ok(Ordering::Less) {
                    findings.push(PhysicalFinding {
                        subject: node.name.clone(),
                        verdict: ConstraintVerdict::Violated,
                        message: "joint lower limit must be below its upper limit".into(),
                    });
                    return None;
                }
                Some(RevoluteJoint {
                    name: node.name.clone(),
                    lower,
                    upper,
                })
            })
            .collect();
        for link in &links {
            if link.length.si_value.checked_cmp(Rational::ZERO) != Some(Ordering::Greater) {
                findings.push(PhysicalFinding {
                    subject: link.name.clone(),
                    verdict: ConstraintVerdict::Violated,
                    message: "link length must be positive".into(),
                });
            }
        }
        if findings.len() == before
            && let Some(payload) = payload
        {
            arms.push(PlanarArm {
                name: arm.name.clone(),
                payload,
                links,
                joints,
            });
        }
    }
    (arms, findings)
}

fn sum(quantities: impl IntoIterator<Item = Quantity>, dimension: Dimension) -> Option<Quantity> {
    quantities
        .into_iter()
        .try_fold(Quantity::new(Rational::ZERO, dimension), |acc, q| {
            acc.checked_add(&q).ok()
        })
}

fn display(value: &Quantity, unit: &str) -> String {
    let (factor, _) = crate::quantity::parse_unit_expression(unit).expect("display unit");
    let scaled = value
        .si_value
        .checked_div(factor)
        .map(Rational::to_f64)
        .unwrap_or(f64::NAN);
    format!("{scaled} {unit}")
}

impl PlanarArm {
    /// Maximum reach: all links extended (sum of lengths). Exact.
    pub fn reach(&self) -> Option<Quantity> {
        sum(self.links.iter().map(|link| link.length), LENGTH)
    }

    /// Inner workspace radius of a 2-link arm, |l1 - l2|, when the elbow can fold fully; for
    /// other chain lengths `None` (not yet modelled).
    pub fn inner_radius(&self) -> Option<Quantity> {
        let [first, second] = self.links.as_slice() else {
            return None;
        };
        let difference = first.length.checked_sub(&second.length).ok()?;
        let negated = Quantity::new(difference.si_value.checked_neg()?, LENGTH);
        match difference
            .checked_cmp(&Quantity::new(Rational::ZERO, LENGTH))
            .ok()?
        {
            Ordering::Less => Some(negated),
            _ => Some(difference),
        }
    }

    /// Worst-case static torque about each joint (base first): arm fully extended horizontally,
    /// uniform links (centre of mass at mid-length), point payload at the tip, standard gravity.
    /// The torque about joint `i` sums the weights of links `i..` and the payload times their
    /// horizontal distance from joint `i`. Exact.
    pub fn static_joint_torques(&self) -> Option<Vec<Quantity>> {
        let gravity = standard_gravity();
        let half = Quantity::new(Rational::new(1, 2)?, Dimension::DIMENSIONLESS);
        let mut torques = Vec::new();
        for joint in 0..self.links.len() {
            let mut offset = Quantity::new(Rational::ZERO, LENGTH);
            let mut moment = Quantity::new(Rational::ZERO, TORQUE)
                .with_kind(QuantityKind::Torque)
                .ok()?;
            for link in &self.links[joint..] {
                let centre = offset
                    .checked_add(&link.length.checked_mul(&half).ok()?)
                    .ok()?;
                let weight = link.mass.checked_mul(&gravity).ok()?;
                moment = moment
                    .checked_add(&weight.checked_mul(&centre).ok()?)
                    .ok()?;
                offset = offset.checked_add(&link.length).ok()?;
            }
            let payload_weight = self.payload.checked_mul(&gravity).ok()?;
            torques.push(
                moment
                    .checked_add(&payload_weight.checked_mul(&offset).ok()?)
                    .ok()?,
            );
        }
        Some(torques)
    }

    /// `static_joint_torques()[0]`.
    pub fn static_shoulder_torque(&self) -> Option<Quantity> {
        self.static_joint_torques()?.into_iter().next()
    }

    /// Planar forward kinematics: tip position in metres for joint angles in radians (f64: the
    /// trigonometry is transcendental). Angles are cumulative from the base x-axis.
    pub fn forward_kinematics(&self, joint_angles: &[f64]) -> Option<(f64, f64)> {
        if joint_angles.len() != self.links.len() {
            return None;
        }
        let (mut x, mut y, mut theta) = (0.0, 0.0, 0.0);
        for (link, angle) in self.links.iter().zip(joint_angles) {
            theta += angle;
            let length = link.length.si_value.to_f64();
            x += length * theta.cos();
            y += length * theta.sin();
        }
        Some((x, y))
    }

    /// Whether every angle is within its joint's declared limits (limits compared in f64 from
    /// their exact values).
    pub fn within_limits(&self, joint_angles: &[f64]) -> bool {
        joint_angles.len() == self.joints.len()
            && self.joints.iter().zip(joint_angles).all(|(joint, angle)| {
                *angle >= joint.lower.si_value.to_f64() && *angle <= joint.upper.si_value.to_f64()
            })
    }
}

fn requirement(
    arm: &DeclaredNode,
    attribute: &str,
    dimension: Dimension,
    derived: Option<&Quantity>,
    derived_name: &str,
    must_be: Ordering,
    findings: &mut Vec<PhysicalFinding>,
) -> Option<RequirementCheck> {
    let text = arm.attributes.get(attribute)?;
    let required = quantity_of(arm, attribute, dimension, findings);
    let (verdict, detail) = match (required, derived) {
        (Some(required), Some(derived)) => match derived.checked_cmp(&required) {
            Ok(ordering) if ordering == must_be || ordering == Ordering::Equal => (
                ConstraintVerdict::Satisfied,
                format!("{derived_name} {derived} satisfies {attribute} {text}"),
            ),
            Ok(_) => (
                ConstraintVerdict::Violated,
                format!("{derived_name} {derived} does not satisfy {attribute} {text}"),
            ),
            Err(error) => (ConstraintVerdict::Unknown, error.to_string()),
        },
        _ => (
            ConstraintVerdict::Unknown,
            format!("{attribute} or {derived_name} could not be evaluated"),
        ),
    };
    Some(RequirementCheck {
        subject: arm.name.clone(),
        requirement: format!("{attribute} = {text}"),
        verdict,
        detail,
    })
}

const CURRENT: Dimension = Dimension {
    exponents: [0, 0, 0, 1, 0, 0, 0, 0],
};
const VOLTAGE: Dimension = Dimension {
    exponents: [2, 1, -3, -1, 0, 0, 0, 0],
};
const RESISTANCE: Dimension = Dimension {
    exponents: [2, 1, -3, -2, 0, 0, 0, 0],
};
const TORQUE_CONSTANT: Dimension = Dimension {
    exponents: [2, 1, -2, -1, 0, 0, 0, 0],
};

/// A dimensionless parameter written as a bare decimal (`gear_ratio = 60`, `efficiency = 0.8`).
fn ratio_of(
    node: &DeclaredNode,
    attribute: &str,
    findings: &mut Vec<PhysicalFinding>,
) -> Option<Quantity> {
    let finding = |verdict, message: String| PhysicalFinding {
        subject: node.name.clone(),
        verdict,
        message,
    };
    let Some(text) = node.attributes.get(attribute) else {
        findings.push(finding(
            ConstraintVerdict::Unknown,
            format!("`{attribute}` is not declared"),
        ));
        return None;
    };
    match Rational::parse_decimal(text.trim()) {
        Some(value) if value.checked_cmp(Rational::ZERO) == Some(Ordering::Greater) => {
            Some(Quantity::new(value, Dimension::DIMENSIONLESS))
        }
        Some(_) => {
            findings.push(finding(
                ConstraintVerdict::Violated,
                format!("`{attribute} = {text}` must be positive"),
            ));
            None
        }
        None => {
            findings.push(finding(
                ConstraintVerdict::Unknown,
                format!("`{attribute} = {text}` is not a dimensionless number"),
            ));
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn derive(
    derived: &mut Vec<DerivedQuantity>,
    subject: &str,
    name: &str,
    value: &Quantity,
    unit: &str,
    rule: &str,
    assumptions: &[&str],
    inputs: Vec<String>,
) {
    derived.push(DerivedQuantity {
        subject: subject.into(),
        name: name.into(),
        display: display(value, unit),
        value: *value,
        status: EpistemicStatus::Derived,
        rule: rule.into(),
        assumptions: assumptions.iter().map(|a| (*a).into()).collect(),
        inputs,
    });
}

/// `capacity >= demand`, as a requirement check named after `attribute`.
fn capacity_check(
    node: &DeclaredNode,
    attribute: &str,
    capacity: &Quantity,
    demand: &Quantity,
    demand_name: &str,
) -> RequirementCheck {
    let subject = node.name.as_str();
    let declared = node
        .attributes
        .get(attribute)
        .map(String::as_str)
        .unwrap_or("");
    let (verdict, relation) = match demand.checked_cmp(capacity) {
        Ok(Ordering::Greater) => (ConstraintVerdict::Violated, "exceeds"),
        Ok(_) => (ConstraintVerdict::Satisfied, "within"),
        Err(_) => (ConstraintVerdict::Unknown, "not comparable with"),
    };
    RequirementCheck {
        subject: subject.into(),
        requirement: format!("{attribute} = {declared}"),
        verdict,
        detail: format!("{demand_name} {demand} {relation} {attribute} {declared}"),
    }
}

/// Physical milestone 2 (ADR 0014): cross-domain drivetrain and power analysis.
///
/// For every `Motor` driving a joint of an assembled arm (`joint = <Joint name>`):
/// - motor torque = static joint torque / (gear_ratio x efficiency)   [mechanics -> drivetrain]
/// - holding current = motor torque / torque_constant                 [drivetrain -> electrical]
/// - holding heat = current^2 x winding_resistance (if declared)      [electrical -> thermal]
///
/// checked against the motor's `rated_torque` and `max_current`. For every `Rail`, the currents of
/// the motors it feeds (`rail = <Rail name>`) are summed and checked against its `max_current`,
/// and its electrical power V x I is derived. Every step is exact and dimension-checked.
fn analyze_drivetrains(
    nodes: &[DeclaredNode],
    arms: &[PlanarArm],
    derived: &mut Vec<DerivedQuantity>,
    requirements: &mut Vec<RequirementCheck>,
    findings: &mut Vec<PhysicalFinding>,
) {
    let mut rail_currents: Vec<(String, Quantity, String)> = Vec::new();
    for arm in arms {
        let Some(joint_torques) = arm.static_joint_torques() else {
            continue;
        };
        for (index, (joint, joint_torque)) in arm.joints.iter().zip(&joint_torques).enumerate() {
            let mut torque_inputs: Vec<String> = arm.links[index..]
                .iter()
                .flat_map(|link| {
                    [
                        format!("{}.length", link.name),
                        format!("{}.mass", link.name),
                    ]
                })
                .collect();
            torque_inputs.push(format!("{}.payload", arm.name));
            for motor in nodes.iter().filter(|node| {
                node.node_kind == "Motor"
                    && node.attributes.get("joint").map(String::as_str) == Some(joint.name.as_str())
            }) {
                let before = findings.len();
                let ratio = ratio_of(motor, "gear_ratio", findings);
                let efficiency = ratio_of(motor, "efficiency", findings);
                let constant = quantity_of(motor, "torque_constant", TORQUE_CONSTANT, findings);
                let rated = quantity_of(motor, "rated_torque", TORQUE, findings);
                let max_current = quantity_of(motor, "max_current", CURRENT, findings);
                if let Some(efficiency) = &efficiency
                    && efficiency.si_value.checked_cmp(Rational::ONE) == Some(Ordering::Greater)
                {
                    findings.push(PhysicalFinding {
                        subject: motor.name.clone(),
                        verdict: ConstraintVerdict::Violated,
                        message: "efficiency above 1 violates energy conservation".into(),
                    });
                }
                let (Some(ratio), Some(efficiency), Some(constant), Some(rated), Some(max_current)) =
                    (ratio, efficiency, constant, rated, max_current)
                else {
                    continue;
                };
                if findings.len() != before {
                    continue;
                }
                let Some(motor_torque) = ratio
                    .checked_mul(&efficiency)
                    .ok()
                    .and_then(|reduction| joint_torque.checked_div(&reduction).ok())
                else {
                    continue;
                };
                let Some(current) = motor_torque
                    .checked_div(&constant)
                    .ok()
                    // Unreachable while torque_constant is dimension-checked (N*m/A); kept as
                    // defense in depth (G46 mutation: equivalent).
                    .filter(|current| current.dimension == CURRENT)
                else {
                    continue;
                };
                let torque_source = format!("{}.static_joint_torque", joint.name);
                derive(
                    derived,
                    &joint.name,
                    "static_joint_torque",
                    joint_torque,
                    "N*m",
                    "sum of m*g*r of links and payload beyond this joint, fully extended",
                    &[
                        "static",
                        "uniform-density links",
                        "point payload",
                        "standard gravity",
                    ],
                    torque_inputs.clone(),
                );
                derive(
                    derived,
                    &motor.name,
                    "motor_torque",
                    &motor_torque,
                    "N*m",
                    "joint torque / (gear_ratio x efficiency)",
                    &["efficiency constant at holding torque"],
                    vec![
                        torque_source,
                        format!("{}.gear_ratio", motor.name),
                        format!("{}.efficiency", motor.name),
                    ],
                );
                derive(
                    derived,
                    &motor.name,
                    "holding_current",
                    &current,
                    "A",
                    "motor torque / torque_constant",
                    &["linear torque-current relation"],
                    vec![
                        format!("{}.motor_torque", motor.name),
                        format!("{}.torque_constant", motor.name),
                    ],
                );
                if let Some(resistance) = motor
                    .attributes
                    .get("winding_resistance")
                    .and_then(|_| quantity_of(motor, "winding_resistance", RESISTANCE, findings))
                    && let Ok(heat) = current
                        .checked_mul(&current)
                        .and_then(|square| square.checked_mul(&resistance))
                {
                    derive(
                        derived,
                        &motor.name,
                        "holding_heat",
                        &heat,
                        "W",
                        "current^2 x winding_resistance",
                        &["DC holding, resistance at declared temperature"],
                        vec![
                            format!("{}.holding_current", motor.name),
                            format!("{}.winding_resistance", motor.name),
                        ],
                    );
                }
                requirements.push(capacity_check(
                    motor,
                    "rated_torque",
                    &rated,
                    &motor_torque,
                    "motor_torque",
                ));
                requirements.push(capacity_check(
                    motor,
                    "max_current",
                    &max_current,
                    &current,
                    "holding_current",
                ));
                if let Some(rail) = motor.attributes.get("rail") {
                    rail_currents.push((rail.clone(), current, motor.name.clone()));
                }
            }
        }
    }
    for rail in nodes.iter().filter(|node| node.node_kind == "Rail") {
        let feeds: Vec<&(String, Quantity, String)> = rail_currents
            .iter()
            .filter(|(name, _, _)| *name == rail.name)
            .collect();
        let voltage = quantity_of(rail, "voltage", VOLTAGE, findings);
        let capacity = quantity_of(rail, "max_current", CURRENT, findings);
        let Some(total) = sum(feeds.iter().map(|(_, current, _)| *current), CURRENT) else {
            continue;
        };
        let inputs: Vec<String> = feeds
            .iter()
            .map(|(_, _, motor)| format!("{motor}.holding_current"))
            .collect();
        derive(
            derived,
            &rail.name,
            "total_current",
            &total,
            "A",
            "sum of holding currents of the motors on this rail",
            &["all joints hold the worst-case static load simultaneously"],
            inputs,
        );
        if let Some(voltage) = voltage
            && let Ok(power) = voltage.checked_mul(&total)
        {
            derive(
                derived,
                &rail.name,
                "electrical_power",
                &power,
                "W",
                "voltage x total_current",
                &["nominal rail voltage"],
                vec![
                    format!("{}.voltage", rail.name),
                    format!("{}.total_current", rail.name),
                ],
            );
        }
        if let Some(capacity) = capacity {
            requirements.push(capacity_check(
                rail,
                "max_current",
                &capacity,
                &total,
                "total_current",
            ));
        }
    }
}

const TIME: Dimension = Dimension {
    exponents: [0, 0, 1, 0, 0, 0, 0, 0],
};

/// Controller gains and integration settings used for every trajectory simulation (recorded in
/// each run's configuration and identity).
/// Computed-torque gains: natural frequency 30 rad/s, critical damping (Kp = w^2, Kd = 2*zeta*w).
const SIM_KP: [f64; 2] = [900.0, 900.0];
const SIM_KD: [f64; 2] = [60.0, 60.0];
const SIM_STEP_S: f64 = 1e-3;
const SIM_HOLD_S: f64 = 0.3;

/// Actuator torque available at a joint: rated motor torque x gear ratio x efficiency, exact and
/// with the declared uncertainty of each factor (G131).
fn joint_torque_limit(nodes: &[DeclaredNode], joint: &str) -> Option<Quantity> {
    let motor = nodes.iter().find(|node| {
        node.node_kind == "Motor" && node.attributes.get("joint").map(String::as_str) == Some(joint)
    })?;
    let mut scratch = Vec::new();
    let rated = quantity_of(motor, "rated_torque", TORQUE, &mut scratch)?;
    let ratio = ratio_of(motor, "gear_ratio", &mut scratch)?;
    let efficiency = ratio_of(motor, "efficiency", &mut scratch)?;
    rated
        .checked_mul(&ratio)
        .ok()?
        .checked_mul(&efficiency)
        .ok()
}

/// Whether `angle` lies within `joint`'s declared limits, decided exactly on the declared
/// quantities: UNKNOWN when an uncertain angle or limit overlaps the boundary.
fn angle_within(joint: &RevoluteJoint, angle: &Quantity) -> ConstraintVerdict {
    use ConstraintVerdict::{Satisfied, Unknown, Violated};
    let at_least = match angle.checked_cmp(&joint.lower) {
        Ok(Ordering::Less) => Violated,
        Ok(_) => Satisfied,
        Err(_) => Unknown,
    };
    let at_most = match angle.checked_cmp(&joint.upper) {
        Ok(Ordering::Greater) => Violated,
        Ok(_) => Satisfied,
        Err(_) => Unknown,
    };
    ConstraintVerdict::all([at_least, at_most])
}

/// `value` against a declared interval, in `f64`: SATISFIED at or below its low bound, VIOLATED
/// above its high bound, UNKNOWN between them.
fn at_most(value: f64, bound: &Quantity) -> ConstraintVerdict {
    let interval = bound.interval();
    if value <= interval.low.to_f64() {
        ConstraintVerdict::Satisfied
    } else if value > interval.high.to_f64() {
        ConstraintVerdict::Violated
    } else {
        ConstraintVerdict::Unknown
    }
}

/// Physical milestone 3 (ADR 0019): simulate every declared `Trajectory` of an assembled 2-link
/// arm and decide its dynamic requirements from `SIMULATED` evidence.
fn simulate_trajectories(
    nodes: &[DeclaredNode],
    arms: &[PlanarArm],
    requirements: &mut Vec<RequirementCheck>,
    findings: &mut Vec<PhysicalFinding>,
) -> Vec<SimulationRecord> {
    let mut records = Vec::new();
    for trajectory in nodes.iter().filter(|node| node.node_kind == "Trajectory") {
        let Some(arm_name) = trajectory.attributes.get("arm") else {
            findings.push(PhysicalFinding {
                subject: trajectory.name.clone(),
                verdict: ConstraintVerdict::Unknown,
                message: "`arm` is not declared".into(),
            });
            continue;
        };
        let Some(arm) = arms.iter().find(|arm| &arm.name == arm_name) else {
            findings.push(PhysicalFinding {
                subject: trajectory.name.clone(),
                verdict: ConstraintVerdict::Unknown,
                message: format!("arm `{arm_name}` was not assembled"),
            });
            continue;
        };
        let before = findings.len();
        let angle = |name: &str, findings: &mut Vec<PhysicalFinding>| {
            quantity_of(trajectory, name, ANGLE, findings)
        };
        let mut drops = Vec::new();
        let from = [
            angle("shoulder_from", findings),
            angle("elbow_from", findings),
        ];
        let to = [angle("shoulder_to", findings), angle("elbow_to", findings)];
        let duration = quantity_of(trajectory, "duration", TIME, findings);
        let tolerance = quantity_of(trajectory, "tracking_tolerance", ANGLE, findings);
        let limits = [
            arm.joints
                .first()
                .and_then(|j| joint_torque_limit(nodes, &j.name)),
            arm.joints
                .get(1)
                .and_then(|j| joint_torque_limit(nodes, &j.name)),
        ];
        let (
            [Some(f0), Some(f1)],
            [Some(t0), Some(t1)],
            Some(duration),
            Some(tolerance),
            [Some(limit0), Some(limit1)],
            Some(dynamics),
        ) = (
            from,
            to,
            duration,
            tolerance,
            limits,
            dynamics::ArmDynamics::from_arm(arm, &mut drops),
        )
        else {
            if findings.len() == before {
                findings.push(PhysicalFinding {
                    subject: trajectory.name.clone(),
                    verdict: ConstraintVerdict::Unknown,
                    message: "simulation needs a 2-link arm with a motor on every joint".into(),
                });
            }
            continue;
        };
        if findings.len() != before {
            continue;
        }
        // Joint limits are decided exactly on the declared quantities (G131).
        let within = ConstraintVerdict::all(
            arm.joints
                .iter()
                .zip([[&f0, &t0], [&f1, &t1]])
                .flat_map(|(joint, ends)| ends.map(|angle| angle_within(joint, angle))),
        );
        requirements.push(RequirementCheck {
            subject: trajectory.name.clone(),
            requirement: "trajectory within joint limits".into(),
            verdict: within,
            detail: "minimum-jerk moves are monotone per joint, so the endpoints bound the path; \
                     decided exactly on the declared angles and limits"
                .into(),
        });
        let name = |attribute: &str| format!("{}.{attribute}", trajectory.name);
        let config = dynamics::SimulationConfig {
            dynamics,
            trajectory: dynamics::JointMove {
                from: [
                    f0.drop_to_f64(name("shoulder_from"), &mut drops),
                    f1.drop_to_f64(name("elbow_from"), &mut drops),
                ],
                to: [
                    t0.drop_to_f64(name("shoulder_to"), &mut drops),
                    t1.drop_to_f64(name("elbow_to"), &mut drops),
                ],
                duration_s: duration.drop_to_f64(name("duration"), &mut drops),
            },
            torque_limits: [
                limit0.drop_to_f64(format!("{} torque limit", arm.joints[0].name), &mut drops),
                limit1.drop_to_f64(format!("{} torque limit", arm.joints[1].name), &mut drops),
            ],
            kp: SIM_KP,
            kd: SIM_KD,
            step_s: SIM_STEP_S,
            hold_s: SIM_HOLD_S,
            integrator: "rk4".into(),
        };
        let run = dynamics::simulate(&config);
        if run.diverged {
            requirements.push(RequirementCheck {
                subject: trajectory.name.clone(),
                requirement: "simulation converged".into(),
                verdict: ConstraintVerdict::Unknown,
                detail: "the integration diverged; no dynamic requirement can be decided from it"
                    .into(),
            });
            records.push(SimulationRecord {
                trajectory: trajectory.name.clone(),
                arm: arm.name.clone(),
                config,
                run,
                inputs: drops,
            });
            continue;
        }
        // G131: the simulation runs on nominal values. A model or trajectory input with declared
        // uncertainty makes every verdict drawn from it UNKNOWN: the nominal run bounds nothing.
        let uncertain: Vec<&str> = drops
            .iter()
            .filter(|d| d.dropped_bounds.is_some() && !d.subject.ends_with("torque limit"))
            .map(|d| d.subject.as_str())
            .collect();
        let from_simulation = |verdict: ConstraintVerdict| {
            if uncertain.is_empty() {
                verdict
            } else {
                ConstraintVerdict::Unknown
            }
        };
        let uncertainty_note = if uncertain.is_empty() {
            String::new()
        } else {
            format!(
                "; UNKNOWN: {} carry declared uncertainty the nominal simulation does not bound",
                uncertain.join(", ")
            )
        };
        for (joint, index) in arm.joints.iter().zip(0..2) {
            let required = run.ideal_peak[index];
            let limit = [&limit0, &limit1][index];
            let (low, high) = limit.bounds();
            requirements.push(RequirementCheck {
                subject: trajectory.name.clone(),
                requirement: format!("{} dynamic torque within actuator capability", joint.name),
                verdict: from_simulation(at_most(required, limit)),
                detail: format!(
                    "DERIVED peak torque the trajectory requires (inverse dynamics) {required:.4} N*m vs [{:.4}, {:.4}] N*m available (rated x gear x efficiency, exact bounds); SIMULATED controller peak {:.4} N*m, {} saturated steps{uncertainty_note}",
                    low.to_f64(),
                    high.to_f64(),
                    run.peak_demand[index],
                    run.saturated_steps[index]
                ),
            });
        }
        let tolerance_shown = tolerance.approximate().value;
        requirements.push(RequirementCheck {
            subject: trajectory.name.clone(),
            requirement: format!("tracking error <= {tolerance_shown} rad"),
            verdict: from_simulation(at_most(run.max_tracking_error_rad, &tolerance)),
            detail: format!(
                "SIMULATED max tracking error {:.6} rad, final error {:.6} rad{uncertainty_note}",
                run.max_tracking_error_rad, run.final_error_rad
            ),
        });
        records.push(SimulationRecord {
            trajectory: trajectory.name.clone(),
            arm: arm.name.clone(),
            config,
            run,
            inputs: drops,
        });
    }
    records
}

/// Physical milestone 1: assemble every declared arm, derive its exact reach, inner radius and
/// worst-case static shoulder torque, and check the arm's declared requirements
/// (`required_reach` against reach, `shoulder_torque_limit` against the derived torque).
pub fn analyze_physical(nodes: &[DeclaredNode]) -> PhysicalReport {
    let (arms, mut findings) = assemble_arms(nodes);
    let mut derived = Vec::new();
    let mut requirements = Vec::new();
    for arm in &arms {
        let reach = arm.reach();
        let torque = arm.static_shoulder_torque();
        let link_inputs = |attributes: &[&str]| -> Vec<String> {
            arm.links
                .iter()
                .flat_map(|link| attributes.iter().map(move |a| format!("{}.{a}", link.name)))
                .collect()
        };
        let mut torque_inputs = link_inputs(&["length", "mass"]);
        torque_inputs.push(format!("{}.payload", arm.name));
        let mut push = |name: &str,
                        value: Option<Quantity>,
                        unit: &str,
                        rule: &str,
                        assumptions: &[&str],
                        inputs: Vec<String>| {
            if let Some(value) = value {
                derived.push(DerivedQuantity {
                    subject: arm.name.clone(),
                    name: name.into(),
                    display: display(&value, unit),
                    value,
                    status: EpistemicStatus::Derived,
                    rule: rule.into(),
                    assumptions: assumptions.iter().map(|a| (*a).into()).collect(),
                    inputs,
                });
            }
        };
        push(
            "reach",
            reach,
            "mm",
            "sum of link lengths",
            &["rigid links", "serial chain"],
            link_inputs(&["length"]),
        );
        push(
            "inner_radius",
            arm.inner_radius(),
            "mm",
            "|l1 - l2| for a 2-link arm",
            &["elbow can fold to +/-pi (not checked against joint limits)"],
            link_inputs(&["length"]),
        );
        push(
            "static_shoulder_torque",
            torque,
            "N*m",
            "sum of m*g*r about joint 1, fully extended horizontally",
            &[
                "static (no acceleration)",
                "uniform-density links, centre of mass at mid-length",
                "point payload at the tip",
                "standard gravity 9.80665 m/s^2",
            ],
            torque_inputs.clone(),
        );
        let declared = nodes
            .iter()
            .find(|node| node.node_kind == "Arm" && node.name == arm.name)
            .expect("assembled from a declared arm");
        requirements.extend(requirement(
            declared,
            "required_reach",
            LENGTH,
            reach.as_ref(),
            "reach",
            Ordering::Greater,
            &mut findings,
        ));
        requirements.extend(requirement(
            declared,
            "shoulder_torque_limit",
            TORQUE,
            torque.as_ref(),
            "static_shoulder_torque",
            Ordering::Less,
            &mut findings,
        ));
    }
    analyze_drivetrains(nodes, &arms, &mut derived, &mut requirements, &mut findings);
    let simulations = simulate_trajectories(nodes, &arms, &mut requirements, &mut findings);
    PhysicalReport {
        schema: "atlas.physical-report.v1".into(),
        evidence_level: if simulations.is_empty() {
            PhysicalEvidenceLevel::SemanticModel
        } else {
            PhysicalEvidenceLevel::Simulated
        },
        arms,
        derived,
        requirements,
        findings,
        simulations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::adl::SourceSpan;
    use std::collections::BTreeMap;

    fn node(kind: &str, name: &str, attributes: &[(&str, &str)]) -> DeclaredNode {
        DeclaredNode {
            id: format!("node:{name}"),
            name: name.into(),
            node_kind: kind.into(),
            attributes: attributes
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect::<BTreeMap<_, _>>(),
            origin: "declared".into(),
            span: SourceSpan {
                path: "arm.adl".into(),
                line: 1,
                column: 1,
            },
        }
    }

    fn arm(extra: &[(&str, &str)]) -> Vec<DeclaredNode> {
        let mut arm_attributes = vec![("payload", "0.5 kg")];
        arm_attributes.extend_from_slice(extra);
        vec![
            node("Arm", "A", &arm_attributes),
            node(
                "Link",
                "Upper",
                &[
                    ("arm", "A"),
                    ("index", "1"),
                    ("length", "300 mm"),
                    ("mass", "0.4 kg"),
                ],
            ),
            node(
                "Link",
                "Fore",
                &[
                    ("arm", "A"),
                    ("index", "2"),
                    ("length", "0.25 m"),
                    ("mass", "300 g"),
                ],
            ),
            node(
                "Joint",
                "Shoulder",
                &[
                    ("arm", "A"),
                    ("index", "1"),
                    ("lower", "-1.5 rad"),
                    ("upper", "1.5 rad"),
                ],
            ),
            node(
                "Joint",
                "Elbow",
                &[
                    ("arm", "A"),
                    ("index", "2"),
                    ("lower", "-2 rad"),
                    ("upper", "2 rad"),
                ],
            ),
        ]
    }

    fn derived<'a>(report: &'a PhysicalReport, name: &str) -> &'a DerivedQuantity {
        report.derived.iter().find(|d| d.name == name).unwrap()
    }

    #[test]
    fn exact_reach_inner_radius_and_static_torque_from_mixed_units() {
        let report = analyze_physical(&arm(&[]));
        assert!(report.findings.is_empty(), "{:?}", report.findings);
        assert_eq!(
            derived(&report, "reach").value,
            Quantity::parse("550 mm").unwrap()
        );
        assert_eq!(
            derived(&report, "inner_radius").value,
            Quantity::parse("50 mm").unwrap()
        );
        // tau = g * (0.4*0.15 + 0.3*(0.3+0.125) + 0.5*0.55) = 9.80665 * 0.4625 = 4.535575625 N*m
        assert_eq!(
            derived(&report, "static_shoulder_torque").value,
            Quantity::parse("4.535575625 N*m torque").unwrap()
        );
        assert!(
            report
                .derived
                .iter()
                .all(|d| d.status == EpistemicStatus::Derived && !d.assumptions.is_empty())
        );
    }

    #[test]
    fn requirements_are_decided_against_derived_quantities() {
        let report = analyze_physical(&arm(&[
            ("required_reach", "0.5 m"),
            ("shoulder_torque_limit", "3 N*m"),
        ]));
        let verdict = |requirement: &str| {
            report
                .requirements
                .iter()
                .find(|check| check.requirement.starts_with(requirement))
                .unwrap()
                .verdict
        };
        assert_eq!(verdict("required_reach"), ConstraintVerdict::Satisfied);
        assert_eq!(
            verdict("shoulder_torque_limit"),
            ConstraintVerdict::Violated
        );

        let boundary = analyze_physical(&arm(&[("required_reach", "550 mm")]));
        assert_eq!(
            boundary.requirements[0].verdict,
            ConstraintVerdict::Satisfied,
            "equality meets a minimum"
        );
        let too_far = analyze_physical(&arm(&[("required_reach", "550.001 mm")]));
        assert_eq!(too_far.requirements[0].verdict, ConstraintVerdict::Violated);
    }

    #[test]
    fn dimensionally_wrong_or_missing_parameters_yield_findings_and_no_model() {
        let mut nodes = arm(&[]);
        nodes[1].attributes.insert("length".into(), "300 kg".into());
        let report = analyze_physical(&nodes);
        assert!(report.arms.is_empty());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.verdict == ConstraintVerdict::Violated && f.subject == "Upper")
        );

        let mut nodes = arm(&[]);
        nodes[3].attributes.insert("upper".into(), "90 deg".into());
        let report = analyze_physical(&nodes);
        assert!(report.arms.is_empty());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.verdict == ConstraintVerdict::Unknown && f.subject == "Shoulder")
        );

        let mut nodes = arm(&[]);
        nodes[4].attributes.insert("lower".into(), "3 rad".into());
        assert!(
            analyze_physical(&nodes).arms.is_empty(),
            "inverted joint limits"
        );

        let mut nodes = arm(&[]);
        nodes.remove(4);
        assert!(
            analyze_physical(&nodes).arms.is_empty(),
            "link/joint count mismatch"
        );

        let report = analyze_physical(&arm(&[("shoulder_torque_limit", "3 kg")]));
        assert_eq!(report.requirements[0].verdict, ConstraintVerdict::Unknown);

        // G124: a torque limit declared in joules has the torque dimension but is an energy.
        let report = analyze_physical(&arm(&[("shoulder_torque_limit", "3 J")]));
        assert!(report.findings.iter().any(
            |f| f.verdict == ConstraintVerdict::Violated && f.message.contains("not a torque")
        ));
        let declared = analyze_physical(&arm(&[("shoulder_torque_limit", "3 N*m torque")]));
        assert!(
            !declared
                .findings
                .iter()
                .any(|f| f.message.contains("not a torque"))
        );
        let unspecified = analyze_physical(&arm(&[("shoulder_torque_limit", "3 N*m")]));
        let verdicts = |r: &PhysicalReport| -> Vec<ConstraintVerdict> {
            r.requirements.iter().map(|c| c.verdict).collect()
        };
        assert_eq!(
            verdicts(&declared),
            verdicts(&unspecified),
            "an undeclared N*m joins the torque kind"
        );
    }

    fn with_drivetrain(payload: &str, rail_capacity: &str) -> Vec<DeclaredNode> {
        let mut nodes = arm(&[]);
        nodes[0].attributes.insert("payload".into(), payload.into());
        nodes.push(node(
            "Motor",
            "ShoulderMotor",
            &[
                ("joint", "Shoulder"),
                ("gear_ratio", "50"),
                ("efficiency", "0.8"),
                ("torque_constant", "0.05 N*m/A"),
                ("rated_torque", "0.3 N*m"),
                ("max_current", "6 A"),
                ("winding_resistance", "2 ohm"),
                ("rail", "Main"),
            ],
        ));
        nodes.push(node(
            "Motor",
            "ElbowMotor",
            &[
                ("joint", "Elbow"),
                ("gear_ratio", "30"),
                ("efficiency", "0.8"),
                ("torque_constant", "50 mN*m/A"),
                ("rated_torque", "0.2 N*m"),
                ("max_current", "4000 mA"),
                ("rail", "Main"),
            ],
        ));
        nodes.push(node(
            "Rail",
            "Main",
            &[("voltage", "12 V"), ("max_current", rail_capacity)],
        ));
        nodes
    }

    fn value<'a>(report: &'a PhysicalReport, subject: &str, name: &str) -> &'a DerivedQuantity {
        report
            .derived
            .iter()
            .find(|d| d.subject == subject && d.name == name)
            .unwrap_or_else(|| panic!("{subject}.{name}"))
    }

    #[test]
    fn drivetrain_and_power_chain_is_exact_across_domains() {
        let report = analyze_physical(&with_drivetrain("0.5 kg", "5 A"));
        assert!(report.findings.is_empty(), "{:?}", report.findings);
        // Shoulder: 4.535575625 N*m / (50 * 0.8) = 0.113389390625 N*m -> / 0.05 = 2.2677878125 A.
        assert_eq!(
            value(&report, "ShoulderMotor", "motor_torque").value,
            Quantity::parse("0.113389390625 N*m").unwrap()
        );
        assert_eq!(
            value(&report, "ShoulderMotor", "holding_current").value,
            Quantity::parse("2.2677878125 A").unwrap()
        );
        // I^2 R = 2.2677878125^2 * 2 W, exact.
        let heat = Rational::new(22677878125, 10_000_000_000).unwrap();
        let heat = heat
            .checked_mul(heat)
            .unwrap()
            .checked_mul(Rational::integer(2))
            .unwrap();
        assert_eq!(
            value(&report, "ShoulderMotor", "holding_heat")
                .value
                .si_value,
            heat
        );
        // Elbow: g*(0.3*0.125 + 0.5*0.25) = 1.593580625 N*m; / (30*0.8) / 0.05 = 1593580625/1200000000 A.
        assert_eq!(
            value(&report, "Elbow", "static_joint_torque").value,
            Quantity::parse("1.593580625 N*m torque").unwrap()
        );
        let elbow_current = Rational::new(1_593_580_625, 1_200_000_000).unwrap();
        assert_eq!(
            value(&report, "ElbowMotor", "holding_current")
                .value
                .si_value,
            elbow_current
        );
        let total = Rational::new(22677878125, 10_000_000_000)
            .unwrap()
            .checked_add(elbow_current)
            .unwrap();
        assert_eq!(
            value(&report, "Main", "total_current").value.si_value,
            total
        );
        assert_eq!(
            value(&report, "Main", "electrical_power").value.si_value,
            total.checked_mul(Rational::integer(12)).unwrap()
        );
        assert!(
            report
                .requirements
                .iter()
                .all(|r| r.verdict == ConstraintVerdict::Satisfied),
            "{:?}",
            report.requirements
        );
        // Provenance: the rail total names both motor currents; the elbow torque names only the
        // links beyond the elbow.
        assert_eq!(
            value(&report, "Main", "total_current").inputs,
            [
                "ShoulderMotor.holding_current",
                "ElbowMotor.holding_current"
            ]
        );
        assert_eq!(
            value(&report, "Elbow", "static_joint_torque").inputs,
            ["Fore.length", "Fore.mass", "A.payload"]
        );
    }

    #[test]
    fn a_heavier_payload_breaks_the_power_rail_while_reach_still_holds() {
        let mut nodes = with_drivetrain("1.5 kg", "5 A");
        nodes[0]
            .attributes
            .insert("required_reach".into(), "500 mm".into());
        let report = analyze_physical(&nodes);
        let verdict = |subject: &str, attribute: &str| {
            report
                .requirements
                .iter()
                .find(|r| r.subject == subject && r.requirement.starts_with(attribute))
                .unwrap()
                .verdict
        };
        assert_eq!(
            verdict("A", "required_reach"),
            ConstraintVerdict::Satisfied,
            "mechanics alone looks fine"
        );
        assert_eq!(
            verdict("Main", "max_current"),
            ConstraintVerdict::Violated,
            "the power domain disagrees"
        );
        assert_eq!(
            verdict("ShoulderMotor", "rated_torque"),
            ConstraintVerdict::Satisfied
        );
    }

    #[test]
    fn invalid_drivetrain_parameters_are_findings_not_values() {
        let mut nodes = with_drivetrain("0.5 kg", "5 A");
        nodes[5]
            .attributes
            .insert("efficiency".into(), "1.2".into());
        let report = analyze_physical(&nodes);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.subject == "ShoulderMotor" && f.verdict == ConstraintVerdict::Violated)
        );
        assert!(report.derived.iter().all(|d| d.subject != "ShoulderMotor"));

        let mut nodes = with_drivetrain("0.5 kg", "5 A");
        nodes[6]
            .attributes
            .insert("torque_constant".into(), "0.05 N*m".into());
        let report = analyze_physical(&nodes);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.subject == "ElbowMotor" && f.verdict == ConstraintVerdict::Violated)
        );
        assert!(report.derived.iter().all(|d| d.subject != "ElbowMotor"));
    }

    fn with_trajectory(duration: &str) -> Vec<DeclaredNode> {
        let mut nodes = with_drivetrain("0.5 kg", "5 A");
        nodes.push(node(
            "Trajectory",
            "Reach1",
            &[
                ("arm", "A"),
                ("shoulder_from", "0 rad"),
                ("elbow_from", "0 rad"),
                ("shoulder_to", "1.2 rad"),
                ("elbow_to", "-0.8 rad"),
                ("duration", duration),
                ("tracking_tolerance", "0.02 rad"),
            ],
        ));
        nodes
    }

    fn check<'a>(report: &'a PhysicalReport, requirement: &str) -> &'a RequirementCheck {
        report
            .requirements
            .iter()
            .find(|r| r.subject == "Reach1" && r.requirement.starts_with(requirement))
            .unwrap_or_else(|| panic!("{requirement}: {:#?}", report.requirements))
    }

    #[test]
    fn a_slow_move_satisfies_its_dynamic_requirements_in_simulation() {
        let report = analyze_physical(&with_trajectory("0.8 s"));
        assert!(report.findings.is_empty(), "{:?}", report.findings);
        assert_eq!(report.evidence_level, PhysicalEvidenceLevel::Simulated);
        // G134: each rung says what it makes a claim in the one vocabulary.
        assert_eq!(
            report.evidence_level.epistemic(),
            EpistemicStatus::Simulated
        );
        assert_eq!(
            PhysicalEvidenceLevel::SemanticModel.epistemic(),
            EpistemicStatus::Derived
        );
        assert_eq!(
            PhysicalEvidenceLevel::HilVerified.epistemic(),
            EpistemicStatus::Observed
        );
        assert_eq!(
            analyze_physical(&arm(&[])).evidence_level,
            PhysicalEvidenceLevel::SemanticModel
        );
        assert_eq!(report.simulations.len(), 1);
        assert_eq!(report.simulations[0].run.status, EpistemicStatus::Simulated);
        for requirement in [
            "trajectory within joint limits",
            "Shoulder dynamic",
            "Elbow dynamic",
            "tracking error",
        ] {
            assert_eq!(
                check(&report, requirement).verdict,
                ConstraintVerdict::Satisfied,
                "{requirement}: {:?}",
                check(&report, requirement)
            );
        }
        // Available torque: 0.3 N*m x 50 x 0.8 = 12 N*m at the shoulder, 0.2 x 30 x 0.8 = 4.8 at the elbow.
        assert!((report.simulations[0].config.torque_limits[0] - 12.0).abs() < 1e-12);
        assert!((report.simulations[0].config.torque_limits[1] - 4.8).abs() < 1e-12);
    }

    /// G131: every value a simulation takes as `f64` is recorded, marked exact or not.
    #[test]
    fn every_value_a_simulation_takes_as_f64_is_recorded() {
        let report = analyze_physical(&with_trajectory("0.8 s"));
        let inputs = &report.simulations[0].inputs;
        let subjects: Vec<&str> = inputs.iter().map(|d| d.subject.as_str()).collect();
        for subject in [
            "Upper.length",
            "Upper.mass",
            "Fore.length",
            "Fore.mass",
            "A.payload",
            "standard_gravity",
            "Reach1.shoulder_from",
            "Reach1.elbow_from",
            "Reach1.shoulder_to",
            "Reach1.elbow_to",
            "Reach1.duration",
            "Shoulder torque limit",
            "Elbow torque limit",
        ] {
            assert!(subjects.contains(&subject), "{subject}: {subjects:?}");
        }
        assert_eq!(inputs.len(), 13);
        let drop = |subject: &str| inputs.iter().find(|d| d.subject == subject).unwrap();
        assert!(!drop("Upper.length").exact, "0.3 m has no exact f64");
        assert!(drop("Fore.length").exact, "0.25 m does");
        assert!(inputs.iter().all(|d| d.dropped_bounds.is_none()));
    }

    /// G131: a nominal simulation never decides a requirement whose inputs carry declared
    /// uncertainty; an uncertain actuator is decided only when its whole interval is on one side;
    /// joint limits are decided exactly.
    #[test]
    fn declared_uncertainty_is_never_decided_from_a_nominal_simulation() {
        use ConstraintVerdict::{Satisfied, Unknown, Violated};
        let mut nodes = with_trajectory("0.8 s");
        nodes[1]
            .attributes
            .insert("mass".into(), "0.4 ± 0.05 kg".into());
        let report = analyze_physical(&nodes);
        for requirement in ["Shoulder dynamic", "Elbow dynamic", "tracking error"] {
            let c = check(&report, requirement);
            assert_eq!(c.verdict, Unknown, "{requirement}: {c:?}");
            assert!(c.detail.contains("Upper.mass"), "{c:?}");
        }
        assert_eq!(
            check(&report, "trajectory within joint limits").verdict,
            Satisfied
        );
        let mass = report.simulations[0]
            .inputs
            .iter()
            .find(|d| d.subject == "Upper.mass")
            .unwrap();
        assert_eq!(mass.dropped_bounds, Some((0.35, 0.45)));

        let nominal = analyze_physical(&with_trajectory("0.8 s"));
        let required = nominal.simulations[0].run.ideal_peak[0];
        let shoulder = |rated: String| {
            let mut nodes = with_trajectory("0.8 s");
            nodes[5].attributes.insert("rated_torque".into(), rated);
            check(&analyze_physical(&nodes), "Shoulder dynamic").verdict
        };
        // 0.3 ± 0.01 N*m x 50 x 0.8: [11.6, 12.4] N*m, all of it above the requirement.
        assert!(required < 11.6, "{required}");
        assert_eq!(shoulder("0.3 ± 0.01 N*m torque".into()), Satisfied);
        // An interval around the requirement decides nothing; one wholly below it violates.
        let at = required / 40.0;
        assert_eq!(
            shoulder(format!("{at:.6} ± {:.6} N*m torque", at * 0.1)),
            Unknown
        );
        let low = required / 80.0;
        assert_eq!(
            shoulder(format!("{low:.6} ± {:.6} N*m torque", low * 0.01)),
            Violated
        );

        // An uncertain angle overlapping the 1.5 rad shoulder limit.
        let mut nodes = with_trajectory("0.8 s");
        nodes[8]
            .attributes
            .insert("shoulder_to".into(), "1.45 ± 0.1 rad".into());
        let report = analyze_physical(&nodes);
        assert_eq!(
            check(&report, "trajectory within joint limits").verdict,
            Unknown
        );
        assert_eq!(check(&report, "Shoulder dynamic").verdict, Unknown);
        let mut nodes = with_trajectory("0.8 s");
        nodes[8]
            .attributes
            .insert("shoulder_to".into(), "1.7 ± 0.1 rad".into());
        assert_eq!(
            check(&analyze_physical(&nodes), "trajectory within joint limits").verdict,
            Violated
        );
    }

    #[test]
    fn a_fast_move_that_passes_every_static_check_is_caught_by_simulation() {
        let report = analyze_physical(&with_trajectory("0.25 s"));
        // Every static requirement still holds...
        assert!(
            report
                .requirements
                .iter()
                .filter(|r| r.subject != "Reach1")
                .all(|r| r.verdict == ConstraintVerdict::Satisfied)
        );
        // ...but the dynamic demand exceeds what the shoulder actuator can deliver.
        assert_eq!(
            check(&report, "Shoulder dynamic").verdict,
            ConstraintVerdict::Violated
        );
        assert!(report.simulations[0].run.saturated_steps[0] > 0);
    }

    #[test]
    fn run_identity_is_deterministic_and_sensitive_to_every_parameter() {
        let first = analyze_physical(&with_trajectory("0.8 s"));
        let again = analyze_physical(&with_trajectory("0.8 s"));
        assert_eq!(
            first.simulations[0].run, again.simulations[0].run,
            "identical config -> identical run"
        );
        let other = analyze_physical(&with_trajectory("0.81 s"));
        assert_ne!(
            first.simulations[0].run.run_identity,
            other.simulations[0].run.run_identity
        );
        let mut config = first.simulations[0].config.clone();
        config.kd[1] += 1e-12;
        assert_ne!(config.run_identity(), first.simulations[0].run.run_identity);
    }

    #[test]
    fn a_trajectory_outside_joint_limits_or_with_wrong_units_is_not_simulated_as_valid() {
        let mut nodes = with_trajectory("0.8 s");
        nodes
            .last_mut()
            .unwrap()
            .attributes
            .insert("shoulder_to".into(), "1.6 rad".into());
        let report = analyze_physical(&nodes);
        assert_eq!(
            check(&report, "trajectory within joint limits").verdict,
            ConstraintVerdict::Violated
        );
        let mut nodes = with_trajectory("0.8 s");
        nodes
            .last_mut()
            .unwrap()
            .attributes
            .insert("duration".into(), "0.8 m".into());
        let report = analyze_physical(&nodes);
        assert!(report.simulations.is_empty());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.subject == "Reach1" && f.verdict == ConstraintVerdict::Violated)
        );
    }

    #[test]
    fn forward_kinematics_matches_the_closed_form_and_never_exceeds_reach() {
        let report = analyze_physical(&arm(&[]));
        let arm = &report.arms[0];
        let (l1, l2) = (0.3, 0.25);
        let mut state = 0x1234_5678_u64;
        for _ in 0..1000 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let q1 = (state % 3000) as f64 / 1000.0 - 1.5;
            let q2 = ((state >> 16) % 4000) as f64 / 1000.0 - 2.0;
            let (x, y) = arm.forward_kinematics(&[q1, q2]).unwrap();
            let expected = (
                l1 * q1.cos() + l2 * (q1 + q2).cos(),
                l1 * q1.sin() + l2 * (q1 + q2).sin(),
            );
            assert!((x - expected.0).abs() < 1e-12 && (y - expected.1).abs() < 1e-12);
            assert!(x.hypot(y) <= 0.55 + 1e-12);
            assert!(arm.within_limits(&[q1, q2]));
        }
        assert_eq!(arm.forward_kinematics(&[0.0, 0.0]), Some((0.55, 0.0)));
        assert!(!arm.within_limits(&[1.6, 0.0]));
        assert_eq!(arm.forward_kinematics(&[0.0]), None);
    }
}
