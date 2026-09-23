//! Stateful entity access identity (field/static reads, writes and, where evidenced,
//! transitions/creation/deletion).
//!
//! R4.8 (`.atlas/contracts/SEMANTIC-FACTS.md#stateaccessfact`, `.atlas/roadmap/SELF-BUILDING-R4-R8.md`
//! R4.8): "state identities; reads; writes; transitions; ... explicit unknown/dynamic behavior."
//! `StateAccessFact` "Represents READ, WRITE, TRANSITION, CREATE or DELETE against a typed state
//! identity, with operation site, alias/resolution status and transaction/authority boundary when
//! known."
//!
//! Scope this wave: single-level self.<field> READ/WRITE is materialized. Compound assignment is
//! explicitly read-modify-write and therefore produces both READ and WRITE records at one site.
//! TRANSITION/CREATE/DELETE, module-level statics, deeper projection identity, closures/async
//! attribution and other unresolved forms remain open; the Rust extractor consequently keeps the
//! overall STATE obligation UNKNOWN rather than fabricating verified absence.

use super::SemanticRecordId;
use crate::identity::RepositoryId;
use crate::language::adl::SourceSpan;
use crate::semantic::SemanticScope;
use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

/// The operation performed against a state entity. Part of `identity_key()`: a Read and a Write
/// are different entities even in the unlikely case they'd otherwise share a (function, entity,
/// span) triple, matching R4.7's `ValueRole` precedent.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StateAccessKind {
    Read,
    Write,
    Transition,
    Create,
    Delete,
}

impl StateAccessKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "READ",
            Self::Write => "WRITE",
            Self::Transition => "TRANSITION",
            Self::Create => "CREATE",
            Self::Delete => "DELETE",
        }
    }
}

/// Whether the accessed entity was identified with confidence by this extractor's syntax-only
/// resolution (a `self.field` on a known owner type, or a bare name matching a `static` declared
/// in the same source unit) or is left explicit rather than guessed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StateResolution {
    Resolved,
    Unresolved,
}

impl StateResolution {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Resolved => "RESOLVED",
            Self::Unresolved => "UNRESOLVED",
        }
    }
}

/// Identity of one state-access event: `kind` performed against the entity named `name` (scoped by
/// `scope`, e.g. `impl:Owner` for a field, or the module path for a `static`), attributed to the
/// accessing `function`, at `span`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateAccessIdentity {
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub function: SemanticRecordId,
    pub scope: SemanticScope,
    pub name: String,
    pub span: SourceSpan,
    pub kind: StateAccessKind,
    pub resolution: StateResolution,
}

impl StateAccessIdentity {
    /// Deterministic, order-independent encoding of this state access's identity fields.
    /// `resolution` is deliberately excluded -- like R4.7's `DataFlowResolution`, it is a
    /// resolution fact about an already-identified access event, not part of what makes the event
    /// itself a distinct entity.
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}:{}|{}|{}|{}|{}:{}:{}|{}",
            self.repository.as_str(),
            self.revision.kind,
            self.revision.value,
            self.function.as_str(),
            self.scope.join(),
            self.name,
            self.span.path,
            self.span.line,
            self.span.column,
            self.kind.as_str(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::SemanticDimension;
    use super::*;

    fn base() -> StateAccessIdentity {
        StateAccessIdentity {
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "fn-key"),
            scope: SemanticScope::new(["impl:Widget"]),
            name: "counter".into(),
            span: SourceSpan {
                path: "src/lib.rs".into(),
                line: 10,
                column: 5,
            },
            kind: StateAccessKind::Read,
            resolution: StateResolution::Resolved,
        }
    }

    #[test]
    fn identity_key_distinguishes_scope() {
        let other = StateAccessIdentity {
            scope: SemanticScope::new(["impl:Other"]),
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn identity_key_distinguishes_name() {
        let other = StateAccessIdentity {
            name: "other_field".into(),
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn identity_key_distinguishes_owning_function() {
        let other = StateAccessIdentity {
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "other-fn-key"),
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn identity_key_distinguishes_span() {
        let other = StateAccessIdentity {
            span: SourceSpan {
                path: "src/lib.rs".into(),
                line: 99,
                column: 5,
            },
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn identity_key_distinguishes_kind() {
        let other = StateAccessIdentity {
            kind: StateAccessKind::Write,
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn identity_key_is_unaffected_by_resolution() {
        let other = StateAccessIdentity {
            resolution: StateResolution::Unresolved,
            ..base()
        };
        assert_eq!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn kind_as_str_matches_the_screaming_snake_vocabulary() {
        assert_eq!(StateAccessKind::Read.as_str(), "READ");
        assert_eq!(StateAccessKind::Write.as_str(), "WRITE");
        assert_eq!(StateAccessKind::Transition.as_str(), "TRANSITION");
        assert_eq!(StateAccessKind::Create.as_str(), "CREATE");
        assert_eq!(StateAccessKind::Delete.as_str(), "DELETE");
    }

    #[test]
    fn resolution_as_str_matches_the_screaming_snake_vocabulary() {
        assert_eq!(StateResolution::Resolved.as_str(), "RESOLVED");
        assert_eq!(StateResolution::Unresolved.as_str(), "UNRESOLVED");
    }
}
