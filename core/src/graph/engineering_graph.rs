use super::{Binding, Edge, EngineeringGraph, Fact, Node};
use crate::{
    census::{DependencyClosureReport, DependencyIdentity},
    identity::stable_id,
    provenance::{Provenance, provenance},
    schema::{
        DocsReport, EpistemicStatus, GraphSummary, NormalizationReport, SemanticFact,
        SemanticFactKind, SourceReport,
    },
    semantic::{SemanticDimension, SemanticObservation, SemanticRecordId},
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

/// Reconstructs the graph node id that `dimension`'s own observation-handling arm above already
/// produced for `record_id` (e.g. `"state-access:{record_id}"` for a STATE access). Used by
/// PERSISTENCE's `PlaceRef::Resolved` projection to draw an edge at an EXISTING node rather than
/// inventing a new one -- see `SemanticObservation::Persistence`'s own doc comment. Every
/// dimension `PlaceRef::Resolved` can legitimately name is listed explicitly; an unexpected
/// dimension falls back to a generic prefix rather than panicking, since this is graph
/// PROJECTION, never a source of new semantic identity truth.
fn place_node_id_for(dimension: SemanticDimension, record_id: &SemanticRecordId) -> String {
    let prefix = match dimension {
        SemanticDimension::DataFlow => "data-flow-value",
        SemanticDimension::State => "state-access",
        SemanticDimension::Ownership => "ownership-op",
        SemanticDimension::Call => "call-site",
        SemanticDimension::ControlFlow => "control-flow-block",
        SemanticDimension::Concurrency => "concurrency-op",
        SemanticDimension::Persistence => "persistence-op",
        SemanticDimension::Symbol
        | SemanticDimension::Type
        | SemanticDimension::FunctionIdentity
        | SemanticDimension::FunctionSignature
        | SemanticDimension::Effect => "semantic-record",
    };
    stable_id("node", &format!("{prefix}:{}", record_id.as_str()))
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
            // R4.6: one ControlFlowBlock node per block observation, a HAS_BLOCK edge from the
            // owning function's FunctionIdentity node (present for every block, not only the
            // entry, so every block belonging to a function is discoverable without scanning
            // record_ids), and one graph edge per successor whose `target` is concrete (`Some`).
            // Successors with `target: None` (Return/Panic/Unresolved -- see
            // `core/src/semantic/control_flow.rs`) leave the function/this block's reach
            // entirely and correctly produce no graph edge; they remain visible as block
            // attributes/evidence, never silently dropped.
            SemanticObservation::ControlFlow(header) => {
                let block_node_id = stable_id(
                    "node",
                    &format!("control-flow-block:{}", header.record_id.as_str()),
                );
                ensure_node(
                    graph,
                    block_node_id.clone(),
                    "ControlFlowBlock".into(),
                    format!(
                        "{}:block#{}",
                        header.subject.function.as_str(),
                        header.subject.block_index
                    ),
                    BTreeMap::from([
                        ("origin".into(), "semantic-extraction".into()),
                        ("kind".into(), header.subject.kind.as_str().into()),
                        ("is_entry".into(), header.subject.is_entry.to_string()),
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
                        &format!("{caller_node_id}:HAS_BLOCK:{block_node_id}"),
                    ),
                    kind: "HAS_BLOCK".into(),
                    from: caller_node_id,
                    to: block_node_id.clone(),
                    attributes: BTreeMap::from([("origin".into(), "semantic-extraction".into())]),
                    provenance: header.provenance.clone(),
                    revision: header.provenance.source_revision.clone(),
                });
                for successor in &header.subject.successors {
                    let Some(target) = &successor.target else {
                        continue;
                    };
                    let target_node_id =
                        stable_id("node", &format!("control-flow-block:{}", target.as_str()));
                    let edge_kind = successor.kind.as_str();
                    graph.edges.push(Edge {
                        id: stable_id(
                            "edge",
                            &format!("{block_node_id}:{edge_kind}:{target_node_id}"),
                        ),
                        kind: edge_kind.into(),
                        from: block_node_id.clone(),
                        to: target_node_id,
                        attributes: BTreeMap::from([(
                            "origin".into(),
                            "semantic-extraction".into(),
                        )]),
                        provenance: header.provenance.clone(),
                        revision: header.provenance.source_revision.clone(),
                    });
                }
            }
            // R4.7: one Value node per DataFlow observation, a HAS_VALUE edge from the owning
            // function (present for every value event, mirroring R4.6's HAS_BLOCK), and a
            // RESOLVES_TO edge from a Use/Store to the Definition it resolved to, when resolved.
            // An Unresolved Use/Store (resolved_definition=None) correctly produces no such edge
            // -- never a dangling or fabricated resolution.
            SemanticObservation::DataFlow(header) => {
                let value_node_id = stable_id(
                    "node",
                    &format!("data-flow-value:{}", header.record_id.as_str()),
                );
                ensure_node(
                    graph,
                    value_node_id.clone(),
                    "Value".into(),
                    format!(
                        "{}@{}:{}:{}",
                        header.subject.name,
                        header.subject.span.path,
                        header.subject.span.line,
                        header.subject.span.column
                    ),
                    BTreeMap::from([
                        ("origin".into(), "semantic-extraction".into()),
                        ("role".into(), header.subject.role.as_str().into()),
                        (
                            "resolution".into(),
                            header.subject.resolution.as_str().into(),
                        ),
                        (
                            "is_parameter".into(),
                            header.subject.is_parameter.to_string(),
                        ),
                        (
                            "is_return_flow".into(),
                            header.subject.is_return_flow.to_string(),
                        ),
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
                        &format!("{caller_node_id}:HAS_VALUE:{value_node_id}"),
                    ),
                    kind: "HAS_VALUE".into(),
                    from: caller_node_id,
                    to: value_node_id.clone(),
                    attributes: BTreeMap::from([("origin".into(), "semantic-extraction".into())]),
                    provenance: header.provenance.clone(),
                    revision: header.provenance.source_revision.clone(),
                });
                if let Some(resolved) = &header.subject.resolved_definition {
                    let def_node_id =
                        stable_id("node", &format!("data-flow-value:{}", resolved.as_str()));
                    graph.edges.push(Edge {
                        id: stable_id(
                            "edge",
                            &format!("{value_node_id}:RESOLVES_TO:{def_node_id}"),
                        ),
                        kind: "RESOLVES_TO".into(),
                        from: value_node_id,
                        to: def_node_id,
                        attributes: BTreeMap::from([(
                            "origin".into(),
                            "semantic-extraction".into(),
                        )]),
                        provenance: header.provenance.clone(),
                        revision: header.provenance.source_revision.clone(),
                    });
                }
            }
            // R4.8: one StateAccess node per observation, a HAS_ACCESS edge from the owning
            // function (mirroring R4.6/R4.7's HAS_BLOCK/HAS_VALUE), and a TARGETS edge to a
            // StateEntity node. The entity itself is not its own top-level SemanticRecord this
            // wave -- its identity is fully determined by (scope, name), so the graph projection
            // derives a stable node id from that content directly (the same pattern already used
            // for a source-file's Technology node, derived from a language string rather than its
            // own semantic record), letting every access to the same field converge on one node.
            SemanticObservation::State(header) => {
                let access_node_id = stable_id(
                    "node",
                    &format!("state-access:{}", header.record_id.as_str()),
                );
                ensure_node(
                    graph,
                    access_node_id.clone(),
                    "StateAccess".into(),
                    format!("{}:{}", header.subject.name, header.subject.kind.as_str()),
                    BTreeMap::from([
                        ("origin".into(), "semantic-extraction".into()),
                        ("status".into(), header.status.as_str().into()),
                        ("kind".into(), header.subject.kind.as_str().into()),
                        (
                            "resolution".into(),
                            header.subject.resolution.as_str().into(),
                        ),
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
                        &format!("{caller_node_id}:HAS_ACCESS:{access_node_id}"),
                    ),
                    kind: "HAS_ACCESS".into(),
                    from: caller_node_id,
                    to: access_node_id.clone(),
                    attributes: BTreeMap::from([("origin".into(), "semantic-extraction".into())]),
                    provenance: header.provenance.clone(),
                    revision: header.provenance.source_revision.clone(),
                });
                let entity_node_id = stable_id(
                    "node",
                    &format!(
                        "state-entity:{}:{}",
                        header.subject.scope.join(),
                        header.subject.name
                    ),
                );
                ensure_node(
                    graph,
                    entity_node_id.clone(),
                    "StateEntity".into(),
                    format!("{}.{}", header.subject.scope.join(), header.subject.name),
                    BTreeMap::from([("origin".into(), "semantic-extraction".into())]),
                    &header.provenance,
                );
                graph.edges.push(Edge {
                    id: stable_id(
                        "edge",
                        &format!("{access_node_id}:TARGETS:{entity_node_id}"),
                    ),
                    kind: "TARGETS".into(),
                    from: access_node_id,
                    to: entity_node_id,
                    attributes: BTreeMap::from([("origin".into(), "semantic-extraction".into())]),
                    provenance: header.provenance.clone(),
                    revision: header.provenance.source_revision.clone(),
                });
            }
            // R4.8: one Effect node per observation and a PRODUCES_EFFECT edge from the owning
            // function. Only `EffectCategory::Panic` is materialized this wave (see
            // `core::semantic::effect`'s module doc comment), but the projection itself handles
            // every category uniformly -- no per-category graph logic to extend later.
            SemanticObservation::Effect(header) => {
                let effect_node_id =
                    stable_id("node", &format!("effect:{}", header.record_id.as_str()));
                ensure_node(
                    graph,
                    effect_node_id.clone(),
                    "Effect".into(),
                    format!(
                        "{}@{}:{}:{}",
                        header.subject.category.as_str(),
                        header.subject.span.path,
                        header.subject.span.line,
                        header.subject.span.column
                    ),
                    BTreeMap::from([
                        ("origin".into(), "semantic-extraction".into()),
                        ("status".into(), header.status.as_str().into()),
                        ("category".into(), header.subject.category.as_str().into()),
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
                        &format!("{caller_node_id}:PRODUCES_EFFECT:{effect_node_id}"),
                    ),
                    kind: "PRODUCES_EFFECT".into(),
                    from: caller_node_id,
                    to: effect_node_id,
                    attributes: BTreeMap::from([("origin".into(), "semantic-extraction".into())]),
                    provenance: header.provenance.clone(),
                    revision: header.provenance.source_revision.clone(),
                });
            }
            // R4.9: one OwnershipOperation node per observation, a HAS_OPERATION edge from the
            // owning function (mirroring R4.8's HAS_ACCESS), and a TARGETS edge to an
            // OwnershipTarget node. When `resolution` is `Resolved` (a bare-identifier referent),
            // the target node id is derived from (function, name) content -- the same
            // content-derived-node pattern R4.8 already uses for StateEntity, letting every
            // borrow/move of the same local converge on one node. When `resolution` is
            // `Unresolved` (the referent is an unresolved temporary expression, e.g. `&foo()`),
            // `name` is only that expression's textual spelling: two independent temporaries with
            // identical spelling (`&foo()` on one line, `&foo()` on the next) are NOT the same
            // value, so the target node id is keyed by this operation's own record instead,
            // deliberately never converging.
            SemanticObservation::Ownership(header) => {
                let op_node_id = stable_id(
                    "node",
                    &format!("ownership-op:{}", header.record_id.as_str()),
                );
                ensure_node(
                    graph,
                    op_node_id.clone(),
                    "OwnershipOperation".into(),
                    format!("{}:{}", header.subject.name, header.subject.kind.as_str()),
                    BTreeMap::from([
                        ("origin".into(), "semantic-extraction".into()),
                        ("status".into(), header.status.as_str().into()),
                        ("kind".into(), header.subject.kind.as_str().into()),
                        (
                            "resolution".into(),
                            header.subject.resolution.as_str().into(),
                        ),
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
                        &format!("{caller_node_id}:HAS_OPERATION:{op_node_id}"),
                    ),
                    kind: "HAS_OPERATION".into(),
                    from: caller_node_id,
                    to: op_node_id.clone(),
                    attributes: BTreeMap::from([("origin".into(), "semantic-extraction".into())]),
                    provenance: header.provenance.clone(),
                    revision: header.provenance.source_revision.clone(),
                });
                let target_node_id = match header.subject.resolution {
                    crate::semantic::OwnershipResolution::Resolved => stable_id(
                        "node",
                        &format!(
                            "ownership-target:{}:{}",
                            header.subject.function.as_str(),
                            header.subject.name
                        ),
                    ),
                    crate::semantic::OwnershipResolution::Unresolved => stable_id(
                        "node",
                        &format!("ownership-target-unresolved:{}", header.record_id.as_str()),
                    ),
                };
                ensure_node(
                    graph,
                    target_node_id.clone(),
                    "OwnershipTarget".into(),
                    header.subject.name.clone(),
                    BTreeMap::from([
                        ("origin".into(), "semantic-extraction".into()),
                        (
                            "resolution".into(),
                            header.subject.resolution.as_str().into(),
                        ),
                    ]),
                    &header.provenance,
                );
                graph.edges.push(Edge {
                    id: stable_id("edge", &format!("{op_node_id}:TARGETS:{target_node_id}")),
                    kind: "TARGETS".into(),
                    from: op_node_id,
                    to: target_node_id,
                    attributes: BTreeMap::from([("origin".into(), "semantic-extraction".into())]),
                    provenance: header.provenance.clone(),
                    revision: header.provenance.source_revision.clone(),
                });
            }
            // R4.10: one ConcurrencyOperation node per observation and a PRODUCES_CONCURRENCY_OP
            // edge from the owning function -- mirroring R4.8's simpler Effect shape (no secondary
            // content-derived target node: unlike Ownership's `name`, a concurrency site has no
            // natural second entity to converge on this wave).
            SemanticObservation::Concurrency(header) => {
                let op_node_id = stable_id(
                    "node",
                    &format!("concurrency-op:{}", header.record_id.as_str()),
                );
                ensure_node(
                    graph,
                    op_node_id.clone(),
                    "ConcurrencyOperation".into(),
                    format!(
                        "{}@{}:{}:{}",
                        header.subject.kind.as_str(),
                        header.subject.span.path,
                        header.subject.span.line,
                        header.subject.span.column
                    ),
                    BTreeMap::from([
                        ("origin".into(), "semantic-extraction".into()),
                        ("status".into(), header.status.as_str().into()),
                        ("kind".into(), header.subject.kind.as_str().into()),
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
                        &format!("{caller_node_id}:PRODUCES_CONCURRENCY_OP:{op_node_id}"),
                    ),
                    kind: "PRODUCES_CONCURRENCY_OP".into(),
                    from: caller_node_id,
                    to: op_node_id,
                    attributes: BTreeMap::from([("origin".into(), "semantic-extraction".into())]),
                    provenance: header.provenance.clone(),
                    revision: header.provenance.source_revision.clone(),
                });
            }
            // R4.11: one PersistenceOperation node per observation and a PRODUCES_PERSISTENCE_OP
            // edge from the owning function, mirroring R4.10 CONCURRENCY's shape. Unlike
            // OWNERSHIP/STATE, this projection never invents its own content-derived target node
            // for the operation's place: when `place` is `PlaceRef::Resolved`, it draws a
            // REFERS_TO_PLACE edge directly to the EXISTING node that dimension's own observation
            // already produced (reconstructing that dimension's own node-id prefix scheme, never a
            // new one) -- proving cross-dimension identity convergence without inventing a fifth
            // place model. When `place` is `PlaceRef::Unresolved` (this extractor's only mode this
            // wave -- see `core::semantic::persistence`), no place-target node or edge is created
            // at all, which is deliberately safer than OWNERSHIP's old bug: there is no
            // spelling-keyed node here to accidentally collapse two unrelated temporaries onto.
            SemanticObservation::Persistence(header) => {
                let op_node_id = stable_id(
                    "node",
                    &format!("persistence-op:{}", header.record_id.as_str()),
                );
                ensure_node(
                    graph,
                    op_node_id.clone(),
                    "PersistenceOperation".into(),
                    format!(
                        "{}@{}:{}:{}",
                        header.subject.kind.as_str(),
                        header.subject.span.path,
                        header.subject.span.line,
                        header.subject.span.column
                    ),
                    BTreeMap::from([
                        ("origin".into(), "semantic-extraction".into()),
                        ("status".into(), header.status.as_str().into()),
                        ("kind".into(), header.subject.kind.as_str().into()),
                        (
                            "resolution".into(),
                            header.subject.resolution.as_str().into(),
                        ),
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
                        &format!("{caller_node_id}:PRODUCES_PERSISTENCE_OP:{op_node_id}"),
                    ),
                    kind: "PRODUCES_PERSISTENCE_OP".into(),
                    from: caller_node_id,
                    to: op_node_id.clone(),
                    attributes: BTreeMap::from([("origin".into(), "semantic-extraction".into())]),
                    provenance: header.provenance.clone(),
                    revision: header.provenance.source_revision.clone(),
                });
                if let crate::semantic::PlaceRef::Resolved {
                    dimension,
                    record_id,
                } = &header.subject.place
                {
                    let place_node_id = place_node_id_for(*dimension, record_id);
                    graph.edges.push(Edge {
                        id: stable_id(
                            "edge",
                            &format!("{op_node_id}:REFERS_TO_PLACE:{place_node_id}"),
                        ),
                        kind: "REFERS_TO_PLACE".into(),
                        from: op_node_id,
                        to: place_node_id,
                        attributes: BTreeMap::from([(
                            "origin".into(),
                            "semantic-extraction".into(),
                        )]),
                        provenance: header.provenance.clone(),
                        revision: header.provenance.source_revision.clone(),
                    });
                }
            }
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

/// Same as `summarize_system_graph`, but also projects `dependency_closure` into the graph before
/// summarizing (`add_dependency_closure`) -- so `nodes_total`/`edges_total` account for resolved
/// dependency edges too, not only source/declared/normalized-semantic facts.
pub fn summarize_system_graph_with_dependencies(
    source: &SourceReport,
    docs: &DocsReport,
    normalization: &NormalizationReport,
    dependency_closure: &DependencyClosureReport,
) -> GraphSummary {
    let mut graph = build_system_graph(source, docs, normalization);
    add_dependency_closure(&mut graph, dependency_closure);
    summarize_engineering_graph(&graph, source, "NORMALIZED_SEMANTIC_GRAPH")
}

fn dependency_consumer_node_id(consumer: &str) -> String {
    stable_id("node", &format!("dependency-consumer:{consumer}"))
}

fn dependency_provider_node_id(provider: &DependencyIdentity) -> String {
    stable_id(
        "node",
        &format!("dependency-provider:{}", provider.identity_key()),
    )
}

/// Projects a resolved `DependencyClosureReport` (`.atlas/contracts/DEPENDENCY-CENSUS.md`) into
/// `graph`: one `Package` node per consumer, one `Dependency` node per distinct resolved provider
/// identity, and a `RESOLVES_DEPENDENCY` edge between them carrying the edge's real, independently-
/// evidenced `role`/`activation` facts as attributes. Additive and idempotent (`ensure_node`
/// dedups by id) -- safe to call on an already-built graph, and a no-op for a closure with no
/// edges (e.g. `NotApplicable`/`Blocked` states, or a genuinely dependency-free workspace).
///
/// `.atlas/contracts/DEPENDENCY-CENSUS.md`: "The dependency graph is part of canonical census
/// truth, not an optional SBOM side report... The same typed dependency records feed query,
/// graph, security, license/provenance, build reasoning, invention, compiler optimization and
/// extinction analysis." Before this function existed, `DependencyClosureReport` was computed and
/// attached only as a sibling top-level `SystemizeReport` field -- never reaching `EngineeringGraph`
/// at all, so that claim did not hold for the actual graph artifact.
pub fn add_dependency_closure(graph: &mut EngineeringGraph, closure: &DependencyClosureReport) {
    for edge in &closure.edges {
        let prov = provenance(&edge.evidence_path, "atlas.dependency-closure.v1");

        let consumer_id = dependency_consumer_node_id(&edge.consumer);
        ensure_node(
            graph,
            consumer_id.clone(),
            "Package".into(),
            edge.consumer.clone(),
            BTreeMap::new(),
            &prov,
        );

        let provider_id = dependency_provider_node_id(&edge.provider);
        ensure_node(
            graph,
            provider_id.clone(),
            "Dependency".into(),
            format!("{}@{}", edge.provider.name, edge.provider.version),
            BTreeMap::from([
                ("ecosystem".into(), edge.provider.ecosystem.as_str().into()),
                ("name".into(), edge.provider.name.clone()),
                ("version".into(), edge.provider.version.clone()),
                (
                    "source_kind".into(),
                    edge.provider.source_kind.as_str().into(),
                ),
            ]),
            &prov,
        );

        let mut edge_attributes = BTreeMap::from([
            ("optional".into(), edge.activation.optional.to_string()),
            (
                "target_conditional".into(),
                edge.activation.target_conditional.to_string(),
            ),
        ]);
        if let Some(role) = edge.role {
            edge_attributes.insert("role".into(), role.as_str().into());
        }
        graph.edges.push(Edge {
            id: stable_id("edge", &format!("dependency:{}", edge.identity_key())),
            kind: "RESOLVES_DEPENDENCY".into(),
            from: consumer_id,
            to: provider_id,
            attributes: edge_attributes,
            provenance: prov.clone(),
            revision: None,
        });
    }
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
            exact_duplicates_merged: 0,
            conflict_candidates: Vec::new(),
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
            CallDispatchKind, CallSiteIdentity, ExtractorIdentity, PlaceRef, SemanticDimension,
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
            arguments: Vec::new(),
            result: PlaceRef::Unresolved,
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

    /// R4.6: one ControlFlow block observation, letting a test construct exactly the shape it
    /// needs (entry/not, kind, successors) while sharing repository/revision/`function` with a
    /// `function_identity_observation`/other blocks in the same test batch.
    fn control_flow_block_observation(
        function: &crate::semantic::SemanticRecordId,
        block_index: usize,
        kind: crate::semantic::ControlFlowBlockKind,
        is_entry: bool,
        successors: Vec<crate::semantic::ControlFlowEdge>,
    ) -> SemanticObservation {
        use crate::identity::RepositoryId;
        use crate::provenance::provenance;
        use crate::semantic::{
            ControlFlowBlockIdentity, ExtractorIdentity, SemanticDimension, SemanticRecordHeader,
            SemanticRecordId, SemanticScope,
        };
        use crate::temporal::RevisionRef;

        let repository = RepositoryId::new("atlas-studio");
        let revision = RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let subject = ControlFlowBlockIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            function: function.clone(),
            block_index,
            kind,
            is_entry,
            successors,
        };
        let record_id =
            SemanticRecordId::new(SemanticDimension::ControlFlow, &subject.identity_key());
        SemanticObservation::ControlFlow(SemanticRecordHeader {
            record_id,
            dimension: SemanticDimension::ControlFlow,
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

    /// R4.7: a DataFlow value observation whose `function` (owner) is the record_id of
    /// `function_identity_observation(owner_name)`'s `FunctionIdentity`, so the two can be
    /// combined in one batch to exercise the HAS_VALUE/RESOLVES_TO edges.
    #[allow(clippy::too_many_arguments)]
    fn data_flow_value_observation(
        function: &crate::semantic::SemanticRecordId,
        name: &str,
        line: usize,
        role: crate::semantic::ValueRole,
        is_parameter: bool,
        is_return_flow: bool,
        resolved_definition: Option<crate::semantic::SemanticRecordId>,
    ) -> SemanticObservation {
        use crate::identity::RepositoryId;
        use crate::provenance::provenance;
        use crate::semantic::{
            DataFlowResolution, ExtractorIdentity, SemanticDimension, SemanticRecordHeader,
            SemanticRecordId, SemanticScope, ValueIdentity,
        };
        use crate::temporal::RevisionRef;

        let repository = RepositoryId::new("atlas-studio");
        let revision = RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let resolution = if resolved_definition.is_some() {
            DataFlowResolution::Resolved
        } else {
            DataFlowResolution::Unresolved
        };
        let subject = ValueIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            function: function.clone(),
            name: name.to_owned(),
            span: crate::language::adl::SourceSpan {
                path: "src/lib.rs".into(),
                line,
                column: 5,
            },
            role,
            is_parameter,
            is_return_flow,
            resolution,
            resolved_definition,
        };
        let record_id = SemanticRecordId::new(SemanticDimension::DataFlow, &subject.identity_key());
        SemanticObservation::DataFlow(SemanticRecordHeader {
            record_id,
            dimension: SemanticDimension::DataFlow,
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

    /// R4.8: a State access observation whose `function` (owner) is a fixed synthetic
    /// `FunctionIdentity` record_id -- the graph tests only need the STATE claim itself to be
    /// independently attributable, not a paired FunctionIdentity observation.
    fn state_access_observation(
        function: &crate::semantic::SemanticRecordId,
        name: &str,
        line: usize,
        kind: crate::semantic::StateAccessKind,
    ) -> SemanticObservation {
        use crate::identity::RepositoryId;
        use crate::provenance::provenance;
        use crate::semantic::{
            ExtractorIdentity, SemanticDimension, SemanticRecordHeader, SemanticRecordId,
            SemanticScope, StateAccessIdentity, StateResolution,
        };
        use crate::temporal::RevisionRef;

        let repository = RepositoryId::new("atlas-studio");
        let revision = RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let scope = SemanticScope::new(["impl:Owner"]);
        let subject = StateAccessIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            function: function.clone(),
            scope: scope.clone(),
            name: name.to_owned(),
            span: crate::language::adl::SourceSpan {
                path: "src/lib.rs".into(),
                line,
                column: 5,
            },
            kind,
            resolution: StateResolution::Resolved,
        };
        let record_id = SemanticRecordId::new(SemanticDimension::State, &subject.identity_key());
        SemanticObservation::State(SemanticRecordHeader {
            record_id,
            dimension: SemanticDimension::State,
            status: EpistemicStatus::Observed,
            subject,
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

    /// R4.8: a Panic Effect observation whose `function` (owner) is a fixed synthetic
    /// `FunctionIdentity` record_id.
    fn effect_observation(
        function: &crate::semantic::SemanticRecordId,
        line: usize,
    ) -> SemanticObservation {
        use crate::identity::RepositoryId;
        use crate::provenance::provenance;
        use crate::semantic::{
            EffectCategory, EffectIdentity, ExtractorIdentity, SemanticDimension,
            SemanticRecordHeader, SemanticRecordId, SemanticScope,
        };
        use crate::temporal::RevisionRef;

        let repository = RepositoryId::new("atlas-studio");
        let revision = RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let subject = EffectIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            function: function.clone(),
            category: EffectCategory::Panic,
            span: crate::language::adl::SourceSpan {
                path: "src/lib.rs".into(),
                line,
                column: 5,
            },
        };
        let record_id = SemanticRecordId::new(SemanticDimension::Effect, &subject.identity_key());
        SemanticObservation::Effect(SemanticRecordHeader {
            record_id,
            dimension: SemanticDimension::Effect,
            status: EpistemicStatus::Observed,
            subject,
            scope: SemanticScope::new(["impl:Owner"]),
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

    /// R4.9: an Ownership operation observation whose `function` (owner) is a fixed synthetic
    /// `FunctionIdentity` record_id.
    fn ownership_observation(
        function: &crate::semantic::SemanticRecordId,
        name: &str,
        line: usize,
        kind: crate::semantic::OwnershipKind,
    ) -> SemanticObservation {
        use crate::identity::RepositoryId;
        use crate::provenance::provenance;
        use crate::semantic::{
            ExtractorIdentity, OwnershipIdentity, OwnershipResolution, SemanticDimension,
            SemanticRecordHeader, SemanticRecordId, SemanticScope,
        };
        use crate::temporal::RevisionRef;

        let repository = RepositoryId::new("atlas-studio");
        let revision = RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let subject = OwnershipIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            function: function.clone(),
            name: name.to_owned(),
            span: crate::language::adl::SourceSpan {
                path: "src/lib.rs".into(),
                line,
                column: 5,
            },
            kind,
            resolution: OwnershipResolution::Resolved,
        };
        let record_id =
            SemanticRecordId::new(SemanticDimension::Ownership, &subject.identity_key());
        SemanticObservation::Ownership(SemanticRecordHeader {
            record_id,
            dimension: SemanticDimension::Ownership,
            status: EpistemicStatus::Observed,
            subject,
            scope: SemanticScope::new(["impl:Owner"]),
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

    /// Like `ownership_observation`, but with an explicit `resolution` -- for proving that an
    /// `Unresolved` (unresolved-temporary) operation never converges with another by spelling
    /// alone, unlike a `Resolved` (real place) one.
    fn ownership_observation_with_resolution(
        function: &crate::semantic::SemanticRecordId,
        name: &str,
        line: usize,
        kind: crate::semantic::OwnershipKind,
        resolution: crate::semantic::OwnershipResolution,
    ) -> SemanticObservation {
        use crate::identity::RepositoryId;
        use crate::provenance::provenance;
        use crate::semantic::{
            ExtractorIdentity, OwnershipIdentity, SemanticDimension, SemanticRecordHeader,
            SemanticRecordId, SemanticScope,
        };
        use crate::temporal::RevisionRef;

        let repository = RepositoryId::new("atlas-studio");
        let revision = RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let subject = OwnershipIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            function: function.clone(),
            name: name.to_owned(),
            span: crate::language::adl::SourceSpan {
                path: "src/lib.rs".into(),
                line,
                column: 5,
            },
            kind,
            resolution,
        };
        let record_id =
            SemanticRecordId::new(SemanticDimension::Ownership, &subject.identity_key());
        SemanticObservation::Ownership(SemanticRecordHeader {
            record_id,
            dimension: SemanticDimension::Ownership,
            status: EpistemicStatus::Observed,
            subject,
            scope: SemanticScope::new(["impl:Owner"]),
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

    /// R4.11: a Persistence candidate observation whose `function` (owner) is a fixed synthetic
    /// `FunctionIdentity` record_id. `place` lets a test prove `PlaceRef::Resolved` convergence
    /// against another dimension's own observation (see `place_node_id_for`), or leave it
    /// `Unresolved` to match this extractor's actual bootstrap behavior.
    fn persistence_observation(
        function: &crate::semantic::SemanticRecordId,
        kind: crate::semantic::PersistenceKind,
        line: usize,
        place: crate::semantic::PlaceRef,
    ) -> SemanticObservation {
        use crate::identity::RepositoryId;
        use crate::provenance::provenance;
        use crate::semantic::{
            ExtractorIdentity, PersistenceIdentity, PersistenceResolution, SemanticDimension,
            SemanticRecordHeader, SemanticRecordId, SemanticScope,
        };
        use crate::temporal::RevisionRef;

        let repository = RepositoryId::new("atlas-studio");
        let revision = RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let subject = PersistenceIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            function: function.clone(),
            kind,
            span: crate::language::adl::SourceSpan {
                path: "src/lib.rs".into(),
                line,
                column: 5,
            },
            place,
            resolution: PersistenceResolution::Unresolved,
        };
        let record_id =
            SemanticRecordId::new(SemanticDimension::Persistence, &subject.identity_key());
        SemanticObservation::Persistence(SemanticRecordHeader {
            record_id,
            dimension: SemanticDimension::Persistence,
            status: EpistemicStatus::Inferred,
            subject,
            scope: SemanticScope::new(["impl:Owner"]),
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
            exact_duplicates_merged: 0,
            conflict_candidates: Vec::new(),
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

    // --- R4.6: CONTROL_FLOW observations produce ControlFlowBlock nodes and structural edges ------

    #[test]
    fn control_flow_block_produces_a_node_and_a_has_block_edge_from_its_function() {
        use crate::semantic::{ControlFlowBlockKind, ControlFlowEdge, ControlFlowEdgeKind};

        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();
        let block = control_flow_block_observation(
            &caller_record_id,
            0,
            ControlFlowBlockKind::FunctionEntry,
            true,
            vec![ControlFlowEdge {
                kind: ControlFlowEdgeKind::Return,
                target: None,
            }],
        );

        let normalization = normalization_with(vec![caller, block], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        let block_node = graph
            .nodes
            .iter()
            .find(|node| node.kind == "ControlFlowBlock")
            .expect("a ControlFlowBlock node must exist");
        assert_eq!(
            block_node.attributes.get("kind").map(String::as_str),
            Some("FUNCTION_ENTRY")
        );
        assert_eq!(
            block_node.attributes.get("is_entry").map(String::as_str),
            Some("true")
        );

        let caller_node_id = stable_id(
            "node",
            &format!("function-identity:{}", caller_record_id.as_str()),
        );
        let has_block_edge = graph
            .edges
            .iter()
            .find(|edge| edge.kind == "HAS_BLOCK")
            .expect("a HAS_BLOCK edge must exist");
        assert_eq!(has_block_edge.from, caller_node_id);
        assert_eq!(has_block_edge.to, block_node.id);
    }

    #[test]
    fn a_targetless_successor_produces_no_graph_edge() {
        use crate::semantic::{ControlFlowBlockKind, ControlFlowEdge, ControlFlowEdgeKind};

        // Return/Panic/Unresolved all carry `target: None` -- they leave the function/this
        // block's reach entirely and must never produce a dangling or synthesized graph edge.
        let caller = function_identity_observation("Foo");
        let block = control_flow_block_observation(
            &caller.record_id().clone(),
            0,
            ControlFlowBlockKind::FunctionEntry,
            true,
            vec![
                ControlFlowEdge {
                    kind: ControlFlowEdgeKind::Panic,
                    target: None,
                },
                ControlFlowEdge {
                    kind: ControlFlowEdgeKind::Unresolved,
                    target: None,
                },
            ],
        );

        let normalization = normalization_with(vec![caller, block], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        assert!(
            graph
                .edges
                .iter()
                .all(|edge| edge.kind != "PANIC" && edge.kind != "UNRESOLVED")
        );
    }

    #[test]
    fn a_branch_successor_with_a_target_produces_a_real_edge_between_two_block_nodes() {
        use crate::semantic::{ControlFlowBlockKind, ControlFlowEdge, ControlFlowEdgeKind};

        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();

        // Block 1 first, so its record_id is known when constructing block 0's Branch edge.
        let then_block = control_flow_block_observation(
            &caller_record_id,
            1,
            ControlFlowBlockKind::IfThen,
            false,
            vec![ControlFlowEdge {
                kind: ControlFlowEdgeKind::Return,
                target: None,
            }],
        );
        let then_record_id = then_block.record_id().clone();

        let entry_block = control_flow_block_observation(
            &caller_record_id,
            0,
            ControlFlowBlockKind::FunctionEntry,
            true,
            vec![ControlFlowEdge {
                kind: ControlFlowEdgeKind::Branch,
                target: Some(then_record_id.clone()),
            }],
        );

        let normalization = normalization_with(vec![caller, entry_block, then_block], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        let then_node_id = stable_id(
            "node",
            &format!("control-flow-block:{}", then_record_id.as_str()),
        );
        let branch_edge = graph
            .edges
            .iter()
            .find(|edge| edge.kind == "BRANCH")
            .expect("a BRANCH edge must exist");
        assert_eq!(branch_edge.to, then_node_id);
        // Two distinct blocks, two distinct ControlFlowBlock nodes, never collapsed.
        let block_nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|node| node.kind == "ControlFlowBlock")
            .collect();
        assert_eq!(block_nodes.len(), 2);
    }

    // --- R4.7: DATA_FLOW observations produce Value nodes and structural edges ------------------

    #[test]
    fn data_flow_value_produces_a_node_and_a_has_value_edge_from_its_function() {
        use crate::semantic::ValueRole;

        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();
        let value = data_flow_value_observation(
            &caller_record_id,
            "x",
            10,
            ValueRole::Definition,
            true,
            false,
            None,
        );

        let normalization = normalization_with(vec![caller, value], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        let value_node = graph
            .nodes
            .iter()
            .find(|node| node.kind == "Value")
            .expect("a Value node must exist");
        assert_eq!(
            value_node.attributes.get("role").map(String::as_str),
            Some("DEFINITION")
        );
        assert_eq!(
            value_node.attributes.get("resolution").map(String::as_str),
            Some("UNRESOLVED")
        );
        assert_eq!(
            value_node
                .attributes
                .get("is_parameter")
                .map(String::as_str),
            Some("true")
        );
        assert_eq!(
            value_node
                .attributes
                .get("is_return_flow")
                .map(String::as_str),
            Some("false")
        );

        let caller_node_id = stable_id(
            "node",
            &format!("function-identity:{}", caller_record_id.as_str()),
        );
        let has_value_edge = graph
            .edges
            .iter()
            .find(|edge| edge.kind == "HAS_VALUE")
            .expect("a HAS_VALUE edge must exist");
        assert_eq!(has_value_edge.from, caller_node_id);
        assert_eq!(has_value_edge.to, value_node.id);
    }

    #[test]
    fn an_unresolved_use_produces_no_resolves_to_edge() {
        use crate::semantic::ValueRole;

        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();
        let unresolved_use = data_flow_value_observation(
            &caller_record_id,
            "y",
            20,
            ValueRole::Use,
            false,
            false,
            None,
        );

        let normalization = normalization_with(vec![caller, unresolved_use], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        assert!(graph.edges.iter().all(|edge| edge.kind != "RESOLVES_TO"));
    }

    #[test]
    fn a_resolved_use_produces_a_real_resolves_to_edge_between_two_value_nodes() {
        use crate::semantic::ValueRole;

        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();

        // Definition first, so its record_id is known when constructing the Use's resolution.
        let definition = data_flow_value_observation(
            &caller_record_id,
            "x",
            10,
            ValueRole::Definition,
            true,
            false,
            None,
        );
        let definition_record_id = definition.record_id().clone();

        let resolved_use = data_flow_value_observation(
            &caller_record_id,
            "x",
            11,
            ValueRole::Use,
            false,
            false,
            Some(definition_record_id.clone()),
        );

        let normalization = normalization_with(vec![caller, definition, resolved_use], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        let definition_node_id = stable_id(
            "node",
            &format!("data-flow-value:{}", definition_record_id.as_str()),
        );
        let resolves_to_edge = graph
            .edges
            .iter()
            .find(|edge| edge.kind == "RESOLVES_TO")
            .expect("a RESOLVES_TO edge must exist");
        assert_eq!(resolves_to_edge.to, definition_node_id);
        // Two distinct values, two distinct Value nodes, never collapsed.
        let value_nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|node| node.kind == "Value")
            .collect();
        assert_eq!(value_nodes.len(), 2);
    }

    // --- R4.8: STATE observations produce StateAccess/StateEntity nodes and structural edges ------

    #[test]
    fn state_access_produces_a_node_and_a_has_access_edge_from_its_function() {
        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();
        let access = state_access_observation(
            &caller_record_id,
            "value",
            10,
            crate::semantic::StateAccessKind::Read,
        );

        let normalization = normalization_with(vec![caller, access], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        let access_node = graph
            .nodes
            .iter()
            .find(|node| node.kind == "StateAccess")
            .expect("a StateAccess node must exist");
        assert_eq!(
            access_node.attributes.get("kind").map(String::as_str),
            Some("READ")
        );
        assert_eq!(
            access_node.attributes.get("resolution").map(String::as_str),
            Some("RESOLVED")
        );

        let caller_node_id = stable_id(
            "node",
            &format!("function-identity:{}", caller_record_id.as_str()),
        );
        let has_access_edge = graph
            .edges
            .iter()
            .find(|edge| edge.kind == "HAS_ACCESS")
            .expect("a HAS_ACCESS edge must exist");
        assert_eq!(has_access_edge.from, caller_node_id);
        assert_eq!(has_access_edge.to, access_node.id);
    }

    #[test]
    fn two_accesses_of_the_same_field_converge_on_one_state_entity_node() {
        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();
        let read = state_access_observation(
            &caller_record_id,
            "value",
            10,
            crate::semantic::StateAccessKind::Read,
        );
        let write = state_access_observation(
            &caller_record_id,
            "value",
            11,
            crate::semantic::StateAccessKind::Write,
        );

        let normalization = normalization_with(vec![caller, read, write], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        let entity_nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|node| node.kind == "StateEntity")
            .collect();
        assert_eq!(
            entity_nodes.len(),
            1,
            "same scope+name must converge on exactly one StateEntity node"
        );

        let targets_edges: Vec<_> = graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "TARGETS")
            .collect();
        assert_eq!(targets_edges.len(), 2, "one TARGETS edge per access");
        assert!(
            targets_edges
                .iter()
                .all(|edge| edge.to == entity_nodes[0].id)
        );

        let access_nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|node| node.kind == "StateAccess")
            .collect();
        assert_eq!(
            access_nodes.len(),
            2,
            "two distinct access events, never collapsed"
        );
    }

    // --- R4.8: EFFECT observations produce Effect nodes and a PRODUCES_EFFECT edge -----------------

    #[test]
    fn effect_produces_a_node_and_a_produces_effect_edge_from_its_function() {
        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();
        let effect = effect_observation(&caller_record_id, 20);

        let normalization = normalization_with(vec![caller, effect], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        let effect_node = graph
            .nodes
            .iter()
            .find(|node| node.kind == "Effect")
            .expect("an Effect node must exist");
        assert_eq!(
            effect_node.attributes.get("category").map(String::as_str),
            Some("PANIC")
        );

        let caller_node_id = stable_id(
            "node",
            &format!("function-identity:{}", caller_record_id.as_str()),
        );
        let produces_effect_edge = graph
            .edges
            .iter()
            .find(|edge| edge.kind == "PRODUCES_EFFECT")
            .expect("a PRODUCES_EFFECT edge must exist");
        assert_eq!(produces_effect_edge.from, caller_node_id);
        assert_eq!(produces_effect_edge.to, effect_node.id);
    }

    #[test]
    fn two_effects_from_the_same_function_produce_two_distinct_effect_nodes() {
        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();
        let first = effect_observation(&caller_record_id, 20);
        let second = effect_observation(&caller_record_id, 30);

        let normalization = normalization_with(vec![caller, first, second], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        let effect_nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|node| node.kind == "Effect")
            .collect();
        assert_eq!(effect_nodes.len(), 2);
    }

    // --- R4.9: OWNERSHIP observations produce OwnershipOperation/OwnershipTarget nodes and
    // structural edges --------------------------------------------------------------------------

    #[test]
    fn ownership_op_produces_a_node_and_a_has_operation_edge_from_its_function() {
        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();
        let op = ownership_observation(
            &caller_record_id,
            "w",
            10,
            crate::semantic::OwnershipKind::BorrowShared,
        );

        let normalization = normalization_with(vec![caller, op], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        let op_node = graph
            .nodes
            .iter()
            .find(|node| node.kind == "OwnershipOperation")
            .expect("an OwnershipOperation node must exist");
        assert_eq!(
            op_node.attributes.get("kind").map(String::as_str),
            Some("BORROW_SHARED")
        );

        let caller_node_id = stable_id(
            "node",
            &format!("function-identity:{}", caller_record_id.as_str()),
        );
        let has_operation_edge = graph
            .edges
            .iter()
            .find(|edge| edge.kind == "HAS_OPERATION")
            .expect("a HAS_OPERATION edge must exist");
        assert_eq!(has_operation_edge.from, caller_node_id);
        assert_eq!(has_operation_edge.to, op_node.id);
    }

    #[test]
    fn a_borrow_and_a_move_of_the_same_name_converge_on_one_ownership_target_node() {
        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();
        let borrow = ownership_observation(
            &caller_record_id,
            "w",
            10,
            crate::semantic::OwnershipKind::BorrowShared,
        );
        let mv = ownership_observation(
            &caller_record_id,
            "w",
            11,
            crate::semantic::OwnershipKind::MoveOrCopy,
        );

        let normalization = normalization_with(vec![caller, borrow, mv], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        let target_nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|node| node.kind == "OwnershipTarget")
            .collect();
        assert_eq!(
            target_nodes.len(),
            1,
            "same function+name must converge on exactly one OwnershipTarget node"
        );

        let targets_edges: Vec<_> = graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "TARGETS")
            .collect();
        assert_eq!(targets_edges.len(), 2, "one TARGETS edge per operation");
        assert!(
            targets_edges
                .iter()
                .all(|edge| edge.to == target_nodes[0].id)
        );

        let op_nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|node| node.kind == "OwnershipOperation")
            .collect();
        assert_eq!(
            op_nodes.len(),
            2,
            "two distinct operations, never collapsed"
        );
    }

    #[test]
    fn two_ownership_ops_on_different_names_produce_two_distinct_target_nodes() {
        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();
        let a = ownership_observation(
            &caller_record_id,
            "a",
            10,
            crate::semantic::OwnershipKind::MoveOrCopy,
        );
        let b = ownership_observation(
            &caller_record_id,
            "b",
            11,
            crate::semantic::OwnershipKind::MoveOrCopy,
        );

        let normalization = normalization_with(vec![caller, a, b], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        let target_nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|node| node.kind == "OwnershipTarget")
            .collect();
        assert_eq!(target_nodes.len(), 2);
    }

    #[test]
    fn two_unresolved_temporaries_with_identical_spelling_never_collapse_onto_one_ownership_target()
    {
        // `let a = &foo(); let b = &foo();` -- two independent call-result temporaries that happen
        // to read alike must never converge onto one OwnershipTarget merely because their spelling
        // matches, unlike a real place name (see the `Resolved` convergence test above).
        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();
        let first = ownership_observation_with_resolution(
            &caller_record_id,
            "foo ()",
            10,
            crate::semantic::OwnershipKind::BorrowShared,
            crate::semantic::OwnershipResolution::Unresolved,
        );
        let second = ownership_observation_with_resolution(
            &caller_record_id,
            "foo ()",
            11,
            crate::semantic::OwnershipKind::BorrowShared,
            crate::semantic::OwnershipResolution::Unresolved,
        );

        let normalization = normalization_with(vec![caller, first, second], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        let target_nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|node| node.kind == "OwnershipTarget")
            .collect();
        assert_eq!(
            target_nodes.len(),
            2,
            "identically-spelled unresolved temporaries must never collapse onto one target"
        );

        let op_nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|node| node.kind == "OwnershipOperation")
            .collect();
        assert_eq!(op_nodes.len(), 2);

        let targets_edges: Vec<_> = graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "TARGETS")
            .collect();
        assert_eq!(targets_edges.len(), 2);
        let target_ids: std::collections::BTreeSet<_> =
            targets_edges.iter().map(|edge| edge.to.clone()).collect();
        assert_eq!(
            target_ids.len(),
            2,
            "each unresolved operation must TARGET its own distinct node"
        );
    }

    #[test]
    fn a_resolved_and_an_unresolved_ownership_operation_with_the_same_spelling_do_not_converge() {
        // A real place name (`x`) and an unresolved temporary that happens to share the same
        // textual spelling (contrived, but the classification must not depend on getting lucky
        // with distinct spellings) must still never converge -- `resolution` alone decides.
        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();
        let resolved = ownership_observation_with_resolution(
            &caller_record_id,
            "x",
            10,
            crate::semantic::OwnershipKind::BorrowShared,
            crate::semantic::OwnershipResolution::Resolved,
        );
        let unresolved = ownership_observation_with_resolution(
            &caller_record_id,
            "x",
            11,
            crate::semantic::OwnershipKind::BorrowShared,
            crate::semantic::OwnershipResolution::Unresolved,
        );

        let normalization = normalization_with(vec![caller, resolved, unresolved], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        let target_nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|node| node.kind == "OwnershipTarget")
            .collect();
        assert_eq!(target_nodes.len(), 2);
    }

    // --- R4.11 PERSISTENCE graph projection ------------------------------------------------------

    #[test]
    fn persistence_observation_produces_a_node_and_a_produces_op_edge_from_its_function() {
        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();
        let candidate = persistence_observation(
            &caller_record_id,
            crate::semantic::PersistenceKind::Commit,
            10,
            crate::semantic::PlaceRef::Unresolved,
        );

        let normalization = normalization_with(vec![caller, candidate], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        let op_node = graph
            .nodes
            .iter()
            .find(|node| node.kind == "PersistenceOperation")
            .expect("a PersistenceOperation node must exist");
        assert_eq!(
            op_node.attributes.get("status").map(String::as_str),
            Some("INFERRED")
        );
        assert_eq!(
            op_node.attributes.get("kind").map(String::as_str),
            Some("COMMIT")
        );
        assert_eq!(
            op_node.attributes.get("resolution").map(String::as_str),
            Some("UNRESOLVED")
        );

        let caller_node_id = stable_id(
            "node",
            &format!("function-identity:{}", caller_record_id.as_str()),
        );
        let produces_edge = graph
            .edges
            .iter()
            .find(|edge| edge.kind == "PRODUCES_PERSISTENCE_OP")
            .expect("a PRODUCES_PERSISTENCE_OP edge must exist");
        assert_eq!(produces_edge.from, caller_node_id);
        assert_eq!(produces_edge.to, op_node.id);
    }

    #[test]
    fn unresolved_persistence_place_produces_no_refers_to_place_edge() {
        // The safest possible choice for the bootstrap's only real mode: no target node/edge at
        // all, rather than a new one keyed by anything spelling-derived (the R4.9 lesson).
        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();
        let candidate = persistence_observation(
            &caller_record_id,
            crate::semantic::PersistenceKind::Flush,
            10,
            crate::semantic::PlaceRef::Unresolved,
        );

        let normalization = normalization_with(vec![caller, candidate], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        assert!(
            !graph
                .edges
                .iter()
                .any(|edge| edge.kind == "REFERS_TO_PLACE"),
            "an Unresolved place must never produce a REFERS_TO_PLACE edge"
        );
    }

    #[test]
    fn resolved_persistence_place_converges_on_the_existing_state_access_node() {
        // Proves cross-dimension identity convergence: a PERSISTENCE operation whose PlaceRef
        // resolves to a STATE access's own record_id draws an edge at that SAME graph node --
        // never a new, independently-invented persistence-only target.
        let caller = function_identity_observation("Foo");
        let caller_record_id = caller.record_id().clone();
        let state_access = state_access_observation(
            &caller_record_id,
            "balance",
            10,
            crate::semantic::StateAccessKind::Write,
        );
        let state_record_id = state_access.record_id().clone();
        let candidate = persistence_observation(
            &caller_record_id,
            crate::semantic::PersistenceKind::Commit,
            11,
            crate::semantic::PlaceRef::Resolved {
                dimension: crate::semantic::SemanticDimension::State,
                record_id: state_record_id.clone(),
            },
        );

        let normalization = normalization_with(vec![caller, state_access, candidate], Vec::new());
        let graph = build_system_graph(&source(), &docs(), &normalization);

        let expected_place_node_id =
            place_node_id_for(crate::semantic::SemanticDimension::State, &state_record_id);
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.id == expected_place_node_id && node.kind == "StateAccess"),
            "the resolved place node must be the SAME node STATE's own observation produced"
        );

        let refers_edge = graph
            .edges
            .iter()
            .find(|edge| edge.kind == "REFERS_TO_PLACE")
            .expect("a REFERS_TO_PLACE edge must exist for a Resolved place");
        assert_eq!(refers_edge.to, expected_place_node_id);
    }

    fn dependency_edge(
        consumer: &str,
        provider_name: &str,
        provider_version: &str,
        role: Option<crate::census::DependencyRole>,
        activation: crate::census::DependencyActivation,
    ) -> crate::census::DependencyEdge {
        crate::census::DependencyEdge {
            consumer: consumer.into(),
            provider: DependencyIdentity {
                ecosystem: crate::census::DependencyEcosystem::Cargo,
                name: provider_name.into(),
                version: provider_version.into(),
                source_kind: crate::census::DependencySourceKind::Registry,
                source_locator: Some("registry+https://example.invalid".into()),
                checksum: None,
            },
            role,
            activation,
            evidence_path: "Cargo.lock".into(),
        }
    }

    fn dependency_closure(edges: Vec<crate::census::DependencyEdge>) -> DependencyClosureReport {
        DependencyClosureReport {
            schema: "test".into(),
            ecosystem: crate::census::DependencyEcosystem::Cargo,
            root: "/repo".into(),
            state: crate::census::DependencyClosureState::Closed,
            edges_total: edges.len(),
            instances_total: edges.len(),
            edges,
            dangling_references: Vec::new(),
            unsupported_constructs: Vec::new(),
            dynamic_obligations: Vec::new(),
        }
    }

    #[test]
    fn dependency_closure_projects_consumer_and_provider_nodes_and_a_depends_on_edge() {
        let mut graph = build_source_graph(&source());
        let closure = dependency_closure(vec![dependency_edge(
            "core",
            "serde",
            "1.0.0",
            Some(crate::census::DependencyRole::Runtime),
            crate::census::DependencyActivation::ALWAYS,
        )]);
        add_dependency_closure(&mut graph, &closure);

        let consumer = graph
            .nodes
            .iter()
            .find(|node| node.kind == "Package" && node.identity == "core")
            .expect("a Package node must exist for the consumer");
        let provider = graph
            .nodes
            .iter()
            .find(|node| node.kind == "Dependency" && node.identity == "serde@1.0.0")
            .expect("a Dependency node must exist for the resolved provider");

        let edge = graph
            .edges
            .iter()
            .find(|edge| edge.kind == "RESOLVES_DEPENDENCY")
            .expect("a RESOLVES_DEPENDENCY edge must exist");
        assert_eq!(edge.from, consumer.id);
        assert_eq!(edge.to, provider.id);
        assert_eq!(edge.attributes.get("role"), Some(&"RUNTIME".to_string()));
        assert_eq!(edge.attributes.get("optional"), Some(&"false".to_string()));
    }

    #[test]
    fn dependency_closure_with_no_edges_is_a_no_op() {
        let mut graph = build_source_graph(&source());
        let nodes_before = graph.nodes.len();
        let edges_before = graph.edges.len();
        add_dependency_closure(&mut graph, &dependency_closure(Vec::new()));
        assert_eq!(graph.nodes.len(), nodes_before);
        assert_eq!(graph.edges.len(), edges_before);
    }

    #[test]
    fn two_edges_sharing_the_same_provider_do_not_duplicate_the_provider_node() {
        let mut graph = build_source_graph(&source());
        let closure = dependency_closure(vec![
            dependency_edge(
                "core",
                "serde",
                "1.0.0",
                Some(crate::census::DependencyRole::Runtime),
                crate::census::DependencyActivation::ALWAYS,
            ),
            dependency_edge(
                "adapter",
                "serde",
                "1.0.0",
                Some(crate::census::DependencyRole::Runtime),
                crate::census::DependencyActivation::ALWAYS,
            ),
        ]);
        add_dependency_closure(&mut graph, &closure);

        let provider_nodes = graph
            .nodes
            .iter()
            .filter(|node| node.kind == "Dependency" && node.identity == "serde@1.0.0")
            .count();
        assert_eq!(provider_nodes, 1, "one shared provider must be one node");
        let depends_on_edges = graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "RESOLVES_DEPENDENCY")
            .count();
        assert_eq!(depends_on_edges, 2, "each consumer still gets its own edge");
    }

    #[test]
    fn an_unevidenced_role_produces_no_role_attribute_but_still_records_the_edge() {
        let mut graph = build_source_graph(&source());
        let closure = dependency_closure(vec![dependency_edge(
            "serde",
            "syn",
            "2.0.0",
            None,
            crate::census::DependencyActivation::ALWAYS,
        )]);
        add_dependency_closure(&mut graph, &closure);

        let edge = graph
            .edges
            .iter()
            .find(|edge| edge.kind == "RESOLVES_DEPENDENCY")
            .expect("a RESOLVES_DEPENDENCY edge must exist even with no evidenced role");
        assert!(!edge.attributes.contains_key("role"));
    }

    #[test]
    fn summarize_system_graph_with_dependencies_counts_projected_dependency_nodes() {
        let normalization = normalization_with(Vec::new(), Vec::new());
        let closure = dependency_closure(vec![dependency_edge(
            "core",
            "serde",
            "1.0.0",
            Some(crate::census::DependencyRole::Runtime),
            crate::census::DependencyActivation::ALWAYS,
        )]);
        let without = summarize_system_graph(&source(), &docs(), &normalization);
        let with =
            summarize_system_graph_with_dependencies(&source(), &docs(), &normalization, &closure);
        assert_eq!(with.nodes_total, without.nodes_total + 2);
        assert_eq!(with.edges_total, without.edges_total + 1);
    }
}
