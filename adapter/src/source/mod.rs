use atlas_core::{
    stable_id, ArtifactDisposition, ArtifactId, ArtifactKind, ArtifactRecord, FileFact,
    InventoryReport, RepoManifest, SourceReport,
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

fn classify_non_file(root: &Path, path: &Path, kind: ArtifactKind, reason: &str) -> io::Result<ArtifactRecord> {
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

fn visit_inventory(
    root: &Path,
    dir: &Path,
    out: &mut BTreeMap<String, ArtifactRecord>,
) -> io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let file_type = entry.file_type()?;

        let record = if file_type.is_dir() {
            if ignored_directory(&name) {
                Some(policy_boundary(root, &path))
            } else {
                visit_inventory(root, &path, out)?;
                None
            }
        } else if file_type.is_file() {
            Some(classify_file(root, &path)?)
        } else if file_type.is_symlink() {
            Some(classify_non_file(
                root,
                &path,
                ArtifactKind::Symlink,
                "symlink-not-followed",
            )?)
        } else {
            Some(classify_non_file(
                root,
                &path,
                ArtifactKind::Special,
                "special-file-not-parsed",
            )?)
        };

        if let Some(record) = record {
            out.insert(record.path.clone(), record);
        }
    }
    Ok(())
}

fn inventory_from_roots(root: PathBuf, roots: Vec<PathBuf>) -> io::Result<InventoryReport> {
    let mut artifacts = BTreeMap::new();
    for path in roots {
        if path.is_dir() {
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
    let roots = declared.into_iter().map(|path| root.join(path)).collect();
    inventory_from_roots(root, roots)
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
        File::create(src.join("huge.rs"))
            .unwrap()
            .set_len(MAX_SEMANTIC_BYTES + 1)
            .unwrap();

        let report = inventory_source(&root).unwrap();

        assert_eq!(report.artifacts_total, 3);
        assert_eq!(report.dispositions.get("PARSED"), Some(&1));
        assert_eq!(report.dispositions.get("UNKNOWN"), Some(&1));
        assert_eq!(report.dispositions.get("UNSUPPORTED"), Some(&1));
        assert!(report.is_closed());

        fs::remove_dir_all(root).unwrap();
    }
}
