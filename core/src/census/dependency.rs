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
///
/// `Cargo.lock` alone cannot distinguish `WorkspaceMember` from `Path` -- both have no `source`
/// field. Correct classification of a source-less package additionally requires the caller to
/// know the real workspace-member package-name set (`adapter::census_cargo_workspace` reads it
/// from the root manifest's `workspace.members` before classifying any package).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DependencySourceKind {
    /// A `Cargo.lock` package entry with no `source` field whose name is a real, evidenced
    /// workspace member (present in the root manifest's `workspace.members`).
    WorkspaceMember,
    /// A `source = "registry+..."` entry.
    Registry,
    /// A `source = "git+..."` entry.
    Vcs,
    /// A `Cargo.lock` package entry with no `source` field whose name is NOT a known workspace
    /// member -- a local `path = "..."` dependency outside the workspace (or, if workspace-member
    /// discovery itself was incomplete, an evidenced-absent case; never silently folded into
    /// `WorkspaceMember`).
    Path,
    /// A `source` field present but matching neither the `registry+` nor `git+` prefix this
    /// bootstrap recognizes (e.g. a future/unfamiliar source-protocol spelling). Explicit rather
    /// than guessed.
    Other,
}

impl DependencySourceKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::WorkspaceMember => "WORKSPACE_MEMBER",
            Self::Registry => "REGISTRY",
            Self::Vcs => "VCS",
            Self::Path => "PATH",
            Self::Other => "OTHER",
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

/// Evidence-backed closure state for one `DependencyClosureReport`
/// (`.atlas/contracts/DEPENDENCY-CENSUS.md#closure`).
///
/// Replaces a bare `dangling_references.is_empty()` boolean, which cannot distinguish "this
/// ecosystem does not exist at this root" from "census never ran" from "census ran and verified a
/// real, dependency-free result" -- three states a fabricated zero-edge default report previously
/// collapsed into one silent `true`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DependencyClosureState {
    /// This ecosystem has no manifest/lockfile at the admitted root at all (e.g. no `Cargo.lock`
    /// for the Cargo ecosystem) -- not a failure, simply not applicable to this repository.
    NotApplicable,
    /// A lockfile exists but yielded zero `[[package]]` entries. Real Cargo output always lists at
    /// least the root/member packages themselves, so this is evidence of a malformed, truncated,
    /// or otherwise unreadable lockfile, not a verified empty project -- distinct from `Closed`
    /// with zero edges (real packages, genuinely zero resolved dependencies).
    Blocked,
    /// At least one package resolved, but closure is incomplete: a dangling reference, an
    /// unresolved multi-version ambiguity, or a detected-but-unsupported manifest construct that
    /// could hide an active dependency exists in scope.
    Partial,
    /// Every referenced provider resolved to a real package entry, no unresolved ambiguity or
    /// detected-but-unsupported construct remains, and at least one real package was observed
    /// (whether or not it has any dependencies of its own).
    Closed,
}

impl DependencyClosureState {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::NotApplicable => "NOT_APPLICABLE",
            Self::Blocked => "BLOCKED",
            Self::Partial => "PARTIAL",
            Self::Closed => "CLOSED",
        }
    }
}

/// A class of dependency obligation that a purely static lockfile/manifest parse cannot resolve
/// (`.atlas/contracts/DEPENDENCY-CENSUS.md#build-time-and-dynamic-dependency-discovery`). Declared
/// in full (matching the `DependencyKind`/`PersistenceKind` precedent); every class this bootstrap
/// reports is always `UNKNOWN` -- none are actually probed yet, so the vocabulary exists to make
/// that gap explicit rather than letting a resolved static package graph silently stand in for
/// "no active dependency behavior can appear from any other source."
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DynamicDependencyObligation {
    /// A `build.rs` build script can introduce or select dependencies Cargo.lock alone does not
    /// reveal the effect of.
    BuildScript,
    /// A proc-macro crate's expansion-time behavior is not observed by a static parse.
    ProcMacroExpansion,
    /// `pkg-config`-discovered native dependencies are resolved at build time, not in Cargo.lock.
    PkgConfig,
    /// Native/FFI linking against a system library is not represented by the Cargo graph.
    NativeLinking,
    /// Source generated during the build (`OUT_DIR` artifacts, codegen) may itself carry further
    /// dependency obligations.
    GeneratedSource,
    /// Build scripts/proc-macros may branch on environment variables in ways that change effective
    /// dependency behavior.
    EnvironmentProbe,
    /// `dlopen`-style dynamic loading is invisible to static manifest/lockfile analysis.
    DynamicLoading,
    /// Runtime plugin discovery can introduce dependency behavior with no manifest trace at all.
    PluginDiscovery,
    /// A dependency on an external service/capability (network API, daemon, ...) is not a Cargo
    /// package and never appears in this closure.
    ExternalCapability,
}

impl DynamicDependencyObligation {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::BuildScript => "BUILD_SCRIPT",
            Self::ProcMacroExpansion => "PROC_MACRO_EXPANSION",
            Self::PkgConfig => "PKG_CONFIG",
            Self::NativeLinking => "NATIVE_LINKING",
            Self::GeneratedSource => "GENERATED_SOURCE",
            Self::EnvironmentProbe => "ENVIRONMENT_PROBE",
            Self::DynamicLoading => "DYNAMIC_LOADING",
            Self::PluginDiscovery => "PLUGIN_DISCOVERY",
            Self::ExternalCapability => "EXTERNAL_CAPABILITY",
        }
    }

    /// The full declared vocabulary, in a stable order. `adapter::census_cargo_workspace` reports
    /// every one of these as an explicit still-`UNKNOWN` obligation whenever it actually attempts
    /// a census (i.e. whenever a lockfile was found at all) -- see `DependencyClosureReport::
    /// dynamic_obligations`.
    pub const ALL: [Self; 9] = [
        Self::BuildScript,
        Self::ProcMacroExpansion,
        Self::PkgConfig,
        Self::NativeLinking,
        Self::GeneratedSource,
        Self::EnvironmentProbe,
        Self::DynamicLoading,
        Self::PluginDiscovery,
        Self::ExternalCapability,
    ];
}

/// The resolved dependency closure for one ecosystem within one repository root
/// (`.atlas/contracts/DEPENDENCY-CENSUS.md#closure`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyClosureReport {
    pub schema: String,
    pub ecosystem: DependencyEcosystem,
    pub root: String,
    pub state: DependencyClosureState,
    pub edges_total: usize,
    /// Distinct `DependencyIdentity` count (`edges_total` counts edges, which may share a
    /// provider; this counts resolved instances).
    pub instances_total: usize,
    pub edges: Vec<DependencyEdge>,
    /// A provider name appearing in some package's `dependencies` list with no corresponding
    /// `[[package]]` entry of its own -- a malformed/incomplete lockfile, never silently dropped.
    pub dangling_references: Vec<String>,
    /// A detected manifest/lockfile construct this bootstrap's parser does not fully read (e.g. a
    /// multi-line `workspace.members` array, or a dependency entry whose inline table spans more
    /// than one physical line) -- named explicitly rather than silently mis-parsed or ignored.
    /// Non-empty here forces `state` below `Closed`, since such a construct could hide an active
    /// dependency this closure would then wrongly omit.
    pub unsupported_constructs: Vec<String>,
    /// The static `DynamicDependencyObligation` classes this closure does not resolve -- see that
    /// type's own doc comment. Always the full `DynamicDependencyObligation::ALL` vocabulary
    /// whenever `state != NotApplicable`, empty otherwise (no ecosystem was found to have dynamic
    /// obligations about).
    pub dynamic_obligations: Vec<DynamicDependencyObligation>,
}

impl DependencyClosureReport {
    /// `true` iff `state == Closed`. Kept as a convenience predicate; new code should generally
    /// match on `state` directly, since `Closed` alone does not distinguish `NotApplicable` from
    /// `Blocked`/`Partial` the way callers deciding *why* closure failed usually need to.
    pub fn is_closed(&self) -> bool {
        self.state == DependencyClosureState::Closed
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

    fn base_report(state: DependencyClosureState) -> DependencyClosureReport {
        DependencyClosureReport {
            schema: "test".into(),
            ecosystem: DependencyEcosystem::Cargo,
            root: "/repo".into(),
            state,
            edges_total: 0,
            instances_total: 0,
            edges: Vec::new(),
            dangling_references: Vec::new(),
            unsupported_constructs: Vec::new(),
            dynamic_obligations: Vec::new(),
        }
    }

    #[test]
    fn a_report_in_the_closed_state_is_closed() {
        let mut report = base_report(DependencyClosureState::Closed);
        report.edges_total = 1;
        report.instances_total = 1;
        report.edges = vec![DependencyEdge {
            consumer: "adapter".into(),
            provider: registry_package("syn", "1.0.0"),
            kind: Some(DependencyKind::Runtime),
            evidence_path: "adapter/Cargo.toml".into(),
        }];
        assert!(report.is_closed());
    }

    #[test]
    fn a_report_with_a_dangling_reference_is_partial_not_closed() {
        let mut report = base_report(DependencyClosureState::Partial);
        report.dangling_references = vec!["ghost-crate".into()];
        assert!(!report.is_closed());
    }

    #[test]
    fn not_applicable_is_not_closed() {
        // The state this repository's own runtime wiring previously collapsed into a silent
        // `is_closed() == true` by fabricating a zero-edge report when no Cargo.lock exists at
        // all -- distinct from a genuinely verified empty closure.
        assert!(!base_report(DependencyClosureState::NotApplicable).is_closed());
    }

    #[test]
    fn blocked_is_not_closed() {
        // A present Cargo.lock that parsed to zero [[package]] entries -- evidence of a malformed
        // or truncated lockfile, not a verified dependency-free project.
        assert!(!base_report(DependencyClosureState::Blocked).is_closed());
    }

    #[test]
    fn closure_state_as_str_matches_the_screaming_snake_vocabulary() {
        assert_eq!(
            DependencyClosureState::NotApplicable.as_str(),
            "NOT_APPLICABLE"
        );
        assert_eq!(DependencyClosureState::Blocked.as_str(), "BLOCKED");
        assert_eq!(DependencyClosureState::Partial.as_str(), "PARTIAL");
        assert_eq!(DependencyClosureState::Closed.as_str(), "CLOSED");
    }

    #[test]
    fn dynamic_dependency_obligation_all_matches_its_declared_length_and_vocabulary() {
        assert_eq!(DynamicDependencyObligation::ALL.len(), 9);
        let mut seen = std::collections::BTreeSet::new();
        for obligation in DynamicDependencyObligation::ALL {
            assert!(
                seen.insert(obligation.as_str()),
                "duplicate obligation class: {}",
                obligation.as_str()
            );
        }
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
        assert_eq!(DependencySourceKind::Other.as_str(), "OTHER");
    }
}
