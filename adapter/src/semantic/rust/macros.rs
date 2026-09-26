//! G119 (P0, the mandatory native attack after the G118 hard-stop audit, ADR 0041): standard
//! macro arguments recovered as expressions.
//!
//! A macro invocation's arguments are an opaque `TokenStream` to `syn`, and this extractor never
//! expands macros (`.atlas/contracts/SEMANTIC-EXTRACTION.md`). That left one shared, permanent gap in
//! every expression walker: a call written only inside `println!(..)`, `assert_eq!(..)`,
//! `format!(..)`, `vec![..]` ... was invisible, so no file could ever claim CALL coverage.
//!
//! The recovery is deliberately narrow and never a guess:
//!
//! - Only the standard-library macros whose documented input is a comma-separated list of
//!   expressions ([`EXPRESSION_MACROS`]) are re-parsed, and only when the whole token stream parses
//!   as that list (`vec![elem; n]` as its repeat form). Every argument of these macros is an
//!   evaluated expression (format arguments by reference, `debug_assert!` arguments in debug
//!   builds), so every call written there is a real call site of the enclosing function.
//! - A named format argument (`name = expr`) contributes its value, never an assignment.
//! - Standard macros that evaluate no expression at all ([`INERT_MACROS`]: `stringify!`, `env!`,
//!   `concat!`, `include_str!`, ...) are recovered as containing no expression.
//! - A file that defines a `macro_rules!` of one of these names shadows it: nothing is recovered
//!   there, and the file stays UNKNOWN.
//!
//! [`opaque_sites`] counts what is still not recovered in a file -- any other macro invocation
//! (function-like at expression, statement or item level; `macro_rules!` definitions excepted),
//! and any attribute that is not a built-in or tool attribute on a function, impl, trait or module
//! (an attribute macro may rewrite a body), outside the CALL profile's own exclusions (closure and
//! `async` bodies, `const`/`static` initializers). CALL may claim OBSERVED coverage for a file only
//! when that count is zero: then every call site inside the declared profile is reachable by the
//! walker, which runtime-independent test
//! `call_sites_equal_an_independent_syntax_enumeration_on_real_sources` proves on every workspace
//! source.

use std::collections::BTreeSet;
use syn::punctuated::Punctuated;
use syn::visit::Visit;

/// Standard macros whose input is a comma-separated list of evaluated expressions.
pub(super) const EXPRESSION_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "dbg",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "eprint",
    "eprintln",
    "format",
    "format_args",
    "panic",
    "print",
    "println",
    "todo",
    "unimplemented",
    "unreachable",
    "vec",
    "write",
    "writeln",
];

/// Standard macros that evaluate no expression of the enclosing function.
pub(super) const INERT_MACROS: &[&str] = &[
    "cfg",
    "column",
    "compile_error",
    "concat",
    "env",
    "file",
    "include_bytes",
    "include_str",
    "line",
    "module_path",
    "option_env",
    "stringify",
];

/// Built-in and tool attributes, which cannot rewrite a function body.
const BUILTIN_ATTRIBUTES: &[&str] = &[
    "allow",
    "automatically_derived",
    "cfg",
    "cfg_attr",
    "cold",
    "deny",
    "deprecated",
    "derive",
    "doc",
    "expect",
    "export_name",
    "forbid",
    "ignore",
    "inline",
    "link_section",
    "macro_export",
    "macro_use",
    "must_use",
    "no_mangle",
    "non_exhaustive",
    "path",
    "repr",
    "should_panic",
    "target_feature",
    "test",
    "track_caller",
    "warn",
];
const TOOL_ATTRIBUTE_ROOTS: &[&str] = &["clippy", "diagnostic", "rustfmt"];

/// The macro's name when its path is a bare name or rooted at `std`, `core` or `alloc`.
fn std_macro_name(mac: &syn::Macro) -> Option<String> {
    let segments: Vec<String> = mac
        .path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    match segments.as_slice() {
        [name] => Some(name.clone()),
        [root, name] if ["std", "core", "alloc"].contains(&root.as_str()) => Some(name.clone()),
        _ => None,
    }
}

/// The names of the `macro_rules!` a file defines (shadowing the standard macros there).
pub(super) fn local_macro_names(file: &syn::File) -> BTreeSet<String> {
    struct Definitions(BTreeSet<String>);
    impl<'ast> Visit<'ast> for Definitions {
        fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
            if let Some(ident) = &item.ident {
                self.0.insert(ident.to_string());
            }
            syn::visit::visit_item_macro(self, item);
        }
    }
    let mut definitions = Definitions(BTreeSet::new());
    definitions.visit_file(file);
    definitions.0
}

/// How a recovered macro argument is used by the standard macro's documented expansion (G120).
/// Each walker applies its own dimension's semantics to the role; the role never invents one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ArgumentRole {
    /// Evaluated by value: `assert!` conditions, `dbg!`/`vec!` elements, a non-literal `panic!`
    /// payload (moved or copied).
    Evaluated,
    /// A format argument: `format_args!` takes it by shared reference (a read, never a move).
    Formatted,
    /// An `assert_eq!`/`assert_ne!` operand: compared through a shared reference.
    Compared,
    /// The destination of `write!`/`writeln!`: the receiver of `.write_fmt(..)`, auto-referenced
    /// mutably by method-call syntax.
    WriteTarget,
    /// The format string itself (a literal, or an expression such as `concat!(..)`).
    FormatString,
}

/// A recovered standard macro invocation: its arguments with their roles, and the implicit
/// captures (`{x}`, `{:width$}`) its format string reads, each at its exact source position.
pub(super) struct Recovered {
    pub(super) name: String,
    pub(super) arguments: Vec<(syn::Expr, ArgumentRole)>,
    pub(super) captures: Vec<(String, proc_macro2::LineColumn)>,
}

/// The standard macros that panic when a condition fails (a conditional panic, never an
/// unconditional one -- `panic!`/`unreachable!`/`todo!`/`unimplemented!` are those).
pub(super) const ASSERT_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
];

/// The recovered structure of a standard macro invocation, or `None` when it stays opaque (not a
/// standard macro, shadowed locally, or not parseable as its documented input).
pub(super) fn recover(mac: &syn::Macro, shadowed: &BTreeSet<String>) -> Option<Recovered> {
    let name = std_macro_name(mac)?;
    if shadowed.contains(&name) {
        return None;
    }
    if INERT_MACROS.contains(&name.as_str()) {
        return Some(Recovered {
            name,
            arguments: Vec::new(),
            captures: Vec::new(),
        });
    }
    if !EXPRESSION_MACROS.contains(&name.as_str()) {
        return None;
    }
    if name == "vec"
        && let Ok((element, length)) = mac.parse_body_with(|input: syn::parse::ParseStream| {
            let element: syn::Expr = input.parse()?;
            input.parse::<syn::Token![;]>()?;
            let length: syn::Expr = input.parse()?;
            Ok((element, length))
        })
    {
        return Some(Recovered {
            name,
            arguments: vec![
                (element, ArgumentRole::Evaluated),
                (length, ArgumentRole::Evaluated),
            ],
            captures: Vec::new(),
        });
    }
    let parsed: Vec<syn::Expr> = mac
        .parse_body_with(Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated)
        .ok()?
        .into_iter()
        .collect();
    let is_literal = |e: &syn::Expr| {
        matches!(
            e,
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(_),
                ..
            })
        )
    };
    // Position of the format string in the documented input, if the macro has one.
    let format_at = match name.as_str() {
        "format" | "format_args" | "print" | "println" | "eprint" | "eprintln" => Some(0),
        "write" | "writeln" => Some(1),
        // A 2018-style `panic!(payload)` with a non-literal single argument moves the payload.
        "panic" | "todo" | "unimplemented" | "unreachable" => {
            (parsed.len() != 1 || is_literal(&parsed[0])).then_some(0)
        }
        "assert" | "debug_assert" => Some(1),
        "assert_eq" | "assert_ne" | "debug_assert_eq" | "debug_assert_ne" => Some(2),
        _ => None, // dbg, vec: every argument is evaluated by value
    };
    let mut named = BTreeSet::new();
    let mut arguments = Vec::with_capacity(parsed.len());
    let mut format_literal = None;
    for (index, argument) in parsed.into_iter().enumerate() {
        let role = match (name.as_str(), format_at) {
            ("write" | "writeln", _) if index == 0 => ArgumentRole::WriteTarget,
            ("assert_eq" | "assert_ne" | "debug_assert_eq" | "debug_assert_ne", _) if index < 2 => {
                ArgumentRole::Compared
            }
            ("assert" | "debug_assert", _) if index == 0 => ArgumentRole::Evaluated,
            (_, Some(at)) if index == at => ArgumentRole::FormatString,
            (_, Some(at)) if index > at => ArgumentRole::Formatted,
            _ => ArgumentRole::Evaluated,
        };
        let argument = match argument {
            // `format!("{x}", x = value)`: a named argument contributes its value.
            syn::Expr::Assign(assign)
                if role == ArgumentRole::Formatted
                    && matches!(*assign.left, syn::Expr::Path(_)) =>
            {
                if let syn::Expr::Path(path) = assign.left.as_ref()
                    && let Some(ident) = path.path.get_ident()
                {
                    named.insert(ident.to_string());
                }
                *assign.right
            }
            other => other,
        };
        if role == ArgumentRole::FormatString
            && let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(literal),
                ..
            }) = &argument
        {
            format_literal = Some(literal.clone());
        }
        arguments.push((argument, role));
    }
    let captures = format_literal
        .map(|literal| implicit_captures(&literal, &named))
        .unwrap_or_default();
    Some(Recovered {
        name,
        arguments,
        captures,
    })
}

/// The evaluated expressions of a recovered standard macro (every role), for CALL (G119).
pub(super) fn recovered_arguments(
    mac: &syn::Macro,
    shadowed: &BTreeSet<String>,
) -> Option<Vec<syn::Expr>> {
    recover(mac, shadowed).map(|r| r.arguments.into_iter().map(|(e, _)| e).collect())
}

/// The identifiers a format string captures implicitly (`{x}`, `{x:?}`, `{:w$}`, `{:.p$}`) that
/// are not explicit named arguments, each at the exact source position of the identifier. The
/// std format grammar is applied to the literal's SOURCE text, so positions are exact; braces
/// are never backslash-escaped in Rust, and `\u{..}` escapes are skipped.
fn implicit_captures(
    literal: &syn::LitStr,
    named: &BTreeSet<String>,
) -> Vec<(String, proc_macro2::LineColumn)> {
    let source = literal.token().to_string();
    let start = literal.span().start();
    // Skip the opening delimiter: `"`, or `r"` / `r#..#"` for raw strings.
    let raw = source.starts_with('r');
    let open = source.find('"').unwrap_or(0) + 1;
    let chars: Vec<char> = source.chars().collect();
    let (mut line, mut column) = (start.line, start.column);
    for c in &chars[..open] {
        if *c == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
    }
    let mut positions = Vec::with_capacity(chars.len());
    for c in &chars[open..] {
        positions.push(proc_macro2::LineColumn { line, column });
        if *c == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
    }
    let body = &chars[open..];
    let mut captures = Vec::new();
    let mut i = 0;
    let identifier = |from: usize| -> (String, usize) {
        let mut end = from;
        while end < body.len() && (body[end].is_alphanumeric() || body[end] == '_') {
            end += 1;
        }
        (body[from..end].iter().collect(), end)
    };
    let is_name = |s: &str| {
        s.chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
            && s != "_"
    };
    while i < body.len() {
        match body[i] {
            '\\' if !raw => {
                // `\u{..}` contains braces that are not format syntax.
                if body.get(i + 1) == Some(&'u') && body.get(i + 2) == Some(&'{') {
                    while i < body.len() && body[i] != '}' {
                        i += 1;
                    }
                }
                i += 2;
            }
            '{' if body.get(i + 1) == Some(&'{') => i += 2,
            '{' => {
                let (argument, mut j) = identifier(i + 1);
                if is_name(&argument) && !named.contains(&argument) {
                    captures.push((argument, positions[i + 1]));
                }
                // Width/precision parameters (`{:w$}`, `{:.p$}`) inside the spec.
                while j < body.len() && body[j] != '}' {
                    if body[j].is_alphabetic() || body[j] == '_' {
                        let (parameter, end) = identifier(j);
                        if body.get(end) == Some(&'$')
                            && is_name(&parameter)
                            && !named.contains(&parameter)
                        {
                            captures.push((parameter, positions[j]));
                        }
                        j = end.max(j + 1);
                    } else {
                        j += 1;
                    }
                }
                i = j + 1;
            }
            _ => i += 1,
        }
    }
    captures
}

fn is_builtin_attribute(attribute: &syn::Attribute) -> bool {
    let segments: Vec<String> = attribute
        .path()
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    match segments.as_slice() {
        [name] => BUILTIN_ATTRIBUTES.contains(&name.as_str()),
        [root, ..] => TOOL_ATTRIBUTE_ROOTS.contains(&root.as_str()),
        [] => false,
    }
}

/// Macro invocations and body-rewriting attributes in `file` that the walkers cannot see through.
pub(super) fn opaque_sites(file: &syn::File, shadowed: &BTreeSet<String>) -> usize {
    struct Opaque<'s> {
        shadowed: &'s BTreeSet<String>,
        count: usize,
    }
    impl Opaque<'_> {
        fn attributes(&mut self, attributes: &[syn::Attribute]) {
            self.count += attributes
                .iter()
                .filter(|a| !is_builtin_attribute(a))
                .count();
        }
    }
    impl<'ast> Visit<'ast> for Opaque<'_> {
        fn visit_macro(&mut self, mac: &'ast syn::Macro) {
            match recovered_arguments(mac, self.shadowed) {
                Some(arguments) => {
                    for argument in &arguments {
                        self.visit_expr(argument);
                    }
                }
                None => self.count += 1,
            }
        }
        fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
            // A `macro_rules!` definition is a template, not a call site of any function.
            if item.ident.is_none() {
                syn::visit::visit_item_macro(self, item);
            }
        }
        fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
            self.attributes(&item.attrs);
            syn::visit::visit_item_fn(self, item);
        }
        fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
            self.attributes(&item.attrs);
            syn::visit::visit_impl_item_fn(self, item);
        }
        fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
            self.attributes(&item.attrs);
            syn::visit::visit_trait_item_fn(self, item);
        }
        fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
            self.attributes(&item.attrs);
            syn::visit::visit_item_impl(self, item);
        }
        fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
            self.attributes(&item.attrs);
            syn::visit::visit_item_trait(self, item);
        }
        fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
            self.attributes(&item.attrs);
            syn::visit::visit_item_mod(self, item);
        }
        // The CALL profile's exclusions (G74): deferred executable regions and initializers with
        // no calling function. What is opaque there cannot hide an in-profile call site.
        // G133: closure bodies are walked as their own regions, so their opaque sites count;
        // since G159 `async` blocks are regions too.
        fn visit_item_const(&mut self, _: &'ast syn::ItemConst) {}
        fn visit_item_static(&mut self, _: &'ast syn::ItemStatic) {}
        fn visit_impl_item_const(&mut self, _: &'ast syn::ImplItemConst) {}
        fn visit_trait_item_const(&mut self, _: &'ast syn::TraitItemConst) {}
    }
    let mut opaque = Opaque { shadowed, count: 0 };
    opaque.visit_file(file);
    opaque.count
}
