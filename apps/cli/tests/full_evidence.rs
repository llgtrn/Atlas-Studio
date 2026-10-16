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

fn bounded_decode(object: &Path, out: Option<&Path>, max: usize) -> Output {
    let mut command = Command::new(BINARY);
    command
        .args(["atlasx", "codec", "--decode"])
        .arg(object)
        .arg("--max-report-bytes")
        .arg(max.to_string());
    if let Some(out) = out {
        command.arg("--out").arg(out);
    }
    command.stdin(Stdio::null()).output().unwrap()
}

fn assert_capacity_refusal(output: &Output, bytes: usize, max: usize) {
    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stdout.is_empty(),
        "capacity refusal leaked partial report"
    );
    assert_eq!(
        String::from_utf8(output.stderr.clone()).unwrap(),
        format!("atlas-systemizer: ATLASX_REPORT_CAPACITY_EXCEEDED: bytes={bytes}; max={max}\n")
    );
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
fn bounded_decode_preserves_exact_bytes_and_refuses_without_output_for_all_seven_cases() {
    let scratch = Scratch::new("bounded-decode");
    let mut unicode_cases = 0;
    for case in cases() {
        let (fixture, object, bytes) = stage_case(&scratch.0, &case);
        let baseline = command("--decode", &object, None);
        assert_report(&baseline, &case);
        let report_bytes = baseline.stdout.len();
        assert_eq!(baseline.stdout.last(), Some(&b'\n'));
        println!(
            "{}: pretty_report_bytes_with_lf={report_bytes}",
            case["case"]
        );

        let exact = bounded_decode(&object, None, report_bytes);
        assert_report(&exact, &case);
        assert_eq!(
            exact.stdout, baseline.stdout,
            "bounded stdout changed bytes"
        );
        assert_eq!(exact.stderr, baseline.stderr, "bounded verdict changed");
        for max in [0, report_bytes - 1] {
            assert_capacity_refusal(&bounded_decode(&object, None, max), report_bytes, max);
        }
        let chars = std::str::from_utf8(&baseline.stdout)
            .unwrap()
            .chars()
            .count();
        if chars < report_bytes {
            unicode_cases += 1;
            assert_capacity_refusal(&bounded_decode(&object, None, chars), report_bytes, chars);
        }

        let dir = object.parent().unwrap();
        let report = dir.join("absent-parent").join("report.json");
        assert_capacity_refusal(
            &bounded_decode(&object, Some(&report), report_bytes - 1),
            report_bytes,
            report_bytes - 1,
        );
        assert!(
            !report.parent().unwrap().exists(),
            "capacity refusal created parent"
        );
        let prior = dir.join("prior-report.json");
        fs::write(&prior, b"prior immutable evidence").unwrap();
        assert_capacity_refusal(&bounded_decode(&object, Some(&prior), 0), report_bytes, 0);
        assert_eq!(fs::read(&prior).unwrap(), b"prior immutable evidence");
        let conflict = bounded_decode(&object, Some(&prior), report_bytes);
        assert_eq!(conflict.status.code(), Some(2));
        assert!(conflict.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&conflict.stderr).contains("not an identical regular file")
        );
        assert_eq!(fs::read(&prior).unwrap(), b"prior immutable evidence");

        // Adequate capacity writes every byte even when the codec verdict itself is REFUSED.
        // Replay keeps that verdict and capacity remains a separate prerequisite on replay.
        for _ in 0..2 {
            let output = bounded_decode(&object, Some(&report), report_bytes);
            assert_eq!(output.status.code(), baseline.status.code());
            assert_eq!(output.stderr, baseline.stderr);
            assert!(output.stdout.is_empty());
            assert_eq!(fs::read(&report).unwrap(), baseline.stdout);
        }
        assert_capacity_refusal(
            &bounded_decode(&object, Some(&report), report_bytes - 1),
            report_bytes,
            report_bytes - 1,
        );
        assert_eq!(fs::read(&report).unwrap(), baseline.stdout);
        assert_eq!(fs::read(&object).unwrap(), bytes, "input changed");
        assert_eq!(fs::read(&fixture).unwrap(), bytes, "fixture changed");
    }
    assert!(
        unicode_cases > 0,
        "the real oracle must exercise UTF-8 byte capacity"
    );
}

#[test]
fn report_capacity_argument_errors_fail_before_reading_or_writing() {
    let scratch = Scratch::new("capacity-arguments");
    let missing_input = scratch.0.join("missing.atlasx");
    let out = scratch.0.join("absent-parent").join("report.json");
    let overflow = format!("{}0", usize::MAX);
    let invalid_limits: Vec<Vec<&str>> = vec![
        vec![],
        vec![""],
        vec!["00"],
        vec!["+1"],
        vec!["-1"],
        vec![" 1"],
        vec!["1.0"],
        vec!["١"],
        vec![&overflow],
        vec!["1", "--max-report-bytes", "2"],
    ];
    for values in invalid_limits {
        let output = Command::new(BINARY)
            .args(["atlasx", "codec", "--decode"])
            .arg(&missing_input)
            .arg("--out")
            .arg(&out)
            .arg("--max-report-bytes")
            .args(values)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(
            error.starts_with("atlas-systemizer: --max-report-bytes"),
            "{error}"
        );
        assert!(!out.parent().unwrap().exists());
    }
    for attached in [
        "--max-report-bytes=0",
        "--max-report-bytes=1024",
        "--max-report-bytes=",
    ] {
        let output = Command::new(BINARY)
            .args(["atlasx", "codec", "--decode"])
            .arg(&missing_input)
            .arg("--out")
            .arg(&out)
            .arg(attached)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "atlas-systemizer: --max-report-bytes requires a separate decimal value\n"
        );
        assert!(!out.parent().unwrap().exists());
    }
    for mode in ["--encode", "--codara-payload"] {
        let output = Command::new(BINARY)
            .args(["atlasx", "codec", mode])
            .arg(&missing_input)
            .arg("--out")
            .arg(&out)
            .args(["--max-report-bytes", "0"])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "atlas-systemizer: --max-report-bytes requires atlasx codec --decode only\n"
        );
        assert!(!out.parent().unwrap().exists());
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
