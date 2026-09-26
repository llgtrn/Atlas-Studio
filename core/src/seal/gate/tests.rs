use super::super::{
    BlockerKind, Disposition, ResidualBound, SEAL_POLICY_SCHEMA_VERSION, ScopePolicy,
};
use super::*;
use crate::design::{
    AuthorityEvent, AuthorityMode, Principal, PrincipalKind, SelectedDesign, event_identity,
};
use crate::integrity::{ImpactClosureRef, IntegrityVerdict};
use crate::verification::VerificationPolicy;
use std::collections::BTreeMap;

const DIGEST: &str = "blake3-256:census";

fn policy() -> SealPolicy {
    let mut rules: BTreeMap<BlockerKind, Disposition> = BlockerKind::ALL
        .into_iter()
        .map(|kind| (kind, Disposition::RequiredClear))
        .collect();
    rules.insert(
        BlockerKind::DynamicDependencyObligations,
        Disposition::AcceptedBoundedResidual {
            bound: ResidualBound::AtMost(9),
            reason: "the nine dynamic dependency classes".into(),
        },
    );
    rules.insert(
        BlockerKind::AtlasRootUnsealed,
        Disposition::OutOfScope {
            reason: "the seal is what seals the root".into(),
        },
    );
    let mut policy = SealPolicy {
        schema: SEAL_POLICY_SCHEMA_VERSION.into(),
        policy_id: String::new(),
        scope: "fixture".into(),
        verification: VerificationPolicy::library_package(),
        certificate: ScopePolicy {
            scope: "fixture".into(),
            rules,
        },
        hard_dimensions: vec!["CALL".into()],
        integrity_verdict_required: "ELIGIBLE".into(),
    };
    policy.policy_id = policy_identity(&policy);
    policy
}

fn container() -> ContainerBinding {
    ContainerBinding {
        root_id: "blake3-256:root".into(),
        census_digest: DIGEST.into(),
        revision: "git:abc123".into(),
        certificate_id: "certificate:fixture".into(),
    }
}

fn certificate() -> CertificateBinding {
    CertificateBinding {
        certificate_id: "certificate:fixture".into(),
        state: "CLOSED".into(),
        blockers: vec![
            "ATLAS_ROOT_UNSEALED: the container is not sealed yet".into(),
            "DYNAMIC_DEPENDENCY_OBLIGATIONS: 9".into(),
        ],
    }
}

fn verification() -> VerificationReport {
    VerificationReport {
        schema: "atlas.verification-report.v1".into(),
        policy: VerificationPolicy::library_package(),
        candidate: DIGEST.into(),
        outcomes: Vec::new(),
        coverage: BTreeMap::new(),
        failures: Vec::new(),
        verdict: ReportVerdict::Admissible,
        blockers: Vec::new(),
    }
}

fn integrity() -> IntegrityReport {
    IntegrityReport {
        schema_version: "atlas.integrity-report.v1".into(),
        report_id: "integrity:fixture".into(),
        candidate_ref: "revision:abc123".into(),
        materialization_ref: None,
        envelope_ref: "envelope:fixture".into(),
        observed_architecture_root: DIGEST.into(),
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
    }
}

/// A test fixture's design: selected by a fixture principal, never a recorded selection.
fn design() -> SelectedDesign {
    let mut design = SelectedDesign {
        schema: "atlas.selected-design.v1".into(),
        design_id: String::new(),
        parent_root: "blake3-256:root".into(),
        candidate: DIGEST.into(),
        scope: "fixture".into(),
        target_kind: "LIBRARY_PACKAGE".into(),
        variant: "default".into(),
        state: DesignState::Selected,
        roots: Vec::new(),
        bindings: Vec::new(),
        candidate_set: Vec::new(),
        evidence_refs: Vec::new(),
        authority_mode: AuthorityMode::HumanRequired,
        authority_event: None,
        rationale: "fixture".into(),
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
    design
}

fn decide(
    policy: &SealPolicy,
    container: &ContainerBinding,
    certificate: &CertificateBinding,
    verification: &VerificationReport,
    integrity: &IntegrityReport,
    design: Option<&SelectedDesign>,
) -> SealEligibility {
    gate(&GateInputs {
        policy,
        container,
        certificate,
        verification,
        integrity,
        design,
    })
}

fn reasons(eligibility: &SealEligibility) -> Vec<IneligibleReason> {
    eligibility.reasons.iter().map(|(r, _)| *r).collect()
}

#[test]
fn a_candidate_every_input_admits_is_eligible_and_its_record_verifies() {
    let design = design();
    let e = decide(
        &policy(),
        &container(),
        &certificate(),
        &verification(),
        &integrity(),
        Some(&design),
    );
    assert_eq!(e.verdict, ScopeVerdict::Eligible, "{:?}", e.reasons);
    let record = e.record.expect("an eligible decision carries its record");
    assert_eq!(record.seal_id, seal_identity(&record));
    assert_eq!(record.design_id, design.design_id);
    assert_eq!(
        record.verification_report,
        verification_identity(&verification())
    );
    assert_eq!(
        record.permitted.len(),
        2,
        "permitted residuals listed, never dropped"
    );
    assert_eq!(
        check_record(&record, &container(), &certificate().blockers),
        Vec::<String>::new()
    );
}

/// An edit of the gate's inputs.
type Mutation<'a> = dyn Fn(
        &mut SealPolicy,
        &mut CertificateBinding,
        &mut VerificationReport,
        &mut IntegrityReport,
        &mut Option<SelectedDesign>,
    ) + 'a;

#[test]
fn every_input_that_does_not_admit_the_candidate_refuses_the_seal() {
    use IneligibleReason as R;
    let base_design = design();
    let case = |mutate: &Mutation<'_>| {
        let (mut p, mut c, mut v, mut i, mut d) = (
            policy(),
            certificate(),
            verification(),
            integrity(),
            Some(base_design.clone()),
        );
        mutate(&mut p, &mut c, &mut v, &mut i, &mut d);
        let e = decide(&p, &container(), &c, &v, &i, d.as_ref());
        assert_eq!(e.verdict, ScopeVerdict::NotEligible);
        assert!(e.record.is_none(), "no record without eligibility");
        reasons(&e)
    };
    assert!(
        case(&|p, _, _, _, _| p.integrity_verdict_required = "INCOMPLETE".into())
            .contains(&R::PolicyInvalid)
    );
    assert_eq!(
        case(&|_, c, _, _, _| c.state = "RECONCILED".into()),
        [R::CertificateNotClosed]
    );
    assert_eq!(
        case(&|_, c, _, _, _| c.certificate_id = "certificate:other".into()),
        [R::CertificateMismatch]
    );
    assert_eq!(
        case(&|_, c, _, _, _| c.blockers.push("REVISION_DIRTY: 3 files".into())),
        [R::CertificateBlockersRefused]
    );
    assert_eq!(
        case(&|_, c, _, _, _| c.blockers.push("DYNAMIC_DEPENDENCY_OBLIGATIONS: 10".into())),
        [R::CertificateBlockersRefused],
        "a residual beyond its bound"
    );
    assert_eq!(
        case(&|_, _, v, _, _| v.verdict = ReportVerdict::Blocked),
        [R::VerificationBlocked]
    );
    assert_eq!(
        case(&|_, _, v, _, _| v.policy.required_classes.clear()),
        [R::VerificationPolicyMismatch]
    );
    assert_eq!(
        case(&|_, _, v, _, _| v.candidate = "blake3-256:other".into()),
        [R::VerificationCandidateMismatch]
    );
    assert_eq!(
        case(&|_, _, _, i, _| i.verdict = IntegrityVerdict::Incomplete),
        [R::IntegrityNotEligible]
    );
    assert_eq!(
        case(&|_, _, _, i, _| i.candidate_ref = "revision:other".into()),
        [R::IntegrityCandidateMismatch]
    );
    assert_eq!(
        case(&|_, _, _, i, _| i.observed_architecture_root = "blake3-256:other".into()),
        [R::IntegrityCandidateMismatch]
    );
    // G161: the container records `git:<sha>[+dirty]`, the integrity report `revision:<sha>`; a
    // dirty tree has no revision an integrity report can name.
    let mut dirty = container();
    dirty.revision = "git:abc123+dirty".into();
    let e = decide(
        &policy(),
        &dirty,
        &certificate(),
        &verification(),
        &integrity(),
        Some(&base_design),
    );
    assert_eq!(reasons(&e), [R::IntegrityCandidateMismatch]);
    assert_eq!(
        integrity_candidate("git:abc123").as_deref(),
        Some("revision:abc123")
    );
    for unnamed in ["abc123", "git:", "git:abc123+dirty", "revision:abc123"] {
        assert_eq!(integrity_candidate(unnamed), None, "{unnamed}");
    }
    assert_eq!(case(&|_, _, _, _, d| *d = None), [R::DesignAbsent]);
    assert!(
        case(&|_, _, _, _, d| d.as_mut().unwrap().state = DesignState::Validated)
            .contains(&R::DesignNotSelected)
    );
    assert_eq!(
        case(&|_, _, _, _, d| d.as_mut().unwrap().authority_event = None),
        [R::DesignWithoutAuthority]
    );
    assert_eq!(
        case(&|_, _, _, _, d| d.as_mut().unwrap().comparison = None),
        [R::DesignWithoutComparison]
    );
    assert_eq!(
        case(&|_, _, _, _, d| d.as_mut().unwrap().design_id = "design:forged".into()),
        [R::DesignIdentityMismatch]
    );
    let mut other_root = base_design.clone();
    other_root.parent_root = "blake3-256:another-root".into();
    other_root.design_id = design_identity(&other_root);
    assert!(
        case(&|_, _, _, _, d| *d = Some(other_root.clone())).contains(&R::DesignCandidateMismatch)
    );
}

#[test]
fn a_record_that_does_not_bind_the_container_or_leaves_a_blocker_unpermitted_is_refused() {
    let design = design();
    let record = decide(
        &policy(),
        &container(),
        &certificate(),
        &verification(),
        &integrity(),
        Some(&design),
    )
    .record
    .unwrap();
    let mut forged = record.clone();
    forged
        .permitted
        .push("REVISION_DIRTY: 3 files: permitted".into());
    assert!(!check_record(&forged, &container(), &certificate().blockers).is_empty());
    let mut other = container();
    other.census_digest = "blake3-256:other".into();
    assert!(!check_record(&record, &other, &certificate().blockers).is_empty());
    let mut open = certificate().blockers;
    open.push("REVISION_DIRTY: 3 files".into());
    assert_eq!(check_record(&record, &container(), &open).len(), 1);
}
