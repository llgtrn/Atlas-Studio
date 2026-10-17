//! Source-only CLI emission uses the retained verified input, without a workspace or toolchain.
//! The historical source text is an oracle for bytes, never native admission or compiler proof.

use std::{fs, path::PathBuf, process::Command};

const BINARY: &str = env!("CARGO_BIN_EXE_atlas-systemizer");
const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tools/atlas/evidence/self-reconstruction/governed-source-handoff"
);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base =
            std::env::temp_dir().join(format!("atlas-cli-emission-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&base).unwrap();
        for name in ["source.atlas", "design.json"] {
            fs::copy(PathBuf::from(FIXTURE).join(name), base.join(name)).unwrap();
        }
        Self(base)
    }

    fn run(&self, extra: &[String]) -> std::process::Output {
        let output = Command::new(BINARY)
            .args([
                "self-reconstruct",
                "emit",
                "--atlas",
                self.0.join("source.atlas").to_str().unwrap(),
                "--path",
                "core/src/lib.rs",
                "--type",
                "Mode",
                "--design",
                self.0.join("design.json").to_str().unwrap(),
            ])
            .args(extra)
            // A successful real caller requires neither workspace sources nor executable tools.
            .env("PATH", self.0.join("absent-toolchain"))
            .current_dir(&self.0)
            .output()
            .unwrap();
        assert!(!self.0.join("target").exists());
        assert!(!self.0.join("report.json").exists());
        output
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn real_emission_preserves_the_full_module_source_identifiers_and_assumptions() {
    let fixture = Fixture::new();
    let before = ["source.atlas", "design.json"].map(|p| fs::read(fixture.0.join(p)).unwrap());
    let output = fixture.run(&[]);
    assert_eq!(output.status.code(), Some(0), "{:?}", output);
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let original: serde_json::Value = serde_json::from_slice(
        &fs::read(PathBuf::from(FIXTURE).join("attempt/module.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(value["module"], original);
    assert_eq!(value.as_object().unwrap().len(), 2);
    assert_eq!(value["emission"].as_object().unwrap().len(), 4);
    assert_eq!(
        value["emission"]["source"].as_str().unwrap().as_bytes(),
        fs::read(PathBuf::from(FIXTURE).join("attempt/shadow.rs.txt")).unwrap()
    );
    assert_eq!(
        value["emission"]["emitted"],
        serde_json::json!([
            "type:core/src/lib.rs::Mode",
            "fn:core/src/lib.rs::Mode::is_local"
        ])
    );
    assert_eq!(value["emission"]["omitted"], serde_json::json!([]));
    assert_eq!(
        value["emission"]["assumptions"],
        serde_json::json!([
            "derive paths: Debug Clone Copy PartialEq Eq PartialOrd Ord Hash Default from the prelude, Serialize Deserialize from serde"
        ])
    );
    assert!(value.get("report").is_none());
    assert!(value.get("verdict").is_none());
    for (name, bytes) in ["source.atlas", "design.json"].into_iter().zip(before) {
        assert_eq!(fs::read(fixture.0.join(name)).unwrap(), bytes);
    }
}

#[test]
fn whole_emission_budget_and_immutable_replay_preserve_every_byte() {
    let fixture = Fixture::new();
    let stdout = fixture.run(&[]).stdout;
    let out = fixture.0.join("evidence/result.json");
    let path = out.to_str().unwrap();
    for limit in [0, 1024, stdout.len() - 1] {
        let output = fixture.run(&args(&[
            "--out",
            path,
            "--max-output-bytes",
            &limit.to_string(),
        ]));
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("RECONSTRUCTION_EMISSION_CAPACITY_EXCEEDED")
        );
        assert!(!out.parent().unwrap().exists());
    }
    let exact = args(&[
        "--out",
        path,
        "--max-output-bytes",
        &stdout.len().to_string(),
    ]);
    for _ in 0..2 {
        let output = fixture.run(&exact);
        assert_eq!(output.status.code(), Some(0), "{:?}", output);
        assert!(output.stdout.is_empty() && output.stderr.is_empty());
        assert_eq!(fs::read(&out).unwrap(), stdout);
    }
    let conflicting = vec![b'x'; stdout.len()];
    fs::write(&out, &conflicting).unwrap();
    let refused = fixture.run(&exact);
    assert_eq!(refused.status.code(), Some(2));
    assert!(refused.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&refused.stderr)
            .contains("existing emission evidence is not an identical regular file")
    );
    assert_eq!(fs::read(&out).unwrap(), conflicting);
}

#[test]
fn stale_parent_and_corrupt_container_refuse_before_creating_evidence() {
    for corrupt_container in [false, true] {
        let fixture = Fixture::new();
        if corrupt_container {
            let path = fixture.0.join("source.atlas");
            let mut bytes = fs::read(&path).unwrap();
            let last = bytes.len() - 1;
            bytes[last] ^= 1;
            fs::write(path, bytes).unwrap();
        } else {
            let path = fixture.0.join("design.json");
            let mut design: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            design["parent_root"] = "wrong-parent".into();
            fs::write(path, serde_json::to_vec(&design).unwrap()).unwrap();
        }
        let out = fixture.0.join("absent/evidence.json");
        let output = fixture.run(&args(&["--out", out.to_str().unwrap()]));
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!out.parent().unwrap().exists());
    }
}

#[test]
fn emission_rejects_workspace_arguments_and_ambiguous_or_invalid_byte_limits() {
    let fixture = Fixture::new();
    let out = fixture.0.join("absent/evidence.json");
    for bad in [
        args(&["--root", "workspace"]),
        args(&["--out-dir", "workspace"]),
        args(&["--max-output-bytes"]),
        args(&["--max-output-bytes", "01"]),
        args(&["--max-output-bytes", "-1"]),
        args(&["--max-output-bytes", "1e6"]),
        args(&["--max-output-bytes=512"]),
        args(&["--max-output-bytes", "1", "--max-output-bytes", "2"]),
        args(&["--design", "another-design.json"]),
    ] {
        let mut arguments = args(&["--out", out.to_str().unwrap()]);
        arguments.extend(bad);
        let output = fixture.run(&arguments);
        assert_eq!(output.status.code(), Some(2), "{:?}", output);
        assert!(output.stdout.is_empty());
        assert!(!out.parent().unwrap().exists());
    }
}
