//! Atlas core: product-neutral engineering semantics.
//!
//! This crate owns the vocabulary that every repository compilation converges on. It performs no
//! filesystem, Git, provider, donor or UI work; those responsibilities live in `adapter`,
//! `runtime`, or application crates.

pub mod capability;
pub mod census;
pub mod constraint;
pub mod evidence;
pub mod graph;
pub mod identity;
pub mod language;
pub mod provenance;
pub mod schema;
pub mod semantic;
pub mod state;
pub mod temporal;

pub use capability::{WorkPrepareReport, WorkRequest};
pub use census::{
    ArtifactDisposition, ArtifactKind, ArtifactRecord, DependencyActivation,
    DependencyClosureReport, DependencyClosureState, DependencyEcosystem, DependencyEdge,
    DependencyIdentity, DependencyRole, DependencySourceKind, DynamicDependencyObligation,
    InventoryReport,
};
pub use constraint::{CodingAdmission, declared_root_is_contained, validate_manifest};
pub use evidence::Evidence;
pub use graph::{
    Binding, Edge, EngineeringGraph, Fact, Node, add_constraint_derivations,
    add_dependency_closure, build_repository_graph, build_source_graph, build_system_graph,
    summarize_graph, summarize_repository_graph, summarize_system_graph,
    summarize_system_graph_with_dependencies,
};
pub use identity::{
    ArtifactId, CapabilityId, ContentFingerprint, EdgeId, EvidenceId, IntegrityDigest, NodeId,
    RawObservationId, RepositoryId, RevisionId, SemanticObligationId, SymbolId, TechnologyId,
    escape_identity_field, stable_id,
};
pub use language::adl::{
    AdlCompileReport, AdlDeclaration, AdlDiagnostic, AdlProgram, AdlSource, AdlToken, AtlasIr,
    BindingDecl, CapabilityDecl, ConstraintCheck, ConstraintDecl, ConstraintResult, DeclaredEdge,
    DeclaredGraph, DeclaredNode, DeclaredObservedDelta, EntityDecl, MaterializationDecl,
    RelationDecl, SourceSpan, TransformDecl, compile_adl, lex_adl, parse_adl_source,
};
pub use provenance::{Provenance, provenance};
pub use schema::{
    CensusReport, ConflictCandidate, DocsReport, DocumentFact, EpistemicStatus, FileFact,
    GraphSummary, NormalizationReport, RepoAudit, RepoManifest, SemanticFact, SemanticFactKind,
    SourceReport, SystemizeReport, TypedClosureAccounting,
};
pub use semantic::{
    CallDispatchKind, CallSiteIdentity, ConcurrencyIdentity, ConcurrencyKind,
    ControlFlowBlockIdentity, ControlFlowBlockKind, ControlFlowEdge, ControlFlowEdgeKind,
    DataFlowResolution, DiagnosticCode, EffectCategory, EffectIdentity, ExtractionDiagnostic,
    ExtractorIdentity, FunctionDeclarationKind, FunctionIdentity, FunctionOwner, FunctionParameter,
    FunctionSignature, OwnershipIdentity, OwnershipKind, OwnershipResolution, PersistenceIdentity,
    PersistenceKind, PersistenceResolution, PlaceRef, SemanticDimension, SemanticObligationRecord,
    SemanticObservation, SemanticRecordHeader, SemanticRecordId, SemanticScope,
    StateAccessIdentity, StateAccessKind, StateResolution, SymbolIdentity, SymbolRole,
    TypeIdentity, ValueIdentity, ValueRole,
};
pub use state::RepositorySnapshot;
pub use temporal::RevisionRef;

use serde::{Deserialize, Serialize};

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
                "check".into(),
                "parse".into(),
                "graph".into(),
            ],
        }
    }
}
