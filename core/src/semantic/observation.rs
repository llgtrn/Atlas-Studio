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

    /// The wrapped header's own `dimension` field, independent of the variant tag. Used only to
    /// check `is_dimension_consistent`; `dimension()` above (derived from the variant) remains the
    /// authoritative value everywhere else.
    fn header_dimension(&self) -> SemanticDimension {
        match self {
            Self::FunctionIdentity(header) => header.dimension,
            Self::FunctionSignature(header) => header.dimension,
            Self::Symbol(header) => header.dimension,
            Self::Type(header) => header.dimension,
            Self::Call(header) => header.dimension,
            Self::ControlFlow(header) => header.dimension,
            Self::DataFlow(header) => header.dimension,
            Self::State(header) => header.dimension,
            Self::Effect(header) => header.dimension,
        }
    }

    /// `true` iff the wrapped header's `dimension` field agrees with `self.dimension()`.
    ///
    /// Rust's type system ties each variant to its `Subject` type (e.g. `Type(SemanticRecordHeader
    /// <TypeIdentity>)`), but nothing ties the header's plain `dimension: SemanticDimension` field
    /// to that variant -- a `Type(..)` observation whose header still says `dimension:
    /// SemanticDimension::Symbol` compiles. This method is the explicit check for that invariant;
    /// callers that build a `SemanticObservation` by hand (extractors, test fixtures) should use
    /// it rather than trust construction alone.
    pub fn is_dimension_consistent(&self) -> bool {
        self.header_dimension() == self.dimension()
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

    fn symbol_header(dimension: SemanticDimension) -> SemanticRecordHeader<SymbolIdentity> {
        SemanticRecordHeader {
            record_id: SemanticRecordId::new(SemanticDimension::Symbol, "k"),
            dimension,
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
        }
    }

    fn type_header(dimension: SemanticDimension) -> SemanticRecordHeader<crate::TypeIdentity> {
        SemanticRecordHeader {
            record_id: SemanticRecordId::new(SemanticDimension::Type, "k"),
            dimension,
            status: EpistemicStatus::Observed,
            subject: crate::TypeIdentity {
                repository: RepositoryId::new("atlas-studio"),
                revision: RevisionRef {
                    kind: "git".into(),
                    value: "abc123".into(),
                },
                scope: SemanticScope::new(["core"]),
                name: "Result".into(),
                canonical: None,
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
        }
    }

    // --- 1. Symbol variant + Symbol dimension is valid ---------------------------------------

    #[test]
    fn symbol_variant_with_symbol_dimension_is_consistent() {
        let observation = SemanticObservation::Symbol(symbol_header(SemanticDimension::Symbol));
        assert!(observation.is_dimension_consistent());
    }

    // --- 2. Type variant + Type dimension is valid --------------------------------------------

    #[test]
    fn type_variant_with_type_dimension_is_consistent() {
        let observation = SemanticObservation::Type(type_header(SemanticDimension::Type));
        assert!(observation.is_dimension_consistent());
    }

    // --- 3. a manually malformed variant/header mismatch is detected -------------------------

    #[test]
    fn malformed_variant_header_mismatch_is_detected() {
        // The type system allows this: the variant tag says Symbol, but the header's own
        // `dimension` field says Type. This is exactly the bug an earlier test fixture had
        // (mutating `header.dimension` without changing the enum variant); the invariant check
        // must catch it rather than silently accept it.
        let mismatched = SemanticObservation::Symbol(symbol_header(SemanticDimension::Type));
        assert!(!mismatched.is_dimension_consistent());
        // dimension() stays authoritative (derived from the variant), never from the header field.
        assert_eq!(mismatched.dimension(), SemanticDimension::Symbol);
    }
}
