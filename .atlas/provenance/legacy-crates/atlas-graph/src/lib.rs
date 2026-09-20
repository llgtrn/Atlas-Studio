use atlas_model::{Edge, EngineeringGraph, Fact, GraphSummary, Node, Provenance, SourceReport};
use std::collections::BTreeMap;

fn stable_id(prefix: &str, identity: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in identity.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{prefix}:{hash:016x}")
}

fn provenance(path: impl Into<String>, hash: Option<String>) -> Provenance {
    Provenance {
        source_path: path.into(),
        source_revision: None,
        extractor: "atlas-graph.source-fact-bootstrap".into(),
        content_hash: hash,
        span: None,
    }
}

pub fn build_from_source(source: &SourceReport) -> EngineeringGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut facts = Vec::new();

    let repo_id = stable_id("node", &source.root);
    nodes.push(Node {
        id: repo_id.clone(),
        kind: "Repository".into(),
        identity: source.root.clone(),
        attributes: BTreeMap::new(),
        provenance: provenance(".atlas/repo.toml", None),
        revision: None,
    });

    let mut language_nodes: Vec<_> = source.languages.keys().cloned().collect();
    language_nodes.sort();

    let mut language_ids = BTreeMap::new();
    for language in &language_nodes {
        let id = stable_id("node", &format!("technology:{language}"));
        language_ids.insert(language.clone(), id.clone());
        let mut attributes = BTreeMap::new();
        attributes.insert("domain".into(), "language".into());
        nodes.push(Node {
            id,
            kind: "Technology".into(),
            identity: language.clone(),
            attributes,
            provenance: provenance(".atlas/repo.toml", None),
            revision: None,
        });
    }

    for file in &source.files {
        let file_id = stable_id("node", &format!("file:{}", file.path));
        let mut attributes = BTreeMap::new();
        attributes.insert("language".into(), file.language.clone());
        attributes.insert("bytes".into(), file.bytes.to_string());
        nodes.push(Node {
            id: file_id.clone(),
            kind: "File".into(),
            identity: file.path.clone(),
            attributes,
            provenance: provenance(&file.path, None),
            revision: None,
        });
        edges.push(Edge {
            id: stable_id("edge", &format!("{}:CONTAINS:{file_id}", repo_id)),
            kind: "CONTAINS".into(),
            from: repo_id.clone(),
            to: file_id.clone(),
            attributes: BTreeMap::new(),
            provenance: provenance(&file.path, None),
            revision: None,
        });
        if let Some(language_id) = language_ids.get(&file.language) {
            edges.push(Edge {
                id: stable_id("edge", &format!("{file_id}:USES:{language_id}")),
                kind: "USES".into(),
                from: file_id.clone(),
                to: language_id.clone(),
                attributes: BTreeMap::new(),
                provenance: provenance(&file.path, None),
                revision: None,
            });
        }
        facts.push(Fact {
            id: stable_id("fact", &format!("{}:language:{}", file.path, file.language)),
            subject: file_id,
            predicate: "language".into(),
            object: file.language.clone(),
            provenance: provenance(&file.path, None),
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

pub fn summarize(source: &SourceReport) -> GraphSummary {
    let graph = build_from_source(source);
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
    use atlas_model::FileFact;

    #[test]
    fn source_files_become_queryable_graph_primitives() {
        let source = SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 1,
            languages: BTreeMap::from([("rust".into(), 1)]),
            files: vec![FileFact { path: "crates/a/src/lib.rs".into(), language: "rust".into(), bytes: 42 }],
        };
        let graph = build_from_source(&source);
        assert!(graph.nodes.iter().any(|node| node.kind == "Repository"));
        assert!(graph.nodes.iter().any(|node| node.kind == "File"));
        assert!(graph.edges.iter().any(|edge| edge.kind == "CONTAINS"));
        assert!(graph.edges.iter().any(|edge| edge.kind == "USES"));
        assert_eq!(graph.facts.len(), 1);
    }
}
