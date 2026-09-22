//! Stateful entity identity (field, resource, global).

use super::SemanticScope;
use crate::identity::RepositoryId;
use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

/// Identity of one stateful entity a function may read/write/transition/create/delete.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateIdentity {
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub scope: SemanticScope,
    pub name: String,
}

impl StateIdentity {
    /// Deterministic, order-independent encoding of this state entity's identity fields.
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}:{}|{}|{}",
            self.repository.as_str(),
            self.revision.kind,
            self.revision.value,
            self.scope.join(),
            self.name,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> StateIdentity {
        StateIdentity {
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            scope: SemanticScope::new(["core", "widgets"]),
            name: "counter".into(),
        }
    }

    #[test]
    fn identity_key_distinguishes_scope() {
        let other = StateIdentity {
            scope: SemanticScope::new(["core", "engine"]),
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }
}
