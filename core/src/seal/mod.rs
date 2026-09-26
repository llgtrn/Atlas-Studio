//! Scoped certificate policy and SealPolicy (G149, ADR 0065; construction nodes M3 and M4).
//!
//! A CensusCertificate lists its blockers as `CODE: detail` text. Here every code is a typed
//! `BlockerKind` -- an unknown code is refused, never skipped -- and a scope's policy gives each
//! kind a disposition:
//! - `REQUIRED_CLEAR`: any such blocker makes the scope ineligible;
//! - `ACCEPTED_BOUNDED_RESIDUAL`: permitted when its detail is one of an enumerated set or a
//!   count within a bound, with a recorded reason; a permitted residual is listed in the verdict,
//!   never dropped (`contracts/CENSUS-CERTIFICATE.md`: "those states never disappear");
//! - `OUT_OF_SCOPE`: the scope does not cover it, with a recorded reason.
//!
//! A `SealPolicy` binds a scope's certificate policy to its verification policy, the dimensions
//! whose coverage it treats as HARD and the integrity verdict it requires, under a digest
//! identity. Some blockers can never be permitted (a dirty revision, incomplete provenance, a
//! replay that did not converge, unresolved dependencies, conflicts), and a HARD dimension's
//! coverage can never be accepted as unknown: `validate_policy` refuses such a policy, so no
//! provider can weaken it to make its own candidate pass.

use crate::identity::IntegrityDigest;
use crate::verification::VerificationPolicy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SEAL_POLICY_SCHEMA_VERSION: &str = "atlas.seal-policy.v1";
pub const SCOPED_VERDICT_SCHEMA_VERSION: &str = "atlas.scoped-certificate-verdict.v1";
/// Where a repository declares its seal policy.
pub const DECLARED_SEAL_POLICY_PATH: &str = ".atlas/declared/seal-policy.json";

crate::vocabulary_enum! {
    /// Every blocker code a CensusCertificate emits (`core::certificate`).
    pub enum BlockerKind {
        RevisionDirty => "REVISION_DIRTY",
        InventoryNotAccounted => "INVENTORY_NOT_ACCOUNTED",
        ArtifactsWithoutSourceFrontend => "ARTIFACTS_WITHOUT_SOURCE_FRONTEND",
        WorkspaceMemberOutsideInventory => "WORKSPACE_MEMBER_OUTSIDE_INVENTORY",
        DependencyClosureNotClosed => "DEPENDENCY_CLOSURE_NOT_CLOSED",
        DependencyUnresolved => "DEPENDENCY_UNRESOLVED",
        SemanticObligationsNotAccounted => "SEMANTIC_OBLIGATIONS_NOT_ACCOUNTED",
        CoverageUnknown => "COVERAGE_UNKNOWN",
        CoverageUnsupported => "COVERAGE_UNSUPPORTED",
        CoverageConflict => "COVERAGE_CONFLICT",
        UnknownFacts => "UNKNOWN_FACTS",
        UnsupportedFacts => "UNSUPPORTED_FACTS",
        DynamicDependencyObligations => "DYNAMIC_DEPENDENCY_OBLIGATIONS",
        ProvenanceIncomplete => "PROVENANCE_INCOMPLETE",
        NormalizationConflicts => "NORMALIZATION_CONFLICTS",
        ReplayNotConverged => "REPLAY_NOT_CONVERGED",
        MultiEngineReconciliationAbsent => "MULTI_ENGINE_RECONCILIATION_ABSENT",
        AtlasRootAbsent => "ATLAS_ROOT_ABSENT",
        AtlasRootUnsealed => "ATLAS_ROOT_UNSEALED",
    }
}

impl BlockerKind {
    pub const ALL: [Self; 19] = [
        Self::RevisionDirty,
        Self::InventoryNotAccounted,
        Self::ArtifactsWithoutSourceFrontend,
        Self::WorkspaceMemberOutsideInventory,
        Self::DependencyClosureNotClosed,
        Self::DependencyUnresolved,
        Self::SemanticObligationsNotAccounted,
        Self::CoverageUnknown,
        Self::CoverageUnsupported,
        Self::CoverageConflict,
        Self::UnknownFacts,
        Self::UnsupportedFacts,
        Self::DynamicDependencyObligations,
        Self::ProvenanceIncomplete,
        Self::NormalizationConflicts,
        Self::ReplayNotConverged,
        Self::MultiEngineReconciliationAbsent,
        Self::AtlasRootAbsent,
        Self::AtlasRootUnsealed,
    ];

    /// Kinds no policy may permit: each means the census itself cannot be trusted.
    pub const NEVER_PERMITTED: [Self; 6] = [
        Self::RevisionDirty,
        Self::DependencyUnresolved,
        Self::ProvenanceIncomplete,
        Self::NormalizationConflicts,
        Self::ReplayNotConverged,
        Self::CoverageConflict,
    ];
}

/// A certificate blocker as its kind and detail (`COVERAGE_UNKNOWN: CALL` is `CoverageUnknown`,
/// `CALL`); a code outside the vocabulary is an error.
pub fn parse_blocker(text: &str) -> Result<(BlockerKind, String), String> {
    let (code, detail) = match text.split_once(':') {
        Some((code, detail)) => (code.trim(), detail.trim()),
        None => (text.trim(), ""),
    };
    BlockerKind::ALL
        .into_iter()
        .find(|kind| kind.as_str() == code)
        .map(|kind| (kind, detail.to_owned()))
        .ok_or(format!("unknown blocker code `{code}`"))
}

/// How much of a residual a disposition permits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResidualBound {
    /// The blocker's detail must be one of these (a dimension, a class).
    Enumerated(Vec<String>),
    /// The blocker's detail is a count, at most this.
    AtMost(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Disposition {
    RequiredClear,
    AcceptedBoundedResidual {
        bound: ResidualBound,
        reason: String,
    },
    OutOfScope {
        reason: String,
    },
}

/// One scope's disposition of every blocker kind.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScopePolicy {
    pub scope: String,
    pub rules: BTreeMap<BlockerKind, Disposition>,
}

crate::vocabulary_enum! {
    /// Whether a certificate is eligible under a scope's policy.
    pub enum ScopeVerdict {
        Eligible => "ELIGIBLE",
        NotEligible => "NOT_ELIGIBLE",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScopedCertificateVerdict {
    pub schema: String,
    pub scope: String,
    pub policy_id: String,
    pub certificate_id: String,
    pub verdict: ScopeVerdict,
    /// Blockers the policy does not permit, with why.
    pub refused: Vec<String>,
    /// Blockers the policy permits, with its reason: listed, never dropped.
    pub permitted: Vec<String>,
}

/// A scope's policy applied to a certificate's blockers.
pub fn evaluate_certificate(
    certificate_id: &str,
    blockers: &[String],
    policy: &SealPolicy,
) -> ScopedCertificateVerdict {
    let mut refused = Vec::new();
    let mut permitted = Vec::new();
    for blocker in blockers {
        let (kind, detail) = match parse_blocker(blocker) {
            Ok(parsed) => parsed,
            Err(why) => {
                refused.push(format!("{blocker}: {why}"));
                continue;
            }
        };
        match policy.certificate.rules.get(&kind) {
            None => refused.push(format!("{blocker}: the policy does not classify {kind}")),
            Some(Disposition::RequiredClear) => refused.push(format!("{blocker}: required clear")),
            Some(Disposition::OutOfScope { reason }) => {
                permitted.push(format!("{blocker}: out of scope ({reason})"));
            }
            Some(Disposition::AcceptedBoundedResidual { bound, reason }) => {
                let within = match bound {
                    ResidualBound::Enumerated(items) => items.contains(&detail),
                    ResidualBound::AtMost(limit) => {
                        detail.parse::<u64>().is_ok_and(|count| count <= *limit)
                    }
                };
                if within {
                    permitted.push(format!("{blocker}: bounded residual ({reason})"));
                } else {
                    refused.push(format!("{blocker}: beyond the permitted bound"));
                }
            }
        }
    }
    ScopedCertificateVerdict {
        schema: SCOPED_VERDICT_SCHEMA_VERSION.into(),
        scope: policy.certificate.scope.clone(),
        policy_id: policy.policy_id.clone(),
        certificate_id: certificate_id.to_owned(),
        verdict: if refused.is_empty() {
            ScopeVerdict::Eligible
        } else {
            ScopeVerdict::NotEligible
        },
        refused,
        permitted,
    }
}

/// The policy a seal is decided under.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SealPolicy {
    pub schema: String,
    /// `policy_identity` of the rest.
    pub policy_id: String,
    pub scope: String,
    /// The verification classes a sealed candidate must be admitted under.
    pub verification: VerificationPolicy,
    pub certificate: ScopePolicy,
    /// Dimensions whose coverage is HARD: never permitted UNKNOWN or UNSUPPORTED.
    pub hard_dimensions: Vec<String>,
    /// The integrity report verdict a sealed candidate needs.
    pub integrity_verdict_required: String,
}

/// A policy's identity: BLAKE3 over its canonical serde form with the identity left empty.
pub fn policy_identity(policy: &SealPolicy) -> String {
    let mut unstamped = policy.clone();
    unstamped.policy_id = String::new();
    let text = serde_json::to_string(&unstamped).expect("a SealPolicy always serializes");
    IntegrityDigest::of_bytes(text.as_bytes())
        .as_str()
        .to_owned()
}

/// Every reason `policy` is not an admissible seal policy.
pub fn validate_policy(policy: &SealPolicy) -> Vec<String> {
    let mut problems = Vec::new();
    if policy.schema != SEAL_POLICY_SCHEMA_VERSION {
        problems.push(format!("unknown schema `{}`", policy.schema));
    }
    let identity = policy_identity(policy);
    if policy.policy_id != identity {
        problems.push(format!(
            "policy_id {} is not the policy's identity {identity}",
            policy.policy_id
        ));
    }
    if policy.certificate.scope != policy.scope {
        problems.push("the certificate policy is for another scope".into());
    }
    if policy.integrity_verdict_required != "ELIGIBLE" {
        problems.push("a seal requires an ELIGIBLE integrity report".into());
    }
    for kind in BlockerKind::ALL {
        let Some(rule) = policy.certificate.rules.get(&kind) else {
            problems.push(format!("{kind} has no disposition"));
            continue;
        };
        if BlockerKind::NEVER_PERMITTED.contains(&kind) && *rule != Disposition::RequiredClear {
            problems.push(format!("{kind} can never be permitted"));
        }
        let coverage =
            kind == BlockerKind::CoverageUnknown || kind == BlockerKind::CoverageUnsupported;
        if coverage {
            let hard_accepted = match rule {
                Disposition::RequiredClear => false,
                Disposition::OutOfScope { .. } => true,
                Disposition::AcceptedBoundedResidual { bound, .. } => match bound {
                    ResidualBound::Enumerated(items) => {
                        items.iter().any(|d| policy.hard_dimensions.contains(d))
                    }
                    ResidualBound::AtMost(_) => true,
                },
            };
            if hard_accepted && !policy.hard_dimensions.is_empty() {
                problems.push(format!("{kind} is permitted for a HARD dimension"));
            }
        }
    }
    problems
}

#[cfg(test)]
mod tests;
