//! Registration from real package metadata. A donor the census discovered is REGISTERED once its
//! origin and licence are recorded; the licence is read from the package itself, never typed in:
//! for Cargo, the `license` of every version `Cargo.lock` pins, from the local registry sources
//! (`$CARGO_HOME/registry/src`); for npm, the installed `node_modules/<name>/package.json`.
//! A package whose metadata is not available locally stays unregistered, with the reason.

use crate::declare::{Donor, Package};
use crate::formats::{json, toml, Value};
use crate::repository::files::Files;
use crate::schema::DonorState;
use crate::schema::Ecosystem;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Where a licence came from, or why none could be read.
#[derive(Clone, Debug, PartialEq)]
pub enum Licence {
    Found {
        licence: String,
        /// The manifests read, as provenance.
        sources: Vec<String>,
    },
    Unavailable(String),
}

/// Package name -> versions pinned by any tracked `Cargo.lock` (registry packages only).
pub fn locked_versions(files: &Files) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for lock in files.paths.iter().filter(|p| {
        p.ends_with("Cargo.lock") && !p.starts_with(".ynventa/") && !p.starts_with(".atlas/")
    }) {
        let Some(v) = files.read(lock).and_then(|t| toml::parse(&t).ok()) else {
            continue;
        };
        for p in v.get("package").map(Value::items).unwrap_or(&[]) {
            if p.str("source").is_some_and(|s| s.starts_with("registry+")) {
                if let (Some(n), Some(ver)) = (p.str("name"), p.str("version")) {
                    out.entry(n.to_string())
                        .or_default()
                        .insert(ver.to_string());
                }
            }
        }
    }
    out
}

pub fn cargo_home() -> Option<PathBuf> {
    std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cargo")))
}

/// The licence of a Cargo package, from the manifests of its pinned versions.
pub fn cargo_licence(home: &Path, name: &str, versions: Option<&BTreeSet<String>>) -> Licence {
    let Some(versions) = versions.filter(|v| !v.is_empty()) else {
        return Licence::Unavailable(format!(
            "`{name}` is not pinned from a registry in Cargo.lock"
        ));
    };
    let Ok(indexes) = std::fs::read_dir(home.join("registry/src")) else {
        return Licence::Unavailable("no local Cargo registry sources (run `cargo fetch`)".into());
    };
    let indexes: Vec<PathBuf> = indexes.filter_map(Result::ok).map(|e| e.path()).collect();
    let mut licences = BTreeSet::new();
    let mut sources = Vec::new();
    for ver in versions {
        let dir = format!("{name}-{ver}");
        let Some(text) = indexes
            .iter()
            .find_map(|i| std::fs::read_to_string(i.join(&dir).join("Cargo.toml")).ok())
        else {
            return Licence::Unavailable(format!(
                "{dir} is not in the local registry (run `cargo fetch`)"
            ));
        };
        let Ok(m) = toml::parse(&text) else {
            return Licence::Unavailable(format!("{dir}/Cargo.toml does not parse"));
        };
        match m.at("package.license").and_then(Value::as_str) {
            Some(l) if !l.trim().is_empty() => {
                licences.insert(l.trim().to_string());
                sources.push(format!("registry:{dir}/Cargo.toml"));
            }
            _ => {
                return Licence::Unavailable(format!(
                    "{dir} declares no SPDX `license` (only a licence file, or none)"
                ))
            }
        }
    }
    Licence::Found {
        licence: licences.into_iter().collect::<Vec<_>>().join("; "),
        sources,
    }
}

/// The licence of an npm package: from a tracked `package-lock.json` (v2/v3 lockfiles record the
/// licence of every pinned package), else from the installed `node_modules/<name>/package.json`.
pub fn npm_licence(root: &Path, files: &Files, name: &str) -> Licence {
    let key = format!("node_modules/{name}");
    for lock in files.paths.iter().filter(|p| {
        p.ends_with("package-lock.json") && !p.starts_with(".atlas/") && !p.starts_with(".ynventa/")
    }) {
        let Some(v) = files.read(lock).and_then(|t| json::parse(&t).ok()) else {
            continue;
        };
        if let Some(l) = v
            .get("packages")
            .and_then(|p| p.get(&key))
            .and_then(|p| p.str("license"))
            .filter(|l| !l.trim().is_empty())
        {
            return Licence::Found {
                licence: l.trim().to_string(),
                sources: vec![format!("npm:{lock}#{key}")],
            };
        }
    }
    let rel = format!("{key}/package.json");
    let Ok(text) = std::fs::read_to_string(root.join(&rel)) else {
        return Licence::Unavailable(format!(
            "`{name}` is in no package-lock.json and {rel} is not installed"
        ));
    };
    match json::parse(&text)
        .ok()
        .as_ref()
        .and_then(|v| v.str("license"))
    {
        Some(l) if !l.trim().is_empty() => Licence::Found {
            licence: l.trim().to_string(),
            sources: vec![format!("npm:{rel}")],
        },
        _ => Licence::Unavailable(format!("{rel} declares no `license`")),
    }
}

/// The licence of a donor: every package it names must resolve; licences of several packages
/// are joined. Programs and other ecosystems have no local metadata to read.
pub fn donor_licence(
    root: &Path,
    files: &Files,
    home: Option<&Path>,
    locked: &BTreeMap<String, BTreeSet<String>>,
    dn: &Donor,
) -> Licence {
    if dn.packages.is_empty() {
        return Licence::Unavailable(
            "no packages: a program or source donor; record its licence by hand".into(),
        );
    }
    let mut licences = BTreeSet::new();
    let mut sources = Vec::new();
    for p in &dn.packages {
        let l = match p.ecosystem {
            Ecosystem::Cargo => match home {
                Some(h) => cargo_licence(h, &p.name, locked.get(&p.name)),
                None => Licence::Unavailable("no CARGO_HOME or HOME".into()),
            },
            Ecosystem::Npm => npm_licence(root, files, &p.name),
            other => Licence::Unavailable(format!("no local metadata reader for {}", other.wire())),
        };
        match l {
            Licence::Found {
                licence,
                sources: s,
            } => {
                licences.insert(licence);
                sources.extend(s);
            }
            u => return u,
        }
    }
    Licence::Found {
        licence: licences.into_iter().collect::<Vec<_>>().join("; "),
        sources,
    }
}

/// The donor record for an external the census observed but nothing declares: DISCOVERED, with the
/// registry origin where one follows from the ecosystem and the observing files as provenance.
/// Registration stays an explicit, evidenced step.
pub fn discovered(eco: Ecosystem, name: &str, in_files: impl IntoIterator<Item = String>) -> Donor {
    let origin = match eco {
        Ecosystem::Cargo => format!("https://crates.io/crates/{name}"),
        Ecosystem::Npm => format!("https://www.npmjs.com/package/{name}"),
        Ecosystem::Python => format!("https://pypi.org/project/{name}"),
        Ecosystem::Native => String::new(),
    };
    let mut provenance: Vec<String> = in_files
        .into_iter()
        .map(|f| format!("census:{f}"))
        .collect();
    provenance.sort();
    provenance.dedup();
    Donor {
        key: format!(
            "{}-{}",
            eco.wire().to_ascii_lowercase(),
            crate::migration::atlas::slug(name)
        ),
        name: name.to_string(),
        origin,
        license: String::new(),
        // An importer cannot verify a licence: observed externals enter as DISCOVERED.
        claimed: DonorState::Discovered,
        exception: None,
        packages: vec![Package {
            ecosystem: eco,
            name: name.to_string(),
        }],
        source_paths: Vec::new(),
        capabilities: Vec::new(),
        cutover: None,
        provenance,
    }
}
