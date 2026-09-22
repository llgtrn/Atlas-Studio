//! Function identity and signature.
//!
//! Function identity is never based on display name alone; disambiguation across overloads,
//! trait/impl methods and generics is `.atlas/contracts/SEMANTIC-EXTRACTION.md` R4.4 closure work.
//! This kernel materializes the typed shape; deeper disambiguation rules land with the Rust
//! extractor.

use super::SemanticScope;
use super::symbol::SymbolIdentity;
use super::types::TypeIdentity;
use crate::identity::RepositoryId;
use crate::language::adl::SourceSpan;
use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

/// Stable function/method identity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionIdentity {
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub language: String,
    pub scope: SemanticScope,
    pub symbol: SymbolIdentity,
    pub span: SourceSpan,
    pub generated: bool,
}

impl FunctionIdentity {
    /// Deterministic, order-independent encoding of this function's identity fields.
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}:{}|{}|{}|{}|{}:{}:{}|{}",
            self.repository.as_str(),
            self.revision.kind,
            self.revision.value,
            self.language,
            self.scope.join(),
            self.symbol.identity_key(),
            self.span.path,
            self.span.line,
            self.span.column,
            self.generated,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionParameter {
    pub name: String,
    pub type_identity: TypeIdentity,
}

/// Typed function signature facet; see `.atlas/contracts/SEMANTIC-FACTS.md#functionsignature`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionSignature {
    pub function: FunctionIdentity,
    pub parameters: Vec<FunctionParameter>,
    pub return_type: Option<TypeIdentity>,
    pub generics: Vec<String>,
    pub abi: Option<String>,
    pub visibility: String,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub is_extern: bool,
}

#[cfg(test)]
mod tests {
    use super::super::symbol::SymbolRole;
    use super::*;

    fn identity(scope_segment: &str) -> FunctionIdentity {
        let repository = RepositoryId::new("atlas-studio");
        let revision = RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let scope = SemanticScope::new(["core", scope_segment]);
        FunctionIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            language: "rust".into(),
            scope: scope.clone(),
            symbol: SymbolIdentity {
                repository,
                revision,
                scope,
                name: "run".into(),
                role: SymbolRole::Definition,
            },
            span: SourceSpan {
                path: "core/src/lib.rs".into(),
                line: 10,
                column: 1,
            },
            generated: false,
        }
    }

    #[test]
    fn same_name_functions_in_separate_scopes_have_distinct_identity() {
        let a = identity("widgets");
        let b = identity("engine");
        assert_ne!(a.identity_key(), b.identity_key());
    }

    #[test]
    fn different_revision_changes_identity() {
        let a = identity("widgets");
        let mut b = identity("widgets");
        b.revision = RevisionRef {
            kind: "git".into(),
            value: "def456".into(),
        };
        b.symbol.revision = b.revision.clone();
        assert_ne!(a.identity_key(), b.identity_key());
    }

    #[test]
    fn generated_origin_changes_identity() {
        let a = identity("widgets");
        let b = FunctionIdentity {
            generated: true,
            ..a.clone()
        };
        assert_ne!(a.identity_key(), b.identity_key());
    }
}
