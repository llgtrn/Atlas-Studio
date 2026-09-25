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

crate::vocabulary_enum! {
    /// G135 (NA-TYPED-RELATIONS, ADR 0054): every relation the engineering graph states. An edge
    /// states a typed relation (UNIVERSAL-GRAPH); a declared ADL relation outside the declared
    /// kinds is rejected at compile time. No kind is causal: a call is an invocation and an
    /// effect site is where an effect is performed, never a cause (causal claims need a
    /// counterfactual record Atlas does not yet have).
    pub enum EdgeKind {
        Contains => "CONTAINS",
        Uses => "USES",
        Documents => "DOCUMENTS",
        Materializes => "MATERIALIZES",
        MaterializesAs => "MATERIALIZES_AS",
        BindsTo => "BINDS_TO",
        DependsOn => "DEPENDS_ON",
        Provides => "PROVIDES",
        Targets => "TARGETS",
        SupportedBy => "SUPPORTED_BY",
        ResolvesDependency => "RESOLVES_DEPENDENCY",
        ImplementedOn => "IMPLEMENTED_ON",
        HasQuantity => "HAS_QUANTITY",
        HasParameterType => "HAS_PARAMETER_TYPE",
        HasReturnType => "HAS_RETURN_TYPE",
        HasBlock => "HAS_BLOCK",
        HasValue => "HAS_VALUE",
        HasAccess => "HAS_ACCESS",
        HasOperation => "HAS_OPERATION",
        MakesCall => "MAKES_CALL",
        Calls => "CALLS",
        BindsArgument => "BINDS_ARGUMENT",
        BindsResult => "BINDS_RESULT",
        ResolvesTo => "RESOLVES_TO",
        RefersToPlace => "REFERS_TO_PLACE",
        ProducesEffect => "PRODUCES_EFFECT",
        ProducesConcurrencyOp => "PRODUCES_CONCURRENCY_OP",
        ProducesPersistenceOp => "PRODUCES_PERSISTENCE_OP",
        Fallthrough => "FALLTHROUGH",
        Branch => "BRANCH",
        LoopRepeat => "LOOP_REPEAT",
        Return => "RETURN",
        Break => "BREAK",
        Panic => "PANIC",
        Unresolved => "UNRESOLVED",
    }
}

/// What kind of relation an edge kind states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeCategory {
    /// Structure of the repository and its declarations.
    Structural,
    /// Authored in ADL.
    Declared,
    /// Observed or derived program semantics.
    Semantic,
    /// Control flow between basic blocks.
    ControlFlow,
}

/// How many targets one source may have and how many sources one target may have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
}

impl EdgeKind {
    /// The ADL relation names that declare an edge (`A ->depends_on-> B`); any other name is an
    /// untyped relation and is rejected (ATLAS-E065).
    pub const DECLARED: [(&'static str, Self); 3] = [
        ("depends_on", Self::DependsOn),
        ("provides", Self::Provides),
        ("contains", Self::Contains),
    ];

    /// The kind an ADL relation name declares, if it is one.
    pub fn declared(relation: &str) -> Option<Self> {
        Self::DECLARED
            .iter()
            .find(|(name, _)| *name == relation)
            .map(|(_, kind)| *kind)
    }

    pub fn category(self) -> EdgeCategory {
        use EdgeKind::*;
        match self {
            DependsOn | Provides => EdgeCategory::Declared,
            Contains | Uses | Documents | Materializes | MaterializesAs | BindsTo | Targets
            | SupportedBy | ResolvesDependency => EdgeCategory::Structural,
            Fallthrough | Branch | LoopRepeat | Return | Break | Panic | Unresolved => {
                EdgeCategory::ControlFlow
            }
            ImplementedOn
            | HasQuantity
            | HasParameterType
            | HasReturnType
            | HasBlock
            | HasValue
            | HasAccess
            | HasOperation
            | MakesCall
            | Calls
            | BindsArgument
            | BindsResult
            | ResolvesTo
            | RefersToPlace
            | ProducesEffect
            | ProducesConcurrencyOp
            | ProducesPersistenceOp => EdgeCategory::Semantic,
        }
    }

    pub fn cardinality(self) -> Cardinality {
        use EdgeKind::*;
        match self {
            // One owner, many owned.
            Contains | HasBlock | HasValue | HasAccess | HasOperation | MakesCall | HasQuantity
            | Materializes => Cardinality::OneToMany,
            // Each site or value resolves to at most one target; many may share it.
            ResolvesTo | RefersToPlace | BindsResult | HasReturnType | ImplementedOn
            | ResolvesDependency | MaterializesAs => Cardinality::ManyToOne,
            // A block has at most one successor of each kind.
            Fallthrough | Return | Break | Panic | LoopRepeat => Cardinality::OneToOne,
            Uses
            | Documents
            | BindsTo
            | DependsOn
            | Provides
            | Targets
            | SupportedBy
            | HasParameterType
            | Calls
            | BindsArgument
            | ProducesEffect
            | ProducesConcurrencyOp
            | ProducesPersistenceOp
            | Branch
            | Unresolved => Cardinality::ManyToMany,
        }
    }

    /// Whether the source owns the target: deleting the source deletes what it owns.
    pub fn owns(self) -> bool {
        use EdgeKind::*;
        matches!(
            self,
            Contains | HasBlock | HasValue | HasAccess | HasOperation | MakesCall | HasQuantity
        )
    }

    /// The edge kind of a control-flow successor.
    pub fn control_flow(kind: crate::semantic::ControlFlowEdgeKind) -> Self {
        use crate::semantic::ControlFlowEdgeKind as K;
        match kind {
            K::Fallthrough => Self::Fallthrough,
            K::Branch => Self::Branch,
            K::LoopRepeat => Self::LoopRepeat,
            K::Return => Self::Return,
            K::Break => Self::Break,
            K::Panic => Self::Panic,
            K::Unresolved => Self::Unresolved,
        }
    }

    /// No edge kind states causation (ADR 0054).
    pub fn causal(self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Edge {
    pub id: String,
    pub kind: EdgeKind,
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

#[cfg(test)]
mod edge_kind_tests {
    use super::*;

    /// G135: every edge kind states its category, cardinality and ownership; the declared ADL
    /// relations are exactly the typed ones; no kind is causal.
    #[test]
    fn every_edge_kind_is_typed_with_its_schema() {
        assert_eq!(EdgeKind::declared("depends_on"), Some(EdgeKind::DependsOn));
        assert_eq!(EdgeKind::declared("contains"), Some(EdgeKind::Contains));
        assert_eq!(EdgeKind::declared("causes"), None);
        assert_eq!(
            EdgeKind::declared("DEPENDS_ON"),
            None,
            "ADL names are lower case"
        );
        assert!(EdgeKind::Contains.owns() && !EdgeKind::Calls.owns());
        assert_eq!(EdgeKind::ResolvesTo.cardinality(), Cardinality::ManyToOne);
        assert_eq!(EdgeKind::DependsOn.category(), EdgeCategory::Declared);
        assert_eq!(
            EdgeKind::control_flow(crate::semantic::ControlFlowEdgeKind::Branch),
            EdgeKind::Branch
        );
        // Wire-compatible: the name an edge kind serializes to is the string it replaced.
        assert_eq!(EdgeKind::ProducesEffect.as_str(), "PRODUCES_EFFECT");
        assert!(!EdgeKind::Calls.causal() && !EdgeKind::ProducesEffect.causal());
    }
}
