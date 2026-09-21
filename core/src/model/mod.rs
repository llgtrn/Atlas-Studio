pub mod engineering_graph;

use crate::{census::InventoryReport, language::adl::AdlCompileReport};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use engineering_graph::{
    build_repository_graph, build_source_graph, build_system_graph, summarize_graph,
    summarize_repository_graph, summarize_system_graph,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevisionRef {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Provenance {
    pub source_path: String,
    pub source_revision: Option<RevisionRef>,
    pub extractor: String,
    pub content_hash: Option<String>,
    pub span: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Node {
    pub id: String,
    pub kind: String,
    pub identity: String,
    pub attributes: BTreeMap<String, String>,
    pub provenance: Provenance,
    pub revision: Option<RevisionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Edge {
    pub id: String,
    pub kind: String,
    pub from: String,
    pub to: String,
    pub attributes: BTreeMap<String, String>,
    pub provenance: Provenance,
    pub revision: Option<RevisionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Binding {
    pub id: String,
    pub source: String,
    pub target: String,
    pub binding_kind: String,
    pub confidence: f32,
    pub evidence: Vec<String>,
    pub revision: Option<RevisionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Fact {
    pub id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub provenance: Provenance,
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Evidence {
    pub id: String,
    pub kind: String,
    pub path: String,
    pub summary: String,
    pub revision: Option<RevisionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepositorySnapshot {
    pub schema: String,
    pub root: String,
    pub head_sha: String,
    pub branch: Option<String>,
    pub dirty: bool,
    pub status_entries: Vec<String>,
}

impl RepositorySnapshot {
    pub fn revision(&self) -> RevisionRef {
        RevisionRef {
            kind: "git".into(),
            value: self.head_sha.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineeringGraph {
    pub schema: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub bindings: Vec<Binding>,
    pub facts: Vec<Fact>,
    pub evidence: Vec<Evidence>,
}

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
pub struct CodingAdmission {
    pub schema: String,
    pub allowed: bool,
    pub docs_standard: String,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkRequest {
    pub schema: String,
    pub repository: String,
    pub base_revision: RevisionRef,
    pub goal: String,
    pub scope: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub required_verification: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkPrepareReport {
    pub schema: String,
    pub request: WorkRequest,
    pub repository: RepoAudit,
    pub snapshot: RepositorySnapshot,
    pub graph: GraphSummary,
    pub coding_admission: CodingAdmission,
    pub allowed: bool,
    pub blockers: Vec<String>,
    pub evidence: Vec<Evidence>,
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
    pub graph: GraphSummary,
    pub invariants: Vec<String>,
}

pub fn provenance(path: impl Into<String>, extractor: impl Into<String>) -> Provenance {
    Provenance {
        source_path: path.into(),
        source_revision: None,
        extractor: extractor.into(),
        content_hash: None,
        span: None,
    }
}

pub fn validate_manifest(manifest: &RepoManifest) -> Vec<String> {
    let mut violations = Vec::new();
    let expected = [
        ("schema", &manifest.schema, "atlas.repo.v2"),
        (
            "system_kind",
            &manifest.system_kind,
            "SYSTEM_INVENTION_FORGE",
        ),
        ("backend_language", &manifest.backend_language, "rust"),
        (
            "frontend_language",
            &manifest.frontend_language,
            "typescript",
        ),
        ("knowledge_root", &manifest.knowledge_root, ".atlas"),
        (
            "temporary_root",
            &manifest.temporary_root,
            ".atlas/temporary",
        ),
        (
            "provenance_root",
            &manifest.provenance_root,
            ".atlas/provenance",
        ),
        ("license_root", &manifest.license_root, ".atlas/licenses"),
    ];
    for (field, actual, required) in expected {
        if actual != required {
            violations.push(format!("{field} must be {required}"));
        }
    }
    let required_true = [
        (
            "coding_requires_docs_gate",
            manifest.coding_requires_docs_gate,
        ),
        (
            "graph_before_code_required",
            manifest.graph_before_code_required,
        ),
        ("exact_base_sha_required", manifest.exact_base_sha_required),
        (
            "single_repository_target_required",
            manifest.single_repository_target_required,
        ),
    ];
    for (field, actual) in required_true {
        if !actual {
            violations.push(format!("{field} must be true"));
        }
    }
    violations
}
