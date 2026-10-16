//! Source caller regression only: full legacy FUNCTIONS reports and immutable payload output.
//! The frozen oracle is not a target wire contract, admission or native qualification.

use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

const BINARY: &str = env!("CARGO_BIN_EXE_atlas-systemizer");
const PARITY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../core/src/codara/artifact/codec/parity"
);

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "atlas-cli-full-evidence-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn cases() -> Vec<Value> {
    let oracle: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../core/src/codara/artifact/codec/parity/full-evidence.json"
    )))
    .unwrap();
    assert_eq!(
        oracle["test_only_oracle"],
        "atlas.functions-full-evidence.v1"
    );
    let cases = oracle["cases"].as_array().unwrap().clone();
    assert_eq!(cases.len(), 7);
    cases
}

fn stage_case(root: &Path, case: &Value) -> (PathBuf, PathBuf, Vec<u8>) {
    let fixture = Path::new(PARITY).join(case["fixture"].as_str().unwrap());
    let bytes = fs::read(&fixture).unwrap();
    assert_eq!(bytes.len() as u64, case["input_bytes"].as_u64().unwrap());
    let dir = root.join(case["case"].as_str().unwrap());
    fs::create_dir(&dir).unwrap();
    // The filename is a caller input: two cases intentionally give otherwise-valid bytes a
    // wrong name, and semantic duplicate-identity diagnoses must still take precedence.
    let object = dir.join(case["file_name"].as_str().unwrap());
    fs::write(&object, &bytes).unwrap();
    (fixture, object, bytes)
}

fn command(mode: &str, object: &Path, out: Option<&Path>) -> Output {
    let mut command = Command::new(BINARY);
    command.args(["atlasx", "codec", mode]).arg(object);
    if let Some(out) = out {
        command.arg("--out").arg(out);
    }
    command.stdin(Stdio::null()).output().unwrap()
}

fn assert_report(output: &Output, case: &Value) {
    let label = case["case"].as_str().unwrap();
    let report: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{label}: invalid report: {error}; stdout={}; stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(
        report, case["source_report"],
        "{label}: full report changed"
    );
    if case["source_report"]["verdict"] == "DECODED" {
        assert_eq!(output.status.code(), Some(0), "{label}");
        assert!(output.stderr.is_empty(), "{label}: unexpected stderr");
    } else {
        assert_eq!(
            output.status.code(),
            Some(2),
            "{label}: refusal exit changed"
        );
        let defects = case["source_report"]["defects"].as_array().unwrap().len();
        assert_eq!(
            String::from_utf8(output.stderr.clone()).unwrap(),
            format!("atlas-systemizer: ATLASX_CODEC_REFUSED: {defects} defects\n"),
            "{label}: refusal diagnostic changed"
        );
    }
}

#[test]
fn decode_stdout_and_exit_preserve_all_seven_full_source_reports() {
    let scratch = Scratch::new("decode");
    for case in cases() {
        let (fixture, object, bytes) = stage_case(&scratch.0, &case);
        let output = command("--decode", &object, None);
        assert_report(&output, &case);
        assert_eq!(fs::read(&object).unwrap(), bytes, "input changed");
        assert_eq!(fs::read(&fixture).unwrap(), bytes, "fixture changed");
        assert_eq!(fs::read_dir(object.parent().unwrap()).unwrap().count(), 1);
    }
}

#[test]
fn payload_export_preserves_full_reports_and_existing_evidence_for_all_seven_cases() {
    let scratch = Scratch::new("payload");
    for case in cases() {
        let (fixture, object, bytes) = stage_case(&scratch.0, &case);
        let dir = object.parent().unwrap();
        let out = dir.join("payload.bin");
        assert_report(&command("--codara-payload", &object, Some(&out)), &case);
        if case["source_report"]["verdict"] == "DECODED" {
            let expected = fs::read(Path::new(PARITY).join("codara-parity.payload")).unwrap();
            assert_eq!(fs::read(&out).unwrap(), expected);
            assert_eq!(fs::read(&out).unwrap(), bytes[80..]);
            assert_report(&command("--codara-payload", &object, Some(&out)), &case);
            assert_eq!(fs::read(&out).unwrap(), expected, "replay changed output");
        } else {
            assert!(!out.exists(), "refusal created output");
            let absent = dir.join("absent-parent").join("payload.bin");
            assert_report(&command("--codara-payload", &object, Some(&absent)), &case);
            assert!(!absent.parent().unwrap().exists(), "refusal created parent");
            fs::write(&out, b"prior evidence").unwrap();
            assert_report(&command("--codara-payload", &object, Some(&out)), &case);
            assert_eq!(fs::read(&out).unwrap(), b"prior evidence");
        }
        assert_eq!(fs::read(&object).unwrap(), bytes, "input changed");
        assert_eq!(fs::read(&fixture).unwrap(), bytes, "fixture changed");
    }
}
