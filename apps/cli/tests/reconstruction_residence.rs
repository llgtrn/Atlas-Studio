//! Real CLI callers retain source-container bytes, evidence and fail-closed root binding.
//! Packaging this temporary source fixture is neither a seal nor native/target admission.

use runtime::{
    codara::{container, reconstruction as codara},
    self_reconstruction as historical,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

// One owned source fixture feeds the real world model, container and all three CLI verbs.
// Opaque cannot emit, so ordinary Cargo regression never recursively invokes Cargo.
const SOURCE: &str = r#"
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode { Remote, Local }
impl Mode { pub fn is_local(self) -> bool { self != Self::Remote } }
pub enum Opaque { Value(u8) }
pub struct Unsafe;
impl Unsafe { pub unsafe fn touch(&self) {} }
"#;

const BINARY: &str = env!("CARGO_BIN_EXE_atlas-systemizer");

struct Fixture {
    base: PathBuf,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!(
            "atlas-cli-reconstruction-{}-{nonce}",
            std::process::id()
        ));
        let root = base.join("repo");
        for (path, bytes) in [
            ("core/src/lib.rs", SOURCE),
            (
                "tools/atlas/declared/system.adl",
                "atlas 1\nsystem Fixture\nentity Runtime Core {\n kind = backend\n language = rust\n responsibility = fixture\n}\nmaterialize Core {\n path = \"core\"\n language = rust\n glob = \"src/**/*.rs\"\n}\n",
            ),
            (
                "core/Cargo.toml",
                "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
            ),
            (
                "Cargo.toml",
                "[workspace]\nresolver = \"2\"\nmembers = [\"core\"]\n",
            ),
            (
                "Cargo.lock",
                "version = 4\n\n[[package]]\nname = \"core\"\nversion = \"0.1.0\"\n",
            ),
            (
                "tools/atlas/genome/atlas.genome.toml",
                "schema = \"atlas.genome.v1\"\n",
            ),
            (
                "tools/atlas/repo.toml",
                "schema = \"atlas.repo.v3\"\nrepo = \"test/container-fixture\"\nsystem_kind = \"SYSTEM_INVENTION_FORGE\"\nbackend_language = \"rust\"\nfrontend_language = \"typescript\"\ncoding_requires_docs_gate = true\ngraph_before_code_required = true\nexact_base_sha_required = true\nsingle_repository_target_required = true\nknowledge_root = \".ynventa\"\ntemporary_root = \"target/donors\"\nprovenance_root = \"tools/atlas/provenance\"\nlicense_root = \"tools/atlas/licenses\"\n[code]\nsource_roots = [\"core/src\"]\nbackend_roots = [\"core/src\"]\nfrontend_roots = []\ntest_roots = []\n",
            ),
        ] {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        for args in [
            vec!["init", "--quiet"],
            vec!["add", "."],
            vec!["commit", "--quiet", "-m", "fixture"],
        ] {
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
                .args(&args)
                .current_dir(&root)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Self { base, root }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

fn cli(args: &[&str]) -> Output {
    Command::new(BINARY)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn path(path: &Path) -> &str {
    path.to_str().unwrap()
}

fn report_bytes<T: serde::Serialize>(report: &T) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(report).unwrap();
    bytes.push(b'\n');
    bytes
}

#[test]
fn real_callers_preserve_candidates_roots_gap_report_and_parent_refusal() {
    let fixture = Fixture::new();
    let model = runtime::agent::world_model(&fixture.root).unwrap();
    let candidates = historical::candidates(&model);
    assert_eq!(candidates, codara::candidates(&model));
    assert!(candidates.iter().any(|c| c.owner_type == "Mode"));
    assert!(
        candidates
            .iter()
            .any(|c| c.owner_type == "Unsafe"
                && c.excluded_by.iter().any(|e| e == "UNSAFE_OR_ASYNC"))
    );
    let model_path = fixture.base.join("model.json");
    fs::write(&model_path, report_bytes(&model)).unwrap();
    let output = cli(&[
        "self-reconstruct",
        "candidates",
        "--model",
        path(&model_path),
        "--limit",
        "1",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::json!({
            "schema": historical::CANDIDATES_SCHEMA, "revision": model.revision,
            "types": candidates.len(), "constructible": candidates.iter().filter(|c| c.constructible).count(),
            "candidates": candidates.into_iter().take(1).collect::<Vec<_>>()
        })
    );
    let atlas_path = fixture.base.join("source.atlas");
    let packed = container::pack(&fixture.root, &atlas_path).unwrap();
    let (atlas, root) = runtime::design::read_container(&atlas_path).unwrap();
    assert_eq!(root, packed.root_id);
    assert!(atlas.seal.is_none());
    let roots = historical::target_roots(&atlas.typed_records, "core/src/lib.rs", "Opaque");
    assert_eq!(roots.len(), 1);
    let output = cli(&[
        "self-reconstruct",
        "roots",
        "--atlas",
        path(&atlas_path),
        "--path",
        "core/src/lib.rs",
        "--type",
        "Opaque",
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, format!("{}\n", roots.join(",")).as_bytes());
    // Candidate is unselected data, with no invented principal, event, grant or admission.
    let mut design: runtime::design::Design = serde_json::from_value(serde_json::json!({
        "schema": "atlas.selected-design.v1", "design_id": "fixture:unselected",
        "parent_root": root, "candidate": packed.census_digest,
        "scope": "fixture", "target_kind": "RUST", "variant": "unselected", "state": "CANDIDATE",
        "roots": [], "bindings": [], "candidate_set": [], "evidence_refs": [],
        "authority_mode": "HUMAN_REQUIRED", "authority_event": null,
        "rationale": "Untrusted candidate input exercises existing reader, never selection or admission",
        "supersedes": null
    })).unwrap();
    let design_path = fixture.base.join("design.json");
    fs::write(&design_path, report_bytes(&design)).unwrap();
    let input = historical::Attempt {
        root: &fixture.root,
        container: &atlas,
        container_root: &root,
        path: "core/src/lib.rs",
        type_name: "Opaque",
        design_id: &design.design_id,
        design_state: design.state,
        comparison_id: None,
    };
    let (module, emission, report) = historical::attempt(&input).unwrap();
    assert!(emission.emitted.is_empty());
    assert!(!module.gaps.is_empty());
    assert!(report.shadow.is_none());
    assert_eq!(
        serde_json::to_value(report.design_state).unwrap(),
        "CANDIDATE"
    );
    assert!(historical::validate(&atlas, &module, &report).is_empty());
    let new = codara::attempt(&input).unwrap();
    assert_eq!(module, new.0);
    assert_eq!(emission, new.1);
    assert_eq!(report, new.2);
    let out_dir = fixture.base.join("attempt");
    let output = cli(&[
        "self-reconstruct",
        "attempt",
        "--atlas",
        path(&atlas_path),
        "--path",
        "core/src/lib.rs",
        "--type",
        "Opaque",
        "--design",
        path(&design_path),
        "--root",
        path(&fixture.root),
        "--out-dir",
        path(&out_dir),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert_eq!(
        fs::read(out_dir.join("module.json")).unwrap(),
        report_bytes(&module)
    );
    assert_eq!(
        fs::read(out_dir.join("report.json")).unwrap(),
        report_bytes(&report)
    );
    assert_eq!(
        fs::read(out_dir.join("shadow.rs.txt")).unwrap(),
        emission.source.as_bytes()
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::json!({
            "verdict": report.verdict, "emitted": emission.emitted, "omitted": emission.omitted,
            "gaps": report.gaps.len(), "checks": report.checks.len(), "violations": []
        })
    );
    assert!(!fixture.root.join(historical::SHADOW_ROOT).exists());
    let mut broken = module.clone();
    broken.inputs.retain(|i| i.reference != design.design_id);
    assert!(!codara::validate(&atlas, &broken, &report).is_empty());
    design.parent_root = "wrong-root".into();
    fs::write(&design_path, report_bytes(&design)).unwrap();
    let refused_dir = fixture.base.join("refused");
    let output = cli(&[
        "self-reconstruct",
        "attempt",
        "--atlas",
        path(&atlas_path),
        "--path",
        "core/src/lib.rs",
        "--type",
        "Opaque",
        "--design",
        path(&design_path),
        "--root",
        path(&fixture.root),
        "--out-dir",
        path(&refused_dir),
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("RECONSTRUCTION_INVALID"));
    assert!(!refused_dir.exists());
    assert!(!fixture.root.join(historical::SHADOW_ROOT).exists());
}
