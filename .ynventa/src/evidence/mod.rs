//! Proof evidence. A proof record binds a result to the exact bytes it judged: the proof file and
//! the replacement node's tree. When either changes, the record is stale and proves nothing.
//! Records are content-addressed files in `.ynventa/evidence/`, so concurrent branches never
//! conflict.

use crate::compact::codec::{DecodeError, Decoder, Encoder};
use crate::declare::{Declaration, Proof};
use crate::digest::{content_digest, hex, Sha256};
use crate::repository::files::Files;
use crate::schema::ProofKind;
use std::collections::BTreeMap;
use std::path::Path;

pub const EVIDENCE_DIR: &str = ".ynventa/evidence";
pub const EVIDENCE_TAG: u8 = 2;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Record {
    pub locator: String,
    pub kind: ProofKind,
    pub passed: bool,
    /// Digest of the proof file when the proof ran.
    pub proof_digest: String,
    /// Replacement node key and the digest of its tree when the proof ran.
    pub subject: String,
    pub subject_digest: String,
    pub command: String,
}

impl Record {
    pub fn encode(&self) -> Vec<u8> {
        let mut e = Encoder::new(EVIDENCE_TAG);
        e.str(&self.locator)
            .u8(self.kind.rank())
            .bool(self.passed)
            .str(&self.proof_digest)
            .str(&self.subject)
            .str(&self.subject_digest)
            .str(&self.command);
        e.finish()
    }
    pub fn decode(b: &[u8]) -> Result<Record, DecodeError> {
        let mut d = Decoder::open(b, EVIDENCE_TAG)?;
        let r = Record {
            locator: d.str()?,
            kind: d.word(ProofKind::ALL)?,
            passed: d.bool()?,
            proof_digest: d.str()?,
            subject: d.str()?,
            subject_digest: d.str()?,
            command: d.str()?,
        };
        d.end()?;
        Ok(r)
    }
}

/// Verdict on one declared proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verdict {
    Pass,
    Fail,
    /// Records exist but none matches the current bytes.
    Stale,
    /// The test exists but has never been recorded.
    Unrecorded,
    /// The locator names no test function.
    Absent,
}

impl Verdict {
    pub fn wire(self) -> &'static str {
        match self {
            Verdict::Pass => "PASS",
            Verdict::Fail => "FAIL",
            Verdict::Stale => "STALE",
            Verdict::Unrecorded => "UNRECORDED",
            Verdict::Absent => "ABSENT",
        }
    }
}

/// Every evidence record found on disk, by locator.
#[derive(Clone, Debug, Default)]
pub struct Store {
    pub records: BTreeMap<String, Vec<Record>>,
    pub unreadable: Vec<String>,
}

impl Store {
    pub fn load(root: &Path) -> Store {
        let mut s = Store::default();
        for (name, bytes) in crate::compact::read_addressed(root, EVIDENCE_DIR, &mut s.unreadable) {
            match Record::decode(&bytes) {
                Ok(r) => s.records.entry(r.locator.clone()).or_default().push(r),
                Err(e) => s.unreadable.push(format!("{EVIDENCE_DIR}/{name}: {e}")),
            }
        }
        s
    }

    /// Persists a record under its content address; returns the file path.
    pub fn write(root: &Path, r: &Record) -> std::io::Result<String> {
        crate::compact::write_addressed(root, EVIDENCE_DIR, &r.encode())
    }
}

/// Ynventa's own state (evidence, history, knowledge, locks) never counts as a node's content:
/// recording evidence must not invalidate the evidence just recorded.
pub fn is_state(path: &str) -> bool {
    ["evidence", "history", "knowledge", "materialized"]
        .iter()
        .any(|d| path.starts_with(&format!(".ynventa/{d}/")))
        || path.starts_with("target/")
}

/// Digest of a node's tree: every tracked file below `path`, with its path.
pub fn tree_digest(files: &Files, path: &str) -> String {
    let mut h = Sha256::new();
    let mut any = false;
    for f in files.under(path).filter(|f| !is_state(f)) {
        if let Some(b) = files.read_bytes(f) {
            h.field(f.as_bytes());
            h.field(&b);
            any = true;
        }
    }
    if !any {
        return String::new();
    }
    format!("sha256:{}", hex(&h.finish()))
}

/// Whether `file` defines a test function named `name`.
pub fn test_exists(files: &Files, file: &str, name: &str) -> bool {
    if name.is_empty() || !files.paths.contains(file) {
        return false;
    }
    let Some(text) = files.read(file) else {
        return false;
    };
    let needle = format!("fn {name}(");
    text.match_indices(&needle).any(|(i, _)| {
        let mut start = i.saturating_sub(300);
        while !text.is_char_boundary(start) {
            start += 1;
        }
        let window = &text[start..i];
        window.contains("#[test]") || window.contains("#[tokio::test") || window.contains("_test]")
    })
}

/// The current bytes a proof about `subject` depends on.
pub fn current_digests(
    files: &Files,
    d: &Declaration,
    proof: &Proof,
    subject: &str,
) -> (String, String) {
    let (file, _) = proof.target();
    let proof_digest = files
        .read_bytes(file)
        .map(|b| content_digest(&b))
        .unwrap_or_default();
    // A technology's proofs judge its canonical sources; a node's proofs judge its tree.
    let subject_digest = match subject.strip_prefix("technology/") {
        Some(key) => d
            .technology(key)
            .map(|t| crate::technology::source_digest(files, t))
            .unwrap_or_default(),
        None => d
            .node(subject)
            .map(|n| tree_digest(files, &n.path))
            .unwrap_or_default(),
    };
    (proof_digest, subject_digest)
}

pub fn judge(
    store: &Store,
    files: &Files,
    d: &Declaration,
    proof: &Proof,
    subject: Option<&str>,
) -> Verdict {
    let (file, name) = proof.target();
    if !test_exists(files, file, name) {
        return Verdict::Absent;
    }
    let Some(subject) = subject else {
        return Verdict::Unrecorded;
    };
    let (pd, sd) = current_digests(files, d, proof, subject);
    let Some(records) = store.records.get(&proof.locator) else {
        return Verdict::Unrecorded;
    };
    let fresh: Vec<&Record> = records
        .iter()
        .filter(|r| {
            r.kind == proof.kind
                && r.subject == subject
                && r.proof_digest == pd
                && r.subject_digest == sd
                && !sd.is_empty()
        })
        .collect();
    if fresh.is_empty() {
        Verdict::Stale
    } else if fresh.iter().any(|r| !r.passed) {
        // Any failing run of the exact same bytes outweighs a passing one.
        Verdict::Fail
    } else {
        Verdict::Pass
    }
}

/// Runs proofs. The real runner shells out to cargo in the repository; tests substitute one.
pub trait Runner {
    fn run(&mut self, root: &Path, files: &Files, proof: &Proof) -> (bool, String);
}

pub struct CargoRunner;

impl Runner for CargoRunner {
    fn run(&mut self, root: &Path, files: &Files, proof: &Proof) -> (bool, String) {
        let (file, name) = proof.target();
        let members = crate::census::cargo::members(files);
        let member = members
            .iter()
            .filter(|m| m.dir.is_empty() || file.starts_with(&format!("{}/", m.dir)))
            .max_by_key(|m| m.dir.len());
        let mut args: Vec<String> = vec!["test".into()];
        if let Some(m) = member {
            args.push("-p".into());
            args.push(m.package.clone());
            let rel = file.strip_prefix(&format!("{}/", m.dir)).unwrap_or(file);
            if rel.starts_with("src/") {
                // A unit test: build only the library, not unrelated integration tests.
                args.push("--lib".into());
            } else if let Some(stem) = rel
                .strip_prefix("tests/")
                .and_then(|t| t.strip_suffix(".rs"))
            {
                if !stem.contains('/') {
                    args.push("--test".into());
                    args.push(stem.into());
                }
            }
        }
        args.push("--".into());
        args.push(name.into());
        let manifest = if member.is_some_and(|m| m.dir == ".ynventa") {
            root.join(".ynventa/Cargo.toml")
        } else {
            root.join("Cargo.toml")
        };
        let command = format!("cargo {} (manifest {})", args.join(" "), manifest.display());
        let out = std::process::Command::new("cargo")
            .arg(&args[0])
            .arg("--manifest-path")
            .arg(&manifest)
            .args(&args[1..])
            .output();
        let ok = match out {
            Ok(o) => {
                let text = String::from_utf8_lossy(&o.stdout);
                o.status.success()
                    && text
                        .lines()
                        .any(|l| l.starts_with("test ") && l.contains(name) && l.ends_with(" ok"))
            }
            Err(_) => false,
        };
        (ok, command)
    }
}

/// Runs every declared proof (of donor capabilities and of technologies, optionally only those
/// of one donor or technology key) and records content-bound results.
pub fn prove(
    root: &Path,
    files: &Files,
    d: &Declaration,
    only: Option<&str>,
    runner: &mut dyn Runner,
) -> Vec<(String, bool, String)> {
    // Donor-capability proofs judge the replacement node; technology proofs judge the
    // technology's canonical sources (subject `technology/<key>`).
    let subjects: Vec<(String, &crate::declare::Technology)> = d
        .technologies
        .iter()
        .filter(|t| only.is_none_or(|k| k == t.key))
        .map(|t| (format!("technology/{}", t.key), t))
        .collect();
    let mut jobs: Vec<(&Proof, &str)> = Vec::new();
    for dn in d
        .donors
        .iter()
        .filter(|dn| only.is_none_or(|k| k == dn.key))
    {
        for c in &dn.capabilities {
            if let Some(subject) = &c.replacement {
                jobs.extend(c.proofs.iter().map(|p| (p, subject.as_str())));
            }
        }
    }
    for (subject, t) in &subjects {
        jobs.extend(t.proofs.iter().map(|p| (p, subject.as_str())));
        for c in &t.claims {
            jobs.extend(c.evidence.iter().map(|p| (p, subject.as_str())));
        }
    }
    let mut done = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for (p, subject) in jobs {
        if !done.insert((p.locator.clone(), p.kind, subject.to_string())) {
            continue;
        }
        let (file, name) = p.target();
        if !test_exists(files, file, name) {
            out.push((
                p.locator.clone(),
                false,
                "ABSENT: no such test function".into(),
            ));
            continue;
        }
        let (passed, command) = runner.run(root, files, p);
        let (pd, sd) = current_digests(files, d, p, subject);
        let rec = Record {
            locator: p.locator.clone(),
            kind: p.kind,
            passed,
            proof_digest: pd,
            subject: subject.to_string(),
            subject_digest: sd,
            command,
        };
        match Store::write(root, &rec) {
            Ok(path) => out.push((p.locator.clone(), passed, path)),
            Err(e) => out.push((p.locator.clone(), false, format!("cannot record: {e}"))),
        }
    }
    out
}
