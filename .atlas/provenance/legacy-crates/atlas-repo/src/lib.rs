use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoManifest {
    pub schema: String,
    pub repo: String,
    pub system_kind: String,
    pub backend_language: String,
    pub frontend_language: String,
    pub coding_requires_docs_gate: bool,
    pub graph_before_code_required: bool,
    pub exact_base_sha_required: bool,
    pub single_repository_target_required: bool,
    pub knowledge_root: String,
    pub temporary_root: String,
    pub provenance_root: String,
    pub license_root: String,
    pub source_roots: Vec<String>,
    pub backend_roots: Vec<String>,
    pub frontend_roots: Vec<String>,
    pub test_roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoAudit {
    pub schema: String,
    pub archetype: String,
    pub manifest: Option<RepoManifest>,
    pub manifest_ready: bool,
    pub policy_violations: Vec<String>,
    pub missing_required_roles: Vec<String>,
    pub missing_mapped_paths: Vec<String>,
    pub forbidden_roots_present: Vec<String>,
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TomlLite {
    values: BTreeMap<String, String>,
}

fn parse_toml_lite(text: &str) -> TomlLite {
    let mut section = String::new();
    let mut values = BTreeMap::new();
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line.trim_start_matches('[').trim_end_matches(']').trim().to_owned();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = key.trim();
        let full_key = if section.is_empty() { key.to_owned() } else { format!("{section}.{key}") };
        values.insert(full_key, value.trim().to_owned());
    }
    TomlLite { values }
}

fn quoted_value(table: &TomlLite, key: &str) -> Option<String> {
    table.values.get(key).and_then(|value| {
        value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .map(ToOwned::to_owned)
    })
}

fn bool_value(table: &TomlLite, key: &str) -> Option<bool> {
    match table.values.get(key).map(String::as_str) {
        Some("true") => Some(true),
        Some("false") => Some(false),
        _ => None,
    }
}

fn string_array(table: &TomlLite, key: &str) -> Option<Vec<String>> {
    let value = table.values.get(key)?.trim();
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    let mut values = Vec::new();
    for raw in inner.split(',') {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let item = line.strip_prefix('"')?.strip_suffix('"')?;
        values.push(item.to_owned());
    }
    Some(values)
}

pub fn parse_repo_manifest(text: &str) -> Result<RepoManifest, Vec<String>> {
    let table = parse_toml_lite(text);
    let mut errors = Vec::new();

    macro_rules! required_string {
        ($key:literal) => {
            match quoted_value(&table, $key) {
                Some(value) if !value.trim().is_empty() => value,
                _ => {
                    errors.push(format!("missing_or_invalid_string:{}", $key));
                    String::new()
                }
            }
        };
    }
    macro_rules! required_bool {
        ($key:literal) => {
            match bool_value(&table, $key) {
                Some(value) => value,
                None => {
                    errors.push(format!("missing_or_invalid_bool:{}", $key));
                    false
                }
            }
        };
    }
    macro_rules! required_array {
        ($key:literal) => {
            match string_array(&table, $key) {
                Some(value) if !value.is_empty() => value,
                _ => {
                    errors.push(format!("missing_or_invalid_array:{}", $key));
                    Vec::new()
                }
            }
        };
    }

    let manifest = RepoManifest {
        schema: required_string!("schema"),
        repo: required_string!("repo"),
        system_kind: required_string!("system_kind"),
        backend_language: required_string!("backend_language"),
        frontend_language: required_string!("frontend_language"),
        coding_requires_docs_gate: required_bool!("coding_requires_docs_gate"),
        graph_before_code_required: required_bool!("graph_before_code_required"),
        exact_base_sha_required: required_bool!("exact_base_sha_required"),
        single_repository_target_required: required_bool!("single_repository_target_required"),
        knowledge_root: required_string!("knowledge_root"),
        temporary_root: required_string!("temporary_root"),
        provenance_root: required_string!("provenance_root"),
        license_root: required_string!("license_root"),
        source_roots: required_array!("code.source_roots"),
        backend_roots: required_array!("code.backend_roots"),
        frontend_roots: required_array!("code.frontend_roots"),
        test_roots: required_array!("code.test_roots"),
    };

    if errors.is_empty() { Ok(manifest) } else { Err(errors) }
}

fn validate_manifest(manifest: &RepoManifest) -> Vec<String> {
    let mut violations = Vec::new();
    let required_strings = [
        ("schema", &manifest.schema, "atlas.repo.v2"),
        ("system_kind", &manifest.system_kind, "SYSTEM_INVENTION_FORGE"),
        ("backend_language", &manifest.backend_language, "rust"),
        ("frontend_language", &manifest.frontend_language, "typescript"),
        ("knowledge_root", &manifest.knowledge_root, ".atlas"),
        ("temporary_root", &manifest.temporary_root, ".atlas/temporary"),
        ("provenance_root", &manifest.provenance_root, ".atlas/provenance"),
        ("license_root", &manifest.license_root, ".atlas/licenses"),
    ];
    for (key, actual, expected) in required_strings {
        if actual != expected {
            violations.push(format!("{key} must be {expected}"));
        }
    }
    let required_bools = [
        ("coding_requires_docs_gate", manifest.coding_requires_docs_gate),
        ("graph_before_code_required", manifest.graph_before_code_required),
        ("exact_base_sha_required", manifest.exact_base_sha_required),
        ("single_repository_target_required", manifest.single_repository_target_required),
    ];
    for (key, actual) in required_bools {
        if !actual {
            violations.push(format!("{key} must be true"));
        }
    }
    violations
}

pub fn audit(root: impl AsRef<Path>) -> io::Result<RepoAudit> {
    let root = root.as_ref();
    let metadata = fs::metadata(root)?;
    if !metadata.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "repository root is not a directory"));
    }

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
            Err(errors) => {
                missing_required_roles.extend(errors);
            }
        }
    }

    let forbidden_roots_present = [
        "docs",
        "migration",
    ]
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
        schema: "atlas.systemizer.repo-audit.v5".into(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs};

    #[test]
    fn requires_atlas_v2_and_forbids_legacy_docs_root() {
        let root = env::temp_dir().join(format!("atlas-repo-v5-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".atlas")).unwrap();
        fs::write(root.join(".atlas/repo.toml"), r#"schema = "atlas.repo.v2"
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
legacy_docs_root_forbidden = true

[code]
source_roots = ["crates"]
backend_roots = ["crates"]
frontend_roots = ["apps/ui"]
test_roots = ["tests"]
"#).unwrap();
        let report = audit(&root).unwrap();
        assert!(report.ready, "{report:?}");
        fs::create_dir_all(root.join("docs")).unwrap();
        let report = audit(&root).unwrap();
        assert!(!report.ready);
        assert!(report.forbidden_roots_present.contains(&"docs".to_string()));
        fs::remove_dir_all(root).unwrap();
    }
}
