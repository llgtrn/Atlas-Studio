//! Atlas adapters for external repository mechanics.

use atlas_core::{AdlSource, DocsReport, DocumentFact, RepoAudit, RepoManifest, validate_manifest};
use std::{collections::BTreeMap, fs, io, path::Path};

pub mod source;
pub mod vcs;

pub use source::{
    SourceFrontend, SourceFrontendMatch, inventory_declared_source, inventory_source,
    resolve_source_frontend, scan_declared_source, scan_source, source_frontends,
    source_report_from_inventory,
};
pub use vcs::snapshot_git;

pub fn read_adl_sources(root: impl AsRef<Path>) -> io::Result<Vec<AdlSource>> {
    let root = root.as_ref().canonicalize()?;
    let declared = root.join(".atlas").join("declared");
    let mut sources = Vec::new();
    if declared.is_dir() {
        visit_adl_sources(&root, &declared, &mut sources)?;
    }
    sources.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(sources)
}

fn visit_adl_sources(root: &Path, dir: &Path, out: &mut Vec<AdlSource>) -> io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            visit_adl_sources(root, &path, out)?;
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("adl") {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        out.push(AdlSource {
            path: relative,
            text: fs::read_to_string(path)?,
        });
    }
    Ok(())
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
        "blueprints/PHYSICAL-REFOUNDATION.md",
        "contracts/SYSTEM-CONTRACT.md",
        "contracts/SEMANTIC-FACTS.md",
        "contracts/SEMANTIC-EXTRACTION.md",
        "contracts/NORMALIZATION.md",
        "contracts/CENSUS-COMPLETENESS.md",
        "contracts/CENSUS-CERTIFICATE.md",
        "decisions/README.md",
        "decisions/0001-one-normalized-semantic-path.md",
        "decisions/0002-epistemic-status-model.md",
        "roadmap/ROADMAP.md",
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
    let mut documents = Vec::new();
    visit_docs(
        &root,
        &root,
        &mut documents_total,
        &mut canonical_frontmatter_total,
        &mut missing_frontmatter,
        &mut documents,
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
        documents,
    })
}

fn has_frontmatter(text: &str) -> bool {
    text.starts_with("---\n") || text.starts_with("---\r\n")
}

fn frontmatter(text: &str) -> BTreeMap<String, String> {
    if !has_frontmatter(text) {
        return BTreeMap::new();
    }
    let body = if let Some(rest) = text.strip_prefix("---\r\n") {
        rest
    } else {
        text.strip_prefix("---\n").unwrap_or(text)
    };
    let Some(end) = body.find("\n---") else {
        return BTreeMap::new();
    };
    let mut fields = BTreeMap::new();
    for line in body[..end].lines() {
        if let Some((key, value)) = line.split_once(':') {
            fields.insert(
                key.trim().to_owned(),
                value.trim().trim_matches('"').to_owned(),
            );
        }
    }
    fields
}

fn markdown_title(text: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.strip_prefix("# ").map(|title| title.trim().to_owned()))
}

fn markdown_headings(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            if !trimmed.starts_with('#') {
                return None;
            }
            let heading = trimmed.trim_start_matches('#').trim();
            if heading.is_empty() {
                None
            } else {
                Some(heading.to_owned())
            }
        })
        .collect()
}

fn path_references(text: &str) -> Vec<String> {
    let mut references = Vec::new();
    for root in ["core/", "runtime/", "adapter/", "apps/studio/"] {
        let mut cursor = 0;
        while let Some(offset) = text[cursor..].find(root) {
            let start = cursor + offset;
            let tail = &text[start..];
            let end = tail
                .find(|ch: char| {
                    !(ch.is_ascii_alphanumeric() || matches!(ch, '/' | '\\' | '.' | '_' | '-'))
                })
                .unwrap_or(tail.len());
            let candidate = tail[..end].replace('\\', "/");
            if candidate.contains('.') && !references.contains(&candidate) {
                references.push(candidate);
            }
            cursor = start + end.max(root.len());
        }
    }
    references
}

fn visit_docs(
    root: &Path,
    dir: &Path,
    documents_total: &mut usize,
    canonical_frontmatter_total: &mut usize,
    missing_frontmatter: &mut Vec<String>,
    documents: &mut Vec<DocumentFact>,
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
                documents,
            )?;
            continue;
        }
        if path.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        *documents_total += 1;
        let text = fs::read_to_string(&path)?;
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let meta = frontmatter(&text);
        if has_frontmatter(&text) {
            *canonical_frontmatter_total += 1;
        } else {
            missing_frontmatter.push(relative.clone());
        }
        documents.push(DocumentFact {
            path: format!(".atlas/{relative}"),
            id: meta.get("id").cloned(),
            kind: meta.get("type").cloned(),
            status: meta.get("status").cloned(),
            canonical: meta.get("canonical").map(String::as_str) == Some("true"),
            title: markdown_title(&text),
            headings: markdown_headings(&text),
            references: path_references(&text),
        });
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
frontend_roots = ["apps/studio"]
test_roots = ["core/tests", "runtime/tests", "adapter/tests", "apps/studio/src"]
"#,
        )
        .unwrap();
        assert!(validate_manifest(&manifest).is_empty());
    }
}
