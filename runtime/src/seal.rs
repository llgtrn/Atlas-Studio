//! The declared seal policy and scoped certificate evaluation (G149, ADR 0065).
//!
//! `self_scope_policy` is the strict default a provider may propose: every blocker must be clear,
//! every semantic dimension is HARD, the nine dynamic dependency classes the Cargo census cannot
//! observe are a residual bounded at nine, and an unsealed root is out of scope because the seal is
//! what seals it. The repository declares its policy at `DECLARED_SEAL_POLICY_PATH`; the owner
//! may relax it only within what `validate_policy` admits.

use crate::design::{invalid, read_json};
use atlas_core::seal::{
    BlockerKind, DECLARED_SEAL_POLICY_PATH, Disposition, ResidualBound, SEAL_POLICY_SCHEMA_VERSION,
    ScopePolicy, ScopedCertificateVerdict, evaluate_certificate, policy_identity,
};
pub use atlas_core::seal::{ScopeVerdict, SealPolicy, validate_policy};
use atlas_core::verification::VerificationPolicy;
use atlas_core::{DynamicDependencyObligation, SemanticDimension};
use std::{io, path::Path};

pub const SELF_SCOPE: &str = "atlas-self";

/// The strict self-scope policy (see the module documentation).
pub fn self_scope_policy() -> SealPolicy {
    let mut rules: std::collections::BTreeMap<BlockerKind, Disposition> = BlockerKind::ALL
        .into_iter()
        .map(|kind| (kind, Disposition::RequiredClear))
        .collect();
    let classes: Vec<String> = DynamicDependencyObligation::ALL
        .iter()
        .map(|c| serde_json::to_value(c).expect("serializes"))
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    rules.insert(
        BlockerKind::DynamicDependencyObligations,
        Disposition::AcceptedBoundedResidual {
            bound: ResidualBound::AtMost(classes.len() as u64),
            reason: format!(
                "the dynamic dependency classes the Cargo census cannot observe ({}); they stay listed as residuals",
                classes.join(", ")
            ),
        },
    );
    rules.insert(
        BlockerKind::AtlasRootUnsealed,
        Disposition::OutOfScope {
            reason: "the seal gate is what seals the candidate's root".into(),
        },
    );
    let hard_dimensions: Vec<String> = [
        SemanticDimension::Symbol,
        SemanticDimension::Type,
        SemanticDimension::FunctionIdentity,
        SemanticDimension::FunctionSignature,
        SemanticDimension::Call,
        SemanticDimension::ControlFlow,
        SemanticDimension::DataFlow,
        SemanticDimension::State,
        SemanticDimension::Effect,
        SemanticDimension::Ownership,
        SemanticDimension::Concurrency,
        SemanticDimension::Persistence,
    ]
    .iter()
    .map(|d| d.as_str().to_owned())
    .collect();
    let mut policy = SealPolicy {
        schema: SEAL_POLICY_SCHEMA_VERSION.into(),
        policy_id: String::new(),
        scope: SELF_SCOPE.into(),
        verification: VerificationPolicy::library_package(),
        certificate: ScopePolicy {
            scope: SELF_SCOPE.into(),
            rules,
        },
        hard_dimensions,
        integrity_verdict_required: "ELIGIBLE".into(),
    };
    policy.policy_id = policy_identity(&policy);
    policy
}

pub fn read_policy(path: impl AsRef<Path>) -> io::Result<SealPolicy> {
    read_json(path)
}

/// The repository's declared seal policy.
pub fn declared_policy(root: impl AsRef<Path>) -> io::Result<SealPolicy> {
    read_policy(root.as_ref().join(DECLARED_SEAL_POLICY_PATH))
}

/// A certificate file's scoped verdict under `policy`.
pub fn evaluate(
    certificate_path: impl AsRef<Path>,
    policy: &SealPolicy,
) -> io::Result<ScopedCertificateVerdict> {
    let certificate: serde_json::Value = read_json(certificate_path)?;
    let id = certificate["certificate_id"]
        .as_str()
        .ok_or_else(|| invalid("certificate without certificate_id"))?;
    let blockers: Vec<String> = certificate["blockers"]
        .as_array()
        .ok_or_else(|| invalid("certificate without blockers"))?
        .iter()
        .map(|b| {
            b.as_str()
                .map(str::to_owned)
                .ok_or_else(|| invalid("a blocker is not text"))
        })
        .collect::<io::Result<_>>()?;
    Ok(evaluate_certificate(id, &blockers, policy))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    /// The declared policy is admissible, stamped with its identity, and no weaker than the strict
    /// default where the default is strict.
    #[test]
    fn the_declared_seal_policy_is_valid_and_pinned_by_identity() {
        let declared = declared_policy(root()).unwrap();
        assert_eq!(validate_policy(&declared), Vec::<String>::new());
        assert_eq!(
            declared,
            self_scope_policy(),
            "re-pin with `atlas-systemizer seal policy --out`"
        );
    }

    /// The self census, as its G148 certificate stands, is not eligible for a seal, and says why:
    /// the dirty revision, the unknown coverage of HARD dimensions, unknown and unsupported facts,
    /// a single engine where two are required -- while its nine dynamic dependency classes stay
    /// listed as permitted residuals.
    #[test]
    fn the_self_census_is_not_eligible_and_the_verdict_names_every_reason() {
        let policy = self_scope_policy();
        let verdict = evaluate(
            root().join(".atlas/evidence/census/G148/certificate.json"),
            &policy,
        )
        .unwrap();
        assert_eq!(verdict.verdict, ScopeVerdict::NotEligible);
        for reason in [
            "REVISION_DIRTY",
            "COVERAGE_UNKNOWN: CALL",
            "UNKNOWN_FACTS",
            "UNSUPPORTED_FACTS",
            "MULTI_ENGINE_RECONCILIATION_ABSENT",
        ] {
            assert!(
                verdict.refused.iter().any(|r| r.starts_with(reason)),
                "{reason}: {:?}",
                verdict.refused
            );
        }
        assert!(
            verdict
                .permitted
                .iter()
                .any(|p| p.starts_with("DYNAMIC_DEPENDENCY_OBLIGATIONS: 9"))
        );
    }
}
