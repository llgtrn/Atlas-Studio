//! Universal engineering graph primitives.
//!
//! Graph construction is deterministic and pure. Repository I/O remains outside core.

pub mod engineering_graph;

use crate::{evidence::Evidence, provenance::Provenance, temporal::RevisionRef};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use engineering_graph::{
    add_constraint_derivations, add_dependency_closure, build_repository_graph, build_source_graph,
    build_system_graph, summarize_graph, summarize_repository_graph, summarize_system_graph,
    summarize_system_graph_with_dependencies,
};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineeringGraph {
    pub schema: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub bindings: Vec<Binding>,
    pub facts: Vec<Fact>,
    pub evidence: Vec<Evidence>,
}
