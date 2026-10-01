//! The restricted-subprocess sandbox backend (G126, NA-SANDBOX-SUBPROCESS, ADR 0047).
//!
//! It stages exactly the declared inputs -- read from a source root and verified against their
//! declared digests -- into a fresh directory, runs the declared program there with only the
//! declared environment, closed stdin and a timeout, optionally inside an unprivileged network
//! namespace, and records a `SandboxRun`. It enforces what this class can enforce and names what it
//! cannot: absolute-path and `..` filesystem reads are not confined (Atlas carries no `unsafe`
//! code, so no Landlock or mount namespace is set up here).

use atlas_core::IntegrityDigest;
use atlas_core::sandbox::{
    DeclaredInput, Enforcement, IsolationProperty, ProducedOutput, RunOutcome, SANDBOX_RUN_SCHEMA,
    SandboxRequest, SandboxRun, canonical_request, is_confined_path,
};
use std::collections::BTreeMap;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const RESTRICTED_SUBPROCESS: &str = "atlas.sandbox.restricted-subprocess.v1";

/// A run and what it printed (the record keeps only digests).
pub struct Execution {
    pub run: SandboxRun,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

fn digest(bytes: &[u8]) -> String {
    IntegrityDigest::of_bytes(bytes).as_str().to_owned()
}

/// The content digest a `DeclaredInput` carries for `bytes`.
pub fn input_digest(bytes: &[u8]) -> String {
    digest(bytes)
}

/// `name` on the colon-separated `path` list, as an absolute path.
fn find_on_path(name: &str, path: &str) -> Option<PathBuf> {
    if name.contains('/') {
        let candidate = PathBuf::from(name);
        return candidate.is_file().then_some(candidate);
    }
    path.split(':')
        .filter(|dir| dir.starts_with('/'))
        .map(|dir| Path::new(dir).join(name))
        .find(|candidate| candidate.is_file())
}

/// Whether an unprivileged network namespace can be entered on this host (probed, not assumed).
pub fn network_isolation_available() -> bool {
    let Some(unshare) = std::env::var("PATH")
        .ok()
        .and_then(|path| find_on_path("unshare", &path))
    else {
        return false;
    };
    Command::new(unshare)
        .args(["--user", "--map-root-user", "--net", "true"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn refused(request: &SandboxRequest, reason: String) -> Execution {
    Execution {
        run: SandboxRun {
            schema: SANDBOX_RUN_SCHEMA.into(),
            backend: RESTRICTED_SUBPROCESS.into(),
            request_digest: digest(canonical_request(request).as_bytes()),
            outcome: RunOutcome::Refused,
            exit_code: None,
            isolation: BTreeMap::new(),
            inputs: request.inputs.clone(),
            outputs: Vec::new(),
            stdout_digest: digest(b""),
            stderr_digest: digest(b""),
            status: atlas_core::EpistemicStatus::Observed,
            refusal: reason,
        },
        stdout: Vec::new(),
        stderr: Vec::new(),
    }
}

fn drain(mut pipe: impl Read + Send + 'static) -> std::sync::mpsc::Receiver<Vec<u8>> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = pipe.read_to_end(&mut bytes);
        let _ = sender.send(bytes);
    });
    receiver
}

/// Kill the run's whole process group (`pgid` = the child's pid), not only the child: a
/// descendant still in the group would otherwise outlive the timeout and hold the output pipes.
fn kill_group(pgid: u32) {
    let path = std::env::var("PATH").unwrap_or_default();
    if let Some(kill) = find_on_path("kill", &path) {
        let _ = Command::new(kill)
            .args(["-KILL", "--", &format!("-{pgid}")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

/// Run `request` with its inputs read from `source_root`, staging under `staging_parent`.
pub fn run(
    request: &SandboxRequest,
    source_root: &Path,
    staging_parent: &Path,
) -> io::Result<Execution> {
    let request_digest = digest(canonical_request(request).as_bytes());
    // Every declared input is confined, present and exactly what was declared -- or nothing runs.
    let mut staged: Vec<(String, Vec<u8>)> = Vec::new();
    for input in &request.inputs {
        if !is_confined_path(&input.path) {
            return Ok(refused(
                request,
                format!("input `{}` leaves the staging root", input.path),
            ));
        }
        let bytes = match std::fs::read(source_root.join(&input.path)) {
            Ok(bytes) => bytes,
            Err(error) => {
                return Ok(refused(request, format!("input `{}`: {error}", input.path)));
            }
        };
        if digest(&bytes) != input.digest {
            return Ok(refused(
                request,
                format!("input `{}` does not match its declared digest", input.path),
            ));
        }
        staged.push((input.path.clone(), bytes));
    }
    if let Some(output) = request.outputs.iter().find(|o| !is_confined_path(o)) {
        return Ok(refused(
            request,
            format!("output `{output}` leaves the staging root"),
        ));
    }
    let declared_path = request.env.get("PATH").cloned().unwrap_or_default();
    let Some(program) = find_on_path(&request.program, &declared_path) else {
        return Ok(refused(
            request,
            format!("`{}` is not on the declared PATH", request.program),
        ));
    };

    let stage = staging_parent.join(format!(
        "atlas-sandbox-{}-{}",
        &request_digest[IntegrityDigest::BLAKE3_256_PREFIX.len()..][..16],
        std::process::id()
    ));
    if stage.exists() {
        std::fs::remove_dir_all(&stage)?;
    }
    std::fs::create_dir_all(&stage)?;
    for (path, bytes) in &staged {
        let target = stage.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(target, bytes)?;
    }

    let network = if request.deny_network {
        if network_isolation_available() {
            Some(find_on_path(
                "unshare",
                &std::env::var("PATH").unwrap_or_default(),
            ))
        } else {
            None
        }
    } else {
        None
    };
    let mut command = match network.flatten() {
        Some(unshare) => {
            let mut command = Command::new(unshare);
            command
                .args(["--user", "--map-root-user", "--net", "--"])
                .arg(&program)
                .args(&request.args);
            command
        }
        None => {
            let mut command = Command::new(&program);
            command.args(&request.args);
            command
        }
    };
    let network_enforced = request.deny_network && command.get_program() != program.as_os_str();
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    command
        .current_dir(&stage)
        .env_clear()
        .envs(&request.env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let stdout = drain(child.stdout.take().expect("piped"));
    let stderr = drain(child.stderr.take().expect("piped"));
    let started = Instant::now();
    let timeout = Duration::from_millis(request.timeout_ms);
    let (outcome, exit_code) = loop {
        if let Some(status) = child.try_wait()? {
            break (RunOutcome::Exited, status.code());
        }
        if started.elapsed() >= timeout {
            kill_group(child.id());
            let _ = child.kill();
            let _ = child.wait();
            break (RunOutcome::TimedOut, None);
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    // A descendant that left the process group may still hold a pipe: output is bounded in time.
    let grace = Duration::from_secs(2);
    let stdout = stdout.recv_timeout(grace).unwrap_or_default();
    let stderr = stderr.recv_timeout(grace).unwrap_or_default();
    let outputs = request
        .outputs
        .iter()
        .map(|path| ProducedOutput {
            path: path.clone(),
            digest: std::fs::read(stage.join(path)).ok().map(|b| digest(&b)),
        })
        .collect();
    std::fs::remove_dir_all(&stage)?;

    let enforced = |mechanism: &str| Enforcement::Enforced {
        mechanism: mechanism.into(),
    };
    let unenforced = |reason: &str| Enforcement::Unenforced {
        reason: reason.into(),
    };
    let isolation = BTreeMap::from([
        (
            IsolationProperty::EnvironmentCleared,
            enforced("env_clear, then only the declared environment"),
        ),
        (
            IsolationProperty::InputsStaged,
            enforced("digest-verified copies of the declared inputs in a fresh directory"),
        ),
        (
            IsolationProperty::WorkingDirectoryConfined,
            enforced("the working directory is the staging directory"),
        ),
        (
            IsolationProperty::StdinClosed,
            enforced("stdin is /dev/null"),
        ),
        (
            IsolationProperty::TimeLimited,
            enforced(
                "the run's process group is killed at the declared timeout (a descendant that \
                 leaves the group is not)",
            ),
        ),
        (
            IsolationProperty::NetworkDenied,
            if network_enforced {
                enforced(
                    "unshare --user --map-root-user --net: a network namespace with loopback only",
                )
            } else if request.deny_network {
                unenforced("unprivileged user namespaces are unavailable on this host")
            } else {
                unenforced("the request does not deny the network")
            },
        ),
        (
            IsolationProperty::FilesystemConfined,
            unenforced(
                "absolute-path and `..` reads are not confined: the restricted-subprocess class \
                 sets up no filesystem namespace or Landlock ruleset",
            ),
        ),
    ]);
    Ok(Execution {
        run: SandboxRun {
            schema: SANDBOX_RUN_SCHEMA.into(),
            backend: RESTRICTED_SUBPROCESS.into(),
            request_digest,
            outcome,
            exit_code,
            isolation,
            inputs: request.inputs.clone(),
            outputs,
            stdout_digest: digest(&stdout),
            stderr_digest: digest(&stderr),
            status: atlas_core::EpistemicStatus::Observed,
            refusal: String::new(),
        },
        stdout,
        stderr,
    })
}

/// A request declaring every file under `root` listed in `paths`, with their current digests.
pub fn declare_inputs(root: &Path, paths: &[&str]) -> io::Result<Vec<DeclaredInput>> {
    paths
        .iter()
        .map(|path| {
            Ok(DeclaredInput {
                path: (*path).to_owned(),
                digest: digest(&std::fs::read(root.join(path))?),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::sandbox::IsolationProperty as P;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("atlas-sandbox-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join("stage")).unwrap();
        std::fs::write(dir.join("src/a.txt"), "declared\n").unwrap();
        std::fs::write(dir.join("src/secret.txt"), "undeclared\n").unwrap();
        dir
    }

    fn request(root: &Path, script: &str) -> SandboxRequest {
        SandboxRequest {
            program: "sh".into(),
            args: vec!["-c".into(), script.into()],
            inputs: declare_inputs(&root.join("src"), &["a.txt"]).unwrap(),
            env: BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]),
            outputs: vec!["out.txt".into()],
            timeout_ms: 20_000,
            deny_network: true,
        }
    }

    fn execute(root: &Path, request: &SandboxRequest) -> Execution {
        run(request, &root.join("src"), &root.join("stage")).unwrap()
    }

    #[test]
    fn only_declared_inputs_and_environment_reach_the_run() {
        let root = scratch("inputs");
        let run = execute(
            &root,
            &request(
                &root,
                "cat a.txt; cat secret.txt; echo ---; env; echo made > out.txt",
            ),
        );
        let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
        assert_eq!(run.run.outcome, RunOutcome::Exited);
        assert!(stdout.starts_with("declared\n"), "{stdout}");
        assert!(
            !stdout.contains("undeclared"),
            "an undeclared sibling is not staged"
        );
        let env = stdout.split("---\n").nth(1).unwrap();
        assert!(
            !env.contains("HOME="),
            "the parent environment is cleared: {env}"
        );
        assert!(env.contains("PATH=/usr/bin:/bin"));
        assert_eq!(
            run.run.outputs[0].digest.as_deref(),
            Some(digest(b"made\n").as_str())
        );
        assert!(run.run.isolation[&P::EnvironmentCleared].is_enforced());
        assert!(run.run.isolation[&P::InputsStaged].is_enforced());
        // Never claimed: the restricted-subprocess class does not confine absolute paths.
        assert!(!run.run.isolation[&P::FilesystemConfined].is_enforced());
        assert!(run.run.unenforced().contains(&"FILESYSTEM_CONFINED"));
        assert!(
            !root.join("stage").read_dir().unwrap().any(|_| true),
            "the stage is removed"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_tampered_or_escaping_input_is_refused_before_anything_runs() {
        let root = scratch("refused");
        let mut tampered = request(&root, "echo ran > out.txt");
        tampered.inputs[0].digest = digest(b"something else");
        let run = execute(&root, &tampered);
        assert_eq!(run.run.outcome, RunOutcome::Refused);
        assert!(run.run.refusal.contains("declared digest"));
        assert!(run.run.isolation.is_empty() && run.run.outputs.is_empty());
        let mut escaping = request(&root, "true");
        escaping.inputs.push(DeclaredInput {
            path: "../secret.txt".into(),
            digest: digest(b"undeclared\n"),
        });
        assert_eq!(execute(&root, &escaping).run.outcome, RunOutcome::Refused);
        let mut hidden = request(&root, "true");
        hidden.env.insert("PATH".into(), "/nonexistent".into());
        assert!(
            execute(&root, &hidden)
                .run
                .refusal
                .contains("declared PATH")
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_run_is_killed_at_its_timeout_with_its_descendants() {
        let root = scratch("timeout");
        let marker = root.join("survived");
        let mut slow = request(&root, "(sleep 1; echo alive > \"$MARK\") & sleep 5");
        slow.env
            .insert("MARK".into(), marker.to_string_lossy().into_owned());
        slow.timeout_ms = 300;
        let started = Instant::now();
        let run = execute(&root, &slow);
        assert_eq!(run.run.outcome, RunOutcome::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(4));
        std::thread::sleep(Duration::from_millis(1500));
        assert!(!marker.exists(), "a descendant outlived the timeout");
        assert_eq!(
            run.run.request_digest,
            execute(&root, &slow).run.request_digest
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Network denial is claimed only when a network namespace was entered, and then the run
    /// sees loopback alone.
    #[test]
    fn network_denial_is_claimed_only_when_probed_and_real() {
        let root = scratch("network");
        let run = execute(&root, &request(&root, "cat /proc/net/dev"));
        let devices: Vec<String> = String::from_utf8_lossy(&run.stdout)
            .lines()
            .skip(2)
            .filter_map(|l| l.split(':').next().map(|d| d.trim().to_owned()))
            .collect();
        match &run.run.isolation[&P::NetworkDenied] {
            Enforcement::Enforced { .. } => assert_eq!(devices, ["lo"], "{devices:?}"),
            Enforcement::Unenforced { reason } => {
                assert!(!network_isolation_available(), "{reason}")
            }
        }
        std::fs::remove_dir_all(&root).unwrap();
    }
}
