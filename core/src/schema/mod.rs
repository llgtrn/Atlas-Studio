//! Stable repository-facing schemas and reports.
//!
//! These records are data contracts. Mechanics that populate them belong in adapter/runtime.

use crate::{
    census::InventoryReport, constraint::CodingAdmission, language::adl::AdlCompileReport,
    provenance::Provenance, state::RepositorySnapshot,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
pub struct FileFact {
    pub path: String,
    pub language: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceReport {
    pub schema: String,
    pub root: String,
    pub files_total: usize,
    pub languages: BTreeMap<String, usize>,
    pub files: Vec<FileFact>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticFactKind {
    SourceArtifact,
    DeclaredNode,
    DeclaredEdge,
    Binding,
    Constraint,
    Invariant,
    ConstraintResult,
    Diagnostic,
    Transform,
    Materialization,
}

impl SemanticFactKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::SourceArtifact => "SOURCE_ARTIFACT",
            Self::DeclaredNode => "DECLARED_NODE",
            Self::DeclaredEdge => "DECLARED_EDGE",
            Self::Binding => "BINDING",
            Self::Constraint => "CONSTRAINT",
            Self::Invariant => "INVARIANT",
            Self::ConstraintResult => "CONSTRAINT_RESULT",
            Self::Diagnostic => "DIAGNOSTIC",
            Self::Transform => "TRANSFORM",
            Self::Materialization => "MATERIALIZATION",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EpistemicStatus {
    Observed,
    Declared,
    Derived,
    Inferred,
    Hypothesis,
    Conflict,
    Unknown,
    Unsupported,
    Ignored,
}

impl EpistemicStatus {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Observed => "OBSERVED",
            Self::Declared => "DECLARED",
            Self::Derived => "DERIVED",
            Self::Inferred => "INFERRED",
            Self::Hypothesis => "HYPOTHESIS",
            Self::Conflict => "CONFLICT",
            Self::Unknown => "UNKNOWN",
            Self::Unsupported => "UNSUPPORTED",
            Self::Ignored => "IGNORED",
        }
    }
}

/// Bootstrap R4 interchange envelope.
///
/// Deep R4 semantics converge on typed records defined by the semantic-facts contract; this
/// subject/predicate/object carrier must not become Atlas's permanent universal semantic model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticFact {
    pub id: String,
    pub kind: SemanticFactKind,
    pub status: EpistemicStatus,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CensusReport {
    pub schema: String,
    pub artifacts_total: usize,
    pub artifacts_accounted_total: usize,
    pub facts_total: usize,
    /// Bootstrap / derived coverage projection: at most one `EpistemicStatus` per dimension key.
    /// This is a noncanonical summary shape kept for existing callers, never the canonical
    /// multi-extractor accounting. Independent extractors that disagree on a dimension are
    /// preserved in `runtime::census::CensusExtractionAccounting`, not collapsed into this map
    /// (`.atlas/contracts/SEMANTIC-EXTRACTION.md#multi-engine-extraction`).
    pub coverage: BTreeMap<String, EpistemicStatus>,
    pub facts: Vec<SemanticFact>,
}

impl CensusReport {
    pub fn is_closed(&self) -> bool {
        self.artifacts_total == self.artifacts_accounted_total
            && self.facts_total == self.facts.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizationReport {
    pub schema: String,
    pub input_facts_total: usize,
    pub normalized_facts_total: usize,
    pub kinds: BTreeMap<String, usize>,
    pub facts: Vec<SemanticFact>,
}

impl NormalizationReport {
    pub fn is_closed(&self) -> bool {
        self.input_facts_total == self.normalized_facts_total
            && self.normalized_facts_total == self.facts.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocsReport {
    pub schema: String,
    pub standard: String,
    pub root: String,
    pub gate_ready: bool,
    pub hard_violations_total: usize,
    pub documents_total: usize,
    pub canonical_frontmatter_total: usize,
    pub required_control_docs_missing: Vec<String>,
    pub missing_frontmatter: Vec<String>,
    pub documents: Vec<DocumentFact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentFact {
    pub path: String,
    pub id: Option<String>,
    pub kind: Option<String>,
    pub status: Option<String>,
    pub canonical: bool,
    pub title: Option<String>,
    pub headings: Vec<String>,
    pub references: Vec<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphSummary {
    pub schema: String,
    pub semantic_grade: String,
    pub nodes_total: usize,
    pub edges_total: usize,
    pub bindings_total: usize,
    pub facts_total: usize,
    pub language_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemizeReport {
    pub schema: String,
    pub cli_api: String,
    pub root: String,
    pub snapshot: RepositorySnapshot,
    pub repository: RepoAudit,
    pub docs: DocsReport,
    pub adl: AdlCompileReport,
    pub coding_admission: CodingAdmission,
    pub inventory: InventoryReport,
    pub source: SourceReport,
    pub census: CensusReport,
    pub normalization: NormalizationReport,
    pub graph: GraphSummary,
    pub invariants: Vec<String>,
}
