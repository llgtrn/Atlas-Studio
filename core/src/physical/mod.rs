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

use crate::{
    ConstraintVerdict, EpistemicStatus,
    language::adl::DeclaredNode,
    quantity::{Dimension, Quantity, Rational},
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequirementCheck {
    pub subject: String,
    pub requirement: String,
    pub verdict: ConstraintVerdict,
    pub detail: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhysicalReport {
    pub schema: String,
    pub evidence_level: String,
    pub arms: Vec<PlanarArm>,
    pub derived: Vec<DerivedQuantity>,
    pub requirements: Vec<RequirementCheck>,
    pub findings: Vec<PhysicalFinding>,
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

    /// Worst-case static torque about the first joint: arm fully extended horizontally, uniform
    /// links (centre of mass at mid-length), point payload at the tip, standard gravity. Exact.
    pub fn static_shoulder_torque(&self) -> Option<Quantity> {
        let gravity = standard_gravity();
        let half = Quantity::new(Rational::new(1, 2)?, Dimension::DIMENSIONLESS);
        let mut offset = Quantity::new(Rational::ZERO, LENGTH);
        let mut moment = Quantity::new(Rational::ZERO, TORQUE);
        for link in &self.links {
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
        moment
            .checked_add(&payload_weight.checked_mul(&offset).ok()?)
            .ok()
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
        let mut push =
            |name: &str, value: Option<Quantity>, unit: &str, rule: &str, assumptions: &[&str]| {
                if let Some(value) = value {
                    derived.push(DerivedQuantity {
                        subject: arm.name.clone(),
                        name: name.into(),
                        display: display(&value, unit),
                        value,
                        status: EpistemicStatus::Derived,
                        rule: rule.into(),
                        assumptions: assumptions.iter().map(|a| (*a).into()).collect(),
                    });
                }
            };
        push(
            "reach",
            reach,
            "mm",
            "sum of link lengths",
            &["rigid links", "serial chain"],
        );
        push(
            "inner_radius",
            arm.inner_radius(),
            "mm",
            "|l1 - l2| for a 2-link arm",
            &["elbow can fold to +/-pi (not checked against joint limits)"],
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
    PhysicalReport {
        schema: "atlas.physical-report.v1".into(),
        evidence_level: "SEMANTIC_MODEL".into(),
        arms,
        derived,
        requirements,
        findings,
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
            Quantity::parse("4.535575625 N*m").unwrap()
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
