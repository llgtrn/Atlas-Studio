//! Candidate designs compared by explicit measured criteria (G150, ADR 0066).
//!
//! A comparison puts two or more designs side by side. They must share one coordinate and one
//! verified container, and each must be valid on its own. Each design is measured by criteria its
//! author declares: a measure over the container's typed records attributed to the design's
//! roots, plus a direction and a stated meaning. The measure is never a number the author
//! supplies. The comparison keeps the Pareto front as a set. It ranks nothing and selects
//! nothing:
//! - a candidate that is not fully measured does not compete;
//! - a dominated candidate names the candidates that dominate it.
//!
//! A SELECTED design must cite a comparison of at least two measured candidates in which it is
//! not dominated. This stops a design from being "selected" because only one candidate existed,
//! or against the criteria its own comparison declared (`contracts/SELECTED-DESIGN.md`).

use super::{DesignState, DesignViolation, SelectedDesign, SemanticRoot, validate, violation};
use crate::SemanticObservation;
use crate::atlas::CensusAtlas;
use crate::identity::IntegrityDigest;
use crate::semantic::SemanticDimension;
use crate::verification::VerificationReport;
use crate::visual::search::{Direction, Objective, dominates, pareto_front};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const COMPARISON_SCHEMA_VERSION: &str = "atlas.design-comparison.v1";

/// What a criterion counts over a design's roots in the container.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Measure {
    /// The number of roots the design selects.
    Roots,
    /// Distinct records of a per-function dimension whose function is a root.
    Records(SemanticDimension),
    /// Call sites in a root function that no engine resolved to a callee.
    UnresolvedCalls,
}

/// The dimensions whose records name the function they belong to.
const ATTRIBUTABLE: [SemanticDimension; 8] = [
    SemanticDimension::Call,
    SemanticDimension::ControlFlow,
    SemanticDimension::DataFlow,
    SemanticDimension::State,
    SemanticDimension::Effect,
    SemanticDimension::Ownership,
    SemanticDimension::Concurrency,
    SemanticDimension::Persistence,
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Criterion {
    pub name: String,
    pub measure: Measure,
    pub direction: Direction,
    /// What the number is and is not.
    pub meaning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidateMeasurement {
    pub design_id: String,
    /// One value per criterion, or none when a criterion is undefined for this design.
    pub values: Option<Vec<u64>>,
    pub unmeasured: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dominance {
    pub design_id: String,
    pub dominated_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesignComparison {
    pub schema: String,
    /// `comparison_identity` of the rest.
    pub comparison_id: String,
    pub parent_root: String,
    pub candidate: String,
    pub scope: String,
    pub target_kind: String,
    pub variant: String,
    pub criteria: Vec<Criterion>,
    /// Sorted by design id.
    pub measurements: Vec<CandidateMeasurement>,
    /// The non-dominated measured candidates: a set, never a ranking.
    pub front: Vec<String>,
    pub dominated: Vec<Dominance>,
    pub unmeasured: Vec<String>,
}

impl DesignComparison {
    pub fn candidate_ids(&self) -> Vec<String> {
        self.measurements
            .iter()
            .map(|m| m.design_id.clone())
            .collect()
    }
}

/// A comparison's identity: BLAKE3 over its canonical serde form with the identity left empty.
pub fn comparison_identity(comparison: &DesignComparison) -> String {
    let mut unstamped = comparison.clone();
    unstamped.comparison_id = String::new();
    let text = serde_json::to_string(&unstamped).expect("a DesignComparison always serializes");
    IntegrityDigest::of_bytes(text.as_bytes())
        .as_str()
        .to_owned()
}

fn function_of(record: &SemanticObservation) -> Option<&str> {
    Some(match record {
        SemanticObservation::Call(h) => h.subject.function.as_str(),
        SemanticObservation::ControlFlow(h) => h.subject.function.as_str(),
        SemanticObservation::DataFlow(h) => h.subject.function.as_str(),
        SemanticObservation::State(h) => h.subject.function.as_str(),
        SemanticObservation::Effect(h) => h.subject.function.as_str(),
        SemanticObservation::Ownership(h) => h.subject.function.as_str(),
        SemanticObservation::Concurrency(h) => h.subject.function.as_str(),
        SemanticObservation::Persistence(h) => h.subject.function.as_str(),
        _ => return None,
    })
}

/// A design's value for each criterion, or why it cannot be measured.
fn measure(
    roots: &[SemanticRoot],
    container: &CensusAtlas,
    criteria: &[Criterion],
) -> Result<Vec<u64>, String> {
    let functions: BTreeSet<&str> = roots
        .iter()
        .filter(|r| r.dimension == SemanticDimension::FunctionIdentity)
        .map(|r| r.record_id.as_str())
        .collect();
    let mut values = Vec::new();
    for criterion in criteria {
        let per_function = !matches!(criterion.measure, Measure::Roots);
        if per_function && functions.len() != roots.len() {
            return Err(format!(
                "{} counts records of root functions, and a root is not a FUNCTION_IDENTITY",
                criterion.name
            ));
        }
        let attributed = || {
            container
                .typed_records
                .iter()
                .filter(|r| function_of(r).is_some_and(|f| functions.contains(f)))
        };
        let value = match criterion.measure {
            Measure::Roots => roots.len(),
            Measure::Records(dimension) => attributed()
                .filter(|r| r.dimension() == dimension)
                .map(|r| r.record_id().as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            Measure::UnresolvedCalls => {
                // One site may carry a syntactic and a resolution record under one id; it is
                // resolved when any of them names a callee.
                let mut sites: BTreeMap<&str, bool> = BTreeMap::new();
                for record in attributed() {
                    if let SemanticObservation::Call(h) = record {
                        *sites.entry(h.record_id.as_str()).or_default() |=
                            !h.subject.callees.is_empty();
                    }
                }
                sites.values().filter(|resolved| !**resolved).count()
            }
        };
        values.push(value as u64);
    }
    Ok(values)
}

/// Compares `designs` over the verified `container` (root identity `root_id`) by `criteria`.
/// `report` is the verification report for the container's candidate, which VALIDATED designs
/// need. Returns every reason the comparison cannot be made.
pub fn compare_designs(
    designs: &[SelectedDesign],
    container: &CensusAtlas,
    root_id: &str,
    report: Option<&VerificationReport>,
    criteria: &[Criterion],
) -> Result<DesignComparison, Vec<DesignViolation>> {
    let mut out = Vec::new();
    if criteria.is_empty() {
        out.push(violation(
            "NO_CRITERIA",
            "a comparison declares its criteria",
        ));
    }
    let mut names = BTreeSet::new();
    for criterion in criteria {
        if !names.insert(criterion.name.as_str()) {
            out.push(violation("CRITERION_DUPLICATE", &criterion.name));
        }
        if let Measure::Records(dimension) = criterion.measure
            && !ATTRIBUTABLE.contains(&dimension)
        {
            out.push(violation(
                "MEASURE_UNDEFINED",
                format!(
                    "{}: {} records do not name a function",
                    criterion.name,
                    dimension.as_str()
                ),
            ));
        }
    }
    let ids: BTreeSet<String> = designs.iter().map(|d| d.design_id.clone()).collect();
    if ids.len() != designs.len() {
        out.push(violation(
            "DUPLICATE_CANDIDATE",
            "a design is compared once",
        ));
    }
    if ids.len() < 2 {
        out.push(violation(
            "TOO_FEW_CANDIDATES",
            format!("{} distinct designs; a comparison needs two", ids.len()),
        ));
    }
    let registry = super::PrincipalRegistry::empty();
    for design in designs {
        let id = &design.design_id;
        let coordinate = |d: &SelectedDesign| {
            (
                d.parent_root.clone(),
                d.scope.clone(),
                d.target_kind.clone(),
                d.variant.clone(),
            )
        };
        if coordinate(design) != coordinate(&designs[0]) {
            out.push(violation(
                "COORDINATE_MIXED",
                format!("{id} is at another coordinate or container"),
            ));
        }
        if !matches!(
            design.state,
            DesignState::Candidate | DesignState::Validated
        ) {
            out.push(violation(
                "STATE_NOT_COMPARABLE",
                format!("{id} is {}", design.state.as_str()),
            ));
        }
        for v in validate(design, container, root_id, report, &registry) {
            out.push(violation(
                "CANDIDATE_INVALID",
                format!("{id}: {} {}", v.code, v.detail),
            ));
        }
        let set: BTreeSet<String> = design.candidate_set.iter().cloned().collect();
        if set != ids {
            out.push(violation(
                "CANDIDATE_SET_MISMATCH",
                format!("{id} does not name exactly the compared designs"),
            ));
        }
    }
    if !out.is_empty() {
        out.sort();
        out.dedup();
        return Err(out);
    }

    let mut sorted: Vec<&SelectedDesign> = designs.iter().collect();
    sorted.sort_by(|a, b| a.design_id.cmp(&b.design_id));
    let measurements: Vec<CandidateMeasurement> = sorted
        .iter()
        .map(|d| match measure(&d.roots, container, criteria) {
            Ok(values) => CandidateMeasurement {
                design_id: d.design_id.clone(),
                values: Some(values),
                unmeasured: None,
            },
            Err(why) => CandidateMeasurement {
                design_id: d.design_id.clone(),
                values: None,
                unmeasured: Some(why),
            },
        })
        .collect();
    let objectives: Vec<Objective> = criteria
        .iter()
        .map(|c| Objective {
            name: c.name.clone(),
            direction: c.direction,
            meaning: c.meaning.clone(),
        })
        .collect();
    let measured: Vec<(&str, Vec<f64>)> = measurements
        .iter()
        .filter_map(|m| {
            let values = m.values.as_ref()?;
            Some((
                m.design_id.as_str(),
                values.iter().map(|&v| v as f64).collect(),
            ))
        })
        .collect();
    let scores: Vec<Vec<f64>> = measured.iter().map(|(_, s)| s.clone()).collect();
    let front: Vec<String> = pareto_front(&scores, &objectives)
        .into_iter()
        .map(|i| measured[i].0.to_owned())
        .collect();
    let dominated: Vec<Dominance> = measured
        .iter()
        .filter_map(|(id, score)| {
            let by: Vec<String> = measured
                .iter()
                .filter(|(other, s)| other != id && dominates(s, score, &objectives))
                .map(|(other, _)| (*other).to_owned())
                .collect();
            (!by.is_empty()).then(|| Dominance {
                design_id: (*id).to_owned(),
                dominated_by: by,
            })
        })
        .collect();
    let unmeasured = measurements
        .iter()
        .filter(|m| m.values.is_none())
        .map(|m| m.design_id.clone())
        .collect();
    let first = sorted[0];
    let mut comparison = DesignComparison {
        schema: COMPARISON_SCHEMA_VERSION.into(),
        comparison_id: String::new(),
        parent_root: first.parent_root.clone(),
        candidate: first.candidate.clone(),
        scope: first.scope.clone(),
        target_kind: first.target_kind.clone(),
        variant: first.variant.clone(),
        criteria: criteria.to_vec(),
        measurements,
        front,
        dominated,
        unmeasured,
    };
    comparison.comparison_id = comparison_identity(&comparison);
    Ok(comparison)
}

/// Every reason `comparison` does not support selecting `design`. The design must cite this
/// comparison, name exactly its candidates, be at its coordinate, and sit on its front among at
/// least two measured candidates.
pub fn validate_selection(
    design: &SelectedDesign,
    comparison: &DesignComparison,
) -> Vec<DesignViolation> {
    let mut out = Vec::new();
    let identity = comparison_identity(comparison);
    if comparison.comparison_id != identity {
        out.push(violation(
            "COMPARISON_ID_MISMATCH",
            format!("recorded {}, computed {identity}", comparison.comparison_id),
        ));
    }
    if design.comparison.as_deref() != Some(comparison.comparison_id.as_str()) {
        out.push(violation(
            "COMPARISON_OTHER",
            "the design does not cite this comparison",
        ));
    }
    let at = (
        &comparison.parent_root,
        &comparison.scope,
        &comparison.target_kind,
        &comparison.variant,
    );
    if at
        != (
            &design.parent_root,
            &design.scope,
            &design.target_kind,
            &design.variant,
        )
    {
        out.push(violation(
            "COMPARISON_OTHER_COORDINATE",
            "the comparison is of another coordinate or container",
        ));
    }
    let ids = comparison.candidate_ids();
    let measured = comparison
        .measurements
        .iter()
        .filter(|m| m.values.is_some())
        .count();
    if measured < 2 {
        out.push(violation(
            "FEWER_THAN_TWO_MEASURED",
            format!("{measured} measured candidates: selection needs a real alternative"),
        ));
    }
    let set: BTreeSet<&String> = design.candidate_set.iter().collect();
    if set != ids.iter().collect() {
        out.push(violation(
            "CANDIDATE_SET_MISMATCH",
            "the design's candidate set is not the compared set",
        ));
    }
    if !ids.contains(&design.design_id) {
        out.push(violation("DESIGN_NOT_COMPARED", &design.design_id));
    } else if comparison.unmeasured.contains(&design.design_id) {
        out.push(violation("DESIGN_UNMEASURED", &design.design_id));
    } else if let Some(d) = comparison
        .dominated
        .iter()
        .find(|d| d.design_id == design.design_id)
    {
        out.push(violation(
            "DESIGN_DOMINATED",
            format!("dominated by {}", d.dominated_by.join(", ")),
        ));
    } else if !comparison.front.contains(&design.design_id) {
        out.push(violation("DESIGN_NOT_ON_FRONT", &design.design_id));
    }
    out.sort();
    out
}
