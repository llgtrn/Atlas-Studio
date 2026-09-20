use atlas_model::{GraphSummary, SourceReport};

pub fn summarize(source: &SourceReport) -> GraphSummary {
    let mut language_nodes: Vec<_> = source.languages.keys().cloned().collect();
    language_nodes.sort();
    GraphSummary {
        schema: "atlas.systemizer.engineering-graph-summary.v1".into(),
        semantic_grade: "SOURCE_FACT_GRAPH".into(),
        nodes_total: source.files_total + language_nodes.len(),
        edges_total: source.files_total,
        language_nodes,
    }
}
