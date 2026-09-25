use super::*;
use crate::census::extraction::{
    CensusExtractionAccounting, extract_semantics, requested_dimensions,
};
use atlas_core::{ArtifactId, ArtifactKind, ArtifactRecord, RepositoryId, RevisionRef};
use std::time::{SystemTime, UNIX_EPOCH};

fn map(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn crate_targets_follow_manifests_renames_roles_and_own_library() {
    let manifests = map(&[
        ("Cargo.toml", "[workspace]\nmembers = [\"core\", \"app\"]\n"),
        ("core/Cargo.toml", "[package]\nname = \"core\"\n"),
        (
            "app/Cargo.toml",
            "[package]\nname = \"atlas-app\"\n\n[dependencies]\natlas_core = { package = \"core\", path = \"../core\" }\nserde = \"1\"\n\n[build-dependencies]\nbuild_core = { package = \"core\", path = \"../core\" }\n",
        ),
    ]);
    let sources = map(&[
        ("core/src/lib.rs", ""),
        ("app/src/lib.rs", ""),
        ("app/src/main.rs", ""),
        ("app/build.rs", ""),
    ]);
    let crates = crate_targets(&manifests, &sources);
    let described: Vec<(String, Vec<(String, String)>)> = crates
        .iter()
        .map(|c| {
            (
                c.root.clone(),
                c.externs
                    .iter()
                    .map(|(name, index)| (name.clone(), crates[*index].root.clone()))
                    .collect(),
            )
        })
        .collect();
    let s = |v: &str| v.to_owned();
    assert_eq!(
        described,
        [
            (
                s("app/src/lib.rs"),
                vec![(s("atlas_core"), s("core/src/lib.rs"))]
            ),
            (s("core/src/lib.rs"), vec![]),
            (
                s("app/src/main.rs"),
                vec![
                    (s("atlas_app"), s("app/src/lib.rs")),
                    (s("atlas_core"), s("core/src/lib.rs")),
                ]
            ),
            (
                s("app/build.rs"),
                vec![(s("build_core"), s("core/src/lib.rs"))]
            ),
        ]
    );
}

fn artifact(path: &str, language: &str) -> ArtifactRecord {
    ArtifactRecord {
        id: ArtifactId::new(format!("artifact:{path}")),
        path: path.to_owned(),
        kind: ArtifactKind::File,
        bytes: 0,
        disposition: ArtifactDisposition::Parsed,
        language: Some(language.into()),
        reason: None,
        content_digest: None,
        content_digest_withheld: None,
    }
}

fn workspace() -> (std::path::PathBuf, InventoryReport) {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "atlas-g75-resolution-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(dir.join("core/src")).unwrap();
    fs::write(dir.join("core/Cargo.toml"), "[package]\nname = \"core\"\n").unwrap();
    fs::write(
        dir.join("core/src/lib.rs"),
        "pub mod a;\npub fn f() {\n    a::g();\n    helper();\n    x.method();\n}\nfn helper() {}\n",
    )
    .unwrap();
    fs::write(dir.join("core/src/a.rs"), "pub fn g() {}\n").unwrap();
    let inventory = InventoryReport::new(
        dir.to_string_lossy().into_owned(),
        vec![
            artifact("core/Cargo.toml", "toml"),
            artifact("core/src/lib.rs", "rust"),
            artifact("core/src/a.rs", "rust"),
        ],
    );
    (dir, inventory)
}

fn revision() -> RevisionRef {
    RevisionRef {
        kind: "git".into(),
        value: "abc123".into(),
    }
}

#[test]
fn resolutions_observe_the_syntactic_claims_and_name_their_callee_identity() {
    let (dir, inventory) = workspace();
    let batches = extract_semantics(&inventory, RepositoryId::new("atlas-studio"), revision());
    let resolution = resolve_rust_path_calls(&inventory, &batches);
    assert_eq!(resolution.len(), 2, "one batch per reached Rust artifact");

    let calls = |batch: &ExtractionBatch| -> Vec<atlas_core::SemanticRecordHeader<atlas_core::CallSiteIdentity>> {
        batch
            .observations
            .iter()
            .filter_map(|o| match o {
                SemanticObservation::Call(h) => Some(h.clone()),
                _ => None,
            })
            .collect()
    };
    let syntactic_lib = batches
        .iter()
        .find(|b| b.artifact.as_str() == "artifact:core/src/lib.rs")
        .unwrap();
    let lib = resolution
        .iter()
        .find(|b| b.artifact.as_str() == "artifact:core/src/lib.rs")
        .unwrap();
    let resolved = calls(lib);
    assert_eq!(
        resolved.len(),
        2,
        "a::g() and helper(); never the method call"
    );
    let functions: BTreeMap<String, SemanticRecordId> = batches
        .iter()
        .flat_map(|b| &b.observations)
        .filter_map(|o| match o {
            SemanticObservation::FunctionIdentity(h) => {
                Some((h.subject.symbol.name.clone(), h.record_id.clone()))
            }
            _ => None,
        })
        .collect();
    for call in &resolved {
        // The same claim the syntactic extractor made.
        let claim = calls(syntactic_lib)
            .into_iter()
            .find(|c| c.record_id == call.record_id)
            .expect("a resolution observes an existing CALL claim");
        assert_eq!(claim.subject.span, call.subject.span);
        assert_eq!(claim.subject.dispatch, CallDispatchKind::Unresolved);
        assert_eq!(call.subject.dispatch, CallDispatchKind::StaticResolved);
        assert_eq!(call.extractor.id, RUST_PATH_RESOLUTION_ID);
        assert_eq!(call.status, EpistemicStatus::Derived);
    }
    let callees: Vec<&SemanticRecordId> = resolved.iter().map(|c| &c.subject.callees[0]).collect();
    assert_eq!(callees, [&functions["g"], &functions["helper"]]);

    // The engine accounts for CALL only, and closure holds per engine.
    assert_eq!(lib.obligations.len(), 1);
    assert_eq!(lib.obligations[0].status, EpistemicStatus::Unknown);
    let mut accounting = CensusExtractionAccounting::new();
    for batch in batches.iter().chain(&resolution) {
        accounting.record_batch(batch);
    }
    assert!(accounting.is_closed_per_extractor(requested_dimensions));
    assert!(
        !accounting.is_closed(&crate::census::extraction::ALL_SEMANTIC_DIMENSIONS),
        "the resolution engine is not asked for every dimension"
    );

    // Two engines' observations of one claim are not a normalization conflict.
    let all: Vec<ExtractionBatch> = batches.iter().chain(&resolution).cloned().collect();
    let source = adapter::source_report_from_inventory(&inventory);
    let adl = atlas_core::compile_adl(&[], &source);
    let census = crate::census::build_census(&inventory, &source, &adl, &all, &revision());
    let normalized = crate::normalize::normalize(&census);
    assert!(normalized.conflict_candidates.is_empty());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_resolution_without_a_syntactic_claim_is_a_diagnosed_disagreement() {
    let (dir, inventory) = workspace();
    let mut batches = extract_semantics(&inventory, RepositoryId::new("atlas-studio"), revision());
    // Drop the syntactic claim for `a::g()` (anchored at `g`, line 3) and keep the others.
    let mut dropped = None;
    for batch in &mut batches {
        batch.observations.retain(|o| match o {
            SemanticObservation::Call(h) if h.subject.span.line == 3 => {
                dropped = Some(h.record_id.clone());
                false
            }
            _ => true,
        });
    }
    let dropped = dropped.expect("the a::g() claim existed");
    let resolution = resolve_rust_path_calls(&inventory, &batches);
    let lib = resolution
        .iter()
        .find(|b| b.artifact.as_str() == "artifact:core/src/lib.rs")
        .unwrap();
    let observed: Vec<&SemanticRecordId> = lib.observations.iter().map(|o| o.record_id()).collect();
    assert_eq!(
        observed.len(),
        1,
        "only helper() still has a claim to observe"
    );
    assert!(!observed.contains(&&dropped), "never a fabricated claim");
    assert!(
        lib.diagnostics
            .iter()
            .any(|d| d.message.starts_with("1 resolved path call(s)")),
        "{:#?}",
        lib.diagnostics
    );
    assert_eq!(lib.obligations[0].diagnostics.len(), 2);
    fs::remove_dir_all(&dir).unwrap();
}
