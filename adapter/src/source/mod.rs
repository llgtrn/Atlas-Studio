use atlas_core::{
    ArtifactDisposition, ArtifactId, ArtifactKind, ArtifactRecord, FileFact, InventoryReport,
    RepoManifest, SourceReport, stable_id,
};

pub mod frontend;

pub use frontend::{
    SourceFrontend, SourceFrontendMatch, resolve_source_frontend, source_frontends,
};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};

const MAX_SEMANTIC_BYTES: u64 = 4 * 1024 * 1024;
const BINARY_SAMPLE_BYTES: usize = 8 * 1024;

fn ignored_directory(name: &str) -> bool {
    matches!(
        name,
        ".atlas"
            | ".git"
            | ".pnpm-store"
            | "target"
            | "node_modules"
            | "dist"
            | "build"
            | ".next"
            | "coverage"
    )
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn artifact_id(path: &str) -> ArtifactId {
    ArtifactId::new(stable_id("artifact", path))
}

fn looks_binary(path: &Path) -> io::Result<bool> {
    let mut file = File::open(path)?;
    let mut sample = [0_u8; BINARY_SAMPLE_BYTES];
    let read = file.read(&mut sample)?;
    Ok(sample[..read].contains(&0))
}

fn policy_boundary(root: &Path, path: &Path) -> ArtifactRecord {
    let path = relative(root, path);
    ArtifactRecord {
        id: artifact_id(&path),
        path,
        kind: ArtifactKind::PolicyBoundary,
        bytes: 0,
        disposition: ArtifactDisposition::IgnoredByExplicitPolicy,
        language: None,
        reason: Some("generated-or-external-directory-boundary".into()),
    }
}

fn classify_file(root: &Path, path: &Path) -> io::Result<ArtifactRecord> {
    let relative_path = relative(root, path);
    let bytes = fs::metadata(path)?.len();
    let frontend = resolve_source_frontend(path);
    let language = frontend.map(|matched| matched.language.to_owned());
    let binary = looks_binary(path)?;

    let (disposition, reason) = if binary {
        (
            ArtifactDisposition::BinaryDescribed,
            Some("binary-content-described-without-semantic-parse".into()),
        )
    } else if language.is_some() && bytes <= MAX_SEMANTIC_BYTES {
        (ArtifactDisposition::Parsed, None)
    } else if language.is_some() {
        (
            ArtifactDisposition::Unsupported,
            Some(format!(
                "semantic-parser-size-limit:{}>{}",
                bytes, MAX_SEMANTIC_BYTES
            )),
        )
    } else {
        (
            ArtifactDisposition::Unknown,
            Some("no-registered-source-frontend".into()),
        )
    };

    Ok(ArtifactRecord {
        id: artifact_id(&relative_path),
        path: relative_path,
        kind: ArtifactKind::File,
        bytes,
        disposition,
        language,
        reason,
    })
}

fn classify_non_file(
    root: &Path,
    path: &Path,
    kind: ArtifactKind,
    reason: &str,
) -> io::Result<ArtifactRecord> {
    let relative_path = relative(root, path);
    Ok(ArtifactRecord {
        id: artifact_id(&relative_path),
        path: relative_path,
        kind,
        bytes: fs::symlink_metadata(path)?.len(),
        disposition: ArtifactDisposition::Unsupported,
        language: None,
        reason: Some(reason.into()),
    })
}

/// Records `path` itself as an accounted, unclassified artifact instead of propagating the
/// `io::Error` that made it inaccessible (permission denial being the common real case for either
/// a directory `read_dir` couldn't list or a file `classify_file`/`classify_non_file` couldn't
/// stat/open -- but any such failure, including one caused by the path vanishing mid-walk, is
/// handled identically). `.atlas/contracts/DEPENDENCY-CENSUS.md`'s "ingestion is not execution"
/// sibling principle applies here too: one inaccessible artifact, anywhere in a real filesystem
/// walk, must never abort accounting for every other artifact in the repository (the same
/// discipline `runtime::census::extraction::extract_semantics` already documents and applies for a
/// per-file read failure). `fs::symlink_metadata` only needs search permission on `path`'s
/// *parent* (already proven, since this entry was just read from there), not on `path` itself, so
/// it still succeeds even when `path`'s own permissions are what caused `error`; a `bytes: 0`
/// fallback covers the rarer case where `path` vanished between being listed and being stat'd.
fn record_unreadable_artifact(
    root: &Path,
    path: &Path,
    out: &mut BTreeMap<String, ArtifactRecord>,
    error: &io::Error,
) {
    let relative_path = relative(root, path);
    let bytes = fs::symlink_metadata(path)
        .map(|meta| meta.len())
        .unwrap_or(0);
    let record = ArtifactRecord {
        id: artifact_id(&relative_path),
        path: relative_path,
        kind: ArtifactKind::Special,
        bytes,
        disposition: ArtifactDisposition::Unknown,
        language: None,
        reason: Some(format!("artifact-not-accessible: {error}")),
    };
    out.insert(record.path.clone(), record);
}

/// The classification outcome for one already-listed directory entry, kept separate from
/// `visit_inventory`'s loop so every failure path -- `entry.file_type()`, `classify_file`,
/// `classify_non_file` -- funnels through one `Result`, handled uniformly by the caller instead of
/// each needing its own bespoke recovery.
fn classify_entry(
    root: &Path,
    entry: &fs::DirEntry,
    out: &mut BTreeMap<String, ArtifactRecord>,
) -> io::Result<Option<ArtifactRecord>> {
    let path = entry.path();
    let name = entry.file_name().to_string_lossy().into_owned();
    let file_type = entry.file_type()?;

    if file_type.is_dir() {
        if ignored_directory(&name) {
            Ok(Some(policy_boundary(root, &path)))
        } else {
            visit_inventory(root, &path, out)?;
            Ok(None)
        }
    } else if file_type.is_file() {
        Ok(Some(classify_file(root, &path)?))
    } else if file_type.is_symlink() {
        Ok(Some(classify_non_file(
            root,
            &path,
            ArtifactKind::Symlink,
            "symlink-not-followed",
        )?))
    } else {
        Ok(Some(classify_non_file(
            root,
            &path,
            ArtifactKind::Special,
            "special-file-not-parsed",
        )?))
    }
}

fn visit_inventory(
    root: &Path,
    dir: &Path,
    out: &mut BTreeMap<String, ArtifactRecord>,
) -> io::Result<()> {
    let read_dir = match fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(error) => {
            record_unreadable_artifact(root, dir, out, &error);
            return Ok(());
        }
    };
    let mut entries: Vec<_> = match read_dir.collect::<Result<_, _>>() {
        Ok(entries) => entries,
        Err(error) => {
            record_unreadable_artifact(root, dir, out, &error);
            return Ok(());
        }
    };
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        // Falsification: confirmed a real, non-root `chmod 000` FILE (distinct from the
        // already-fixed unreadable-DIRECTORY case) still aborted the whole walk here -- `?`
        // propagated `entry.file_type()`/`classify_file`/`classify_non_file`'s `io::Error` (e.g.
        // `fs::metadata`/`File::open` failing on a permission-denied or vanished file) straight
        // out of `visit_inventory`, exactly the same defect class, just one level of granularity
        // finer (one FILE, not one DIRECTORY). Same recovery: record it, never abort every other
        // artifact because of it.
        match classify_entry(root, &entry, out) {
            Ok(Some(record)) => {
                out.insert(record.path.clone(), record);
            }
            Ok(None) => {}
            Err(error) => {
                record_unreadable_artifact(root, &path, out, &error);
            }
        }
    }
    Ok(())
}

fn inventory_from_roots(root: PathBuf, roots: Vec<PathBuf>) -> io::Result<InventoryReport> {
    let mut artifacts = BTreeMap::new();
    for path in roots {
        // `path.is_dir()`/`path.is_file()` below both follow symlinks. `visit_inventory`'s own
        // recursive walk correctly never follows a symlinked subdirectory it encounters (its
        // `DirEntry::file_type()` does not follow links), but a ROOT passed into this function
        // (each manifest-declared root, for `inventory_declared_source`) never went through that
        // same check -- a declared root that is itself, on disk, a symlink pointing outside the
        // repository (e.g. `source_roots = ["vendor"]` where `vendor` is a symlink) would be
        // silently followed and fully walked, reading real file content from outside the intended
        // boundary. `declared_root_is_contained`'s lexical string check cannot catch this: the
        // string "vendor" is perfectly ordinary and contained; only the filesystem knows it is a
        // symlink. `fs::symlink_metadata` (unlike `Path::is_dir`/`is_file`) never follows the
        // final component, so this check is safe to perform before deciding how to treat `path`.
        let is_symlink = fs::symlink_metadata(&path)
            .map(|meta| meta.file_type().is_symlink())
            .unwrap_or(false);
        if is_symlink {
            let record =
                classify_non_file(&root, &path, ArtifactKind::Symlink, "symlink-not-followed")?;
            artifacts.insert(record.path.clone(), record);
        } else if path.is_dir() {
            visit_inventory(&root, &path, &mut artifacts)?;
        } else if path.is_file() {
            let record = classify_file(&root, &path)?;
            artifacts.insert(record.path.clone(), record);
        } else if path.exists() {
            let record = classify_non_file(
                &root,
                &path,
                ArtifactKind::Special,
                "declared-root-is-not-a-regular-file-or-directory",
            )?;
            artifacts.insert(record.path.clone(), record);
        }
    }

    Ok(InventoryReport::new(
        root.to_string_lossy().into_owned(),
        artifacts.into_values().collect(),
    ))
}

pub fn inventory_source(root: impl AsRef<Path>) -> io::Result<InventoryReport> {
    let root = root.as_ref().canonicalize()?;
    inventory_from_roots(root.clone(), vec![root])
}

/// Whether a manifest-declared root, taken as a standalone string, could ever resolve outside
/// the repository root it is about to be joined against. Checked purely lexically, without
/// touching the filesystem (the declared root may not exist yet, and must never be
/// `canonicalize`d before this check -- that would itself follow symlinks/`..` on disk).
///
/// Rejects any absolute path: `Path::join` returns an absolute joinee verbatim, silently
/// discarding the intended root entirely (`root.join("/etc")` == `/etc`). Also rejects any path
/// whose `..` components would net-escape above where it started (e.g. `"../../etc"` against a
/// shallow root). A `.atlas/repo.toml` manifest is not necessarily pre-admitted, trusted config:
/// `ADL-TO-ATLAS.md`'s candidate reconciliation path and this session's own donor-corpus census
/// sweeps can both point `inventory_declared_source` at externally-authored trees, so a hostile
/// or careless `source_roots`/`frontend_roots`/`test_roots` entry must never cause a filesystem
/// walk (and file-content read, via `looks_binary`) outside the intended repository boundary.
fn declared_root_is_contained(declared: &str) -> bool {
    let declared_path = Path::new(declared);
    if declared_path.is_absolute() {
        return false;
    }
    let mut depth: i64 = 0;
    for component in declared_path.components() {
        match component {
            std::path::Component::Normal(_) => depth += 1,
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => return false,
        }
    }
    true
}

fn rejected_declared_root(declared: &str) -> ArtifactRecord {
    ArtifactRecord {
        id: artifact_id(declared),
        path: declared.to_owned(),
        kind: ArtifactKind::PolicyBoundary,
        bytes: 0,
        disposition: ArtifactDisposition::IgnoredByExplicitPolicy,
        language: None,
        reason: Some("declared-root-escapes-repository-boundary".into()),
    }
}

pub fn inventory_declared_source(
    root: impl AsRef<Path>,
    manifest: &RepoManifest,
) -> io::Result<InventoryReport> {
    let root = root.as_ref().canonicalize()?;
    let mut declared = manifest.source_roots.clone();
    declared.extend(manifest.frontend_roots.clone());
    declared.extend(manifest.test_roots.clone());
    declared.sort();
    declared.dedup();

    let mut rejected = BTreeMap::new();
    let mut roots = Vec::new();
    for path in &declared {
        if declared_root_is_contained(path) {
            roots.push(root.join(path));
        } else {
            let record = rejected_declared_root(path);
            rejected.insert(record.path.clone(), record);
        }
    }

    let report = inventory_from_roots(root, roots)?;
    if rejected.is_empty() {
        return Ok(report);
    }
    rejected.extend(report.artifacts.into_iter().map(|a| (a.path.clone(), a)));
    Ok(InventoryReport::new(
        report.root,
        rejected.into_values().collect(),
    ))
}

pub fn source_report_from_inventory(inventory: &InventoryReport) -> SourceReport {
    let mut files = inventory
        .artifacts
        .iter()
        .filter(|artifact| artifact.disposition == ArtifactDisposition::Parsed)
        .filter_map(|artifact| {
            artifact.language.as_ref().map(|language| FileFact {
                path: artifact.path.clone(),
                language: language.clone(),
                bytes: artifact.bytes,
            })
        })
        .collect::<Vec<_>>();
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let mut languages = BTreeMap::new();
    for file in &files {
        *languages.entry(file.language.clone()).or_insert(0) += 1;
    }

    SourceReport {
        schema: "atlas.systemizer.source-report.v2".into(),
        root: inventory.root.clone(),
        files_total: files.len(),
        languages,
        files,
    }
}

pub fn scan_source(root: impl AsRef<Path>) -> io::Result<SourceReport> {
    let inventory = inventory_source(root)?;
    Ok(source_report_from_inventory(&inventory))
}

pub fn scan_declared_source(
    root: impl AsRef<Path>,
    manifest: &RepoManifest,
) -> io::Result<SourceReport> {
    let inventory = inventory_declared_source(root, manifest)?;
    Ok(source_report_from_inventory(&inventory))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("atlas-inventory-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn inventory_accounts_for_unknown_and_oversized_files() {
        let root = scratch_root();
        let src = root.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("ok.rs"), "fn main() {}\n").unwrap();
        fs::write(src.join("unknown.xyz"), "semantic depth unavailable\n").unwrap();
        // `set_len` alone would leave a sparse file whose unwritten region reads back as
        // null bytes, which `looks_binary` would (correctly) classify as BinaryDescribed
        // instead of exercising the oversized-text path this test targets.
        let oversized_text = vec![b'a'; (MAX_SEMANTIC_BYTES + 1) as usize];
        fs::write(src.join("huge.rs"), &oversized_text).unwrap();

        let report = inventory_source(&root).unwrap();

        assert_eq!(report.artifacts_total, 3);
        assert_eq!(report.dispositions.get("PARSED"), Some(&1));
        assert_eq!(report.dispositions.get("UNKNOWN"), Some(&1));
        assert_eq!(report.dispositions.get("UNSUPPORTED"), Some(&1));
        assert!(report.is_closed());

        fs::remove_dir_all(root).unwrap();
    }

    fn manifest_with_declared_roots(source_roots: Vec<&str>) -> RepoManifest {
        RepoManifest {
            schema: "atlas.repo.v2".into(),
            repo: "org/repo".into(),
            system_kind: "SYSTEM_INVENTION_FORGE".into(),
            backend_language: "rust".into(),
            frontend_language: "typescript".into(),
            coding_requires_docs_gate: true,
            graph_before_code_required: true,
            exact_base_sha_required: true,
            single_repository_target_required: true,
            knowledge_root: ".atlas".into(),
            temporary_root: ".atlas/temporary".into(),
            provenance_root: ".atlas/provenance".into(),
            license_root: ".atlas/licenses".into(),
            source_roots: source_roots.into_iter().map(String::from).collect(),
            backend_roots: Vec::new(),
            frontend_roots: Vec::new(),
            test_roots: Vec::new(),
        }
    }

    #[test]
    fn a_declared_root_escaping_via_dotdot_is_rejected_not_walked() {
        let root = scratch_root();
        fs::create_dir_all(&root).unwrap();
        // A sibling of `root` that a `../` escape would reach if it were ever walked.
        let outside = root.parent().unwrap().join(format!(
            "atlas-inventory-outside-{}",
            root.file_name().unwrap().to_string_lossy()
        ));
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.rs"), "fn leaked() {}\n").unwrap();

        let manifest = manifest_with_declared_roots(vec![&format!(
            "../{}",
            outside.file_name().unwrap().to_string_lossy()
        )]);
        let report = inventory_declared_source(&root, &manifest).unwrap();

        assert!(
            report
                .artifacts
                .iter()
                .all(|artifact| !artifact.path.contains("secret.rs")),
            "an escaping declared root must never be walked: {:?}",
            report.artifacts
        );
        assert_eq!(
            report.dispositions.get("IGNORED_BY_EXPLICIT_POLICY"),
            Some(&1)
        );
        assert!(report.is_closed());

        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&outside).unwrap();
    }

    #[test]
    fn a_declared_root_that_is_an_absolute_path_is_rejected_not_walked() {
        let root = scratch_root();
        fs::create_dir_all(&root).unwrap();

        let manifest = manifest_with_declared_roots(vec!["/etc"]);
        let report = inventory_declared_source(&root, &manifest).unwrap();

        assert_eq!(report.artifacts_total, 1);
        assert_eq!(
            report.artifacts[0].disposition,
            ArtifactDisposition::IgnoredByExplicitPolicy
        );
        assert_eq!(report.artifacts[0].path, "/etc");
        assert!(report.is_closed());

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn an_ordinary_declared_root_is_still_walked_normally() {
        let root = scratch_root();
        let src = root.join("core");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("ok.rs"), "fn main() {}\n").unwrap();

        let manifest = manifest_with_declared_roots(vec!["core"]);
        let report = inventory_declared_source(&root, &manifest).unwrap();

        assert_eq!(report.dispositions.get("PARSED"), Some(&1));
        assert!(
            !report
                .dispositions
                .contains_key("IGNORED_BY_EXPLICIT_POLICY")
        );
        assert!(report.is_closed());

        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_declared_root_that_is_itself_a_symlink_escaping_the_repository_is_never_followed() {
        // `declared_root_is_contained` is a purely lexical check on the manifest's own string --
        // it cannot see that a perfectly ordinary-looking name like "vendor" is, ON DISK, a
        // symlink to somewhere outside the repository. `inventory_from_roots` previously decided
        // whether to walk a declared root via `path.is_dir()`/`path.is_file()`, both of which
        // follow symlinks -- so a declared root that was itself a symlink pointing outside the
        // repository would be silently walked, reading real file content from outside the
        // intended boundary, with no defense at all (distinct from the `..`/absolute-path string
        // escape fixed separately).
        let root = scratch_root();
        fs::create_dir_all(&root).unwrap();
        let outside = root.parent().unwrap().join(format!(
            "atlas-inventory-symlink-outside-{}",
            root.file_name().unwrap().to_string_lossy()
        ));
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.rs"), "fn leaked() {}\n").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("vendor")).unwrap();

        let manifest = manifest_with_declared_roots(vec!["vendor"]);
        let report = inventory_declared_source(&root, &manifest).unwrap();

        assert!(
            report
                .artifacts
                .iter()
                .all(|artifact| !artifact.path.contains("secret.rs")),
            "a declared root that is itself a symlink escaping the repository must never be \
             walked: {:?}",
            report.artifacts
        );
        assert!(report.is_closed());

        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&outside).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_cycle_inside_the_repository_terminates_and_is_recorded_not_followed() {
        // A distinct DoS vector from the path-escape cases above: a symlink that creates a CYCLE
        // reachable entirely from within the repository (never escaping it), which could in
        // principle cause unbounded/infinite directory-walk recursion if symlinks were ever
        // followed mid-walk. `visit_inventory`'s recursive walk uses `DirEntry::file_type()`
        // (never follows a symlink) rather than `Path::is_dir()` (follows), so a cycle should be
        // caught the same way any other symlink is -- recorded as `Symlink`/`symlink-not-followed`
        // and never recursed into. This test's own completion is the proof of termination: if this
        // guard were ever broken, this call would hang indefinitely rather than return.
        let root = scratch_root();
        let src = root.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("ok.rs"), "fn main() {}\n").unwrap();
        // `self_loop -> .` inside `src/` -- the directory entry points back at its own parent.
        std::os::unix::fs::symlink(".", src.join("self_loop")).unwrap();

        let report = inventory_source(&root).unwrap();

        assert_eq!(report.dispositions.get("PARSED"), Some(&1));
        let symlink_artifacts = report
            .artifacts
            .iter()
            .filter(|artifact| artifact.kind == ArtifactKind::Symlink)
            .count();
        assert_eq!(symlink_artifacts, 1, "artifacts: {:?}", report.artifacts);
        assert!(report.is_closed());

        fs::remove_dir_all(root).unwrap();
    }

    // Whether this test process runs as root (uid 0). Root bypasses discretionary directory
    // permission checks entirely on Linux, so a `chmod 000` directory remains fully readable to
    // it and this test's crash-vs-graceful assertion cannot be exercised meaningfully -- reading
    // `/proc/self/status` avoids adding a new dependency (e.g. `libc::geteuid`) for a single
    // environment probe.
    #[cfg(unix)]
    fn running_as_root() -> bool {
        fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|status| {
                status
                    .lines()
                    .find_map(|line| line.strip_prefix("Uid:"))
                    .and_then(|rest| rest.split_whitespace().next())
                    .map(|uid| uid == "0")
            })
            .unwrap_or(false)
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_subdirectory_is_recorded_not_walked_and_does_not_abort_the_rest_of_the_tree() {
        // Falsification: confirmed against the unfixed code via a real, isolated, non-root
        // process (`su ubuntu -c '.../atlas-systemizer code analyze --root ...'`) before writing
        // this fix -- a permission-denied subdirectory anywhere in the source tree aborted the
        // ENTIRE walk with "Permission denied (os error 13)" and zero output, exactly the same
        // defect class `visit_adl_sources`/`audit_repository`/`visit_docs` had for a non-UTF-8
        // file read (fixed earlier this session) -- just triggered by `fs::read_dir` failing
        // instead of `fs::read_to_string`. This process itself typically runs as root in CI
        // sandboxes, where root bypasses the permission check being tested (`fs::read_dir` on a
        // `chmod 000` directory succeeds for root) -- skip rather than falsely pass in that case;
        // the real-process verification above is the authoritative evidence for this fix.
        if running_as_root() {
            eprintln!(
                "skipping an_unreadable_subdirectory_is_recorded_not_walked_and_does_not_abort_the_rest_of_the_tree: \
                 running as root, which bypasses the permission check this test exercises"
            );
            return;
        }

        let root = scratch_root();
        let src = root.join("src");
        let locked = src.join("locked");
        fs::create_dir_all(&locked).unwrap();
        fs::write(src.join("ok.rs"), "fn main() {}\n").unwrap();
        fs::write(locked.join("secret.rs"), "fn hidden() {}\n").unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

        let report = inventory_source(&root).unwrap();

        assert_eq!(
            report.dispositions.get("PARSED"),
            Some(&1),
            "the readable sibling file must still be inventoried: {:?}",
            report.artifacts
        );
        let locked_artifact = report
            .artifacts
            .iter()
            .find(|artifact| artifact.path == "src/locked")
            .unwrap_or_else(|| {
                panic!(
                    "locked directory must still be accounted for, never silently dropped: {:?}",
                    report.artifacts
                )
            });
        assert_eq!(locked_artifact.disposition, ArtifactDisposition::Unknown);
        assert!(report.is_closed());

        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_file_is_recorded_not_classified_and_does_not_abort_the_rest_of_the_tree() {
        // Same defect class as the unreadable-directory case above, one level of granularity
        // finer: confirmed against the unfixed code via `su ubuntu -c '.../atlas-systemizer
        // code analyze --root ...'` that a single `chmod 000` FILE (not directory) still aborted
        // the whole walk with "Permission denied (os error 13)" and zero output -- `classify_file`'s
        // `fs::metadata`/`File::open` calls were still `?`-propagated out of `visit_inventory`
        // even after the directory-level fix. Same root-detection skip as above; the real-process
        // verification is the authoritative evidence for this fix.
        if running_as_root() {
            eprintln!(
                "skipping an_unreadable_file_is_recorded_not_classified_and_does_not_abort_the_rest_of_the_tree: \
                 running as root, which bypasses the permission check this test exercises"
            );
            return;
        }

        let root = scratch_root();
        let src = root.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("ok.rs"), "fn main() {}\n").unwrap();
        let locked = src.join("locked.rs");
        fs::write(&locked, "fn hidden() {}\n").unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

        let report = inventory_source(&root).unwrap();

        assert_eq!(
            report.dispositions.get("PARSED"),
            Some(&1),
            "the readable sibling file must still be inventoried: {:?}",
            report.artifacts
        );
        let locked_artifact = report
            .artifacts
            .iter()
            .find(|artifact| artifact.path == "src/locked.rs")
            .unwrap_or_else(|| {
                panic!(
                    "the unreadable file must still be accounted for, never silently dropped: {:?}",
                    report.artifacts
                )
            });
        assert_eq!(locked_artifact.disposition, ArtifactDisposition::Unknown);
        assert!(report.is_closed());

        fs::set_permissions(&locked, fs::Permissions::from_mode(0o644)).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
