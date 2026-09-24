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
        // A qualified path (`<A as Container>::Item`) stores its qualifying type in `qself`, NOT
        // as a path segment: `type_path.path.segments` alone is just `["Container", "Item"]"
        // regardless of whether the qself type is `A`, `B`, or anything else, so reading only
        // `.path` here would collide two genuinely different types (`<A as Container>::Item` and
        // `<B as Container>::Item`) onto the identical spelling -- the same class of collision
        // already fixed for 1-element tuples. Rather than hand-reimplement `syn`'s own qself/path
        // splitting logic (trait-vs-rest segment position, optional `as Trait`), fall back to the
        // raw token stream here, matching this function's own documented policy for rarer shapes.
        syn::Type::Path(type_path) if type_path.qself.is_none() => path_spelling(&type_path.path),
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
                if tuple.elems.len() == 1 {
                    // `syn::TypeTuple.elems` holds only the element itself -- the trailing comma
                    // that syntactically distinguishes a real 1-element tuple type (`(u64,)`) from
                    // a merely-parenthesized type (`(u64)`, i.e. plain `u64`, handled by the
                    // `Type::Paren` arm below) is not itself an element. Without restoring it here,
                    // `(u64,)` and `(u64)` would render to the identical string `"(u64)"`, and
                    // since `TypeIdentity.canonical` stays `None` throughout this extractor's scope
                    // (this module's own doc comment), two genuinely different Rust types would
                    // collide onto the exact same `TypeIdentity.identity_key()` -- the same graph
                    // node for a real, observable type difference.
                    format!("({elems},)")
                } else {
                    format!("({elems})")
                }
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
            let mut spelling = format!("'{}", lifetime_param.lifetime.ident);
            if !lifetime_param.bounds.is_empty() {
                let bounds = lifetime_param
                    .bounds
                    .iter()
                    .map(|bound| format!("'{}", bound.ident))
                    .collect::<Vec<_>>()
                    .join(" + ");
                spelling.push_str(": ");
                spelling.push_str(&bounds);
            }
            spelling
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
            let path = restricted.path.to_token_stream().to_string();
            // `restricted.in_token` is `Some` only for `pub(in path::to::mod)` -- `pub(crate)`,
            // `pub(super)`, and `pub(self)` have no `in` keyword and must not gain one, since
            // `pub(in crate)` and `pub(crate)` are different, both-valid, differently-scoped
            // restrictions in real Rust syntax.
            if restricted.in_token.is_some() {
                format!("pub(in {})", path.trim())
            } else {
                format!("pub({})", path.trim())
            }
        }
        syn::Visibility::Inherited => "inherited".to_owned(),
    }
}

/// The spelling of every `where`-clause predicate on a generic item, one entry per predicate
/// (e.g. `["where T: Clone", "where 'a: 'b"]`), appended alongside -- never merged into --
/// `generic_param_spelling`'s own per-parameter entries: a predicate's `to_token_stream`
/// rendering does not by itself say which earlier parameter it constrains, so keeping each
/// predicate as its own distinct entry avoids fabricating that association. `syn::WherePredicate`
/// is `#[non_exhaustive]`, so this deliberately renders the whole predicate via its token stream
/// rather than matching its variants, staying correct if `syn` ever adds a new predicate kind.
pub fn where_predicate_spelling(clause: &syn::WhereClause) -> Vec<String> {
    clause
        .predicates
        .iter()
        .map(|predicate| format!("where {}", predicate.to_token_stream()))
        .collect()
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

/// The identifier a `let`/parameter pattern binds, if it is (or wraps, via type ascription) a
/// plain `syn::Pat::Ident` -- i.e. a SINGLE simple binding, not a tuple/struct/slice/... pattern
/// that would bind more than one name (or none). Used where a caller needs to know "does this
/// whole pattern reduce to exactly one identifier", distinct from `dataflow.rs`'s
/// `walk_binding_pat`, which instead finds EVERY identifier a pattern binds (including inside a
/// destructuring one).
pub fn simple_binding_ident(pat: &syn::Pat) -> Option<&syn::Ident> {
    match pat {
        syn::Pat::Ident(pat_ident) => Some(&pat_ident.ident),
        syn::Pat::Type(pat_type) => simple_binding_ident(&pat_type.pat),
        _ => None,
    }
}

/// The identifier `expr` names, if it is a bare, single-segment, unqualified path expression
/// (`x`, not `x.field`, `Type::x`, `::x`, `(expr)`, or anything else). This is the exact shape
/// `dataflow.rs`'s `Expr::Path` arm itself matches -- but NOT, on its own, every shape that
/// ultimately REACHES that arm: `dataflow.rs`'s own recursive `walk_expr` transparently descends
/// through `Expr::Paren`/`Expr::Reference` wrapping BEFORE reaching it (its `Expr::Paren`/
/// `Expr::Reference` arms), so `(x)` and `&x` both still produce a real `Use`/`Store` record for
/// `x`. A caller that needs to recognize a WRAPPED shape identically to how it will ultimately
/// resolve in DATA_FLOW must unwrap first, with `unwrap_parens` (place OR value position -- a
/// parenthesized expression is always semantically identical to what it wraps) or
/// `unwrap_value_read` (value-read position only -- `&expr` is never a legal assignment target,
/// so it must never be unwrapped when recognizing a Store/assignment place).
pub fn simple_path_ident(expr: &syn::Expr) -> Option<&syn::Ident> {
    match expr {
        syn::Expr::Path(path)
            if path.path.leading_colon.is_none() && path.path.segments.len() == 1 =>
        {
            Some(&path.path.segments[0].ident)
        }
        _ => None,
    }
}

/// Recursively strips `(expr)` wrapping to find the underlying expression -- always safe in any
/// position (a value read OR an assignable place): a parenthesized expression is semantically
/// identical to what it wraps.
pub fn unwrap_parens(expr: &syn::Expr) -> &syn::Expr {
    match expr {
        syn::Expr::Paren(paren) => unwrap_parens(&paren.expr),
        _ => expr,
    }
}

/// Recursively strips `(expr)` and `&expr`/`&mut expr` wrapping to find the underlying place a
/// READ of `expr` ultimately reads -- the same transparent unwrapping `dataflow.rs`'s own
/// recursive `walk_expr` performs (its `Expr::Paren`/`Expr::Reference` arms) before ever reaching
/// a `Use`-emitting `Expr::Path`. Only valid for a value-read position: `&expr` is never a legal
/// assignment target, so this must never be used to recognize a Store/assignment place (use
/// `unwrap_parens` alone there, matching `dataflow.rs`'s own `Expr::Assign` arm, which does not
/// unwrap `Expr::Reference` on its left-hand side either).
pub fn unwrap_value_read(expr: &syn::Expr) -> &syn::Expr {
    match expr {
        syn::Expr::Paren(paren) => unwrap_value_read(&paren.expr),
        syn::Expr::Reference(reference) => unwrap_value_read(&reference.expr),
        _ => expr,
    }
}

/// Whether `op` is a compound-assignment operator (`+=`, `-=`, `*=`, `/=`, `%=`, `^=`, `&=`, `|=`,
/// `<<=`, `>>=`) rather than a plain arithmetic/bitwise operator (`+`, `-`, ...).
///
/// `syn` gives these distinct `BinOp` variants (`BinOp::AddAssign` vs `BinOp::Add`, etc.) -- fully
/// syntax-determined, no type resolution needed. A `syn::Expr::Binary` carrying one of these ops is
/// therefore provably a read-modify-write of its left operand, not merely a read: R4.7's
/// `dataflow.rs` uses this to emit a Use+Store pair for a simple-identifier left operand,
/// correcting an earlier reading of this `syn` version's `Expr::Binary` representation that
/// concluded (wrongly) that compound assignment could not be distinguished from plain arithmetic
/// without deeper analysis.
pub fn is_compound_assign_op(op: &syn::BinOp) -> bool {
    matches!(
        op,
        syn::BinOp::AddAssign(_)
            | syn::BinOp::SubAssign(_)
            | syn::BinOp::MulAssign(_)
            | syn::BinOp::DivAssign(_)
            | syn::BinOp::RemAssign(_)
            | syn::BinOp::BitXorAssign(_)
            | syn::BinOp::BitAndAssign(_)
            | syn::BinOp::BitOrAssign(_)
            | syn::BinOp::ShlAssign(_)
            | syn::BinOp::ShrAssign(_)
    )
}

/// Whether `mac`'s path ends in a segment textually spelled `panic`/`unreachable`/`todo`/
/// `unimplemented` -- a macro name is useful evidence but never proof the invoked macro resolves
/// to Rust's standard panic behavior (macro bindings may be shadowed; this extractor performs no
/// macro/name resolution), so every caller of this function must record its own finding as
/// `EpistemicStatus::Inferred`, never `Observed`.
///
/// Shared by `cfg.rs` (a block whose only successor is a panic-like macro invocation) and
/// `effect.rs` (the same invocation as an `EffectCategory::Panic` candidate) so the two dimensions
/// can never silently disagree about which macro invocations count -- the same defect class (and
/// the same fix shape) as `persistence.rs`'s `persistence_kind_for_spelling` unification: this
/// function used to be defined twice, once per file, byte-for-byte identical, with nothing
/// preventing the two copies from drifting apart on a future edit to only one of them.
pub fn is_panic_like_macro(mac: &syn::Macro) -> bool {
    mac.path.segments.last().is_some_and(|segment| {
        matches!(
            segment.ident.to_string().as_str(),
            "panic" | "unreachable" | "todo" | "unimplemented"
        )
    })
}
