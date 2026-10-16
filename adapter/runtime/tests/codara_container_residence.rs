//! Existing source-container callers across the historical and Codara public routes.
//! A fixture census remains unsealed and supplies no native identity or target authority.

use runtime::{
    atlas as historical,
    certificate::{CertificateInputs, CertificateState, certify, genome_identity},
    codara::{artifact::inputs, container as codara},
    recensus::CensusSnapshot,
};
use std::{
    any::type_name,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const SOURCE: &str = "pub fn value() -> u8 { 7 }\n";

struct Fixture {
    base: PathBuf,
    root: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!(
            "atlas-runtime-container-{label}-{}-{nonce}",
            std::process::id()
        ));
        let root = base.join("repo");
        for (path, bytes) in [
            ("src/lib.rs", SOURCE),
            (
                "Cargo.toml",
                "[package]\nname = \"container-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            ),
            (
                "Cargo.lock",
                "version = 4\n\n[[package]]\nname = \"container-fixture\"\nversion = \"0.1.0\"\n",
            ),
            (
                "tools/atlas/genome/atlas.genome.toml",
                "schema = \"atlas.genome.v1\"\n",
            ),
            (
                "tools/atlas/repo.toml",
                "schema = \"atlas.repo.v3\"\nrepo = \"test/container-fixture\"\nsystem_kind = \"SYSTEM_INVENTION_FORGE\"\nbackend_language = \"rust\"\nfrontend_language = \"typescript\"\ncoding_requires_docs_gate = true\ngraph_before_code_required = true\nexact_base_sha_required = true\nsingle_repository_target_required = true\nknowledge_root = \".ynventa\"\ntemporary_root = \"target/donors\"\nprovenance_root = \"tools/atlas/provenance\"\nlicense_root = \"tools/atlas/licenses\"\n[code]\nsource_roots = [\"src\"]\nbackend_roots = [\"src\"]\nfrontend_roots = []\ntest_roots = []\n",
            ),
        ] {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        git(&root, &["init", "--quiet"]);
        git(&root, &["add", "."]);
        git(&root, &["commit", "--quiet", "-m", "fixture"]);
        Self { base, root }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=Atlas fixture",
            "-c",
            "user.email=fixture@example.invalid",
        ])
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn sorted_rows<T: serde::Serialize>(rows: &[T]) -> Vec<String> {
    let mut rows: Vec<_> = rows
        .iter()
        .map(|row| serde_json::to_string(row).unwrap())
        .collect();
    rows.sort();
    rows
}

#[test]
fn public_routes_package_the_same_real_census_bytes_and_historical_ids() {
    let fixture = Fixture::new("public-routes");
    assert_eq!(
        type_name::<codara::PackOutcome>(),
        "runtime::atlas::PackOutcome"
    );
    assert_eq!(
        type_name::<historical::PackOutcome>(),
        type_name::<codara::PackOutcome>()
    );
    let report = runtime::systemize(&fixture.root).unwrap();
    assert!(!report.snapshot.dirty);
    let source = report
        .inventory
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "src/lib.rs")
        .unwrap();
    // This is the existing path-derived source ArtifactId, not a native art1 identity.
    assert_eq!(source.id.as_str(), "artifact:2fba4152af2c3a3a");
    let digest = CensusSnapshot::from_report(&report).census_digest;
    let replay = runtime::recensus::snapshot(&fixture.root)
        .unwrap()
        .census_digest;
    let (schema, genome) = genome_identity(&fixture.root).unwrap();
    let certificate = certify(
        &report,
        &CertificateInputs {
            genome_schema: &schema,
            genome_hash: genome.clone(),
            pass_digests: &[digest, replay],
            // Existing census_atlas self-reference convention; the result is explicitly UNSEALED.
            atlas_root_hash: Some(String::new()),
            atlas_root_sealed: false,
        },
    );
    let direct = codara::atlas_of(&report, &schema, &genome, &certificate).unwrap();
    let old = historical::census_atlas(&fixture.root).unwrap();
    let new = codara::census_atlas(&fixture.root).unwrap();
    assert_eq!(direct, old);
    assert_eq!(old, new);
    assert_eq!(new.manifest.seal, inputs::UNSEALED);
    assert!(new.seal.is_none());
    assert!(
        new.certificate
            .blockers
            .iter()
            .any(|b| b.starts_with("ATLAS_ROOT_UNSEALED"))
    );
    assert!(
        !new.certificate
            .blockers
            .iter()
            .any(|b| b.starts_with("ATLAS_ROOT_ABSENT"))
    );
    assert!(
        !new.typed_records.is_empty(),
        "fixture must exercise typed source records"
    );
    assert!(
        !new.evidence.is_empty(),
        "fixture must exercise immutable evidence"
    );
    assert_eq!(
        sorted_rows(&new.typed_records),
        sorted_rows(&report.census.typed_semantic_records)
    );
    assert_eq!(
        sorted_rows(&new.evidence),
        sorted_rows(&report.census.evidence)
    );
    assert_eq!(
        sorted_rows(&new.diagnostics),
        sorted_rows(&report.census.diagnostics)
    );
    assert!(
        new.obligations
            .iter()
            .any(|obligation| obligation.artifact == source.id.as_str())
    );
    let old_bytes = inputs::write(&old).unwrap();
    let new_bytes = inputs::write(&new).unwrap();
    assert_eq!(old_bytes, new_bytes);
    let (read_back, root_id) = inputs::read(&new_bytes).unwrap();
    assert_eq!(read_back, new);
    let out = fixture.base.join("source.atlas");
    let packed: historical::PackOutcome = codara::publish(&new, &out).unwrap();
    assert_eq!(packed.root_id, root_id.as_str());
    assert_eq!(packed.bytes, new_bytes.len());
    assert_eq!(fs::read(&out).unwrap(), new_bytes);
    let verified: codara::PackOutcome = historical::verify(&out, Some(&fixture.root)).unwrap();
    assert_eq!(verified, packed);
    assert_eq!(codara::verify(&out, Some(&fixture.root)).unwrap(), packed);
    let with_root = codara::certificate_with_atlas(&fixture.root, &out).unwrap();
    assert_eq!(
        with_root,
        historical::certificate_with_atlas(&fixture.root, &out).unwrap()
    );
    assert_eq!(
        with_root.atlas.as_ref().unwrap().root_hash.as_deref(),
        Some(packed.root_id.as_str())
    );
    assert_ne!(with_root.state, CertificateState::Sealed);
    assert!(
        with_root
            .blockers
            .iter()
            .any(|b| b.starts_with("ATLAS_ROOT_UNSEALED"))
    );
}

#[test]
fn corrupt_content_source_changes_and_revision_drift_refuse_current_root_binding() {
    let fixture = Fixture::new("drift");
    let out = fixture.base.join("source.atlas");
    let original = historical::pack(&fixture.root, &out).unwrap();
    let bytes = fs::read(&out).unwrap();
    let corrupt = fixture.base.join("corrupt.atlas");
    let mut changed = bytes.clone();
    let last = changed.len() - 1;
    changed[last] ^= 1;
    fs::write(&corrupt, changed).unwrap();
    assert!(historical::verify(&corrupt, None).is_err());
    assert!(codara::verify(&corrupt, None).is_err());

    let assert_drift_refused = || {
        let old = historical::verify(&out, Some(&fixture.root)).unwrap_err();
        let new = codara::verify(&out, Some(&fixture.root)).unwrap_err();
        assert_eq!(old.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(old.to_string(), new.to_string());
        assert!(
            old.to_string()
                .contains("does not package the current census")
        );
        assert!(codara::certificate_with_atlas(&fixture.root, &out).is_err());
        assert_eq!(fs::read(&out).unwrap(), bytes);
        assert_eq!(codara::verify(&out, None).unwrap(), original);
    };
    fs::write(
        fixture.root.join("src/lib.rs"),
        "pub fn value() -> u8 { 8 }\n",
    )
    .unwrap();
    assert_drift_refused();
    fs::write(fixture.root.join("src/lib.rs"), SOURCE).unwrap();
    assert_eq!(codara::verify(&out, Some(&fixture.root)).unwrap(), original);
    git(
        &fixture.root,
        &["commit", "--quiet", "--allow-empty", "-m", "revision drift"],
    );
    assert_drift_refused();
}
