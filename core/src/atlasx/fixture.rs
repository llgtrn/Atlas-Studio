//! The AtlasX parent fixture shared by the precondition (M10) and materializer (M11) tests: the
//! G161 fixture container, carrying real census records of `MaterializationMode::is_local`
//! (`materialize/census_records.json`, copied from the G184 self census), sealed by the real seal
//! gate over every input it joins. The design is selected by a fixture principal signing with a
//! test key (ed25519-compact, a dev-dependency), never a recorded selection: Atlas signs nothing.

use crate::Evidence;
use crate::SemanticObservation;
use crate::atlas::{
    CensusAtlas, CertificateRecord, RootManifest, SEALED, UNSEALED, read, seal_binding, write,
};
use crate::atlasx::precondition::Declared;
use crate::design::{
    AuthorityEvent, AuthorityMode, DesignState, EventSignature, Principal, PrincipalKey,
    PrincipalKind, PrincipalRegistry, SelectedDesign, SemanticRoot, container_candidate,
    design_identity, event_identity, signing_message,
};
use crate::integrity::{
    EnvelopeInvariant, EnvelopeStatus, IntegrityEnvelope, IntegrityReport, InvariantClass,
    Strength, ViolationAction, envelope_identity,
};
use crate::seal::{
    BlockerKind, CertificateBinding, Disposition, GateInputs, SEAL_POLICY_SCHEMA_VERSION,
    ScopePolicy, ScopeVerdict, SealPolicy, gate, policy_identity,
};
use crate::semantic::SemanticDimension;
use crate::verification::{
    EvidenceResult, Producer, VerificationEvidence, VerificationObligation, VerificationPlan,
    VerificationPolicy, VerificationReport,
};
use serde::Deserialize;
use std::collections::BTreeMap;

/// The revision the fixture's census records were observed at.
pub const SHA: &str = "c5fc38bd6acb2c8352da02c5202ba91af5cd9ee8";
/// The FUNCTION_IDENTITY record of `MaterializationMode::is_local`.
pub const IS_LOCAL_IDENTITY: &str = "semantic:FUNCTION_IDENTITY:f9cc9be4f375986b";
/// Its FUNCTION_SIGNATURE record.
pub const IS_LOCAL_SIGNATURE: &str = "semantic:FUNCTION_SIGNATURE:43bada38908117fd";
/// The SYMBOL record of the `MaterializationMode` definition.
pub const MODE_SYMBOL: &str = "semantic:SYMBOL:02255955f298ded0";

pub struct Fixture {
    pub unsealed: CensusAtlas,
    pub sealed: CensusAtlas,
    pub design: SelectedDesign,
    pub verification: VerificationReport,
    pub integrity: IntegrityReport,
    /// The declarations the fixture repository makes: its policy, pinned envelope and principals.
    pub declared: Declared,
}

/// The verification report the real evaluator writes for `candidate` from one OBSERVED,
/// satisfying run per required class of the library-package policy.
pub fn verification(candidate: &str) -> VerificationReport {
    verification_with(candidate, EvidenceResult::Satisfied)
}

/// As `verification`, the UNIT run giving `unit`.
pub fn verification_with(candidate: &str, unit: EvidenceResult) -> VerificationReport {
    let policy = VerificationPolicy::library_package();
    let obligations: Vec<VerificationObligation> = policy
        .required_classes
        .iter()
        .map(|class| VerificationObligation {
            id: format!("fixture:{}", class.as_str()),
            class: *class,
            subject: "fixture".into(),
            statement: format!("the fixture's {} obligation", class.as_str()),
            required: true,
        })
        .collect();
    let evidence: Vec<VerificationEvidence> = obligations
        .iter()
        .map(|o| VerificationEvidence {
            id: format!("evidence:{}", o.id),
            obligations: vec![o.id.clone()],
            producer: Producer {
                tool: "fixture".into(),
                version: "1".into(),
            },
            run_id: format!("run:{}", o.id),
            candidate: candidate.into(),
            environment: "fixture".into(),
            inputs: Vec::new(),
            result: if o.class == crate::verification::VerificationClass::Unit {
                unit
            } else {
                EvidenceResult::Satisfied
            },
            content_hash: format!("fixture:{}", o.id),
            counterexample: (unit == EvidenceResult::Violated).then(|| "case 1".into()),
            status: crate::EpistemicStatus::Observed,
        })
        .collect();
    crate::verification::evaluate(
        &VerificationPlan {
            policy,
            candidate: candidate.into(),
            obligations,
        },
        &evidence,
    )
}

/// The integrity report the real evaluator writes for `candidate` against the pinned envelope,
/// its one invariant decided satisfied.
pub fn integrity(envelope: &IntegrityEnvelope, candidate: &str) -> IntegrityReport {
    integrity_for(envelope, &format!("revision:{SHA}"), candidate)
}

/// As `integrity`, for the revision `candidate_ref`.
pub fn integrity_for(
    envelope: &IntegrityEnvelope,
    candidate_ref: &str,
    candidate: &str,
) -> IntegrityReport {
    let satisfied = crate::language::adl::ConstraintResult {
        name: "core-does-not-call-runtime".into(),
        passed: true,
        verdict: crate::constraint::ConstraintVerdict::Satisfied,
        diagnostics: Vec::new(),
        derivation: Vec::new(),
    };
    crate::integrity::evaluate(envelope, envelope, &[satisfied], candidate_ref, candidate)
}

#[derive(Deserialize)]
struct CensusRecords {
    typed_records: Vec<SemanticObservation>,
    evidence: Vec<Evidence>,
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn envelope() -> IntegrityEnvelope {
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

pub fn policy() -> SealPolicy {
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

/// The fixture principal's key pair, from a fixed test seed.
pub fn key() -> ed25519_compact::KeyPair {
    ed25519_compact::KeyPair::from_seed(ed25519_compact::Seed::new([11; 32]))
}

/// `design` with its identity restamped and a fresh authority event the fixture principal signs.
pub fn select(design: &mut SelectedDesign) {
    design.authority_event = None;
    design.design_id = design_identity(design);
    let mut event = AuthorityEvent {
        event_id: String::new(),
        principal: principal(),
        mode: AuthorityMode::HumanRequired,
        design_id: design.design_id.clone(),
        generation: "G161".into(),
        statement: "test fixture".into(),
        signature: None,
    };
    event.event_id = event_identity(&event);
    let key = key();
    event.signature = Some(EventSignature {
        public_key: hex(&key.pk[..]),
        signature: hex(&key.sk.sign(signing_message(&event).as_bytes(), None)[..]),
    });
    design.authority_event = Some(event);
}

fn principal() -> Principal {
    Principal {
        id: "fixture-principal".into(),
        kind: PrincipalKind::Human,
    }
}

/// The unsealed fixture container carrying the real census records.
pub fn unsealed() -> CensusAtlas {
    let records: CensusRecords =
        serde_json::from_str(include_str!("materialize/census_records.json"))
            .expect("the census records parse");
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
        typed_records: records.typed_records,
        evidence: records.evidence,
        diagnostics: Vec::new(),
        seal: None,
    };
    unsealed.canonicalize();
    unsealed
}

/// The design over `unsealed` selecting `roots`, signed by the fixture principal.
pub fn design_over(unsealed: &CensusAtlas, roots: Vec<SemanticRoot>) -> SelectedDesign {
    let (_, root) = read(&write(unsealed).unwrap()).unwrap();
    let mut design = SelectedDesign {
        schema: "atlas.selected-design.v1".into(),
        design_id: String::new(),
        parent_root: root.as_str().to_owned(),
        candidate: container_candidate(unsealed),
        scope: "fixture".into(),
        target_kind: "LIBRARY_PACKAGE".into(),
        variant: "default".into(),
        state: DesignState::Selected,
        roots,
        bindings: Vec::new(),
        candidate_set: Vec::new(),
        evidence_refs: Vec::new(),
        authority_mode: AuthorityMode::HumanRequired,
        authority_event: None,
        rationale: "test fixture".into(),
        supersedes: None,
        comparison: Some("comparison:fixture".into()),
    };
    select(&mut design);
    design
}

/// The roots the fixture design selects: `is_local`'s identity and its type's definition.
pub fn roots() -> Vec<SemanticRoot> {
    vec![
        SemanticRoot {
            dimension: SemanticDimension::FunctionIdentity,
            record_id: IS_LOCAL_IDENTITY.into(),
        },
        SemanticRoot {
            dimension: SemanticDimension::Symbol,
            record_id: MODE_SYMBOL.into(),
        },
    ]
}

/// `unsealed` sealed by the record the real gate decides over the fixture's inputs and `design`.
pub fn seal(
    unsealed: &CensusAtlas,
    design: &SelectedDesign,
    f: &Fixture,
) -> Result<CensusAtlas, Vec<String>> {
    let (_, root) = read(&write(unsealed).unwrap()).unwrap();
    let mut container = seal_binding(&unsealed.manifest);
    container.root_id = root.as_str().to_owned();
    let certificate = CertificateBinding {
        certificate_id: unsealed.certificate.certificate_id.clone(),
        state: unsealed.certificate.state.clone(),
        blockers: unsealed.certificate.blockers.clone(),
    };
    let eligibility = gate(&GateInputs {
        policy: &f.declared.policy,
        container: &container,
        certificate: &certificate,
        verification: &f.verification,
        integrity: &f.integrity,
        envelope: &f.declared.envelope,
        design: Some(design),
        registry: &f.declared.registry,
    });
    if eligibility.verdict != ScopeVerdict::Eligible {
        return Err(eligibility
            .reasons
            .iter()
            .map(|(r, d)| format!("{r}: {d}"))
            .collect());
    }
    let mut sealed = unsealed.clone();
    sealed.manifest.seal = SEALED.into();
    sealed.seal = eligibility.record;
    Ok(sealed)
}

pub fn fixture() -> Fixture {
    let unsealed = unsealed();
    let candidate = container_candidate(&unsealed);
    let envelope = envelope();
    let verification = verification(&candidate);
    let integrity = integrity(&envelope, &candidate);
    let key = key();
    let registry = PrincipalRegistry {
        principals: vec![principal()],
        keys: vec![PrincipalKey {
            principal: principal().id,
            algorithm: "ed25519".into(),
            public_key: hex(&key.pk[..]),
        }],
        ..PrincipalRegistry::empty()
    };
    let design = design_over(&unsealed, roots());
    let mut f = Fixture {
        sealed: unsealed.clone(),
        unsealed,
        design,
        verification,
        integrity,
        declared: Declared {
            policy: policy(),
            envelope,
            registry,
        },
    };
    f.sealed = seal(&f.unsealed, &f.design, &f).expect("the fixture is ELIGIBLE");
    f
}
