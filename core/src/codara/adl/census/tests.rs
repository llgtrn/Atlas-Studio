use super::*;
use crate::{
    AdlSource, DependencyActivation, DependencyClosureState, DependencyEcosystem, DependencyEdge,
    DependencyIdentity, FileFact, compile_adl,
};

fn member_edge(consumer: &str, provider: &str, role: DependencyRole) -> DependencyEdge {
    DependencyEdge {
        consumer: consumer.into(),
        provider: DependencyIdentity {
            ecosystem: DependencyEcosystem::Cargo,
            name: provider.into(),
            version: "0.1.0".into(),
            source_kind: DependencySourceKind::WorkspaceMember,
            source_locator: None,
            checksum: None,
        },
        role: Some(role),
        activation: DependencyActivation::ALWAYS,
        evidence_path: format!("{}/Cargo.toml", dir_of(consumer)),
    }
}

fn registry_edge(consumer: &str, provider: &str) -> DependencyEdge {
    let mut edge = member_edge(consumer, provider, DependencyRole::Runtime);
    edge.provider.source_kind = DependencySourceKind::Registry;
    edge
}

fn dir_of(package: &str) -> &str {
    match package {
        "atlas-cli" => "apps/cli",
        other => other,
    }
}

/// The real shape of this repository's workspace: core <- adapter <- runtime <- atlas-cli, plus a
/// dev-only edge and a member (`ghost`) seen only as a provider.
fn closure() -> DependencyClosureReport {
    let edges = vec![
        member_edge("adapter", "core", DependencyRole::Runtime),
        registry_edge("core", "serde"),
        member_edge("runtime", "core", DependencyRole::Runtime),
        member_edge("runtime", "adapter", DependencyRole::Runtime),
        member_edge("atlas-cli", "runtime", DependencyRole::Runtime),
        member_edge("atlas-cli", "core", DependencyRole::Dev),
        member_edge("runtime", "ghost", DependencyRole::Build),
    ];
    DependencyClosureReport {
        schema: "test".into(),
        ecosystem: DependencyEcosystem::Cargo,
        root: "/repo".into(),
        state: DependencyClosureState::Closed,
        edges_total: edges.len(),
        instances_total: 0,
        edges,
        dangling_references: Vec::new(),
        unsupported_constructs: Vec::new(),
        dynamic_obligations: Vec::new(),
        reachability: None,
    }
}

fn source() -> SourceReport {
    let files: Vec<FileFact> = [
        "core/src/lib.rs",
        "adapter/src/lib.rs",
        "runtime/src/lib.rs",
    ]
    .iter()
    .chain(["apps/cli/src/main.rs"].iter())
    .map(|path| FileFact {
        path: (*path).into(),
        language: "rust".into(),
        bytes: 1,
    })
    .collect();
    SourceReport {
        schema: "test".into(),
        root: "/repo".into(),
        files_total: files.len(),
        languages: BTreeMap::from([("rust".into(), files.len())]),
        files,
    }
}

const AUTHORED: &str = "atlas 1\nsystem Test\n\
entity Runtime Core {\n    kind = backend\n}\n\
entity Runtime Adapter {\n    kind = backend\n}\n\
entity Runtime Runtime {\n    kind = backend\n}\n\
entity Runtime WebUI {\n    kind = frontend\n}\n\
Runtime ->depends_on-> Core\n\
Runtime ->depends_on-> Adapter\n\
Core ->depends_on-> Adapter\n\
WebUI ->depends_on-> Runtime\n\
Core ->contains-> Runtime\n\
materialize Core {\n    path = \"core\"\n}\n\
materialize Adapter {\n    path = \"adapter/\"\n}\n\
materialize Runtime {\n    path = \"runtime\"\n}\n\
materialize WebUI {\n    path = \"apps/studio\"\n}\n";

fn declared(sources: &[(&str, &str)]) -> DeclaredGraph {
    let sources: Vec<AdlSource> = sources
        .iter()
        .map(|(path, text)| AdlSource {
            path: (*path).into(),
            text: (*text).into(),
        })
        .collect();
    let report = compile_adl(&sources, &source());
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    report.ir.declared
}

#[test]
fn the_observed_architecture_is_member_level_and_excludes_dev_edges() {
    let observed = ObservedArchitecture::from_closure(&closure());
    let packages: Vec<(&str, &str)> = observed
        .members
        .iter()
        .map(|m| (m.package.as_str(), m.dir.as_str()))
        .collect();
    assert_eq!(
        packages,
        [
            ("adapter", "adapter"),
            ("atlas-cli", "apps/cli"),
            ("core", "core"),
            ("runtime", "runtime")
        ]
    );
    let pairs: Vec<(&str, &str)> = observed
        .dependencies
        .iter()
        .map(|d| (d.consumer.as_str(), d.provider.as_str()))
        .collect();
    assert_eq!(
        pairs,
        [
            ("adapter", "core"),
            ("atlas-cli", "runtime"),
            ("runtime", "adapter"),
            ("runtime", "core"),
            ("runtime", "ghost"),
        ],
        "dev-only atlas-cli -> core is not architecture; registry serde is not a member"
    );
    assert_eq!(observed.unplaced_members, ["ghost"]);
}

#[test]
fn authored_dependencies_are_reconciled_by_materialization_path_never_by_name() {
    let observed = ObservedArchitecture::from_closure(&closure());
    let r = reconcile_dependencies(&declared(&[("system.adl", AUTHORED)]), &observed);
    let pair = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    assert_eq!(
        r.agreed,
        [pair("Runtime", "Adapter"), pair("Runtime", "Core")]
    );
    assert_eq!(r.declared_not_observed, [pair("Core", "Adapter")]);
    assert_eq!(r.not_censusable, [pair("WebUI", "Runtime")]);
    let undeclared: Vec<&str> = r
        .observed_not_declared
        .iter()
        .map(|d| d.consumer.as_str())
        .collect();
    assert_eq!(
        undeclared,
        ["adapter", "atlas-cli", "runtime"],
        "adapter -> core, atlas-cli -> runtime and runtime -> ghost are observed, not declared"
    );
    let members: Vec<&str> = r
        .undeclared_members
        .iter()
        .map(|m| m.package.as_str())
        .collect();
    assert_eq!(members, ["atlas-cli"]);

    // Name equality is not a correspondence: an entity called `core` materialized elsewhere
    // relates to nothing the census observed.
    let renamed = AUTHORED.replace("path = \"core\"", "path = \"elsewhere\"");
    let r = reconcile_dependencies(&declared(&[("system.adl", &renamed)]), &observed);
    assert!(r.not_censusable.contains(&pair("Runtime", "Core")));
    assert!(r.undeclared_members.iter().any(|m| m.package == "core"));
}

#[test]
fn reconciliation_results_are_typed_and_disagreement_is_violated() {
    let observed = ObservedArchitecture::from_closure(&closure());
    let r = reconcile_dependencies(&declared(&[("system.adl", AUTHORED)]), &observed);
    let results = r.constraint_results();
    let verdict = |name: &str| {
        results.iter().find(|c| c.name == name).map(|c| {
            (
                c.verdict,
                c.passed,
                c.diagnostics.first().map(|d| d.code.clone()),
            )
        })
    };
    assert_eq!(
        verdict("DeclaredDependency:Runtime->Core"),
        Some((ConstraintVerdict::Satisfied, true, None))
    );
    assert_eq!(
        verdict("DeclaredDependency:Core->Adapter"),
        Some((
            ConstraintVerdict::Violated,
            false,
            Some("ATLAS-E060".into())
        ))
    );
    assert_eq!(
        verdict("ObservedDependency:adapter->core"),
        Some((
            ConstraintVerdict::Violated,
            false,
            Some("ATLAS-E061".into())
        ))
    );
    assert_eq!(
        verdict("ObservedMember:atlas-cli"),
        Some((
            ConstraintVerdict::Violated,
            false,
            Some("ATLAS-E062".into())
        ))
    );
    assert!(
        verdict("DeclaredDependency:WebUI->Runtime").is_none(),
        "a relation no census can check is neither passed nor failed"
    );
    assert!(
        results
            .iter()
            .all(|c| c.derivation[0].rule == ConstraintCheckKind::ObservedDependency)
    );
    let deltas = r.deltas();
    assert_eq!(deltas.len(), 1);
    assert_eq!(deltas[0].code, "DECLARED_DEPENDENCY_NOT_CENSUSABLE");
    assert_eq!(deltas[0].subject, "WebUI->Runtime");
}

#[test]
fn derived_census_adl_closes_the_gap_and_is_a_fixed_point() {
    let observed = ObservedArchitecture::from_closure(&closure());
    let authored = AUTHORED.replace("Core ->depends_on-> Adapter\n", "");
    let derived = derive_census_adl(
        &declared(&[("system.adl", &authored)]),
        &observed,
        &source(),
    );
    assert!(derived.starts_with("atlas 1\n"), "{derived}");
    assert!(derived.contains(CENSUS_DERIVATION_ID));
    assert!(derived.contains("entity Package AtlasCli {"), "{derived}");
    assert!(derived.contains("package = \"atlas-cli\""));
    assert!(derived.contains("path = \"apps/cli\"\n    language = rust\n"));
    assert!(derived.contains("require materialized AtlasCli"));
    assert!(derived.contains(
        "# census: `adapter` -> `core` (RUNTIME, adapter/Cargo.toml)\nAdapter ->depends_on-> Core\n"
    ));
    assert!(derived.contains("AtlasCli ->depends_on-> Runtime"));
    assert!(
        !derived.contains("->depends_on-> Ghost"),
        "an unplaced member cannot be declared"
    );
    assert!(derived.contains("`ghost` is only seen as a provider"));

    // Authored + derived: every observed member and every placed dependency is declared, and the
    // derived text itself compiles without diagnostics.
    let union = declared(&[("census.adl", &derived), ("system.adl", &authored)]);
    let r = reconcile_dependencies(&union, &observed);
    assert!(r.undeclared_members.is_empty(), "{r:#?}");
    assert!(r.declared_not_observed.is_empty(), "{r:#?}");
    let unresolved: Vec<&str> = r
        .observed_not_declared
        .iter()
        .map(|d| d.provider.as_str())
        .collect();
    assert_eq!(unresolved, ["ghost"], "only the unplaced member remains");

    // Deterministic, and derived from the authored ADL alone.
    assert_eq!(
        derived,
        derive_census_adl(
            &declared(&[("system.adl", &authored)]),
            &observed,
            &source()
        )
    );
    // Deriving over the union adds nothing new: no entity, no relation.
    let again = derive_census_adl(&union, &observed, &source());
    assert!(!again.contains("entity "), "{again}");
    assert!(!again.contains("->depends_on->"), "{again}");
}

#[test]
fn entity_names_are_valid_handles_and_never_collide() {
    let taken = BTreeSet::from(["AtlasCli".to_owned()]);
    assert_eq!(entity_name("atlas-cli", &taken), "AtlasCliPackage");
    assert_eq!(entity_name("serde_json", &BTreeSet::new()), "SerdeJson");
    assert_eq!(entity_name("--", &BTreeSet::new()), "Member");
}
