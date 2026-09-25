use super::*;
use crate::language::adl::{AdlSource, compile_adl};
use crate::{SourceReport, constraint::ConstraintVerdict};

const ADL: &str = "atlas 1\nsystem Example\n\
entity Runtime Core {\n    kind = backend\n}\n\
entity Runtime App {\n    kind = backend\n}\n\
entity Runtime Ui {\n    kind = frontend\n}\n\
materialize Core {\n    path = \"core\"\n    language = rust\n}\n\
materialize App {\n    path = \"app\"\n    language = rust\n}\n\
materialize Ui {\n    path = \"ui\"\n}\n\
App ->depends_on-> Core\n\
Ui ->depends_on-> App\n\
invariant CoreCallsNoApp {\n    forall f: function in Core forbid call to App\n}\n\
invariant CoreSpawnsNothing {\n    forall f: function in Core forbid effect PROCESS_SPAWN\n}\n\
invariant CoreEnvelope {\n    forall f: function in Core observed effect within FILESYSTEM_WRITE\n}\n\
constraint CoreIsMaterialized {\n    require materialized Core\n}\n";

fn declared(text: &str) -> DeclaredGraph {
    let source = AdlSource {
        path: ".atlas/declared/system.adl".into(),
        text: text.into(),
    };
    let observed = SourceReport {
        schema: "test".into(),
        root: "/repo".into(),
        files_total: 0,
        languages: BTreeMap::new(),
        files: Vec::new(),
    };
    compile_adl(&[source], &observed).ir.declared
}

fn result(name: &str, verdict: ConstraintVerdict) -> ConstraintResult {
    ConstraintResult {
        name: name.into(),
        passed: verdict.admits(),
        verdict,
        diagnostics: Vec::new(),
        derivation: Vec::new(),
    }
}

fn all_satisfied() -> Vec<ConstraintResult> {
    [
        "CoreCallsNoApp",
        "CoreSpawnsNothing",
        "CoreEnvelope",
        "DeclaredDependency:App->Core",
        "CoreIsMaterialized",
    ]
    .into_iter()
    .map(|n| result(n, ConstraintVerdict::Satisfied))
    .collect()
}

#[test]
fn the_envelope_lowers_every_census_falsifiable_declaration() {
    let envelope = derive_envelope("Example", &declared(ADL));
    let kinds: Vec<(&str, InvariantClass, Strength)> = envelope
        .invariants
        .iter()
        .map(|i| (i.invariant_id.as_str(), i.kind, i.strength))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (
                "AIE:CoreCallsNoApp",
                InvariantClass::DependencyDirection,
                Strength::Hard
            ),
            (
                "AIE:CoreEnvelope",
                InvariantClass::AuthorityBoundary,
                Strength::Hard
            ),
            (
                "AIE:CoreSpawnsNothing",
                InvariantClass::AuthorityBoundary,
                Strength::Hard
            ),
            (
                "AIE:DeclaredDependency:App->Core",
                InvariantClass::DependencyDirection,
                Strength::Hard
            ),
            // The census cannot observe a dependency of a non-Rust entity: SOFT, and noted.
            (
                "AIE:DeclaredDependency:Ui->App",
                InvariantClass::DependencyDirection,
                Strength::Soft
            ),
        ]
    );
    assert_eq!(envelope.notes.len(), 1);
    // A materialization constraint is not architecture: outside the envelope.
    assert!(
        !envelope
            .invariants
            .iter()
            .any(|i| i.invariant_id.contains("Materialized"))
    );
    for i in &envelope.invariants {
        assert!(!i.falsification_conditions.is_empty() && !i.evidence_refs.is_empty());
    }
    let classes: Vec<(&str, Classification)> = envelope
        .elements
        .iter()
        .map(|e| (e.element_ref.as_str(), e.classification))
        .collect();
    assert_eq!(
        classes,
        vec![
            ("App", Classification::LoadBearing),
            ("Core", Classification::LoadBearing),
            ("Ui", Classification::Structural),
        ]
    );
    assert_eq!(envelope.genome_ref, GENOME_UNPINNED);
    // The identity pins the invariants: any change changes it.
    let weaker = derive_envelope(
        "Example",
        &declared(&ADL.replace(
            "within FILESYSTEM_WRITE",
            "within FILESYSTEM_WRITE, NETWORK",
        )),
    );
    assert_ne!(weaker.envelope_id, envelope.envelope_id);
    assert_eq!(envelope.envelope_id, envelope_identity(&envelope));
}

#[test]
fn the_report_rejects_hard_violations_and_silent_weakening_and_is_consistent() {
    let pinned = derive_envelope("Example", &declared(ADL));
    let clean = evaluate(&pinned, &pinned, &all_satisfied(), "candidate", "digest");
    // Ui -> App has no decided result: a SOFT unknown never blocks.
    assert_eq!(clean.verdict, IntegrityVerdict::Eligible, "{clean:#?}");
    assert_eq!(
        (clean.hard_violation_count, clean.required_unknown_count),
        (0, 0)
    );
    assert!(check_report(&clean, &pinned).is_empty());
    let soft = clean
        .evaluations
        .iter()
        .find(|e| e.invariant_ref == "AIE:DeclaredDependency:Ui->App")
        .unwrap();
    assert_eq!(soft.status, EvaluationStatus::Unknown);
    assert!(soft.diagnostic_codes.is_empty());

    // A seeded HARD violation fails the gate.
    let mut results = all_satisfied();
    results[0] = result("CoreCallsNoApp", ConstraintVerdict::Violated);
    let violated = evaluate(&pinned, &pinned, &results, "candidate", "digest");
    assert_eq!(violated.verdict, IntegrityVerdict::Rejected);
    assert_eq!(violated.hard_violation_count, 1);
    assert!(check_report(&violated, &pinned).is_empty());

    // A HARD invariant the census cannot decide keeps the candidate INCOMPLETE, never ELIGIBLE.
    let mut results = all_satisfied();
    results[1] = result("CoreSpawnsNothing", ConstraintVerdict::Unknown);
    let unknown = evaluate(&pinned, &pinned, &results, "candidate", "digest");
    assert_eq!(unknown.verdict, IntegrityVerdict::Incomplete);
    assert_eq!(unknown.required_unknown_count, 1);

    // Silent weakening: the candidate drops a pinned invariant (and so its result).
    let weakened = derive_envelope(
        "Example",
        &declared(&ADL.replace(
            "invariant CoreCallsNoApp {\n    forall f: function in Core forbid call to App\n}\n",
            "",
        )),
    );
    let results: Vec<ConstraintResult> = all_satisfied().into_iter().skip(1).collect();
    let silent = evaluate(&pinned, &weakened, &results, "candidate", "digest");
    assert_eq!(silent.verdict, IntegrityVerdict::Rejected);
    let dropped = silent
        .evaluations
        .iter()
        .find(|e| e.invariant_ref == "AIE:CoreCallsNoApp")
        .unwrap();
    assert_eq!(
        dropped.diagnostic_codes,
        vec![ArchitectureDiagnostic::UnselectedRevision]
    );

    // The consistency check catches a tampered summary.
    let mut tampered = violated.clone();
    tampered.hard_violation_count = 0;
    tampered.verdict = IntegrityVerdict::Eligible;
    assert_eq!(check_report(&tampered, &pinned).len(), 2);
    // A pin edited by hand (one statement reworded) no longer matches its own identity.
    let mut edited = pinned.clone();
    edited.invariants[0].statement.push_str(" (reworded)");
    assert_eq!(
        check_report(&clean, &edited),
        vec!["the report does not cite the pinned envelope's identity".to_owned()]
    );
}
