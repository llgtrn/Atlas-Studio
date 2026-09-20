use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io, path::Path};

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

fn parse_array(value: &str) -> Vec<String> {
    let value = value.trim();
    let Some(value) = value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) else {
        return Vec::new();
    };
    value
        .split(',')
        .filter_map(|item| {
            let item = item.trim();
            item.strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn roots(text: &str) -> BTreeMap<String, Vec<String>> {
    let mut in_roots = false;
    let mut result = BTreeMap::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_roots = line == "[roots]";
            continue;
        }
        if !in_roots || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            result.insert(key.trim().to_owned(), parse_array(value));
        }
    }
    result
}

fn required_roles(archetype: &str) -> &'static [&'static str] {
    match archetype {
        "CANONICAL_PRODUCT" => &["backend", "frontend", "docs", "atlas_client"],
        "DEVELOPMENT_CELL" => &["docs", "donors", "provenance", "licenses", "evidence", "work"],
        "ENGINEERING_SUBSYSTEM" => &["backend", "docs", "contracts", "references", "fleet"],
        "DOMAIN_SUBSYSTEM" => &["backend", "frontend", "docs", "contracts"],
        _ => &[],
    }
}

fn forbidden_physical_roots(archetype: &str) -> &'static [&'static str] {
    match archetype {
        "DEVELOPMENT_CELL" => &["core", "runtime", "adapter", "organism", "apps", "src", "backend", "frontend", "services", "packages"],
        _ => &[],
    }
}

pub fn audit(root: impl AsRef<Path>) -> io::Result<RepoAudit> {
    let root = root.as_ref();
    let manifest = fs::read_to_string(root.join(".atlas/repo.toml"))?;
    let archetype = quoted_value(&manifest, "archetype")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing archetype in .atlas/repo.toml"))?;
    let required = required_roles(&archetype);
    if required.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown repo archetype {archetype}")));
    }

    let mapped = roots(&manifest);
    let missing_required_roles = required
        .iter()
        .filter(|role| !mapped.contains_key(**role))
        .map(|role| (*role).to_owned())
        .collect::<Vec<_>>();

    let mut missing_mapped_paths = Vec::new();
    for role in required {
        if let Some(paths) = mapped.get(*role) {
            if paths.is_empty() {
                missing_mapped_paths.push(format!("{role}:<no-path>"));
            }
            for path in paths {
                if !root.join(path).exists() {
                    missing_mapped_paths.push(format!("{role}:{path}"));
                }
            }
        }
    }

    let forbidden_roots_present = forbidden_physical_roots(&archetype)
        .iter()
        .filter(|path| root.join(path).exists())
        .map(|path| (*path).to_owned())
        .collect::<Vec<_>>();

    let ready = missing_required_roles.is_empty()
        && missing_mapped_paths.is_empty()
        && forbidden_roots_present.is_empty();

    Ok(RepoAudit {
        schema: "atlas.systemizer.repo-audit.v1".into(),
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
    fn semantic_roles_are_stable_across_physical_paths() {
        let root = env::temp_dir().join(format!("atlas-repo-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".atlas")).unwrap();
        for path in ["engine", "ui", "documents"] {
            fs::create_dir_all(root.join(path)).unwrap();
        }
        fs::write(root.join(".atlas/repo.toml"), r#"
archetype = "CANONICAL_PRODUCT"
[roots]
backend = ["engine"]
frontend = ["ui"]
docs = ["documents"]
atlas_client = [".atlas"]
"#).unwrap();
        let report = audit(&root).unwrap();
        assert!(report.ready, "{report:?}");
        fs::remove_dir_all(root).unwrap();
    }
}
