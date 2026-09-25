//! Verification obligations, evidence and reports (G127, NA-VERIFICATION-EVIDENCE, construction
//! node M2; `contracts/VERIFICATION-METRICS-PERFORMANCE.md`, ADR 0048).
//!
//! An obligation is what must hold; evidence is what a producer observed about it for one exact
//! candidate; a report is a plan's obligations evaluated against the evidence. An obligation's
//! state is a function of its admissible evidence set, never of one record alone:
//!
//! - no admissible evidence -> `UNVERIFIED` (never `SATISFIED`);
//! - any admissible evidence `VIOLATED` -> `FAILED`, with a `VerificationFailure`;
//! - every admissible evidence `SATISFIED` -> `SATISFIED`;
//! - otherwise -> `UNRESOLVED`.
//!
//! Evidence is admissible only for the exact candidate identity it names. A policy, keyed by
//! artifact kind, names the classes a candidate must cover; a required class without an obligation
//! is a coverage gap, not a pass. Coverage stays per class, never one percentage.

use crate::EpistemicStatus;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const VERIFICATION_REPORT_SCHEMA: &str = "atlas.verification-report.v1";

/// The extensible, named verification classes the contract materializes today.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationClass {
    Semantic,
    Static,
    Unit,
    Property,
    Fuzz,
    Integration,
    Scenario,
    Compatibility,
    Failure,
    Security,
    Benchmark,
}

impl VerificationClass {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Semantic => "SEMANTIC",
            Self::Static => "STATIC",
            Self::Unit => "UNIT",
            Self::Property => "PROPERTY",
            Self::Fuzz => "FUZZ",
            Self::Integration => "INTEGRATION",
            Self::Scenario => "SCENARIO",
            Self::Compatibility => "COMPATIBILITY",
            Self::Failure => "FAILURE",
            Self::Security => "SECURITY",
            Self::Benchmark => "BENCHMARK",
        }
    }
}

/// A property that must hold for a candidate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationObligation {
    pub id: String,
    pub class: VerificationClass,
    pub subject: String,
    pub statement: String,
    pub required: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceResult {
    Satisfied,
    Violated,
    Unverified,
    Unresolved,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Producer {
    pub tool: String,
    pub version: String,
}

/// What one producer observed, for one exact candidate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationEvidence {
    pub id: String,
    pub obligations: Vec<String>,
    pub producer: Producer,
    pub run_id: String,
    /// The exact candidate identity under test (a census digest or a revision).
    pub candidate: String,
    /// Where it ran, including whether the run was sandboxed.
    pub environment: String,
    pub inputs: Vec<String>,
    pub result: EvidenceResult,
    pub content_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counterexample: Option<String>,
    /// OBSERVED for a run that happened; a provider's summary of a hypothetical run is never
    /// evidence.
    pub status: EpistemicStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ObligationState {
    Satisfied,
    Failed,
    Unverified,
    Unresolved,
}

impl ObligationState {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Satisfied => "SATISFIED",
            Self::Failed => "FAILED",
            Self::Unverified => "UNVERIFIED",
            Self::Unresolved => "UNRESOLVED",
        }
    }
}

/// The typed record a failed obligation hands back to repair.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationFailure {
    pub obligation: String,
    pub counterexample: Option<String>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObligationOutcome {
    pub obligation: String,
    pub class: VerificationClass,
    pub required: bool,
    pub state: ObligationState,
    /// The admissible evidence the state rests on.
    pub evidence: Vec<String>,
    /// Evidence naming this obligation but another candidate: never counted.
    pub inadmissible: Vec<String>,
}

/// The classes a candidate of one artifact kind must cover.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationPolicy {
    pub artifact_kind: String,
    pub required_classes: Vec<VerificationClass>,
}

impl VerificationPolicy {
    /// The contract's minimal library-package policy: SEMANTIC, UNIT and COMPATIBILITY.
    pub fn library_package() -> Self {
        Self {
            artifact_kind: "LIBRARY_PACKAGE".into(),
            required_classes: vec![
                VerificationClass::Semantic,
                VerificationClass::Unit,
                VerificationClass::Compatibility,
            ],
        }
    }
}

/// The planned obligations a policy requires for one candidate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationPlan {
    pub policy: VerificationPolicy,
    pub candidate: String,
    pub obligations: Vec<VerificationObligation>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClassCoverage {
    pub obligations: usize,
    pub satisfied: usize,
    pub failed: usize,
    pub unverified: usize,
    pub unresolved: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationReport {
    pub schema: String,
    pub policy: VerificationPolicy,
    pub candidate: String,
    pub outcomes: Vec<ObligationOutcome>,
    /// Per class, never collapsed into one number.
    pub coverage: BTreeMap<VerificationClass, ClassCoverage>,
    pub failures: Vec<VerificationFailure>,
    /// `ADMISSIBLE` only when every required class has an obligation and every required
    /// obligation is `SATISFIED`; otherwise `BLOCKED`.
    pub verdict: String,
    pub blockers: Vec<String>,
}

/// The state of one obligation from the evidence admissible for `candidate`.
pub fn evaluate_obligation(
    obligation: &VerificationObligation,
    candidate: &str,
    evidence: &[VerificationEvidence],
) -> (ObligationOutcome, Option<VerificationFailure>) {
    let naming: Vec<&VerificationEvidence> = evidence
        .iter()
        .filter(|e| e.obligations.contains(&obligation.id))
        .collect();
    let (admissible, inadmissible): (Vec<&VerificationEvidence>, Vec<&VerificationEvidence>) =
        naming
            .into_iter()
            .partition(|e| e.candidate == candidate && e.status == EpistemicStatus::Observed);
    let state = if admissible.is_empty() {
        ObligationState::Unverified
    } else if admissible
        .iter()
        .any(|e| e.result == EvidenceResult::Violated)
    {
        ObligationState::Failed
    } else if admissible
        .iter()
        .all(|e| e.result == EvidenceResult::Satisfied)
    {
        ObligationState::Satisfied
    } else {
        ObligationState::Unresolved
    };
    let failure = (state == ObligationState::Failed).then(|| VerificationFailure {
        obligation: obligation.id.clone(),
        counterexample: admissible
            .iter()
            .filter(|e| e.result == EvidenceResult::Violated)
            .find_map(|e| e.counterexample.clone()),
        evidence: admissible
            .iter()
            .filter(|e| e.result == EvidenceResult::Violated)
            .map(|e| e.id.clone())
            .collect(),
    });
    (
        ObligationOutcome {
            obligation: obligation.id.clone(),
            class: obligation.class,
            required: obligation.required,
            state,
            evidence: admissible.iter().map(|e| e.id.clone()).collect(),
            inadmissible: inadmissible.iter().map(|e| e.id.clone()).collect(),
        },
        failure,
    )
}

/// Evaluate a plan against evidence.
pub fn evaluate(plan: &VerificationPlan, evidence: &[VerificationEvidence]) -> VerificationReport {
    let mut outcomes = Vec::new();
    let mut failures = Vec::new();
    let mut coverage: BTreeMap<VerificationClass, ClassCoverage> = BTreeMap::new();
    for class in &plan.policy.required_classes {
        coverage.entry(*class).or_default();
    }
    for obligation in &plan.obligations {
        let (outcome, failure) = evaluate_obligation(obligation, &plan.candidate, evidence);
        let tally = coverage.entry(obligation.class).or_default();
        tally.obligations += 1;
        match outcome.state {
            ObligationState::Satisfied => tally.satisfied += 1,
            ObligationState::Failed => tally.failed += 1,
            ObligationState::Unverified => tally.unverified += 1,
            ObligationState::Unresolved => tally.unresolved += 1,
        }
        outcomes.push(outcome);
        failures.extend(failure);
    }
    let mut blockers = Vec::new();
    for class in &plan.policy.required_classes {
        if coverage.get(class).is_none_or(|c| c.obligations == 0) {
            blockers.push(format!(
                "required class {} has no obligation: a required obligation may not disappear",
                class.as_str()
            ));
        }
    }
    for outcome in &outcomes {
        if outcome.required && outcome.state != ObligationState::Satisfied {
            blockers.push(format!(
                "{} is {}",
                outcome.obligation,
                outcome.state.as_str()
            ));
        }
    }
    VerificationReport {
        schema: VERIFICATION_REPORT_SCHEMA.into(),
        policy: plan.policy.clone(),
        candidate: plan.candidate.clone(),
        outcomes,
        coverage,
        failures,
        verdict: if blockers.is_empty() {
            "ADMISSIBLE".into()
        } else {
            "BLOCKED".into()
        },
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obligation(id: &str, class: VerificationClass) -> VerificationObligation {
        VerificationObligation {
            id: id.into(),
            class,
            subject: "self".into(),
            statement: id.into(),
            required: true,
        }
    }

    fn evidence(
        id: &str,
        obligation: &str,
        candidate: &str,
        result: EvidenceResult,
    ) -> VerificationEvidence {
        VerificationEvidence {
            id: id.into(),
            obligations: vec![obligation.into()],
            producer: Producer {
                tool: "test".into(),
                version: "1".into(),
            },
            run_id: id.into(),
            candidate: candidate.into(),
            environment: "test".into(),
            inputs: Vec::new(),
            result,
            content_hash: id.into(),
            counterexample: (result == EvidenceResult::Violated).then(|| "case 7".into()),
            status: EpistemicStatus::Observed,
        }
    }

    fn plan() -> VerificationPlan {
        VerificationPlan {
            policy: VerificationPolicy::library_package(),
            candidate: "c1".into(),
            obligations: vec![
                obligation("SEM", VerificationClass::Semantic),
                obligation("UNIT", VerificationClass::Unit),
                obligation("COMPAT", VerificationClass::Compatibility),
            ],
        }
    }

    #[test]
    fn nothing_is_satisfied_without_admissible_evidence() {
        let report = evaluate(&plan(), &[]);
        assert!(
            report
                .outcomes
                .iter()
                .all(|o| o.state == ObligationState::Unverified)
        );
        assert_eq!(report.verdict, "BLOCKED");
        // Evidence for another candidate, or not observed, is never counted.
        let other = evidence("e1", "SEM", "c2", EvidenceResult::Satisfied);
        let mut hypothetical = evidence("e2", "SEM", "c1", EvidenceResult::Satisfied);
        hypothetical.status = EpistemicStatus::Hypothesis;
        let report = evaluate(&plan(), &[other, hypothetical]);
        let sem = &report.outcomes[0];
        assert_eq!(sem.state, ObligationState::Unverified);
        assert_eq!(sem.inadmissible, ["e1", "e2"]);
    }

    #[test]
    fn state_is_a_function_of_the_admissible_evidence_set() {
        let all = [
            evidence("s", "SEM", "c1", EvidenceResult::Satisfied),
            evidence("u1", "UNIT", "c1", EvidenceResult::Satisfied),
            evidence("u2", "UNIT", "c1", EvidenceResult::Violated),
            evidence("c", "COMPAT", "c1", EvidenceResult::Unresolved),
        ];
        let report = evaluate(&plan(), &all);
        let states: Vec<ObligationState> = report.outcomes.iter().map(|o| o.state).collect();
        assert_eq!(
            states,
            [
                ObligationState::Satisfied,
                ObligationState::Failed,
                ObligationState::Unresolved
            ]
        );
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures[0].counterexample.as_deref(), Some("case 7"));
        assert_eq!(report.coverage[&VerificationClass::Unit].failed, 1);
        assert_eq!(report.verdict, "BLOCKED");
    }

    #[test]
    fn a_required_class_without_an_obligation_blocks_and_all_satisfied_admits() {
        let mut partial = plan();
        partial
            .obligations
            .retain(|o| o.class != VerificationClass::Compatibility);
        let satisfied = [
            evidence("s", "SEM", "c1", EvidenceResult::Satisfied),
            evidence("u", "UNIT", "c1", EvidenceResult::Satisfied),
            evidence("c", "COMPAT", "c1", EvidenceResult::Satisfied),
        ];
        let report = evaluate(&partial, &satisfied);
        assert_eq!(report.verdict, "BLOCKED");
        assert!(report.blockers[0].contains("COMPATIBILITY has no obligation"));
        assert_eq!(evaluate(&plan(), &satisfied).verdict, "ADMISSIBLE");
    }
}
