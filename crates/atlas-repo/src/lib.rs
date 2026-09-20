use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoAudit {
    pub schema: String,
    pub archetype: String,
    pub missing_required_roots: Vec<String>,
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

fn policy(archetype: &str) -> (&'static [&'static str], &'static [&'static str]) {
    match archetype {
        "CANONICAL_PRODUCT" => (&[".atlas","core","runtime","adapter","organism","apps","docs"], &[]),
        "DEVELOPMENT_CELL" => (
            &[".atlas","docs","temporary","provenance","license","evidence","work"],
            &["core","runtime","adapter","organism","apps","src","backend","frontend","services","packages"],
        ),
        "ENGINEERING_SUBSYSTEM" => (&[".atlas","crates","contracts","docs","references","fleet"], &[]),
        "DOMAIN_SUBSYSTEM" => (&[".atlas","crates","apps","docs","contracts"], &[]),
        _ => (&[], &[]),
    }
}

pub fn audit(root: impl AsRef<Path>) -> io::Result<RepoAudit> {
    let root = root.as_ref();
    let manifest = fs::read_to_string(root.join(".atlas/repo.toml"))?;
    let archetype = quoted_value(&manifest, "archetype")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing archetype in .atlas/repo.toml"))?;
    let (required, forbidden) = policy(&archetype);
    if required.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown repo archetype {archetype}")));
    }
    let missing_required_roots = required.iter().filter(|p| !root.join(p).exists()).map(|p| (*p).to_owned()).collect::<Vec<_>>();
    let forbidden_roots_present = forbidden.iter().filter(|p| root.join(p).exists()).map(|p| (*p).to_owned()).collect::<Vec<_>>();
    let ready = missing_required_roots.is_empty() && forbidden_roots_present.is_empty();
    Ok(RepoAudit {
        schema: "atlas.systemizer.repo-audit.v1".into(),
        archetype,
        missing_required_roots,
        forbidden_roots_present,
        ready,
    })
}
