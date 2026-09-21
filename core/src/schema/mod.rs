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
    Symbol,
    Type,
    FunctionIdentity,
    FunctionSignature,
    Call,
    ControlFlow,
    DataFlow,
    StateAccess,
    Effect,
    Ownership,
    Concurrency,
    Persistence,
    EvidenceLink,
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
            Self::Symbol => "SYMBOL",
            Self::Type => "TYPE",
            Self::FunctionIdentity => "FUNCTION_IDENTITY",
            Self::FunctionSignature => "FUNCTION_SIGNATURE",
            Self::Call => "CALL",
            Self::ControlFlow => "CONTROL_FLOW",
            Self::DataFlow => "DATA_FLOW",
            Self::StateAccess => "STATE_ACCESS",
            Self::Effect => "EFFECT",
            Self::Ownership => "OWNERSHIP",
            Self::Concurrency => "CONCURRENCY",
            Self::Persistence => "PERSISTENCE",
            Self::EvidenceLink => "EVIDENCE_LINK",
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
    Unsupported,
    Unknown,
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
            Self::Unsupported => "UNSUPPORTED",
            Self::Unknown => "UNKNOWN",
            Self::Ignored => "IGNORED",
        }
    }
}

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
    pub coverage: BTreeMap<String, String>,
    pub facts: Vec<SemanticFact>,
}

impl CensusReport {
    pub fn is_closed(&self) -> bool {
        self.artifacts_total == self.artifacts_accounted_total
            && self.facts_total == self.facts.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizationConflict {
    pub id: String,
    pub slot: String,
    pub fact_ids: Vec<String>,
    pub objects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizationReport {
    pub schema: String,
    pub input_facts_total: usize,
    pub normalized_facts_total: usize,
    pub equivalence_classes_total: usize,
    pub duplicate_observations_total: usize,
    pub conflict_candidates_total: usize,
    pub conflict_candidates: Vec<NormalizationConflict>,
    pub kinds: BTreeMap<String, usize>,
    pub facts: Vec<SemanticFact>,
}

impl NormalizationReport {
    pub fn is_closed(&self) -> bool {
        self.input_facts_total == self.normalized_facts_total
            && self.normalized_facts_total == self.facts.len()
            && self.equivalence_classes_total <= self.normalized_facts_total
            && self.duplicate_observations_total
                == self.normalized_facts_total.saturating_sub(self.equivalence_classes_total)
            && self.conflict_candidates_total == self.conflict_candidates.len()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CensusCertificateState {
    Draft,
    Censused,
    Reconciled,
    Closed,
    Sealed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SemanticCoverageSummary {
    pub required: bool,
    pub total: usize,
    pub observed: usize,
    pub declared: usize,
    pub derived: usize,
    pub inferred: usize,
    pub unknown: usize,
    pub unsupported: usize,
    pub conflict: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CensusCertificate {
    pub schema: String,
    pub state: CensusCertificateState,
    pub corpus_id: String,
    pub revisions: Vec<String>,
    pub genome_hash: String,
    pub policy_id: Option<String>,
    pub inventory_total: usize,
    pub accounted_total: usize,
    pub semantic_coverage: BTreeMap<String, SemanticCoverageSummary>,
    pub unresolved_artifacts: usize,
    pub unsupported_artifacts: usize,
    pub dynamic_edges: usize,
    pub binding_gaps: usize,
    pub conflicts: usize,
    pub fixed_point_iterations: usize,
    pub independent_pass_agreement: Vec<String>,
    pub atlas_root_hash: Option<String>,
}

impl CensusCertificate {
    pub fn accounting_closed(&self) -> bool {
        self.inventory_total == self.accounted_total
    }

    pub fn has_fixed_point_evidence(&self) -> bool {
        self.fixed_point_iterations > 0
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
