//! Atlas core: product-neutral engineering semantics.
//!
//! This crate owns the vocabulary that every repository compilation converges on: facts,
//! nodes, edges, bindings, revisions, evidence and policy. It performs no filesystem, Git,
//! provider, donor or UI work.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const CLI_API: &str = "atlas.systemizer.cli.v1";
pub const BINARY: &str = "atlas-systemizer";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Contract {
    pub schema: String,
    pub binary: String,
    pub subsystem_kind: String,
    pub runtime_dependency_allowed: bool,
    pub commands: Vec<String>,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            schema: CLI_API.to_owned(),
            binary: BINARY.to_owned(),
            subsystem_kind: "SYSTEM_INVENTION_FORGE".to_owned(),
            runtime_dependency_allowed: false,
            commands: vec![
                "contract".into(),
                "systemize".into(),
                "docs audit".into(),
                "code analyze".into(),
                "work prepare".into(),
            ],
        }
    }
}

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
pub struct SystemizeReport {
    pub schema: String,
    pub cli_api: String,
    pub root: String,
    pub repository: RepoAudit,
    pub docs: DocsReport,
    pub coding_admission: CodingAdmission,
    pub source: SourceReport,
    pub graph: GraphSummary,
    pub invariants: Vec<String>,
}

pub fn stable_id(prefix: &str, identity: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in identity.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{prefix}:{hash:016x}")
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

pub fn build_source_graph(source: &SourceReport) -> EngineeringGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut facts = Vec::new();
    let repo_id = stable_id("node", &source.root);
    nodes.push(Node {
        id: repo_id.clone(),
        kind: "Repository".into(),
        identity: source.root.clone(),
        attributes: BTreeMap::new(),
        provenance: provenance(".atlas/repo.toml", "atlas-core.graph-bootstrap"),
        revision: None,
    });
    let mut language_ids = BTreeMap::new();
    for language in source.languages.keys() {
        let id = stable_id("node", &format!("technology:{language}"));
        language_ids.insert(language.clone(), id.clone());
        nodes.push(Node {
            id,
            kind: "Technology".into(),
            identity: language.clone(),
            attributes: BTreeMap::from([("domain".into(), "language".into())]),
            provenance: provenance(".atlas/repo.toml", "atlas-core.graph-bootstrap"),
            revision: None,
        });
    }
    for file in &source.files {
        let file_id = stable_id("node", &format!("file:{}", file.path));
        nodes.push(Node {
            id: file_id.clone(),
            kind: "File".into(),
            identity: file.path.clone(),
            attributes: BTreeMap::from([
                ("language".into(), file.language.clone()),
                ("bytes".into(), file.bytes.to_string()),
            ]),
            provenance: provenance(&file.path, "atlas-core.graph-bootstrap"),
            revision: None,
        });
        edges.push(Edge {
            id: stable_id("edge", &format!("{repo_id}:CONTAINS:{file_id}")),
            kind: "CONTAINS".into(),
            from: repo_id.clone(),
            to: file_id.clone(),
            attributes: BTreeMap::new(),
            provenance: provenance(&file.path, "atlas-core.graph-bootstrap"),
            revision: None,
        });
        if let Some(language_id) = language_ids.get(&file.language) {
            edges.push(Edge {
                id: stable_id("edge", &format!("{file_id}:USES:{language_id}")),
                kind: "USES".into(),
                from: file_id.clone(),
                to: language_id.clone(),
                attributes: BTreeMap::new(),
                provenance: provenance(&file.path, "atlas-core.graph-bootstrap"),
                revision: None,
            });
        }
        facts.push(Fact {
            id: stable_id("fact", &format!("{}:language:{}", file.path, file.language)),
            subject: file_id,
            predicate: "language".into(),
            object: file.language.clone(),
            provenance: provenance(&file.path, "atlas-core.graph-bootstrap"),
            confidence: Some(1.0),
        });
    }
    EngineeringGraph {
        schema: "atlas.engineering-graph.v1".into(),
        nodes,
        edges,
        bindings: Vec::new(),
        facts,
        evidence: Vec::new(),
    }
}

pub fn build_repository_graph(source: &SourceReport, docs: &DocsReport) -> EngineeringGraph {
    let mut graph = build_source_graph(source);
    let mut node_ids_by_identity = BTreeMap::new();
    for node in &graph.nodes {
        node_ids_by_identity.insert(node.identity.clone(), node.id.clone());
    }

    for document in &docs.documents {
        let document_id = stable_id("node", &format!("document:{}", document.path));
        let mut attributes = BTreeMap::new();
        if let Some(id) = &document.id {
            attributes.insert("document_id".into(), id.clone());
        }
        if let Some(kind) = &document.kind {
            attributes.insert("document_type".into(), kind.clone());
        }
        if let Some(status) = &document.status {
            attributes.insert("status".into(), status.clone());
        }
        if let Some(title) = &document.title {
            attributes.insert("title".into(), title.clone());
        }
        attributes.insert("canonical".into(), document.canonical.to_string());
        graph.nodes.push(Node {
            id: document_id.clone(),
            kind: "Document".into(),
            identity: document.path.clone(),
            attributes,
            provenance: provenance(&document.path, "atlas-core.document-graph"),
            revision: None,
        });

        for heading in &document.headings {
            let section_id = stable_id(
                "node",
                &format!("document-section:{}:{heading}", document.path),
            );
            graph.nodes.push(Node {
                id: section_id.clone(),
                kind: "DocumentSection".into(),
                identity: format!("{}#{heading}", document.path),
                attributes: BTreeMap::from([("heading".into(), heading.clone())]),
                provenance: provenance(&document.path, "atlas-core.document-graph"),
                revision: None,
            });
            graph.edges.push(Edge {
                id: stable_id("edge", &format!("{document_id}:CONTAINS:{section_id}")),
                kind: "CONTAINS".into(),
                from: document_id.clone(),
                to: section_id,
                attributes: BTreeMap::new(),
                provenance: provenance(&document.path, "atlas-core.document-graph"),
                revision: None,
            });
        }

        for reference in &document.references {
            if let Some(target_id) = node_ids_by_identity.get(reference) {
                let edge_id = stable_id("edge", &format!("{document_id}:DOCUMENTS:{target_id}"));
                graph.edges.push(Edge {
                    id: edge_id,
                    kind: "DOCUMENTS".into(),
                    from: document_id.clone(),
                    to: target_id.clone(),
                    attributes: BTreeMap::from([("reference".into(), reference.clone())]),
                    provenance: provenance(&document.path, "atlas-core.document-graph"),
                    revision: None,
                });
                graph.bindings.push(Binding {
                    id: stable_id(
                        "binding",
                        &format!("{document_id}:DocumentationBinding:{target_id}"),
                    ),
                    source: document_id.clone(),
                    target: target_id.clone(),
                    binding_kind: "DocumentationBinding".into(),
                    confidence: 0.85,
                    evidence: vec![document.path.clone(), reference.clone()],
                    revision: None,
                });
            }
        }
    }

    graph
}

pub fn summarize_graph(source: &SourceReport) -> GraphSummary {
    let graph = build_source_graph(source);
    summarize_engineering_graph(&graph, source)
}

pub fn summarize_repository_graph(source: &SourceReport, docs: &DocsReport) -> GraphSummary {
    let graph = build_repository_graph(source, docs);
    summarize_engineering_graph(&graph, source)
}

fn summarize_engineering_graph(graph: &EngineeringGraph, source: &SourceReport) -> GraphSummary {
    let mut language_nodes: Vec<_> = source.languages.keys().cloned().collect();
    language_nodes.sort();
    GraphSummary {
        schema: "atlas.systemizer.engineering-graph-summary.v1".into(),
        semantic_grade: "SOURCE_FACT_GRAPH".into(),
        nodes_total: graph.nodes.len(),
        edges_total: graph.edges.len(),
        bindings_total: graph.bindings.len(),
        facts_total: graph.facts.len(),
        language_nodes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_fact_compiles_to_graph_primitives() {
        let source = SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 1,
            languages: BTreeMap::from([("rust".into(), 1)]),
            files: vec![FileFact {
                path: "core/src/lib.rs".into(),
                language: "rust".into(),
                bytes: 10,
            }],
        };
        let graph = build_source_graph(&source);
        assert!(graph.nodes.iter().any(|node| node.kind == "Repository"));
        assert!(graph.nodes.iter().any(|node| node.kind == "File"));
        assert!(graph.edges.iter().any(|edge| edge.kind == "CONTAINS"));
        assert!(graph.edges.iter().any(|edge| edge.kind == "USES"));
        assert_eq!(graph.facts.len(), 1);
    }
}
