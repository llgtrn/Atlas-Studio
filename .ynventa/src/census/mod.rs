//! The census observes, from committed content only, every way foreign technology participates
//! in the repository: manifest dependencies (Cargo, npm, Python), source imports, native links,
//! process invocations, references to held donor source, and the locked dependency closure.
//! Observations are facts; the extinction verifier and the metrics judge them.

pub mod cargo;
pub mod sources;

use crate::declare::Declaration;
use crate::repository::files::Files;
use crate::schema::{Ecosystem, Scope};
use std::collections::{BTreeMap, BTreeSet};

/// How an observation was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Via {
    Manifest,
    Import,
    Link,
    Process,
    SourceReference,
}

impl Via {
    pub fn wire(self) -> &'static str {
        match self {
            Via::Manifest => "MANIFEST",
            Via::Import => "IMPORT",
            Via::Link => "LINK",
            Via::Process => "PROCESS",
            Via::SourceReference => "SOURCE_REFERENCE",
        }
    }
}

/// One observed use of something foreign.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Observation {
    /// The file that makes the reference.
    pub file: String,
    pub ecosystem: Ecosystem,
    /// Package, library, program or donor-source path referenced.
    pub name: String,
    /// The identifier source code uses for it (Cargo renames), else the name.
    pub ident: String,
    pub scope: Scope,
    pub via: Via,
}

/// An internal workspace package.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Member {
    pub dir: String,
    pub package: String,
}

#[derive(Clone, Debug, Default)]
pub struct Census {
    pub members: Vec<Member>,
    /// Workspace-internal dependency edges: (from dir, to dir, scope).
    pub internal: BTreeSet<(String, String, Scope)>,
    /// Every foreign reference.
    pub observations: BTreeSet<Observation>,
    /// Source-level crate identifiers used by each `.rs` file (`ident::` paths, `use`, `extern crate`).
    pub imports: BTreeMap<String, BTreeSet<String>>,
    /// External packages in the locked closure (`Cargo.lock` entries with a source).
    pub closure: BTreeSet<String>,
    /// Files that could not be read or parsed.
    pub unreadable: Vec<String>,
}

impl Census {
    pub fn external(&self) -> impl Iterator<Item = &Observation> {
        self.observations
            .iter()
            .filter(|o| matches!(o.via, Via::Manifest | Via::Link | Via::Process))
    }
}

/// Paths that are never scanned as repository technology: held donor source (judged separately
/// as resident source) and the legacy knowledge tree.
pub fn excluded_roots(d: &Declaration) -> Vec<String> {
    let mut v: Vec<String> = d
        .donors
        .iter()
        .flat_map(|dn| dn.source_paths.iter().cloned())
        .map(|p| p.trim_end_matches('/').to_string())
        .collect();
    v.push(".atlas".into());
    v.sort();
    v.dedup();
    v
}

pub fn is_excluded(path: &str, excluded: &[String]) -> bool {
    excluded
        .iter()
        .any(|e| path == e || path.starts_with(&format!("{e}/")))
}

/// Scope of a reference made by `file`: tests are TEST, tools and CI are BUILD, else RUNTIME.
pub fn scope_of_file(file: &str) -> Scope {
    let segs: Vec<&str> = file.split('/').collect();
    let name = segs.last().copied().unwrap_or("");
    if segs.contains(&"tests") || segs.contains(&"benches") || name.ends_with("_test.rs") {
        Scope::Test
    } else if name == "build.rs" || segs[0] == "tools" || segs[0] == ".github" {
        Scope::Build
    } else {
        Scope::Runtime
    }
}

pub fn run(files: &Files, d: &Declaration) -> Census {
    let excluded = excluded_roots(d);
    let mut c = Census::default();
    cargo::observe(files, &excluded, &mut c);
    sources::observe(files, d, &excluded, &mut c);
    c
}
