use super::{Binding, Edge, EngineeringGraph, Fact, Node};
use crate::{
    identity::stable_id,
    provenance::{Provenance, provenance},
    schema::{
        DocsReport, EpistemicStatus, GraphSummary, NormalizationReport, SemanticFact,
        SemanticFactKind, SourceReport,
    },
    semantic::SemanticObservation,
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
    provenance: &Provenance,
) {
    if graph.nodes.iter().any(|node| node.id == id) {
        return;
    }
    graph.nodes.push(Node {
        id,
        kind,
        identity,
        attributes,
        provenance: provenance.clone(),
        revision: provenance.source_revision.clone(),
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
                    &fact.provenance,
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
                    &fact.provenance,
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
                    &fact.provenance,
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
                    &fact.provenance,
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
                    &fact.provenance,
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
            // R4.3.3: Symbol/Type/FunctionIdentity/FunctionSignature nodes are no longer created
            // from this compatibility-fact loop. They are now built directly from
            // `normalization.typed_semantic_records` by `add_typed_semantic_nodes` below, so
            // semantic graph identity no longer depends on the lossy `SemanticFact` projection
            // (`.atlas/contracts/SEMANTIC-EXTRACTION.md#r4-acceptance-matrix`: "SemanticFact
            // remains a compatibility envelope, not the only semantic type system"). These facts
            // still reach `graph.facts` via `add_normalized_fact` above (legacy/report
            // convenience), but that projection is no longer a second path to graph truth: adding
            // or removing it changes no node, no edge, and no binding.
            SemanticFactKind::Symbol
            | SemanticFactKind::Type
            | SemanticFactKind::FunctionIdentity
            | SemanticFactKind::FunctionSignature => {}
            // Per-artifact obligation-status bookkeeping (OBSERVED/UNKNOWN/UNSUPPORTED for one
            // dimension): already recorded via `add_normalized_fact` above; no dedicated node,
            // exactly like `SourceArtifact`'s "language" fact.
            SemanticFactKind::SemanticObligation => {}
        }
    }

    add_typed_semantic_nodes(&mut graph, &normalization.typed_semantic_records);

    graph
}

fn scoped_identity(scope: &crate::semantic::SemanticScope, name: &str) -> String {
    if scope.segments.is_empty() {
        name.to_owned()
    } else {
        format!("{}::{}", scope.join(), name)
    }
}

fn function_signature_identity(signature: &crate::semantic::FunctionSignature) -> String {
    let params = signature
        .parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, parameter.type_identity.name))
        .collect::<Vec<_>>()
        .join(", ");
    let return_type = signature
        .return_type
        .as_ref()
        .map(|type_identity| type_identity.name.clone())
        .unwrap_or_else(|| "()".to_owned());
    let asyncness = if signature.is_async { "async " } else { "" };
    let unsafety = if signature.is_unsafe { "unsafe " } else { "" };
    format!("{asyncness}{unsafety}fn({params}) -> {return_type}")
}

/// R4.3.3: the authoritative source for SYMBOL/TYPE/FUNCTION_IDENTITY/FUNCTION_SIGNATURE graph
/// nodes -- built directly from typed `SemanticObservation` records, never by parsing
/// `SemanticFact.object` and never gated on the compatibility fact existing at all
/// (`.atlas/decisions/0001-one-normalized-semantic-path.md`: no parser/extractor bypass may create
/// canonical semantics, but the typed kernel record is exactly the *normalized* semantic path this
/// graph is meant to project from -- the lossy string projection was the bypass). Each observation
/// gets a lightweight node keyed by its `record_id`; deeper graph modeling (call edges,
/// control/data-flow projection) remains out of scope until those dimensions themselves become
/// real (R4.4+). CALL/CONTROL_FLOW/DATA_FLOW/STATE/EFFECT observations are matched explicitly (never
/// via a wildcard) precisely so a future dimension is impossible to forget here silently.
fn add_typed_semantic_nodes(
    graph: &mut EngineeringGraph,
    typed_semantic_records: &[SemanticObservation],
) {
    for observation in typed_semantic_records {
        match observation {
            SemanticObservation::Symbol(header) => {
                ensure_node(
                    graph,
                    stable_id("node", &format!("symbol:{}", header.record_id.as_str())),
                    "Symbol".into(),
                    scoped_identity(&header.scope, &header.subject.name),
                    BTreeMap::from([("origin".into(), "semantic-extraction".into())]),
                    &header.provenance,
                );
            }
            SemanticObservation::Type(header) => {
                ensure_node(
                    graph,
                    stable_id("node", &format!("type:{}", header.record_id.as_str())),
                    "Type".into(),
                    header.subject.name.clone(),
                    BTreeMap::from([("origin".into(), "semantic-extraction".into())]),
                    &header.provenance,
                );
            }
            SemanticObservation::FunctionIdentity(header) => {
                ensure_node(
                    graph,
                    stable_id(
                        "node",
                        &format!("function-identity:{}", header.record_id.as_str()),
                    ),
                    "FunctionIdentity".into(),
                    scoped_identity(&header.scope, &header.subject.symbol.name),
                    BTreeMap::from([("origin".into(), "semantic-extraction".into())]),
                    &header.provenance,
                );
            }
            SemanticObservation::FunctionSignature(header) => {
                ensure_node(
                    graph,
                    stable_id(
                        "node",
                        &format!("function-signature:{}", header.record_id.as_str()),
                    ),
                    "FunctionSignature".into(),
                    function_signature_identity(&header.subject),
                    BTreeMap::from([("origin".into(), "semantic-extraction".into())]),
                    &header.provenance,
                );
            }
            // R4.5: one lightweight CallSite node per Call observation, plus a MAKES_CALL edge
            // from the caller's FunctionIdentity node. Never an edge to a callee: every
            // `CallSiteIdentity` produced this wave has `dispatch == Unresolved` and empty
            // `callees` (see `core/src/semantic/call.rs`), so there is nothing to point at yet.
            // The caller's node id is derived the same way `FunctionIdentity`'s own node id is
            // derived above (`function-identity:<record_id>`), so the edge resolves correctly
            // whether or not that FunctionIdentity observation is present in this same batch.
            SemanticObservation::Call(header) => {
                let call_site_id =
                    stable_id("node", &format!("call-site:{}", header.record_id.as_str()));
                ensure_node(
                    graph,
                    call_site_id.clone(),
                    "CallSite".into(),
                    format!(
                        "{}:{}:{}",
                        header.subject.span.path,
                        header.subject.span.line,
                        header.subject.span.column
                    ),
                    BTreeMap::from([
                        ("origin".into(), "semantic-extraction".into()),
                        ("dispatch".into(), header.subject.dispatch.as_str().into()),
                    ]),
                    &header.provenance,
                );
                let caller_node_id = stable_id(
                    "node",
                    &format!("function-identity:{}", header.subject.function.as_str()),
                );
                graph.edges.push(Edge {
                    id: stable_id(
                        "edge",
                        &format!("{caller_node_id}:MAKES_CALL:{call_site_id}"),
                    ),
                    kind: "MAKES_CALL".into(),
                    from: caller_node_id,
                    to: call_site_id,
                    attributes: BTreeMap::from([("origin".into(), "semantic-extraction".into())]),
                    provenance: header.provenance.clone(),
                    revision: header.provenance.source_revision.clone(),
                });
            }
            // Not yet materialized by any extractor (R4.6+); no graph logic to add here until
            // they become real observations. Matched explicitly, never via `_ => {}`, so adding a
            // new dimension's extractor without updating this function fails to compile instead
            // of silently producing no graph nodes.
            SemanticObservation::ControlFlow(_)
            | SemanticObservation::DataFlow(_)
            | SemanticObservation::State(_)
            | SemanticObservation::Effect(_) => {}
        }
    }
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
    use crate::{
        EpistemicStatus, NormalizationReport, Provenance, SemanticFact, SemanticFactKind,
        TypedClosureAccounting,
    };

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
            typed_semantic_records: Vec::new(),
            evidence: Vec::new(),
            diagnostics: Vec::new(),
            typed_obligations: Vec::new(),
            input_typed_closure: TypedClosureAccounting {
                typed_observations_total: 0,
                typed_obligations_total: 0,
            },
            normalized_typed_closure: TypedClosureAccounting {
                typed_observations_total: 0,
                typed_obligations_total: 0,
            },
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

    // === R4.3.3: typed records, not compatibility facts, drive semantic graph identity =========

    fn symbol_observation() -> crate::semantic::SemanticObservation {
        use crate::identity::RepositoryId;
        use crate::provenance::provenance;
        use crate::semantic::{
            ExtractorIdentity, SemanticDimension, SemanticRecordHeader, SemanticRecordId,
            SemanticScope, SymbolIdentity, SymbolRole,
        };
        use crate::temporal::RevisionRef;

        let repository = RepositoryId::new("atlas-studio");
        let revision = RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let scope = SemanticScope::new(Vec::<String>::new());
        let symbol = SymbolIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            scope: scope.clone(),
            name: "known".into(),
            role: SymbolRole::Definition,
        };
        let record_id = SemanticRecordId::new(SemanticDimension::Symbol, &symbol.identity_key());
        crate::semantic::SemanticObservation::Symbol(SemanticRecordHeader {
            record_id,
            dimension: SemanticDimension::Symbol,
            status: EpistemicStatus::Observed,
            subject: symbol,
            scope,
            repository,
            revision,
            extractor: ExtractorIdentity {
                id: "atlas.test".into(),
                version: "0.1.0".into(),
            },
            evidence_refs: Vec::new(),
            provenance: provenance("src/lib.rs", "atlas.test"),
        })
    }

    /// R4.4: a strengthened `FunctionIdentity` observation for an inherent method `get` owned by
    /// `owner_name` (e.g. `"Foo"`/`"Bar"`), so two calls with different `owner_name` produce two
    /// distinct declarations of the same method name -- exactly the shape the graph boundary must
    /// keep as two distinct nodes.
    fn function_identity_observation(owner_name: &str) -> crate::semantic::SemanticObservation {
        use crate::identity::RepositoryId;
        use crate::provenance::provenance;
        use crate::semantic::{
            ExtractorIdentity, FunctionDeclarationKind, FunctionIdentity, FunctionOwner,
            SemanticDimension, SemanticRecordHeader, SemanticRecordId, SemanticScope,
            SymbolIdentity, SymbolRole, TypeIdentity,
        };
        use crate::temporal::RevisionRef;

        let repository = RepositoryId::new("atlas-studio");
        let revision = RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let scope = SemanticScope::new([format!("impl:{owner_name}")]);
        let identity = FunctionIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            language: "rust".into(),
            scope: scope.clone(),
            symbol: SymbolIdentity {
                repository: repository.clone(),
                revision: revision.clone(),
                scope: scope.clone(),
                name: "get".into(),
                role: SymbolRole::Definition,
            },
            span: crate::language::adl::SourceSpan {
                path: "src/lib.rs".into(),
                line: 1,
                column: 1,
            },
            generated: false,
            declaration_kind: FunctionDeclarationKind::InherentMethod,
            owner: FunctionOwner {
                target: Some(TypeIdentity {
                    repository: repository.clone(),
                    revision: revision.clone(),
                    scope: SemanticScope::new(Vec::<String>::new()),
                    name: owner_name.into(),
                    canonical: None,
                }),
                trait_path: None,
            },
            generics: Vec::new(),
        };
        let record_id = SemanticRecordId::new(
            SemanticDimension::FunctionIdentity,
            &identity.identity_key(),
        );
        crate::semantic::SemanticObservation::FunctionIdentity(Box::new(SemanticRecordHeader {
            record_id,
            dimension: SemanticDimension::FunctionIdentity,
            status: EpistemicStatus::Observed,
            subject: identity,
            scope,
            repository,
            revision,
            extractor: ExtractorIdentity {
                id: "atlas.test".into(),
                version: "0.1.0".into(),
            },
            evidence_refs: Vec::new(),
            provenance: provenance("src/lib.rs", "atlas.test"),
        }))
    }

    /// R4.5: a Call observation whose `function` (caller) is the record_id of
    /// `function_identity_observation(owner_name)`'s `FunctionIdentity`, so the two can be
    /// combined in one batch to exercise the MAKES_CALL edge.
    fn call_observation(caller: &crate::semantic::SemanticRecordId) -> SemanticObservation {
        use crate::identity::RepositoryId;
        use crate::provenance::provenance;
        use crate::semantic::{
            CallDispatchKind, CallSiteIdentity, ExtractorIdentity, SemanticDimension,
            SemanticRecordHeader, SemanticRecordId, SemanticScope,
        };
        use crate::temporal::RevisionRef;

        let repository = RepositoryId::new("atlas-studio");
        let revision = RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let subject = CallSiteIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            function: caller.clone(),
            span: crate::language::adl::SourceSpan {
                path: "src/lib.rs".into(),
                line: 5,
                column: 4,
            },
            dispatch: CallDispatchKind::Unresolved,
            callees: Vec::new(),
        };
        let record_id = SemanticRecordId::new(SemanticDimension::Call, &subject.identity_key());
        SemanticObservation::Call(SemanticRecordHeader {
            record_id,
            dimension: SemanticDimension::Call,
            status: EpistemicStatus::Observed,
            subject,
            scope: SemanticScope::new(["impl:owner"]),
            repository,
            revision,
            extractor: ExtractorIdentity {
                id: "atlas.test".into(),
                version: "0.1.0".into(),
            },
            evidence_refs: Vec::new(),
            provenance: provenance("src/lib.rs", "atlas.test"),
        })
    }

    fn normalization_with(
        typed_semantic_records: Vec<crate::semantic::SemanticObservation>,
        facts: Vec<SemanticFact>,
    ) -> NormalizationReport {
        NormalizationReport {
            schema: "test".into(),
            input_facts_total: facts.len(),
            normalized_facts_total: facts.len(),
            kinds: BTreeMap::new(),
            typed_obligations: Vec::new(),
            input_typed_closure: TypedClosureAccounting {
                typed_observations_total: typed_semantic_records.len(),
                typed_obligations_total: 0,
            },
            normalized_typed_closure: TypedClosureAccounting {
                typed_observations_total: typed_semantic_records.len(),
                typed_obligations_total: 0,
            },
            typed_semantic_records,
            evidence: Vec::new(),
            diagnostics: Vec::new(),
            facts,
        }
    }

    #[test]
    fn semantic_graph_projects_from_typed_records_without_compatibility_facts() {
        // No compatibility SemanticFact at all -- `facts` is empty. Under the pre-R4.3.3
        // implementation (which only created Symbol/Type/FunctionIdentity/FunctionSignature nodes
        // while iterating `normalization.facts`), this would produce zero semantic nodes.
        let normalization = normalization_with(vec![symbol_observation()], Vec::new());

        let graph = build_system_graph(&source(), &docs(), &normalization);
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == "Symbol" && node.identity == "known"),
            "a typed Symbol observation must produce a graph node even with no compatibility fact"
        );
    }

    #[test]
    fn compatibility_semantic_fact_does_not_change_the_semantic_graph_result() {
        let observation = symbol_observation();
        let compat_fact = semantic(
            "compat-symbol",
            SemanticFactKind::Symbol,
            EpistemicStatus::Observed,
            observation.record_id().as_str(),
            "declares_symbol",
            "known",
        );

        let without_fact = normalization_with(vec![observation.clone()], Vec::new());
        let with_fact = normalization_with(vec![observation], vec![compat_fact]);

        let graph_without = build_system_graph(&source(), &docs(), &without_fact);
        let graph_with = build_system_graph(&source(), &docs(), &with_fact);

        // Same semantic graph truth (nodes/edges/bindings) either way: the compatibility fact is
        // never a second path to graph truth, only ever additional `graph.facts` bookkeeping.
        assert_eq!(graph_without.nodes, graph_with.nodes);
        assert_eq!(graph_without.edges, graph_with.edges);
        assert_eq!(graph_without.bindings, graph_with.bindings);
        // The compatibility fact IS still visible as report bookkeeping, so the two are not
        // wholesale identical -- only their semantic-graph-truth parts are.
        assert_ne!(graph_without.facts.len(), graph_with.facts.len());
    }

    #[test]
    fn typed_semantic_graph_projection_is_deterministic() {
        let normalization = normalization_with(vec![symbol_observation()], Vec::new());
        let a = build_system_graph(&source(), &docs(), &normalization);
        let b = build_system_graph(&source(), &docs(), &normalization);
        assert_eq!(a, b);
    }

    // --- R4.4 required test 18: distinct FunctionIdentity records => distinct graph nodes -------

    #[test]
    fn distinct_function_identities_never_collapse_to_one_graph_node() {
        let foo_get = function_identity_observation("Foo");
        let bar_get = function_identity_observation("Bar");
        // Same method NAME ("get"), same dimension, different owner -- the pre-R4.4 graph node id
        // was keyed by `fact.subject` (== record_id), so this only stays distinct because R4.4
        // strengthened FunctionIdentity's identity_key to include `owner`.
        assert_ne!(foo_get.record_id(), bar_get.record_id());

        let normalization = normalization_with(vec![foo_get, bar_get], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        let function_identity_nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|node| node.kind == "FunctionIdentity")
            .collect();
        assert_eq!(
            function_identity_nodes.len(),
            2,
            "two distinct FunctionIdentity records (different owner) must produce two distinct \
             graph nodes, never collapse because their method name matches"
        );
        assert_ne!(
            function_identity_nodes[0].id, function_identity_nodes[1].id,
            "graph node ids must differ for distinct FunctionIdentity records"
        );
    }

    // --- R4.4 required test 19: compatibility fact is not a second FunctionIdentity graph source -

    #[test]
    fn function_identity_graph_nodes_do_not_depend_on_the_compatibility_fact() {
        let observation = function_identity_observation("Foo");
        let record_id = observation.record_id().as_str().to_owned();
        let compat_fact = semantic(
            "compat-function-identity",
            SemanticFactKind::FunctionIdentity,
            EpistemicStatus::Observed,
            &record_id,
            "declares_function",
            "Foo::get",
        );

        let without_fact = normalization_with(vec![observation.clone()], Vec::new());
        let with_fact = normalization_with(vec![observation], vec![compat_fact]);

        let graph_without = build_system_graph(&source(), &docs(), &without_fact);
        let graph_with = build_system_graph(&source(), &docs(), &with_fact);

        assert!(
            graph_without
                .nodes
                .iter()
                .any(|node| node.kind == "FunctionIdentity"),
            "a typed FunctionIdentity observation must produce a graph node with zero \
             compatibility facts present"
        );
        assert_eq!(graph_without.nodes, graph_with.nodes);
        assert_eq!(graph_without.edges, graph_with.edges);
        assert_eq!(graph_without.bindings, graph_with.bindings);
    }

    // --- R4.5: CALL observations produce a CallSite node and a MAKES_CALL edge -------------------

    #[test]
    fn call_observation_produces_a_call_site_node_and_makes_call_edge() {
        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();
        let call = call_observation(&caller_record_id);

        let normalization = normalization_with(vec![caller, call], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        let call_site = graph
            .nodes
            .iter()
            .find(|node| node.kind == "CallSite")
            .expect("a CallSite node must exist for the Call observation");

        let caller_node_id = stable_id(
            "node",
            &format!("function-identity:{}", caller_record_id.as_str()),
        );
        let edge = graph
            .edges
            .iter()
            .find(|edge| edge.kind == "MAKES_CALL")
            .expect("a MAKES_CALL edge must exist");
        assert_eq!(edge.from, caller_node_id);
        assert_eq!(edge.to, call_site.id);
        assert_eq!(
            call_site.attributes.get("dispatch").map(String::as_str),
            Some("UNRESOLVED")
        );
    }

    #[test]
    fn call_site_node_id_is_derived_from_the_caller_even_without_its_own_function_identity_observation()
     {
        // The graph must resolve a caller's node id the same way whether or not that caller's own
        // FunctionIdentity observation is present in this exact batch (e.g. a requested-dimension
        // subset that includes CALL but not FUNCTION_IDENTITY).
        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();
        let call = call_observation(&caller_record_id);

        let with_caller = normalization_with(vec![caller.clone(), call.clone()], Vec::new());
        let without_caller = normalization_with(vec![call], Vec::new());

        let graph_with = build_system_graph(&source(), &docs(), &with_caller);
        let graph_without = build_system_graph(&source(), &docs(), &without_caller);

        let edge_with = graph_with
            .edges
            .iter()
            .find(|edge| edge.kind == "MAKES_CALL")
            .expect("MAKES_CALL edge with caller present");
        let edge_without = graph_without
            .edges
            .iter()
            .find(|edge| edge.kind == "MAKES_CALL")
            .expect("MAKES_CALL edge with caller absent");
        assert_eq!(edge_with.from, edge_without.from);
    }

    #[test]
    fn no_call_edge_ever_targets_a_callee_this_wave() {
        // Every Call observation this wave carries an empty `callees` list -- the graph must never
        // synthesize a callee edge from thin air.
        let caller = function_identity_observation("Foo");
        let call = call_observation(&caller.record_id().clone());
        let normalization = normalization_with(vec![caller, call], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);
        assert!(graph.edges.iter().all(|edge| edge.kind != "CALLS"));
    }
}
