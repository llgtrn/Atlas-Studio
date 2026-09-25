//! Typed semantic record kernel (R4.1).
//!
//! This module materializes the shared identity/header primitives that every deep semantic fact
//! family (function, symbol, type, call, control-flow, data-flow, state, effect, ...) builds on.
//! It carries no filesystem, parser or extraction logic; that belongs to `adapter`/`runtime`. See
//! `.atlas/contracts/SEMANTIC-FACTS.md` and `.atlas/contracts/SEMANTIC-EXTRACTION.md`.

pub mod call;
pub mod concurrency;
pub mod control_flow;
pub mod data_flow;
pub mod diagnostic;
pub mod effect;
pub mod function;
pub mod obligation;
pub mod observation;
pub mod ownership;
pub mod persistence;
pub mod place;
pub mod state;
pub mod symbol;
pub mod types;

pub use call::{CallDispatchKind, CallSiteIdentity};
pub use concurrency::{ConcurrencyIdentity, ConcurrencyKind};
pub use control_flow::{
    ControlFlowBlockIdentity, ControlFlowBlockKind, ControlFlowEdge, ControlFlowEdgeKind,
};
pub use data_flow::{DataFlowResolution, ValueIdentity, ValueRole};
pub use diagnostic::{DiagnosticCode, ExtractionDiagnostic};
pub use effect::{EffectCategory, EffectIdentity, std_path_effects};
pub use function::{
    FunctionDeclarationKind, FunctionIdentity, FunctionOwner, FunctionParameter, FunctionSignature,
};
pub use obligation::SemanticObligationRecord;
pub use observation::SemanticObservation;
pub use ownership::{OwnershipIdentity, OwnershipKind, OwnershipResolution};
pub use persistence::{PersistenceIdentity, PersistenceKind, PersistenceResolution};
pub use place::PlaceRef;
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

    /// `name` qualified by this scope (`"core::widgets::run"`), or `name` alone when this scope
    /// has no segments. Shared canonical form for every caller that needs a scoped display/summary
    /// string for a symbol -- previously duplicated, byte-for-byte identically, as a private free
    /// function in both `core::graph::engineering_graph` and `runtime::census`, with nothing
    /// preventing the two copies from silently drifting apart on a future edit to only one.
    pub fn scoped_name(&self, name: &str) -> String {
        if self.segments.is_empty() {
            name.to_owned()
        } else {
            format!("{}::{}", self.join(), name)
        }
    }

    /// Collision-safe encoding of this scope's segments for use inside an `identity_key()`
    /// composite string -- unlike `join()` (a human-readable display form, kept as-is for that
    /// purpose), this escapes each segment before joining so a segment is never confused, after
    /// joining, with a different split of the same characters across more/fewer segments. Needed
    /// because a scope segment is not always a bare module-path identifier: the Rust extractor's
    /// `handle_impl` (`adapter::semantic::rust::mod`) pushes a segment like
    /// `"impl:std::collections::HashMap<K, V> for MyType"` for an `impl` block, built from
    /// unrestricted type/trait spellings that can themselves contain `:` (nested paths, or a
    /// const-generic array-length expression such as `[u8; A | B]` — no restrictive lexer bounds
    /// this text, exactly the field class `escape_identity_field`'s own doc comment names).
    pub fn identity_key(&self) -> String {
        self.segments
            .iter()
            .map(|segment| crate::identity::escape_identity_field(segment, ':'))
            .collect::<Vec<_>>()
            .join("::")
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

    #[test]
    fn scoped_name_is_bare_when_scope_has_no_segments() {
        let scope = SemanticScope::new(Vec::<String>::new());
        assert_eq!(scope.scoped_name("run"), "run");
    }

    #[test]
    fn scoped_name_joins_segments_before_the_name() {
        let scope = SemanticScope::new(["core", "widgets"]);
        assert_eq!(scope.scoped_name("run"), "core::widgets::run");
    }

    #[test]
    fn plain_join_can_collide_across_a_different_segment_split() {
        // Demonstrates the defect `identity_key()` exists to close: `join()` is a bare "::"
        // concatenation with no escaping, so two scopes with genuinely different segment lists
        // (different structural nesting) can render to the identical string when a segment
        // itself contains "::" -- exactly what a real `impl:Trait<Nested::Path> for Type` scope
        // segment can (see `identity_key()`'s own doc comment).
        let a = SemanticScope::new(["a::b", "c"]);
        let b = SemanticScope::new(["a", "b::c"]);
        assert_ne!(a, b, "sanity: these are genuinely different scopes");
        assert_eq!(
            a.join(),
            b.join(),
            "join() is documented as display-only precisely because it can collide like this"
        );
    }

    #[test]
    fn identity_key_does_not_collide_across_a_different_segment_split() {
        let a = SemanticScope::new(["a::b", "c"]);
        let b = SemanticScope::new(["a", "b::c"]);
        assert_ne!(
            a.identity_key(),
            b.identity_key(),
            "escaping each segment before joining must prevent the join() collision: {} == {}",
            a.identity_key(),
            b.identity_key(),
        );
    }

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
                path: String::new(),
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
