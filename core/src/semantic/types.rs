//! Canonical type identity.

use super::SemanticScope;
use crate::identity::RepositoryId;
use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

/// Canonical type identity. `canonical` carries compiler-provided identity as evidence; it never
/// grants truth by itself (see `.atlas/contracts/SEMANTIC-FACTS.md#symbolfact`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TypeIdentity {
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub scope: SemanticScope,
    pub name: String,
    pub canonical: Option<String>,
}

impl TypeIdentity {
    /// Deterministic, order-independent encoding of this type's identity fields. `scope` and
    /// `name` are escaped before joining: `name` is a source-syntax type spelling
    /// (`adapter::semantic::rust::spelling::type_spelling`), not restricted to a bare identifier --
    /// it can contain `|` (e.g. a const-generic array-length expression such as `[u8; A | B]`, or
    /// a raw token-stream fallback for any type form the spelling function doesn't special-case),
    /// so it isn't provably free of the delimiter this format uses. See
    /// `SemanticScope::identity_key()`'s doc comment for the matching `scope` reasoning.
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}:{}|{}|{}|{}",
            self.repository.as_str(),
            self.revision.kind,
            self.revision.value,
            self.scope.identity_key(),
            crate::identity::escape_identity_field(&self.name, '|'),
            self.canonical.as_deref().unwrap_or(""),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> TypeIdentity {
        TypeIdentity {
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            scope: SemanticScope::new(["core"]),
            name: "Result".into(),
            canonical: None,
        }
    }

    #[test]
    fn identity_key_distinguishes_canonical_identity() {
        let resolved = TypeIdentity {
            canonical: Some("core::result::Result".into()),
            ..base()
        };
        assert_ne!(base().identity_key(), resolved.identity_key());
    }

    #[test]
    fn identity_key_does_not_collide_when_name_contains_the_join_separator() {
        // `name` is a source-syntax type spelling, not always a bare identifier: a const-generic
        // array-length expression such as `[u8; A | B]` is syntactically valid and produces a
        // `name` containing `|`, the same character this format uses as its own field delimiter.
        let a = TypeIdentity {
            name: "x".into(),
            canonical: Some("y|CANON".into()),
            ..base()
        };
        let b = TypeIdentity {
            name: "x|y".into(),
            canonical: Some("CANON".into()),
            ..base()
        };
        assert_ne!(a, b, "sanity: genuinely different TypeIdentity values");
        assert_ne!(
            a.identity_key(),
            b.identity_key(),
            "an unescaped `|`-join let two distinct types collapse onto one identity: {} == {}",
            a.identity_key(),
            b.identity_key(),
        );
    }

    #[test]
    fn identity_key_distinguishes_scope() {
        let other_scope = TypeIdentity {
            scope: SemanticScope::new(["adapter"]),
            ..base()
        };
        assert_ne!(base().identity_key(), other_scope.identity_key());
    }
}
