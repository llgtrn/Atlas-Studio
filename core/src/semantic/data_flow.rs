//! Data-flow value identity.

use super::SemanticRecordId;
use crate::identity::RepositoryId;
use crate::language::adl::SourceSpan;
use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

/// Identity of one value definition/use site within a function.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValueIdentity {
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub function: SemanticRecordId,
    pub name: String,
    pub span: SourceSpan,
}

impl ValueIdentity {
    /// Deterministic, order-independent encoding of this value's identity fields.
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}:{}|{}|{}|{}:{}:{}",
            self.repository.as_str(),
            self.revision.kind,
            self.revision.value,
            self.function.as_str(),
            self.name,
            self.span.path,
            self.span.line,
            self.span.column,
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
}
