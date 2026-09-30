//! M10 falsified on the G161 fixture: a container sealed by the real seal gate is admitted; the
//! same container unsealed is refused NOT_SEALED; a tampered seal record, envelope or design is
//! refused with exactly its reason; unreadable input is refused, never a panic. G185: a seal
//! record the gate did not decide -- the G179 self-certification -- is refused, and so is every
//! gate input that does not decide the record the container carries.

use super::*;
use crate::atlas::{read, write};
use crate::atlasx::fixture::{self, Fixture, SHA, fixture};
use crate::design::{PrincipalRegistry, SemanticRoot, container_candidate};
use crate::integrity::{ArchitectureDiagnostic, EvaluationStatus, IntegrityVerdict, Strength};
use crate::seal::seal_identity;
use crate::semantic::SemanticDimension;
use crate::verification::{EvidenceResult, ReportVerdict};

use PreconditionRefusal as R;

fn json<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

/// The precondition over the encoded `parent` and `design`, with `envelope` as the pinned one
/// and the fixture's reports, policy and registry.
fn run(
    parent: &[u8],
    envelope: &IntegrityEnvelope,
    design: Option<&SelectedDesign>,
) -> Precondition {
    run_with(&fixture(), parent, envelope, design)
}

/// The precondition with the reports and declaration of `f`, `envelope` pinned.
fn run_with(
    f: &Fixture,
    parent: &[u8],
    envelope: &IntegrityEnvelope,
    design: Option<&SelectedDesign>,
) -> Precondition {
    let design = design.map(json);
    precondition(&PreconditionInputs {
        parent,
        design: design.as_deref(),
        verification: &json(&f.verification),
        integrity: &json(&f.integrity),
        declaration: &Declaration {
            policy: &f.declared.policy,
            envelope,
            registry: &f.declared.registry,
        },
    })
}

fn reports(f: &Fixture) -> Reports<'_> {
    Reports {
        verification: &f.verification,
        integrity: &f.integrity,
    }
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
    let verdict = run(&bytes, &f.declared.envelope, Some(&f.design));
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
            envelope_id: f.declared.envelope.envelope_id.clone(),
            design_id: f.design.design_id.clone(),
            policy_id: f.declared.policy.policy_id.clone(),
            principal: "fixture-principal".into(),
            principal_key: fixture::hex(&fixture::key().pk[..]),
            registry_digest: IntegrityDigest::of_bytes(&json(&f.declared.registry))
                .as_str()
                .to_owned(),
        })
    );
    // G185: the admission names who selected, under which declaration: another registry
    // declaring the same principal and key gives another digest.
    let mut g = fixture();
    g.declared.registry.note = "another declaration".into();
    let other = run_with(&g, &bytes, &g.declared.envelope, Some(&g.design));
    assert_ne!(
        other.admitted.unwrap().registry_digest,
        verdict.admitted.as_ref().unwrap().registry_digest
    );
    assert_eq!(record.integrity_envelope, f.declared.envelope.envelope_id);
    assert_eq!(record.design_id, f.design.design_id);
    assert_eq!(verdict.schema, PRECONDITION_SCHEMA_VERSION);
    assert_eq!(verdict.not_verified, NOT_VERIFIED);
    // The typed entry decides the same.
    let (decoded, _) = read(&bytes).unwrap();
    assert_eq!(
        admit(
            &decoded,
            root.as_str(),
            Some(&f.design),
            &reports(&f),
            &f.declared.declaration()
        ),
        verdict
    );
    // Its verdict is typed vocabulary on the wire.
    let text = serde_json::to_value(&verdict).unwrap();
    assert_eq!(text["verdict"], "ADMITTED");
}

#[test]
fn the_unsealed_container_is_refused_not_sealed() {
    let f = fixture();
    let verdict = run(
        &write(&f.unsealed).unwrap(),
        &f.declared.envelope,
        Some(&f.design),
    );
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
        let verdict = admit(
            atlas,
            root.as_str(),
            Some(&f.design),
            &reports(&f),
            &f.declared.declaration(),
        );
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
    let verdict = run(&bytes, &f.declared.envelope, Some(&f.design));
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
    let verdict = run(&other_envelope, &f.declared.envelope, Some(&f.design));
    assert_eq!(kinds(&verdict), [R::EnvelopeMismatch]);
    // Another design id.
    let other_design = restamped(&f.sealed, |r| r.design_id = "blake3-256:other".into());
    let verdict = run(&other_design, &f.declared.envelope, Some(&f.design));
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
        let verdict = run(&bytes, &f.declared.envelope, Some(&design));
        assert_eq!(kinds(&verdict), [R::DesignMismatch], "{design:?}");
    }
}

#[test]
fn a_tampered_envelope_is_refused_with_exactly_its_reason() {
    let f = fixture();
    let bytes = write(&f.sealed).unwrap();
    // The pinned envelope weakened under its identity: the identity no longer verifies, and the
    // integrity report no longer cites a verified pin.
    let mut weakened = f.declared.envelope.clone();
    weakened.invariants[0].strength = Strength::Soft;
    let verdict = run(&bytes, &weakened, Some(&f.design));
    assert_eq!(
        kinds(&verdict),
        [R::EnvelopeUnverified, R::IntegrityReportInconsistent]
    );
    // Weakened and re-stamped: it verifies, but it is not the envelope the seal names.
    weakened.envelope_id = envelope_identity(&weakened);
    let verdict = run(&bytes, &weakened, Some(&f.design));
    assert_eq!(
        kinds(&verdict),
        [R::EnvelopeMismatch, R::IntegrityReportInconsistent]
    );
    assert_eq!(
        verdict.reasons[0].1,
        format!(
            "the seal names envelope {}, the repository pins {}",
            f.declared.envelope.envelope_id, weakened.envelope_id
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
    let verdict = run(&bytes, &f.declared.envelope, Some(&widened));
    assert_eq!(kinds(&verdict), [R::DesignUnverified]);
    // Widened and re-stamped: it verifies, but it is not the design the seal names.
    restamp_design(&mut widened);
    let verdict = run(&bytes, &f.declared.envelope, Some(&widened));
    assert_eq!(kinds(&verdict), [R::DesignMismatch]);
    // The sealed design itself, superseded since (state is not part of its identity).
    let mut superseded = f.design.clone();
    superseded.state = DesignState::Superseded;
    let verdict = run(&bytes, &f.declared.envelope, Some(&superseded));
    assert_eq!(kinds(&verdict), [R::DesignNotSelected]);
    // No design at all, as bytes or decoded.
    let verdict = run(&bytes, &f.declared.envelope, None);
    assert_eq!(kinds(&verdict), [R::DesignAbsent]);
    let (decoded, root) = read(&bytes).unwrap();
    assert_eq!(
        admit(
            &decoded,
            root.as_str(),
            None,
            &reports(&f),
            &f.declared.declaration()
        ),
        verdict
    );
}

#[test]
fn unreadable_input_is_refused_never_a_panic() {
    let f = fixture();
    let bytes = write(&f.sealed).unwrap();
    let design = json(&f.design);
    let (verification, integrity) = (json(&f.verification), json(&f.integrity));
    let declaration = f.declared.declaration();
    // Every truncation of the sealed container, and bytes that are not a container at all.
    for len in 0..bytes.len() {
        let verdict = precondition(&PreconditionInputs {
            parent: &bytes[..len],
            design: Some(&design),
            verification: &verification,
            integrity: &integrity,
            declaration: &declaration,
        });
        assert_eq!(kinds(&verdict), [R::Unreadable], "prefix {len}");
    }
    let verdict = precondition(&PreconditionInputs {
        parent: b"not a container",
        design: Some(b"[]"),
        verification: b"null",
        integrity: b"{}",
        declaration: &declaration,
    });
    assert_eq!(verdict.verdict, PreconditionVerdict::Refused);
    let details: Vec<&str> = verdict.reasons.iter().map(|(_, d)| d.as_str()).collect();
    assert_eq!(kinds(&verdict), [R::Unreadable; 4]);
    for (detail, name) in details.iter().zip([
        "design: ",
        "integrity: ",
        "parent: ATLAS_REJECTED",
        "verification: ",
    ]) {
        assert!(detail.starts_with(name), "{details:?}");
    }
    // G185: a report alone that does not decode refuses a parent every other check admits.
    for (at, name) in [(0, "verification: "), (1, "integrity: ")] {
        let mut inputs = [verification.clone(), integrity.clone()];
        inputs[at] = b"[1]".to_vec();
        let verdict = precondition(&PreconditionInputs {
            parent: &bytes,
            design: Some(&design),
            verification: &inputs[0],
            integrity: &inputs[1],
            declaration: &declaration,
        });
        assert_eq!(kinds(&verdict), [R::Unreadable], "{name}");
        assert!(verdict.reasons[0].1.starts_with(name), "{verdict:?}");
    }
}

#[test]
fn every_reason_is_collected_in_a_deterministic_order() {
    let f = fixture();
    let mut weakened = f.declared.envelope.clone();
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
            R::DesignNotSelected,
            R::IntegrityReportInconsistent,
            R::IntegrityReportInconsistent,
        ]
    );
    assert_eq!(verdict, run(&unsealed, &weakened, Some(&superseded)));
    // Against the sealed parent the forged design id also mismatches the one the seal names (the
    // pinned envelope still carries the named id, so only its identity fails; the integrity report
    // evaluates an invariant the weakened pin no longer carries).
    let verdict = run(&write(&f.sealed).unwrap(), &weakened, Some(&superseded));
    assert_eq!(
        kinds(&verdict),
        [
            R::EnvelopeUnverified,
            R::DesignUnverified,
            R::DesignMismatch,
            R::DesignNotSelected,
            R::IntegrityReportInconsistent,
            R::IntegrityReportInconsistent,
        ]
    );
}

/// The G179 reproduction (RES-G179-SEAL-SELF-CERTIFYING): an unsealed census container sealed by
/// a record nobody's gate decided -- arbitrary policy and report ids, the manifest's certificate,
/// census and revision copied, every blocker covered, `seal_id = seal_identity(record)` -- naming
/// the pinned envelope and a SELECTED design. The M9 writer and reader accept it, and every
/// check G179 made passes; the re-run gate refuses it.
#[test]
fn a_self_certified_seal_record_is_refused_seal_record_not_decided() {
    let f = fixture();
    let mut forged = f.unsealed.clone();
    let binding = seal_binding(&forged.manifest);
    let mut record = SealRecord {
        schema: crate::seal::gate::SEAL_RECORD_SCHEMA_VERSION.into(),
        seal_id: String::new(),
        scope: "fixture".into(),
        policy_id: "blake3-256:any-policy".into(),
        certificate_id: binding.certificate_id.clone(),
        census_digest: binding.census_digest.clone(),
        revision: binding.revision.clone(),
        verification_report: "blake3-256:any-report".into(),
        integrity_report: "report:any".into(),
        integrity_envelope: f.declared.envelope.envelope_id.clone(),
        design_id: f.design.design_id.clone(),
        permitted: forged
            .certificate
            .blockers
            .iter()
            .map(|b| format!("{b}: permitted"))
            .collect(),
    };
    record.seal_id = seal_identity(&record);
    forged.manifest.seal = SEALED.into();
    forged.seal = Some(record.clone());
    let bytes = write(&forged).expect("the M9 writer accepts a record that binds the container");
    let (read_back, _) = read(&bytes).expect("and it reads back SEALED");
    assert_eq!(read_back.manifest.seal, SEALED);
    let verdict = run(&bytes, &f.declared.envelope, Some(&f.design));
    assert_eq!(verdict.verdict, PreconditionVerdict::Refused);
    // Every G179 check passes: the gate re-run is the only reason, so G179 admitted it.
    assert_eq!(kinds(&verdict), [R::SealRecordNotDecided], "{verdict:?}");
    assert_eq!(
        verdict.reasons[0].1,
        format!(
            "the gate decides seal {}, not the container's {}",
            f.sealed.seal.as_ref().unwrap().seal_id,
            record.seal_id
        )
    );
    assert_eq!(verdict.admitted, None);

    // A forger who also forges the design, selected by a principal of its own choosing, meets
    // the gate's authority check: only a declared principal's signature selects.
    let mut own = f.design.clone();
    own.rationale = "self-selected".into();
    fixture::select(&mut own);
    let foreign_key = ed25519_compact::KeyPair::from_seed(ed25519_compact::Seed::new([12; 32]));
    let event = own.authority_event.as_mut().unwrap();
    event.signature = Some(crate::design::EventSignature {
        public_key: fixture::hex(&foreign_key.pk[..]),
        signature: fixture::hex(
            &foreign_key
                .sk
                .sign(crate::design::signing_message(event).as_bytes(), None)[..],
        ),
    });
    let bytes = {
        let mut atlas = forged.clone();
        let r = atlas.seal.as_mut().unwrap();
        r.design_id = own.design_id.clone();
        r.seal_id = seal_identity(r);
        write(&atlas).unwrap()
    };
    let verdict = run(&bytes, &f.declared.envelope, Some(&own));
    assert_eq!(kinds(&verdict), [R::SealGateNotEligible], "{verdict:?}");
    assert!(
        verdict.reasons[0]
            .1
            .starts_with("DESIGN_AUTHORITY_REFUSED: "),
        "{verdict:?}"
    );
}

/// G185: the record the container carries must be the one the gate decides over exactly the
/// inputs supplied. Each input changed alone refuses the parent: a gate input the gate refuses is
/// SEAL_GATE_NOT_ELIGIBLE with the gate's own reason; an input the gate accepts but the record
/// does not name is SEAL_RECORD_NOT_DECIDED.
#[test]
fn every_gate_input_must_decide_the_record_the_container_carries() {
    let f = fixture();
    let bytes = write(&f.sealed).unwrap();
    let gate_reason = |v: &Precondition| -> Vec<String> {
        let mut reasons: Vec<String> = v
            .reasons
            .iter()
            .map(|(r, d)| format!("{r} {}", d.split(':').next().unwrap_or_default()))
            .collect();
        reasons.dedup();
        reasons
    };
    let check = |g: Fixture, expected: &[&str]| {
        let verdict = run_with(&g, &bytes, &g.declared.envelope, Some(&g.design));
        assert_eq!(gate_reason(&verdict), expected, "{verdict:?}");
        assert_eq!(verdict.admitted, None);
    };
    // Another admissible policy: the gate decides, for another policy id.
    let mut g = fixture();
    g.declared.policy.hard_dimensions.push("TYPE".into());
    g.declared.policy.policy_id = crate::seal::policy_identity(&g.declared.policy);
    check(
        g,
        &["SEAL_RECORD_NOT_DECIDED the gate decides seal blake3-256"],
    );
    // A policy edited under its identity.
    let mut g = fixture();
    g.declared.policy.hard_dimensions.clear();
    check(g, &["SEAL_GATE_NOT_ELIGIBLE POLICY_INVALID"]);
    // A verification report that blocks, consistently, or that admits another candidate.
    let mut g = fixture();
    g.verification =
        fixture::verification_with(&g.verification.candidate, EvidenceResult::Violated);
    assert_eq!(
        crate::verification::check_report(&g.verification),
        Vec::<String>::new()
    );
    check(g, &["SEAL_GATE_NOT_ELIGIBLE VERIFICATION_BLOCKED"]);
    let mut g = fixture();
    g.verification.candidate = "blake3-256:other".into();
    check(
        g,
        &["SEAL_GATE_NOT_ELIGIBLE VERIFICATION_CANDIDATE_MISMATCH"],
    );
    // A consistent, admissible report the record does not name (its identity differs).
    let mut g = fixture();
    g.verification.outcomes[0]
        .evidence
        .push("evidence:another-run".into());
    check(
        g,
        &["SEAL_RECORD_NOT_DECIDED the gate decides seal blake3-256"],
    );
    // An integrity report that is consistently not ELIGIBLE, or is another report.
    let mut g = fixture();
    g.integrity.evaluations[0].status = EvaluationStatus::Fail;
    g.integrity.evaluations[0].diagnostic_codes = vec![ArchitectureDiagnostic::HardViolation];
    g.integrity.hard_violation_count = 1;
    g.integrity.verdict = IntegrityVerdict::Rejected;
    check(g, &["SEAL_GATE_NOT_ELIGIBLE INTEGRITY_NOT_ELIGIBLE"]);
    let mut g = fixture();
    g.integrity.report_id = "report:other".into();
    check(
        g,
        &["SEAL_RECORD_NOT_DECIDED the gate decides seal blake3-256"],
    );
    // A verdict edited against the report's own contents never reaches the gate.
    let mut g = fixture();
    g.verification.verdict = ReportVerdict::Blocked;
    check(
        g,
        &["VERIFICATION_REPORT_INCONSISTENT verdict BLOCKED does not follow from the outcomes"],
    );
    let mut g = fixture();
    g.integrity.verdict = IntegrityVerdict::Rejected;
    check(
        g,
        &["INTEGRITY_REPORT_INCONSISTENT verdict REJECTED does not follow from the evaluations"],
    );
    // A registry that does not declare the principal who selected the design.
    let mut g = fixture();
    g.declared.registry = PrincipalRegistry::empty();
    check(g, &["SEAL_GATE_NOT_ELIGIBLE DESIGN_AUTHORITY_REFUSED"]);
    // The authority check runs first, before the container is re-encoded: with it refused, the
    // gate's other reasons are not reached.
    let mut g = fixture();
    g.declared.registry = PrincipalRegistry::empty();
    g.verification.candidate = "blake3-256:other".into();
    check(g, &["SEAL_GATE_NOT_ELIGIBLE DESIGN_AUTHORITY_REFUSED"]);
    // A design with no authority event is refused the same way, before the re-encode.
    let mut g = fixture();
    g.design.authority_event = None;
    g.verification.candidate = "blake3-256:other".into();
    check(g, &["SEAL_GATE_NOT_ELIGIBLE DESIGN_WITHOUT_AUTHORITY"]);
}

/// G185 closes RES-G179-DESIGN-PARENT-ROOT-UNCOMPARED: the gate compares the design's parent root
/// with the container's root before the seal, recomputed from the sealed container. A design the
/// principal selected over the sealed root, sealed into a record that names it, is refused.
#[test]
fn a_design_over_another_root_is_refused_by_the_re_run_gate() {
    let f = fixture();
    let (_, sealed_root) = read(&write(&f.sealed).unwrap()).unwrap();
    let mut design = f.design.clone();
    design.parent_root = sealed_root.as_str().to_owned();
    fixture::select(&mut design);
    let bytes = restamped(&f.sealed, |r| r.design_id = design.design_id.clone());
    let verdict = run(&bytes, &f.declared.envelope, Some(&design));
    assert_eq!(kinds(&verdict), [R::SealGateNotEligible], "{verdict:?}");
    assert!(
        verdict.reasons[0]
            .1
            .starts_with("DESIGN_CANDIDATE_MISMATCH: ")
    );
    // The same design over the true unsealed root, sealed by the gate, is admitted.
    let (_, unsealed_root) = read(&write(&f.unsealed).unwrap()).unwrap();
    assert_eq!(f.design.parent_root, unsealed_root.as_str());
    let verdict = run(
        &write(&f.sealed).unwrap(),
        &f.declared.envelope,
        Some(&f.design),
    );
    assert_eq!(verdict.verdict, PreconditionVerdict::Admitted);
    // And a container sealed by the gate for a design with other roots is admitted with it.
    let other = fixture::design_over(
        &f.unsealed,
        vec![SemanticRoot {
            dimension: SemanticDimension::Symbol,
            record_id: fixture::MODE_SYMBOL.into(),
        }],
    );
    let sealed = fixture::seal(&f.unsealed, &other, &f).unwrap();
    let verdict = run(&write(&sealed).unwrap(), &f.declared.envelope, Some(&other));
    assert_eq!(
        verdict.verdict,
        PreconditionVerdict::Admitted,
        "{verdict:?}"
    );
}

/// G185 review P1: a principal holding a SELECTED design can author the reports themselves. The
/// seal gate refuses reports that contradict their own contents, and so does the precondition,
/// for a record forged to name them anyway.
#[test]
fn a_seal_over_self_contradicting_reports_is_refused() {
    let dirty_integrity = |edit: &dyn Fn(&mut IntegrityReport)| {
        let mut g = fixture();
        edit(&mut g.integrity);
        g
    };
    for (g, detail) in [
        // A HARD violation, counted as none, reported ELIGIBLE.
        (
            dirty_integrity(&|r| {
                r.evaluations[0].status = EvaluationStatus::Fail;
                r.evaluations[0].diagnostic_codes = vec![ArchitectureDiagnostic::HardViolation];
            }),
            &[
                "hard_violation_count 0 but 1 evaluations carry a hard violation",
                "verdict ELIGIBLE does not follow from the evaluations",
            ][..],
        ),
        // The same failure with its diagnostic dropped, so the counts agree.
        (
            dirty_integrity(&|r| r.evaluations[0].status = EvaluationStatus::Fail),
            &[
                "invariant:fixture:core-does-not-call-runtime is FAIL for a HARD invariant without \
               HARD_VIOLATION",
            ][..],
        ),
        // A HARD invariant left undecided, its REQUIRED_UNKNOWN dropped.
        (
            dirty_integrity(&|r| r.evaluations[0].status = EvaluationStatus::Unknown),
            &[
                "invariant:fixture:core-does-not-call-runtime is UNKNOWN for a HARD invariant \
               without REQUIRED_UNKNOWN",
            ][..],
        ),
        // A HARD invariant decided CONFLICT or NOT_AFFECTED, which `evaluate` never writes.
        (
            dirty_integrity(&|r| r.evaluations[0].status = EvaluationStatus::Conflict),
            &[
                "invariant:fixture:core-does-not-call-runtime is CONFLICT for a HARD invariant: \
               only PASS, FAIL or UNKNOWN is decided",
            ][..],
        ),
        (
            dirty_integrity(&|r| r.evaluations[0].status = EvaluationStatus::NotAffected),
            &[
                "invariant:fixture:core-does-not-call-runtime is NOT_AFFECTED for a HARD \
               invariant: only PASS, FAIL or UNKNOWN is decided",
            ][..],
        ),
    ] {
        let sealed = sealed_over(&g, "INTEGRITY_REPORT_INCONSISTENT");
        let verdict = run_with(&g, &sealed, &g.declared.envelope, Some(&g.design));
        let expected: Vec<_> = detail
            .iter()
            .map(|d| (R::IntegrityReportInconsistent, (*d).to_owned()))
            .collect();
        assert_eq!(verdict.reasons, expected);
        assert_eq!(verdict.admitted, None);
    }
    // An ADMISSIBLE verification report that names its own blockers.
    let mut g = fixture();
    g.verification
        .blockers
        .push("UNIT: 12 required unit obligations fail".into());
    let sealed = sealed_over(&g, "VERIFICATION_REPORT_INCONSISTENT");
    let verdict = run_with(&g, &sealed, &g.declared.envelope, Some(&g.design));
    assert_eq!(kinds(&verdict), [R::VerificationReportInconsistent]);
    assert!(
        verdict.reasons[0].1.starts_with("blockers [\"UNIT: 12"),
        "{verdict:?}"
    );
    // Every required class covered only by an obligation that is not required, each FAILED,
    // reported ADMISSIBLE: a required class needs a required obligation.
    let mut g = fixture();
    g.verification.failures.clear();
    for outcome in &mut g.verification.outcomes {
        outcome.required = false;
        outcome.state = crate::verification::ObligationState::Failed;
        g.verification
            .failures
            .push(crate::verification::VerificationFailure {
                obligation: outcome.obligation.clone(),
                counterexample: None,
                evidence: outcome.evidence.clone(),
            });
    }
    for tally in g.verification.coverage.values_mut() {
        (tally.satisfied, tally.failed) = (0, 1);
    }
    let sealed = sealed_over(&g, "VERIFICATION_REPORT_INCONSISTENT");
    let verdict = run_with(&g, &sealed, &g.declared.envelope, Some(&g.design));
    assert_eq!(
        kinds(&verdict),
        [R::VerificationReportInconsistent; 2],
        "{verdict:?}"
    );
    assert!(
        verdict.reasons[0]
            .1
            .contains("required class SEMANTIC has no obligation")
    );
    // Coverage that does not count the outcomes.
    let mut g = fixture();
    for tally in g.verification.coverage.values_mut() {
        tally.satisfied += 1;
    }
    let verdict = run_with(
        &g,
        &write(&g.sealed).unwrap(),
        &g.declared.envelope,
        Some(&g.design),
    );
    assert_eq!(
        verdict.reasons,
        [(
            R::VerificationReportInconsistent,
            "coverage does not follow from the outcomes".to_owned()
        )]
    );
    // A failure naming an obligation that did not fail, and a SATISFIED one resting on nothing.
    let mut g = fixture();
    g.verification
        .failures
        .push(crate::verification::VerificationFailure {
            obligation: "fixture:UNIT".into(),
            counterexample: None,
            evidence: Vec::new(),
        });
    let verdict = run_with(
        &g,
        &write(&g.sealed).unwrap(),
        &g.declared.envelope,
        Some(&g.design),
    );
    assert_eq!(
        verdict.reasons,
        [(
            R::VerificationReportInconsistent,
            "failures name [\"fixture:UNIT\"], the FAILED outcomes are []".to_owned()
        )]
    );
    let mut g = fixture();
    g.verification.outcomes[0].evidence.clear();
    let verdict = run_with(
        &g,
        &write(&g.sealed).unwrap(),
        &g.declared.envelope,
        Some(&g.design),
    );
    assert_eq!(kinds(&verdict), [R::VerificationReportInconsistent]);
    assert!(
        verdict.reasons[0]
            .1
            .ends_with("is SATISFIED without admissible evidence")
    );
}

/// The fixture container sealed by a record naming the reports of `g`, which the seal gate
/// itself refuses with `reason`: re-stamped as a forger would, so only the precondition decides.
fn sealed_over(g: &Fixture, reason: &str) -> Vec<u8> {
    let refused = fixture::seal(&g.unsealed, &g.design, g).expect_err("the gate refuses");
    assert!(refused.iter().any(|r| r.starts_with(reason)), "{refused:?}");
    restamped(&g.sealed, |r| {
        r.verification_report = crate::seal::verification_identity(&g.verification);
        r.integrity_report = g.integrity.report_id.clone();
        r.integrity_envelope = g.integrity.envelope_ref.clone();
    })
}

/// G185 review P1: the envelope is the repository's pinned one. A seal whose integrity report
/// cites an envelope its author wrote -- consistent with it, ELIGIBLE, and sealed by the gate --
/// is refused against the pin.
#[test]
fn a_seal_over_a_self_authored_envelope_is_refused() {
    let f = fixture();
    let mut g = fixture();
    let mut own = g.declared.envelope.clone();
    own.invariants[0].strength = Strength::Soft;
    own.envelope_id = envelope_identity(&own);
    g.integrity = fixture::integrity(&own, &g.verification.candidate);
    g.declared.envelope = own;
    let sealed = fixture::seal(&g.unsealed, &g.design, &g).expect("the gate does not see the pin");
    // Under the pin it was authored against, it is admitted: the pin is the repository's choice.
    let bytes = write(&sealed).unwrap();
    assert_eq!(
        run_with(&g, &bytes, &g.declared.envelope, Some(&g.design)).verdict,
        PreconditionVerdict::Admitted
    );
    // Under the repository's pin, with the author's own reports, refused.
    let verdict = precondition(&PreconditionInputs {
        parent: &bytes,
        design: Some(&json(&g.design)),
        verification: &json(&g.verification),
        integrity: &json(&g.integrity),
        declaration: &f.declared.declaration(),
    });
    assert_eq!(
        kinds(&verdict),
        [R::EnvelopeMismatch, R::IntegrityReportInconsistent],
        "{verdict:?}"
    );
}
