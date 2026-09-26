//! The declared seal policy and scoped certificate evaluation (G149, ADR 0065).
//!
//! `self_scope_policy` is the strict default a provider may propose: every blocker must be clear,
//! every semantic dimension is HARD, the nine dynamic dependency classes the Cargo census cannot
//! observe are a residual bounded at nine, and an unsealed root is out of scope because the seal is
//! what seals it. The repository declares its policy at `DECLARED_SEAL_POLICY_PATH`; the owner
//! may relax it only within what `validate_policy` admits.

use crate::design::{invalid, read_container, read_design, read_json, read_report};
use atlas_core::atlas::{SEALED, seal_binding, write};
use atlas_core::integrity::IntegrityReport;
use atlas_core::seal::{
    BlockerKind, CertificateBinding, DECLARED_SEAL_POLICY_PATH, Disposition, GateInputs,
    ResidualBound, SEAL_POLICY_SCHEMA_VERSION, ScopePolicy, ScopedCertificateVerdict,
    evaluate_certificate, gate, policy_identity,
};
pub use atlas_core::seal::{
    IneligibleReason, ScopeVerdict, SealEligibility, SealPolicy, SealRecord, validate_policy,
};
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
        SemanticDimension::Resource,
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

/// G161 (M8, ADR 0076): the seal gate for the verified container at `container`, joined with the
/// verification report, the integrity report and (when one exists) the SelectedDesign that name
/// it. The container binding and its certificate are read from the container itself -- the
/// certificate the seal would bind -- never from a separate file.
pub fn gate_container(
    container: impl AsRef<Path>,
    verification: impl AsRef<Path>,
    integrity: impl AsRef<Path>,
    design: Option<&Path>,
    policy: &SealPolicy,
) -> io::Result<SealEligibility> {
    let (atlas, root_id) = read_container(container)?;
    if atlas.seal.is_some() {
        return Err(invalid("the container is already sealed"));
    }
    let mut binding = seal_binding(&atlas.manifest);
    binding.root_id = root_id;
    let certificate = CertificateBinding {
        certificate_id: atlas.certificate.certificate_id.clone(),
        state: atlas.certificate.state.clone(),
        blockers: atlas.certificate.blockers.clone(),
    };
    let verification = read_report(verification)?;
    let integrity: IntegrityReport = read_json(integrity)?;
    let design = design.map(read_design).transpose()?;
    Ok(gate(&GateInputs {
        policy,
        container: &binding,
        certificate: &certificate,
        verification: &verification,
        integrity: &integrity,
        design: design.as_ref(),
    }))
}

/// G161 (M9): writes the container at `container`, sealed by an ELIGIBLE decision's record, to
/// `out`; returns the sealed root identity. Anything else is refused, and the writer re-checks
/// that the record binds this container's certificate, census and revision.
pub fn seal_container(
    container: impl AsRef<Path>,
    eligibility: &SealEligibility,
    out: impl AsRef<Path>,
) -> io::Result<String> {
    let record = match (&eligibility.verdict, &eligibility.record) {
        (ScopeVerdict::Eligible, Some(record)) => record.clone(),
        _ => {
            return Err(invalid(format!(
                "SEAL_NOT_ELIGIBLE: {} reasons",
                eligibility.reasons.len()
            )));
        }
    };
    let (mut atlas, _) = read_container(container)?;
    if atlas.seal.is_some() {
        return Err(invalid("the container is already sealed"));
    }
    atlas.manifest.seal = SEALED.into();
    atlas.seal = Some(record);
    let bytes = write(&atlas).map_err(invalid)?;
    let (_, root) = atlas_core::atlas::read(&bytes).map_err(invalid)?;
    std::fs::write(out, &bytes)?;
    Ok(root.as_str().to_owned())
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

    /// G161: a fixture container every input admits is ELIGIBLE, is sealed, and reads back sealed;
    /// sealing twice and sealing without eligibility are refused. The design is selected by a
    /// test-fixture principal, never a recorded selection.
    #[test]
    fn a_container_every_input_admits_is_gated_sealed_and_read_back_sealed() {
        use atlas_core::atlas::{CensusAtlas, CertificateRecord, RootManifest, UNSEALED};
        use atlas_core::design::{
            AuthorityEvent, AuthorityMode, DesignState, Principal, PrincipalKind, SelectedDesign,
            container_candidate, design_identity, event_identity,
        };
        use atlas_core::integrity::{ImpactClosureRef, IntegrityVerdict};
        let dir = std::env::temp_dir().join(format!("atlas-seal-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sha = "0123456789abcdef0123456789abcdef01234567";
        let mut atlas = CensusAtlas {
            manifest: RootManifest {
                genome_schema: "atlas.genome.v1".into(),
                genome_hash: [7; 32],
                census_digest: [9; 32],
                revision: format!("git:{sha}"),
                certificate_id: "census-certificate:fixture".into(),
                seal: UNSEALED.into(),
                tool: "test".into(),
                mode: "THIN".into(),
            },
            facts: Vec::new(),
            obligations: Vec::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            certificate: CertificateRecord {
                certificate_id: "census-certificate:fixture".into(),
                state: "CLOSED".into(),
                blockers: vec!["ATLAS_ROOT_UNSEALED: the fixture container".into()],
            },
            typed_records: Vec::new(),
            evidence: Vec::new(),
            diagnostics: Vec::new(),
            seal: None,
        };
        atlas.canonicalize();
        let container = dir.join("fixture.atlas");
        std::fs::write(&container, write(&atlas).unwrap()).unwrap();
        let (_, root) = read_container(&container).unwrap();
        let candidate = container_candidate(&atlas);
        let verification = dir.join("verification.json");
        let body = serde_json::json!({
            "schema": "atlas.verification-report.v1",
            "policy": VerificationPolicy::library_package(),
            "candidate": candidate,
            "outcomes": [], "coverage": {}, "failures": [],
            "verdict": "ADMISSIBLE", "blockers": []
        });
        std::fs::write(&verification, body.to_string()).unwrap();
        let integrity = dir.join("integrity.json");
        let report = IntegrityReport {
            schema_version: "atlas.architectural-integrity-report.v1".into(),
            report_id: "report:fixture".into(),
            candidate_ref: format!("revision:{sha}"),
            materialization_ref: None,
            envelope_ref: "envelope:fixture".into(),
            observed_architecture_root: candidate.clone(),
            impact_closure: ImpactClosureRef {
                closed: true,
                affected_semantic_refs: Vec::new(),
                affected_invariant_refs: Vec::new(),
                closure_root: None,
            },
            evaluations: Vec::new(),
            hard_violation_count: 0,
            required_unknown_count: 0,
            load_bearing_equivalence_closed: true,
            blueprint_revision_ref: None,
            verdict: IntegrityVerdict::Eligible,
            evidence_refs: Vec::new(),
        };
        std::fs::write(&integrity, serde_json::to_string(&report).unwrap()).unwrap();
        let mut design = SelectedDesign {
            schema: "atlas.selected-design.v1".into(),
            design_id: String::new(),
            parent_root: root.clone(),
            candidate: candidate.clone(),
            scope: SELF_SCOPE.into(),
            target_kind: "LIBRARY_PACKAGE".into(),
            variant: "default".into(),
            state: DesignState::Selected,
            roots: Vec::new(),
            bindings: Vec::new(),
            candidate_set: Vec::new(),
            evidence_refs: Vec::new(),
            authority_mode: AuthorityMode::HumanRequired,
            authority_event: None,
            rationale: "test fixture".into(),
            supersedes: None,
            comparison: Some("comparison:fixture".into()),
        };
        design.design_id = design_identity(&design);
        let mut event = AuthorityEvent {
            event_id: String::new(),
            principal: Principal {
                id: "fixture-principal".into(),
                kind: PrincipalKind::Human,
            },
            mode: AuthorityMode::HumanRequired,
            design_id: design.design_id.clone(),
            generation: "G161".into(),
            statement: "test fixture".into(),
        };
        event.event_id = event_identity(&event);
        design.authority_event = Some(event);
        let design_path = dir.join("design.json");
        std::fs::write(&design_path, serde_json::to_string(&design).unwrap()).unwrap();
        let policy = self_scope_policy();

        // Without a design: NOT_ELIGIBLE, and nothing can be sealed with that decision.
        let refused = gate_container(&container, &verification, &integrity, None, &policy).unwrap();
        assert_eq!(refused.verdict, ScopeVerdict::NotEligible);
        let reasons: Vec<IneligibleReason> = refused.reasons.iter().map(|(r, _)| *r).collect();
        assert_eq!(reasons, [IneligibleReason::DesignAbsent]);
        let sealed = dir.join("sealed.atlas");
        assert!(seal_container(&container, &refused, &sealed).is_err());
        assert!(!sealed.exists());

        // With the selected design: ELIGIBLE, sealed, and the sealed container reads back.
        let eligible = gate_container(
            &container,
            &verification,
            &integrity,
            Some(design_path.as_path()),
            &policy,
        )
        .unwrap();
        assert_eq!(
            eligible.verdict,
            ScopeVerdict::Eligible,
            "{:?}",
            eligible.reasons
        );
        let sealed_root = seal_container(&container, &eligible, &sealed).unwrap();
        assert_ne!(sealed_root, root, "the seal is inside the root identity");
        let (read_back, read_root) = read_container(&sealed).unwrap();
        assert_eq!(read_root, sealed_root);
        assert_eq!(read_back.manifest.seal, SEALED);
        assert_eq!(read_back.seal, eligible.record);
        // A sealed container is not gated or sealed again.
        assert!(gate_container(&sealed, &verification, &integrity, None, &policy).is_err());
        assert!(seal_container(&sealed, &eligible, dir.join("twice.atlas")).is_err());
        // A decision for another container does not seal this one: the writer refuses it.
        let mut other = atlas.clone();
        other.manifest.revision = "git:fedcba9876543210fedcba9876543210fedcba98".into();
        let other_path = dir.join("other.atlas");
        std::fs::write(&other_path, write(&other).unwrap()).unwrap();
        assert!(seal_container(&other_path, &eligible, dir.join("wrong.atlas")).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
