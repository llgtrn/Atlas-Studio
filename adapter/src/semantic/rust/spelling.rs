//! Deterministic source-spelling stringifiers for `syn` AST fragments.
//!
//! These render the *declared* spelling of a type/pattern/visibility exactly as it appears in
//! source (e.g. `"Result<User, Error>"`, `"&mut User"`, `"[u8; 4]"`) — never a compiler-resolved
//! canonical form. `TypeIdentity.canonical` stays `None` throughout R4.3; this module only ever
//! produces the `name` half of a `TypeIdentity`. Common shapes (paths with generic arguments,
//! references, tuples, arrays, slices, raw pointers) are rendered by hand for clean output; rarer
//! shapes (trait objects, `impl Trait`, bare `fn` pointers, macros-in-type-position) fall back to
//! `quote`'s raw token-stream rendering, which is still deterministic, just less manicured.

use quote::ToTokens;

pub fn type_spelling(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => path_spelling(&type_path.path),
        syn::Type::Reference(reference) => {
            let lifetime = reference
                .lifetime
                .as_ref()
                .map(|lt| format!("'{} ", lt.ident))
                .unwrap_or_default();
            let mutability = if reference.mutability.is_some() {
                "mut "
            } else {
                ""
            };
            format!("&{lifetime}{mutability}{}", type_spelling(&reference.elem))
        }
        syn::Type::Tuple(tuple) => {
            if tuple.elems.is_empty() {
                "()".to_owned()
            } else {
                let elems = tuple
                    .elems
                    .iter()
                    .map(type_spelling)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({elems})")
            }
        }
        syn::Type::Array(array) => {
            let len = array.len.to_token_stream().to_string().replace(' ', "");
            format!("[{}; {len}]", type_spelling(&array.elem))
        }
        syn::Type::Slice(slice) => format!("[{}]", type_spelling(&slice.elem)),
        syn::Type::Ptr(ptr) => {
            let mutability = match ptr.mutability {
                syn::PointerMutability::Mut(_) => "mut",
                syn::PointerMutability::Const(_) => "const",
            };
            format!("*{mutability} {}", type_spelling(&ptr.elem))
        }
        syn::Type::Paren(paren) => format!("({})", type_spelling(&paren.elem)),
        syn::Type::Never(_) => "!".to_owned(),
        syn::Type::Infer(_) => "_".to_owned(),
        // Trait objects, `impl Trait`, bare `fn` pointers, macros-in-type-position, and any future
        // `syn::Type` variant: rendered via the raw token stream rather than hand-modeled. Still
        // deterministic (`syn`/`quote` are pure functions of the input tokens), just unmanicured.
        other => other.to_token_stream().to_string(),
    }
}

pub fn path_spelling(path: &syn::Path) -> String {
    let mut segments = Vec::with_capacity(path.segments.len());
    for segment in &path.segments {
        let mut piece = segment.ident.to_string();
        match &segment.arguments {
            syn::PathArguments::None => {}
            syn::PathArguments::AngleBracketed(args) => {
                let rendered = args
                    .args
                    .iter()
                    .map(generic_argument_spelling)
                    .collect::<Vec<_>>()
                    .join(", ");
                if !rendered.is_empty() {
                    piece.push('<');
                    piece.push_str(&rendered);
                    piece.push('>');
                }
            }
            syn::PathArguments::Parenthesized(args) => {
                piece.push_str(&args.to_token_stream().to_string());
            }
        }
        segments.push(piece);
    }
    let leading = if path.leading_colon.is_some() {
        "::"
    } else {
        ""
    };
    format!("{leading}{}", segments.join("::"))
}

fn generic_argument_spelling(arg: &syn::GenericArgument) -> String {
    match arg {
        syn::GenericArgument::Type(ty) => type_spelling(ty),
        syn::GenericArgument::Lifetime(lifetime) => format!("'{}", lifetime.ident),
        other => other.to_token_stream().to_string(),
    }
}

pub fn generic_param_spelling(param: &syn::GenericParam) -> String {
    match param {
        syn::GenericParam::Type(type_param) => {
            let mut spelling = type_param.ident.to_string();
            if !type_param.bounds.is_empty() {
                let bounds = type_param
                    .bounds
                    .iter()
                    .map(|bound| bound.to_token_stream().to_string())
                    .collect::<Vec<_>>()
                    .join(" + ");
                spelling.push_str(": ");
                spelling.push_str(&bounds);
            }
            spelling
        }
        syn::GenericParam::Lifetime(lifetime_param) => {
            format!("'{}", lifetime_param.lifetime.ident)
        }
        syn::GenericParam::Const(const_param) => {
            format!(
                "const {}: {}",
                const_param.ident,
                type_spelling(&const_param.ty)
            )
        }
    }
}

pub fn visibility_spelling(vis: &syn::Visibility) -> String {
    match vis {
        syn::Visibility::Public(_) => "pub".to_owned(),
        syn::Visibility::Restricted(restricted) => {
            format!(
                "pub({})",
                restricted.path.to_token_stream().to_string().trim()
            )
        }
        syn::Visibility::Inherited => "inherited".to_owned(),
    }
}

pub fn abi_spelling(abi: &syn::Abi) -> String {
    abi.name
        .as_ref()
        .map(|name| name.value())
        .unwrap_or_else(|| "C".to_owned())
}

/// The declared spelling of a method receiver's *type* -- `"Self"`, `"&Self"`, `"&mut Self"`, or
/// (for `self: Type` syntax) the explicit `Type`. Never the resolved concrete type (e.g. `"User"`):
/// `self`/`&self` literally spell `Self` in source, and only compiler resolution (out of scope for
/// R4.3) could say which concrete type that is.
pub fn receiver_type_spelling(receiver: &syn::Receiver) -> String {
    match &receiver.kind {
        syn::ReceiverKind::Value => "Self".to_owned(),
        syn::ReceiverKind::Reference(_, lifetime, mutability) => {
            let lifetime = lifetime
                .as_ref()
                .map(|lt| format!("'{} ", lt.ident))
                .unwrap_or_default();
            let mutability = if mutability.is_some() { "mut " } else { "" };
            format!("&{lifetime}{mutability}Self")
        }
        syn::ReceiverKind::Typed(_, ty) => type_spelling(ty),
        // `syn::ReceiverKind` is `#[non_exhaustive]`; any future variant is rendered via the raw
        // token stream rather than causing a compile break.
        _ => receiver.to_token_stream().to_string(),
    }
}

/// The declared spelling of a method receiver itself -- `"self"`, `"&self"`, `"&mut self"`, or
/// `"self: Type"`.
pub fn receiver_label(receiver: &syn::Receiver) -> String {
    match &receiver.kind {
        syn::ReceiverKind::Value if receiver.mutability.is_some() => "mut self".to_owned(),
        syn::ReceiverKind::Value => "self".to_owned(),
        syn::ReceiverKind::Reference(_, _, mutability) if mutability.is_some() => {
            "&mut self".to_owned()
        }
        syn::ReceiverKind::Reference(..) => "&self".to_owned(),
        syn::ReceiverKind::Typed(_, ty) => format!("self: {}", type_spelling(ty)),
        _ => receiver.to_token_stream().to_string(),
    }
}

/// The declared spelling of a direct call's callee expression -- e.g. `"foo"`, `"Type::method"`,
/// `"(get_fn())"`. This is a raw textual summary for evidence only, never a resolved target: R4.5's
/// Rust extractor makes no claim about which function a call reaches (see the `rust` module's doc
/// comment and `core/src/semantic/call.rs`).
pub fn call_callee_spelling(expr: &syn::Expr) -> String {
    match expr {
        syn::Expr::Path(path) => path_spelling(&path.path),
        other => other.to_token_stream().to_string(),
    }
}

pub fn pattern_spelling(pat: &syn::Pat) -> String {
    match pat {
        syn::Pat::Ident(ident) => ident.ident.to_string(),
        syn::Pat::Wild(_) => "_".to_owned(),
        other => other.to_token_stream().to_string(),
    }
}
