//! Sandboxed execution records (G126, NA-SANDBOX-SUBPROCESS, construction node M16).
//!
//! A `SandboxRequest` declares everything a run may use: its command, its input files with their
//! content digests, the environment variables it may see, and a timeout. A backend stages exactly
//! those inputs, runs the command, and returns a `SandboxRun` recording what was used and what was
//! produced, together with every isolation property it claims -- each `ENFORCED` (the backend
//! applied a mechanism and, where possible, probed it) or `UNENFORCED` with the reason. A property
//! a backend cannot enforce is never implied: the restricted-subprocess class does not confine
//! absolute-path filesystem reads, and says so.
//!
//! Pure vocabulary; the backends live in `runtime::sandbox`. See ADR 0047.

use crate::EpistemicStatus;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SANDBOX_RUN_SCHEMA: &str = "atlas.sandbox-run.v1";

/// One declared input: a path relative to the staging root and its content digest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclaredInput {
    pub path: String,
    pub digest: String,
}

/// Everything a sandboxed run may use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    text
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
