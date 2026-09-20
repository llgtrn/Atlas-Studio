//! Atlas adapters for external repository mechanics.

use atlas_core::{DocsReport, FileFact, RepoAudit, RepoManifest, SourceReport, validate_manifest};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

fn language(path: &Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "rs" => Some("rust"),
        "ts" | "tsx" => Some("typescript"),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript"),
        "md" => Some("markdown"),
        "toml" => Some("toml"),
        "json" => Some("json"),
        "yaml" | "yml" => Some("yaml"),
        _ => None,
    }
}

fn ignored(name: &str) -> bool {
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

fn visit_source(root: &Path, dir: &Path, out: &mut Vec<FileFact>) -> io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type()?.is_dir() {
            if !ignored(&name) {
                visit_source(root, &path, out)?;
            }
            continue;
        }
        if !entry.file_type()?.is_file() {
            continue;
        }
        let Some(lang) = language(&path) else {
            continue;
        };
        let bytes = entry.metadata()?.len();
        if bytes > 4 * 1024 * 1024 {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        out.push(FileFact {
            path: relative,
            language: lang.into(),
            bytes,
        });
    }
    Ok(())
}

pub fn scan_source(root: impl AsRef<Path>) -> io::Result<SourceReport> {
    let root: PathBuf = root.as_ref().canonicalize()?;
    let mut files = Vec::new();
    visit_source(&root, &root, &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut languages = BTreeMap::new();
    for file in &files {
        *languages.entry(file.language.clone()).or_insert(0) += 1;
    }
    Ok(SourceReport {
        schema: "atlas.systemizer.source-report.v1".into(),
        root: root.to_string_lossy().into_owned(),
        files_total: files.len(),
        languages,
        files,
    })
}

fn value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|raw| {
        let line = raw.split('#').next().unwrap_or_default().trim();
        let (left, right) = line.split_once('=')?;
        if left.trim() != key {
            return None;
        }
        right
            .trim()
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .map(ToOwned::to_owned)
    })
}

fn bool_value(text: &str, key: &str) -> Option<bool> {
    text.lines().find_map(|raw| {
        let line = raw.split('#').next().unwrap_or_default().trim();
        let (left, right) = line.split_once('=')?;
        if left.trim() != key {
            return None;
        }
        match right.trim() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    })
}

fn array_value(text: &str, section: &str, key: &str) -> Option<Vec<String>> {
    let mut active = "";
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or_default().trim();
        if line.starts_with('[') && line.ends_with(']') {
            active = line.trim_start_matches('[').trim_end_matches(']').trim();
            continue;
        }
        if active != section {
            continue;
        }
        let (left, right) = line.split_once('=')?;
        if left.trim() != key {
            continue;
        }
        let inner = right.trim().strip_prefix('[')?.strip_suffix(']')?;
        return Some(
            inner
                .split(',')
                .filter_map(|item| {
                    let item = item.trim();
                    item.strip_prefix('"')
                        .and_then(|v| v.strip_suffix('"'))
                        .map(ToOwned::to_owned)
                })
                .collect(),
        );
    }
    None
}

pub fn parse_repo_manifest(text: &str) -> Result<RepoManifest, Vec<String>> {
    let mut errors = Vec::new();
    macro_rules! string {
        ($key:literal) => {
            value(text, $key).unwrap_or_else(|| {
                errors.push(format!("missing_or_invalid_string:{}", $key));
                String::new()
            })
        };
    }
    macro_rules! boolean {
        ($key:literal) => {
            bool_value(text, $key).unwrap_or_else(|| {
                errors.push(format!("missing_or_invalid_bool:{}", $key));
                false
            })
        };
    }
    macro_rules! array {
        ($section:literal, $key:literal) => {
            array_value(text, $section, $key).unwrap_or_else(|| {
                errors.push(format!("missing_or_invalid_array:{}.{}", $section, $key));
                Vec::new()
            })
        };
    }
    let manifest = RepoManifest {
        schema: string!("schema"),
        repo: string!("repo"),
        system_kind: string!("system_kind"),
        backend_language: string!("backend_language"),
        frontend_language: string!("frontend_language"),
        coding_requires_docs_gate: boolean!("coding_requires_docs_gate"),
        graph_before_code_required: boolean!("graph_before_code_required"),
        exact_base_sha_required: boolean!("exact_base_sha_required"),
        single_repository_target_required: boolean!("single_repository_target_required"),
        knowledge_root: string!("knowledge_root"),
        temporary_root: string!("temporary_root"),
        provenance_root: string!("provenance_root"),
        license_root: string!("license_root"),
        source_roots: array!("code", "source_roots"),
        backend_roots: array!("code", "backend_roots"),
        frontend_roots: array!("code", "frontend_roots"),
        test_roots: array!("code", "test_roots"),
    };
    if errors.is_empty() {
        Ok(manifest)
    } else {
        Err(errors)
    }
}

pub fn audit_repository(root: impl AsRef<Path>) -> io::Result<RepoAudit> {
    let root = root.as_ref();
    let atlas_root = root.join(".atlas");
    let repo_manifest = atlas_root.join("repo.toml");
    let mut missing_required_roles = Vec::new();
    let mut missing_mapped_paths = Vec::new();
    if !atlas_root.is_dir() {
        missing_required_roles.push("canonical_knowledge_root".into());
        missing_mapped_paths.push(".atlas".into());
    }
    if !repo_manifest.is_file() {
        missing_required_roles.push("repository_manifest".into());
        missing_mapped_paths.push(".atlas/repo.toml".into());
    }

    let mut archetype = "UNKNOWN".to_owned();
    let mut manifest = None;
    let mut policy_violations = Vec::new();
    if repo_manifest.is_file() {
        let text = fs::read_to_string(&repo_manifest)?;
        match parse_repo_manifest(&text) {
            Ok(parsed) => {
                archetype = parsed.system_kind.clone();
                policy_violations = validate_manifest(&parsed);
                manifest = Some(parsed);
            }
            Err(errors) => missing_required_roles.extend(errors),
        }
    }

    let forbidden_roots_present = ["docs", "migration", "oss", "donors", "references"]
        .iter()
        .filter(|path| root.join(path).exists())
        .map(|path| (*path).to_owned())
        .collect::<Vec<_>>();
    let manifest_ready = manifest.is_some() && policy_violations.is_empty();
    let ready = missing_required_roles.is_empty()
        && missing_mapped_paths.is_empty()
        && forbidden_roots_present.is_empty()
        && manifest_ready;
    Ok(RepoAudit {
        schema: "atlas.systemizer.repo-audit.v6".into(),
        archetype,
        manifest,
        manifest_ready,
        policy_violations,
        missing_required_roles,
        missing_mapped_paths,
        forbidden_roots_present,
        ready,
    })
}

pub fn audit_docs(root: impl AsRef<Path>) -> io::Result<DocsReport> {
    let root = root.as_ref().canonicalize()?;
    let required = [
        "README.md",
        "INDEX.md",
        "TEMPLATE.md",
        "architecture/README.md",
        "architecture/SYSTEM.md",
        "architecture/constitution/NORTH-STAR.md",
        "blueprints/SYSTEM-BLUEPRINT.md",
        "contracts/SYSTEM-CONTRACT.md",
        "decisions/README.md",
        "guides/DEVELOPMENT.md",
        "references/README.md",
    ];
    let required_control_docs_missing = required
        .iter()
        .filter(|path| !root.join(path).is_file())
        .map(|path| (*path).to_owned())
        .collect::<Vec<_>>();
    let mut documents_total = 0;
    let mut canonical_frontmatter_total = 0;
    let mut missing_frontmatter = Vec::new();
    visit_docs(
        &root,
        &root,
        &mut documents_total,
        &mut canonical_frontmatter_total,
        &mut missing_frontmatter,
    )?;
    let hard_violations_total = required_control_docs_missing.len() + missing_frontmatter.len();
    Ok(DocsReport {
        schema: "atlas.systemizer.docs-report.v2".into(),
        standard: "atlas.docs.v1".into(),
        root: root.to_string_lossy().into_owned(),
        gate_ready: hard_violations_total == 0,
        hard_violations_total,
        documents_total,
        canonical_frontmatter_total,
        required_control_docs_missing,
        missing_frontmatter,
    })
}

fn visit_docs(
    root: &Path,
    dir: &Path,
    documents_total: &mut usize,
    canonical_frontmatter_total: &mut usize,
    missing_frontmatter: &mut Vec<String>,
) -> io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if matches!(name.as_str(), "temporary" | "provenance" | "licenses") {
                continue;
            }
            visit_docs(
                root,
                &path,
                documents_total,
                canonical_frontmatter_total,
                missing_frontmatter,
            )?;
            continue;
        }
        if path.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        *documents_total += 1;
        let text = fs::read_to_string(&path)?;
        if text.starts_with("---\n") {
            *canonical_frontmatter_total += 1;
        } else {
            missing_frontmatter.push(
                path.strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_atlas_manifest_policy_fields() {
        let manifest = parse_repo_manifest(
            r#"schema = "atlas.repo.v2"
repo = "org/repo"
system_kind = "SYSTEM_INVENTION_FORGE"
backend_language = "rust"
frontend_language = "typescript"
coding_requires_docs_gate = true
graph_before_code_required = true
exact_base_sha_required = true
single_repository_target_required = true
knowledge_root = ".atlas"
temporary_root = ".atlas/temporary"
provenance_root = ".atlas/provenance"
license_root = ".atlas/licenses"

[code]
source_roots = ["core"]
backend_roots = ["core"]
frontend_roots = ["apps/ui"]
test_roots = ["tests"]
"#,
        )
        .unwrap();
        assert!(validate_manifest(&manifest).is_empty());
    }
}
