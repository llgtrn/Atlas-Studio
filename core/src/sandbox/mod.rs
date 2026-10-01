//! Sandboxed execution records (G126, NA-SANDBOX-SUBPROCESS; G189, NA-SANDBOX-FS-CONFINEMENT,
//! construction node M22).
//!
//! A `SandboxRequest` declares everything a run may use: its command, its input files with their
//! content digests, the environment variables it may see, and a timeout. A backend stages exactly
//! those inputs, runs the command, and returns a `SandboxRun` recording what was used and what was
//! produced, together with every isolation property it claims -- each `ENFORCED` (the backend
//! applied a mechanism and, where possible, probed it) or `UNENFORCED` with the reason. A property
//! a backend cannot enforce is never implied: the restricted-subprocess class does not confine
//! absolute-path filesystem reads, and says so.
//!
//! A request may ask for the confined class (`confinement`): the run then sees a root holding only
//! its staged inputs, `/dev/null`, the declared read-only toolchain paths and, when asked, a procfs
//! of its own pid namespace. Every run records the identity of the toolchain it ran with
//! (`ToolchainIdentity`); a request may pin one, and a run whose identity is absent or differs
//! from the pin is refused before it runs.
//!
//! Pure vocabulary; the backends live in `runtime::sandbox`. See ADRs 0047 and 0102.

use crate::EpistemicStatus;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// v2 (G189): a run carries `toolchain` and `refusal_kind`; a request may carry `confinement`,
/// `toolchain_binaries` and `pinned_toolchain`. Every added field is optional on read, so a v1
/// record still reads (with no toolchain identity and an untyped refusal).
pub const SANDBOX_RUN_SCHEMA: &str = "atlas.sandbox-run.v2";
/// The schema of records written before G189.
pub const SANDBOX_RUN_SCHEMA_V1: &str = "atlas.sandbox-run.v1";

/// One declared input: a path relative to the staging root and its content digest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclaredInput {
    pub path: String,
    pub digest: String,
}

/// An absolute host path bound read-only (and `nosuid`, `nodev`) at the same path inside a
/// confined root: a toolchain directory or file the run may read and execute but never write.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ToolchainPath {
    pub path: String,
}

/// The confined class: what exists inside the run's root besides `/work` (the staged inputs, the
/// only writable path) and `/dev/null`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Confinement {
    /// Read-only toolchain paths. Absolute, normalized, existing, none inside another, none
    /// covering `/work`, `/dev`, `/proc` or the staging directory.
    pub toolchain: Vec<ToolchainPath>,
    /// Mount a procfs showing only the run's own pid namespace (`subset=pid`), for programs that
    /// locate themselves through `/proc/self/exe` (the dynamic loader's `$ORIGIN`, as `rustc`
    /// needs). Off unless asked.
    #[serde(default)]
    pub proc: bool,
}

/// One binary a run used: the path it was found at, that path with every link resolved, and the
/// content digest of the resolved file (the digest function of declared inputs).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct BinaryIdentity {
    pub path: String,
    pub resolved: String,
    pub digest: String,
}

/// The identity of the toolchain a run ran with.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolchainIdentity {
    /// The program, as resolved on the declared `PATH`.
    pub program: BinaryIdentity,
    /// The declared `toolchain_binaries`, sorted by path.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub binaries: Vec<BinaryIdentity>,
    /// `<program> -vV` standard output, when the program is `rustc` or `cargo`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

impl ToolchainIdentity {
    /// The first field in which `observed` differs from this pinned identity, or `None`.
    pub fn mismatch(&self, observed: &ToolchainIdentity) -> Option<String> {
        fn binary(what: &str, pinned: &BinaryIdentity, seen: &BinaryIdentity) -> Option<String> {
            [
                ("path", &pinned.path, &seen.path),
                ("resolved", &pinned.resolved, &seen.resolved),
                ("digest", &pinned.digest, &seen.digest),
            ]
            .into_iter()
            .find(|(_, p, s)| p != s)
            .map(|(field, p, s)| format!("{what} {field}: pinned `{p}`, observed `{s}`"))
        }
        if let Some(diff) = binary("program", &self.program, &observed.program) {
            return Some(diff);
        }
        if self.binaries.len() != observed.binaries.len() {
            return Some(format!(
                "binaries: pinned {}, observed {}",
                self.binaries.len(),
                observed.binaries.len()
            ));
        }
        for (pinned, seen) in self.binaries.iter().zip(&observed.binaries) {
            if let Some(diff) = binary("binary", pinned, seen) {
                return Some(diff);
            }
        }
        (self.version != observed.version).then(|| {
            format!(
                "version: pinned {:?}, observed {:?}",
                self.version, observed.version
            )
        })
    }
}

/// Everything a sandboxed run may use.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SandboxRequest {
    pub program: String,
    pub args: Vec<String>,
    pub inputs: Vec<DeclaredInput>,
    /// The only environment variables the run sees.
    pub env: BTreeMap<String, String>,
    /// Relative paths the run is expected to produce; their digests are recorded.
    pub outputs: Vec<String>,
    pub timeout_ms: u64,
    pub deny_network: bool,
    /// `Some`: the confined class (ADR 0102). `None`: the restricted-subprocess class.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confinement: Option<Confinement>,
    /// Absolute paths of further binaries whose identity the run records (a linker, `cc`); in the
    /// confined class each must lie under a declared toolchain path.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub toolchain_binaries: Vec<String>,
    /// The toolchain identity the run must have; any difference refuses it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_toolchain: Option<ToolchainIdentity>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IsolationProperty {
    /// Only the declared environment is visible.
    EnvironmentCleared,
    /// Exactly the declared inputs, digest-verified, are staged in a fresh directory.
    InputsStaged,
    /// The working directory is the staging directory: relative paths reach declared inputs only.
    WorkingDirectoryConfined,
    /// Standard input is closed.
    StdinClosed,
    /// The run is killed at its timeout.
    TimeLimited,
    /// No network interface but loopback is reachable.
    NetworkDenied,
    /// Absolute-path reads outside the declared inputs are refused.
    FilesystemConfined,
}

impl IsolationProperty {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::EnvironmentCleared => "ENVIRONMENT_CLEARED",
            Self::InputsStaged => "INPUTS_STAGED",
            Self::WorkingDirectoryConfined => "WORKING_DIRECTORY_CONFINED",
            Self::StdinClosed => "STDIN_CLOSED",
            Self::TimeLimited => "TIME_LIMITED",
            Self::NetworkDenied => "NETWORK_DENIED",
            Self::FilesystemConfined => "FILESYSTEM_CONFINED",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "enforcement", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Enforcement {
    /// Applied by the backend; `mechanism` names how.
    Enforced { mechanism: String },
    /// Not applied; `reason` says why. Never implied by any other property.
    Unenforced { reason: String },
}

impl Enforcement {
    pub fn is_enforced(&self) -> bool {
        matches!(self, Self::Enforced { .. })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RunOutcome {
    Exited,
    TimedOut,
    /// The request was refused before anything ran (an input missing or not matching its digest).
    Refused,
}

/// Why a request was refused before its program ran.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefusalKind {
    /// `request_defect`: a value that would make the request identity ambiguous, a relative
    /// program path, or a staging directory that cannot be resolved.
    RequestMalformed,
    /// A declared input path leaves the staging root, or resolves outside the source root.
    InputEscapes,
    /// A declared input could not be read as a regular file.
    InputUnreadable,
    /// A declared input does not match its declared digest.
    InputDigestMismatch,
    /// A declared output path leaves the staging root.
    OutputEscapes,
    /// The program is not on the declared `PATH`.
    ProgramNotOnPath,
    /// A declared environment variable the confined class refuses (a name that is not a plain
    /// identifier, or one the dynamic loader or C library of the pre-confinement helpers acts on).
    EnvironmentRefused,
    /// A declared toolchain path or binary the confined class refuses.
    ToolchainPathRefused,
    /// In the confined class, the program -- the path it was found at, or the file that path
    /// resolves to -- does not lie under a declared toolchain path.
    ProgramOutsideToolchain,
    /// The program, or a declared toolchain binary, is found as or resolves to `rustc` or
    /// `cargo` -- whose identity rests on `-vV` -- but is not a toolchain's own binary: `rustc
    /// --print sysroot` (for `cargo`, the `rustc` beside it), run as the run runs it, does not
    /// name a sysroot whose `bin/rustc` is that file. A dispatcher such as the rustup proxy
    /// (linked, hard-linked or copied) answers `-vV` for whichever toolchain an argument
    /// (`+stable`), the environment or a staged `rust-toolchain.toml` selects, not for the one
    /// the run uses. Declare the toolchain's own binary (`<sysroot>/bin/rustc`).
    ToolchainProxy,
    /// The toolchain identity could not be established (unreadable binary, failed `-vV`).
    ToolchainIdentityAbsent,
    /// The toolchain identity differs from the pinned one.
    ToolchainIdentityMismatch,
    /// Confinement was requested but the mechanism is unavailable or its probe failed: nothing
    /// runs unconfined in its place.
    ConfinementUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProducedOutput {
    pub path: String,
    /// `None` when the declared output was not produced.
    pub digest: Option<String>,
}

/// What a backend did for one request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SandboxRun {
    pub schema: String,
    pub backend: String,
    /// Digest of the canonical request: the same request has the same identity.
    pub request_digest: String,
    pub outcome: RunOutcome,
    pub exit_code: Option<i32>,
    pub isolation: BTreeMap<IsolationProperty, Enforcement>,
    pub inputs: Vec<DeclaredInput>,
    pub outputs: Vec<ProducedOutput>,
    pub stdout_digest: String,
    pub stderr_digest: String,
    /// OBSERVED: a record of what this backend actually did on this host.
    pub status: EpistemicStatus,
    /// Why a refused request did not run.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub refusal: String,
    /// The typed class of `refusal` (absent on v1 records and on runs that were not refused).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refusal_kind: Option<RefusalKind>,
    /// The toolchain the run ran with (absent on v1 records, and on refusals before it was known).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toolchain: Option<ToolchainIdentity>,
}

impl SandboxRun {
    /// The properties the run could not enforce, by name.
    pub fn unenforced(&self) -> Vec<&'static str> {
        self.isolation
            .iter()
            .filter(|(_, e)| !e.is_enforced())
            .map(|(p, _)| p.as_str())
            .collect()
    }
}

/// The canonical text of a request, the input to its identity digest: inputs sorted, env sorted.
pub fn canonical_request(request: &SandboxRequest) -> String {
    let mut inputs = request.inputs.clone();
    inputs.sort();
    let mut text = format!(
        "program={}\nargs={}\ntimeout_ms={}\ndeny_network={}\n",
        request.program,
        request.args.join("\u{1f}"),
        request.timeout_ms,
        request.deny_network
    );
    for input in &inputs {
        text.push_str(&format!("input={}:{}\n", input.path, input.digest));
    }
    for (key, value) in &request.env {
        text.push_str(&format!("env={key}={value}\n"));
    }
    let mut outputs = request.outputs.clone();
    outputs.sort();
    for output in outputs {
        text.push_str(&format!("output={output}\n"));
    }
    // G189 fields are appended only when present, so a request without them keeps its v1 identity.
    // Their values are length-prefixed (`<bytes>:<value>`), so no value can forge another line.
    fn field(value: &str) -> String {
        format!("{}:{value}", value.len())
    }
    if let Some(confinement) = &request.confinement {
        let mut toolchain = confinement.toolchain.clone();
        toolchain.sort();
        text.push_str(&format!("confined proc={}\n", confinement.proc));
        for path in toolchain {
            text.push_str(&format!("toolchain_path={}\n", field(&path.path)));
        }
    }
    let mut binaries = request.toolchain_binaries.clone();
    binaries.sort();
    for binary in binaries {
        text.push_str(&format!("toolchain_binary={}\n", field(&binary)));
    }
    if let Some(pin) = &request.pinned_toolchain {
        let binary = |b: &BinaryIdentity| {
            format!(
                "{}{}{}",
                field(&b.path),
                field(&b.resolved),
                field(&b.digest)
            )
        };
        text.push_str(&format!("pinned_program={}\n", binary(&pin.program)));
        for pinned in &pin.binaries {
            text.push_str(&format!("pinned_binary={}\n", binary(pinned)));
        }
        if let Some(version) = &pin.version {
            text.push_str(&format!("pinned_version={}\n", field(version)));
        }
    }
    text
}

/// Why a request cannot be run, before anything is read or staged: a value that would make its
/// canonical text ambiguous (a control character in the program, an argument, a path, an
/// environment name or value; a single empty argument, which joins like none; an `=` in an
/// environment name; an input digest that is not a BLAKE3-256 digest, whose fixed shape ends the
/// `input=` line unambiguously), or a program path that is neither a bare name nor a normalized
/// absolute path (a relative path would resolve against whatever directory the program is
/// started in). On the requests that pass, `canonical_request` is injective up to the order of
/// `inputs`, `outputs`, the toolchain paths and `toolchain_binaries`: two of them share a
/// canonical text only if they differ in nothing but that order, which the canonical form sorts
/// away and which does not change what runs (inputs are staged by path, outputs digested by
/// path, toolchain paths bound and binaries identified in sorted order; the record lists the
/// outputs in request order). Every other v1 request passes, so its digest is unchanged.
pub fn request_defect(request: &SandboxRequest) -> Option<String> {
    let control = |value: &str| value.chars().any(char::is_control);
    if request.program.is_empty() || control(&request.program) {
        return Some("the program is empty or holds a control character".into());
    }
    if request.program.contains('/') && !is_normal_absolute_path(&request.program) {
        return Some(format!(
            "program `{}` is a path that is not absolute and normalized",
            request.program
        ));
    }
    if let Some(i) = request.args.iter().position(|a| control(a)) {
        return Some(format!("argument {i} holds a control character"));
    }
    // `args=` joins the arguments with U+001F: once control characters are refused, the one
    // remaining collision is a single empty argument against none.
    if request.args == [""] {
        return Some("a single empty argument has the identity of no argument".into());
    }
    for input in &request.inputs {
        if control(&input.path) {
            return Some(format!(
                "input path {:?} holds a control character",
                input.path
            ));
        }
        if let Err(error) = crate::IntegrityDigest::parse(&input.digest) {
            return Some(format!("input `{}`: {error}", input.path));
        }
    }
    if let Some(output) = request.outputs.iter().find(|o| control(o)) {
        return Some(format!("output path {output:?} holds a control character"));
    }
    for (name, value) in &request.env {
        if name.is_empty() || name.contains('=') || control(name) {
            return Some(format!(
                "environment name {name:?} is empty, holds `=` or a control character"
            ));
        }
        if control(value) {
            return Some(format!(
                "environment variable `{name}` holds a control character"
            ));
        }
    }
    let toolchain = request
        .confinement
        .iter()
        .flat_map(|c| &c.toolchain)
        .map(|t| &t.path);
    if let Some(path) = toolchain
        .chain(&request.toolchain_binaries)
        .find(|p| control(p))
    {
        return Some(format!("toolchain path {path:?} holds a control character"));
    }
    None
}

/// A declared toolchain path: absolute and normalized (no `.`, `..` or empty part), never `/`.
pub fn is_normal_absolute_path(path: &str) -> bool {
    path.strip_prefix('/').is_some_and(is_confined_path)
}

/// A declared input path must stay inside the staging root: relative, no `..`, no empty part.
pub fn is_confined_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> SandboxRequest {
        SandboxRequest {
            program: "sh".into(),
            args: vec!["-c".into(), "true".into()],
            inputs: vec![
                DeclaredInput {
                    path: "b.txt".into(),
                    digest: "d2".into(),
                },
                DeclaredInput {
                    path: "a.txt".into(),
                    digest: "d1".into(),
                },
            ],
            env: BTreeMap::from([("PATH".into(), "/usr/bin".into())]),
            outputs: vec!["out".into()],
            timeout_ms: 1000,
            deny_network: true,
            ..SandboxRequest::default()
        }
    }

    fn binary(path: &str, digest: &str) -> BinaryIdentity {
        BinaryIdentity {
            path: path.into(),
            resolved: path.into(),
            digest: digest.into(),
        }
    }

    #[test]
    fn a_request_without_g189_fields_keeps_its_v1_identity_and_each_field_changes_it() {
        let v1 = canonical_request(&request());
        assert!(!v1.contains("confined") && !v1.contains("toolchain") && !v1.contains("pinned"));
        let json = serde_json::to_string(&request()).unwrap();
        assert!(!json.contains("confinement") && !json.contains("pinned_toolchain"));
        let mut confined = request();
        confined.confinement = Some(Confinement {
            toolchain: vec![ToolchainPath {
                path: "/usr".into(),
            }],
            proc: false,
        });
        let mut with_proc = confined.clone();
        with_proc.confinement.as_mut().unwrap().proc = true;
        let mut binaries = request();
        binaries.toolchain_binaries = vec!["/usr/bin/cc".into()];
        let mut pinned = request();
        pinned.pinned_toolchain = Some(ToolchainIdentity {
            program: binary("/usr/bin/sh", "d"),
            binaries: Vec::new(),
            version: None,
        });
        let texts = [&confined, &with_proc, &binaries, &pinned].map(canonical_request);
        for (i, text) in texts.iter().enumerate() {
            assert_ne!(text, &v1, "{i}");
            for other in &texts[i + 1..] {
                assert_ne!(text, other);
            }
        }
    }

    #[test]
    fn the_v1_canonical_text_is_unchanged() {
        assert_eq!(
            canonical_request(&request()),
            "program=sh\nargs=-c\u{1f}true\ntimeout_ms=1000\ndeny_network=true\n\
             input=a.txt:d1\ninput=b.txt:d2\nenv=PATH=/usr/bin\noutput=out\n"
        );
    }

    fn valid() -> SandboxRequest {
        let mut request = request();
        for input in &mut request.inputs {
            input.digest = crate::IntegrityDigest::of_bytes(input.path.as_bytes())
                .as_str()
                .to_owned();
        }
        request
    }

    /// G189 review f3: an output holding a newline forged the confinement lines of another
    /// request with the same digest. Such a value is now refused before anything runs, and the
    /// new lines are length-prefixed.
    #[test]
    fn a_value_that_could_forge_a_line_is_refused_and_new_lines_are_length_prefixed() {
        assert_eq!(request_defect(&valid()), None);
        let mut forged = valid();
        forged.outputs = vec!["x\nconfined proc=false\ntoolchain_path=/lib".into()];
        let mut confined = valid();
        confined.outputs = vec!["x".into()];
        confined.confinement = Some(Confinement {
            toolchain: vec![ToolchainPath {
                path: "/lib".into(),
            }],
            proc: false,
        });
        assert!(request_defect(&forged).unwrap().contains("output"));
        assert_eq!(request_defect(&confined), None);
        let defective = |edit: &dyn Fn(&mut SandboxRequest)| {
            let mut request = valid();
            edit(&mut request);
            request_defect(&request)
        };
        assert!(defective(&|r| r.program = "sh\n".into()).is_some());
        assert!(defective(&|r| r.program = "../bin/sh".into()).is_some());
        assert!(defective(&|r| r.program = "bin/sh".into()).is_some());
        assert!(defective(&|r| r.program = "/usr/bin/../bin/sh".into()).is_some());
        assert!(defective(&|r| r.program = "/usr/bin/sh".into()).is_none());
        assert!(defective(&|r| r.args.push("a\u{1f}b".into())).is_some());
        assert!(defective(&|r| r.args.push("a\nb".into())).is_some());
        // G189 round 3: `[]` and `[""]` both joined to `args=`.
        let mut none = valid();
        none.args.clear();
        let mut empty = valid();
        empty.args = vec![String::new()];
        assert_eq!(canonical_request(&none), canonical_request(&empty));
        assert_eq!(request_defect(&none), None);
        assert!(request_defect(&empty).unwrap().contains("empty argument"));
        assert!(defective(&|r| r.args = vec![String::new(), String::new()]).is_none());
        assert!(defective(&|r| r.args = vec!["a".into(), String::new()]).is_none());
        assert!(defective(&|r| r.inputs[0].path = "a\tb".into()).is_some());
        assert!(defective(&|r| r.inputs[0].digest = "d1".into()).is_some());
        assert!(
            defective(&|r| {
                r.env.insert("A=B".into(), "x".into());
            })
            .is_some()
        );
        assert!(
            defective(&|r| {
                r.env.insert("A".into(), "x\ny".into());
            })
            .is_some()
        );
        assert!(defective(&|r| r.toolchain_binaries.push("/usr/bin/cc\n".into())).is_some());
        let mut toolchain_newline = confined.clone();
        toolchain_newline.confinement.as_mut().unwrap().toolchain[0].path = "/lib\n".into();
        assert!(request_defect(&toolchain_newline).is_some());

        // A pinned path holding `:` cannot be confused with another split of the same bytes.
        let pin = |path: &str, resolved: &str| {
            let mut request = valid();
            request.pinned_toolchain = Some(ToolchainIdentity {
                program: BinaryIdentity {
                    path: path.into(),
                    resolved: resolved.into(),
                    digest: "d".into(),
                },
                binaries: Vec::new(),
                version: None,
            });
            canonical_request(&request)
        };
        assert_ne!(pin("/a:b", "/c"), pin("/a", "b:/c"));
        assert!(pin("/a:b", "/c").contains("pinned_program=4:/a:b2:/c1:d\n"));
    }

    #[test]
    fn a_v1_record_still_reads_without_toolchain_or_typed_refusal() {
        let v1 = r#"{"schema":"atlas.sandbox-run.v1","backend":"atlas.sandbox.restricted-subprocess.v1",
            "request_digest":"d","outcome":"REFUSED","exit_code":null,"isolation":{},"inputs":[],
            "outputs":[],"stdout_digest":"o","stderr_digest":"e","status":"OBSERVED",
            "refusal":"input `a` does not match its declared digest"}"#;
        let run: SandboxRun = serde_json::from_str(v1).unwrap();
        assert_eq!(run.schema, SANDBOX_RUN_SCHEMA_V1);
        assert_eq!(run.outcome, RunOutcome::Refused);
        assert!(run.toolchain.is_none() && run.refusal_kind.is_none());
        let written = serde_json::to_value(&run).unwrap();
        assert!(written.get("toolchain").is_none() && written.get("refusal_kind").is_none());
    }

    #[test]
    fn a_pinned_identity_names_the_first_difference() {
        let pinned = ToolchainIdentity {
            program: binary("/r/bin/rustc", "d1"),
            binaries: vec![binary("/usr/bin/cc", "d2")],
            version: Some("rustc 1.90.0".into()),
        };
        assert_eq!(pinned.mismatch(&pinned.clone()), None);
        let mut other = pinned.clone();
        other.program.digest = "x".into();
        assert!(
            pinned
                .mismatch(&other)
                .unwrap()
                .starts_with("program digest")
        );
        let mut other = pinned.clone();
        other.program.resolved = "/elsewhere".into();
        assert!(
            pinned
                .mismatch(&other)
                .unwrap()
                .starts_with("program resolved")
        );
        let mut other = pinned.clone();
        other.binaries[0].digest = "x".into();
        assert!(
            pinned
                .mismatch(&other)
                .unwrap()
                .starts_with("binary digest")
        );
        let mut other = pinned.clone();
        other.binaries.clear();
        assert!(pinned.mismatch(&other).unwrap().starts_with("binaries"));
        let mut other = pinned.clone();
        other.version = Some("rustc 1.91.0".into());
        assert!(pinned.mismatch(&other).unwrap().starts_with("version"));
    }

    #[test]
    fn toolchain_paths_are_absolute_and_normalized() {
        for good in ["/usr", "/usr/lib", "/a/b.c"] {
            assert!(is_normal_absolute_path(good), "{good}");
        }
        for bad in [
            "/",
            "",
            "usr",
            "/usr/",
            "//usr",
            "/usr/../etc",
            "/./usr",
            "/a\\b",
        ] {
            assert!(!is_normal_absolute_path(bad), "{bad}");
        }
    }

    #[test]
    fn request_identity_is_independent_of_declaration_order() {
        let mut reordered = request();
        reordered.inputs.reverse();
        assert_eq!(canonical_request(&request()), canonical_request(&reordered));
        let mut other = request();
        other.inputs[0].digest = "changed".into();
        assert_ne!(canonical_request(&request()), canonical_request(&other));
    }

    #[test]
    fn declared_paths_never_escape_the_staging_root() {
        for good in ["a.txt", "src/lib.rs", "a/b/c"] {
            assert!(is_confined_path(good), "{good}");
        }
        for bad in [
            "",
            "/etc/passwd",
            "../x",
            "a/../../x",
            "a//b",
            "./a",
            "a\\b",
        ] {
            assert!(!is_confined_path(bad), "{bad}");
        }
    }
}
