//! Atlas core: product-neutral engineering semantics.
//!
//! This crate owns the vocabulary that every repository compilation converges on. It performs no
//! filesystem, Git, provider, donor or UI work; those responsibilities live in `adapter`,
//! `runtime`, or application crates.

pub mod census;
pub mod identity;
pub mod language;
pub mod model;

pub use identity::{
    ArtifactId, CapabilityId, ContentFingerprint, EdgeId, EvidenceId, IntegrityDigest, NodeId,
    RepositoryId, RevisionId, SymbolId, TechnologyId, stable_id,
};
pub use census::{ArtifactDisposition, ArtifactKind, ArtifactRecord, InventoryReport};
pub use language::adl::{
    AdlCompileReport, AdlDeclaration, AdlDiagnostic, AdlProgram, AdlSource, AdlToken, AtlasIr,
    BindingDecl, CapabilityDecl, ConstraintCheck, ConstraintDecl, ConstraintResult, DeclaredEdge,
    DeclaredGraph, DeclaredNode, DeclaredObservedDelta, EntityDecl, MaterializationDecl,
    RelationDecl, SourceSpan, TransformDecl, compile_adl, lex_adl, parse_adl_source,
};
pub use model::{
    Binding, CodingAdmission, DocsReport, DocumentFact, Edge, EngineeringGraph, Evidence, Fact,
    FileFact, GraphSummary, Node, Provenance, RepoAudit, RepoManifest, RepositorySnapshot,
    RevisionRef, SourceReport, SystemizeReport, WorkPrepareReport, WorkRequest,
    build_repository_graph, build_source_graph, build_system_graph, provenance, summarize_graph,
    summarize_repository_graph, summarize_system_graph, validate_manifest,
};
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
