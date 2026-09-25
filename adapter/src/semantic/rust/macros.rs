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

/// The evaluated expressions of a recovered standard macro, or `None` when the invocation stays
/// opaque (not a standard macro, shadowed locally, or not parseable as its documented input).
pub(super) fn recovered_arguments(
    mac: &syn::Macro,
    shadowed: &BTreeSet<String>,
) -> Option<Vec<syn::Expr>> {
    let name = std_macro_name(mac)?;
    if shadowed.contains(&name) {
        return None;
    }
    if INERT_MACROS.contains(&name.as_str()) {
        return Some(Vec::new());
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
        return Some(vec![element, length]);
    }
    let arguments = mac
        .parse_body_with(Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated)
        .ok()?;
    Some(
        arguments
            .into_iter()
            .map(|argument| match argument {
                // `format!("{x}", x = value)`: a named argument contributes its value.
                syn::Expr::Assign(assign) if matches!(*assign.left, syn::Expr::Path(_)) => {
                    *assign.right
                }
                other => other,
            })
            .collect(),
    )
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
        fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}
        fn visit_expr_async(&mut self, _: &'ast syn::ExprAsync) {}
        fn visit_item_const(&mut self, _: &'ast syn::ItemConst) {}
        fn visit_item_static(&mut self, _: &'ast syn::ItemStatic) {}
        fn visit_impl_item_const(&mut self, _: &'ast syn::ImplItemConst) {}
        fn visit_trait_item_const(&mut self, _: &'ast syn::TraitItemConst) {}
    }
    let mut opaque = Opaque { shadowed, count: 0 };
    opaque.visit_file(file);
    opaque.count
}
