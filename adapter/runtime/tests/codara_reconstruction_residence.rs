//! Both supported routes consume the same frozen source observations, never source as IR input.
use adapter::semantic::{ExtractionInput, SemanticExtractor};
use atlas_core::codara::construction::{CheckResult, module_identity};
use atlas_core::{ArtifactId, RepositoryId, RevisionRef, SemanticDimension};
use runtime::{codara::reconstruction as codara, self_reconstruction as historical};

#[test]
fn public_routes_preserve_full_lift_lineage_identity_and_refusals() {
    assert_eq!(
        std::any::type_name::<codara::Candidate>(),
        "runtime::self_reconstruction::Candidate"
    );
    assert_eq!(
        std::any::type_name::<codara::Attempt<'static>>(),
        "runtime::self_reconstruction::Attempt"
    );
    assert_eq!(codara::SHADOW_ROOT, historical::SHADOW_ROOT);
    assert_eq!(codara::CANDIDATES_SCHEMA, historical::CANDIDATES_SCHEMA);
    let path = "core/src/donor/mod.rs";
    let extracted = adapter::semantic::rust::RustSemanticExtractor.extract(&ExtractionInput {
        repository: RepositoryId::new("fixture"),
        revision: RevisionRef {
            kind: "GIT_COMMIT".into(),
            value: "fixture".into(),
        },
        artifact: ArtifactId::new(path),
        artifact_path: path.into(),
        source_text: include_str!("../../../core/src/donor/mod.rs").into(),
        content_fingerprint: None,
        source_frontend_id: "atlas.source.rust.bootstrap.v1".into(),
        language: "rust".into(),
        build_profile: None,
        scope_policy: None,
        requested_dimensions: vec![
            SemanticDimension::Symbol,
            SemanticDimension::FunctionIdentity,
            SemanticDimension::FunctionSignature,
        ],
    });
    let records = extracted.observations;
    let before = serde_json::to_vec(&records).unwrap();
    let roots = codara::target_roots(&records, path, "MaterializationMode");
    assert_eq!(
        roots,
        historical::target_roots(&records, path, "MaterializationMode")
    );
    assert!(roots.len() > 1);
    let old = historical::lift(
        &records,
        "fixture-root",
        path,
        "MaterializationMode",
        "design:fixture",
        Some("comparison:fixture"),
    )
    .unwrap();
    let new = codara::lift(
        &records,
        "fixture-root",
        path,
        "MaterializationMode",
        "design:fixture",
        Some("comparison:fixture"),
    )
    .unwrap();
    assert_eq!(new, old);
    assert_eq!(new.module_id, module_identity(&new));
    assert_eq!(new.target, "core/src/donor/mod.rs::MaterializationMode");
    assert!(new.gaps.is_empty());
    assert!(new.inputs.iter().any(|i| i.reference == "design:fixture"));
    let emission = atlas_core::codara::construction::rust::emit(&new);
    assert_eq!(emission.emitted.len(), 2);
    assert!(emission.omitted.is_empty());
    let checks = codara::semantic_checks(&new, &emission, &emission.source);
    assert_eq!(
        checks,
        historical::semantic_checks(&old, &emission, &emission.source)
    );
    assert!(checks.iter().all(|c| c.result == CheckResult::Equivalent));
    let wrong = emission.source.replace(
        "self != Self::RemoteMetadata",
        "self == Self::RemoteMetadata",
    );
    assert_ne!(wrong, emission.source);
    let checks = codara::semantic_checks(&new, &emission, &wrong);
    assert_eq!(checks, historical::semantic_checks(&old, &emission, &wrong));
    assert!(checks.iter().any(|c| c.result == CheckResult::Mismatch));
    assert_eq!(
        codara::declared_variations(&records, &new, &emission),
        historical::declared_variations(&records, &old, &emission)
    );
    assert_eq!(
        codara::oracle_path(path, "MaterializationMode"),
        Some("atlas_core::donor::MaterializationMode".into())
    );
    for path in [
        "adapter/runtime/src/self_reconstruction.rs",
        "adapter/runtime/src/codara/reconstruction.rs",
    ] {
        assert_eq!(codara::oracle_path(path, "Candidate"), None);
    }
    assert_eq!(
        codara::lift(
            &records,
            "fixture-root",
            path,
            "Missing",
            "design:fixture",
            None
        )
        .unwrap_err(),
        historical::lift(
            &records,
            "fixture-root",
            path,
            "Missing",
            "design:fixture",
            None
        )
        .unwrap_err()
    );
    let unsupported = codara::lift(
        &records,
        "fixture-root",
        path,
        "StorageState",
        "design:fixture",
        None,
    )
    .unwrap();
    assert!(!unsupported.gaps.is_empty());
    assert!(unsupported.functions.iter().all(|f| f.body.is_none()));
    assert_eq!(
        serde_json::to_vec(&records).unwrap(),
        before,
        "input observations and evidence remain immutable"
    );
}
