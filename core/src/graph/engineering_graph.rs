use super::{Binding, Edge, EngineeringGraph, Fact, Node};
use crate::{
    identity::stable_id,
    language::adl::{AdlCompileReport, SourceSpan},
    provenance::provenance,
    schema::{DocsReport, GraphSummary, SourceReport},
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

fn span_attributes(span: &SourceSpan) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("source_path".into(), span.path.clone()),
        ("source_line".into(), span.line.to_string()),
        ("source_column".into(), span.column.to_string()),
    ])
}

fn declared_node_id(name: &str) -> String {
    stable_id("node", &format!("declared:{name}"))
}

pub fn build_system_graph(
    source: &SourceReport,
    docs: &DocsReport,
    adl: &AdlCompileReport,
) -> EngineeringGraph {
    let mut graph = build_repository_graph(source, docs);
    let mut file_ids_by_path = BTreeMap::new();
    for node in &graph.nodes {
        if node.kind == "File" {
            file_ids_by_path.insert(node.identity.clone(), node.id.clone());
        }
    }

    for declared in &adl.ir.declared.nodes {
        let mut attributes = declared.attributes.clone();
        attributes.insert("origin".into(), declared.origin.clone());
        attributes.insert("declared_name".into(), declared.name.clone());
        attributes.extend(span_attributes(&declared.span));
        graph.nodes.push(Node {
            id: declared_node_id(&declared.name),
            kind: declared.node_kind.clone(),
            identity: declared.name.clone(),
            attributes,
            provenance: provenance(&declared.span.path, "atlas-core.adl-graph"),
            revision: None,
        });
    }

    for declared in &adl.ir.declared.edges {
        let from = declared_node_id(&declared.from);
        let to = declared_node_id(&declared.to);
        graph.edges.push(Edge {
            id: stable_id(
                "edge",
                &format!(
                    "declared:{}:{}:{}",
                    declared.from, declared.relation, declared.to
                ),
            ),
            kind: declared.relation.to_ascii_uppercase(),
            from,
            to,
            attributes: BTreeMap::from([
                ("origin".into(), declared.origin.clone()),
                ("declared_relation".into(), declared.relation.clone()),
            ]),
            provenance: provenance(&declared.span.path, "atlas-core.adl-graph"),
            revision: None,
        });
    }

    for binding in &adl.ir.declared.bindings {
        graph.bindings.push(Binding {
            id: stable_id("binding", &format!("adl-binding:{}", binding.name)),
            source: declared_node_id(&binding.consumer),
            target: declared_node_id(&binding.provider),
            binding_kind: "DeclaredCapabilityBinding".into(),
            confidence: 1.0,
            evidence: vec![
                binding.span.path.clone(),
                binding.consumer.clone(),
                binding.provider.clone(),
                binding.capability.clone(),
            ],
            revision: None,
        });
        graph.edges.push(Edge {
            id: stable_id(
                "edge",
                &format!("{}:BINDS_TO:{}", binding.consumer, binding.provider),
            ),
            kind: "BINDS_TO".into(),
            from: declared_node_id(&binding.consumer),
            to: declared_node_id(&binding.provider),
            attributes: BTreeMap::from([
                ("binding".into(), binding.name.clone()),
                ("capability".into(), binding.capability.clone()),
                ("origin".into(), "declared".into()),
            ]),
            provenance: provenance(&binding.span.path, "atlas-core.adl-graph"),
            revision: None,
        });
    }

    for materialization in &adl.ir.declared.materializations {
        let materialization_id = stable_id(
            "node",
            &format!(
                "materialization:{}:{}",
                materialization.target, materialization.path
            ),
        );
        let mut attributes = BTreeMap::from([
            ("target".into(), materialization.target.clone()),
            ("path".into(), materialization.path.clone()),
            ("origin".into(), "declared".into()),
        ]);
        if let Some(language) = &materialization.source_language {
            attributes.insert("source_language".into(), language.clone());
        }
        if let Some(glob) = &materialization.source_glob {
            attributes.insert("source_glob".into(), glob.clone());
        }
        attributes.extend(span_attributes(&materialization.span));
        graph.nodes.push(Node {
            id: materialization_id.clone(),
            kind: "Materialization".into(),
            identity: format!("{}@{}", materialization.target, materialization.path),
            attributes,
            provenance: provenance(&materialization.span.path, "atlas-core.adl-graph"),
            revision: None,
        });
        graph.edges.push(Edge {
            id: stable_id(
                "edge",
                &format!(
                    "{}:MATERIALIZES:{}",
                    materialization.target, materialization.path
                ),
            ),
            kind: "MATERIALIZES".into(),
            from: declared_node_id(&materialization.target),
            to: materialization_id.clone(),
            attributes: BTreeMap::from([("origin".into(), "declared".into())]),
            provenance: provenance(&materialization.span.path, "atlas-core.adl-graph"),
            revision: None,
        });
        let prefix = materialization.path.trim_end_matches('/');
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
                    provenance: provenance(&materialization.span.path, "atlas-core.adl-graph"),
                    revision: None,
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
                    evidence: vec![materialization.span.path.clone(), path.clone()],
                    revision: None,
                });
            }
        }
    }

    for constraint in &adl.ir.declared.constraints {
        let constraint_id = stable_id("node", &format!("constraint:{}", constraint.name));
        graph.nodes.push(Node {
            id: constraint_id.clone(),
            kind: "Constraint".into(),
            identity: constraint.name.clone(),
            attributes: span_attributes(&constraint.span),
            provenance: provenance(&constraint.span.path, "atlas-core.adl-graph"),
            revision: None,
        });
        graph.facts.push(Fact {
            id: stable_id("fact", &format!("constraint:{}:declared", constraint.name)),
            subject: constraint_id,
            predicate: "origin".into(),
            object: "declared".into(),
            provenance: provenance(&constraint.span.path, "atlas-core.adl-graph"),
            confidence: Some(1.0),
        });
    }

    for result in &adl.constraint_results {
        let result_id = stable_id("node", &format!("constraint-result:{}", result.name));
        graph.nodes.push(Node {
            id: result_id.clone(),
            kind: "Diagnostic".into(),
            identity: format!("constraint-result:{}", result.name),
            attributes: BTreeMap::from([
                ("constraint".into(), result.name.clone()),
                ("passed".into(), result.passed.to_string()),
                ("origin".into(), "derived".into()),
            ]),
            provenance: provenance(".atlas/declared", "atlas-core.adl-constraint-evaluator"),
            revision: None,
        });
        graph.facts.push(Fact {
            id: stable_id("fact", &format!("constraint-result:{}:passed", result.name)),
            subject: result_id,
            predicate: "passed".into(),
            object: result.passed.to_string(),
            provenance: provenance(".atlas/declared", "atlas-core.adl-constraint-evaluator"),
            confidence: Some(1.0),
        });
    }

    for delta in &adl.deltas {
        let delta_id = stable_id("node", &format!("delta:{}:{}", delta.code, delta.subject));
        graph.nodes.push(Node {
            id: delta_id.clone(),
            kind: "Diagnostic".into(),
            identity: format!("{}:{}", delta.code, delta.subject),
            attributes: BTreeMap::from([
                ("code".into(), delta.code.clone()),
                ("message".into(), delta.message.clone()),
                ("subject".into(), delta.subject.clone()),
                ("origin".into(), "derived".into()),
            ]),
            provenance: provenance(".atlas/declared", "atlas-core.adl-delta"),
            revision: None,
        });
        graph.facts.push(Fact {
            id: stable_id("fact", &format!("delta:{}:{}", delta.code, delta.subject)),
            subject: delta_id,
            predicate: "delta".into(),
            object: delta.code.clone(),
            provenance: provenance(".atlas/declared", "atlas-core.adl-delta"),
            confidence: Some(1.0),
        });
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

pub fn summarize_system_graph(
    source: &SourceReport,
    docs: &DocsReport,
    adl: &AdlCompileReport,
) -> GraphSummary {
    let graph = build_system_graph(source, docs, adl);
    let mut summary = summarize_engineering_graph(&graph, source);
    summary.semantic_grade = "ADL_DECLARED_OBSERVED_GRAPH".into();
    summary
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
    use crate::language::adl::compile_adl;

    #[test]
    fn file_fact_compiles_to_graph_primitives() {
        let source = SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 1,
            languages: BTreeMap::from([("rust".into(), 1)]),
            files: vec![crate::schema::FileFact {
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

    #[test]
    fn adl_declarations_materialize_into_engineering_graph() {
        let source = SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 1,
            languages: BTreeMap::from([("rust".into(), 1)]),
            files: vec![crate::schema::FileFact {
                path: "core/src/lib.rs".into(),
                language: "rust".into(),
                bytes: 10,
            }],
        };
        let docs = DocsReport {
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
        };
        let adl_source = crate::language::adl::AdlSource {
            path: ".atlas/declared/system.adl".into(),
            text: r#"atlas 1
system Example
entity Runtime Core {
    kind = backend
    language = rust
}
capability CompileGraph {
    input = AST
    output = SystemGraph
}
Core ->provides-> CompileGraph
binding CoreCompiler {
    consumer = Core
    provider = Core
    capability = CompileGraph
}
materialize Core {
    path = "core"
    language = rust
}
constraint BackendIsRust {
    forall x: Runtime
        where x.kind == backend
    require x.language == rust
}
"#
            .into(),
        };
        let adl = compile_adl(&[adl_source], &source);
        let graph = build_system_graph(&source, &docs, &adl);
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
