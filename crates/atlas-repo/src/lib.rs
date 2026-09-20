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

const REQUIRED_ROOTS: &[&str] = &["docs"];
const FORBIDDEN_ROOTS: &[&str] = &["tools/system-atlas", "tools/docs-atlas", "tools/reality-atlas"];

pub fn audit(root: impl AsRef<Path>) -> io::Result<RepoAudit> {
    let root = root.as_ref();
    let metadata = fs::metadata(root)?;
    if !metadata.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "repository root is not a directory"));
    }

    let missing_mapped_paths = REQUIRED_ROOTS
        .iter()
        .filter(|path| !root.join(path).is_dir())
        .map(|path| (*path).to_owned())
        .collect::<Vec<_>>();

    let forbidden_roots_present = FORBIDDEN_ROOTS
        .iter()
        .filter(|path| root.join(path).exists())
        .map(|path| (*path).to_owned())
        .collect::<Vec<_>>();

    Ok(RepoAudit {
        schema: "atlas.systemizer.repo-audit.v4".into(),
        archetype: "ATLAS_MANAGED_REPOSITORY".into(),
        missing_required_roles: if root.join("docs").is_dir() { Vec::new() } else { vec!["canonical_docs".into()] },
        ready: missing_mapped_paths.is_empty() && forbidden_roots_present.is_empty(),
        missing_mapped_paths,
        forbidden_roots_present,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs};

    #[test]
    fn blank_repo_is_not_ready_until_docs_exist() {
        let root = env::temp_dir().join(format!("atlas-repo-blank-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let report = audit(&root).unwrap();
        assert!(!report.ready);
        assert!(report.missing_required_roles.contains(&"canonical_docs".to_string()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn docs_root_is_required_not_forbidden() {
        let root = env::temp_dir().join(format!("atlas-repo-docs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs")).unwrap();
        let report = audit(&root).unwrap();
        assert!(report.ready, "{report:?}");
        fs::remove_dir_all(root).unwrap();
    }
}
