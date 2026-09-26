//! Data-flow value identity.
//!
//! R4.7 (`.atlas/contracts/SEMANTIC-FACTS.md#dataflowfact`, `.atlas/roadmap/SELF-BUILDING-R4-R8.md`
//! R4.7): "values; definitions/uses; parameter flow; return flow; load/store relationships; local
//! propagation; typed unresolved/alias ambiguity where deeper analysis is unavailable. Do not claim
//! compiler-complete alias analysis unless actually evidenced."
//!
//! Like R4.6's CONTROL_FLOW (and unlike R4.5's CALL), *which lexical binding a bare identifier use
//! refers to within one function* is fully determined by Rust's own scoping/shadowing rules --
//! syntax alone, no type inference needed -- so `adapter`'s extractor resolves this for real. What
//! it does NOT attempt is anything requiring actual alias/borrow analysis: whether two references
//! (`&x`, `&mut y`) denote overlapping storage, whether a value that crossed a function boundary or
//! a compound place (`self.field`, `arr[i]`, `*ptr`) aliases another, or any points-to reasoning.
//! Those stay `DataFlowResolution::Unresolved`, per the contract's explicit warning against
//! unevidenced compiler-complete alias claims.

use super::SemanticRecordId;
use crate::identity::RepositoryId;
use crate::language::adl::SourceSpan;
use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

/// What kind of data-flow event this record represents. Part of `identity_key()`: a Definition and
/// a Use are different entities even in the (extremely unlikely) case they'd otherwise share a
/// (function, name, span) triple, unlike e.g. R4.6's `ControlFlowBlockKind` (which is descriptive
/// content, since `block_index` alone already disambiguates blocks).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValueRole {
    /// A binding is introduced: a `let PAT = ..` (simple identifier patterns only this wave) or a
    /// function parameter.
    Definition,
    /// A previously-bound name is read.
    Use,
    /// A previously-bound name is the target of a plain assignment (`name = ..`; simple identifier
    /// targets only this wave).
    Store,
}

impl ValueRole {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Definition => "DEFINITION",
            Self::Use => "USE",
            Self::Store => "STORE",
        }
    }
}

/// Whether a `Use`/`Store` event's binding was found by this extractor's local, syntax-only scope
/// tracking. `Definition` events are always `Resolved` (a definition trivially resolves to itself).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DataFlowResolution {
    /// A matching `Definition` (same name, nearest enclosing scope, respecting shadowing) was
    /// found earlier in this same function's local scope tracking.
    Resolved,
    /// No matching local `Definition` was found -- the name may be a field access, a module-level
    /// item, a closure capture from an enclosing function, or genuinely ambiguous. Never guessed;
    /// always explicit (contract: "typed unresolved/alias ambiguity where deeper analysis is
    /// unavailable").
    Unresolved,
}

impl DataFlowResolution {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Resolved => "RESOLVED",
            Self::Unresolved => "UNRESOLVED",
        }
    }
}

/// Identity of one value definition/use/store site within a function.
///
/// `resolution`/`resolved_definition`/`is_parameter`/`is_return_flow` are deliberately NOT part of
/// `identity_key()`, for the same reason R4.5's `CallSiteIdentity` excludes `dispatch`/`callees` and
/// R4.6's `ControlFlowBlockIdentity` excludes `successors`: they are facts ABOUT an
/// already-identified event (this exact name, at this exact span, playing this exact role), not
/// additional identity-disambiguating fields -- so `record_id` stays stable if a later wave
/// resolves a use this extractor could only mark `Unresolved` today.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValueIdentity {
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub function: SemanticRecordId,
    pub name: String,
    pub span: SourceSpan,
    pub role: ValueRole,
    /// `true` only for a `Definition` introduced by a function parameter (as opposed to a `let`
    /// binding).
    pub is_parameter: bool,
    /// `true` only for a `Use` whose value is the function's own return value: an explicit
    /// `return expr;`, or a bare identifier reached via a chain of tail positions starting at the
    /// function body itself (its own tail expression, and transitively the tail of a nested
    /// `if`/`match`/`{ }` block expression that is *itself* in that chain). A block reached as a
    /// `let` initializer, a loop body, or any other non-tail position is never return-flow,
    /// regardless of that block's own last statement's syntactic shape.
    pub is_return_flow: bool,
    pub resolution: DataFlowResolution,
    /// The resolved `Definition`'s own record_id, when `resolution == Resolved`. Always `None` for
    /// a `Definition` event itself and for any `Unresolved` `Use`/`Store`.
    pub resolved_definition: Option<SemanticRecordId>,
    /// G146 (mission M6): for a `Use` of a field chain rooted at this name (`report.census.facts`),
    /// the fields read, outermost first (`["census", "facts"]`); empty when the value is used
    /// whole. Content, not identity: the event is this name at this span, and the projection is
    /// a fact about it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub projection: Vec<String>,
}

impl ValueIdentity {
    /// Deterministic, order-independent encoding of this value event's identity fields.
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}:{}|{}|{}|{}:{}:{}|{}",
            self.repository.as_str(),
            self.revision.kind,
            self.revision.value,
            self.function.as_str(),
            self.name,
            self.span.path,
            self.span.line,
            self.span.column,
            self.role.as_str(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::SemanticDimension;
    use super::*;

    fn base() -> ValueIdentity {
        ValueIdentity {
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "fn-key"),
            name: "x".into(),
            span: SourceSpan {
                path: "core/src/lib.rs".into(),
                line: 10,
                column: 5,
            },
            role: ValueRole::Definition,
            is_parameter: false,
            is_return_flow: false,
            resolution: DataFlowResolution::Resolved,
            resolved_definition: None,
            projection: Vec::new(),
        }
    }

    #[test]
    fn identity_key_distinguishes_name() {
        let other = ValueIdentity {
            name: "y".into(),
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn identity_key_distinguishes_span() {
        let other = ValueIdentity {
            span: SourceSpan {
                line: 11,
                ..base().span
            },
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn identity_key_distinguishes_owning_function() {
        let other = ValueIdentity {
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "other-fn-key"),
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    // --- R4.7: role participates in identity (a Definition and a Use are different entities) ----

    #[test]
    fn identity_key_distinguishes_role() {
        let other = ValueIdentity {
            role: ValueRole::Use,
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    // --- R4.7: resolution/resolved_definition/is_parameter/is_return_flow are content, not identity

    #[test]
    fn identity_key_is_unaffected_by_resolution_fields() {
        let resolved = ValueIdentity {
            role: ValueRole::Use,
            is_parameter: true,
            is_return_flow: true,
            resolution: DataFlowResolution::Unresolved,
            resolved_definition: Some(SemanticRecordId::new(SemanticDimension::DataFlow, "def")),
            projection: vec!["census".into(), "facts".into()],
            ..base()
        };
        let unresolved = ValueIdentity {
            role: ValueRole::Use,
            ..base()
        };
        assert_eq!(resolved.identity_key(), unresolved.identity_key());
    }

    #[test]
    fn role_as_str_matches_the_screaming_snake_vocabulary() {
        assert_eq!(ValueRole::Definition.as_str(), "DEFINITION");
        assert_eq!(ValueRole::Use.as_str(), "USE");
        assert_eq!(ValueRole::Store.as_str(), "STORE");
    }

    #[test]
    fn resolution_as_str_matches_the_screaming_snake_vocabulary() {
        assert_eq!(DataFlowResolution::Resolved.as_str(), "RESOLVED");
        assert_eq!(DataFlowResolution::Unresolved.as_str(), "UNRESOLVED");
    }
}
