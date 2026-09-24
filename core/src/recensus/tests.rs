use super::*;

fn artifact(path: &str, content: &str, semantic: &str, unknowns: usize) -> ArtifactState {
    let mut facts = BTreeMap::new();
    facts.insert("Symbol|OBSERVED".to_owned(), 3);
    if unknowns > 0 {
        facts.insert("SemanticObligation|UNKNOWN".to_owned(), unknowns);
    }
    ArtifactState {
        path: path.into(),
        disposition: "Parsed".into(),
        language: Some("rust".into()),
        content_digest: Some(content.into()),
        facts,
        records: BTreeMap::new(),
        obligations: BTreeMap::new(),
        unknowns,
        semantic_digest: semantic.into(),
    }
}

fn snapshot(revision: &str, artifacts: Vec<ArtifactState>) -> CensusSnapshot {
    let mut coverage = BTreeMap::new();
    coverage.insert("SYMBOL".to_owned(), "OBSERVED".to_owned());
    coverage.insert("CALL".to_owned(), "UNKNOWN".to_owned());
    let mut totals = BTreeMap::new();
    totals.insert("artifacts".to_owned(), artifacts.len());
    totals.insert("graph_nodes".to_owned(), 10 * artifacts.len());
    let mut s = CensusSnapshot {
        schema: SNAPSHOT_SCHEMA.into(),
        revision: revision.into(),
        dirty: false,
        census_digest: String::new(),
        artifacts,
        coverage,
        totals,
        dependency: DependencyState {
            state: "Closed".into(),
            edges: vec!["runtime -> core@0.1.0 (WorkspaceMember) [Some(Runtime)]".into()],
            ..DependencyState::default()
        },
        adl: AdlState::default(),
        admission_allowed: true,
        admission_blockers: vec![],
        docs_gate_ready: true,
        typed_semantics_closed: true,
    };
    s.census_digest = s.compute_digest().as_str().to_owned();
    s
}

fn reseal(mut s: CensusSnapshot) -> CensusSnapshot {
    s.census_digest = s.compute_digest().as_str().to_owned();
    s
}

fn base() -> CensusSnapshot {
    snapshot(
        "aaa",
        vec![
            artifact("core/src/a.rs", "d1", "s1", 0),
            artifact("core/src/b.rs", "d2", "s2", 1),
        ],
    )
}

fn intent(objective: &str, paths: &[&str], totals: &[&str]) -> RecensusIntent {
    RecensusIntent {
        objective: objective.into(),
        changed_paths: paths.iter().map(|p| (*p).into()).collect(),
        total_changes: totals.iter().map(|t| (*t).into()).collect(),
        ..RecensusIntent::default()
    }
}

#[test]
fn the_digest_is_revision_independent_and_content_sensitive() {
    let a = base();
    let b = reseal(CensusSnapshot {
        revision: "bbb".into(),
        dirty: true,
        ..base()
    });
    assert_eq!(
        a.census_digest, b.census_digest,
        "same semantics at a new revision"
    );
    let mut c = base();
    c.artifacts[0].semantic_digest = "s1-changed".into();
    assert_ne!(reseal(c).census_digest, a.census_digest);
    let mut tampered = base();
    tampered.totals.insert("graph_nodes".into(), 999);
    assert!(!tampered.verify_digest());
}

#[test]
fn an_intended_change_with_no_regression_is_proven() {
    let before = base();
    let mut after = base();
    after
        .artifacts
        .push(artifact("core/src/c.rs", "d3", "s3", 0));
    after.totals.insert("artifacts".into(), 3);
    after.totals.insert("graph_nodes".into(), 30);
    let after = reseal(after);
    let report = prove(
        "G1",
        &before,
        &after,
        &after.clone(),
        &intent(
            "add c.rs",
            &["core/src/c.rs"],
            &["+artifacts", "+graph_nodes"],
        ),
    );
    assert_eq!(report.verdict, Verdict::Proven, "{report:#?}");
    assert_eq!(report.inventory_delta.created, ["core/src/c.rs"]);
    assert_eq!(report.graph_delta.get("graph_nodes"), Some(&10));
}

#[test]
fn an_unintended_change_is_not_proven_unless_accepted_with_a_reason() {
    let before = base();
    let mut after = base();
    after.artifacts[1].content_digest = Some("d2-edited".into());
    let after = reseal(after);
    let report = prove(
        "G1",
        &before,
        &after,
        &after,
        &intent("touch nothing", &[], &[]),
    );
    assert_eq!(report.verdict, Verdict::GenerationNotProven);
    assert_eq!(report.unexpected_changes, ["path core/src/b.rs"]);
    let mut accepted = intent("touch nothing", &[], &[]);
    accepted.accepted_unexpected = vec!["core/src/b.rs: formatting only".into()];
    assert_eq!(
        prove("G1", &before, &after, &after, &accepted).verdict,
        Verdict::Proven
    );
    accepted.accepted_unexpected = vec!["core/src/b.rs: ".into()];
    assert_eq!(
        prove("G1", &before, &after, &after, &accepted).verdict,
        Verdict::GenerationNotProven,
        "acceptance needs a reason"
    );
}

#[test]
fn an_intended_change_that_did_not_happen_is_not_proven() {
    let before = base();
    let report = prove(
        "G1",
        &before,
        &before,
        &before,
        &intent("add c.rs", &["core/src/c.rs"], &["+artifacts"]),
    );
    assert_eq!(report.verdict, Verdict::GenerationNotProven);
    assert_eq!(
        report.intended_but_unobserved,
        ["path core/src/c.rs", "total +artifacts"]
    );
}

#[test]
fn forbidden_regressions_block_even_intended_changes() {
    let before = base();
    type Mutation = Box<dyn Fn(&mut CensusSnapshot)>;
    let cases: Vec<(&str, Mutation)> = vec![
        (
            "coverage regression SYMBOL",
            Box::new(|s| {
                s.coverage.insert("SYMBOL".into(), "UNKNOWN".into());
            }),
        ),
        (
            "dependency closure left CLOSED",
            Box::new(|s| s.dependency.state = "Partial".into()),
        ),
        (
            "new dangling dependency reference",
            Box::new(|s| s.dependency.dangling.push("ghost".into())),
        ),
        (
            "typed semantic closure broken",
            Box::new(|s| s.typed_semantics_closed = false),
        ),
        (
            "coding admission blocked",
            Box::new(|s| s.admission_allowed = false),
        ),
        (
            "docs gate no longer ready",
            Box::new(|s| s.docs_gate_ready = false),
        ),
        (
            "unexplained new unknowns",
            Box::new(|s| {
                s.artifacts[0].unknowns = 2;
            }),
        ),
    ];
    for (expected, mutate) in cases {
        let mut after = base();
        mutate(&mut after);
        let after = reseal(after);
        let mut i = intent("anything", &["core/"], &[]);
        i.accepted_unexpected = vec![
            "coverage SYMBOL: OBSERVED -> UNKNOWN: test".into(),
            "dependency: test".into(),
        ];
        let report = prove("G1", &before, &after, &after, &i);
        assert_eq!(report.verdict, Verdict::GenerationNotProven, "{expected}");
        assert!(
            report.regressions.iter().any(|r| r.starts_with(expected)),
            "{expected}: {:?}",
            report.regressions
        );
    }
}

#[test]
fn new_unknowns_pass_only_when_explained() {
    let before = base();
    let mut after = base();
    after.artifacts[0].unknowns = 4;
    after.artifacts[0].semantic_digest = "s1b".into();
    let after = reseal(after);
    let mut i = intent("extend extraction", &["core/src/a.rs"], &[]);
    assert_eq!(
        prove("G1", &before, &after, &after, &i).verdict,
        Verdict::GenerationNotProven
    );
    i.explained_unknowns =
        vec!["core/src/a.rs: new CALL obligations are reported as UNKNOWN until resolved".into()];
    assert_eq!(
        prove("G1", &before, &after, &after, &i).verdict,
        Verdict::Proven
    );
}

#[test]
fn replay_divergence_and_tampering_are_regressions() {
    let before = base();
    let mut other = base();
    other.artifacts[0].semantic_digest = "nondeterministic".into();
    let other = reseal(other);
    let report = prove("G1", &before, &before, &other, &intent("x", &[], &[]));
    assert!(
        report
            .regressions
            .iter()
            .any(|r| r.starts_with("deterministic replay failed"))
    );
    let mut forged = base();
    forged.census_digest = "blake3-256:00".into();
    let report = prove("G1", &forged, &before, &before, &intent("x", &[], &[]));
    assert!(
        report
            .regressions
            .iter()
            .any(|r| r.starts_with("before snapshot digest"))
    );
}

#[test]
fn a_generation_must_declare_its_objective() {
    let before = base();
    let report = prove("G1", &before, &before, &before, &RecensusIntent::default());
    assert_eq!(report.verdict, Verdict::GenerationNotProven);
}

#[test]
fn intended_coverage_improvement_is_observed_and_downgrades_need_intent() {
    let before = base();
    let mut after = base();
    after.coverage.insert("CALL".into(), "OBSERVED".into());
    let after = reseal(after);
    let mut i = intent("resolve calls", &[], &[]);
    assert_eq!(
        prove("G1", &before, &after, &after, &i).verdict,
        Verdict::GenerationNotProven
    );
    i.coverage_changes = vec!["CALL=OBSERVED".into()];
    assert_eq!(
        prove("G1", &before, &after, &after, &i).verdict,
        Verdict::Proven
    );
}
