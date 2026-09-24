//! Uniform carrier for one typed semantic observation, whichever family produced it.
//!
//! `ExtractionBatch` (materialized in `adapter`, per `.atlas/contracts/SEMANTIC-EXTRACTION.md`)
//! must hold observations across multiple `SemanticDimension` families in one collection without
//! collapsing them into an untyped subject/predicate/object triple. `SemanticObservation` is that
//! neutral carrier: each variant wraps a typed `SemanticRecordHeader<Subject>` for exactly one
//! family.

use super::{
    CallSiteIdentity, ConcurrencyIdentity, ControlFlowBlockIdentity, EffectIdentity,
    FunctionIdentity, FunctionSignature, OwnershipIdentity, PersistenceIdentity, SemanticDimension,
    SemanticRecordHeader, SemanticRecordId, StateAccessIdentity, SymbolIdentity, TypeIdentity,
    ValueIdentity,
};
use crate::identity::{EvidenceId, RawObservationId, stable_id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SemanticObservation {
    // Boxed: R4.4 grew `FunctionIdentity` with `declaration_kind`/`owner`/`generics` (`owner`
    // embeds an `Option<TypeIdentity>`), making it far larger than the smallest variants; boxing
    // keeps the enum itself compact.
    FunctionIdentity(Box<SemanticRecordHeader<FunctionIdentity>>),
    // Boxed: FunctionSignature embeds a FunctionIdentity plus parameter/generic vectors, making
    // it far larger than the other variants; boxing keeps the enum itself compact.
    FunctionSignature(Box<SemanticRecordHeader<FunctionSignature>>),
    Symbol(SemanticRecordHeader<SymbolIdentity>),
    Type(SemanticRecordHeader<TypeIdentity>),
    Call(SemanticRecordHeader<CallSiteIdentity>),
    ControlFlow(SemanticRecordHeader<ControlFlowBlockIdentity>),
    DataFlow(SemanticRecordHeader<ValueIdentity>),
    State(SemanticRecordHeader<StateAccessIdentity>),
    Effect(SemanticRecordHeader<EffectIdentity>),
    Ownership(SemanticRecordHeader<OwnershipIdentity>),
    Concurrency(SemanticRecordHeader<ConcurrencyIdentity>),
    Persistence(SemanticRecordHeader<PersistenceIdentity>),
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
            Self::Ownership(_) => SemanticDimension::Ownership,
            Self::Concurrency(_) => SemanticDimension::Concurrency,
            Self::Persistence(_) => SemanticDimension::Persistence,
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
            Self::Ownership(header) => &header.record_id,
            Self::Concurrency(header) => &header.record_id,
            Self::Persistence(header) => &header.record_id,
        }
    }

    pub fn status(&self) -> crate::EpistemicStatus {
        match self {
            Self::FunctionIdentity(header) => header.status,
            Self::FunctionSignature(header) => header.status,
            Self::Symbol(header) => header.status,
            Self::Type(header) => header.status,
            Self::Call(header) => header.status,
            Self::ControlFlow(header) => header.status,
            Self::DataFlow(header) => header.status,
            Self::State(header) => header.status,
            Self::Effect(header) => header.status,
            Self::Ownership(header) => header.status,
            Self::Concurrency(header) => header.status,
            Self::Persistence(header) => header.status,
        }
    }

    pub fn scope(&self) -> &super::SemanticScope {
        match self {
            Self::FunctionIdentity(header) => &header.scope,
            Self::FunctionSignature(header) => &header.scope,
            Self::Symbol(header) => &header.scope,
            Self::Type(header) => &header.scope,
            Self::Call(header) => &header.scope,
            Self::ControlFlow(header) => &header.scope,
            Self::DataFlow(header) => &header.scope,
            Self::State(header) => &header.scope,
            Self::Effect(header) => &header.scope,
            Self::Ownership(header) => &header.scope,
            Self::Concurrency(header) => &header.scope,
            Self::Persistence(header) => &header.scope,
        }
    }

    pub fn provenance(&self) -> &crate::Provenance {
        match self {
            Self::FunctionIdentity(header) => &header.provenance,
            Self::FunctionSignature(header) => &header.provenance,
            Self::Symbol(header) => &header.provenance,
            Self::Type(header) => &header.provenance,
            Self::Call(header) => &header.provenance,
            Self::ControlFlow(header) => &header.provenance,
            Self::DataFlow(header) => &header.provenance,
            Self::State(header) => &header.provenance,
            Self::Effect(header) => &header.provenance,
            Self::Ownership(header) => &header.provenance,
            Self::Concurrency(header) => &header.provenance,
            Self::Persistence(header) => &header.provenance,
        }
    }

    pub fn evidence_refs(&self) -> &[EvidenceId] {
        match self {
            Self::FunctionIdentity(header) => &header.evidence_refs,
            Self::FunctionSignature(header) => &header.evidence_refs,
            Self::Symbol(header) => &header.evidence_refs,
            Self::Type(header) => &header.evidence_refs,
            Self::Call(header) => &header.evidence_refs,
            Self::ControlFlow(header) => &header.evidence_refs,
            Self::DataFlow(header) => &header.evidence_refs,
            Self::State(header) => &header.evidence_refs,
            Self::Effect(header) => &header.evidence_refs,
            Self::Ownership(header) => &header.evidence_refs,
            Self::Concurrency(header) => &header.evidence_refs,
            Self::Persistence(header) => &header.evidence_refs,
        }
    }

    /// Deterministic identity of this exact RAW observation -- distinct from `record_id()`, the
    /// semantic CLAIM identity. `record_id()` is derived only from a family's `identity_key()`
    /// (e.g. repository/revision/scope/name for a symbol), which deliberately does NOT include
    /// which extractor produced the observation, what evidence backs it, its epistemic status, or
    /// (for families like `FunctionSignature` whose `identity_key()` covers only the underlying
    /// `FunctionIdentity`) every field of the typed payload itself. Two independent extractors --
    /// or the same extractor reporting genuinely different content -- MAY legitimately share a
    /// `record_id()` while differing here.
    ///
    /// Only an observation identical in every field -- claim identity, extractor id/version,
    /// epistemic status, evidence refs, provenance, and the full typed payload -- shares this id
    /// with another. Callers MUST key raw-observation de-duplication on THIS identity, never on
    /// `record_id()` alone: collapsing by `record_id()` would silently erase one extractor's
    /// observation whenever another extractor (or a later revision of this one) reports the same
    /// semantic claim (`.atlas/contracts/SEMANTIC-EXTRACTION.md#multi-engine-extraction`:
    /// "Independent extractors MAY analyze the same dimension. Their identities/evidence remain
    /// separate ... one extractor may not overwrite another"; `.atlas/contracts/NORMALIZATION.md`:
    /// `RawRecordId != NormalizedRecordId`).
    pub fn raw_observation_id(&self) -> RawObservationId {
        fn seed<Subject: std::fmt::Debug>(
            record_id: &SemanticRecordId,
            header: &SemanticRecordHeader<Subject>,
        ) -> String {
            // `EvidenceId` has no charset restriction (`typed_id!`'s constructor accepts any
            // `impl Into<String>`) -- every current production call site happens to construct one
            // via `stable_id(...)`, whose fixed `"{prefix}:{hash:016x}"` shape never contains a
            // `,`, but this function's own contract above ("only an observation identical in
            // every field... shares this id") must hold unconditionally, not merely for today's
            // callers. Escaping each ref before joining (the same `escape_identity_field` pattern
            // already used for `DependencyIdentity`/`DependencyEdge`'s own composite keys) prevents
            // one evidence ref containing a literal `,` from becoming indistinguishable, after
            // joining, from two separate refs split at that same character.
            let evidence_refs: Vec<String> = header
                .evidence_refs
                .iter()
                .map(|id| crate::identity::escape_identity_field(id.as_str(), ','))
                .collect();
            format!(
                "{}|{}:{}|{}|evidence=[{}]|provenance={}:{}:{}:{}|payload={:?}",
                record_id.as_str(),
                header.extractor.id,
                header.extractor.version,
                header.status.as_str(),
                evidence_refs.join(","),
                header.provenance.source_path,
                header.provenance.extractor,
                header.provenance.content_hash.as_deref().unwrap_or(""),
                header.provenance.span.as_deref().unwrap_or(""),
                header.subject,
            )
        }

        let content = match self {
            Self::FunctionIdentity(header) => seed(&header.record_id, header),
            Self::FunctionSignature(header) => seed(&header.record_id, header),
            Self::Symbol(header) => seed(&header.record_id, header),
            Self::Type(header) => seed(&header.record_id, header),
            Self::Call(header) => seed(&header.record_id, header),
            Self::ControlFlow(header) => seed(&header.record_id, header),
            Self::DataFlow(header) => seed(&header.record_id, header),
            Self::State(header) => seed(&header.record_id, header),
            Self::Effect(header) => seed(&header.record_id, header),
            Self::Ownership(header) => seed(&header.record_id, header),
            Self::Concurrency(header) => seed(&header.record_id, header),
            Self::Persistence(header) => seed(&header.record_id, header),
        };
        RawObservationId::new(stable_id("raw-observation", &content))
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
            Self::Ownership(header) => header.dimension,
            Self::Concurrency(header) => header.dimension,
            Self::Persistence(header) => header.dimension,
        }
    }

    /// Debug-formatted subject payload only -- the same `payload={:?}` component already used
    /// inside `raw_observation_id()`'s seed, exposed on its own. Used ONLY to detect whether two
    /// observations sharing a `record_id` (the same semantic claim identity) actually agree on
    /// WHAT that claim is (`normalize::detect_conflict_candidates`,
    /// `.atlas/contracts/NORMALIZATION.md#conflict-handling`). Not an identity, hash or ordering
    /// source: two observations may legitimately have equal `subject_repr()` while differing in
    /// extractor/evidence/provenance (independent corroboration), and the reverse (differing
    /// `subject_repr()` under the same `record_id`) is exactly the conflict-candidate signal.
    pub fn subject_repr(&self) -> String {
        match self {
            Self::FunctionIdentity(header) => format!("{:?}", header.subject),
            Self::FunctionSignature(header) => format!("{:?}", header.subject),
            Self::Symbol(header) => format!("{:?}", header.subject),
            Self::Type(header) => format!("{:?}", header.subject),
            Self::Call(header) => format!("{:?}", header.subject),
            Self::ControlFlow(header) => format!("{:?}", header.subject),
            Self::DataFlow(header) => format!("{:?}", header.subject),
            Self::State(header) => format!("{:?}", header.subject),
            Self::Effect(header) => format!("{:?}", header.subject),
            Self::Ownership(header) => format!("{:?}", header.subject),
            Self::Concurrency(header) => format!("{:?}", header.subject),
            Self::Persistence(header) => format!("{:?}", header.subject),
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

    // --- 4. raw_observation_id: R4.3.3 raw-vs-claim identity separation ----------------------

    #[test]
    fn raw_observation_id_differs_from_record_id() {
        let observation = SemanticObservation::Symbol(symbol_header(SemanticDimension::Symbol));
        assert_ne!(
            observation.raw_observation_id().as_str(),
            observation.record_id().as_str()
        );
    }

    #[test]
    fn two_extractors_reporting_the_same_claim_get_different_raw_observation_ids() {
        let mut header_a = symbol_header(SemanticDimension::Symbol);
        header_a.extractor = ExtractorIdentity {
            id: "extractor-a".into(),
            version: "0.1.0".into(),
        };
        let mut header_b = header_a.clone();
        header_b.extractor = ExtractorIdentity {
            id: "extractor-b".into(),
            version: "0.1.0".into(),
        };

        let a = SemanticObservation::Symbol(header_a);
        let b = SemanticObservation::Symbol(header_b);

        // Same semantic claim identity (record_id unchanged by extractor)...
        assert_eq!(a.record_id(), b.record_id());
        // ...but distinct raw observation identity: neither may silently erase the other.
        assert_ne!(a.raw_observation_id(), b.raw_observation_id());
    }

    #[test]
    fn identical_raw_observations_share_a_raw_observation_id() {
        let header = symbol_header(SemanticDimension::Symbol);
        let a = SemanticObservation::Symbol(header.clone());
        let b = SemanticObservation::Symbol(header);
        assert_eq!(a.raw_observation_id(), b.raw_observation_id());
    }

    #[test]
    fn differing_evidence_refs_change_the_raw_observation_id_even_with_the_same_extractor() {
        let mut header_a = symbol_header(SemanticDimension::Symbol);
        header_a.evidence_refs = vec![crate::identity::EvidenceId::new("evidence:a")];
        let mut header_b = header_a.clone();
        header_b.evidence_refs = vec![crate::identity::EvidenceId::new("evidence:b")];

        let a = SemanticObservation::Symbol(header_a);
        let b = SemanticObservation::Symbol(header_b);

        assert_eq!(a.record_id(), b.record_id());
        assert_ne!(a.raw_observation_id(), b.raw_observation_id());
    }

    #[test]
    fn an_evidence_ref_containing_a_comma_never_collides_with_a_differently_split_evidence_list() {
        // `EvidenceId` has no charset restriction -- nothing stops one evidence ref from
        // containing a literal `,`. Before escaping, joining `evidence_refs` with `,` made a
        // single ref `"ev,idence"` indistinguishable, once joined, from two separate refs `"ev"`
        // and `"idence"` -- a real violation of this method's own documented contract ("only an
        // observation identical in every field... shares this id"), reachable via the public
        // `EvidenceId::new` constructor alone, with no extractor-specific assumption needed.
        let mut header_a = symbol_header(SemanticDimension::Symbol);
        header_a.evidence_refs = vec![crate::identity::EvidenceId::new("ev,idence")];
        let mut header_b = header_a.clone();
        header_b.evidence_refs = vec![
            crate::identity::EvidenceId::new("ev"),
            crate::identity::EvidenceId::new("idence"),
        ];

        let a = SemanticObservation::Symbol(header_a);
        let b = SemanticObservation::Symbol(header_b);

        assert_ne!(
            a.raw_observation_id(),
            b.raw_observation_id(),
            "one evidence ref containing a comma must never collide with two refs split at that \
             same character"
        );
    }

    #[test]
    fn evidence_refs_accessor_matches_the_wrapped_header() {
        let mut header = symbol_header(SemanticDimension::Symbol);
        header.evidence_refs = vec![crate::identity::EvidenceId::new("evidence:known")];
        let observation = SemanticObservation::Symbol(header.clone());
        assert_eq!(observation.evidence_refs(), header.evidence_refs.as_slice());
    }
}
