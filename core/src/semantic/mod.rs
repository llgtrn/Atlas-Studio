//! Typed semantic record kernel (R4.1).
//!
//! This module materializes the shared identity/header primitives that every deep semantic fact
//! family (function, symbol, type, call, control-flow, data-flow, state, effect, ...) builds on.
//! It carries no filesystem, parser or extraction logic; that belongs to `adapter`/`runtime`. See
//! `.atlas/contracts/SEMANTIC-FACTS.md` and `.atlas/contracts/SEMANTIC-EXTRACTION.md`.

pub mod call;
pub mod control_flow;
pub mod data_flow;
pub mod diagnostic;
pub mod effect;
pub mod function;
pub mod obligation;
pub mod observation;
pub mod state;
pub mod symbol;
pub mod types;

pub use call::{CallDispatchKind, CallSiteIdentity};
pub use control_flow::{
    ControlFlowBlockIdentity, ControlFlowBlockKind, ControlFlowEdge, ControlFlowEdgeKind,
};
pub use data_flow::{DataFlowResolution, ValueIdentity, ValueRole};
pub use diagnostic::{DiagnosticCode, ExtractionDiagnostic};
pub use effect::{EffectCategory, EffectIdentity};
pub use function::{
    FunctionDeclarationKind, FunctionIdentity, FunctionOwner, FunctionParameter, FunctionSignature,
};
pub use obligation::SemanticObligationRecord;
pub use observation::SemanticObservation;
pub use state::{StateAccessIdentity, StateAccessKind, StateResolution};
pub use symbol::{SymbolIdentity, SymbolRole};
pub use types::TypeIdentity;

use crate::identity::{EvidenceId, RepositoryId, stable_id};
use crate::provenance::Provenance;
use crate::schema::EpistemicStatus;
use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

/// The typed semantic obligation families R4 extraction/census/normalization operate over. See
/// `.atlas/contracts/SEMANTIC-EXTRACTION.md`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticDimension {
    Symbol,
    Type,
    FunctionIdentity,
    FunctionSignature,
    Call,
    ControlFlow,
    DataFlow,
    State,
    Effect,
    Ownership,
    Concurrency,
    Persistence,
}

impl SemanticDimension {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Symbol => "SYMBOL",
            Self::Type => "TYPE",
            Self::FunctionIdentity => "FUNCTION_IDENTITY",
            Self::FunctionSignature => "FUNCTION_SIGNATURE",
            Self::Call => "CALL",
            Self::ControlFlow => "CONTROL_FLOW",
            Self::DataFlow => "DATA_FLOW",
            Self::State => "STATE",
            Self::Effect => "EFFECT",
            Self::Ownership => "OWNERSHIP",
            Self::Concurrency => "CONCURRENCY",
            Self::Persistence => "PERSISTENCE",
        }
    }
}

/// Deterministic identity of a typed semantic record, scoped by the dimension that produced it.
///
/// Constructed from a family's `identity_key()`, never from enumeration order, so equal inputs
/// always produce equal ids regardless of traversal/thread order (see
/// `.atlas/contracts/SEMANTIC-EXTRACTION.md#determinism`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticRecordId(String);

impl SemanticRecordId {
    pub fn new(dimension: SemanticDimension, identity_key: &str) -> Self {
        Self(stable_id(
            &format!("semantic:{}", dimension.as_str()),
            identity_key,
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A hierarchical containing scope (crate/module/impl/trait/function nesting), outermost first.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticScope {
    pub segments: Vec<String>,
}

impl SemanticScope {
    pub fn new(segments: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            segments: segments.into_iter().map(Into::into).collect(),
        }
    }

    pub fn join(&self) -> String {
        self.segments.join("::")
    }
}

/// Identity and implementation version of the extractor that produced a semantic record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtractorIdentity {
    pub id: String,
    pub version: String,
}

/// Common header every typed semantic record carries, per
/// `.atlas/contracts/SEMANTIC-FACTS.md#common-semantic-header`.
///
/// `Subject` is the family-specific identity type (e.g. `FunctionIdentity`, `SymbolIdentity`).
/// Parameterizing on it keeps every fact family typed instead of collapsing into one generic
/// subject/predicate/object triple.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticRecordHeader<Subject> {
    pub record_id: SemanticRecordId,
    pub dimension: SemanticDimension,
    pub status: EpistemicStatus,
    pub subject: Subject,
    pub scope: SemanticScope,
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub extractor: ExtractorIdentity,
    pub evidence_refs: Vec<EvidenceId>,
    pub provenance: Provenance,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extractor() -> ExtractorIdentity {
        ExtractorIdentity {
            id: "atlas.rust.extractor".into(),
            version: "0.1.0".into(),
        }
    }

    fn revision(sha: &str) -> RevisionRef {
        RevisionRef {
            kind: "git".into(),
            value: sha.into(),
        }
    }

    #[test]
    fn record_id_is_deterministic_for_identical_identity_key() {
        let a = SemanticRecordId::new(SemanticDimension::Symbol, "repo@rev|module::foo|DEFINITION");
        let b = SemanticRecordId::new(SemanticDimension::Symbol, "repo@rev|module::foo|DEFINITION");
        assert_eq!(a, b);
    }

    #[test]
    fn record_id_differs_by_dimension_for_identical_identity_key() {
        let symbol = SemanticRecordId::new(SemanticDimension::Symbol, "same-key");
        let call = SemanticRecordId::new(SemanticDimension::Call, "same-key");
        assert_ne!(symbol, call);
    }

    #[test]
    fn header_carries_canonical_epistemic_status_and_typed_subject() {
        let header = SemanticRecordHeader {
            record_id: SemanticRecordId::new(SemanticDimension::Symbol, "k"),
            dimension: SemanticDimension::Symbol,
            status: EpistemicStatus::Observed,
            subject: SymbolIdentity {
                repository: RepositoryId::new("atlas-studio"),
                revision: revision("abc123"),
                scope: SemanticScope::new(["core", "lib"]),
                name: "run".into(),
                role: SymbolRole::Definition,
            },
            scope: SemanticScope::new(["core", "lib"]),
            repository: RepositoryId::new("atlas-studio"),
            revision: revision("abc123"),
            extractor: extractor(),
            evidence_refs: Vec::new(),
            provenance: crate::provenance::provenance("core/src/lib.rs", "atlas.rust.extractor"),
        };

        assert_eq!(header.status, EpistemicStatus::Observed);
        assert_eq!(header.subject.name, "run");
    }
}
