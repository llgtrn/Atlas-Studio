//! Function identity and signature.
//!
//! Function identity is never based on display name alone. R4.4 (`.atlas/contracts/
//! SEMANTIC-EXTRACTION.md`) closes declaration-identity ambiguity across free functions, nested
//! modules, inherent/trait methods, trait declarations/defaults/implementations, associated
//! functions and generic declarations, so a later CALL relation can target one stable,
//! scope-correct `FunctionIdentity` rather than a matching name. This kernel materializes the
//! typed shape; the Rust extractor (`adapter/src/semantic/rust`) is the only producer of real
//! values.
//!
//! What source syntax alone can prove, and what it cannot: `declaration_kind`/`owner` are always
//! derived from what `syn` literally parsed (an `impl` block's self type and optional trait path,
//! whether a trait method has a default body, whether a method has a receiver) -- never from
//! compiler name resolution. `owner.target`/`owner.trait_path` are source-level spellings, exactly
//! like `TypeIdentity.name`; they never claim a globally-resolved compiler `DefId`, and never
//! assert that a trait declaration and its implementation are the *same* canonical declaration
//! (`.atlas/contracts/NORMALIZATION.md`: identity resolution precedes dedup, and forbids "same
//! display name" as an equivalence proof). Whether `impl Reader for Foo::read` canonically
//! implements `trait Reader::read` remains a reconciliation-owned relation, not an identity claim.

use super::SemanticScope;
use super::symbol::SymbolIdentity;
use super::types::TypeIdentity;
use crate::identity::RepositoryId;
use crate::language::adl::SourceSpan;
use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

/// Which declaration shape produced a `FunctionIdentity`, as literally observable from source
/// syntax alone. Kept as an explicit typed enum -- never inferred later from a scope/display
/// string -- so a later CALL relation (R4.5+) can match on it without re-parsing anything.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FunctionDeclarationKind {
    /// A top-level or nested-module function: not inside any `impl`/`trait` block.
    FreeFunction,
    /// A function inside an inherent `impl Type { ... }` block WITH a `self`/`&self`/`&mut self`
    /// receiver.
    InherentMethod,
    /// A function inside an inherent `impl Type { ... }` block with NO receiver (e.g.
    /// `Type::new()`).
    AssociatedFunction,
    /// A method declared inside a `trait Name { ... }` block with no body (`fn read(&self);`).
    TraitMethodDeclaration,
    /// A method declared inside a `trait Name { ... }` block WITH a default body.
    TraitDefaultMethod,
    /// A method inside an `impl Trait for Type { ... }` block, receiver or not.
    TraitImplementationMethod,
}

impl FunctionDeclarationKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::FreeFunction => "FREE_FUNCTION",
            Self::InherentMethod => "INHERENT_METHOD",
            Self::AssociatedFunction => "ASSOCIATED_FUNCTION",
            Self::TraitMethodDeclaration => "TRAIT_METHOD_DECLARATION",
            Self::TraitDefaultMethod => "TRAIT_DEFAULT_METHOD",
            Self::TraitImplementationMethod => "TRAIT_IMPLEMENTATION_METHOD",
        }
    }
}

/// Syntactically-observed declaration ownership for a method/associated function: who source
/// syntax says this function is declared "inside". Never a globally-resolved compiler entity --
/// see this module's doc comment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionOwner {
    /// The enclosing `impl` block's self type (e.g. `Foo`, `Bar<T>`), present whenever this
    /// function is declared inside an `impl` block (inherent or trait impl). `None` for a
    /// `FreeFunction`, `TraitMethodDeclaration` or `TraitDefaultMethod` -- those have no `impl`
    /// target; the trait itself is named via `trait_path` instead.
    pub target: Option<TypeIdentity>,
    /// The trait path syntactically named by `impl <trait_path> for <target>`, or by the
    /// enclosing `trait <trait_path> { ... }`. `None` for a `FreeFunction`, `InherentMethod` or
    /// `AssociatedFunction`.
    pub trait_path: Option<String>,
}

impl FunctionOwner {
    /// No enclosing `impl`/`trait` -- a free function.
    pub const fn none() -> Self {
        Self {
            target: None,
            trait_path: None,
        }
    }

    fn identity_key(&self) -> String {
        format!(
            "target={}|trait={}",
            self.target
                .as_ref()
                .map(TypeIdentity::identity_key)
                .unwrap_or_default(),
            self.trait_path.as_deref().unwrap_or(""),
        )
    }
}

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
    /// Which declaration shape this is (free function, inherent/trait method, ...). See
    /// `FunctionDeclarationKind`.
    pub declaration_kind: FunctionDeclarationKind,
    /// Syntactically-observed enclosing `impl`/`trait` context. See `FunctionOwner`.
    pub owner: FunctionOwner,
    /// This function's own declared generic parameters (e.g. `["T", "U: Clone"]`), in source
    /// declaration order -- never a monomorphized instance identity (`convert::<u32>` is R4.5+
    /// territory, not modeled here).
    pub generics: Vec<String>,
}

impl FunctionIdentity {
    /// Deterministic, order-independent encoding of this function's identity fields.
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}:{}|{}|{}|{}|{}:{}:{}|{}|kind={}|{}|generics=[{}]",
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
            self.declaration_kind.as_str(),
            self.owner.identity_key(),
            self.generics.join(","),
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

impl FunctionSignature {
    /// A source-spelling display summary (`"async unsafe fn(x: u8) -> Result<T, E>"`), never a
    /// canonical/compiler-resolved signature -- built purely from each parameter/return type's own
    /// `TypeIdentity.name`, which is exactly the source-syntax spelling this whole kernel commits to
    /// (see this module's own doc comment). Shared canonical form for every caller that needs a
    /// display/summary string for a signature -- previously duplicated, byte-for-byte identically,
    /// as a private free function in both `core::graph::engineering_graph` and `runtime::census`.
    pub fn summary(&self) -> String {
        let params = self
            .parameters
            .iter()
            .map(|parameter| format!("{}: {}", parameter.name, parameter.type_identity.name))
            .collect::<Vec<_>>()
            .join(", ");
        let return_type = self
            .return_type
            .as_ref()
            .map(|type_identity| type_identity.name.clone())
            .unwrap_or_else(|| "()".to_owned());
        let asyncness = if self.is_async { "async " } else { "" };
        let unsafety = if self.is_unsafe { "unsafe " } else { "" };
        format!("{asyncness}{unsafety}fn({params}) -> {return_type}")
    }
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
            declaration_kind: FunctionDeclarationKind::FreeFunction,
            owner: FunctionOwner::none(),
            generics: Vec::new(),
        }
    }

    #[test]
    fn summary_renders_asyncness_unsafety_parameters_and_return_type() {
        let signature = FunctionSignature {
            function: identity("widgets"),
            parameters: vec![
                FunctionParameter {
                    name: "x".into(),
                    type_identity: type_identity("Vec<T>"),
                },
                FunctionParameter {
                    name: "y".into(),
                    type_identity: type_identity("&mut usize"),
                },
            ],
            return_type: Some(type_identity("Result<T, Error>")),
            generics: Vec::new(),
            abi: None,
            visibility: "pub".into(),
            is_async: true,
            is_unsafe: true,
            is_extern: false,
        };
        assert_eq!(
            signature.summary(),
            "async unsafe fn(x: Vec<T>, y: &mut usize) -> Result<T, Error>"
        );
    }

    #[test]
    fn summary_renders_unit_return_and_no_modifiers_when_absent() {
        let signature = FunctionSignature {
            function: identity("widgets"),
            parameters: Vec::new(),
            return_type: None,
            generics: Vec::new(),
            abi: None,
            visibility: "pub".into(),
            is_async: false,
            is_unsafe: false,
            is_extern: false,
        };
        assert_eq!(signature.summary(), "fn() -> ()");
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

    // --- R4.4: declaration_kind/owner/generics participate in identity_key ---------------------

    #[test]
    fn declaration_kind_alone_changes_identity() {
        let a = identity("widgets");
        let b = FunctionIdentity {
            declaration_kind: FunctionDeclarationKind::AssociatedFunction,
            ..a.clone()
        };
        assert_ne!(a.identity_key(), b.identity_key());
    }

    fn type_identity(name: &str) -> TypeIdentity {
        TypeIdentity {
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            scope: SemanticScope::new(Vec::<String>::new()),
            name: name.into(),
            canonical: None,
        }
    }

    #[test]
    fn owner_target_alone_distinguishes_same_named_methods_on_different_impl_targets() {
        let base = identity("widgets");
        let foo = FunctionIdentity {
            declaration_kind: FunctionDeclarationKind::InherentMethod,
            owner: FunctionOwner {
                target: Some(type_identity("Foo")),
                trait_path: None,
            },
            ..base.clone()
        };
        let bar = FunctionIdentity {
            owner: FunctionOwner {
                target: Some(type_identity("Bar")),
                trait_path: None,
            },
            ..foo.clone()
        };
        assert_ne!(foo.identity_key(), bar.identity_key());
    }

    #[test]
    fn owner_trait_path_alone_distinguishes_inherent_from_trait_impl_method() {
        let base = identity("widgets");
        let inherent = FunctionIdentity {
            declaration_kind: FunctionDeclarationKind::InherentMethod,
            owner: FunctionOwner {
                target: Some(type_identity("Foo")),
                trait_path: None,
            },
            ..base.clone()
        };
        let trait_impl = FunctionIdentity {
            declaration_kind: FunctionDeclarationKind::TraitImplementationMethod,
            owner: FunctionOwner {
                target: Some(type_identity("Foo")),
                trait_path: Some("Reader".into()),
            },
            ..inherent.clone()
        };
        assert_ne!(inherent.identity_key(), trait_impl.identity_key());
    }

    #[test]
    fn owner_trait_path_alone_distinguishes_two_traits_on_the_same_target() {
        let base = identity("widgets");
        let reader = FunctionIdentity {
            declaration_kind: FunctionDeclarationKind::TraitImplementationMethod,
            owner: FunctionOwner {
                target: Some(type_identity("Foo")),
                trait_path: Some("Reader".into()),
            },
            ..base.clone()
        };
        let other_reader = FunctionIdentity {
            owner: FunctionOwner {
                target: Some(type_identity("Foo")),
                trait_path: Some("OtherReader".into()),
            },
            ..reader.clone()
        };
        assert_ne!(reader.identity_key(), other_reader.identity_key());
    }

    #[test]
    fn generics_participate_in_identity() {
        let a = identity("widgets");
        let single = FunctionIdentity {
            generics: vec!["T".into()],
            ..a.clone()
        };
        let double = FunctionIdentity {
            generics: vec!["T".into(), "U".into()],
            ..a.clone()
        };
        assert_ne!(a.identity_key(), single.identity_key());
        assert_ne!(single.identity_key(), double.identity_key());
    }

    #[test]
    fn identical_fields_produce_identical_identity() {
        let a = identity("widgets");
        let b = identity("widgets");
        assert_eq!(a.identity_key(), b.identity_key());
    }
}
