use super::*;
use crate::atlas::{CensusAtlas, CertificateRecord, RootManifest, UNSEALED};
use crate::identity::blake3;
use crate::verification::{ReportVerdict, VerificationPolicy, VerificationReport};
use crate::{Evidence, ExtractionDiagnostic, SemanticObservation};

#[derive(serde::Deserialize)]
struct Fixture {
    typed_records: Vec<SemanticObservation>,
    evidence: Vec<Evidence>,
    diagnostics: Vec<ExtractionDiagnostic>,
}

const ROOT_ID: &str = "blake3-256:root";

/// A container of the G147 typed fixture (records of every family).
fn container() -> CensusAtlas {
    let fixture: Fixture =
        serde_json::from_str(include_str!("../atlas/typed_fixture.json")).unwrap();
    let mut atlas = CensusAtlas {
        manifest: RootManifest {
            genome_schema: "atlas.genome.v1".into(),
            genome_hash: blake3::hash(b"genome"),
            census_digest: blake3::hash(b"census"),
            revision: "git:abc123".into(),
            certificate_id: "census-certificate:x".into(),
            seal: UNSEALED.into(),
            tool: "atlas-systemizer test".into(),
            mode: "THIN".into(),
        },
        facts: Vec::new(),
        obligations: Vec::new(),
        nodes: Vec::new(),
        edges: Vec::new(),
        certificate: CertificateRecord {
            certificate_id: "census-certificate:x".into(),
            state: "RECONCILED".into(),
            blockers: Vec::new(),
        },
        typed_records: fixture.typed_records,
        evidence: fixture.evidence,
        diagnostics: fixture.diagnostics,
    };
    atlas.canonicalize();
    atlas
}

fn report(candidate: &str, verdict: ReportVerdict) -> VerificationReport {
    serde_json::from_value(serde_json::json!({
        "schema": "atlas.verification-report.v1",
        "policy": serde_json::to_value(VerificationPolicy::library_package()).unwrap(),
        "candidate": candidate,
        "outcomes": [],
        "coverage": {},
        "failures": [],
        "verdict": verdict.as_str(),
        "blockers": if verdict == ReportVerdict::Admissible { vec![] } else { vec!["SEMANTIC unverified"] },
    }))
    .unwrap()
}

/// A VALIDATED design over the fixture container selecting its first function identity.
fn validated(container: &CensusAtlas) -> SelectedDesign {
    let root = container
        .typed_records
        .iter()
        .find(|r| r.dimension() == SemanticDimension::FunctionIdentity)
        .unwrap();
    let mut design = SelectedDesign {
        schema: DESIGN_SCHEMA_VERSION.into(),
        design_id: String::new(),
        parent_root: ROOT_ID.into(),
        candidate: container_candidate(container),
        scope: "atlas-container".into(),
        target_kind: "RUST_COMPONENT".into(),
        variant: "default".into(),
        state: DesignState::Validated,
        roots: vec![SemanticRoot {
            dimension: SemanticDimension::FunctionIdentity,
            record_id: root.record_id().as_str().to_owned(),
        }],
        bindings: vec![DesignBinding {
            name: "codec".into(),
            kind: BindingKind::StaticSelected,
            target: "core::atlas".into(),
        }],
        candidate_set: Vec::new(),
        evidence_refs: vec!["verification/self-scope.json".into()],
        authority_mode: AuthorityMode::HumanRequired,
        authority_event: None,
        rationale: "the container codec as built".into(),
        supersedes: None,
    };
    design.design_id = design_identity(&design);
    design.candidate_set = vec![design.design_id.clone()];
    design
}

fn human() -> Principal {
    Principal {
        id: "owner@example.invalid".into(),
        kind: PrincipalKind::Human,
    }
}

/// `design` SELECTED by `principal` under the design's mode.
fn selected_by(mut design: SelectedDesign, principal: Principal) -> SelectedDesign {
    design.state = DesignState::Selected;
    let mut event = AuthorityEvent {
        event_id: String::new(),
        principal,
        mode: design.authority_mode,
        design_id: design.design_id.clone(),
        generation: "G148".into(),
        statement: "select the container codec".into(),
    };
    event.event_id = event_identity(&event);
    design.authority_event = Some(event);
    design
}

fn codes(violations: &[DesignViolation]) -> Vec<&str> {
    violations.iter().map(|v| v.code.as_str()).collect()
}

fn registry(principals: Vec<Principal>) -> PrincipalRegistry {
    PrincipalRegistry {
        principals,
        ..PrincipalRegistry::empty()
    }
}

#[test]
fn a_validated_design_resolves_its_roots_against_verified_evidence() {
    let c = container();
    let ok = report(&container_candidate(&c), ReportVerdict::Admissible);
    let design = validated(&c);
    assert_eq!(
        validate(&design, &c, ROOT_ID, Some(&ok), &PrincipalRegistry::empty()),
        []
    );
    let round: SelectedDesign =
        serde_json::from_str(&serde_json::to_string(&design).unwrap()).unwrap();
    assert_eq!(round, design);
}

#[test]
fn a_design_that_does_not_resolve_in_its_container_is_refused() {
    let c = container();
    let ok = report(&container_candidate(&c), ReportVerdict::Admissible);
    let empty = PrincipalRegistry::empty();
    let refused = |edit: &dyn Fn(&mut SelectedDesign), recompute: bool| {
        let mut design = validated(&c);
        edit(&mut design);
        if recompute {
            design.design_id = design_identity(&design);
            design.candidate_set = vec![design.design_id.clone()];
        }
        codes(&validate(&design, &c, ROOT_ID, Some(&ok), &empty))
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        refused(
            &|d| d.roots[0].record_id = "semantic:FUNCTION_IDENTITY:absent".into(),
            true
        ),
        ["ROOT_UNRESOLVED"]
    );
    assert_eq!(
        refused(&|d| d.roots[0].dimension = SemanticDimension::Call, true),
        ["ROOT_UNRESOLVED"],
        "a record id under another dimension does not resolve"
    );
    assert_eq!(refused(&|d| d.roots.clear(), true), ["NO_ROOTS"]);
    assert_eq!(
        refused(&|d| d.parent_root = "blake3-256:other".into(), true),
        ["PARENT_ROOT_MISMATCH"]
    );
    assert_eq!(
        refused(&|d| d.candidate = "blake3-256:other".into(), true),
        ["CANDIDATE_MISMATCH"]
    );
    // A tampered binding that leaves the recorded identity in place.
    assert_eq!(
        refused(&|d| d.bindings[0].target = "somewhere::else".into(), false),
        ["DESIGN_ID_MISMATCH"]
    );
    assert_eq!(
        refused(&|d| d.candidate_set.clear(), false),
        ["CANDIDATE_SET_OMITS_DESIGN"]
    );
    assert_eq!(
        refused(&|d| d.evidence_refs.clear(), true),
        ["EVIDENCE_MISSING"]
    );
    // Blocked evidence, or none at all.
    let design = validated(&c);
    let blocked = report(&container_candidate(&c), ReportVerdict::Blocked);
    assert_eq!(
        codes(&validate(&design, &c, ROOT_ID, Some(&blocked), &empty)),
        ["EVIDENCE_BLOCKED"]
    );
    assert_eq!(
        codes(&validate(&design, &c, ROOT_ID, None, &empty)),
        ["EVIDENCE_MISSING"]
    );
    let elsewhere = report("blake3-256:other", ReportVerdict::Admissible);
    assert_eq!(
        codes(&validate(&design, &c, ROOT_ID, Some(&elsewhere), &empty)),
        ["EVIDENCE_OTHER_CANDIDATE"]
    );
    // A CANDIDATE needs no evidence yet.
    let mut candidate = validated(&c);
    candidate.state = DesignState::Candidate;
    assert_eq!(validate(&candidate, &c, ROOT_ID, None, &empty), []);
}

#[test]
fn only_a_registered_human_selects_and_the_event_commits_to_one_design() {
    let c = container();
    let ok = report(&container_candidate(&c), ReportVerdict::Admissible);
    let owner = registry(vec![human()]);
    let design = selected_by(validated(&c), human());
    assert_eq!(validate(&design, &c, ROOT_ID, Some(&ok), &owner), []);
    // Nobody is registered: nothing may be selected.
    assert_eq!(
        codes(&validate(
            &design,
            &c,
            ROOT_ID,
            Some(&ok),
            &PrincipalRegistry::empty()
        )),
        ["PRINCIPAL_UNREGISTERED"]
    );
    // No event at all -- even with a single candidate.
    let mut no_event = design.clone();
    no_event.authority_event = None;
    assert_eq!(
        codes(&validate(&no_event, &c, ROOT_ID, Some(&ok), &owner)),
        ["AUTHORITY_EVENT_MISSING"]
    );
    // A provider never selects, registered or not.
    let provider = Principal {
        id: "claude".into(),
        kind: PrincipalKind::Provider,
    };
    let by_provider = selected_by(validated(&c), provider.clone());
    assert_eq!(
        codes(&validate(
            &by_provider,
            &c,
            ROOT_ID,
            Some(&ok),
            &registry(vec![provider])
        )),
        ["PRINCIPAL_IS_PROVIDER"]
    );
    // A policy principal under HUMAN_REQUIRED, and POLICY_AUTO without a policy envelope.
    let policy = Principal {
        id: "self-build".into(),
        kind: PrincipalKind::Policy,
    };
    let by_policy = selected_by(validated(&c), policy.clone());
    assert_eq!(
        codes(&validate(
            &by_policy,
            &c,
            ROOT_ID,
            Some(&ok),
            &registry(vec![policy.clone()])
        )),
        ["PRINCIPAL_KIND_INADMISSIBLE"]
    );
    let mut auto = validated(&c);
    auto.authority_mode = AuthorityMode::PolicyAuto;
    let auto = selected_by(auto, policy.clone());
    assert_eq!(
        codes(&validate(
            &auto,
            &c,
            ROOT_ID,
            Some(&ok),
            &registry(vec![policy])
        )),
        ["POLICY_ENVELOPE_UNDEFINED"]
    );
    // The event names another design, another mode, or was edited after it was made.
    let mut other = design.clone();
    let event = other.authority_event.as_mut().unwrap();
    event.design_id = "blake3-256:other".into();
    event.event_id = event_identity(event);
    assert_eq!(
        codes(&validate(&other, &c, ROOT_ID, Some(&ok), &owner)),
        ["AUTHORITY_EVENT_OTHER_DESIGN"]
    );
    let mut mode = design.clone();
    let event = mode.authority_event.as_mut().unwrap();
    event.mode = AuthorityMode::Hybrid;
    event.event_id = event_identity(event);
    assert_eq!(
        codes(&validate(&mode, &c, ROOT_ID, Some(&ok), &owner)),
        ["AUTHORITY_MODE_MISMATCH"]
    );
    let mut edited = design.clone();
    edited.authority_event.as_mut().unwrap().statement = "something else".into();
    assert_eq!(
        codes(&validate(&edited, &c, ROOT_ID, Some(&ok), &owner)),
        ["AUTHORITY_EVENT_ID_MISMATCH"]
    );
    // An unresolved blocker binding, and an event on a design that is not SELECTED.
    let mut blocker = validated(&c);
    blocker.bindings[0].kind = BindingKind::UnresolvedBlocker;
    blocker.design_id = design_identity(&blocker);
    blocker.candidate_set = vec![blocker.design_id.clone()];
    let blocker = selected_by(blocker, human());
    assert_eq!(
        codes(&validate(&blocker, &c, ROOT_ID, Some(&ok), &owner)),
        ["UNRESOLVED_BLOCKER"]
    );
    let mut unexpected = design;
    unexpected.state = DesignState::Validated;
    assert_eq!(
        codes(&validate(&unexpected, &c, ROOT_ID, Some(&ok), &owner)),
        ["AUTHORITY_EVENT_UNEXPECTED"]
    );
}

#[test]
fn one_selected_design_per_coordinate_unless_superseded() {
    let c = container();
    let first = selected_by(validated(&c), human());
    let mut second = validated(&c);
    second.bindings[0].target = "core::atlas::typed".into();
    second.design_id = design_identity(&second);
    let second = selected_by(second, human());
    assert_eq!(
        codes(&coordinate_conflicts(&[first.clone(), second.clone()])),
        ["COORDINATE_CONFLICT"]
    );
    let mut replacing = second;
    replacing.supersedes = Some(first.design_id.clone());
    assert_eq!(coordinate_conflicts(&[first.clone(), replacing]), []);
    let mut other_variant = first.clone();
    other_variant.variant = "minimal".into();
    assert_eq!(coordinate_conflicts(&[first, other_variant]), []);
}

#[test]
fn identity_ignores_what_the_design_does_not_select() {
    let c = container();
    let design = validated(&c);
    let mut restated = design.clone();
    restated.rationale = "another explanation".into();
    restated.state = DesignState::Candidate;
    restated.evidence_refs.clear();
    restated.bindings.reverse();
    assert_eq!(design_identity(&restated), design.design_id);
    let mut rescoped = design.clone();
    rescoped.scope = "elsewhere".into();
    assert_ne!(design_identity(&rescoped), design.design_id);
}
