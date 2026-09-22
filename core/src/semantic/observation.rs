//! Uniform carrier for one typed semantic observation, whichever family produced it.
//!
//! `ExtractionBatch` (materialized in `adapter`, per `.atlas/contracts/SEMANTIC-EXTRACTION.md`)
//! must hold observations across multiple `SemanticDimension` families in one collection without
//! collapsing them into an untyped subject/predicate/object triple. `SemanticObservation` is that
//! neutral carrier: each variant wraps a typed `SemanticRecordHeader<Subject>` for exactly one
//! family. Ownership/concurrency/persistence have no identity kernel yet (deferred to R4.10), so
//! they carry no variant here either; their obligations are still accounted for, just with zero
//! observations.

use super::{
    CallSiteIdentity, ControlFlowBlockIdentity, EffectIdentity, FunctionIdentity,
    FunctionSignature, SemanticDimension, SemanticRecordHeader, SemanticRecordId, StateIdentity,
    SymbolIdentity, TypeIdentity, ValueIdentity,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SemanticObservation {
    FunctionIdentity(SemanticRecordHeader<FunctionIdentity>),
    // Boxed: FunctionSignature embeds a FunctionIdentity plus parameter/generic vectors, making
    // it far larger than the other variants; boxing keeps the enum itself compact.
    FunctionSignature(Box<SemanticRecordHeader<FunctionSignature>>),
    Symbol(SemanticRecordHeader<SymbolIdentity>),
    Type(SemanticRecordHeader<TypeIdentity>),
    Call(SemanticRecordHeader<CallSiteIdentity>),
    ControlFlow(SemanticRecordHeader<ControlFlowBlockIdentity>),
    DataFlow(SemanticRecordHeader<ValueIdentity>),
    State(SemanticRecordHeader<StateIdentity>),
    Effect(SemanticRecordHeader<EffectIdentity>),
}

impl SemanticObservation {
    pub const fn dimension(&self) -> SemanticDimension {
        match self {
            Self::FunctionIdentity(_) => SemanticDimension::FunctionIdentity,
            Self::FunctionSignature(_) => SemanticDimension::FunctionSignature,
            Self::Symbol(_) => SemanticDimension::Symbol,
            Self::Type(_) => SemanticDimension::Type,
            Self::Call(_) => SemanticDimension::Call,
            Self::ControlFlow(_) => SemanticDimension::ControlFlow,
            Self::DataFlow(_) => SemanticDimension::DataFlow,
            Self::State(_) => SemanticDimension::State,
            Self::Effect(_) => SemanticDimension::Effect,
        }
    }

    pub fn record_id(&self) -> &SemanticRecordId {
        match self {
            Self::FunctionIdentity(header) => &header.record_id,
            Self::FunctionSignature(header) => &header.record_id,
            Self::Symbol(header) => &header.record_id,
            Self::Type(header) => &header.record_id,
            Self::Call(header) => &header.record_id,
            Self::ControlFlow(header) => &header.record_id,
            Self::DataFlow(header) => &header.record_id,
            Self::State(header) => &header.record_id,
            Self::Effect(header) => &header.record_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::RepositoryId;
    use crate::provenance::provenance;
    use crate::schema::EpistemicStatus;
    use crate::semantic::{ExtractorIdentity, SemanticScope, SymbolRole};
    use crate::temporal::RevisionRef;

    #[test]
    fn dimension_and_record_id_match_the_wrapped_header() {
        let header = SemanticRecordHeader {
            record_id: SemanticRecordId::new(SemanticDimension::Symbol, "k"),
            dimension: SemanticDimension::Symbol,
            status: EpistemicStatus::Observed,
            subject: SymbolIdentity {
                repository: RepositoryId::new("atlas-studio"),
                revision: RevisionRef {
                    kind: "git".into(),
                    value: "abc123".into(),
                },
                scope: SemanticScope::new(["core"]),
                name: "run".into(),
                role: SymbolRole::Definition,
            },
            scope: SemanticScope::new(["core"]),
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            extractor: ExtractorIdentity {
                id: "atlas.test".into(),
                version: "0.1.0".into(),
            },
            evidence_refs: Vec::new(),
            provenance: provenance("core/src/lib.rs", "atlas.test"),
        };
        let observation = SemanticObservation::Symbol(header.clone());

        assert_eq!(observation.dimension(), SemanticDimension::Symbol);
        assert_eq!(observation.record_id(), &header.record_id);
    }
}
