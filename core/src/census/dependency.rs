//! Typed dependency-census kernel (`.atlas/contracts/DEPENDENCY-CENSUS.md`).
//!
//! Census does not stop at the repository boundary: the resolved build/runtime dependency graph
//! is canonical census truth, not an optional SBOM side report. This module is the typed carrier
//! for that graph -- the extraction/parsing mechanics that populate it live in `adapter`
//! (filesystem/parser access), matching this crate's own "no filesystem, Git, provider, donor or
//! UI mechanics" boundary.
//!
//! # Scope this wave
//!
//! Only the Cargo ecosystem, and only Atlas's own workspace (`.atlas/contracts/
//! DEPENDENCY-CENSUS.md#atlas-self-census`: "Atlas Studio is subject to this contract itself").
//! Other ecosystems (npm, pip, ...) remain explicit TARGET work -- adding them is bounded,
//! evidence-linked scope growth when a concrete gap motivates it, not speculative expansion now.
//!
//! `Cargo.lock` is itself Cargo's own already-fully-resolved transitive dependency graph (each
//! `[[package]]` entry lists its own resolved `dependencies`), so parsing it -- pure static
//! parsing, never `cargo tree`/`cargo metadata` execution, matching the "ingestion is not
//! execution" security boundary -- yields the complete transitive closure for this ecosystem
//! without needing a separate resolution algorithm. What `Cargo.lock` does NOT carry is which
//! manifest section (`[dependencies]`/`[dev-dependencies]`/`[build-dependencies]`) an edge came
//! from; that requires also reading each workspace member's own `Cargo.toml`.

use serde::{Deserialize, Serialize};

/// The package ecosystem a dependency was resolved in. An extensible, named vocabulary (matching
/// `EffectCategory`/`PersistenceKind`'s precedent) so a future ecosystem is a new variant, never a
/// free-form string.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DependencyEcosystem {
    Cargo,
}

impl DependencyEcosystem {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Cargo => "CARGO",
        }
    }
}

/// `.atlas/contracts/DEPENDENCY-CENSUS.md#dependency-edge`'s dependency-kind vocabulary. Declared
/// in full; only `Runtime`/`Dev`/`Build` are reachable from a Cargo manifest's own three
/// dependency-table headers. The rest name real Cargo/ecosystem concepts this wave does not yet
/// evidence (matching the `PersistenceKind`/`ConcurrencyKind` precedent: declared, not emitted,
/// rather than silently absent from the enum).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DependencyKind {
    /// A plain `[dependencies]` table entry.
    Runtime,
    /// A `[dev-dependencies]` table entry.
    Dev,
    /// A `[build-dependencies]` table entry.
    Build,
    /// Reserved; not emitted this wave (would require reading the crate's own build.rs/proc-macro
    /// declaration, e.g. a `proc-macro = true` manifest flag this parser does not yet read).
    ProcMacro,
    /// Reserved; not emitted this wave (target-conditional `[target.'cfg(...)'.dependencies]`
    /// tables are not yet parsed -- see the module doc comment's scope note).
    TargetConditional,
    /// Reserved; not emitted this wave (an `optional = true` manifest entry is not yet parsed).
    Optional,
}

impl DependencyKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Runtime => "RUNTIME",
            Self::Dev => "DEV",
            Self::Build => "BUILD",
            Self::ProcMacro => "PROC_MACRO",
            Self::TargetConditional => "TARGET_CONDITIONAL",
            Self::Optional => "OPTIONAL",
        }
    }
}

/// Where a resolved dependency's source lives (`.atlas/contracts/DEPENDENCY-CENSUS.md#source-backed-dependencies`
/// / `#non-source-terminal-boundaries`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DependencySourceKind {
    /// A local workspace path member (a `Cargo.lock` package entry with no `source` field).
    WorkspaceMember,
    /// A registry-resolved package (a `Cargo.lock` package entry with a `source = "registry+..."`
    /// field).
    Registry,
    /// Reserved; not emitted this wave (a `source = "git+..."` entry).
    Vcs,
    /// Reserved; not emitted this wave (a non-workspace local `path = "..."` dependency).
    Path,
}

impl DependencySourceKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::WorkspaceMember => "WORKSPACE_MEMBER",
            Self::Registry => "REGISTRY",
            Self::Vcs => "VCS",
            Self::Path => "PATH",
        }
    }
}

/// Identity of one resolved dependency instance (`.atlas/contracts/DEPENDENCY-CENSUS.md#dependency-identity`).
///
/// Two versions of the same package are distinct instances: `identity_key()` includes `version`,
/// never name alone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyIdentity {
    pub ecosystem: DependencyEcosystem,
    pub name: String,
    pub version: String,
    pub source_kind: DependencySourceKind,
    /// The registry/source locator (e.g. `"registry+https://github.com/rust-lang/crates.io-index"`)
    /// when `source_kind == Registry`; `None` for a workspace member (no `source` field exists to
    /// read) or any other source kind not yet emitted.
    pub source_locator: Option<String>,
    /// The `Cargo.lock` `checksum` field, when present. Absent for a workspace member (no
    /// checksum -- it is not a fetched artifact) and for any registry entry `Cargo.lock` itself
    /// omits one for.
    pub checksum: Option<String>,
}

impl DependencyIdentity {
    /// Deterministic, order-independent encoding of this dependency instance's identity fields.
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.ecosystem.as_str(),
            self.name,
            self.version,
            self.source_kind.as_str(),
        )
    }
}

/// One resolved dependency edge: `consumer` requires `provider`
/// (`.atlas/contracts/DEPENDENCY-CENSUS.md#dependency-edge`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyEdge {
    /// The requiring package's own name (a Cargo package name -- a workspace member for a direct
    /// edge, or another resolved package for a transitive edge).
    pub consumer: String,
    pub provider: DependencyIdentity,
    /// `Some(kind)` only when this edge's manifest declaration was actually read (a workspace
    /// member's own `Cargo.toml`); `None` for a transitive edge between two external packages,
    /// whose originating manifest this parser never fetches or reads (`.atlas/contracts/
    /// SEMANTIC-EXTRACTION.md`-style discipline: never claim a fact this extractor cannot prove
    /// from the input it actually read). Never defaulted to `Runtime` by assumption.
    pub kind: Option<DependencyKind>,
    /// `.atlas` root-relative path to the file this edge's existence was read from (e.g.
    /// `"Cargo.lock"` or `"adapter/Cargo.toml"`).
    pub evidence_path: String,
}

impl DependencyEdge {
    /// Deterministic, order-independent encoding of this edge's identity: which consumer requires
    /// which exact provider instance. `kind`/`evidence_path` are content about an already-
    /// identified edge, not part of what makes the edge itself distinct -- excluded here for the
    /// same reason `CallSiteIdentity` excludes `dispatch`/`callees`.
    pub fn identity_key(&self) -> String {
        format!("{}|{}", self.consumer, self.provider.identity_key())
    }
}

/// The resolved dependency closure for one ecosystem within one repository root
/// (`.atlas/contracts/DEPENDENCY-CENSUS.md#closure`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyClosureReport {
    pub schema: String,
    pub ecosystem: DependencyEcosystem,
    pub root: String,
    pub edges_total: usize,
    /// Distinct `DependencyIdentity` count (`edges_total` counts edges, which may share a
    /// provider; this counts resolved instances).
    pub instances_total: usize,
    pub edges: Vec<DependencyEdge>,
    /// A provider name appearing in some package's `dependencies` list with no corresponding
    /// `[[package]]` entry of its own -- a malformed/incomplete lockfile, never silently dropped.
    pub dangling_references: Vec<String>,
}

impl DependencyClosureReport {
    /// `true` iff every referenced provider actually resolved to a real package entry (no
    /// dangling reference) and at least one edge was found. An empty, otherwise-clean closure
    /// (zero edges) is not itself a failure, but callers deciding overall census coverage should
    /// treat zero edges as "not attempted/not applicable" rather than "closed", since a genuinely
    /// dependency-free workspace is not the expected case here.
    pub fn is_closed(&self) -> bool {
        self.dangling_references.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_member(name: &str) -> DependencyIdentity {
        DependencyIdentity {
            ecosystem: DependencyEcosystem::Cargo,
            name: name.into(),
            version: "0.1.0".into(),
            source_kind: DependencySourceKind::WorkspaceMember,
            source_locator: None,
            checksum: None,
        }
    }

    fn registry_package(name: &str, version: &str) -> DependencyIdentity {
        DependencyIdentity {
            ecosystem: DependencyEcosystem::Cargo,
            name: name.into(),
            version: version.into(),
            source_kind: DependencySourceKind::Registry,
            source_locator: Some("registry+https://github.com/rust-lang/crates.io-index".into()),
            checksum: Some("abc123".into()),
        }
    }

    #[test]
    fn identity_key_distinguishes_version() {
        let a = registry_package("syn", "1.0.0");
        let b = registry_package("syn", "2.0.0");
        assert_ne!(a.identity_key(), b.identity_key());
    }

    #[test]
    fn identity_key_distinguishes_source_kind() {
        let a = workspace_member("core");
        let mut b = a.clone();
        b.source_kind = DependencySourceKind::Registry;
        assert_ne!(a.identity_key(), b.identity_key());
    }

    #[test]
    fn identity_key_is_unaffected_by_source_locator_and_checksum() {
        let mut a = registry_package("syn", "1.0.0");
        let mut b = a.clone();
        a.source_locator = Some("one".into());
        a.checksum = Some("one".into());
        b.source_locator = Some("two".into());
        b.checksum = Some("two".into());
        assert_eq!(a.identity_key(), b.identity_key());
    }

    #[test]
    fn edge_identity_key_is_unaffected_by_kind_and_evidence_path() {
        let base = DependencyEdge {
            consumer: "adapter".into(),
            provider: registry_package("syn", "1.0.0"),
            kind: None,
            evidence_path: "Cargo.lock".into(),
        };
        let with_kind = DependencyEdge {
            kind: Some(DependencyKind::Runtime),
            evidence_path: "adapter/Cargo.toml".into(),
            ..base.clone()
        };
        assert_eq!(base.identity_key(), with_kind.identity_key());
    }

    #[test]
    fn edge_identity_key_distinguishes_consumer() {
        let a = DependencyEdge {
            consumer: "adapter".into(),
            provider: registry_package("syn", "1.0.0"),
            kind: None,
            evidence_path: "Cargo.lock".into(),
        };
        let b = DependencyEdge {
            consumer: "runtime".into(),
            ..a.clone()
        };
        assert_ne!(a.identity_key(), b.identity_key());
    }

    #[test]
    fn closure_with_no_dangling_references_is_closed() {
        let report = DependencyClosureReport {
            schema: "test".into(),
            ecosystem: DependencyEcosystem::Cargo,
            root: "/repo".into(),
            edges_total: 1,
            instances_total: 1,
            edges: vec![DependencyEdge {
                consumer: "adapter".into(),
                provider: registry_package("syn", "1.0.0"),
                kind: Some(DependencyKind::Runtime),
                evidence_path: "adapter/Cargo.toml".into(),
            }],
            dangling_references: Vec::new(),
        };
        assert!(report.is_closed());
    }

    #[test]
    fn closure_with_a_dangling_reference_is_not_closed() {
        let report = DependencyClosureReport {
            schema: "test".into(),
            ecosystem: DependencyEcosystem::Cargo,
            root: "/repo".into(),
            edges_total: 0,
            instances_total: 0,
            edges: Vec::new(),
            dangling_references: vec!["ghost-crate".into()],
        };
        assert!(!report.is_closed());
    }

    #[test]
    fn kind_and_ecosystem_and_source_kind_as_str_match_the_screaming_snake_vocabulary() {
        assert_eq!(DependencyEcosystem::Cargo.as_str(), "CARGO");
        assert_eq!(DependencyKind::Runtime.as_str(), "RUNTIME");
        assert_eq!(DependencyKind::Dev.as_str(), "DEV");
        assert_eq!(DependencyKind::Build.as_str(), "BUILD");
        assert_eq!(DependencyKind::ProcMacro.as_str(), "PROC_MACRO");
        assert_eq!(
            DependencyKind::TargetConditional.as_str(),
            "TARGET_CONDITIONAL"
        );
        assert_eq!(DependencyKind::Optional.as_str(), "OPTIONAL");
        assert_eq!(
            DependencySourceKind::WorkspaceMember.as_str(),
            "WORKSPACE_MEMBER"
        );
        assert_eq!(DependencySourceKind::Registry.as_str(), "REGISTRY");
        assert_eq!(DependencySourceKind::Vcs.as_str(), "VCS");
        assert_eq!(DependencySourceKind::Path.as_str(), "PATH");
    }
}
