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
    /// Deterministic, order-independent encoding of this type's identity fields.
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}:{}|{}|{}|{}",
            self.repository.as_str(),
            self.revision.kind,
            self.revision.value,
            self.scope.join(),
            self.name,
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
    fn identity_key_distinguishes_scope() {
        let other_scope = TypeIdentity {
            scope: SemanticScope::new(["adapter"]),
            ..base()
        };
        assert_ne!(base().identity_key(), other_scope.identity_key());
    }
}
