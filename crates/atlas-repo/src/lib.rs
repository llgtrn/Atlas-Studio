use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoAudit {
    pub schema: String,
    pub archetype: String,
    pub missing_required_roles: Vec<String>,
    pub missing_mapped_paths: Vec<String>,
    pub forbidden_roots_present: Vec<String>,
    pub ready: bool,
}

fn quoted_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let line = line.trim();
        let prefix = format!("{key} = ");
        line.strip_prefix(&prefix)
            .map(str::trim)
            .and_then(|value| value.strip_prefix('"'))
            .and_then(|value| value.strip_suffix('"'))
            .map(ToOwned::to_owned)
    })
}

fn bool_value(text: &str, key: &str) -> Option<bool> {
    text.lines().find_map(|line| {
        let line = line.trim();
        let prefix = format!("{key} = ");
        match line.strip_prefix(&prefix).map(str::trim) {
            Some("true") => Some(true),
            Some("false") => Some(false),
            _ => None,
        }
    })
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
    if repo_manifest.is_file() {
        let text = fs::read_to_string(&repo_manifest)?;
        if quoted_value(&text, "schema").as_deref() != Some("atlas.repo.v2") {
            missing_required_roles.push("atlas.repo.v2".into());
        }
        if quoted_value(&text, "knowledge_root").as_deref() != Some(".atlas") {
            missing_required_roles.push("knowledge_root=.atlas".into());
        }
        if bool_value(&text, "legacy_docs_root_forbidden") != Some(true) {
            missing_required_roles.push("legacy_docs_root_forbidden=true".into());
        }
        archetype = quoted_value(&text, "kind").unwrap_or_else(|| "UNKNOWN".into());
    }

    let forbidden_roots_present = [
        "docs",
        "tools/system-atlas",
        "tools/docs-atlas",
        "tools/reality-atlas",
    ]
    .iter()
    .filter(|path| root.join(path).exists())
    .map(|path| (*path).to_owned())
    .collect::<Vec<_>>();

    let ready = missing_required_roles.is_empty()
        && missing_mapped_paths.is_empty()
        && forbidden_roots_present.is_empty();

    Ok(RepoAudit {
        schema: "atlas.systemizer.repo-audit.v5".into(),
        archetype,
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
kind = "DEVELOPMENT_CELL"
knowledge_root = ".atlas"
legacy_docs_root_forbidden = true
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
