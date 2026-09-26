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
    /// G133 (NA-CLOSURE-REGIONS): a closure expression -- its own executable region, run later
    /// (possibly never, possibly by another caller) than the function that defines it. Named
    /// `{closure@line:column}` and scoped under `fn <enclosing region>`.
    Closure,
    /// G154 (replay R3, tree-sitter): a function declared in an `extern "ABI" { ... }` block --
    /// implemented outside the census, in another language or object. Its signature (with the
    /// block's ABI) is observed; it has no body here, and a call to it crosses a language
    /// boundary.
    ForeignFunction,
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
            Self::Closure => "CLOSURE",
            Self::ForeignFunction => "FOREIGN_FUNCTION",
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
        // `trait_path` is a source-syntax path spelling (`path_spelling`), not restricted to a
        // bare identifier -- it can carry generic arguments and so, like `TypeIdentity.name`,
        // isn't provably free of the `|` this format uses as a delimiter.
        format!(
            "target={}|trait={}",
            self.target
                .as_ref()
                .map(TypeIdentity::identity_key)
                .unwrap_or_default(),
            crate::identity::escape_identity_field(self.trait_path.as_deref().unwrap_or(""), '|'),
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
    ///
    /// `scope` and `generics` are escaped before joining: a scope segment is not always a bare
    /// module-path identifier (`adapter::semantic::rust`'s `handle_impl` pushes a segment like
    /// `"impl:Trait<Nested::Path> for Type"`), and a generic param's spelling is not always a bare
    /// identifier either (`syn`'s bound rendering falls back to raw token-stream text for a bound
    /// like `From<(A, B)>`) -- both can contain the literal character their own join uses as a
    /// delimiter, which would otherwise let two structurally different functions collapse onto one
    /// identity. See `SemanticScope::identity_key()`'s doc comment for the same reasoning.
    pub fn identity_key(&self) -> String {
        let generics = self
            .generics
            .iter()
            .map(|generic| crate::identity::escape_identity_field(generic, ','))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{}|{}:{}|{}|{}|{}|{}:{}:{}|{}|kind={}|{}|generics=[{}]",
            self.repository.as_str(),
            self.revision.kind,
            self.revision.value,
            self.language,
            self.scope.identity_key(),
            self.symbol.identity_key(),
            self.span.path,
            self.span.line,
            self.span.column,
            self.generated,
            self.declaration_kind.as_str(),
            self.owner.identity_key(),
            generics,
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
    /// BLAKE3 of the function body's token stream (G66): position-free, whitespace- and
    /// comment-insensitive, literal-sensitive. Content evidence for cross-revision correspondence,
    /// never part of any identity key. `None` for a declaration without a body.
    #[serde(default)]
    pub body_fingerprint: Option<String>,
}

impl FunctionSignature {
    /// A source-spelling display summary (`"pub async unsafe extern \"C\" fn<T>(x: u8) -> Result<T,
    /// E>"`), never a canonical/compiler-resolved signature -- built purely from each
    /// parameter/return type's own `TypeIdentity.name`, which is exactly the source-syntax spelling
    /// this whole kernel commits to (see this module's own doc comment). Shared canonical form for
    /// every caller that needs a display/summary string for a signature -- previously duplicated,
    /// byte-for-byte identically, as a private free function in both `core::graph::engineering_graph`
    /// and `runtime::census`.
    ///
    /// `.atlas/contracts/SEMANTIC-FACTS.md#functionsignature` requires this facet to "account for
    /// parameters/order/types, return type, generics, ABI/calling convention, visibility/export
    /// surface, async/coroutine form and language-specific safety/effect qualifiers" -- every one of
    /// these is rendered here, not merely carried on the struct, because `runtime::census` uses this
    /// summary as the ENTIRE `object` value of the compatibility `SemanticFact` for the
    /// `function_signature` predicate (the sole content representing that claim for any consumer of
    /// the compat subject/predicate/object envelope rather than the full typed record).
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
        // "inherited" (`spelling::visibility_spelling`'s own name for no visibility keyword at
        // all) renders as nothing, matching real Rust's own private-item source spelling -- every
        // other value (`pub`, `pub(crate)`, ...) is a real keyword prefix.
        let visibility = if self.visibility == "inherited" {
            String::new()
        } else {
            format!("{} ", self.visibility)
        };
        let asyncness = if self.is_async { "async " } else { "" };
        let unsafety = if self.is_unsafe { "unsafe " } else { "" };
        let extern_abi = if self.is_extern {
            match &self.abi {
                Some(abi) => format!("extern \"{abi}\" "),
                None => "extern ".to_owned(),
            }
        } else {
            String::new()
        };
        let generics = if self.generics.is_empty() {
            String::new()
        } else {
            format!("<{}>", self.generics.join(", "))
        };
        format!(
            "{visibility}{asyncness}{unsafety}{extern_abi}fn{generics}({params}) -> {return_type}"
        )
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
                path: String::new(),
                repository,
                revision,
                scope,
                name: "run".into(),
                role: SymbolRole::Definition,
                documentation: None,
                declaration: None,
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
            body_fingerprint: None,
        };
        assert_eq!(
            signature.summary(),
            "pub async unsafe fn(x: Vec<T>, y: &mut usize) -> Result<T, Error>"
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
            visibility: "inherited".into(),
            is_async: false,
            is_unsafe: false,
            is_extern: false,
            body_fingerprint: None,
        };
        assert_eq!(signature.summary(), "fn() -> ()");
    }

    #[test]
    fn summary_renders_visibility_generics_and_extern_abi() {
        // The exact scenario `.atlas/contracts/SEMANTIC-FACTS.md#functionsignature` requires this
        // facet to account for: a real `pub(crate) unsafe extern "C" fn foo<T>(x: T) -> i32`. Every
        // field is real, extractor-populated data this summary is the SOLE rendering of for any
        // consumer of the compatibility `SemanticFact` envelope (`runtime::census`'s
        // `function_signature` predicate uses this string as its entire `object` value).
        let signature = FunctionSignature {
            function: identity("widgets"),
            parameters: vec![FunctionParameter {
                name: "x".into(),
                type_identity: type_identity("T"),
            }],
            return_type: Some(type_identity("i32")),
            generics: vec!["T".into()],
            abi: Some("C".into()),
            visibility: "pub(crate)".into(),
            is_async: false,
            is_unsafe: true,
            is_extern: true,
            body_fingerprint: None,
        };
        assert_eq!(
            signature.summary(),
            "pub(crate) unsafe extern \"C\" fn<T>(x: T) -> i32"
        );
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
            path: String::new(),
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

    #[test]
    fn identity_key_does_not_collide_across_a_different_scope_segment_split() {
        // A scope segment is not always a bare module-path identifier: `adapter::semantic::rust`'s
        // `handle_impl` pushes a segment like `"impl:Trait<Nested::Path> for Type"`, built from
        // unrestricted type/trait spellings that can themselves contain `:`. Two structurally
        // different scopes must never collapse onto one `FunctionIdentity` merely because their
        // segments happen to concatenate to the same "::"-joined string.
        let a = FunctionIdentity {
            scope: SemanticScope::new(["a::b", "c"]),
            ..identity("unused")
        };
        let b = FunctionIdentity {
            scope: SemanticScope::new(["a", "b::c"]),
            ..identity("unused")
        };
        assert_ne!(a.scope, b.scope, "sanity: genuinely different scopes");
        assert_ne!(
            a.identity_key(),
            b.identity_key(),
            "an unescaped scope join let two distinct scopes collapse onto one identity: {} == {}",
            a.identity_key(),
            b.identity_key(),
        );
    }

    #[test]
    fn identity_key_does_not_collide_across_a_different_generics_split() {
        // A generic param's spelling is not always a bare identifier: `syn`'s bound rendering
        // falls back to raw token-stream text for a bound like `From<(A, B)>`, which can contain a
        // literal `,` -- the same character `generics.join(",")` uses as its own delimiter.
        let a = FunctionIdentity {
            generics: vec!["T: From<(A, B)>".into()],
            ..identity("widgets")
        };
        let b = FunctionIdentity {
            generics: vec!["T: From<(A".into(), " B)>".into()],
            ..identity("widgets")
        };
        assert_ne!(
            a.generics, b.generics,
            "sanity: genuinely different generics lists"
        );
        assert_ne!(
            a.identity_key(),
            b.identity_key(),
            "an unescaped generics join let two distinct generics lists collapse onto one identity: {} == {}",
            a.identity_key(),
            b.identity_key(),
        );
    }
}
