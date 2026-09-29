//! M10 falsified on the G161 fixture: a container sealed by the real seal gate is admitted; the
//! same container unsealed is refused NOT_SEALED; a tampered seal record, envelope or design is
//! refused with exactly its reason; unreadable input is refused, never a panic.

use super::*;
use crate::atlas::{CertificateRecord, RootManifest, UNSEALED, read, write};
use crate::design::{
    AuthorityEvent, AuthorityMode, EventSignature, Principal, PrincipalKey, PrincipalKind,
    PrincipalRegistry, SemanticRoot, container_candidate, event_identity, signing_message,
};
use crate::integrity::{
    EnvelopeInvariant, EnvelopeStatus, ImpactClosureRef, IntegrityReport, IntegrityVerdict,
    InvariantClass, Strength, ViolationAction,
};
use crate::seal::{
    BlockerKind, CertificateBinding, Disposition, GateInputs, SEAL_POLICY_SCHEMA_VERSION,
    ScopePolicy, ScopeVerdict, SealPolicy, gate, policy_identity, seal_identity,
};
use crate::semantic::SemanticDimension;
use crate::verification::{ReportVerdict, VerificationPolicy, VerificationReport};
use std::collections::BTreeMap;

use PreconditionRefusal as R;

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

/// The G161 fixture (runtime `seal` tests), sealed here by the real seal gate: an unsealed
/// container every gate input admits, the envelope its integrity report cites, the design a
/// fixture principal selected (a test key, never a recorded selection), and the sealed container
/// the gate's record makes of it.
struct Fixture {
    unsealed: CensusAtlas,
    sealed: CensusAtlas,
    envelope: IntegrityEnvelope,
    design: SelectedDesign,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn envelope() -> IntegrityEnvelope {
    let mut envelope = IntegrityEnvelope {
        schema_version: crate::integrity::ENVELOPE_SCHEMA_VERSION.into(),
        envelope_id: String::new(),
        subject_ref: "system:Fixture".into(),
        genome_ref: "atlas.genome.v1".into(),
        selected_design_ref: None,
        blueprint_revision_ref: None,
        status: EnvelopeStatus::Active,
        elements: Vec::new(),
        invariants: vec![EnvelopeInvariant {
            invariant_id: "invariant:fixture:core-does-not-call-runtime".into(),
            kind: InvariantClass::DependencyDirection,
            strength: Strength::Hard,
            subject_refs: vec!["Core".into(), "Runtime".into()],
            statement: "forall f: function in Core forbid call to Runtime".into(),
            rule_ref: Some("adl:core-does-not-call-runtime".into()),
            falsification_conditions: vec!["a call from Core to Runtime".into()],
            violation_action: ViolationAction::Reject,
            evidence_refs: Vec::new(),
        }],
        provenance_refs: Vec::new(),
        notes: Vec::new(),
    };
    envelope.envelope_id = envelope_identity(&envelope);
    envelope
}

fn policy() -> SealPolicy {
    let mut rules: BTreeMap<BlockerKind, Disposition> = BlockerKind::ALL
        .into_iter()
        .map(|kind| (kind, Disposition::RequiredClear))
        .collect();
    rules.insert(
        BlockerKind::DynamicDependencyObligations,
        Disposition::AcceptedBoundedResidual {
            bound: crate::seal::ResidualBound::AtMost(9),
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

fn fixture() -> Fixture {
    let mut unsealed = CensusAtlas {
        manifest: RootManifest {
            genome_schema: "atlas.genome.v1".into(),
            genome_hash: [7; 32],
            census_digest: [9; 32],
            revision: format!("git:{SHA}"),
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
    unsealed.canonicalize();
    let (_, root) = read(&write(&unsealed).unwrap()).unwrap();
    let candidate = container_candidate(&unsealed);
    let envelope = envelope();
    let policy = policy();
    let verification = VerificationReport {
        schema: "atlas.verification-report.v1".into(),
        policy: VerificationPolicy::library_package(),
        candidate: candidate.clone(),
        outcomes: Vec::new(),
        coverage: BTreeMap::new(),
        failures: Vec::new(),
        verdict: ReportVerdict::Admissible,
        blockers: Vec::new(),
    };
    let integrity = IntegrityReport {
        schema_version: crate::integrity::REPORT_SCHEMA_VERSION.into(),
        report_id: "report:fixture".into(),
        candidate_ref: format!("revision:{SHA}"),
        materialization_ref: None,
        envelope_ref: envelope.envelope_id.clone(),
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
    let mut design = SelectedDesign {
        schema: "atlas.selected-design.v1".into(),
        design_id: String::new(),
        parent_root: root.as_str().to_owned(),
        candidate,
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
        rationale: "test fixture".into(),
        supersedes: None,
        comparison: Some("comparison:fixture".into()),
    };
    design.design_id = design_identity(&design);
    let principal = Principal {
        id: "fixture-principal".into(),
        kind: PrincipalKind::Human,
    };
    let mut event = AuthorityEvent {
        event_id: String::new(),
        principal: principal.clone(),
        mode: AuthorityMode::HumanRequired,
        design_id: design.design_id.clone(),
        generation: "G161".into(),
        statement: "test fixture".into(),
        signature: None,
    };
    event.event_id = event_identity(&event);
    let key = ed25519_compact::KeyPair::from_seed(ed25519_compact::Seed::new([11; 32]));
    event.signature = Some(EventSignature {
        public_key: hex(&key.pk[..]),
        signature: hex(&key.sk.sign(signing_message(&event).as_bytes(), None)[..]),
    });
    design.authority_event = Some(event);
    let registry = PrincipalRegistry {
        principals: vec![principal.clone()],
        keys: vec![PrincipalKey {
            principal: principal.id,
            algorithm: "ed25519".into(),
            public_key: hex(&key.pk[..]),
        }],
        ..PrincipalRegistry::empty()
    };
    let mut container = seal_binding(&unsealed.manifest);
    container.root_id = root.as_str().to_owned();
    let certificate = CertificateBinding {
        certificate_id: unsealed.certificate.certificate_id.clone(),
        state: unsealed.certificate.state.clone(),
        blockers: unsealed.certificate.blockers.clone(),
    };
    let eligibility = gate(&GateInputs {
        policy: &policy,
        container: &container,
        certificate: &certificate,
        verification: &verification,
        integrity: &integrity,
        design: Some(&design),
        registry: &registry,
    });
    assert_eq!(
        eligibility.verdict,
        ScopeVerdict::Eligible,
        "{:?}",
        eligibility.reasons
    );
    let mut sealed = unsealed.clone();
    sealed.manifest.seal = SEALED.into();
    sealed.seal = eligibility.record;
    Fixture {
        unsealed,
        sealed,
        envelope,
        design,
    }
}

fn json<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

/// The precondition over the encoded `parent`, `envelope` and `design`.
fn run(
    parent: &[u8],
    envelope: &IntegrityEnvelope,
    design: Option<&SelectedDesign>,
) -> Precondition {
    let design = design.map(json);
    precondition(&PreconditionInputs {
        parent,
        envelope: &json(envelope),
        design: design.as_deref(),
    })
}

fn kinds(verdict: &Precondition) -> Vec<PreconditionRefusal> {
    verdict.reasons.iter().map(|(r, _)| *r).collect()
}

/// `sealed` with its seal record edited and re-stamped, so the record still binds the container
/// and the M9 writer and reader accept it.
fn restamped(sealed: &CensusAtlas, edit: impl Fn(&mut SealRecord)) -> Vec<u8> {
    let mut atlas = sealed.clone();
    let record = atlas.seal.as_mut().unwrap();
    edit(record);
    record.seal_id = seal_identity(record);
    write(&atlas).expect("a re-stamped record still binds the container")
}

fn restamp_design(design: &mut SelectedDesign) {
    design.design_id = design_identity(design);
}

#[test]
fn the_g161_sealed_container_is_admitted_with_the_identities_it_binds() {
    let f = fixture();
    let bytes = write(&f.sealed).unwrap();
    let (_, root) = read(&bytes).unwrap();
    let verdict = run(&bytes, &f.envelope, Some(&f.design));
    assert_eq!(
        verdict.verdict,
        PreconditionVerdict::Admitted,
        "{verdict:?}"
    );
    assert!(verdict.reasons.is_empty());
    let record = f.sealed.seal.as_ref().unwrap();
    assert_eq!(
        verdict.admitted,
        Some(AdmittedParent {
            parent_root: root.as_str().to_owned(),
            census_digest: container_candidate(&f.sealed),
            revision: format!("git:{SHA}"),
            scope: "fixture".into(),
            seal_id: record.seal_id.clone(),
            envelope_id: f.envelope.envelope_id.clone(),
            design_id: f.design.design_id.clone(),
        })
    );
    assert_eq!(record.integrity_envelope, f.envelope.envelope_id);
    assert_eq!(record.design_id, f.design.design_id);
    assert_eq!(verdict.schema, PRECONDITION_SCHEMA_VERSION);
    assert_eq!(verdict.not_verified, NOT_VERIFIED);
    // The typed entry decides the same.
    let (decoded, _) = read(&bytes).unwrap();
    assert_eq!(
        admit(&decoded, root.as_str(), &f.envelope, Some(&f.design)),
        verdict
    );
    // Its verdict is typed vocabulary on the wire.
    let text = serde_json::to_value(&verdict).unwrap();
    assert_eq!(text["verdict"], "ADMITTED");
}

#[test]
fn the_unsealed_container_is_refused_not_sealed() {
    let f = fixture();
    let verdict = run(&write(&f.unsealed).unwrap(), &f.envelope, Some(&f.design));
    assert_eq!(verdict.verdict, PreconditionVerdict::Refused);
    assert_eq!(kinds(&verdict), [R::NotSealed]);
    assert_eq!(
        verdict.reasons[0].1,
        "seal status UNSEALED_CENSUS_CONTAINER"
    );
    assert_eq!(verdict.admitted, None);
    let text = serde_json::to_value(&verdict).unwrap();
    assert_eq!(text["verdict"], "REFUSED");
    assert_eq!(text["reasons"][0][0], "NOT_SEALED");
}

#[test]
fn a_seal_record_that_does_not_bind_the_parent_is_refused_seal_record_unbound() {
    let f = fixture();
    let (decoded, root) = read(&write(&f.sealed).unwrap()).unwrap();
    // The reader already rejects such a record, so the precondition's own re-check is reached
    // only through the typed entry: a record forged, re-stamped for another revision, or left
    // behind by a blocker the certificate gained after the seal.
    let forged = {
        let mut atlas = decoded.clone();
        atlas.seal.as_mut().unwrap().seal_id = "blake3-256:forged".into();
        atlas
    };
    let moved = {
        let mut atlas = decoded.clone();
        let record = atlas.seal.as_mut().unwrap();
        record.revision = "git:fedcba9876543210fedcba9876543210fedcba98".into();
        record.seal_id = seal_identity(record);
        atlas
    };
    let reopened = {
        let mut atlas = decoded.clone();
        atlas
            .certificate
            .blockers
            .push("COVERAGE_UNKNOWN: CALL".into());
        atlas
    };
    for (atlas, detail) in [
        (&forged, "the seal identity does not verify"),
        (
            &moved,
            "the seal binds another certificate, census or revision",
        ),
        (
            &reopened,
            "blocker `COVERAGE_UNKNOWN: CALL` is open and the seal did not permit it",
        ),
    ] {
        let verdict = admit(atlas, root.as_str(), &f.envelope, Some(&f.design));
        assert_eq!(kinds(&verdict), [R::SealRecordUnbound], "{detail}");
        assert_eq!(verdict.reasons[0].1, detail);
        assert_eq!(verdict.admitted, None);
        // As bytes, the M9 reader rejects it first: UNREADABLE, naming the seal.
        assert!(write(atlas).is_err(), "{detail}");
    }
    // A tampered byte of the seal record never reads back.
    let mut bytes = write(&f.sealed).unwrap();
    let seal_id = f.sealed.seal.as_ref().unwrap().seal_id.as_bytes();
    let at = bytes
        .windows(seal_id.len())
        .position(|w| w == seal_id)
        .expect("the seal id is in the seal section");
    bytes[at + seal_id.len() - 1] ^= 1;
    let verdict = run(&bytes, &f.envelope, Some(&f.design));
    assert_eq!(kinds(&verdict), [R::Unreadable]);
    assert!(verdict.reasons[0].1.starts_with("parent: ATLAS_REJECTED"));
}

#[test]
fn a_re_stamped_seal_record_naming_other_inputs_is_refused_with_their_mismatch() {
    let f = fixture();
    // The record re-stamped to name another envelope: it binds the container, so it reads back,
    // but the pinned envelope is not the one it names.
    let other_envelope = restamped(&f.sealed, |r| {
        r.integrity_envelope = "envelope:other".into()
    });
    let verdict = run(&other_envelope, &f.envelope, Some(&f.design));
    assert_eq!(kinds(&verdict), [R::EnvelopeMismatch]);
    // Another design id.
    let other_design = restamped(&f.sealed, |r| r.design_id = "blake3-256:other".into());
    let verdict = run(&other_design, &f.envelope, Some(&f.design));
    assert_eq!(kinds(&verdict), [R::DesignMismatch]);
    // A record naming a verified design for another census, or for another scope.
    for edit in [
        (|d: &mut SelectedDesign| d.candidate = "blake3-256:other".into())
            as fn(&mut SelectedDesign),
        |d| d.scope = "other".into(),
    ] {
        let mut design = f.design.clone();
        edit(&mut design);
        restamp_design(&mut design);
        let bytes = restamped(&f.sealed, |r| r.design_id = design.design_id.clone());
        let verdict = run(&bytes, &f.envelope, Some(&design));
        assert_eq!(kinds(&verdict), [R::DesignMismatch], "{design:?}");
    }
}

#[test]
fn a_tampered_envelope_is_refused_with_exactly_its_reason() {
    let f = fixture();
    let bytes = write(&f.sealed).unwrap();
    // An invariant weakened under the pinned identity: the identity no longer verifies.
    let mut weakened = f.envelope.clone();
    weakened.invariants[0].strength = Strength::Soft;
    let verdict = run(&bytes, &weakened, Some(&f.design));
    assert_eq!(kinds(&verdict), [R::EnvelopeUnverified]);
    // Weakened and re-stamped: it verifies, but it is not the envelope the seal names.
    weakened.envelope_id = envelope_identity(&weakened);
    let verdict = run(&bytes, &weakened, Some(&f.design));
    assert_eq!(kinds(&verdict), [R::EnvelopeMismatch]);
    assert_eq!(
        verdict.reasons[0].1,
        format!(
            "the seal names envelope {}, not {}",
            f.envelope.envelope_id, weakened.envelope_id
        )
    );
}

#[test]
fn a_tampered_design_is_refused_with_exactly_its_reason() {
    let f = fixture();
    let bytes = write(&f.sealed).unwrap();
    // A root added under the selected identity: the identity no longer verifies.
    let mut widened = f.design.clone();
    widened.roots.push(SemanticRoot {
        dimension: SemanticDimension::FunctionIdentity,
        record_id: "semantic:FUNCTION_IDENTITY:injected".into(),
    });
    let verdict = run(&bytes, &f.envelope, Some(&widened));
    assert_eq!(kinds(&verdict), [R::DesignUnverified]);
    // Widened and re-stamped: it verifies, but it is not the design the seal names.
    restamp_design(&mut widened);
    let verdict = run(&bytes, &f.envelope, Some(&widened));
    assert_eq!(kinds(&verdict), [R::DesignMismatch]);
    // The sealed design itself, superseded since (state is not part of its identity).
    let mut superseded = f.design.clone();
    superseded.state = DesignState::Superseded;
    let verdict = run(&bytes, &f.envelope, Some(&superseded));
    assert_eq!(kinds(&verdict), [R::DesignNotSelected]);
    // No design at all, as bytes or decoded.
    let verdict = run(&bytes, &f.envelope, None);
    assert_eq!(kinds(&verdict), [R::DesignAbsent]);
    let (decoded, root) = read(&bytes).unwrap();
    assert_eq!(admit(&decoded, root.as_str(), &f.envelope, None), verdict);
}

#[test]
fn unreadable_input_is_refused_never_a_panic() {
    let f = fixture();
    let bytes = write(&f.sealed).unwrap();
    let envelope = json(&f.envelope);
    let design = json(&f.design);
    // Every truncation of the sealed container, and bytes that are not a container at all.
    for len in 0..bytes.len() {
        let verdict = precondition(&PreconditionInputs {
            parent: &bytes[..len],
            envelope: &envelope,
            design: Some(&design),
        });
        assert_eq!(kinds(&verdict), [R::Unreadable], "prefix {len}");
    }
    let verdict = precondition(&PreconditionInputs {
        parent: b"not a container",
        envelope: b"{",
        design: Some(b"[]"),
    });
    assert_eq!(verdict.verdict, PreconditionVerdict::Refused);
    let details: Vec<&str> = verdict.reasons.iter().map(|(_, d)| d.as_str()).collect();
    assert_eq!(kinds(&verdict), [R::Unreadable; 3]);
    assert!(details[0].starts_with("design: "), "{details:?}");
    assert!(details[1].starts_with("envelope: "), "{details:?}");
    assert!(
        details[2].starts_with("parent: ATLAS_REJECTED"),
        "{details:?}"
    );
}

#[test]
fn every_reason_is_collected_in_a_deterministic_order() {
    let f = fixture();
    let mut weakened = f.envelope.clone();
    weakened.invariants.clear();
    let mut superseded = f.design.clone();
    superseded.state = DesignState::Superseded;
    superseded.rationale = "changed".into();
    superseded.design_id = "blake3-256:forged".into();
    let unsealed = write(&f.unsealed).unwrap();
    let verdict = run(&unsealed, &weakened, Some(&superseded));
    assert_eq!(
        kinds(&verdict),
        [
            R::NotSealed,
            R::EnvelopeUnverified,
            R::DesignUnverified,
            R::DesignNotSelected
        ]
    );
    assert_eq!(verdict, run(&unsealed, &weakened, Some(&superseded)));
    // Against the sealed parent the forged design id also mismatches the one the seal names (the
    // envelope still carries the named id, so only its identity fails).
    let verdict = run(&write(&f.sealed).unwrap(), &weakened, Some(&superseded));
    assert_eq!(
        kinds(&verdict),
        [
            R::EnvelopeUnverified,
            R::DesignUnverified,
            R::DesignMismatch,
            R::DesignNotSelected
        ]
    );
}
