use super::{Binding, Edge, EngineeringGraph, Fact, Node};
use crate::{
    identity::stable_id,
    provenance::provenance,
    schema::{
        DocsReport, EpistemicStatus, GraphSummary, NormalizationReport, SemanticFact,
        SemanticFactKind, SourceReport,
    },
};
use std::collections::BTreeMap;

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
        provenance: provenance(".atlas/repo.toml", "atlas-core.graph-source-projection"),
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
            provenance: provenance(".atlas/repo.toml", "atlas-core.graph-source-projection"),
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
            provenance: provenance(&file.path, "atlas-core.graph-source-projection"),
            revision: None,
        });
        edges.push(Edge {
            id: stable_id("edge", &format!("{repo_id}:CONTAINS:{file_id}")),
            kind: "CONTAINS".into(),
            from: repo_id.clone(),
            to: file_id.clone(),
            attributes: BTreeMap::new(),
            provenance: provenance(&file.path, "atlas-core.graph-source-projection"),
            revision: None,
        });
        if let Some(language_id) = language_ids.get(&file.language) {
            edges.push(Edge {
                id: stable_id("edge", &format!("{file_id}:USES:{language_id}")),
                kind: "USES".into(),
                from: file_id.clone(),
                to: language_id.clone(),
                attributes: BTreeMap::new(),
                provenance: provenance(&file.path, "atlas-core.graph-source-projection"),
                revision: None,
            });
        }
        facts.push(Fact {
            id: stable_id("fact", &format!("{}:language:{}", file.path, file.language)),
            subject: file_id,
            predicate: "language".into(),
            object: file.language.clone(),
            provenance: provenance(&file.path, "atlas-core.graph-source-projection"),
            confidence: Some(1.0),
        });
    }

    EngineeringGraph {
        schema: "atlas.engineering-graph.v2".into(),
        nodes,
        edges,
        bindings: Vec::new(),
        facts,
        evidence: Vec::new(),
    }
}

pub fn build_repository_graph(source: &SourceReport, docs: &DocsReport) -> EngineeringGraph {
    let mut graph = build_source_graph(source);
    let mut node_ids_by_identity = graph
        .nodes
        .iter()
        .map(|node| (node.identity.clone(), node.id.clone()))
        .collect::<BTreeMap<_, _>>();

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
        node_ids_by_identity.insert(document.path.clone(), document_id.clone());

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
                graph.edges.push(Edge {
                    id: stable_id("edge", &format!("{document_id}:DOCUMENTS:{target_id}")),
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

fn declared_node_id(name: &str) -> String {
    stable_id("node", &format!("declared:{name}"))
}

fn confidence(status: &EpistemicStatus) -> Option<f32> {
    match status {
        EpistemicStatus::Observed | EpistemicStatus::Declared => Some(1.0),
        EpistemicStatus::Derived => Some(0.9),
        EpistemicStatus::Inferred
        | EpistemicStatus::Hypothesis
        | EpistemicStatus::Conflict
        | EpistemicStatus::Unsupported
        | EpistemicStatus::Unknown
        | EpistemicStatus::Ignored => None,
    }
}

fn ensure_node(
    graph: &mut EngineeringGraph,
    id: String,
    kind: String,
    identity: String,
    attributes: BTreeMap<String, String>,
    fact: &SemanticFact,
) {
    if graph.nodes.iter().any(|node| node.id == id) {
        return;
    }
    graph.nodes.push(Node {
        id,
        kind,
        identity,
        attributes,
        provenance: fact.provenance.clone(),
        revision: fact.provenance.source_revision.clone(),
    });
}

fn add_normalized_fact(graph: &mut EngineeringGraph, fact: &SemanticFact) {
    graph.facts.push(Fact {
        id: fact.id.clone(),
        subject: fact.subject.clone(),
        predicate: fact.predicate.clone(),
        object: fact.object.clone(),
        provenance: fact.provenance.clone(),
        confidence: confidence(&fact.status),
    });
}

pub fn build_system_graph(
    source: &SourceReport,
    docs: &DocsReport,
    normalization: &NormalizationReport,
) -> EngineeringGraph {
    let mut graph = build_repository_graph(source, docs);
    let repo_id = stable_id("node", &source.root);
    let file_ids_by_path = graph
        .nodes
        .iter()
        .filter(|node| node.kind == "File")
        .map(|node| (node.identity.clone(), node.id.clone()))
        .collect::<BTreeMap<_, _>>();

    for fact in &normalization.facts {
        add_normalized_fact(&mut graph, fact);

        match fact.kind {
            SemanticFactKind::SourceArtifact => {
                if fact.predicate != "disposition" {
                    continue;
                }
                let path = fact.provenance.source_path.clone();
                if file_ids_by_path.contains_key(&path) {
                    continue;
                }
                let artifact_id = stable_id("node", &format!("artifact:{path}"));
                ensure_node(
                    &mut graph,
                    artifact_id.clone(),
                    "Artifact".into(),
                    path.clone(),
                    BTreeMap::from([
                        ("disposition".into(), fact.object.clone()),
                        ("epistemic_status".into(), format!("{:?}", fact.status)),
                    ]),
                    fact,
                );
                graph.edges.push(Edge {
                    id: stable_id("edge", &format!("{repo_id}:CONTAINS:{artifact_id}")),
                    kind: "CONTAINS".into(),
                    from: repo_id.clone(),
                    to: artifact_id,
                    attributes: BTreeMap::new(),
                    provenance: fact.provenance.clone(),
                    revision: fact.provenance.source_revision.clone(),
                });
            }
            SemanticFactKind::DeclaredNode => {
                ensure_node(
                    &mut graph,
                    declared_node_id(&fact.subject),
                    fact.object.clone(),
                    fact.subject.clone(),
                    BTreeMap::from([("origin".into(), "normalized-declared".into())]),
                    fact,
                );
            }
            SemanticFactKind::DeclaredEdge => {
                graph.edges.push(Edge {
                    id: stable_id(
                        "edge",
                        &format!(
                            "normalized:{}:{}:{}",
                            fact.subject, fact.predicate, fact.object
                        ),
                    ),
                    kind: fact.predicate.to_ascii_uppercase(),
                    from: declared_node_id(&fact.subject),
                    to: declared_node_id(&fact.object),
                    attributes: BTreeMap::from([("origin".into(), "normalized-declared".into())]),
                    provenance: fact.provenance.clone(),
                    revision: fact.provenance.source_revision.clone(),
                });
            }
            SemanticFactKind::Binding if fact.predicate == "binds_to" => {
                let from = declared_node_id(&fact.subject);
                let to = declared_node_id(&fact.object);
                graph.bindings.push(Binding {
                    id: stable_id(
                        "binding",
                        &format!("normalized:{}:{}", fact.subject, fact.object),
                    ),
                    source: from.clone(),
                    target: to.clone(),
                    binding_kind: "DeclaredCapabilityBinding".into(),
                    confidence: confidence(&fact.status).unwrap_or(0.0),
                    evidence: vec![fact.provenance.source_path.clone()],
                    revision: fact.provenance.source_revision.clone(),
                });
                graph.edges.push(Edge {
                    id: stable_id(
                        "edge",
                        &format!("normalized:{}:BINDS_TO:{}", fact.subject, fact.object),
                    ),
                    kind: "BINDS_TO".into(),
                    from,
                    to,
                    attributes: BTreeMap::from([("origin".into(), "normalized-declared".into())]),
                    provenance: fact.provenance.clone(),
                    revision: fact.provenance.source_revision.clone(),
                });
            }
            SemanticFactKind::Constraint
            | SemanticFactKind::Invariant
            | SemanticFactKind::Transform => {
                ensure_node(
                    &mut graph,
                    stable_id("node", &format!("{}:{}", fact.kind.as_str(), fact.subject)),
                    match fact.kind {
                        SemanticFactKind::Constraint => "Constraint",
                        SemanticFactKind::Invariant => "Invariant",
                        SemanticFactKind::Transform => "Transform",
                        _ => unreachable!(),
                    }
                    .into(),
                    fact.subject.clone(),
                    BTreeMap::from([("origin".into(), "normalized-declared".into())]),
                    fact,
                );
            }
            SemanticFactKind::ConstraintResult | SemanticFactKind::Diagnostic => {
                let diagnostic_id = stable_id(
                    "node",
                    &format!(
                        "diagnostic:{}:{}:{}",
                        fact.subject, fact.predicate, fact.object
                    ),
                );
                ensure_node(
                    &mut graph,
                    diagnostic_id,
                    "Diagnostic".into(),
                    format!("{}:{}", fact.predicate, fact.subject),
                    BTreeMap::from([
                        ("code".into(), fact.predicate.clone()),
                        ("value".into(), fact.object.clone()),
                    ]),
                    fact,
                );
            }
            SemanticFactKind::Materialization => {
                let materialization_id = stable_id(
                    "node",
                    &format!("materialization:{}:{}", fact.subject, fact.object),
                );
                ensure_node(
                    &mut graph,
                    materialization_id.clone(),
                    "Materialization".into(),
                    format!("{}@{}", fact.subject, fact.object),
                    BTreeMap::from([
                        ("target".into(), fact.subject.clone()),
                        ("path".into(), fact.object.clone()),
                        ("origin".into(), "normalized-declared".into()),
                    ]),
                    fact,
                );
                graph.edges.push(Edge {
                    id: stable_id(
                        "edge",
                        &format!("normalized:{}:MATERIALIZES:{}", fact.subject, fact.object),
                    ),
                    kind: "MATERIALIZES".into(),
                    from: declared_node_id(&fact.subject),
                    to: materialization_id.clone(),
                    attributes: BTreeMap::from([("origin".into(), "normalized-declared".into())]),
                    provenance: fact.provenance.clone(),
                    revision: fact.provenance.source_revision.clone(),
                });

                let prefix = fact.object.trim_end_matches('/');
                for (path, file_id) in &file_ids_by_path {
                    if path == prefix
                        || path
                            .strip_prefix(prefix)
                            .is_some_and(|tail| tail.starts_with('/'))
                    {
                        graph.edges.push(Edge {
                            id: stable_id(
                                "edge",
                                &format!("{materialization_id}:MATERIALIZES_AS:{file_id}"),
                            ),
                            kind: "MATERIALIZES_AS".into(),
                            from: materialization_id.clone(),
                            to: file_id.clone(),
                            attributes: BTreeMap::from([("path".into(), path.clone())]),
                            provenance: fact.provenance.clone(),
                            revision: fact.provenance.source_revision.clone(),
                        });
                        graph.bindings.push(Binding {
                            id: stable_id(
                                "binding",
                                &format!("{materialization_id}:MaterializationBinding:{file_id}"),
                            ),
                            source: materialization_id.clone(),
                            target: file_id.clone(),
                            binding_kind: "MaterializationBinding".into(),
                            confidence: 1.0,
                            evidence: vec![fact.provenance.source_path.clone(), path.clone()],
                            revision: fact.provenance.source_revision.clone(),
                        });
                    }
                }
            }
            SemanticFactKind::Binding => {}
        }
    }

    graph
}

pub fn summarize_graph(source: &SourceReport) -> GraphSummary {
    let graph = build_source_graph(source);
    summarize_engineering_graph(&graph, source, "SOURCE_FACT_GRAPH")
}

pub fn summarize_repository_graph(source: &SourceReport, docs: &DocsReport) -> GraphSummary {
    let graph = build_repository_graph(source, docs);
    summarize_engineering_graph(&graph, source, "REPOSITORY_FACT_GRAPH")
}

pub fn summarize_system_graph(
    source: &SourceReport,
    docs: &DocsReport,
    normalization: &NormalizationReport,
) -> GraphSummary {
    let graph = build_system_graph(source, docs, normalization);
    summarize_engineering_graph(&graph, source, "NORMALIZED_SEMANTIC_GRAPH")
}

fn summarize_engineering_graph(
    graph: &EngineeringGraph,
    source: &SourceReport,
    semantic_grade: &str,
) -> GraphSummary {
    let mut language_nodes: Vec<_> = source.languages.keys().cloned().collect();
    language_nodes.sort();
    GraphSummary {
        schema: "atlas.systemizer.engineering-graph-summary.v2".into(),
        semantic_grade: semantic_grade.into(),
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
    use crate::{EpistemicStatus, NormalizationReport, Provenance, SemanticFact, SemanticFactKind};

    fn source() -> SourceReport {
        SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 1,
            languages: BTreeMap::from([("rust".into(), 1)]),
            files: vec![crate::schema::FileFact {
                path: "core/src/lib.rs".into(),
                language: "rust".into(),
                bytes: 10,
            }],
        }
    }

    fn docs() -> DocsReport {
        DocsReport {
            schema: "test".into(),
            standard: "test".into(),
            root: "/repo/.atlas".into(),
            gate_ready: true,
            hard_violations_total: 0,
            documents_total: 0,
            canonical_frontmatter_total: 0,
            required_control_docs_missing: Vec::new(),
            missing_frontmatter: Vec::new(),
            documents: Vec::new(),
        }
    }

    fn semantic(
        id: &str,
        kind: SemanticFactKind,
        status: EpistemicStatus,
        subject: &str,
        predicate: &str,
        object: &str,
    ) -> SemanticFact {
        SemanticFact {
            id: id.into(),
            kind,
            status,
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            provenance: Provenance {
                source_path: ".atlas/declared/system.adl".into(),
                source_revision: None,
                extractor: "test".into(),
                content_hash: None,
                span: Some("1:1".into()),
            },
        }
    }

    #[test]
    fn file_fact_compiles_to_graph_primitives() {
        let graph = build_source_graph(&source());
        assert!(graph.nodes.iter().any(|node| node.kind == "Repository"));
        assert!(graph.nodes.iter().any(|node| node.kind == "File"));
        assert!(graph.edges.iter().any(|edge| edge.kind == "CONTAINS"));
        assert!(graph.edges.iter().any(|edge| edge.kind == "USES"));
        assert_eq!(graph.facts.len(), 1);
    }

    #[test]
    fn normalized_semantics_materialize_into_engineering_graph() {
        let facts = vec![
            semantic(
                "n1",
                SemanticFactKind::DeclaredNode,
                EpistemicStatus::Declared,
                "Core",
                "node_kind",
                "Runtime",
            ),
            semantic(
                "n2",
                SemanticFactKind::DeclaredNode,
                EpistemicStatus::Declared,
                "CompileGraph",
                "node_kind",
                "Capability",
            ),
            semantic(
                "e1",
                SemanticFactKind::DeclaredEdge,
                EpistemicStatus::Declared,
                "Core",
                "provides",
                "CompileGraph",
            ),
            semantic(
                "b1",
                SemanticFactKind::Binding,
                EpistemicStatus::Declared,
                "Core",
                "binds_to",
                "Core",
            ),
            semantic(
                "m1",
                SemanticFactKind::Materialization,
                EpistemicStatus::Declared,
                "Core",
                "materializes_at",
                "core",
            ),
            semantic(
                "c1",
                SemanticFactKind::ConstraintResult,
                EpistemicStatus::Derived,
                "BackendIsRust",
                "passed",
                "true",
            ),
        ];
        let normalization = NormalizationReport {
            schema: "test".into(),
            input_facts_total: facts.len(),
            normalized_facts_total: facts.len(),
            kinds: BTreeMap::new(),
            facts,
        };

        let graph = build_system_graph(&source(), &docs(), &normalization);
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == "Runtime" && node.identity == "Core")
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == "Materialization" && node.identity == "Core@core")
        );
        assert!(graph.edges.iter().any(|edge| edge.kind == "PROVIDES"));
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.kind == "MATERIALIZES_AS")
        );
        assert!(
            graph
                .bindings
                .iter()
                .any(|binding| binding.binding_kind == "MaterializationBinding")
        );
        assert!(
            graph
                .facts
                .iter()
                .any(|fact| fact.predicate == "passed" && fact.object == "true")
        );
    }
}
