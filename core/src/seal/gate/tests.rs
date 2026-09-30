use super::super::{
    BlockerKind, Disposition, ResidualBound, SEAL_POLICY_SCHEMA_VERSION, ScopePolicy,
};
use super::*;
use crate::design::{
    AuthorityEvent, AuthorityMode, EventSignature, Principal, PrincipalKey, PrincipalKind,
    PrincipalRegistry, SelectedDesign, event_identity, signing_message,
};
use crate::integrity::IntegrityVerdict;
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

/// Written by the real evaluator (G185: the gate refuses a report its contents contradict).
fn verification() -> VerificationReport {
    crate::atlasx::fixture::verification(DIGEST)
}

/// Written by the real evaluator against the pinned envelope.
fn integrity() -> IntegrityReport {
    crate::atlasx::fixture::integrity_for(&envelope(), "revision:abc123", DIGEST)
}

fn envelope() -> crate::integrity::IntegrityEnvelope {
    crate::atlasx::fixture::envelope()
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
        signature: None,
    };
    event.event_id = event_identity(&event);
    sign(&mut event, &principal_key());
    design.authority_event = Some(event);
    design
}

/// The fixture principal's key; only tests sign, through the test-only implementation.
fn principal_key() -> ed25519_compact::KeyPair {
    ed25519_compact::KeyPair::from_seed(ed25519_compact::Seed::new([9; 32]))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sign(event: &mut AuthorityEvent, key: &ed25519_compact::KeyPair) {
    event.signature = Some(EventSignature {
        public_key: hex(&key.pk[..]),
        signature: hex(&key.sk.sign(signing_message(event).as_bytes(), None)[..]),
    });
}

/// The fixture principal, declared with its key.
fn registry() -> PrincipalRegistry {
    PrincipalRegistry {
        principals: vec![Principal {
            id: "fixture-principal".into(),
            kind: PrincipalKind::Human,
        }],
        keys: vec![PrincipalKey {
            principal: "fixture-principal".into(),
            algorithm: "ed25519".into(),
            public_key: hex(&principal_key().pk[..]),
        }],
        ..PrincipalRegistry::empty()
    }
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
        envelope: &envelope(),
        design,
        registry: &registry(),
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

/// G171 falsification: the gate once asked only whether an authority event existed, so a
/// design "selected" by a provider, by an undeclared principal, or by an unsigned or forged event
/// was ELIGIBLE. Each is now refused with DESIGN_AUTHORITY_REFUSED and the design's own codes.
#[test]
fn a_design_without_selection_authority_does_not_seal() {
    let reselect = |edit: &dyn Fn(&mut AuthorityEvent)| {
        let mut d = design();
        let mut event = d.authority_event.take().unwrap();
        edit(&mut event);
        event.event_id = event_identity(&event);
        d.authority_event = Some(event);
        d
    };
    let refused = |d: &SelectedDesign| {
        let e = decide(
            &policy(),
            &container(),
            &certificate(),
            &verification(),
            &integrity(),
            Some(d),
        );
        assert_eq!(e.verdict, ScopeVerdict::NotEligible);
        assert!(e.record.is_none());
        assert_eq!(reasons(&e), [IneligibleReason::DesignAuthorityRefused]);
        e.reasons[0].1.clone()
    };
    let provider = reselect(&|event| {
        event.principal.kind = PrincipalKind::Provider;
        sign(event, &principal_key());
    });
    assert!(refused(&provider).contains("PRINCIPAL_IS_PROVIDER"));
    let stranger = reselect(&|event| {
        event.principal.id = "stranger".into();
        sign(event, &principal_key());
    });
    let codes = refused(&stranger);
    assert!(codes.contains("PRINCIPAL_UNREGISTERED") && codes.contains("SIGNING_KEY_UNDECLARED"));
    let unsigned = reselect(&|event| event.signature = None);
    assert_eq!(refused(&unsigned), "AUTHORITY_EVENT_UNSIGNED");
    // Signed, then the statement edited: the identity moves and the signature no longer covers it.
    let forged = reselect(&|event| event.statement = "a different decision".into());
    assert_eq!(refused(&forged), "SIGNATURE_INVALID");
    let other_key = reselect(&|event| {
        sign(
            event,
            &ed25519_compact::KeyPair::from_seed(ed25519_compact::Seed::new([3; 32])),
        )
    });
    assert_eq!(refused(&other_key), "SIGNING_KEY_UNDECLARED");
}

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
        case(
            &|_, _, v, _, _| *v = crate::atlasx::fixture::verification_with(
                DIGEST,
                crate::verification::EvidenceResult::Violated
            )
        ),
        [R::VerificationBlocked]
    );
    // G185: a verdict the report's own outcomes contradict, either way.
    assert_eq!(
        case(&|_, _, v, _, _| v.verdict = ReportVerdict::Blocked),
        [R::VerificationBlocked, R::VerificationReportInconsistent]
    );
    assert_eq!(
        case(&|_, _, v, _, _| v
            .blockers
            .push("UNIT: 12 required unit obligations fail".into())),
        [R::VerificationReportInconsistent]
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
        case(&|_, _, _, i, _| {
            i.evaluations[0].status = crate::integrity::EvaluationStatus::Unknown;
            i.evaluations[0].diagnostic_codes =
                vec![crate::integrity::ArchitectureDiagnostic::RequiredUnknown];
            i.required_unknown_count = 1;
            i.verdict = IntegrityVerdict::Incomplete;
        }),
        [R::IntegrityNotEligible]
    );
    // G185: the review's P1 report -- a HARD violation counted as none, verdict ELIGIBLE -- and
    // a report citing another envelope.
    assert_eq!(
        case(&|_, _, _, i, _| {
            i.evaluations[0].status = crate::integrity::EvaluationStatus::Fail;
            i.evaluations[0].diagnostic_codes =
                vec![crate::integrity::ArchitectureDiagnostic::HardViolation];
        }),
        [
            R::IntegrityReportInconsistent,
            R::IntegrityReportInconsistent
        ]
    );
    assert_eq!(
        case(&|_, _, _, i, _| i.envelope_ref = "envelope:other".into()),
        [R::IntegrityReportInconsistent]
    );
    assert_eq!(
        case(&|_, _, _, i, _| i.verdict = IntegrityVerdict::Incomplete),
        [R::IntegrityNotEligible, R::IntegrityReportInconsistent]
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
        // G171: the signed event still selects the original identity, so its authority is refused.
        [R::DesignAuthorityRefused, R::DesignIdentityMismatch]
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
