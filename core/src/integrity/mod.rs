//! The architectural integrity envelope and its evaluated report (G138, NA-INTEGRITY-ENVELOPE,
//! ADR 0056; `.atlas/contracts/ARCHITECTURAL-INTEGRITY.md`, construction milestones M5/M6).
//!
//! `derive_envelope` lowers the declared architecture Atlas can falsify with its census into typed
//! invariants (`architectural-integrity-envelope.schema.json`):
//! - `forall f: function in <A> forbid call to <B>` -> HARD `DEPENDENCY_DIRECTION`;
//! - `forbid effect` and `observed effect within` -> HARD `AUTHORITY_BOUNDARY`;
//! - `<A> ->depends_on-> <B>` between two Rust materializations (members the dependency census
//!   reads) -> HARD `DEPENDENCY_DIRECTION`; between entities the census cannot observe (no
//!   dependency census for their language) -> SOFT, `DIAGNOSTIC_ONLY`, and said so.
//!
//! The envelope is pinned by its identity, a digest of its invariants. `evaluate` checks one
//! candidate -- its decided constraint results and the envelope its ADL declares now -- against a
//! pinned envelope (`architectural-integrity-report.schema.json`): each pinned invariant PASS,
//! FAIL or UNKNOWN from the decided result it cites, and FAIL with
//! `ARCHITECTURE_UNSELECTED_REVISION` when the candidate no longer declares it, so no invariant
//! is weakened silently. Counts and verdict are derived from the evaluations, and
//! `check_report` re-derives them.

use crate::constraint::ConstraintVerdict;
use crate::identity::IntegrityDigest;
use crate::language::adl::{CensusForbidden, ConstraintCheck, ConstraintResult, DeclaredGraph};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const ENVELOPE_SCHEMA_VERSION: &str = "atlas.architectural-integrity-envelope.v1";
pub const REPORT_SCHEMA_VERSION: &str = "atlas.architectural-integrity-report.v1";
/// Where a repository pins its envelope.
pub const PINNED_ENVELOPE_PATH: &str = ".atlas/declared/integrity-envelope.json";
/// No Genome record is pinned yet (`DEBT-SELECTED_DESIGN`): said, never invented.
pub const GENOME_UNPINNED: &str = "UNPINNED (no Genome or SelectedDesign record exists yet)";

crate::vocabulary_enum! {
    /// An envelope's lifecycle.
    pub enum EnvelopeStatus {
        Active => "ACTIVE",
        Superseded => "SUPERSEDED",
        Draft => "DRAFT",
    }
}

crate::vocabulary_enum! {
    /// The contract's baseline invariant classes.
    pub enum InvariantClass {
        OwnershipBoundary => "OWNERSHIP_BOUNDARY",
        DependencyDirection => "DEPENDENCY_DIRECTION",
        AuthorityBoundary => "AUTHORITY_BOUNDARY",
        StateSourceOfTruth => "STATE_SOURCE_OF_TRUTH",
        InterfaceProtocol => "INTERFACE_PROTOCOL",
        LifecycleOrdering => "LIFECYCLE_ORDERING",
        TemporalOrdering => "TEMPORAL_ORDERING",
        FailureContainment => "FAILURE_CONTAINMENT",
        RecoveryInvariant => "RECOVERY_INVARIANT",
        PersistenceBoundary => "PERSISTENCE_BOUNDARY",
        ConcurrencyBoundary => "CONCURRENCY_BOUNDARY",
        SecurityTrustBoundary => "SECURITY_TRUST_BOUNDARY",
        ExternalBoundary => "EXTERNAL_BOUNDARY",
        ResourceSafetyBoundary => "RESOURCE_SAFETY_BOUNDARY",
    }
}

crate::vocabulary_enum! {
    pub enum Strength {
        Hard => "HARD",
        Soft => "SOFT",
    }
}

crate::vocabulary_enum! {
    pub enum ViolationAction {
        Reject => "REJECT",
        RequireReview => "REQUIRE_REVIEW",
        DiagnosticOnly => "DIAGNOSTIC_ONLY",
    }
}

crate::vocabulary_enum! {
    pub enum Classification {
        LoadBearing => "LOAD_BEARING",
        Structural => "STRUCTURAL",
        Replaceable => "REPLACEABLE",
        Decorative => "DECORATIVE",
    }
}

crate::vocabulary_enum! {
    /// One invariant's evaluation against one candidate.
    pub enum EvaluationStatus {
        Pass => "PASS",
        Fail => "FAIL",
        Unknown => "UNKNOWN",
        Conflict => "CONFLICT",
        NotAffected => "NOT_AFFECTED",
    }
}

crate::vocabulary_enum! {
    pub enum IntegrityVerdict {
        Eligible => "ELIGIBLE",
        Rejected => "REJECTED",
        Incomplete => "INCOMPLETE",
    }
}

crate::vocabulary_enum! {
    pub enum ArchitectureDiagnostic {
        HardViolation => "ARCHITECTURE_HARD_VIOLATION",
        RequiredUnknown => "ARCHITECTURE_REQUIRED_UNKNOWN",
        Conflict => "ARCHITECTURE_CONFLICT",
        ImpactClosureOpen => "ARCHITECTURE_IMPACT_CLOSURE_OPEN",
        EquivalenceUnproven => "ARCHITECTURE_EQUIVALENCE_UNPROVEN",
        UnselectedRevision => "ARCHITECTURE_UNSELECTED_REVISION",
        MaterializationViolation => "ARCHITECTURE_MATERIALIZATION_VIOLATION",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EnvelopeElement {
    pub element_ref: String,
    pub classification: Classification,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owner_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rationale: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EnvelopeInvariant {
    pub invariant_id: String,
    pub kind: InvariantClass,
    pub strength: Strength,
    pub subject_refs: Vec<String>,
    pub statement: String,
    /// The evaluator: `adl:<constraint result name>`.
    pub rule_ref: Option<String>,
    pub falsification_conditions: Vec<String>,
    pub violation_action: ViolationAction,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IntegrityEnvelope {
    pub schema_version: String,
    pub envelope_id: String,
    pub subject_ref: String,
    pub genome_ref: String,
    pub selected_design_ref: Option<String>,
    pub blueprint_revision_ref: Option<String>,
    pub status: EnvelopeStatus,
    pub elements: Vec<EnvelopeElement>,
    pub invariants: Vec<EnvelopeInvariant>,
    pub provenance_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ImpactClosureRef {
    pub closed: bool,
    pub affected_semantic_refs: Vec<String>,
    pub affected_invariant_refs: Vec<String>,
    pub closure_root: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InvariantEvaluation {
    pub invariant_ref: String,
    pub status: EvaluationStatus,
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostic_codes: Vec<ArchitectureDiagnostic>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IntegrityReport {
    pub schema_version: String,
    pub report_id: String,
    pub candidate_ref: String,
    pub materialization_ref: Option<String>,
    pub envelope_ref: String,
    pub observed_architecture_root: String,
    pub impact_closure: ImpactClosureRef,
    pub evaluations: Vec<InvariantEvaluation>,
    pub hard_violation_count: usize,
    pub required_unknown_count: usize,
    pub load_bearing_equivalence_closed: bool,
    pub blueprint_revision_ref: Option<String>,
    pub verdict: IntegrityVerdict,
    pub evidence_refs: Vec<String>,
}

fn rust_materialized(declared: &DeclaredGraph) -> BTreeSet<&str> {
    declared
        .materializations
        .iter()
        .filter(|m| m.source_language.as_deref() == Some("rust"))
        .map(|m| m.target.as_str())
        .collect()
}

/// The envelope `declared` states: every census-falsifiable declaration as a typed invariant,
/// every materialized entity as an element (LOAD_BEARING when a HARD invariant names it).
pub fn derive_envelope(subject_ref: &str, declared: &DeclaredGraph) -> IntegrityEnvelope {
    let mut invariants = Vec::new();
    let mut provenance: BTreeSet<String> = BTreeSet::new();
    for decl in declared.constraints.iter().chain(&declared.invariants) {
        let at = format!("{}:{}", decl.span.path, decl.span.line);
        for check in &decl.checks {
            let ConstraintCheck::CensusForbid {
                subsystem,
                forbidden,
            } = check
            else {
                continue;
            };
            let (kind, subjects, statement, falsifier) = match forbidden {
                CensusForbidden::CallTo { subsystem: target } => (
                    InvariantClass::DependencyDirection,
                    vec![subsystem.clone(), target.clone()],
                    format!("no function of {subsystem} calls a function of {target}"),
                    format!(
                        "a resolved call from a function of {subsystem} into a function of {target}"
                    ),
                ),
                CensusForbidden::Effect { category } => (
                    InvariantClass::AuthorityBoundary,
                    vec![subsystem.clone()],
                    format!("no function of {subsystem} performs {category}"),
                    format!("an OBSERVED or DERIVED {category} site in a function of {subsystem}"),
                ),
                CensusForbidden::ObservedEffectOutside { allowed } => {
                    let envelope = if allowed.is_empty() {
                        "no effect".to_owned()
                    } else {
                        allowed.join(", ")
                    };
                    (
                        InvariantClass::AuthorityBoundary,
                        vec![subsystem.clone()],
                        format!(
                            "every definite effect of {subsystem}'s functions is within {envelope}"
                        ),
                        format!(
                            "an OBSERVED or DERIVED effect site in a function of {subsystem} \
                             outside {envelope}"
                        ),
                    )
                }
            };
            provenance.insert(decl.span.path.clone());
            invariants.push(EnvelopeInvariant {
                invariant_id: format!("AIE:{}", decl.name),
                kind,
                strength: Strength::Hard,
                subject_refs: subjects,
                statement,
                rule_ref: Some(format!("adl:{}", decl.name)),
                falsification_conditions: vec![falsifier],
                violation_action: ViolationAction::Reject,
                evidence_refs: vec![at.clone()],
            });
        }
    }
    let rust = rust_materialized(declared);
    let mut notes = Vec::new();
    for edge in declared.edges.iter().filter(|e| e.relation == "depends_on") {
        let (from, to) = (&edge.from, &edge.to);
        let observable = rust.contains(from.as_str()) && rust.contains(to.as_str());
        provenance.insert(edge.span.path.clone());
        let name = format!("DeclaredDependency:{from}->{to}");
        if !observable {
            notes.push(format!(
                "{from} -> {to} is declared between entities the dependency census cannot observe \
                 (not both Rust materializations): SOFT, evaluated as UNKNOWN"
            ));
        }
        invariants.push(EnvelopeInvariant {
            invariant_id: format!("AIE:{name}"),
            kind: InvariantClass::DependencyDirection,
            strength: if observable {
                Strength::Hard
            } else {
                Strength::Soft
            },
            subject_refs: vec![from.clone(), to.clone()],
            statement: format!("{from} depends on {to}, and the dependency census observes it"),
            rule_ref: Some(format!("adl:{name}")),
            falsification_conditions: vec![format!(
                "no runtime or build dependency between the workspace members of {from} and {to}"
            )],
            violation_action: if observable {
                ViolationAction::Reject
            } else {
                ViolationAction::DiagnosticOnly
            },
            evidence_refs: vec![format!("{}:{}", edge.span.path, edge.span.line)],
        });
    }
    invariants.sort_by(|a, b| a.invariant_id.cmp(&b.invariant_id));
    let hard_subjects: BTreeSet<&str> = invariants
        .iter()
        .filter(|i| i.strength == Strength::Hard)
        .flat_map(|i| i.subject_refs.iter().map(String::as_str))
        .collect();
    let mut elements: Vec<EnvelopeElement> = declared
        .materializations
        .iter()
        .map(|m| {
            let load_bearing = hard_subjects.contains(m.target.as_str());
            EnvelopeElement {
                element_ref: m.target.clone(),
                classification: if load_bearing {
                    Classification::LoadBearing
                } else {
                    Classification::Structural
                },
                owner_refs: Vec::new(),
                rationale: if load_bearing {
                    "named by a HARD invariant".into()
                } else {
                    "materialized, named by no HARD invariant".into()
                },
                evidence_refs: vec![format!("{}:{}", m.span.path, m.span.line)],
            }
        })
        .collect();
    elements.sort_by(|a, b| a.element_ref.cmp(&b.element_ref));
    elements.dedup_by(|a, b| a.element_ref == b.element_ref);
    let mut envelope = IntegrityEnvelope {
        schema_version: ENVELOPE_SCHEMA_VERSION.into(),
        envelope_id: String::new(),
        subject_ref: subject_ref.into(),
        genome_ref: GENOME_UNPINNED.into(),
        selected_design_ref: None,
        blueprint_revision_ref: None,
        status: EnvelopeStatus::Active,
        elements,
        invariants,
        provenance_refs: if provenance.is_empty() {
            vec!["no declaration".into()]
        } else {
            provenance.into_iter().collect()
        },
        notes,
    };
    envelope.envelope_id = envelope_identity(&envelope);
    envelope
}

/// The envelope's identity: a digest of its invariants' canonical text, so any added, removed or
/// changed invariant changes it.
pub fn envelope_identity(envelope: &IntegrityEnvelope) -> String {
    let mut text = String::new();
    for i in &envelope.invariants {
        text.push_str(&format!(
            "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1e}",
            i.invariant_id,
            i.kind,
            i.strength,
            i.subject_refs.join("\u{1d}"),
            i.statement,
            i.rule_ref.as_deref().unwrap_or(""),
            i.falsification_conditions.join("\u{1d}"),
            i.violation_action
        ));
    }
    format!(
        "envelope:{}",
        IntegrityDigest::of_bytes(text.as_bytes()).as_str()
    )
}

/// Evaluate `pinned` against one candidate: `current` is the envelope the candidate's ADL declares
/// now, `results` its decided constraint results (census-quantified invariants decided over the
/// composed census, dependencies reconciled), `observed_root` its census digest.
pub fn evaluate(
    pinned: &IntegrityEnvelope,
    current: &IntegrityEnvelope,
    results: &[ConstraintResult],
    candidate_ref: &str,
    observed_root: &str,
) -> IntegrityReport {
    let by_name: BTreeMap<&str, &ConstraintResult> =
        results.iter().map(|r| (r.name.as_str(), r)).collect();
    let current_by_id: BTreeMap<&str, &EnvelopeInvariant> = current
        .invariants
        .iter()
        .map(|i| (i.invariant_id.as_str(), i))
        .collect();
    let mut evaluations = Vec::new();
    for invariant in &pinned.invariants {
        let hard = invariant.strength == Strength::Hard;
        let rule = invariant.rule_ref.as_deref().unwrap_or("");
        let mut evidence = vec![invariant.invariant_id.clone()];
        // A pinned invariant the candidate no longer declares, or declares differently.
        if current_by_id.get(invariant.invariant_id.as_str()) != Some(&invariant) {
            evaluations.push(InvariantEvaluation {
                invariant_ref: invariant.invariant_id.clone(),
                status: EvaluationStatus::Fail,
                evidence_refs: evidence,
                diagnostic_codes: vec![ArchitectureDiagnostic::UnselectedRevision],
                details: "the candidate no longer declares this pinned invariant as pinned; \
                          changing it needs a selected blueprint revision and a new pin"
                    .into(),
            });
            continue;
        }
        let result = rule.strip_prefix("adl:").and_then(|n| by_name.get(n));
        let (status, details) = match result {
            None => (
                EvaluationStatus::Unknown,
                format!("no decided result for {rule}: the census could not evaluate it"),
            ),
            Some(r) => {
                evidence.push(rule.to_owned());
                let said: Vec<String> = r
                    .diagnostics
                    .iter()
                    .map(|d| format!("{}: {}", d.code, d.message))
                    .chain(r.derivation.iter().flat_map(|d| d.basis.iter().cloned()))
                    .collect();
                let status = match r.verdict {
                    ConstraintVerdict::Satisfied => EvaluationStatus::Pass,
                    ConstraintVerdict::Violated => EvaluationStatus::Fail,
                    ConstraintVerdict::Unknown => EvaluationStatus::Unknown,
                };
                (status, said.join("; "))
            }
        };
        let diagnostic_codes = match (status, hard) {
            (EvaluationStatus::Fail, true) => vec![ArchitectureDiagnostic::HardViolation],
            (EvaluationStatus::Unknown, true) => vec![ArchitectureDiagnostic::RequiredUnknown],
            _ => Vec::new(),
        };
        evaluations.push(InvariantEvaluation {
            invariant_ref: invariant.invariant_id.clone(),
            status,
            evidence_refs: evidence,
            diagnostic_codes,
            details,
        });
    }
    let (hard_violation_count, required_unknown_count) = counts(&evaluations);
    let affected: Vec<String> = pinned
        .invariants
        .iter()
        .map(|i| i.invariant_id.clone())
        .collect();
    let mut report = IntegrityReport {
        schema_version: REPORT_SCHEMA_VERSION.into(),
        report_id: String::new(),
        candidate_ref: candidate_ref.into(),
        materialization_ref: None,
        envelope_ref: pinned.envelope_id.clone(),
        observed_architecture_root: observed_root.into(),
        // Every pinned invariant is evaluated over the full census: the closure is closed by
        // full recompute, not by an incremental closure.
        impact_closure: ImpactClosureRef {
            closed: true,
            affected_semantic_refs: Vec::new(),
            affected_invariant_refs: affected,
            closure_root: Some("FULL_RECOMPUTE".into()),
        },
        evaluations,
        hard_violation_count,
        required_unknown_count,
        // No load-bearing replacement is under evaluation.
        load_bearing_equivalence_closed: true,
        blueprint_revision_ref: None,
        verdict: verdict(hard_violation_count, required_unknown_count),
        evidence_refs: vec![pinned.envelope_id.clone(), observed_root.into()],
    };
    report.report_id = format!(
        "report:{}",
        IntegrityDigest::of_bytes(
            format!(
                "{}\u{1f}{}\u{1f}{}",
                report.envelope_ref, report.candidate_ref, report.observed_architecture_root
            )
            .as_bytes()
        )
        .as_str()
    );
    report
}

fn counts(evaluations: &[InvariantEvaluation]) -> (usize, usize) {
    let with = |code: ArchitectureDiagnostic| {
        evaluations
            .iter()
            .filter(|e| e.diagnostic_codes.contains(&code))
            .count()
    };
    (
        with(ArchitectureDiagnostic::HardViolation)
            + with(ArchitectureDiagnostic::UnselectedRevision),
        with(ArchitectureDiagnostic::RequiredUnknown),
    )
}

fn verdict(hard_violations: usize, required_unknowns: usize) -> IntegrityVerdict {
    if hard_violations > 0 {
        IntegrityVerdict::Rejected
    } else if required_unknowns > 0 {
        IntegrityVerdict::Incomplete
    } else {
        IntegrityVerdict::Eligible
    }
}

/// Re-derive a report's summary fields from its evaluations (the contract's internal-consistency
/// rule): every mismatch, named.
pub fn check_report(report: &IntegrityReport, pinned: &IntegrityEnvelope) -> Vec<String> {
    let mut problems = Vec::new();
    let (hard, unknown) = counts(&report.evaluations);
    if report.hard_violation_count != hard {
        problems.push(format!(
            "hard_violation_count {} but {hard} evaluations carry a hard violation",
            report.hard_violation_count
        ));
    }
    if report.required_unknown_count != unknown {
        problems.push(format!(
            "required_unknown_count {} but {unknown} evaluations are required unknowns",
            report.required_unknown_count
        ));
    }
    if report.verdict != verdict(hard, unknown) {
        problems.push(format!(
            "verdict {} does not follow from the evaluations",
            report.verdict
        ));
    }
    if report.envelope_ref != pinned.envelope_id || envelope_identity(pinned) != pinned.envelope_id
    {
        problems.push("the report does not cite the pinned envelope's identity".into());
    }
    let evaluated: BTreeSet<&str> = report
        .evaluations
        .iter()
        .map(|e| e.invariant_ref.as_str())
        .collect();
    for invariant in &pinned.invariants {
        if !evaluated.contains(invariant.invariant_id.as_str()) {
            problems.push(format!("{} is not evaluated", invariant.invariant_id));
        }
    }
    problems
}

#[cfg(test)]
mod tests;
