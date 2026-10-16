//! Real CLI callers retain source-container bytes, evidence and fail-closed root binding.
//! Packaging this temporary source fixture is neither a seal nor native/target admission.

use runtime::{atlas as historical, certificate::CertificateState, codara::container};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

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
            "atlas-cli-container-{}-{nonce}",
            std::process::id()
        ));
        let root = base.join("repo");
        for (path, bytes) in [
            ("src/lib.rs", "pub fn value() -> u8 { 7 }\n"),
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
fn pack_verify_and_certificate_cli_preserve_public_api_bytes_and_unsealed_outcome() {
    let fixture = Fixture::new();
    // Outputs stay outside the root so they cannot alter its census between callers.
    let out = fixture.base.join("fixture.atlas");
    let expected = historical::pack(&fixture.root, &out).unwrap();
    let bytes = fs::read(&out).unwrap();
    let packed = cli(&[
        "atlas",
        "pack",
        "--root",
        path(&fixture.root),
        "--out",
        path(&out),
    ]);
    assert_eq!(
        packed.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&packed.stderr)
    );
    assert!(packed.stderr.is_empty());
    assert_eq!(packed.stdout, report_bytes(&expected));
    assert_eq!(fs::read(&out).unwrap(), bytes);
    assert!(!out.with_extension("atlas.partial").exists());
    let verified = cli(&["atlas", "verify", path(&out), "--root", path(&fixture.root)]);
    assert_eq!(
        verified.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&verified.stderr)
    );
    assert!(verified.stderr.is_empty());
    assert_eq!(verified.stdout, report_bytes(&expected));
    assert_eq!(
        container::verify(&out, Some(&fixture.root)).unwrap(),
        expected
    );

    let expected_certificate = historical::certificate_with_atlas(&fixture.root, &out).unwrap();
    assert_eq!(
        expected_certificate,
        container::certificate_with_atlas(&fixture.root, &out).unwrap()
    );
    assert_ne!(expected_certificate.state, CertificateState::Sealed);
    assert!(
        expected_certificate
            .blockers
            .iter()
            .any(|b| b.starts_with("ATLAS_ROOT_UNSEALED"))
    );
    assert!(
        !expected_certificate
            .blockers
            .iter()
            .any(|b| b.starts_with("ATLAS_ROOT_ABSENT"))
    );
    assert_eq!(
        expected_certificate
            .atlas
            .as_ref()
            .unwrap()
            .root_hash
            .as_deref(),
        Some(expected.root_id.as_str())
    );
    println!(
        "fixture certificate state={:?}; blockers={:?}",
        expected_certificate.state, expected_certificate.blockers
    );
    let certificate_path = fixture.base.join("certificate.json");
    let certificate = cli(&[
        "census",
        "certificate",
        "--root",
        path(&fixture.root),
        "--atlas",
        path(&out),
        "--out",
        path(&certificate_path),
    ]);
    assert_eq!(certificate.stdout, report_bytes(&expected_certificate));
    assert_eq!(fs::read(&certificate_path).unwrap(), certificate.stdout);
    if expected_certificate.state < CertificateState::Closed {
        assert_eq!(certificate.status.code(), Some(2));
        assert_eq!(certificate.stderr, b"atlas-systemizer: CENSUS_NOT_CLOSED\n");
    } else {
        assert_eq!(certificate.status.code(), Some(0));
        assert!(certificate.stderr.is_empty());
    }
    assert_eq!(fs::read(&out).unwrap(), bytes);

    let corrupt = fixture.base.join("corrupt.atlas");
    let mut corrupt_bytes = bytes.clone();
    let last = corrupt_bytes.len() - 1;
    corrupt_bytes[last] ^= 1;
    fs::write(&corrupt, corrupt_bytes).unwrap();
    let corrupt_error = historical::verify(&corrupt, None).unwrap_err();
    let rejected = cli(&["atlas", "verify", path(&corrupt)]);
    assert_eq!(rejected.status.code(), Some(2));
    assert!(rejected.stdout.is_empty());
    assert_eq!(
        String::from_utf8(rejected.stderr).unwrap(),
        format!("atlas-systemizer: {}: {corrupt_error}\n", corrupt.display())
    );

    fs::write(
        fixture.root.join("src/lib.rs"),
        "pub fn value() -> u8 { 8 }\n",
    )
    .unwrap();
    let rejected = cli(&["atlas", "verify", path(&out), "--root", path(&fixture.root)]);
    assert_eq!(rejected.status.code(), Some(2));
    assert!(rejected.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("does not package the current census")
    );
    let absent_report = fixture.base.join("absent-parent").join("certificate.json");
    let rejected = cli(&[
        "census",
        "certificate",
        "--root",
        path(&fixture.root),
        "--atlas",
        path(&out),
        "--out",
        path(&absent_report),
    ]);
    assert_eq!(rejected.status.code(), Some(2));
    assert!(rejected.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("does not package the current census")
    );
    assert!(!absent_report.parent().unwrap().exists());
    assert_eq!(fs::read(&out).unwrap(), bytes);
    assert_eq!(
        fs::read(&certificate_path).unwrap(),
        report_bytes(&expected_certificate)
    );
}
