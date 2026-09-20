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

const FORBIDDEN_ATLAS_FOOTPRINTS: &[&str] = &[
    ".atlas/repo.toml",
    ".atlas/systemizer.toml",
    "tools/system-atlas",
    "tools/docs-atlas",
    "tools/reality-atlas",
];

pub fn audit(root: impl AsRef<Path>) -> io::Result<RepoAudit> {
    let root = root.as_ref();
    let metadata = fs::metadata(root)?;
    if !metadata.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "repository root is not a directory"));
    }

    let forbidden_roots_present = FORBIDDEN_ATLAS_FOOTPRINTS
        .iter()
        .filter(|path| root.join(path).exists())
        .map(|path| (*path).to_owned())
        .collect::<Vec<_>>();

    Ok(RepoAudit {
        schema: "atlas.systemizer.repo-audit.v2".into(),
        archetype: "MANAGED_REPOSITORY".into(),
        missing_required_roles: Vec::new(),
        missing_mapped_paths: Vec::new(),
        ready: forbidden_roots_present.is_empty(),
        forbidden_roots_present,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs};

    #[test]
    fn clean_target_requires_no_atlas_metadata() {
        let root = env::temp_dir().join(format!("atlas-repo-clean-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs")).unwrap();
        let report = audit(&root).unwrap();
        assert!(report.ready, "{report:?}");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_embedded_atlas_is_rejected() {
        let root = env::temp_dir().join(format!("atlas-repo-legacy-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("tools/system-atlas")).unwrap();
        let report = audit(&root).unwrap();
        assert!(!report.ready);
        assert_eq!(report.forbidden_roots_present, vec!["tools/system-atlas".to_string()]);
        fs::remove_dir_all(root).unwrap();
    }
}
