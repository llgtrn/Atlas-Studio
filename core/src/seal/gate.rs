//! The seal eligibility gate (G161, construction node M8, ADR 0076).
//!
//! A seal is decided here and nowhere else. The gate is the first consumer of the verification
//! and integrity reports: it joins the seal policy, the census certificate, the verification
//! report, the integrity report and a SELECTED design on one candidate -- the census container
//! being sealed -- and decides ELIGIBLE only when every one of them admits that exact candidate.
//! Nothing is taken on trust:
//!
//! - the policy must be valid and carry its own identity;
//! - the certificate verdict is recomputed from the certificate's blockers under the policy,
//!   never read from a stored verdict, and the certificate must be at least CLOSED;
//! - the verification report must be ADMISSIBLE, under the policy's verification classes, for
//!   the container's census digest;
//! - the integrity report must carry the verdict the policy requires, for the container's
//!   revision and census digest;
//! - the design must be SELECTED by an authority event, rest on a comparison, carry its own
//!   identity, and select roots in this container.
//!
//! An ELIGIBLE decision yields a [`SealRecord`]: what the SEALED container (M9) carries and its
//! reader re-checks. A NOT_ELIGIBLE decision yields every reason, typed.

use super::{ScopeVerdict, SealPolicy, evaluate_certificate, policy_identity, validate_policy};
use crate::design::{DesignState, SelectedDesign, design_identity};
use crate::identity::IntegrityDigest;
use crate::integrity::IntegrityReport;
use crate::verification::{ReportVerdict, VerificationReport};
use serde::{Deserialize, Serialize};

pub const SEAL_RECORD_SCHEMA_VERSION: &str = "atlas.seal-record.v1";
pub const SEAL_ELIGIBILITY_SCHEMA_VERSION: &str = "atlas.seal-eligibility.v1";

crate::vocabulary_enum! {
    /// Why a candidate cannot be sealed.
    pub enum IneligibleReason {
        PolicyInvalid => "POLICY_INVALID",
        CertificateNotClosed => "CERTIFICATE_NOT_CLOSED",
        CertificateMismatch => "CERTIFICATE_MISMATCH",
        CertificateBlockersRefused => "CERTIFICATE_BLOCKERS_REFUSED",
        VerificationBlocked => "VERIFICATION_BLOCKED",
        VerificationPolicyMismatch => "VERIFICATION_POLICY_MISMATCH",
        VerificationCandidateMismatch => "VERIFICATION_CANDIDATE_MISMATCH",
        IntegrityNotEligible => "INTEGRITY_NOT_ELIGIBLE",
        IntegrityCandidateMismatch => "INTEGRITY_CANDIDATE_MISMATCH",
        DesignAbsent => "DESIGN_ABSENT",
        DesignNotSelected => "DESIGN_NOT_SELECTED",
        DesignWithoutAuthority => "DESIGN_WITHOUT_AUTHORITY",
        DesignWithoutComparison => "DESIGN_WITHOUT_COMPARISON",
        DesignIdentityMismatch => "DESIGN_IDENTITY_MISMATCH",
        DesignCandidateMismatch => "DESIGN_CANDIDATE_MISMATCH",
    }
}

/// The census container a seal is decided for, as its root manifest states it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContainerBinding {
    /// The root identity of the unsealed container the design's roots are in.
    pub root_id: String,
    /// The container's census digest (`blake3-256:<hex>`): the candidate every report names.
    pub census_digest: String,
    pub revision: String,
    pub certificate_id: String,
}

/// The census certificate as the container carries it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CertificateBinding {
    pub certificate_id: String,
    pub state: String,
    pub blockers: Vec<String>,
}

/// Everything the gate joins.
pub struct GateInputs<'a> {
    pub policy: &'a SealPolicy,
    pub container: &'a ContainerBinding,
    pub certificate: &'a CertificateBinding,
    pub verification: &'a VerificationReport,
    pub integrity: &'a IntegrityReport,
    pub design: Option<&'a SelectedDesign>,
}

/// What a SEALED container carries: the identities the seal was decided on, and the certificate
/// blockers the policy permitted. `seal_id` is `seal_identity` of the rest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SealRecord {
    pub schema: String,
    pub seal_id: String,
    pub scope: String,
    pub policy_id: String,
    pub certificate_id: String,
    pub census_digest: String,
    pub revision: String,
    pub verification_report: String,
    pub integrity_report: String,
    pub integrity_envelope: String,
    pub design_id: String,
    /// Every blocker of the certificate, each permitted by the policy (with its reason).
    pub permitted: Vec<String>,
}

/// A seal record's identity: BLAKE3 over its canonical serde form with the identity left empty.
pub fn seal_identity(record: &SealRecord) -> String {
    let mut unstamped = record.clone();
    unstamped.seal_id = String::new();
    let text = serde_json::to_string(&unstamped).expect("a SealRecord always serializes");
    IntegrityDigest::of_bytes(text.as_bytes())
        .as_str()
        .to_owned()
}

/// A verification report's identity: BLAKE3 over its canonical serde form.
pub fn verification_identity(report: &VerificationReport) -> String {
    let text = serde_json::to_string(report).expect("a VerificationReport always serializes");
    IntegrityDigest::of_bytes(text.as_bytes())
        .as_str()
        .to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SealEligibility {
    pub schema: String,
    pub scope: String,
    pub policy_id: String,
    pub verdict: ScopeVerdict,
    /// Every reason the candidate is not eligible, typed, with its detail.
    pub reasons: Vec<(IneligibleReason, String)>,
    /// The record a SEALED container carries, only when ELIGIBLE.
    pub record: Option<SealRecord>,
}

/// M8: whether the container can be sealed under the policy, and the seal record when it can.
pub fn gate(inputs: &GateInputs) -> SealEligibility {
    use IneligibleReason as R;
    let GateInputs {
        policy,
        container,
        certificate,
        verification,
        integrity,
        design,
    } = *inputs;
    let mut reasons: Vec<(IneligibleReason, String)> = Vec::new();
    for problem in validate_policy(policy) {
        reasons.push((R::PolicyInvalid, problem));
    }
    if policy.policy_id != policy_identity(policy) {
        reasons.push((
            R::PolicyInvalid,
            "the policy identity does not verify".into(),
        ));
    }
    // The certificate: recomputed, never trusted.
    if !matches!(certificate.state.as_str(), "CLOSED" | "SEALED") {
        reasons.push((
            R::CertificateNotClosed,
            format!("certificate state {}", certificate.state),
        ));
    }
    if certificate.certificate_id != container.certificate_id {
        reasons.push((
            R::CertificateMismatch,
            format!(
                "certificate {} is not the container's {}",
                certificate.certificate_id, container.certificate_id
            ),
        ));
    }
    let verdict = evaluate_certificate(&certificate.certificate_id, &certificate.blockers, policy);
    for refused in &verdict.refused {
        reasons.push((R::CertificateBlockersRefused, refused.clone()));
    }
    // The verification report: admissible, under the policy, for this candidate.
    if verification.verdict != ReportVerdict::Admissible {
        reasons.push((
            R::VerificationBlocked,
            format!("verification verdict {}", verification.verdict.as_str()),
        ));
    }
    if verification.policy != policy.verification {
        reasons.push((
            R::VerificationPolicyMismatch,
            "the report was evaluated under another verification policy".into(),
        ));
    }
    if verification.candidate != container.census_digest {
        reasons.push((
            R::VerificationCandidateMismatch,
            format!(
                "the report admits {}, not the container's {}",
                verification.candidate, container.census_digest
            ),
        ));
    }
    // The integrity report: the required verdict, for this revision and census.
    if integrity.verdict.as_str() != policy.integrity_verdict_required {
        reasons.push((
            R::IntegrityNotEligible,
            format!("integrity verdict {}", integrity.verdict.as_str()),
        ));
    }
    match integrity_candidate(&container.revision) {
        None => reasons.push((
            R::IntegrityCandidateMismatch,
            format!(
                "container revision `{}` is not a clean git revision an integrity report can name",
                container.revision
            ),
        )),
        Some(revision_ref)
            if integrity.candidate_ref != revision_ref
                || integrity.observed_architecture_root != container.census_digest =>
        {
            reasons.push((
                R::IntegrityCandidateMismatch,
                format!(
                    "the report is for {} at {}, not {revision_ref} at {}",
                    integrity.candidate_ref,
                    integrity.observed_architecture_root,
                    container.census_digest
                ),
            ))
        }
        Some(_) => {}
    }
    // The design: selected by authority, compared, and selecting roots in this container.
    let design_id = match design {
        None => {
            reasons.push((
                R::DesignAbsent,
                "no SelectedDesign names this container".into(),
            ));
            String::new()
        }
        Some(design) => {
            if design.state != DesignState::Selected {
                reasons.push((
                    R::DesignNotSelected,
                    format!("design state {}", design.state.as_str()),
                ));
            }
            if design.authority_event.is_none() {
                reasons.push((R::DesignWithoutAuthority, "no authority event".into()));
            }
            if design.comparison.is_none() {
                reasons.push((R::DesignWithoutComparison, "no design comparison".into()));
            }
            if design.design_id != design_identity(design) {
                reasons.push((
                    R::DesignIdentityMismatch,
                    "the design identity does not verify".into(),
                ));
            }
            if design.parent_root != container.root_id
                || design.candidate != container.census_digest
                || design.scope != policy.scope
            {
                reasons.push((
                    R::DesignCandidateMismatch,
                    "the design selects roots of another container, candidate or scope".into(),
                ));
            }
            design.design_id.clone()
        }
    };
    reasons.sort();
    reasons.dedup();
    let eligible = reasons.is_empty();
    let record = eligible.then(|| {
        let mut record = SealRecord {
            schema: SEAL_RECORD_SCHEMA_VERSION.into(),
            seal_id: String::new(),
            scope: policy.scope.clone(),
            policy_id: policy.policy_id.clone(),
            certificate_id: container.certificate_id.clone(),
            census_digest: container.census_digest.clone(),
            revision: container.revision.clone(),
            verification_report: verification_identity(verification),
            integrity_report: integrity.report_id.clone(),
            integrity_envelope: integrity.envelope_ref.clone(),
            design_id,
            permitted: verdict.permitted.clone(),
        };
        record.seal_id = seal_identity(&record);
        record
    });
    SealEligibility {
        schema: SEAL_ELIGIBILITY_SCHEMA_VERSION.into(),
        scope: policy.scope.clone(),
        policy_id: policy.policy_id.clone(),
        verdict: if eligible {
            ScopeVerdict::Eligible
        } else {
            ScopeVerdict::NotEligible
        },
        reasons,
        record,
    }
}

/// The integrity report's name for a container revision: a clean `git:<sha>` is `revision:<sha>`.
/// A dirty tree (`git:<sha>+dirty`) has no name an integrity report can carry -- the report names
/// the commit, not the working tree -- so it joins nothing.
pub fn integrity_candidate(revision: &str) -> Option<String> {
    let sha = revision.strip_prefix("git:")?;
    let clean = !sha.is_empty() && sha.chars().all(|c| c.is_ascii_hexdigit());
    clean.then(|| format!("revision:{sha}"))
}

/// Every reason `record` does not seal a container whose manifest states `container` and whose
/// certificate carries `blockers` (M9: the writer refuses, the reader rejects).
pub fn check_record(
    record: &SealRecord,
    container: &ContainerBinding,
    blockers: &[String],
) -> Vec<String> {
    let mut problems = Vec::new();
    if record.schema != SEAL_RECORD_SCHEMA_VERSION {
        problems.push(format!("unknown seal record schema `{}`", record.schema));
    }
    if record.seal_id != seal_identity(record) {
        problems.push("the seal identity does not verify".into());
    }
    if record.certificate_id != container.certificate_id
        || record.census_digest != container.census_digest
        || record.revision != container.revision
    {
        problems.push("the seal binds another certificate, census or revision".into());
    }
    for blocker in blockers {
        let prefix = format!("{blocker}: ");
        if !record.permitted.iter().any(|p| p.starts_with(&prefix)) {
            problems.push(format!(
                "blocker `{blocker}` is open and the seal did not permit it"
            ));
        }
    }
    problems
}

#[cfg(test)]
mod tests;
