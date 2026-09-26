//! The one epistemic vocabulary (G134, NA-EPISTEMIC-UNIFICATION, ADR 0053).
//!
//! `EpistemicStatus` says what is known about a claim and how. Every other type that carries a
//! status, verdict, outcome, state or level is listed here with its role, so no domain grows a
//! second vocabulary unnoticed: a test walks the workspace's struct fields with a status-like name
//! and rejects any whose type is not mapped here (a float confidence and a free-string status were
//! found this way and removed). Text at a boundary -- a record decoded from a container, a
//! snapshot, a document's front matter -- is listed with the place it is parsed or produced.
//!
//! Futures states decided here: OBSERVED, DERIVED, SIMULATED and HYPOTHESIS are statuses;
//! PREDICTED is a `SIMULATED` record (a simulation run), not a status; VALIDATED, FALSIFIED and
//! STILL_HYPOTHESIZED are the outcomes of a hypothesis record (`HypothesisOutcome`), not statuses;
//! COUNTERFACTUAL is a record kind with no consumer yet; MEASURED and CALIBRATED wait for
//! telemetry. VALIDATED has one meaning per carrier: a hypothesis outcome, a product evidence
//! basis, a physical evidence rung (FIELD_VALIDATED) -- never an epistemic status.

/// A closed set of named values that serializes exactly to its `SCREAMING_SNAKE_CASE` names,
/// compares equal to them, and displays as them: a typed carrier that stays wire-compatible with
/// the strings it replaces.
#[macro_export]
macro_rules! vocabulary_enum {
    ($(#[$meta:meta])* $vis:vis enum $name:ident { $($(#[$vmeta:meta])* $variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis enum $name {
            $($(#[$vmeta])* #[serde(rename = $text)] $variant),+
        }

        impl $name {
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }
    };
}

/// What a status-bearing type is, relative to the one vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarrierRole {
    /// The vocabulary itself.
    Epistemic,
    /// A three-valued (or finer) decision over claims; not a knowledge status.
    Verdict,
    /// The state of a process or record lifecycle, computed from evidence.
    Lifecycle,
    /// Whether a mechanism was applied (sandbox isolation).
    Enforcement,
    /// A ladder of evidence kinds with a declared mapping onto `EpistemicStatus`.
    EvidenceLadder,
    /// The outcome of a record (a hypothesis, a run, a resolution).
    Outcome,
}

/// One mapped carrier type.
#[derive(Debug, Clone, Copy)]
pub struct Carrier {
    pub type_name: &'static str,
    pub role: CarrierRole,
    pub meaning: &'static str,
}

const fn carrier(type_name: &'static str, role: CarrierRole, meaning: &'static str) -> Carrier {
    Carrier {
        type_name,
        role,
        meaning,
    }
}

/// Every type allowed to carry a status, verdict, outcome, state or level.
pub const CARRIERS: &[Carrier] = &[
    carrier(
        "EpistemicStatus",
        CarrierRole::Epistemic,
        "what is known about a claim and how",
    ),
    carrier(
        "ConstraintVerdict",
        CarrierRole::Verdict,
        "a constraint or requirement decided over claims (ADR 0007)",
    ),
    carrier(
        "Verdict",
        CarrierRole::Verdict,
        "a self-recensus generation proven or not",
    ),
    carrier(
        "EvidenceResult",
        CarrierRole::Verdict,
        "one verification evidence item's result",
    ),
    carrier(
        "ReportVerdict",
        CarrierRole::Verdict,
        "a verification report admissible or blocked",
    ),
    carrier(
        "TraceVerdict",
        CarrierRole::Verdict,
        "whether a resolved call path was observed",
    ),
    carrier(
        "DeltaVerdict",
        CarrierRole::Verdict,
        "whether invariants held across two world models",
    ),
    carrier(
        "ClosureVerdict",
        CarrierRole::Verdict,
        "whether an impact closure held the full-recompute diff",
    ),
    carrier(
        "DependencyVerdict",
        CarrierRole::Verdict,
        "a declared or observed dependency against resolved calls",
    ),
    carrier(
        "ObligationState",
        CarrierRole::Lifecycle,
        "a verification obligation's state from admissible evidence",
    ),
    carrier(
        "CertificateState",
        CarrierRole::Lifecycle,
        "a census certificate's closure state",
    ),
    carrier(
        "DependencyClosureState",
        CarrierRole::Lifecycle,
        "a dependency closure's completeness",
    ),
    carrier(
        "StorageState",
        CarrierRole::Lifecycle,
        "a donor's materialization state",
    ),
    carrier(
        "IntegrityVerdict",
        CarrierRole::Verdict,
        "a candidate eligible, rejected or incomplete against the pinned integrity envelope",
    ),
    carrier(
        "EvaluationStatus",
        CarrierRole::Verdict,
        "one pinned integrity invariant's evaluation against a candidate",
    ),
    carrier(
        "EnvelopeStatus",
        CarrierRole::Lifecycle,
        "an integrity envelope active, superseded or draft",
    ),
    carrier(
        "DesignState",
        CarrierRole::Lifecycle,
        "a design candidate, validated, selected, rejected or superseded (G148)",
    ),
    carrier(
        "DesignVerdict",
        CarrierRole::Verdict,
        "a design accepted or rejected by its check against container, evidence and authority",
    ),
    carrier(
        "Enforcement",
        CarrierRole::Enforcement,
        "an isolation property applied or not, with why",
    ),
    carrier(
        "PhysicalEvidenceLevel",
        CarrierRole::EvidenceLadder,
        "the physical evidence ladder; maps by PhysicalEvidenceLevel::epistemic",
    ),
    carrier(
        "EvidenceBasis",
        CarrierRole::EvidenceLadder,
        "a commercial value's basis, hypothesized to production-measured",
    ),
    carrier(
        "HypothesisOutcome",
        CarrierRole::Outcome,
        "a hypothesis validated, falsified or still hypothesized",
    ),
    carrier(
        "RunOutcome",
        CarrierRole::Outcome,
        "a sandboxed run exited, timed out or was refused",
    ),
    carrier(
        "PathCallOutcome",
        CarrierRole::Outcome,
        "a path call resolved, external or unresolved with why",
    ),
];

/// Status-like field names whose value is text at a boundary, with where it is parsed or
/// produced: `(struct, field, boundary)`.
pub const BOUNDARY_TEXT: &[(&str, &str, &str)] = &[
    (
        "CensusFact",
        "status",
        "an .atlas container record; decoded as an EpistemicStatus name",
    ),
    (
        "CensusObligation",
        "status",
        "an .atlas container record; decoded as an EpistemicStatus name",
    ),
    (
        "CertificateRecord",
        "state",
        "an .atlas container record; a CertificateState name",
    ),
    (
        "PackOutcome",
        "certificate_state",
        "the packed container's certificate state, as recorded",
    ),
    (
        "DependencyState",
        "state",
        "a recensus snapshot; a DependencyClosureState name",
    ),
    (
        "DocumentFact",
        "status",
        "a document's own front-matter lifecycle (accepted, draft), not a claim status",
    ),
    (
        "SelfRecensusReport",
        "dependency_state",
        "the two snapshots' DependencyClosureState names, before and after",
    ),
    (
        "DonorEntry",
        "census_status",
        "a donor campaign ledger row, as written in the markdown ledger",
    ),
    (
        "DonorEntry",
        "decision_status",
        "a donor campaign ledger row, as written in the markdown ledger",
    ),
    (
        "DonorEntry",
        "ingestion_status",
        "a donor campaign ledger row, as written in the markdown ledger",
    ),
];

/// Fields with a status-like name that carry no status at all: `(struct, field, what it is)`.
pub const NOT_A_STATUS: &[(&str, &str, &str)] = &[
    (
        "StateAccess",
        "state",
        "the key of the program state accessed",
    ),
    (
        "FunctionBehavior",
        "state",
        "the program state a function accesses",
    ),
    (
        "WorldModel",
        "state",
        "the program state variables of the model",
    ),
    (
        "MissionContext",
        "state",
        "the program state in a mission's scope",
    ),
    (
        "ImpactClosure",
        "affected_state",
        "the program state keys a change may affect",
    ),
    (
        "ClosureOracle",
        "state",
        "the oracle's check of the program-state level",
    ),
    (
        "Comparison",
        "shared_state",
        "program state two targets both touch",
    ),
    ("Hasher", "chunk_state", "BLAKE3's internal chunk state"),
];

/// Field names that mark a status carrier.
pub const STATUS_FIELD_NAMES: &[&str] = &[
    "status",
    "confidence",
    "evidence_level",
    "verdict",
    "outcome",
    "state",
    "certainty",
];

/// Whether a field named `field` in `strukt` with type spelling `ty` is admitted by the map: its
/// type (inside `Option`, `Vec` or `Box`) is a mapped carrier, or the field is listed as boundary
/// text or as carrying no status. Fields whose name is not status-like are always admitted.
pub fn admits(strukt: &str, field: &str, ty: &str) -> bool {
    let status_like = STATUS_FIELD_NAMES.contains(&field)
        || field.ends_with("_status")
        || field.ends_with("_verdict")
        || field.ends_with("_state");
    if !status_like {
        return true;
    }
    if BOUNDARY_TEXT
        .iter()
        .chain(NOT_A_STATUS)
        .any(|(s, f, _)| *s == strukt && *f == field)
    {
        return true;
    }
    let mut inner = ty.replace(' ', "");
    for wrapper in ["Option<", "Vec<", "Box<"] {
        while let Some(rest) = inner.strip_prefix(wrapper) {
            inner = rest.strip_suffix('>').unwrap_or(rest).to_owned();
        }
    }
    let name = inner.rsplit("::").next().unwrap_or(&inner);
    CARRIERS.iter().any(|c| c.type_name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_map_admits_carriers_and_rejects_a_second_vocabulary() {
        assert!(admits("Anything", "status", "EpistemicStatus"));
        assert!(admits(
            "Anything",
            "status",
            "Option<crate::EpistemicStatus>"
        ));
        assert!(admits("Report", "verdict", "Vec<ConstraintVerdict>"));
        assert!(admits("CensusFact", "status", "String"), "boundary text");
        assert!(admits("StateAccess", "state", "String"), "not a status");
        assert!(admits("Anything", "name", "String"), "not status-like");
        assert!(!admits("Binding", "confidence", "f32"));
        assert!(!admits("HypothesisRecord", "status", "String"));
        assert!(!admits("Anything", "evidence_level", "String"));
        assert!(!admits("Anything", "review_state", "String"));
        for carrier in CARRIERS {
            assert!(!carrier.meaning.is_empty(), "{}", carrier.type_name);
        }
    }
}
