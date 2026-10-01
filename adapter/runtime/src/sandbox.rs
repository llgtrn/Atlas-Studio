//! The sandbox backends (G126, NA-SANDBOX-SUBPROCESS, ADR 0047; G189, NA-SANDBOX-FS-CONFINEMENT,
//! ADR 0102).
//!
//! Both classes stage exactly the declared inputs -- read from a source root, never through a
//! link that leaves it, and verified against their declared digests -- into a fresh directory, run
//! the declared program there with only the declared environment, closed stdin and a timeout, and
//! record a `SandboxRun` with the identity of the toolchain the run used.
//!
//! - The restricted-subprocess class runs the program on the host filesystem, optionally inside an
//!   unprivileged network namespace. It does not confine absolute-path or `..` reads, and says so.
//! - The confined class (`SandboxRequest::confinement`) runs it through util-linux `unshare` in a
//!   fresh user, mount and pid namespace whose root is a private tmpfs holding only `/work` (the
//!   stage), `/dev/null`, the declared read-only toolchain paths and, when asked, a procfs of its
//!   own pid namespace; the program is chrooted there as a non-root user of a nested user
//!   namespace, with no capability. Filesystem confinement is claimed only when a probe run in the
//!   same confinement found a per-run host sentinel absent and a staged control file present. When
//!   the mechanism is unavailable or the probe fails, nothing runs.
//!
//! Atlas carries no `unsafe` code: every namespace, mount and chroot is made by an external
//! util-linux binary, driven by a constant shell script whose values are positional parameters.

use atlas_core::IntegrityDigest;
use atlas_core::sandbox::{
    BinaryIdentity, Confinement, DeclaredInput, Enforcement, IsolationProperty, ProducedOutput,
    RefusalKind, RunOutcome, SANDBOX_RUN_SCHEMA, SandboxRequest, SandboxRun, ToolchainIdentity,
    canonical_request, is_confined_path, is_normal_absolute_path, request_defect,
};
pub use atlas_core::sandbox::{RunOutcome as SandboxOutcome, SandboxRequest as Request};
use std::collections::BTreeMap;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime};

pub const RESTRICTED_SUBPROCESS: &str = "atlas.sandbox.restricted-subprocess.v1";
pub const CONFINED_SUBPROCESS: &str = "atlas.sandbox.confined-subprocess.v1";

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

/// The host binaries the confined class runs before the program. A field left `None`, or naming
/// a missing file, makes confinement unavailable. Passed explicitly so a test can point one at a
/// missing binary without touching global state.
#[derive(Debug, Clone)]
pub struct HostTools {
    pub unshare: Option<PathBuf>,
    pub sh: Option<PathBuf>,
    pub mount: Option<PathBuf>,
    pub mkdir: Option<PathBuf>,
    pub ln: Option<PathBuf>,
}

impl HostTools {
    /// The tools on the host `PATH`, then the standard system directories.
    pub fn from_host() -> Self {
        let path = format!(
            "{}:/usr/sbin:/usr/bin:/sbin:/bin",
            std::env::var("PATH").unwrap_or_default()
        );
        let find = |name: &str| find_on_path(name, &path);
        Self {
            unshare: find("unshare"),
            sh: find("sh"),
            mount: find("mount"),
            mkdir: find("mkdir"),
            ln: find("ln"),
        }
    }

    fn resolve(&self) -> Result<Tools, String> {
        let tool = |name: &str, path: &Option<PathBuf>| match path {
            Some(path) if path.is_file() => Ok(path.clone()),
            Some(path) => Err(format!(
                "the confinement mechanism is unavailable: `{name}` was not found at {}",
                path.display()
            )),
            None => Err(format!(
                "the confinement mechanism is unavailable: `{name}` is not on the host PATH"
            )),
        };
        Ok(Tools {
            unshare: tool("unshare", &self.unshare)?,
            sh: tool("sh", &self.sh)?,
            mount: tool("mount", &self.mount)?,
            mkdir: tool("mkdir", &self.mkdir)?,
            ln: tool("ln", &self.ln)?,
        })
    }
}

struct Tools {
    unshare: PathBuf,
    sh: PathBuf,
    mount: PathBuf,
    mkdir: PathBuf,
    ln: PathBuf,
}

/// Runs inside the outer namespaces as their (namespaced) root, before anything of the request.
/// Values arrive only as positional parameters: `new work mount mkdir ln proc n`, then `n`
/// triples `kind path target` (`d` directory and `f` file, bound read-only, `target` empty; `l` a
/// link, created in the root with the host link's own `target` text, so it resolves inside the
/// root exactly as on the host), then the declared environment as `NAME=value` words,
/// then `--`, then the final command line, built by Atlas: the nested `unshare` that chroots into
/// `new` as uid 1000 of a fresh user namespace and executes the program. No helper variable is
/// read after the declared environment is exported, so a declared name cannot redirect it.
const CONFINE_SCRIPT: &str = r#"set -eu
new=$1 work=$2 mount=$3 mkdir=$4 ln=$5 proc=$6 n=$7
shift 7
"$mount" -t tmpfs -o mode=0755,size=1m,nosuid,nodev atlas-sandbox-root "$new"
"$mkdir" "$new/work" "$new/dev"
"$mount" --bind -o nosuid,nodev "$work" "$new/work"
: > "$new/dev/null"
"$mount" --bind /dev/null "$new/dev/null"
while [ "$n" -gt 0 ]; do
  kind=$1 path=$2 target=$3
  shift 3
  n=$((n - 1))
  if [ "$kind" = l ]; then
    "$mkdir" -p "$new${path%/*}"
    "$ln" -s -- "$target" "$new$path"
    continue
  fi
  if [ "$kind" = d ]; then
    "$mkdir" -p "$new$path"
  else
    "$mkdir" -p "$new${path%/*}"
    [ -e "$new$path" ] || : > "$new$path"
  fi
  "$mount" --bind -o ro,nosuid,nodev "$path" "$new$path"
done
if [ "$proc" = 1 ]; then
  "$mkdir" "$new/proc"
  "$mount" -t proc -o subset=pid,nosuid,nodev,noexec atlas-sandbox-proc "$new/proc"
fi
"$mount" -o remount,ro,nosuid,nodev "$new"
unset PWD
while [ "$1" != -- ]; do
  export "$1"
  shift
done
shift
exec "$@"
"#;

/// Environment names the confined class refuses: they would act on the dynamic loader or C
/// library of the helpers that run before the chroot (the nested `unshare` receives the declared
/// environment so that the program does).
fn refused_env_name(name: &str) -> Option<&'static str> {
    let mut chars = name.chars();
    let plain = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !plain {
        return Some("is not a plain identifier");
    }
    if name.starts_with("LD_")
        || ["GCONV_PATH", "GLIBC_TUNABLES", "LOCPATH", "NLSPATH"].contains(&name)
    {
        return Some("acts on the dynamic loader or C library of the pre-confinement helpers");
    }
    None
}

/// The paths inside a confined root that a toolchain path may not cover.
const RESERVED: [&str; 3] = ["/work", "/dev", "/proc"];

/// Host directories that hold sockets and other live endpoints: a read-only bind does not stop
/// `connect()` on a socket, so a toolchain path may be none of them, inside none of them, and
/// contain none of them.
const RUNTIME_DIRECTORIES: [&str; 5] = ["/run", "/var/run", "/tmp", "/var/tmp", "/dev/shm"];

fn overlaps(a: &Path, b: &Path) -> bool {
    a.starts_with(b) || b.starts_with(a)
}

/// How a declared toolchain path appears in the root.
#[derive(Debug, Clone, PartialEq, Eq)]
enum BindKind {
    /// A directory, bound read-only at its own (canonical) path.
    Directory,
    /// A regular file, bound read-only at its own (canonical) path.
    File,
    /// A link: created in the root with the host link's text, never bound with its target's
    /// content, so paths beneath it resolve inside the root as they do on the host.
    Link(String),
}

/// A declared toolchain path and how it appears in the root.
#[derive(Debug, Clone)]
struct Bind {
    kind: BindKind,
    path: String,
}

/// Checks the declared toolchain paths; returns them as binds, sorted.
/// `staging` is the canonical staging directory.
fn toolchain_binds(confinement: &Confinement, staging: &Path) -> Result<Vec<Bind>, String> {
    let mut binds = Vec::new();
    for declared in &confinement.toolchain {
        let path = declared.path.as_str();
        if !is_normal_absolute_path(path) {
            return Err(format!(
                "toolchain path `{path}` is not absolute and normalized"
            ));
        }
        if let Some(reserved) = RESERVED
            .iter()
            .find(|r| overlaps(Path::new(path), Path::new(r)))
        {
            return Err(format!(
                "toolchain path `{path}` covers `{reserved}`, which the confinement provides"
            ));
        }
        let metadata =
            std::fs::metadata(path).map_err(|e| format!("toolchain path `{path}`: {e}"))?;
        if !metadata.is_dir() && !metadata.is_file() {
            return Err(format!(
                "toolchain path `{path}` is neither a directory nor a regular file"
            ));
        }
        let canonical = Path::new(path)
            .canonicalize()
            .map_err(|e| format!("toolchain path `{path}`: {e}"))?;
        // A path is bound at its own path, so it must be canonical; only its last component may
        // be a link, which the root then holds as that same link (its target declared below).
        let link = std::fs::symlink_metadata(path)
            .map_err(|e| format!("toolchain path `{path}`: {e}"))?
            .file_type()
            .is_symlink();
        let parent = Path::new(path).parent().unwrap_or(Path::new("/"));
        let parent_canonical = parent
            .canonicalize()
            .map_err(|e| format!("toolchain path `{path}`: {e}"))?;
        if parent_canonical != parent || (!link && canonical != Path::new(path)) {
            return Err(format!(
                "toolchain path `{path}` passes through a link (it resolves to `{}`): declare \
                 the resolved path",
                canonical.display()
            ));
        }
        let kind = if link {
            let text = std::fs::read_link(path)
                .map_err(|e| format!("toolchain path `{path}`: {e}"))?
                .to_string_lossy()
                .into_owned();
            if text.is_empty() || text.chars().any(char::is_control) {
                return Err(format!("toolchain link `{path}` has an unusable target"));
            }
            BindKind::Link(text)
        } else if metadata.is_dir() {
            BindKind::Directory
        } else {
            BindKind::File
        };
        if overlaps(staging, &canonical) {
            return Err(format!(
                "toolchain path `{path}` overlaps the staging directory {}",
                staging.display()
            ));
        }
        if let Some(runtime) = RUNTIME_DIRECTORIES
            .iter()
            .find(|r| overlaps(Path::new(path), Path::new(r)) || overlaps(&canonical, Path::new(r)))
        {
            return Err(format!(
                "toolchain path `{path}` overlaps `{runtime}`, a directory of sockets and other \
                 live endpoints a read-only bind does not close"
            ));
        }
        binds.push((
            Bind {
                kind,
                path: path.to_owned(),
            },
            canonical,
        ));
    }
    binds.sort_by(|a, b| a.0.path.cmp(&b.0.path));
    for (i, (outer, _)) in binds.iter().enumerate() {
        if let Some((inner, _)) = binds[i + 1..]
            .iter()
            .find(|(other, _)| Path::new(&other.path).starts_with(&outer.path))
        {
            return Err(format!(
                "toolchain path `{}` lies inside the declared toolchain path `{}`",
                inner.path, outer.path
            ));
        }
    }
    // A declared link's target must itself be declared (under a bound path), or the link would
    // dangle -- or reach something undeclared -- inside the root.
    for (bind, canonical) in &binds {
        if matches!(bind.kind, BindKind::Link(_))
            && !binds.iter().any(|(other, _)| {
                !matches!(other.kind, BindKind::Link(_)) && canonical.starts_with(&other.path)
            })
        {
            return Err(format!(
                "toolchain link `{}` resolves to `{}`, which is not under a declared toolchain \
                 path: declare it",
                bind.path,
                canonical.display()
            ));
        }
    }
    Ok(binds.into_iter().map(|(bind, _)| bind).collect())
}

/// Whether `path` lies under one of the declared toolchain paths.
fn under_toolchain(path: &str, binds: &[Bind]) -> bool {
    is_normal_absolute_path(path) && binds.iter().any(|b| Path::new(path).starts_with(&b.path))
}

/// A fresh per-run directory: `work` (the stage), `root` (the confined root's mount point, empty
/// on the host) and `sentinel` (a host file outside the stage the probe must not reach). Removed
/// when dropped.
struct RunDir {
    dir: PathBuf,
    work: PathBuf,
    root: PathBuf,
    sentinel: PathBuf,
}

impl RunDir {
    fn create(staging_parent: &Path, request_digest: &str) -> io::Result<Self> {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let dir = staging_parent.join(format!(
            "atlas-sandbox-{}-{}-{}",
            &request_digest[IntegrityDigest::BLAKE3_256_PREFIX.len()..][..16],
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        std::fs::create_dir_all(dir.join("work"))?;
        std::fs::create_dir(dir.join("root"))?;
        Ok(Self {
            work: dir.join("work"),
            root: dir.join("root"),
            sentinel: dir.join("sentinel"),
            dir,
        })
    }

    /// Empties the stage (after the probe and the identity run, before the inputs are staged).
    fn reset_work(&self) -> io::Result<()> {
        std::fs::remove_dir_all(&self.work)?;
        std::fs::create_dir(&self.work)
    }
}

impl Drop for RunDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// How a request is executed: its class and, for the confined class, everything the helper needs.
enum Class<'a> {
    Restricted {
        /// The host `unshare`, when network denial was asked and is available.
        network: Option<PathBuf>,
    },
    Confined {
        tools: Tools,
        binds: Vec<Bind>,
        proc: bool,
        deny_network: bool,
        run: &'a RunDir,
    },
}

/// The backend a request runs in, by its class.
fn backend(request: &SandboxRequest) -> &'static str {
    if request.confinement.is_some() {
        CONFINED_SUBPROCESS
    } else {
        RESTRICTED_SUBPROCESS
    }
}

/// A program resolved once: the absolute path found on the declared `PATH` (or given as a
/// normalized absolute path) and that path with every link resolved. Every class executes
/// `found`, so `argv[0]` is the name the program was asked for (a multi-call binary such as the
/// rustup proxy runs as the tool it was asked as); the identity digests `resolved`, the file the
/// kernel reaches through `found`. They can differ only if a link on the path is swapped between
/// resolution and exec (RES-G189-HOST-PATH-TOCTOU). `found` is absolute, so no working
/// directory changes what it names.
struct Program {
    found: PathBuf,
    resolved: PathBuf,
}

impl Class<'_> {
    /// The command that executes `program` by the absolute path it was found at (its `argv[0]`)
    /// with `args` and exactly `env` in the stage `work`.
    fn command(
        &self,
        work: &Path,
        env: &BTreeMap<String, String>,
        program: &Program,
        args: &[String],
    ) -> Command {
        match self {
            Self::Restricted { network } => {
                let mut command = match network {
                    Some(unshare) => {
                        let mut command = Command::new(unshare);
                        command
                            .args(["--user", "--map-root-user", "--net", "--"])
                            .arg(&program.found)
                            .args(args);
                        command
                    }
                    None => {
                        let mut command = Command::new(&program.found);
                        command.args(args);
                        command
                    }
                };
                command.current_dir(work).env_clear().envs(env);
                command
            }
            Self::Confined {
                tools,
                binds,
                proc,
                deny_network,
                run,
            } => {
                let mut command = Command::new(&tools.unshare);
                command.args([
                    "--user",
                    "--map-root-user",
                    "--mount",
                    "--pid",
                    "--fork",
                    "--kill-child",
                    "--propagation",
                    "private",
                ]);
                if *deny_network {
                    command.arg("--net");
                }
                command
                    .arg("--")
                    .arg(&tools.sh)
                    .args(["-c", CONFINE_SCRIPT, "atlas-sandbox-confine"])
                    .arg(&run.root)
                    .arg(work)
                    .arg(&tools.mount)
                    .arg(&tools.mkdir)
                    .arg(&tools.ln)
                    .arg(if *proc { "1" } else { "0" })
                    .arg(binds.len().to_string());
                for bind in binds {
                    let (kind, target) = match &bind.kind {
                        BindKind::Directory => ("d", ""),
                        BindKind::File => ("f", ""),
                        BindKind::Link(target) => ("l", target.as_str()),
                    };
                    command.arg(kind).arg(&bind.path).arg(target);
                }
                for (name, value) in env {
                    command.arg(format!("{name}={value}"));
                }
                let mut root_arg = std::ffi::OsString::from("--root=");
                root_arg.push(&run.root);
                command
                    .arg("--")
                    .arg(&tools.unshare)
                    .args(["--user", "--map-user=1000", "--map-group=1000"])
                    .arg(root_arg)
                    .args(["--wd=/work", "--"])
                    .arg(&program.found)
                    .args(args);
                // The helpers see no environment and no host working directory.
                command.current_dir("/").env_clear();
                command
            }
        }
    }
}

struct Finished {
    outcome: RunOutcome,
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
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
/// The signal is sent by the POSIX `kill` builtin of `sh` (`kill -s KILL -- -<pgid>`), so no
/// separately installed `kill` program (procps-ng or util-linux, depending on the host) is run.
fn kill_group(pgid: u32) {
    let path = std::env::var("PATH").unwrap_or_default();
    if let Some(sh) = find_on_path("sh", &path) {
        let _ = Command::new(sh)
            .args(["-c", "kill -s KILL -- \"-$1\"", "atlas-kill-group"])
            .arg(pgid.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

/// Spawns `command` in its own process group with stdin closed, and waits up to `timeout`.
fn execute(mut command: Command, timeout: Duration) -> io::Result<Finished> {
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let stdout = drain(child.stdout.take().expect("piped"));
    let stderr = drain(child.stderr.take().expect("piped"));
    let started = Instant::now();
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
    Ok(Finished {
        outcome,
        exit_code,
        stdout: stdout.recv_timeout(grace).unwrap_or_default(),
        stderr: stderr.recv_timeout(grace).unwrap_or_default(),
    })
}

const PROBE_CONTROL: &str = ".atlas-probe-control";
const PROBE_TIMEOUT: Duration = Duration::from_secs(20);

/// The probe: in the same confinement as the run (same helper, binds, namespaces and stage), the
/// kernel is asked to execute paths, and the answer shows what the root holds. A staged,
/// non-executable control file must be found (`EACCES`, exit 126); the host path of a per-run
/// sentinel beside the stage, the host path of the control itself and, with a procfs, the
/// sentinel through `/proc/1/root` must not exist (`ENOENT`, exit 127). Anything else -- a helper
/// that failed, a namespace refused, a path that resolves -- is a failed probe. Returns what was
/// shown, or why the confinement does not hold.
fn probe(
    class: &Class<'_>,
    run: &RunDir,
    env: &BTreeMap<String, String>,
) -> Result<String, String> {
    let Class::Confined { proc, .. } = class else {
        return Err("the restricted-subprocess class sets up no filesystem confinement".into());
    };
    let token = digest(
        format!(
            "{}:{:?}:{:?}",
            run.dir.display(),
            std::process::id(),
            SystemTime::now()
        )
        .as_bytes(),
    );
    let io_failure = |e: io::Error| format!("probe: {e}");
    std::fs::write(&run.sentinel, &token).map_err(io_failure)?;
    let control = run.work.join(PROBE_CONTROL);
    std::fs::write(&control, &token).map_err(io_failure)?;
    std::fs::set_permissions(
        &control,
        std::os::unix::fs::PermissionsExt::from_mode(0o644),
    )
    .map_err(io_failure)?;
    let sentinel = run.sentinel.to_string_lossy().into_owned();
    let mut checks = vec![
        (format!("/work/{PROBE_CONTROL}"), true),
        (sentinel.clone(), false),
        (control.to_string_lossy().into_owned(), false),
    ];
    if *proc {
        checks.push((format!("/proc/1/root{sentinel}"), false));
    }
    for (path, present) in &checks {
        let probed = Program {
            found: PathBuf::from(path),
            resolved: PathBuf::from(path),
        };
        let finished = execute(class.command(&run.work, env, &probed, &[]), PROBE_TIMEOUT)
            .map_err(|e| format!("the confinement could not be started: {e}"))?;
        let (code, answer) = if *present {
            (126, "Permission denied")
        } else {
            (127, "No such file or directory")
        };
        let stderr = String::from_utf8_lossy(&finished.stderr);
        let expected = format!("failed to execute {path}: {answer}");
        if finished.exit_code != Some(code)
            || stderr
                .trim_end()
                .rsplit('\n')
                .next()
                .is_none_or(|line| !line.ends_with(&expected))
        {
            let what = if *present {
                "the staged control file was not reachable inside the confinement"
            } else {
                "a host path outside the stage was reachable inside the confinement"
            };
            return Err(format!(
                "probe failed: {what} (`{path}`: exit {:?}, {:?})",
                finished.exit_code,
                stderr.trim_end()
            ));
        }
    }
    std::fs::remove_file(&control).map_err(io_failure)?;
    Ok(format!(
        "probed this run: the staged control was found (EACCES) and {} host path(s) outside the \
         stage were absent (ENOENT): a per-run sentinel beside the stage, the control's own host \
         path{}",
        checks.len() - 1,
        if *proc {
            ", the sentinel through /proc/1/root"
        } else {
            ""
        }
    ))
}

const CONFINEMENT_MECHANISM: &str = "unshare --user --map-root-user --mount --pid: a private \
     tmpfs root, read-only, holding only /work (the stage, the one writable path), /dev/null and \
     the declared toolchain paths (read-only, nosuid, nodev); the program runs chrooted there \
     (unshare --root) as uid 1000 of a nested user namespace, with no capability";

/// Whether this host can confine a run: the probe, in a confinement with no toolchain path.
pub fn confinement_available(tools: &HostTools, staging_parent: &Path) -> Enforcement {
    let outcome = (|| {
        let tools = tools.resolve()?;
        let staging = staging_directory(staging_parent).map_err(|e| e.to_string())?;
        let run = RunDir::create(&staging, &digest(b"atlas-sandbox-confinement-probe"))
            .map_err(|e| format!("staging directory {}: {e}", staging_parent.display()))?;
        let class = Class::Confined {
            tools,
            binds: Vec::new(),
            proc: false,
            deny_network: true,
            run: &run,
        };
        probe(&class, &run, &BTreeMap::new())
    })();
    match outcome {
        Ok(probed) => Enforcement::Enforced {
            mechanism: format!("{CONFINEMENT_MECHANISM}; {probed}"),
        },
        Err(reason) => Enforcement::Unenforced { reason },
    }
}

/// A refusal: nothing of the request ran.
struct Refusal {
    kind: RefusalKind,
    reason: String,
    isolation: BTreeMap<IsolationProperty, Enforcement>,
    toolchain: Option<Box<ToolchainIdentity>>,
}

fn refusal(kind: RefusalKind, reason: String) -> Refusal {
    Refusal {
        kind,
        reason,
        isolation: BTreeMap::new(),
        toolchain: None,
    }
}

impl Refusal {
    /// Confinement asked for and not established: `FILESYSTEM_CONFINED` is recorded UNENFORCED
    /// with the same reason.
    fn unconfined(reason: String) -> Self {
        let mut refusal = refusal(RefusalKind::ConfinementUnavailable, reason);
        refusal.isolation.insert(
            IsolationProperty::FilesystemConfined,
            Enforcement::Unenforced {
                reason: refusal.reason.clone(),
            },
        );
        refusal
    }

    fn with_toolchain(mut self, toolchain: ToolchainIdentity) -> Self {
        self.toolchain = Some(Box::new(toolchain));
        self
    }
}

/// The canonical staging directory, created when missing (as before G189): a normalized absolute
/// path the helpers can take as an argument (never one read as an option, never relative to `/`).
/// A staging directory that cannot be created or resolved is a host error, not a refusal of the
/// request.
fn staging_directory(staging_parent: &Path) -> io::Result<PathBuf> {
    let context = |e: io::Error| {
        io::Error::new(
            e.kind(),
            format!("staging directory {}: {e}", staging_parent.display()),
        )
    };
    std::fs::create_dir_all(staging_parent).map_err(context)?;
    let staging = staging_parent.canonicalize().map_err(context)?;
    let text = staging.to_string_lossy();
    if !is_normal_absolute_path(&text) || text.chars().any(char::is_control) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "staging directory {} does not resolve to a normalized absolute path",
                staging_parent.display()
            ),
        ));
    }
    Ok(staging)
}

fn refused(request: &SandboxRequest, request_digest: &str, refusal: Refusal) -> Execution {
    Execution {
        run: SandboxRun {
            schema: SANDBOX_RUN_SCHEMA.into(),
            backend: backend(request).into(),
            request_digest: request_digest.to_owned(),
            outcome: RunOutcome::Refused,
            exit_code: None,
            isolation: refusal.isolation,
            inputs: request.inputs.clone(),
            outputs: Vec::new(),
            stdout_digest: digest(b""),
            stderr_digest: digest(b""),
            status: atlas_core::EpistemicStatus::Observed,
            refusal: refusal.reason,
            refusal_kind: Some(refusal.kind),
            toolchain: refusal.toolchain.map(|identity| *identity),
        },
        stdout: Vec::new(),
        stderr: Vec::new(),
    }
}

/// An early, name-based refusal (clearer, before any confinement is set up): found as `rustc` or
/// `cargo` but resolving to a file of another name, such as a linked rustup proxy. The soundness
/// rule is `verify_rust_tool`, which also refuses a hard-linked or copied proxy.
fn toolchain_proxy(program: &Program) -> Option<String> {
    let found = program.found.file_name()?;
    ((found == "rustc" || found == "cargo") && program.resolved.file_name() != Some(found)).then(
        || {
            format!(
                "`{}` is a link to `{}`, a toolchain dispatcher whose `-vV` does not identify the \
                 toolchain a run uses (`+toolchain`, the environment or a staged \
                 rust-toolchain.toml select it): declare the toolchain's own binary",
                program.found.display(),
                program.resolved.display()
            )
        },
    )
}

impl Program {
    fn resolve(found: PathBuf) -> Result<Self, String> {
        let resolved = found
            .canonicalize()
            .map_err(|e| format!("`{}`: {e}", found.display()))?;
        Ok(Self { found, resolved })
    }

    /// Its identity: the path it was found at, resolved, and the content digest of the resolved
    /// file (the file the run executes).
    fn identity(&self) -> Result<BinaryIdentity, String> {
        let bytes = std::fs::read(&self.resolved)
            .map_err(|e| format!("`{}`: {e}", self.resolved.display()))?;
        Ok(BinaryIdentity {
            path: self.found.to_string_lossy().into_owned(),
            resolved: self.resolved.to_string_lossy().into_owned(),
            digest: digest(&bytes),
        })
    }
}

/// A Rust tool whose identity rests on `-vV`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RustTool {
    Rustc,
    Cargo,
}

/// Whether `program` is found as, or resolves to, `rustc` or `cargo`.
fn rust_tool(program: &Program) -> Option<RustTool> {
    let named = |name: &str| {
        program.found.file_name().is_some_and(|f| f == name)
            || program.resolved.file_name().is_some_and(|f| f == name)
    };
    if named("rustc") {
        Some(RustTool::Rustc)
    } else if named("cargo") {
        Some(RustTool::Cargo)
    } else {
        None
    }
}

/// The soundness rule for a Rust tool (review round 6), independent of names: a `rustc` is the
/// toolchain's own binary only if `<it> --print sysroot`, run exactly as the run executes it
/// (same class, path and declared environment, no request argument, the stage still empty),
/// prints a sysroot whose `bin/rustc` canonicalizes to the file it resolved to. A dispatcher --
/// the rustup proxy linked, hard-linked or copied under the name `rustc` -- prints the sysroot of
/// the toolchain it selected, whose `bin/rustc` is another file. A `cargo` passes when the
/// `rustc` beside its resolved file passes; that binds the toolchain directory cargo was taken
/// from, not the compiler cargo invokes at run time (RES-G189-TOOLCHAIN-DIRS-UNHASHED).
fn verify_rust_tool(
    program: &Program,
    tool: RustTool,
    request: &SandboxRequest,
    class: &Class<'_>,
    work: &Path,
) -> Result<(), String> {
    let rustc = match tool {
        RustTool::Rustc => Program {
            found: program.found.clone(),
            resolved: program.resolved.clone(),
        },
        RustTool::Cargo => {
            let sibling = program.resolved.with_file_name("rustc");
            Program::resolve(sibling.clone()).map_err(|e| {
                format!(
                    "`{}` is a cargo with no rustc beside it ({e}): not a toolchain's own binary",
                    program.found.display()
                )
            })?
        }
    };
    let finished = execute(
        class.command(
            work,
            &request.env,
            &rustc,
            &["--print".into(), "sysroot".into()],
        ),
        Duration::from_millis(request.timeout_ms),
    )
    .map_err(|e| format!("`{} --print sysroot`: {e}", rustc.found.display()))?;
    let printed = String::from_utf8_lossy(&finished.stdout).trim().to_owned();
    let exited = finished.outcome == RunOutcome::Exited && finished.exit_code == Some(0);
    // The sysroot's own `bin/<tool>` must be the file run: a real rustc prints its own sysroot,
    // and a cargo must be the one in the sysroot its sibling rustc prints, so a dispatcher
    // copied beside a real rustc under the name cargo is refused too.
    let own = |tool: &str, resolved: &Path| {
        exited
            && printed.starts_with('/')
            && Path::new(&printed)
                .join("bin")
                .join(tool)
                .canonicalize()
                .is_ok_and(|own| own == resolved)
    };
    let cargo_is_own = match tool {
        RustTool::Rustc => true,
        RustTool::Cargo => own("cargo", &program.resolved),
    };
    if own("rustc", &rustc.resolved) && cargo_is_own {
        return Ok(());
    }
    Err(format!(
        "`{}` is not a toolchain's own binary: `{} --print sysroot` printed {printed:?} (exit \
         {:?}), whose bin/rustc is not `{}` or, for a cargo, whose bin/cargo is not the cargo \
         run -- a toolchain dispatcher (such as the rustup proxy, \
         linked or copied) answers for whichever toolchain `+toolchain`, the environment or a \
         staged rust-toolchain.toml selects: declare the toolchain's own binary",
        program.found.display(),
        rustc.found.display(),
        finished.exit_code,
        rustc.resolved.display()
    ))
}

/// The toolchain identity of a run: the program, the declared binaries and, when the program
/// resolves to a toolchain's own `rustc` or `cargo` (verified first; refused `TOOLCHAIN_PROXY`
/// otherwise, as is a declared binary that fails the same check), its `-vV` output, run the way
/// the run executes it.
fn toolchain_identity(
    request: &SandboxRequest,
    program: &Program,
    class: &Class<'_>,
    work: &Path,
) -> Result<ToolchainIdentity, Refusal> {
    let absent = |reason: String| refusal(RefusalKind::ToolchainIdentityAbsent, reason);
    let proxy = |reason: String| refusal(RefusalKind::ToolchainProxy, reason);
    let program_identity = program.identity().map_err(absent)?;
    let mut binaries = Vec::new();
    for binary in &request.toolchain_binaries {
        let binary = Program::resolve(PathBuf::from(binary)).map_err(absent)?;
        if let Some(tool) = rust_tool(&binary) {
            verify_rust_tool(&binary, tool, request, class, work).map_err(proxy)?;
        }
        binaries.push(binary.identity().map_err(absent)?);
    }
    binaries.sort();
    let tool = rust_tool(program);
    if let Some(tool) = tool {
        verify_rust_tool(program, tool, request, class, work).map_err(proxy)?;
    }
    let version = if tool.is_some() {
        let finished = execute(
            class.command(work, &request.env, program, &["-vV".into()]),
            Duration::from_millis(request.timeout_ms),
        )
        .map_err(|e| absent(format!("`{} -vV`: {e}", program.found.display())))?;
        if finished.outcome != RunOutcome::Exited
            || finished.exit_code != Some(0)
            || finished.stdout.is_empty()
        {
            return Err(absent(format!(
                "`{} -vV` did not report a version ({:?}, exit {:?}): {}",
                program.found.display(),
                finished.outcome,
                finished.exit_code,
                String::from_utf8_lossy(&finished.stderr).trim_end()
            )));
        }
        Some(String::from_utf8_lossy(&finished.stdout).into_owned())
    } else {
        None
    };
    Ok(ToolchainIdentity {
        program: program_identity,
        binaries,
        version,
    })
}

/// The digest of a produced output: only a regular file whose resolved path stays in the stage
/// (a link, or a path through a link that leaves the stage, is recorded as not produced).
fn output_digest(work: &Path, path: &str) -> Option<String> {
    let full = work.join(path);
    let inside = full
        .canonicalize()
        .ok()?
        .starts_with(work.canonicalize().ok()?);
    let regular = std::fs::symlink_metadata(&full).ok()?.is_file();
    (inside && regular)
        .then(|| std::fs::read(&full).ok().map(|bytes| digest(&bytes)))
        .flatten()
}

/// Reads every declared input from `source_root`: confined, inside the source root after every
/// link is resolved, a regular file, and exactly what was declared.
fn read_inputs(
    request: &SandboxRequest,
    source_root: &Path,
) -> Result<Vec<(String, Vec<u8>)>, Refusal> {
    let root = source_root.canonicalize().map_err(|e| {
        refusal(
            RefusalKind::InputUnreadable,
            format!("source root {}: {e}", source_root.display()),
        )
    })?;
    let mut staged = Vec::new();
    for input in &request.inputs {
        if !is_confined_path(&input.path) {
            return Err(refusal(
                RefusalKind::InputEscapes,
                format!("input `{}` leaves the staging root", input.path),
            ));
        }
        let unreadable = |error: String| {
            refusal(
                RefusalKind::InputUnreadable,
                format!("input `{}`: {error}", input.path),
            )
        };
        let resolved = root
            .join(&input.path)
            .canonicalize()
            .map_err(|e| unreadable(e.to_string()))?;
        if !resolved.starts_with(&root) {
            return Err(refusal(
                RefusalKind::InputEscapes,
                format!(
                    "input `{}` resolves outside the source root through a link",
                    input.path
                ),
            ));
        }
        if !std::fs::metadata(&resolved).is_ok_and(|m| m.is_file()) {
            return Err(unreadable("not a regular file".into()));
        }
        let bytes = std::fs::read(&resolved).map_err(|e| unreadable(e.to_string()))?;
        if digest(&bytes) != input.digest {
            return Err(refusal(
                RefusalKind::InputDigestMismatch,
                format!("input `{}` does not match its declared digest", input.path),
            ));
        }
        staged.push((input.path.clone(), bytes));
    }
    Ok(staged)
}

/// Run `request` with its inputs read from `source_root`, staging under `staging_parent`, with the
/// host's `unshare`, `sh`, `mount` and `mkdir` for the confined class.
pub fn run(
    request: &SandboxRequest,
    source_root: &Path,
    staging_parent: &Path,
) -> io::Result<Execution> {
    run_with(
        request,
        source_root,
        staging_parent,
        &HostTools::from_host(),
    )
}

/// `run`, with the confined class's helper binaries given explicitly.
pub fn run_with(
    request: &SandboxRequest,
    source_root: &Path,
    staging_parent: &Path,
    tools: &HostTools,
) -> io::Result<Execution> {
    // A malformed request has no canonical text; its record is still identified, under a tag no
    // canonical text starts with.
    let request_digest = match request_defect(request) {
        None => digest(canonical_request(request).as_bytes()),
        Some(_) => digest(
            format!(
                "atlas.sandbox-request.malformed\n{}",
                serde_json::to_string(request).unwrap_or_default()
            )
            .as_bytes(),
        ),
    };
    match prepare_and_run(request, &request_digest, source_root, staging_parent, tools)? {
        Ok(execution) => Ok(execution),
        Err(refusal) => Ok(refused(request, &request_digest, refusal)),
    }
}

fn prepare_and_run(
    request: &SandboxRequest,
    request_digest: &str,
    source_root: &Path,
    staging_parent: &Path,
    tools: &HostTools,
) -> io::Result<Result<Execution, Refusal>> {
    if let Some(defect) = request_defect(request) {
        return Ok(Err(refusal(RefusalKind::RequestMalformed, defect)));
    }
    let staging = staging_directory(staging_parent)?;
    // Every declared input is confined, present and exactly what was declared -- or nothing runs.
    let staged = match read_inputs(request, source_root) {
        Ok(staged) => staged,
        Err(refusal) => return Ok(Err(refusal)),
    };
    if let Some(output) = request.outputs.iter().find(|o| !is_confined_path(o)) {
        return Ok(Err(refusal(
            RefusalKind::OutputEscapes,
            format!("output `{output}` leaves the staging root"),
        )));
    }
    let binds = match &request.confinement {
        Some(confinement) => {
            if let Some((name, why)) = request
                .env
                .keys()
                .find_map(|name| refused_env_name(name).map(|why| (name, why)))
            {
                return Ok(Err(refusal(
                    RefusalKind::EnvironmentRefused,
                    format!("environment variable `{name}` {why}"),
                )));
            }
            match toolchain_binds(confinement, &staging) {
                Ok(binds) => Some(binds),
                Err(reason) => {
                    return Ok(Err(refusal(RefusalKind::ToolchainPathRefused, reason)));
                }
            }
        }
        None => None,
    };
    let declared_path = request.env.get("PATH").cloned().unwrap_or_default();
    let Some(found) = find_on_path(&request.program, &declared_path) else {
        return Ok(Err(refusal(
            RefusalKind::ProgramNotOnPath,
            format!("`{}` is not on the declared PATH", request.program),
        )));
    };
    let program = match Program::resolve(found) {
        Ok(program) => program,
        Err(reason) => return Ok(Err(refusal(RefusalKind::ToolchainIdentityAbsent, reason))),
    };
    if let Some(reason) = toolchain_proxy(&program) {
        return Ok(Err(refusal(RefusalKind::ToolchainProxy, reason)));
    }
    for binary in &request.toolchain_binaries {
        if let Ok(binary) = Program::resolve(PathBuf::from(binary))
            && let Some(reason) = toolchain_proxy(&binary)
        {
            return Ok(Err(refusal(RefusalKind::ToolchainProxy, reason)));
        }
    }
    // The restricted class enters a network namespace through `unshare` when asked and possible.
    let network =
        (request.confinement.is_none() && request.deny_network && network_isolation_available())
            .then(|| find_on_path("unshare", &std::env::var("PATH").unwrap_or_default()))
            .flatten();
    for binary in &request.toolchain_binaries {
        let declared = match &binds {
            Some(binds) => under_toolchain(binary, binds),
            None => is_normal_absolute_path(binary),
        };
        if !declared {
            return Ok(Err(refusal(
                RefusalKind::ToolchainPathRefused,
                format!(
                    "toolchain binary `{binary}` is not an absolute path{}",
                    if binds.is_some() {
                        " under a declared toolchain path"
                    } else {
                        ""
                    }
                ),
            )));
        }
    }
    // Inside the confined root the found path must exist at its host path and resolve, through
    // the same binds, to the same file: both lie under declared toolchain paths (anything
    // undeclared on the way is absent there).
    if let Some(binds) = &binds {
        for (what, path) in [
            ("is found at", &program.found),
            ("resolves to", &program.resolved),
        ] {
            let path = path.to_string_lossy();
            if !under_toolchain(&path, binds) {
                return Ok(Err(refusal(
                    RefusalKind::ProgramOutsideToolchain,
                    format!(
                        "`{}` {what} `{path}`, which is not under a declared toolchain path",
                        request.program
                    ),
                )));
            }
        }
    }

    let run = RunDir::create(&staging, request_digest)?;
    let (class, probed) = match (&request.confinement, binds) {
        (Some(confinement), Some(binds)) => {
            let tools = match tools.resolve() {
                Ok(tools) => tools,
                Err(reason) => return Ok(Err(Refusal::unconfined(reason))),
            };
            let class = Class::Confined {
                tools,
                binds,
                proc: confinement.proc,
                deny_network: request.deny_network,
                run: &run,
            };
            match probe(&class, &run, &request.env) {
                Ok(probed) => (class, Some(probed)),
                Err(reason) => return Ok(Err(Refusal::unconfined(reason))),
            }
        }
        _ => (Class::Restricted { network }, None),
    };

    let toolchain = match toolchain_identity(request, &program, &class, &run.work) {
        Ok(identity) => identity,
        Err(refusal) => return Ok(Err(refusal)),
    };
    if let Some(pinned) = &request.pinned_toolchain
        && let Some(difference) = pinned.mismatch(&toolchain)
    {
        return Ok(Err(refusal(
            RefusalKind::ToolchainIdentityMismatch,
            format!("the toolchain differs from the pinned identity: {difference}"),
        )
        .with_toolchain(toolchain)));
    }

    run.reset_work()?;
    for (path, bytes) in &staged {
        let target = run.work.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(target, bytes)?;
    }
    let network_enforced = matches!(&class, Class::Restricted { network: Some(_) })
        || matches!(
            &class,
            Class::Confined {
                deny_network: true,
                ..
            }
        );
    let finished = execute(
        class.command(&run.work, &request.env, &program, &request.args),
        Duration::from_millis(request.timeout_ms),
    )?;
    let outputs = request
        .outputs
        .iter()
        .map(|path| ProducedOutput {
            path: path.clone(),
            digest: output_digest(&run.work, path),
        })
        .collect();
    let isolation = isolation(request, &class, network_enforced, probed);
    drop(class);
    drop(run);

    Ok(Ok(Execution {
        run: SandboxRun {
            schema: SANDBOX_RUN_SCHEMA.into(),
            backend: backend(request).into(),
            request_digest: request_digest.to_owned(),
            outcome: finished.outcome,
            exit_code: finished.exit_code,
            isolation,
            inputs: request.inputs.clone(),
            outputs,
            stdout_digest: digest(&finished.stdout),
            stderr_digest: digest(&finished.stderr),
            status: atlas_core::EpistemicStatus::Observed,
            refusal: String::new(),
            refusal_kind: None,
            toolchain: Some(toolchain),
        },
        stdout: finished.stdout,
        stderr: finished.stderr,
    }))
}

/// Every isolation property of a run that ran, `ENFORCED` with its mechanism or `UNENFORCED`
/// with why.
fn isolation(
    request: &SandboxRequest,
    class: &Class<'_>,
    network_enforced: bool,
    probed: Option<String>,
) -> BTreeMap<IsolationProperty, Enforcement> {
    let enforced = |mechanism: &str| Enforcement::Enforced {
        mechanism: mechanism.into(),
    };
    let unenforced = |reason: &str| Enforcement::Unenforced {
        reason: reason.into(),
    };
    let confined = matches!(class, Class::Confined { .. });
    BTreeMap::from([
        (
            IsolationProperty::EnvironmentCleared,
            enforced(if confined {
                "env_clear; the helpers run with no environment and export only the declared one \
                 (loader variables refused) before the program is executed"
            } else {
                "env_clear, then only the declared environment"
            }),
        ),
        (
            IsolationProperty::InputsStaged,
            enforced(
                "digest-verified copies of the declared inputs (never through a link leaving the \
                 source root) in a fresh directory",
            ),
        ),
        (
            IsolationProperty::WorkingDirectoryConfined,
            enforced(if confined {
                "the working directory is /work, the stage bound into the confined root"
            } else {
                "the working directory is the staging directory"
            }),
        ),
        (
            IsolationProperty::StdinClosed,
            enforced("stdin is /dev/null"),
        ),
        (
            IsolationProperty::TimeLimited,
            enforced(if confined {
                "the run's process group is killed at the declared timeout; the program is pid 1 \
                 of its own pid namespace, so every descendant dies with it"
            } else {
                "the run's process group is killed at the declared timeout (a descendant that \
                 leaves the group is not)"
            }),
        ),
        (
            IsolationProperty::NetworkDenied,
            if network_enforced && confined {
                enforced("unshare --net in the confinement's user namespace: loopback only")
            } else if network_enforced {
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
            match probed {
                Some(probed) if confined => Enforcement::Enforced {
                    mechanism: format!("{CONFINEMENT_MECHANISM}; {probed}"),
                },
                _ => unenforced(
                    "absolute-path and `..` reads are not confined: the restricted-subprocess \
                     class sets up no filesystem namespace or Landlock ruleset",
                ),
            },
        ),
    ])
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
    use atlas_core::sandbox::ToolchainPath;

    /// A test directory removed when dropped, so a failing test leaves nothing behind.
    struct Scratch(PathBuf);

    impl std::ops::Deref for Scratch {
        type Target = Path;
        fn deref(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn scratch_at(dir: PathBuf) -> Scratch {
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join("stage")).unwrap();
        std::fs::write(dir.join("src/a.txt"), "declared\n").unwrap();
        std::fs::write(dir.join("src/secret.txt"), "undeclared\n").unwrap();
        std::fs::write(dir.join("outside.txt"), "host sentinel\n").unwrap();
        Scratch(dir)
    }

    fn scratch(name: &str) -> Scratch {
        scratch_at(
            std::env::temp_dir().join(format!("atlas-sandbox-test-{name}-{}", std::process::id())),
        )
    }

    /// A test directory under the build's `target/` directory, for declarations that `/tmp` (a
    /// socket directory) may not hold.
    fn target_scratch(name: &str) -> Scratch {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        scratch_at(workspace.join(format!(
            "target/atlas-sandbox-test-{name}-{}",
            std::process::id()
        )))
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
            ..SandboxRequest::default()
        }
    }

    /// The host directories a dynamically linked `/usr/bin/sh` and coreutils need.
    fn shell_toolchain() -> Vec<ToolchainPath> {
        ["/usr/bin", "/usr/lib", "/usr/lib64", "/lib", "/lib64"]
            .into_iter()
            .filter(|p| Path::new(p).exists())
            .map(|p| ToolchainPath { path: p.into() })
            .collect()
    }

    fn confined(root: &Path, script: &str) -> SandboxRequest {
        SandboxRequest {
            env: BTreeMap::from([("PATH".into(), "/usr/bin".into())]),
            confinement: Some(Confinement {
                toolchain: shell_toolchain(),
                proc: false,
            }),
            ..request(root, script)
        }
    }

    fn execute(root: &Path, request: &SandboxRequest) -> Execution {
        run(request, &root.join("src"), &root.join("stage")).unwrap()
    }

    /// Whether this host lets `unshare` enter the namespaces the confined class uses, asked
    /// directly (not through the backend under test).
    fn namespaces_available() -> bool {
        Command::new("unshare")
            .args([
                "--user",
                "--map-root-user",
                "--mount",
                "--pid",
                "--fork",
                "true",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    /// `true` when the run was confined. On a host whose namespaces work, a refused confinement
    /// fails the test. On a host without them the refusal must say so (UNENFORCED with the
    /// reason, nothing run) and the caller stops: reported, not faked.
    fn confined_here(run: &Execution) -> bool {
        if run.run.refusal_kind == Some(RefusalKind::ConfinementUnavailable) {
            assert!(
                !namespaces_available(),
                "the host has the namespaces but the run was refused: {}",
                run.run.refusal
            );
            let host = confinement_available(&HostTools::from_host(), &std::env::temp_dir());
            let Enforcement::Unenforced { reason } = host else {
                panic!(
                    "the host confines but the run was refused: {}",
                    run.run.refusal
                );
            };
            assert!(!run.run.isolation[&P::FilesystemConfined].is_enforced());
            eprintln!("FILESYSTEM_CONFINED UNENFORCED on this host: {reason}");
            return false;
        }
        assert_eq!(run.run.backend, CONFINED_SUBPROCESS);
        assert_ne!(
            run.run.outcome,
            RunOutcome::Refused,
            "refused: {}",
            run.run.refusal
        );
        assert!(
            run.run.isolation[&P::FilesystemConfined].is_enforced(),
            "{:?}",
            run.run
        );
        true
    }

    fn stdout(run: &Execution) -> String {
        String::from_utf8_lossy(&run.stdout).into_owned()
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
    }

    #[test]
    fn a_tampered_or_escaping_input_is_refused_before_anything_runs() {
        let root = scratch("refused");
        let mut tampered = request(&root, "echo ran > out.txt");
        tampered.inputs[0].digest = digest(b"something else");
        let run = execute(&root, &tampered);
        assert_eq!(run.run.outcome, RunOutcome::Refused);
        assert!(run.run.refusal.contains("declared digest"));
        assert_eq!(run.run.refusal_kind, Some(RefusalKind::InputDigestMismatch));
        assert!(run.run.isolation.is_empty() && run.run.outputs.is_empty());
        let mut escaping = request(&root, "true");
        escaping.inputs.push(DeclaredInput {
            path: "../secret.txt".into(),
            digest: digest(b"undeclared\n"),
        });
        let run = execute(&root, &escaping);
        assert_eq!(run.run.outcome, RunOutcome::Refused);
        assert_eq!(run.run.refusal_kind, Some(RefusalKind::InputEscapes));
        let mut hidden = request(&root, "true");
        hidden.env.insert("PATH".into(), "/nonexistent".into());
        let run = execute(&root, &hidden);
        assert!(run.run.refusal.contains("declared PATH"));
        assert_eq!(run.run.refusal_kind, Some(RefusalKind::ProgramNotOnPath));
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
    }

    /// The falsification of FILESYSTEM_CONFINED: every absolute or `..` path outside the declared
    /// inputs and toolchain is unreadable, the declared input is readable, the toolchain is
    /// readable but not writable, and writes land only in the stage.
    #[test]
    fn a_confined_run_reads_only_its_inputs_and_toolchain_and_writes_only_its_stage() {
        let root = scratch("confined");
        let repo_manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
        assert!(
            repo_manifest.is_file(),
            "the repository manifest exists on the host"
        );
        let host_write = root.join("host-write");
        let tmp_write = std::env::temp_dir().join(format!("atlas-g189-{}", std::process::id()));
        let unreadable = [
            "/etc/passwd".to_owned(),
            repo_manifest.to_string_lossy().into_owned(),
            root.join("outside.txt").to_string_lossy().into_owned(),
            root.join("src/secret.txt").to_string_lossy().into_owned(),
            root.join("src/a.txt").to_string_lossy().into_owned(),
            "../../../../../../etc/passwd".to_owned(),
            "../outside.txt".to_owned(),
            "/proc/self/root/etc/passwd".to_owned(),
        ];
        let mut script = String::from("cat a.txt; ");
        for path in &unreadable {
            script.push_str(&format!(
                "if cat '{path}' >/dev/null 2>&1; then echo LEAK:{path}; else echo DENIED; fi; "
            ));
        }
        script.push_str(&format!(
            "test -r /usr/bin/sh && test -x /usr/bin/cat && echo TOOLCHAIN_READABLE; \
             if test -w /usr/bin || test -w /usr/bin/sh; then echo LEAK:toolchain-writable; fi; \
             if echo x > /atlas-g189-root-write 2>/dev/null; then echo LEAK:root-write; fi; \
             if echo x > '{}' 2>/dev/null; then echo LEAK:host-write; fi; \
             if echo x > '{}' 2>/dev/null; then echo LEAK:tmp-write; fi; \
             if echo x > ../escaped 2>/dev/null; then echo LEAK:parent-write; fi; \
             if mount -o remount,rw /usr/bin 2>/dev/null; then echo LEAK:remount; fi; \
             if test -x /usr/bin/perl; then mkdir x; perl -e 'chroot(\"x\") or die \"chroot \
             refused: $!\\n\"; chdir(\"../../../../../../..\"); chroot(\".\"); \
             open(F, \"<\", \"/etc/passwd\") or die \"open: $!\\n\"; print \"LEAK:chroot\\n\"' 2>&1; \
             else echo chroot refused: no perl; fi; \
             echo made > out.txt; echo ---; env",
            host_write.display(),
            tmp_write.display()
        ));
        let run = execute(&root, &confined(&root, &script));
        if !confined_here(&run) {
            return;
        }
        let out = stdout(&run);
        assert_eq!(run.run.outcome, RunOutcome::Exited, "{out}");
        assert!(
            out.starts_with("declared\n"),
            "the declared input is readable: {out}"
        );
        assert!(!out.contains("LEAK"), "{out}");
        assert_eq!(out.matches("DENIED").count(), unreadable.len(), "{out}");
        assert!(out.contains("TOOLCHAIN_READABLE"), "{out}");
        assert!(
            out.contains("chroot refused"),
            "no capability to leave the chroot: {out}"
        );
        let env = out.split("---\n").nth(1).unwrap();
        assert!(
            !env.contains("HOME=") && env.contains("PATH=/usr/bin"),
            "{env}"
        );
        assert_eq!(
            run.run.outputs[0].digest.as_deref(),
            Some(digest(b"made\n").as_str()),
            "the write to the stage landed"
        );
        assert!(!host_write.exists() && !tmp_write.exists());
        assert!(!root.join("stage").read_dir().unwrap().any(|_| true));
        assert!(run.run.isolation[&P::NetworkDenied].is_enforced());
        let Enforcement::Enforced { mechanism } = &run.run.isolation[&P::FilesystemConfined] else {
            unreachable!()
        };
        assert!(mechanism.contains("probed this run"), "{mechanism}");
    }

    /// The confined root, as the run itself sees it in `/proc/self/mountinfo`: exactly `/`,
    /// `/work`, `/dev/null`, `/proc` and the declared toolchain paths (a regular file among them),
    /// every mount private (no `shared:`/`master:` peer group), the root and the toolchain
    /// read-only, `nosuid` and `nodev`, and only `/work` writable. A declared file is readable
    /// and not writable (`test -w` asks `access(2)`, so no write is attempted on the host).
    #[test]
    fn the_confined_root_holds_exactly_its_declared_mounts_private_and_read_only() {
        let root = scratch("mounts");
        let mut toolchain = shell_toolchain();
        let file = Path::new("/etc/hostname");
        if file.is_file() {
            toolchain.push(ToolchainPath {
                path: "/etc/hostname".into(),
            });
        }
        let mut request = confined(
            &root,
            "cat /etc/hostname 2>/dev/null; \
             if test -w /etc/hostname; then echo LEAK:file-writable; fi; \
             if cat /etc/passwd >/dev/null 2>&1; then echo LEAK:passwd; fi; \
             echo ---; cat /proc/self/mountinfo",
        );
        request.confinement = Some(Confinement {
            toolchain: toolchain.clone(),
            proc: true,
        });
        let run = execute(&root, &request);
        if !confined_here(&run) {
            return;
        }
        let out = stdout(&run);
        let (head, mountinfo) = out.split_once("---\n").expect("{out}");
        assert!(!head.contains("LEAK"), "{head}");
        if file.is_file() {
            assert_eq!(
                head,
                std::fs::read_to_string(file).unwrap(),
                "the file is readable"
            );
        }
        let mut seen = BTreeMap::new();
        for line in mountinfo.lines() {
            let fields: Vec<&str> = line.split(' ').collect();
            let separator = fields.iter().position(|f| *f == "-").expect(line);
            let optional = &fields[6..separator];
            assert!(
                optional.iter().all(|f| !f.starts_with("shared:")
                    && !f.starts_with("master:")
                    && !f.starts_with("propagate_from:")),
                "a mount is not private: {line}"
            );
            let options: Vec<String> = fields[5].split(',').map(str::to_owned).collect();
            assert!(
                seen.insert(fields[4].to_owned(), options).is_none(),
                "{line}"
            );
        }
        // A declared link (`/lib -> usr/lib`) is a link in the root, not a mount.
        let bound: Vec<String> = toolchain
            .iter()
            .map(|t| t.path.clone())
            .filter(|p| {
                !std::fs::symlink_metadata(p)
                    .unwrap()
                    .file_type()
                    .is_symlink()
            })
            .collect();
        let mut expected: Vec<String> = ["/", "/work", "/dev/null", "/proc"]
            .map(str::to_owned)
            .into_iter()
            .chain(bound.iter().cloned())
            .collect();
        expected.sort();
        assert_eq!(
            seen.keys().cloned().collect::<Vec<_>>(),
            expected,
            "{mountinfo}"
        );
        let has = |point: &str, option: &str| seen[point].iter().any(|o| o == option);
        for point in std::iter::once("/").chain(bound.iter().map(String::as_str)) {
            for option in ["ro", "nosuid", "nodev"] {
                assert!(
                    has(point, option),
                    "{point} lacks {option}: {:?}",
                    seen[point]
                );
            }
        }
        for option in ["rw", "nosuid", "nodev"] {
            assert!(has("/work", option), "/work lacks {option}");
        }
        for option in ["nosuid", "nodev", "noexec"] {
            assert!(has("/proc", option), "/proc lacks {option}");
        }
    }

    /// G189 review f1: a program given as a relative path was hashed relative to Atlas's working
    /// directory and executed relative to the stage. Such a path is refused before anything runs,
    /// in both classes, and the file a run executes is the one its identity names.
    #[test]
    fn a_relative_program_path_is_refused_and_the_executed_file_is_the_identified_one() {
        let root = scratch("program");
        for dir in ["h1/h2/h3/h4", "h1/alt", "alt"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        let marker = root.join("ran");
        for (dir, text) in [("h1/alt", "MEASURED"), ("alt", "OTHER")] {
            let program = root.join(dir).join("x");
            std::fs::write(
                &program,
                format!("#!/bin/sh\necho {text} > '{}'\n", marker.display()),
            )
            .unwrap();
            std::fs::set_permissions(
                &program,
                std::os::unix::fs::PermissionsExt::from_mode(0o755),
            )
            .unwrap();
        }
        for confine in [false, true] {
            for program in [
                "../../../alt/x",
                "alt/x",
                "./x",
                "/usr/bin/../bin/sh",
                "/usr/bin/",
            ] {
                let mut request = if confine {
                    confined(&root, "true")
                } else {
                    request(&root, "true")
                };
                request.program = program.into();
                let run = execute(&root, &request);
                assert_eq!(
                    run.run.refusal_kind,
                    Some(RefusalKind::RequestMalformed),
                    "{program}: {}",
                    run.run.refusal
                );
                assert!(!marker.exists(), "{program} ran");
            }
            let mut request = if confine {
                confined(&root, "readlink /proc/$$/exe")
            } else {
                request(&root, "readlink /proc/$$/exe")
            };
            if let Some(confinement) = request.confinement.as_mut() {
                confinement.proc = true;
            }
            let run = execute(&root, &request);
            if confine && !confined_here(&run) {
                continue;
            }
            assert_eq!(
                stdout(&run).trim_end(),
                run.run.toolchain.as_ref().unwrap().program.resolved,
                "{confine}: the executed file is the identified one"
            );
        }
    }

    /// G189 review f3 at the runtime: the forging request is refused under a digest no canonical
    /// text can have; the request it imitated keeps its own.
    #[test]
    fn a_request_forging_another_identity_is_refused_under_its_own_digest() {
        let root = scratch("forged");
        let mut forged = request(&root, "true");
        forged.outputs = vec!["x\nconfined proc=false\ntoolchain_path=11:/usr/bin".into()];
        let mut imitated = confined(&root, "true");
        imitated.outputs = vec!["x".into()];
        let forged_run = execute(&root, &forged);
        assert_eq!(
            forged_run.run.refusal_kind,
            Some(RefusalKind::RequestMalformed)
        );
        assert_ne!(
            forged_run.run.request_digest,
            digest(canonical_request(&imitated).as_bytes())
        );
        assert_ne!(
            forged_run.run.request_digest,
            digest(canonical_request(&forged).as_bytes()),
            "a malformed request is not identified by its ambiguous canonical text"
        );
    }

    /// A staging directory is created when missing (as before G189) and resolved before the
    /// helpers see it: a non-normalized one works, a missing one is created, and one that cannot
    /// be created is a host error, not a refusal.
    #[test]
    fn the_staging_directory_is_created_and_resolved_before_the_helpers_see_it() {
        let root = scratch("staging");
        let request = confined(&root, "cat a.txt");
        for staging in [root.join("stage/../stage"), root.join("missing/deeper")] {
            let run = run(&request, &root.join("src"), &staging).unwrap();
            if confined_here(&run) {
                assert_eq!(stdout(&run), "declared\n", "{}", staging.display());
            }
            assert!(staging.is_dir());
        }
        let error = run(&request, &root.join("src"), &root.join("outside.txt/stage"))
            .err()
            .expect("a staging directory under a file cannot be created");
        assert!(error.to_string().contains("staging directory"), "{error}");
    }

    /// `argv[0]` and links (G189 rounds 3-4). Every class executes the path the program was found
    /// at, so `argv[0]`'s base name is the link name in all three modes; the identity digests the
    /// file the link resolves to. In the confined class both the found path and its target must
    /// lie under declared toolchain paths. `-vV` is asked by the found name. A declared link that
    /// resolves into a socket directory is refused.
    #[test]
    fn a_linked_program_runs_under_its_link_name_in_every_class() {
        let root = target_scratch("alias");
        // `sh -c 'echo $0'` prints the shell's own argv[0] (a `#!` script would see the path the
        // kernel executed instead), so links to the host shell show what argv[0] a run gets.
        let bin = root.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let host_shell = Path::new("/usr/bin/sh").canonicalize().unwrap();
        for link in ["x", "rustc"] {
            std::os::unix::fs::symlink(&host_shell, bin.join(link)).unwrap();
        }
        let request = |program: &str, deny_network: bool| SandboxRequest {
            program: program.into(),
            args: vec!["-c".into(), "echo \"${0##*/}\"".into()],
            env: BTreeMap::from([("PATH".into(), format!("{}:/usr/bin", bin.display()))]),
            timeout_ms: 20_000,
            deny_network,
            ..SandboxRequest::default()
        };
        let bin_path = ToolchainPath {
            path: bin.to_string_lossy().into_owned(),
        };
        let confine = |mut request: SandboxRequest, toolchain: Vec<ToolchainPath>| {
            request.confinement = Some(Confinement {
                toolchain,
                proc: false,
            });
            execute(&root, &request)
        };
        let mut full = shell_toolchain();
        full.push(bin_path.clone());
        for (mode, run) in [
            ("direct", execute(&root, &request("x", false))),
            ("network namespace", execute(&root, &request("x", true))),
            ("confined", confine(request("x", true), full.clone())),
        ] {
            if mode == "confined" && !confined_here(&run) {
                continue;
            }
            assert_eq!(stdout(&run), "x\n", "{mode}: argv[0] is the link name");
            let identity = run.run.toolchain.unwrap();
            assert!(identity.program.path.ends_with("/bin/x"), "{mode}");
            assert_eq!(
                identity.program.resolved,
                host_shell.to_string_lossy(),
                "{mode}"
            );
        }
        // A program found as `rustc` that is a link to another file is a toolchain dispatcher.
        let asked = execute(&root, &request("rustc", false));
        assert_eq!(
            asked.run.refusal_kind,
            Some(RefusalKind::ToolchainProxy),
            "{}",
            asked.run.refusal
        );
        // Confined: the found path and its target must both be declared.
        let undeclared_link = confine(request("x", true), shell_toolchain());
        assert_eq!(
            undeclared_link.run.refusal_kind,
            Some(RefusalKind::ProgramOutsideToolchain)
        );
        assert!(undeclared_link.run.refusal.contains("is found at"));
        let libraries: Vec<ToolchainPath> = ["/usr/lib", "/usr/lib64", "/lib", "/lib64"]
            .into_iter()
            .filter(|p| Path::new(p).exists())
            .map(|p| ToolchainPath { path: p.into() })
            .chain([bin_path.clone()])
            .collect();
        let undeclared_target = confine(request("x", true), libraries);
        assert_eq!(
            undeclared_target.run.refusal_kind,
            Some(RefusalKind::ProgramOutsideToolchain)
        );
        assert!(undeclared_target.run.refusal.contains("resolves to"));
        // The socket-directory check also applies to where a declared path resolves.
        if Path::new("/run").is_dir() {
            std::os::unix::fs::symlink("/run", root.join("runlink")).unwrap();
            let run = confine(
                request("x", true),
                vec![
                    bin_path.clone(),
                    ToolchainPath {
                        path: root.join("runlink").to_string_lossy().into_owned(),
                    },
                ],
            );
            assert_eq!(
                run.run.refusal_kind,
                Some(RefusalKind::ToolchainPathRefused)
            );
            assert!(
                run.run.refusal.contains("live endpoints"),
                "{}",
                run.run.refusal
            );
        }
    }

    /// G189 review round 6: the dispatcher check does not rest on names. A rustup proxy copied
    /// (and hard-linked) under the names `rustc` and `cargo` -- rustup's fallback install mode --
    /// is refused by its sysroot in every class, as program and as declared binary; a real
    /// `rustc` found under another name (`rustc-1.90`) is verified and its `-vV` recorded.
    #[test]
    fn a_copied_dispatcher_is_refused_by_its_sysroot_and_a_renamed_rustc_is_identified() {
        let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
        let rustup = home.join(".cargo/bin/rustup");
        let rustup_home = home.join(".rustup");
        let Some(sysroot) = sysroot() else {
            eprintln!("no rustc on this host: not run");
            return;
        };
        if !sysroot.starts_with(&rustup_home) {
            eprintln!("the toolchain is not under ~/.rustup: not run");
            return;
        }
        let root = target_scratch("copied-proxy");
        let bin = root.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::os::unix::fs::symlink(sysroot.join("bin/rustc"), bin.join("rustc-1.90")).unwrap();
        let modes = [
            ("direct", false, false),
            ("network namespace", true, false),
            ("confined", true, true),
        ];
        let request = |program: &str, deny_network: bool, confine: bool| SandboxRequest {
            program: program.into(),
            args: vec!["--version".into()],
            env: BTreeMap::from([
                ("PATH".into(), bin.to_string_lossy().into_owned()),
                (
                    "RUSTUP_HOME".into(),
                    rustup_home.to_string_lossy().into_owned(),
                ),
                (
                    "CARGO_HOME".into(),
                    home.join(".cargo").to_string_lossy().into_owned(),
                ),
                ("RUSTUP_TOOLCHAIN".into(), "1.90.0".into()),
            ]),
            timeout_ms: 60_000,
            deny_network,
            confinement: confine.then(|| Confinement {
                toolchain: [bin.to_string_lossy(), rustup_home.to_string_lossy()]
                    .into_iter()
                    .map(|p| p.into_owned())
                    .chain(
                        ["/usr/lib", "/usr/lib64", "/lib", "/lib64"]
                            .into_iter()
                            .filter(|p| Path::new(p).exists())
                            .map(str::to_owned),
                    )
                    .map(|path| ToolchainPath { path })
                    .collect(),
                proc: true,
            }),
            ..SandboxRequest::default()
        };
        // The renamed real rustc: verified by its sysroot and identified by -vV.
        for (mode, deny_network, confine) in modes {
            let run = execute(&root, &request("rustc-1.90", deny_network, confine));
            if confine && !confined_here(&run) {
                continue;
            }
            assert_eq!(run.run.exit_code, Some(0), "{mode}: {}", run.run.refusal);
            let identity = run.run.toolchain.unwrap();
            assert_eq!(
                identity.program.resolved,
                sysroot.join("bin/rustc").to_string_lossy()
            );
            assert!(
                identity
                    .version
                    .as_deref()
                    .is_some_and(|v| v.contains("release: ")),
                "{mode}: a real rustc under another name is still asked -vV"
            );
        }
        if !rustup.is_file() {
            eprintln!("no rustup on this host: the copied proxy was not exercised");
            return;
        }
        std::fs::copy(&rustup, bin.join("rustc")).unwrap();
        std::fs::hard_link(bin.join("rustc"), bin.join("cargo")).unwrap();
        for (mode, deny_network, confine) in modes {
            let mut plus = request("rustc", deny_network, confine);
            plus.args = vec!["+stable".into(), "--version".into()];
            let mut declared = request("rustc-1.90", deny_network, confine);
            declared.toolchain_binaries = vec![bin.join("rustc").to_string_lossy().into()];
            for (case, request) in [
                ("rustc", request("rustc", deny_network, confine)),
                ("+stable", plus),
                ("cargo", request("cargo", deny_network, confine)),
                ("declared binary", declared),
            ] {
                let run = execute(&root, &request);
                assert_eq!(
                    run.run.refusal_kind,
                    Some(RefusalKind::ToolchainProxy),
                    "{mode} {case}: {}",
                    run.run.refusal
                );
                assert!(
                    run.run.refusal.contains("--print sysroot"),
                    "{mode} {case}: {}",
                    run.run.refusal
                );
                assert!(run.stdout.is_empty(), "{mode} {case}: nothing ran");
            }
        }
        // G189 review round 6: a dispatcher named cargo beside a real rustc is refused (cargo
        // must be its sysroot's own bin/cargo), while the toolchain's own cargo is identified.
        let beside = root.join("beside");
        std::fs::create_dir_all(&beside).unwrap();
        std::fs::hard_link(bin.join("rustc"), beside.join("cargo")).unwrap();
        std::os::unix::fs::symlink(sysroot.join("bin/rustc"), beside.join("rustc")).unwrap();
        let in_dir = |dir: &Path, deny_network: bool, confine: bool| {
            let mut cargo = request("cargo", deny_network, confine);
            cargo
                .env
                .insert("PATH".into(), dir.to_string_lossy().into_owned());
            if let Some(confinement) = cargo.confinement.as_mut()
                && !confinement
                    .toolchain
                    .iter()
                    .any(|declared| dir.starts_with(&declared.path))
            {
                confinement.toolchain.push(ToolchainPath {
                    path: dir.to_string_lossy().into_owned(),
                });
                confinement.toolchain.sort();
            }
            cargo
        };
        for (mode, deny_network, confine) in modes {
            let run = execute(&root, &in_dir(&beside, deny_network, confine));
            if run.run.refusal_kind == Some(RefusalKind::ConfinementUnavailable)
                && !confined_here(&run)
            {
                continue;
            }
            assert_eq!(
                run.run.refusal_kind,
                Some(RefusalKind::ToolchainProxy),
                "{mode} cargo beside a real rustc: {}",
                run.run.refusal
            );
            assert!(run.stdout.is_empty(), "{mode}: nothing ran");
            let own = sysroot.join("bin");
            if !own.join("cargo").is_file() {
                continue;
            }
            let run = execute(&root, &in_dir(&own, deny_network, confine));
            assert_eq!(run.run.exit_code, Some(0), "{mode}: {}", run.run.refusal);
            assert!(
                run.run
                    .toolchain
                    .and_then(|t| t.version)
                    .is_some_and(|v| v.contains("release: ")),
                "{mode}: the toolchain's own cargo is identified by -vV"
            );
        }
    }

    /// G189 review round 5: a declared link is a link inside the root (not a bind of its
    /// target's content), so `..` beneath it climbs as on the host; its target must be declared,
    /// and a declaration passing through a link is refused.
    #[test]
    fn a_declared_link_is_a_link_in_the_root_and_its_target_must_be_declared() {
        let root = target_scratch("declared-links");
        let nest = root.join("nest");
        std::fs::create_dir_all(nest.join("real")).unwrap();
        std::fs::write(nest.join("marker"), "nest\n").unwrap();
        std::fs::write(nest.join("real/file"), "real\n").unwrap();
        std::os::unix::fs::symlink("nest/real", root.join("alias")).unwrap();
        let text = |p: PathBuf| p.to_string_lossy().into_owned();
        let alias = text(root.join("alias"));
        let run_with_paths = |extra: Vec<String>| {
            let mut request = confined(
                &root,
                &format!("test -L '{alias}' && cat '{alias}/file' '{alias}/../marker'"),
            );
            request
                .confinement
                .as_mut()
                .unwrap()
                .toolchain
                .extend(extra.into_iter().map(|path| ToolchainPath { path }));
            execute(&root, &request)
        };
        let dangling = run_with_paths(vec![alias.clone()]);
        assert_eq!(
            dangling.run.refusal_kind,
            Some(RefusalKind::ToolchainPathRefused)
        );
        assert!(
            dangling.run.refusal.contains("declare it"),
            "{}",
            dangling.run.refusal
        );
        let through = run_with_paths(vec![text(root.join("alias/file")), text(nest.clone())]);
        assert_eq!(
            through.run.refusal_kind,
            Some(RefusalKind::ToolchainPathRefused)
        );
        assert!(
            through.run.refusal.contains("passes through a link"),
            "{}",
            through.run.refusal
        );
        let declared = run_with_paths(vec![alias.clone(), text(nest)]);
        if confined_here(&declared) {
            assert_eq!(
                stdout(&declared),
                "real\nnest\n",
                "the link resolves, and `..` beneath it climbs, as on the host: {}",
                String::from_utf8_lossy(&declared.stderr)
            );
        }
    }

    /// A plain v1 request for `sh` (a link to the host shell) runs as `sh` in every mode: `$0` is
    /// `/usr/bin/sh` directly, inside a network namespace and confined.
    #[test]
    fn a_plain_sh_request_runs_as_sh_in_every_mode() {
        let root = scratch("plain-sh");
        let plain = |deny_network: bool| SandboxRequest {
            program: "sh".into(),
            args: vec!["-c".into(), "echo \"$0\"".into()],
            env: BTreeMap::from([("PATH".into(), "/usr/bin".into())]),
            timeout_ms: 20_000,
            deny_network,
            ..SandboxRequest::default()
        };
        let mut confined_request = plain(true);
        confined_request.confinement = Some(Confinement {
            toolchain: shell_toolchain(),
            proc: false,
        });
        for (mode, run) in [
            ("direct", execute(&root, &plain(false))),
            ("network namespace", execute(&root, &plain(true))),
            ("confined", execute(&root, &confined_request)),
        ] {
            if mode == "confined" && !confined_here(&run) {
                continue;
            }
            assert_eq!(stdout(&run), "/usr/bin/sh\n", "{mode}");
            assert_eq!(run.run.toolchain.unwrap().program.path, "/usr/bin/sh");
        }
    }

    /// G189 review rounds 4-5: the rustup proxy is a toolchain dispatcher. Its `-vV` answers for
    /// whichever toolchain `+toolchain`, the environment or a staged `rust-toolchain.toml`
    /// selects, so a pin over it could accept another toolchain; it is refused as the program and
    /// as a declared binary, in every class. The toolchain's own binary still runs and pins.
    #[test]
    fn a_toolchain_dispatcher_is_refused_and_the_toolchain_binary_pins_in_every_class() {
        let root = scratch("dispatcher");
        let libraries = ["/usr/lib", "/usr/lib64", "/lib", "/lib64"]
            .into_iter()
            .filter(|p| Path::new(p).exists())
            .map(str::to_owned);
        let confinement = |paths: Vec<String>| Confinement {
            toolchain: paths
                .into_iter()
                .chain(libraries.clone())
                .map(|path| ToolchainPath { path })
                .collect(),
            proc: true,
        };
        let modes = [
            ("direct", false, false),
            ("network namespace", true, false),
            ("confined", true, true),
        ];
        let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
        let cargo_bin = home.join(".cargo/bin");
        let rustup_home = home.join(".rustup");
        let proxy = std::fs::symlink_metadata(cargo_bin.join("rustc"))
            .is_ok_and(|m| m.file_type().is_symlink());
        if proxy {
            std::fs::write(
                root.join("src/rust-toolchain.toml"),
                "[toolchain]\nchannel = \"1.90.0\"\n",
            )
            .unwrap();
            let request = |deny_network: bool, confine: bool| SandboxRequest {
                program: "rustc".into(),
                args: vec!["--version".into()],
                env: BTreeMap::from([
                    ("PATH".into(), cargo_bin.to_string_lossy().into_owned()),
                    (
                        "RUSTUP_HOME".into(),
                        rustup_home.to_string_lossy().into_owned(),
                    ),
                    ("RUSTUP_TOOLCHAIN".into(), "1.90.0".into()),
                ]),
                timeout_ms: 60_000,
                deny_network,
                confinement: confine.then(|| {
                    confinement(vec![
                        cargo_bin.to_string_lossy().into_owned(),
                        rustup_home.to_string_lossy().into_owned(),
                    ])
                }),
                ..SandboxRequest::default()
            };
            for (mode, deny_network, confine) in modes {
                let base = request(deny_network, confine);
                let mut plus = base.clone();
                plus.args = vec!["+stable".into(), "--version".into()];
                let mut file = base.clone();
                file.env.remove("RUSTUP_TOOLCHAIN");
                file.inputs = declare_inputs(&root.join("src"), &["rust-toolchain.toml"]).unwrap();
                let mut binary = confined(&root, "true");
                binary.confinement = None;
                binary.deny_network = deny_network;
                binary.toolchain_binaries = vec![cargo_bin.join("rustc").to_string_lossy().into()];
                if confine {
                    let mut paths = vec![cargo_bin.to_string_lossy().into_owned()];
                    paths.extend(["/usr/bin".to_owned()]);
                    binary.confinement = Some(confinement(paths));
                }
                let mut cargo = base.clone();
                cargo.program = "cargo".into();
                for (case, request) in [
                    ("base", base),
                    ("cargo", cargo),
                    ("+stable", plus),
                    ("rust-toolchain.toml", file),
                    ("declared binary", binary),
                ] {
                    let run = execute(&root, &request);
                    assert_eq!(
                        run.run.refusal_kind,
                        Some(RefusalKind::ToolchainProxy),
                        "{mode} {case}: {}",
                        run.run.refusal
                    );
                    assert!(
                        run.run.refusal.contains("is a link to"),
                        "{mode} {case}: the early, name-based refusal: {}",
                        run.run.refusal
                    );
                    assert!(run.stdout.is_empty(), "{mode} {case}: nothing ran");
                }
            }
        } else {
            eprintln!("no rustup proxy on this host: the dispatcher refusal was not exercised");
        }

        let Some(sysroot) = sysroot() else {
            eprintln!("no rustc on this host: the toolchain binary was not exercised");
            return;
        };
        let other = std::fs::read_dir(sysroot.parent().unwrap())
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .find(|p| p != &sysroot && p.join("bin/rustc").is_file());
        let main = sysroot.clone();
        let request = |sysroot: &Path, deny_network: bool, confine: bool| SandboxRequest {
            program: "rustc".into(),
            args: vec!["--version".into()],
            env: BTreeMap::from([("PATH".into(), format!("{}/bin", sysroot.display()))]),
            timeout_ms: 60_000,
            deny_network,
            confinement: confine.then(|| {
                confinement(
                    std::iter::once(main.as_path())
                        .chain(other.as_deref())
                        .map(|p| p.to_string_lossy().into_owned())
                        .collect(),
                )
            }),
            ..SandboxRequest::default()
        };
        for (mode, deny_network, confine) in modes {
            let first = execute(&root, &request(&sysroot, deny_network, confine));
            if confine && !confined_here(&first) {
                continue;
            }
            assert_eq!(
                first.run.exit_code,
                Some(0),
                "{mode}: {}",
                String::from_utf8_lossy(&first.stderr)
            );
            let identity = first.run.toolchain.clone().unwrap();
            assert_eq!(identity.program.path, identity.program.resolved, "{mode}");
            let version = identity.version.clone().unwrap();
            assert!(version.contains("release: "), "{mode}: {version}");
            assert!(
                stdout(&first).starts_with(version.lines().next().unwrap()),
                "{mode}"
            );
            let mut pinned = request(&sysroot, deny_network, confine);
            pinned.pinned_toolchain = Some(identity.clone());
            assert_eq!(execute(&root, &pinned).run.exit_code, Some(0), "{mode}");
            if let Some(other) = &other {
                let mut elsewhere = request(other, deny_network, confine);
                elsewhere.pinned_toolchain = Some(identity);
                let refused = execute(&root, &elsewhere);
                assert_eq!(
                    refused.run.refusal_kind,
                    Some(RefusalKind::ToolchainIdentityMismatch),
                    "{mode}: {}",
                    refused.run.refusal
                );
                assert!(refused.stdout.is_empty(), "{mode}");
            }
        }
    }

    /// Whether a process whose command line is exactly `argv` is alive on the host.
    fn alive(argv: &[&str]) -> bool {
        let wanted: Vec<u8> = argv
            .iter()
            .flat_map(|a| [a.as_bytes(), b"\0"].concat())
            .collect();
        std::fs::read_dir("/proc").unwrap().flatten().any(|entry| {
            std::fs::read(entry.path().join("cmdline")).is_ok_and(|cmdline| cmdline == wanted)
        })
    }

    /// TIME_LIMITED in the confined class: the program is pid 1 of its own pid namespace, so a
    /// descendant that left the process group (`setsid`) dies with it; in the restricted class
    /// the same descendant survives the timeout (the limit that class records).
    #[test]
    fn a_confined_run_is_killed_with_a_descendant_that_left_its_process_group() {
        let root = scratch("pidns");
        let nonce = format!("7.{}", std::process::id());
        let script = format!("setsid sleep {nonce} & sleep 5");
        let mut request = confined(&root, &script);
        request.timeout_ms = 300;
        let run = execute(&root, &request);
        if !confined_here(&run) {
            return;
        }
        assert_eq!(run.run.outcome, RunOutcome::TimedOut);
        std::thread::sleep(Duration::from_millis(300));
        assert!(
            !alive(&["sleep", &nonce]),
            "a descendant outlived the confined run"
        );
        let mut restricted = request.clone();
        restricted.confinement = None;
        restricted.env.insert("PATH".into(), "/usr/bin:/bin".into());
        let run = execute(&root, &restricted);
        assert_eq!(run.run.outcome, RunOutcome::TimedOut);
        std::thread::sleep(Duration::from_millis(300));
        assert!(
            alive(&["sleep", &nonce]),
            "the restricted class does not kill a descendant that left its group"
        );
        // The escaped descendant is bounded by construction: it sleeps `7.<pid>` seconds, under
        // eight. It is awaited through /proc rather than signalled by an external program, so the
        // test leaves nothing running and needs neither `pkill` nor `unsafe` code.
        let deadline = Instant::now() + Duration::from_secs(20);
        while alive(&["sleep", &nonce]) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(
            !alive(&["sleep", &nonce]),
            "the escaped descendant outlived its own duration"
        );
    }

    /// Inputs are staged as copies, never as links, and a link that leaves the source root is
    /// refused; a produced link is not read as an output.
    #[test]
    fn links_in_inputs_or_outputs_cannot_point_outside() {
        let root = scratch("links");
        std::os::unix::fs::symlink(root.join("outside.txt"), root.join("src/link.txt")).unwrap();
        std::os::unix::fs::symlink("a.txt", root.join("src/alias.txt")).unwrap();
        for confine in [false, true] {
            let base = if confine {
                confined(&root, "true")
            } else {
                request(&root, "true")
            };
            let mut escaping = base.clone();
            escaping.inputs.push(DeclaredInput {
                path: "link.txt".into(),
                digest: digest(b"host sentinel\n"),
            });
            let run = execute(&root, &escaping);
            assert_eq!(
                run.run.refusal_kind,
                Some(RefusalKind::InputEscapes),
                "{confine}"
            );
            let mut aliased = SandboxRequest {
                args: vec![
                    "-c".into(),
                    "if test -L alias.txt; then echo LINK; else cat alias.txt; fi; \
                     ln -s /etc/passwd out.txt"
                        .into(),
                ],
                ..base
            };
            aliased.inputs.push(DeclaredInput {
                path: "alias.txt".into(),
                digest: digest(b"declared\n"),
            });
            let run = execute(&root, &aliased);
            if confine && !confined_here(&run) {
                continue;
            }
            assert_eq!(stdout(&run), "declared\n", "{confine}: staged as a copy");
            assert_eq!(
                run.run.outputs[0].digest, None,
                "{confine}: a produced link is not an output"
            );
        }
    }

    #[test]
    fn the_toolchain_identity_is_recorded_stable_and_a_mismatched_pin_is_refused() {
        let root = scratch("identity");
        for confine in [false, true] {
            let base = if confine {
                confined(&root, "echo ran > out.txt")
            } else {
                request(&root, "echo ran > out.txt")
            };
            let first = execute(&root, &base);
            if confine && !confined_here(&first) {
                continue;
            }
            let identity = first.run.toolchain.clone().expect("recorded");
            assert_eq!(identity.program.path, "/usr/bin/sh");
            let resolved = Path::new("/usr/bin/sh").canonicalize().unwrap();
            assert_eq!(identity.program.resolved, resolved.to_string_lossy());
            assert_eq!(
                identity.program.digest,
                digest(&std::fs::read(&resolved).unwrap())
            );
            assert_eq!(identity.version, None, "not a Rust tool");
            let second = execute(&root, &base);
            assert_eq!(second.run.toolchain.as_ref(), Some(&identity), "stable");

            let mut pinned = base.clone();
            pinned.pinned_toolchain = Some(identity.clone());
            let run = execute(&root, &pinned);
            assert_eq!(run.run.outcome, RunOutcome::Exited);
            assert!(run.run.outputs[0].digest.is_some());

            let mut wrong = identity.clone();
            wrong.program.digest = digest(b"another shell");
            pinned.pinned_toolchain = Some(wrong);
            let run = execute(&root, &pinned);
            assert_eq!(run.run.outcome, RunOutcome::Refused);
            assert_eq!(
                run.run.refusal_kind,
                Some(RefusalKind::ToolchainIdentityMismatch)
            );
            assert!(
                run.run.refusal.contains("program digest"),
                "{}",
                run.run.refusal
            );
            assert_eq!(
                run.run.toolchain.as_ref(),
                Some(&identity),
                "the observed one"
            );
            assert!(run.run.outputs.is_empty() && run.stdout.is_empty());

            let mut absent = base.clone();
            absent.toolchain_binaries = vec!["/usr/bin/atlas-g189-no-such-binary".into()];
            let run = execute(&root, &absent);
            assert_eq!(
                run.run.refusal_kind,
                Some(RefusalKind::ToolchainIdentityAbsent),
                "{}",
                run.run.refusal
            );
            let mut declared = base.clone();
            declared.toolchain_binaries = vec!["/usr/bin/cat".into()];
            let run = execute(&root, &declared);
            let binaries = &run.run.toolchain.as_ref().unwrap().binaries;
            assert_eq!(binaries.len(), 1);
            assert_eq!(binaries[0].path, "/usr/bin/cat");
        }
    }

    #[test]
    fn filesystem_confinement_is_unenforced_and_nothing_runs_when_the_mechanism_is_missing() {
        let root = scratch("unavailable");
        let marker = root.join("ran");
        let mut request = confined(&root, "echo ran > \"$MARK\"");
        request
            .env
            .insert("MARK".into(), marker.to_string_lossy().into_owned());
        let missing = HostTools {
            unshare: Some(root.join("no-such-unshare")),
            ..HostTools::from_host()
        };
        let run = run_with(&request, &root.join("src"), &root.join("stage"), &missing).unwrap();
        assert_eq!(run.run.outcome, RunOutcome::Refused);
        assert_eq!(
            run.run.refusal_kind,
            Some(RefusalKind::ConfinementUnavailable)
        );
        let Enforcement::Unenforced { reason } = &run.run.isolation[&P::FilesystemConfined] else {
            panic!("confinement claimed without its mechanism");
        };
        assert!(reason.contains("`unshare` was not found at"), "{reason}");
        assert!(!marker.exists(), "nothing ran unconfined in its place");
        let Enforcement::Unenforced { reason } =
            confinement_available(&missing, &root.join("stage"))
        else {
            panic!("availability claimed without the mechanism");
        };
        assert!(reason.contains("not found"), "{reason}");
        let unnamed = HostTools {
            unshare: None,
            ..HostTools::from_host()
        };
        let run = run_with(&request, &root.join("src"), &root.join("stage"), &unnamed).unwrap();
        assert!(
            run.run.refusal.contains("not on the host PATH"),
            "{}",
            run.run.refusal
        );
        // A helper that runs but does not set the confinement up fails the probe.
        let broken = HostTools {
            mount: Some(PathBuf::from("/usr/bin/true")),
            ..HostTools::from_host()
        };
        let run = run_with(&request, &root.join("src"), &root.join("stage"), &broken).unwrap();
        assert_eq!(
            run.run.refusal_kind,
            Some(RefusalKind::ConfinementUnavailable)
        );
        assert!(!marker.exists());
        assert!(!root.join("stage").read_dir().unwrap().any(|_| true));
    }

    /// The probe is not vacuous. Each `mount` below runs the real one and then breaks the
    /// confinement in one way: the root also shows the host run directory (sentinel and stage),
    /// the stage is not bound at `/work`, or the procfs is the host's (`/proc/1/root` is the host
    /// root). Each fails the probe, and nothing runs.
    #[test]
    fn a_confinement_that_breaks_in_any_probed_way_fails_its_probe() {
        let root = scratch("broken");
        let host = HostTools::from_host();
        let (Some(mount), Some(mkdir)) = (&host.mount, &host.mkdir) else {
            eprintln!("no mount or mkdir on this host: the probe falsification did not run");
            return;
        };
        let (mount, mkdir) = (mount.display(), mkdir.display());
        let breaks = [
            (
                "shows-run-dir",
                format!(
                    "'{mount}' \"$@\" || exit $?\n\
                     if [ \"$1\" = -t ] && [ \"$2\" = tmpfs ]; then\n\
                     new=$6; run=${{new%/root}}\n\
                     '{mkdir}' -p \"$new$run\"; '{mount}' --bind \"$run\" \"$new$run\"\nfi\n"
                ),
                false,
                "a host path outside the stage was reachable",
            ),
            (
                "hides-stage",
                format!(
                    "if [ \"$1\" = --bind ] && [ \"$3\" = nosuid,nodev ]; then exit 0; fi\n\
                     exec '{mount}' \"$@\"\n"
                ),
                false,
                "the staged control file was not reachable",
            ),
            (
                "host-proc",
                format!(
                    "if [ \"$1\" = -t ] && [ \"$2\" = proc ]; then\n\
                     exec '{mount}' --rbind /proc \"$6\"\nfi\nexec '{mount}' \"$@\"\n"
                ),
                true,
                "a host path outside the stage was reachable",
            ),
        ];
        let marker = root.join("ran");
        for (name, body, proc, expected) in breaks {
            let fake = root.join(format!("mount-{name}"));
            std::fs::write(&fake, format!("#!/bin/sh\n{body}")).unwrap();
            std::fs::set_permissions(&fake, std::os::unix::fs::PermissionsExt::from_mode(0o755))
                .unwrap();
            let mut request = confined(&root, "echo ran > \"$MARK\"");
            request.confinement.as_mut().unwrap().proc = proc;
            request
                .env
                .insert("MARK".into(), marker.to_string_lossy().into_owned());
            let tools = HostTools {
                mount: Some(fake),
                ..host.clone()
            };
            let run = run_with(&request, &root.join("src"), &root.join("stage"), &tools).unwrap();
            assert_eq!(
                run.run.refusal_kind,
                Some(RefusalKind::ConfinementUnavailable),
                "{name}: {:?}",
                run.run
            );
            assert!(!run.run.isolation[&P::FilesystemConfined].is_enforced());
            assert!(!marker.exists(), "{name}: nothing ran");
            if !namespaces_available() {
                continue;
            }
            assert!(
                run.run.refusal.contains(expected),
                "{name}: {}",
                run.run.refusal
            );
            // The same request with the real `mount` is confined.
            assert!(confined_here(&execute(&root, &request)), "{name}");
        }
    }

    #[test]
    fn refused_toolchain_paths_environment_and_programs() {
        let root = scratch("declarations");
        let with = |paths: &[&str]| {
            let mut request = confined(&root, "true");
            request.confinement.as_mut().unwrap().toolchain = paths
                .iter()
                .map(|p| ToolchainPath { path: (*p).into() })
                .collect();
            execute(&root, &request)
        };
        let temp = std::env::temp_dir().to_string_lossy().into_owned();
        let stage = root.join("stage").to_string_lossy().into_owned();
        let normalized = "not absolute and normalized";
        let provided = "which the confinement provides";
        let staging = "overlaps the staging directory";
        let live = "live endpoints";
        let mut cases = vec![
            (vec!["/"], normalized),
            (vec!["/usr/bin/../../etc"], normalized),
            (vec!["/usr/bin/"], normalized),
            (vec!["usr/bin"], normalized),
            (vec!["/work"], provided),
            (vec!["/proc/self"], provided),
            (vec!["/dev"], provided),
            (vec!["/dev/shm"], provided),
            (vec!["/usr", "/usr/bin"], "lies inside"),
            (vec!["/usr/bin", "/usr/bin"], "lies inside"),
            (vec!["/nonexistent-atlas-g189"], "No such file"),
            (vec!["/usr/bin", temp.as_str()], staging),
            (vec!["/usr/bin", stage.as_str()], staging),
        ];
        for runtime in ["/run", "/var/run", "/var/tmp", "/var", "/run/user"] {
            if Path::new(runtime).exists() {
                cases.push((vec!["/usr/bin", runtime], live));
            }
        }
        for (paths, why) in cases {
            let run = with(&paths);
            assert_eq!(
                run.run.refusal_kind,
                Some(RefusalKind::ToolchainPathRefused),
                "{paths:?}: {}",
                run.run.refusal
            );
            assert!(
                run.run.refusal.contains(why),
                "{paths:?}: {}",
                run.run.refusal
            );
        }
        let libraries: Vec<&str> = ["/usr/lib", "/usr/lib64", "/lib", "/lib64"]
            .into_iter()
            .filter(|p| Path::new(p).exists())
            .collect();
        let run = with(&libraries);
        assert_eq!(
            run.run.refusal_kind,
            Some(RefusalKind::ProgramOutsideToolchain),
            "{}",
            run.run.refusal
        );
        for name in [
            "LD_PRELOAD",
            "LD_LIBRARY_PATH",
            "LD_AUDIT",
            "GCONV_PATH",
            "GLIBC_TUNABLES",
            "LOCPATH",
            "NLSPATH",
            "A-B",
            "1X",
        ] {
            let mut request = confined(&root, "true");
            request.env.insert(name.into(), "/x".into());
            assert_eq!(
                execute(&root, &request).run.refusal_kind,
                Some(RefusalKind::EnvironmentRefused),
                "{name}"
            );
        }
        let mut outside = confined(&root, "true");
        outside.toolchain_binaries = vec!["/etc/passwd".into()];
        assert_eq!(
            execute(&root, &outside).run.refusal_kind,
            Some(RefusalKind::ToolchainPathRefused)
        );
    }

    /// The host's Rust toolchain, when it has one: `rustc --print sysroot`.
    fn sysroot() -> Option<PathBuf> {
        let output = Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
            .ok()?;
        let sysroot = PathBuf::from(String::from_utf8(output.stdout).ok()?.trim());
        sysroot.join("bin/rustc").is_file().then_some(sysroot)
    }

    /// The realistic demonstration (ADR 0102): `rustc` compiles and links a declared crate inside
    /// the confinement; the same compile including a host file outside its inputs fails inside
    /// and succeeds in the restricted-subprocess class, so the confinement is what refuses it.
    #[test]
    fn a_confined_rustc_compiles_its_declared_crate_and_cannot_include_a_host_file() {
        let Some(sysroot) = sysroot() else {
            eprintln!("no rustc on this host: the demonstration did not run");
            return;
        };
        let root = scratch("rustc");
        let sentinel = root.join("outside.txt");
        std::fs::write(
            root.join("src/main.rs"),
            "fn main() { println!(\"confined\"); }\n",
        )
        .unwrap();
        std::fs::write(
            root.join("src/leak.rs"),
            format!(
                "const HOST: &str = include_str!({:?});\nfn main() {{ print!(\"{{HOST}}\"); }}\n",
                sentinel.to_string_lossy()
            ),
        )
        .unwrap();
        let sysroot_text = sysroot.to_string_lossy().into_owned();
        let mut toolchain: Vec<ToolchainPath> = [
            sysroot_text.as_str(),
            "/usr/bin",
            "/usr/lib",
            "/usr/libexec",
            "/usr/lib64",
            "/lib",
            "/lib64",
            "/etc/alternatives",
        ]
        .into_iter()
        .filter(|p| Path::new(p).exists())
        .map(|p| ToolchainPath { path: p.into() })
        .collect();
        toolchain.sort();
        let compile = |file: &str, confine: bool| SandboxRequest {
            program: "rustc".into(),
            args: vec![
                "--edition=2021".into(),
                "-Copt-level=0".into(),
                file.into(),
                "-o".into(),
                "main".into(),
            ],
            inputs: declare_inputs(&root.join("src"), &[file]).unwrap(),
            env: BTreeMap::from([
                ("PATH".into(), format!("{sysroot_text}/bin:/usr/bin")),
                (
                    "TMPDIR".into(),
                    if confine {
                        "/work".into()
                    } else {
                        std::env::temp_dir().to_string_lossy().into_owned()
                    },
                ),
            ]),
            outputs: vec!["main".into()],
            timeout_ms: 120_000,
            deny_network: true,
            confinement: confine.then(|| Confinement {
                toolchain: toolchain.clone(),
                proc: true,
            }),
            toolchain_binaries: vec!["/usr/bin/cc".into(), "/usr/bin/ld".into()],
            pinned_toolchain: None,
        };
        let built = execute(&root, &compile("main.rs", true));
        if !confined_here(&built) {
            return;
        }
        assert_eq!(
            built.run.exit_code,
            Some(0),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
        assert!(
            built.run.outputs[0].digest.is_some(),
            "the binary is recorded"
        );
        let identity = built.run.toolchain.as_ref().unwrap();
        assert!(identity.version.as_deref().unwrap().starts_with("rustc "));
        assert_eq!(identity.binaries.len(), 2);

        let leaked = execute(&root, &compile("leak.rs", true));
        assert!(confined_here(&leaked));
        let stderr = String::from_utf8_lossy(&leaked.stderr);
        assert_ne!(leaked.run.exit_code, Some(0), "{stderr}");
        assert!(stderr.contains("couldn't read"), "{stderr}");
        assert_eq!(leaked.run.outputs[0].digest, None);

        let ambient = execute(&root, &compile("leak.rs", false));
        assert_eq!(
            ambient.run.exit_code,
            Some(0),
            "outside the confinement the same compile reads the host file: {}",
            String::from_utf8_lossy(&ambient.stderr)
        );
        assert!(!ambient.run.isolation[&P::FilesystemConfined].is_enforced());
        assert_eq!(ambient.run.toolchain.as_ref(), Some(identity));
    }
}
