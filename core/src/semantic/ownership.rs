//! Ownership/borrow/move operation-site identity.
//!
//! R4.9 (`.atlas/contracts/SEMANTIC-FACTS.md#ownershipfact`, `.atlas/roadmap/SELF-BUILDING-R4-R8.md`
//! R4.9): "borrow/move/copy behavior where evidenced; allocation/free/resource acquisition/
//! release; resource ownership transfer; lifetime/region facts only to the level supported by
//! admitted evidence; explicit unresolved ownership where compiler-grade analysis is absent. Do
//! not claim rustc-equivalent borrow checking merely because ownership facts exist."
//!
//! Scope this wave (see `adapter::semantic::rust::ownership` for the extractor): reuses R4.6/R4.7's
//! key insight -- `&expr`/`&mut expr` is fully syntax-determined (Rust's borrow syntax is
//! unambiguous regardless of the referent's type), so `BorrowShared`/`BorrowMut` sites are real,
//! precise observations, not guesses. Whether a bare identifier used by value is actually MOVED
//! (its type is not `Copy`) or merely COPIED (its type IS `Copy`) genuinely requires type
//! resolution this extractor does not have -- exactly the same epistemic gap R4.5's CALL dispatch
//! has for callee resolution -- so this wave deliberately does not attempt to distinguish them:
//! `OwnershipKind::MoveOrCopy` names the ambiguity explicitly rather than guessing either way.
//! `Acquire`/`Release`/`Transfer`/`Escape` are declared for a future wave (resource acquisition/
//! release requires resolving an overloaded call to a specific known API -- the same problem
//! R4.8's EFFECT explicitly declined to solve without fabricating semantics) but never emitted by
//! this extractor this wave.

use super::SemanticRecordId;
use crate::identity::RepositoryId;
use crate::language::adl::SourceSpan;
use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

/// The ownership operation this record represents. Part of `identity_key()`: a shared borrow and
/// a mutable borrow of the same place at the same span are different entities, mirroring R4.7's
/// `ValueRole`/R4.8's `StateAccessKind` precedent.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OwnershipKind {
    /// `&expr` -- a shared reference is taken. Fully syntax-determined.
    BorrowShared,
    /// `&mut expr` -- a mutable reference is taken. Fully syntax-determined.
    BorrowMut,
    /// A bare identifier is used by value at a position that would move or copy it in real Rust
    /// (a call argument, a `let` initializer, an assignment RHS, a `return` value, or the
    /// function's own implicit tail-expression return) -- syntax alone cannot say which, since
    /// that depends on whether the identifier's type implements `Copy`.
    MoveOrCopy,
    /// Reserved; never emitted this wave.
    Acquire,
    /// Reserved; never emitted this wave.
    Release,
    /// Reserved; never emitted this wave.
    Transfer,
    /// Reserved; never emitted this wave.
    Escape,
}

impl OwnershipKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::BorrowShared => "BORROW_SHARED",
            Self::BorrowMut => "BORROW_MUT",
            Self::MoveOrCopy => "MOVE_OR_COPY",
            Self::Acquire => "ACQUIRE",
            Self::Release => "RELEASE",
            Self::Transfer => "TRANSFER",
            Self::Escape => "ESCAPE",
        }
    }
}

/// Whether `name` identifies a real, reusable place (a bare local/parameter identifier -- the same
/// binding really is targeted by every operation sharing that name in that function) or is merely
/// the textual spelling of an unresolved temporary expression (e.g. `foo()` inside `&foo()`), where
/// two operations with identical spelling are NOT the same value merely because they read alike.
/// Mirrors R4.8's `StateResolution` precedent.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OwnershipResolution {
    /// `name` is a bare identifier: a real place, safe to converge multiple operations onto.
    Resolved,
    /// `name` is only the spelling of a temporary expression: never converge on it by name alone.
    Unresolved,
}

impl OwnershipResolution {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Resolved => "RESOLVED",
            Self::Unresolved => "UNRESOLVED",
        }
    }
}

/// Identity of one ownership-relevant operation site: `kind` performed against the place/value
/// named `name`, attributed to the accessing `function`, at `span`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnershipIdentity {
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub function: SemanticRecordId,
    pub name: String,
    pub span: SourceSpan,
    pub kind: OwnershipKind,
    pub resolution: OwnershipResolution,
}

impl OwnershipIdentity {
    /// Deterministic, order-independent encoding of this ownership operation's identity fields.
    /// `resolution` is deliberately excluded -- like R4.8's `StateResolution` precedent, it is a
    /// resolution fact about an already-identified operation, not part of what makes the operation
    /// itself a distinct entity.
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
            self.kind.as_str(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::SemanticDimension;
    use super::*;

    fn base() -> OwnershipIdentity {
        OwnershipIdentity {
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "fn-key"),
            name: "x".into(),
            span: SourceSpan {
                path: "src/lib.rs".into(),
                line: 10,
                column: 5,
            },
            kind: OwnershipKind::BorrowShared,
            resolution: OwnershipResolution::Resolved,
        }
    }

    #[test]
    fn identity_key_distinguishes_name() {
        let other = OwnershipIdentity {
            name: "y".into(),
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn identity_key_distinguishes_owning_function() {
        let other = OwnershipIdentity {
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "other-fn-key"),
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn identity_key_distinguishes_span() {
        let other = OwnershipIdentity {
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
        let other = OwnershipIdentity {
            kind: OwnershipKind::BorrowMut,
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn identity_key_is_unaffected_by_resolution() {
        let other = OwnershipIdentity {
            resolution: OwnershipResolution::Unresolved,
            ..base()
        };
        assert_eq!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn resolution_as_str_matches_the_screaming_snake_vocabulary() {
        assert_eq!(OwnershipResolution::Resolved.as_str(), "RESOLVED");
        assert_eq!(OwnershipResolution::Unresolved.as_str(), "UNRESOLVED");
    }

    #[test]
    fn kind_as_str_matches_the_screaming_snake_vocabulary() {
        assert_eq!(OwnershipKind::BorrowShared.as_str(), "BORROW_SHARED");
        assert_eq!(OwnershipKind::BorrowMut.as_str(), "BORROW_MUT");
        assert_eq!(OwnershipKind::MoveOrCopy.as_str(), "MOVE_OR_COPY");
        assert_eq!(OwnershipKind::Acquire.as_str(), "ACQUIRE");
        assert_eq!(OwnershipKind::Release.as_str(), "RELEASE");
        assert_eq!(OwnershipKind::Transfer.as_str(), "TRANSFER");
        assert_eq!(OwnershipKind::Escape.as_str(), "ESCAPE");
    }
}
