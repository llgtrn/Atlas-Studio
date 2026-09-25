//! Universal engineering graph primitives.
//!
//! Graph construction is deterministic and pure. Repository I/O remains outside core.

pub mod engineering_graph;

use crate::EpistemicStatus;
use crate::{evidence::Evidence, provenance::Provenance, temporal::RevisionRef};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

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
    /// G134 (NA-EPISTEMIC-UNIFICATION): how the binding is known, in the one vocabulary; a
    /// numeric confidence was a second vocabulary UNIVERSAL-GRAPH forbids.
    pub status: EpistemicStatus,
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
    /// G134: the fact's own epistemic status (was a numeric confidence derived from it).
    pub status: EpistemicStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineeringGraph {
    pub schema: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub bindings: Vec<Binding>,
    pub facts: Vec<Fact>,
    pub evidence: Vec<Evidence>,
    /// Derived membership index over `nodes[..].id`, so `ensure_node`'s dedup is O(1) instead of
    /// a linear scan (which made graph construction quadratic: 145 s for 47k nodes, ADR 0009).
    /// Never serialized and never part of equality.
    #[serde(skip)]
    pub(crate) node_index: NodeIdIndex,
}

/// Incrementally maintained set of node ids.
///
/// Contract: `nodes` is append-only between lookups -- nodes are pushed (by `ensure_node` or
/// directly), never re-identified, reordered or replaced in place. That is the only way Atlas
/// mutates it. The index catches up on every appended node, including direct `nodes.push`, and
/// rebuilds from scratch if `nodes` shrinks. Rewriting already-indexed positions (e.g. truncating
/// and pushing back to the same length) is outside the contract; debug builds catch it through the
/// last-indexed-id sentinel.
#[derive(Debug, Clone, Default)]
pub(crate) struct NodeIdIndex {
    ids: HashSet<String>,
    indexed: usize,
    last_indexed_id: Option<String>,
}

impl PartialEq for NodeIdIndex {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl NodeIdIndex {
    pub(crate) fn contains(&mut self, nodes: &[Node], id: &str) -> bool {
        if nodes.len() < self.indexed {
            *self = Self::default();
        }
        debug_assert!(
            self.indexed == 0
                || self.last_indexed_id.as_deref() == Some(nodes[self.indexed - 1].id.as_str()),
            "EngineeringGraph.nodes was rewritten in place; NodeIdIndex requires append-only use"
        );
        for node in &nodes[self.indexed..] {
            self.ids.insert(node.id.clone());
        }
        self.indexed = nodes.len();
        self.last_indexed_id = nodes.last().map(|node| node.id.clone());
        self.ids.contains(id)
    }
}
