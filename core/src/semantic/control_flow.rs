//! Control-flow block identity.

use super::SemanticRecordId;
use crate::identity::RepositoryId;
use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

/// Identity of one CFG block, stable for the pinned semantic input (see
/// `.atlas/contracts/SEMANTIC-FACTS.md#controlflowfact`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ControlFlowBlockIdentity {
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub function: SemanticRecordId,
    pub block_index: usize,
}

impl ControlFlowBlockIdentity {
    /// Deterministic, order-independent encoding of this block's identity fields.
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}:{}|{}|{}",
            self.repository.as_str(),
            self.revision.kind,
            self.revision.value,
            self.function.as_str(),
            self.block_index,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::SemanticDimension;
    use super::*;

    fn base() -> ControlFlowBlockIdentity {
        ControlFlowBlockIdentity {
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "fn-key"),
            block_index: 0,
        }
    }

    #[test]
    fn identity_key_distinguishes_block_index() {
        let other = ControlFlowBlockIdentity {
            block_index: 1,
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }
}
