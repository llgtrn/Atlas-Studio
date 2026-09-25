//! Symbol identity: declaration/definition/reference/resolution scoping.
//!
//! Name equality alone is never identity equality; see
//! `.atlas/contracts/NORMALIZATION.md#identity`.

use super::SemanticScope;
use crate::identity::RepositoryId;
use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SymbolRole {
    Definition,
    Declaration,
    Reference,
    Unresolved,
}

impl SymbolRole {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Definition => "DEFINITION",
            Self::Declaration => "DECLARATION",
            Self::Reference => "REFERENCE",
            Self::Unresolved => "UNRESOLVED",
        }
    }
}

/// Scope-aware symbol identity covering definition/declaration/reference/unresolved cases.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolIdentity {
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub scope: SemanticScope,
    pub name: String,
    pub role: SymbolRole,
    /// The source artifact that spells this symbol (G67, Kythe's VName `path`). The lexical scope
    /// carries no module, so without it two `tests::report` definitions in different files had
    /// one identity and one of them vanished from the graph.
    #[serde(default)]
    pub path: String,
}

impl SymbolIdentity {
    /// Deterministic, order-independent encoding of this symbol's identity fields. `scope` is
    /// escaped before joining -- see `SemanticScope::identity_key()`'s doc comment.
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}:{}|{}|{}|{}|{}",
            self.repository.as_str(),
            self.revision.kind,
            self.revision.value,
            self.scope.identity_key(),
            self.name,
            self.role.as_str(),
            crate::identity::escape_identity_field(&self.path, '|'),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> RepositoryId {
        RepositoryId::new("atlas-studio")
    }

    fn revision() -> RevisionRef {
        RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        }
    }

    /// G67: the same scope, name and role spelled in two files are two symbols (Kythe's VName
    /// path). Before, both `tests::report` definitions shared one identity.
    #[test]
    fn identity_key_distinguishes_the_source_artifact() {
        let symbol = |path: &str| SymbolIdentity {
            path: path.into(),
            repository: repo(),
            revision: revision(),
            scope: SemanticScope::new(["tests"]),
            name: "report".into(),
            role: SymbolRole::Definition,
        };
        assert_ne!(
            symbol("core/src/visual/mod.rs").identity_key(),
            symbol("core/src/census/delta.rs").identity_key()
        );
        assert_eq!(
            symbol("core/src/a.rs").identity_key(),
            symbol("core/src/a.rs").identity_key()
        );
        // The path is escaped like every other field: a `|` in it cannot forge another key.
        assert_ne!(symbol("a|b").identity_key(), symbol("a").identity_key());
    }

    #[test]
    fn identity_key_distinguishes_scope() {
        let a = SymbolIdentity {
            path: String::new(),
            repository: repo(),
            revision: revision(),
            scope: SemanticScope::new(["core", "widgets"]),
            name: "run".into(),
            role: SymbolRole::Definition,
        };
        let b = SymbolIdentity {
            scope: SemanticScope::new(["core", "engine"]),
            ..a.clone()
        };
        assert_ne!(a.identity_key(), b.identity_key());
    }

    #[test]
    fn identity_key_distinguishes_role() {
        let a = SymbolIdentity {
            path: String::new(),
            repository: repo(),
            revision: revision(),
            scope: SemanticScope::new(["core"]),
            name: "run".into(),
            role: SymbolRole::Definition,
        };
        let b = SymbolIdentity {
            role: SymbolRole::Reference,
            ..a.clone()
        };
        assert_ne!(a.identity_key(), b.identity_key());
    }

    #[test]
    fn identity_key_is_stable_for_equal_values() {
        let a = SymbolIdentity {
            path: String::new(),
            repository: repo(),
            revision: revision(),
            scope: SemanticScope::new(["core"]),
            name: "run".into(),
            role: SymbolRole::Definition,
        };
        let b = a.clone();
        assert_eq!(a.identity_key(), b.identity_key());
    }
}
