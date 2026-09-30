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

use std::collections::{BTreeMap, BTreeSet};
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

/// G174 (review of ADR 0088): the entry of a shadowing set that stands for every bare name --
/// a `#[macro_use] extern crate` brings in macros whose names this pass does not read. It is not
/// an identifier, so no macro is named by it.
pub(super) const EVERY_BARE_NAME: &str = "*";

/// The macro names that may shadow a standard macro's bare name in `files` (G174, review of ADR
/// 0088): every `macro_rules!` they define (textual scope reaches child modules and, through
/// `#[macro_use] mod`, parents), every name a `use` binds -- its last segment or its rename --
/// except a standard macro imported from `std`, `core` or `alloc` under its own name, and
/// [`EVERY_BARE_NAME`] when an `extern crate` other than those carries `#[macro_use]`. A bare
/// invocation of a name in the set is left opaque; a path-qualified `std::`/`core::`/`alloc::`
/// form still names the standard macro only if its name is not in the set either.
pub(super) fn shadowing_macro_names<'a>(
    files: impl IntoIterator<Item = &'a syn::File>,
) -> BTreeSet<String> {
    struct Bindings(BTreeSet<String>);
    fn use_tree(tree: &syn::UseTree, root: Option<&str>, out: &mut BTreeSet<String>) {
        let standard = |root: Option<&str>| matches!(root, Some("std" | "core" | "alloc"));
        match tree {
            syn::UseTree::Path(path) => {
                let segment = path.ident.to_string();
                use_tree(&path.tree, Some(root.unwrap_or(&segment)), out);
            }
            syn::UseTree::Name(name) => {
                if !standard(root) {
                    out.insert(name.ident.to_string());
                }
            }
            syn::UseTree::Rename(rename) => {
                if !standard(root) || rename.ident != rename.rename {
                    out.insert(rename.rename.to_string());
                }
            }
            syn::UseTree::Group(group) => {
                for tree in &group.items {
                    use_tree(tree, root, out);
                }
            }
            syn::UseTree::Glob(_) => {}
        }
    }
    impl<'ast> Visit<'ast> for Bindings {
        fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
            if let Some(ident) = &item.ident {
                self.0.insert(ident.to_string());
            }
            syn::visit::visit_item_macro(self, item);
        }
        fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
            use_tree(&item.tree, None, &mut self.0);
        }
        fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
            let name = item.ident.to_string();
            if item.attrs.iter().any(|a| a.path().is_ident("macro_use"))
                && !["std", "core", "alloc"].contains(&name.as_str())
            {
                self.0.insert(EVERY_BARE_NAME.to_owned());
            }
        }
    }
    let mut bindings = Bindings(BTreeSet::new());
    for file in files {
        bindings.visit_file(file);
    }
    bindings.0
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
    let bare = mac.path.segments.len() == 1;
    if shadowed.contains(&name) || (bare && shadowed.contains(EVERY_BARE_NAME)) {
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

/// Keywords that open a declaration the declaration walk records (G184): items with a SYMBOL or
/// TYPE record, and `fn`, whose FUNCTION_IDENTITY and FUNCTION_SIGNATURE records exist too.
const DECLARATION_KEYWORDS: &[&str] = &[
    "fn", "struct", "enum", "union", "trait", "impl", "type", "mod", "const", "static",
];

/// G184 (replay R18, verus): a macro invocation this extractor does not expand, or an item `syn`
/// kept as unparsed tokens, whose written tokens spell a declaration. Nothing inside it is
/// recorded, so the declarations written there are missing from the file's declaration records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HiddenDeclarations {
    /// `name!` as written, or `item` for tokens `syn` did not structure.
    pub(super) invocation: String,
    pub(super) line: usize,
    /// `fn` is spelled: functions may be hidden, not only symbols and types.
    pub(super) functions: bool,
    /// Inside a function body, a closure or `async` block is spelled: an executable region
    /// (a FUNCTION_IDENTITY record, G133/G159) may be hidden.
    pub(super) regions: bool,
    /// The invocation names a `macro_rules!` of this file whose transcribers spell a declaration.
    pub(super) template: bool,
    /// A hidden function CALL's `opaque_sites` does not count: an item `syn` kept as tokens or
    /// one written where the walk does not enter, or a macro in a `const`/`static` initializer.
    pub(super) call_opaque: bool,
}

/// `None` when `tokens` spell no declaration keyword, else whether one is `fn`. An identifier
/// after `'` is a lifetime (`'static`) and after `$` a metavariable, never a keyword.
fn spelled_declarations(tokens: proc_macro2::TokenStream) -> Option<bool> {
    use proc_macro2::TokenTree;
    let mut spelled = None;
    let mut previous: Option<char> = None;
    for token in tokens {
        match &token {
            TokenTree::Ident(ident) if !matches!(previous, Some('\'' | '$')) => {
                let text = ident.to_string();
                if DECLARATION_KEYWORDS.contains(&text.as_str()) {
                    spelled = Some(spelled.unwrap_or(false) || text == "fn");
                }
            }
            TokenTree::Group(group) => {
                if let Some(functions) = spelled_declarations(group.stream()) {
                    spelled = Some(spelled.unwrap_or(false) || functions);
                }
            }
            _ => {}
        }
        previous = match &token {
            TokenTree::Punct(punct) => Some(punct.as_char()),
            _ => None,
        };
    }
    spelled
}

/// Whether `tokens` spell a closure or an `async` block: a `|` (or `||`) where an expression starts
/// -- first in its group, or after an operator or separator (`,` `(` `=` `=>` `;` `&` ...), or
/// after `return`, `move`, `async`, `break` or `yield` -- never after a name, literal or closing group, so the
/// pattern alternation `A | B` and the bit-or `a | b` are not closures; and closed by a second
/// `|` with a body after it, so `Token![|]` is not one either; or `async` before a brace group or
/// `move`.
fn spelled_region(tokens: proc_macro2::TokenStream) -> bool {
    use proc_macro2::{Delimiter, TokenTree};
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Punct(punct) if punct.as_char() == '|' => {
                let expression_start = match index.checked_sub(1).map(|i| &tokens[i]) {
                    None => true,
                    // The second `|` of `||` is judged with the first.
                    Some(TokenTree::Punct(previous)) if previous.as_char() == '|' => false,
                    Some(TokenTree::Punct(previous)) => !matches!(previous.as_char(), '?' | '\''),
                    Some(TokenTree::Ident(previous)) => matches!(
                        previous.to_string().as_str(),
                        "return" | "move" | "async" | "break" | "yield"
                    ),
                    Some(_) => false,
                };
                // A closure closes its parameters and has a body: `|x| body`, `|| body`. A lone
                // `|` or `||` (`Token![|]`, `Token![||]`) is not one.
                let rest = &tokens[index + 1..];
                let closes = rest
                    .iter()
                    .position(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == '|'))
                    .is_some_and(|close| close + 1 < rest.len());
                if expression_start && closes {
                    return true;
                }
            }
            TokenTree::Ident(ident) if *ident == "async" => {
                let block = match tokens.get(index + 1) {
                    Some(TokenTree::Group(group)) => group.delimiter() == Delimiter::Brace,
                    Some(TokenTree::Ident(next)) => *next == "move",
                    _ => false,
                };
                if block {
                    return true;
                }
            }
            TokenTree::Group(group) if spelled_region(group.stream()) => return true,
            _ => {}
        }
    }
    false
}

/// What a `macro_rules!` of the file expands to, as far as its transcribers show: a declaration
/// (a keyword, or an invocation of a template that declares), a `fn`, a closure or `async` block.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Template {
    declares: bool,
    functions: bool,
    regions: bool,
}

impl Template {
    fn merge(&mut self, other: Template) -> bool {
        let before = *self;
        self.declares |= other.declares;
        self.functions |= other.functions;
        self.regions |= other.regions;
        *self != before
    }
}

/// The names a token stream invokes as macros: the last identifier before each `!`, so any path
/// (`name!`, `self::name!`, `$crate::m::name!`) names its last segment.
fn invoked_names(tokens: proc_macro2::TokenStream, out: &mut BTreeSet<String>) {
    use proc_macro2::TokenTree;
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Ident(ident) if matches!(tokens.get(index + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!') =>
            {
                let name = ident.to_string();
                out.insert(name.strip_prefix("r#").unwrap_or(&name).to_owned());
            }
            TokenTree::Group(group) => invoked_names(group.stream(), out),
            _ => {}
        }
    }
}

/// G184: what each `macro_rules!` of `file` expands to, as a fixpoint -- a template reaches what
/// it spells and what every same-file template it invokes reaches (`impl_all!` through
/// `impl_one!`), and a `use` alias (`use mk as other;`) reaches what its target does.
fn local_templates(file: &syn::File) -> BTreeMap<String, Template> {
    #[derive(Default)]
    struct Definitions {
        templates: Vec<(String, Template, BTreeSet<String>)>,
        aliases: Vec<(String, String)>,
    }
    fn renames(tree: &syn::UseTree, out: &mut Vec<(String, String)>) {
        match tree {
            syn::UseTree::Path(path) => renames(&path.tree, out),
            syn::UseTree::Rename(rename) => {
                out.push((rename.ident.to_string(), rename.rename.to_string()))
            }
            syn::UseTree::Group(group) => group.items.iter().for_each(|t| renames(t, out)),
            syn::UseTree::Name(_) | syn::UseTree::Glob(_) => {}
        }
    }
    impl<'ast> Visit<'ast> for Definitions {
        fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
            if let Some(ident) = &item.ident {
                let tokens = item.mac.tokens.clone();
                let spelled = spelled_declarations(tokens.clone());
                let own = Template {
                    declares: spelled.is_some(),
                    functions: spelled.unwrap_or(false),
                    regions: spelled_region(tokens.clone()),
                };
                let mut invoked = BTreeSet::new();
                invoked_names(tokens, &mut invoked);
                self.templates.push((ident.to_string(), own, invoked));
            }
            syn::visit::visit_item_macro(self, item);
        }
        fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
            renames(&item.tree, &mut self.aliases);
        }
    }
    let mut definitions = Definitions::default();
    definitions.visit_file(file);
    // A worklist over reverse edges (callee -> callers, alias target -> alias): a name is queued
    // when what it reaches grows, and its flags only grow (three bits), so each name is queued at
    // most four times -- linear in templates plus invocations, not one pass per chain link.
    let mut reach: BTreeMap<String, Template> = BTreeMap::new();
    let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (name, own, invoked) in &definitions.templates {
        reach.entry(name.clone()).or_default().merge(*own);
        for callee in invoked {
            dependents.entry(callee).or_default().push(name);
        }
    }
    for (target, alias) in &definitions.aliases {
        dependents.entry(target).or_default().push(alias);
    }
    let mut queue: std::collections::VecDeque<String> = reach.keys().cloned().collect();
    while let Some(name) = queue.pop_front() {
        let Some(reached) = reach.get(&name).copied() else {
            continue;
        };
        for dependent in dependents.get(name.as_str()).into_iter().flatten() {
            if reach
                .entry((*dependent).to_owned())
                .or_default()
                .merge(reached)
            {
                queue.push_back((*dependent).to_owned());
            }
        }
    }
    reach
}

/// Every place in `file` where the declaration walk cannot see declarations written in source
/// (G184): an invocation of a macro this extractor does not expand -- at item, associated-item,
/// statement or expression position; a `macro_rules!` definition is a template and declares
/// nothing -- or an item `syn` kept as unparsed tokens, whose tokens spell a declaration keyword.
/// A recovered standard macro's arguments are walked (G119), so they are searched instead. What
/// an invocation expands to without spelling a declaration (`typed_id!(NodeId)`), a derive, an
/// attribute macro's output and a monomorphized instance are generated, not written: they stay
/// outside the declared profile, as before. `const`/`static` initializers are not walked for
/// declarations, as for CALL.
pub(super) fn hidden_declarations(
    file: &syn::File,
    shadowed: &BTreeSet<String>,
) -> Vec<HiddenDeclarations> {
    use quote::ToTokens;
    struct Hidden<'s> {
        shadowed: &'s BTreeSet<String>,
        /// What each `macro_rules!` of the file (and each `use` alias of one) expands to.
        templates: BTreeMap<String, Template>,
        sites: Vec<HiddenDeclarations>,
        /// Depth of function bodies the walk enters (closures and `async` blocks inside them are
        /// regions too): only there is a nested item recorded and a closure a region.
        bodies: usize,
        /// Inside a `const`/`static` initializer, where CALL's profile does not look (G74).
        initializer: bool,
    }
    impl Hidden<'_> {
        fn push(&mut self, invocation: String, line: usize, functions: bool, regions: bool) {
            self.sites.push(HiddenDeclarations {
                invocation,
                line,
                functions,
                regions,
                template: false,
                // CALL counts a hidden function it would not otherwise see as opaque: an unwalked
                // item, or a macro in an initializer (G74 counts macros everywhere else).
                call_opaque: false,
            });
        }
        fn item(&mut self, what: &str, line: usize, tokens: proc_macro2::TokenStream) {
            if let Some(functions) = spelled_declarations(tokens) {
                self.push(what.to_owned(), line, functions, false);
                if let Some(site) = self.sites.last_mut() {
                    site.call_opaque = functions;
                }
            }
        }
        fn verbatim(&mut self, tokens: &proc_macro2::TokenStream) {
            use syn::spanned::Spanned;
            let line = tokens.span().start().line;
            self.item("item", line, tokens.clone());
        }
        /// An expression the walk never enters (an initializer, a discriminant, a type's array
        /// length): items there are unwalked and closures there are no regions (G133).
        fn unwalked(&mut self, initializer: bool, visit: impl FnOnce(&mut Self)) {
            let (bodies, outer) = (self.bodies, self.initializer);
            self.bodies = 0;
            self.initializer |= initializer;
            visit(self);
            self.bodies = bodies;
            self.initializer = outer;
        }
        fn body(&mut self, visit: impl FnOnce(&mut Self)) {
            let outer = self.initializer;
            self.initializer = false;
            self.bodies += 1;
            visit(self);
            self.bodies -= 1;
            self.initializer = outer;
        }
    }
    impl<'ast> Visit<'ast> for Hidden<'_> {
        fn visit_macro(&mut self, mac: &'ast syn::Macro) {
            match recovered_arguments(mac, self.shadowed) {
                Some(arguments) => {
                    for argument in &arguments {
                        self.visit_expr(argument);
                    }
                }
                None => {
                    let path = mac
                        .path
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect::<Vec<_>>();
                    let colon = if mac.path.leading_colon.is_some() {
                        "::"
                    } else {
                        ""
                    };
                    let line = mac.bang_token.span.start().line;
                    // A `macro_rules!` of this file (or an alias of one), by any path whose last
                    // segment names it, expands to what its transcribers spell or invoke.
                    let local = path
                        .last()
                        .and_then(|name| self.templates.get(name))
                        .copied()
                        .unwrap_or_default();
                    let in_body = self.bodies > 0;
                    let regions = in_body && (local.regions || spelled_region(mac.tokens.clone()));
                    let written = spelled_declarations(mac.tokens.clone());
                    if written.is_some() || regions || local.declares {
                        let functions = written.unwrap_or(false) || local.functions;
                        self.push(
                            format!("{colon}{}!", path.join("::")),
                            line,
                            functions,
                            regions,
                        );
                        if let Some(site) = self.sites.last_mut() {
                            site.template = local.declares || (in_body && local.regions);
                            site.call_opaque = self.initializer && functions;
                        }
                    }
                }
            }
        }
        fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
            // A `macro_rules!` definition is a template: it declares nothing until invoked.
            if item.ident.is_none() {
                syn::visit::visit_item_macro(self, item);
            }
        }
        fn visit_item(&mut self, item: &'ast syn::Item) {
            match item {
                syn::Item::Verbatim(tokens) => self.verbatim(tokens),
                _ => syn::visit::visit_item(self, item),
            }
        }
        fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
            match item {
                syn::ImplItem::Verbatim(tokens) => self.verbatim(tokens),
                _ => syn::visit::visit_impl_item(self, item),
            }
        }
        fn visit_trait_item(&mut self, item: &'ast syn::TraitItem) {
            match item {
                syn::TraitItem::Verbatim(tokens) => self.verbatim(tokens),
                _ => syn::visit::visit_trait_item(self, item),
            }
        }
        fn visit_foreign_item(&mut self, item: &'ast syn::ForeignItem) {
            match item {
                syn::ForeignItem::Verbatim(tokens) => self.verbatim(tokens),
                _ => syn::visit::visit_foreign_item(self, item),
            }
        }
        // An item written in a block the walk does not enter is in no record.
        fn visit_stmt(&mut self, stmt: &'ast syn::Stmt) {
            match stmt {
                syn::Stmt::Item(item)
                    if self.bodies == 0 && !matches!(item, syn::Item::Macro(_)) =>
                {
                    use syn::spanned::Spanned;
                    let line = item.span().start().line;
                    let what = if self.initializer {
                        "item in an initializer"
                    } else {
                        "item in an unwalked block"
                    };
                    self.item(what, line, item.to_token_stream());
                }
                _ => syn::visit::visit_stmt(self, stmt),
            }
        }
        fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
            for attribute in &item.attrs {
                self.visit_attribute(attribute);
            }
            self.visit_signature(&item.sig);
            self.body(|h| h.visit_block(&item.block));
        }
        fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
            for attribute in &item.attrs {
                self.visit_attribute(attribute);
            }
            self.visit_signature(&item.sig);
            self.body(|h| h.visit_block(&item.block));
        }
        fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
            for attribute in &item.attrs {
                self.visit_attribute(attribute);
            }
            self.visit_signature(&item.sig);
            if let Some(block) = &item.default {
                self.body(|h| h.visit_block(block));
            }
        }
        fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
            self.unwalked(true, |h| syn::visit::visit_item_const(h, item));
        }
        fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
            self.unwalked(true, |h| syn::visit::visit_item_static(h, item));
        }
        fn visit_impl_item_const(&mut self, item: &'ast syn::ImplItemConst) {
            self.unwalked(true, |h| syn::visit::visit_impl_item_const(h, item));
        }
        fn visit_trait_item_const(&mut self, item: &'ast syn::TraitItemConst) {
            self.unwalked(true, |h| syn::visit::visit_trait_item_const(h, item));
        }
        fn visit_variant(&mut self, variant: &'ast syn::Variant) {
            self.unwalked(false, |h| syn::visit::visit_variant(h, variant));
        }
        fn visit_type(&mut self, ty: &'ast syn::Type) {
            self.unwalked(false, |h| syn::visit::visit_type(h, ty));
        }
    }
    let mut hidden = Hidden {
        shadowed,
        templates: local_templates(file),
        sites: Vec::new(),
        bodies: 0,
        initializer: false,
    };
    hidden.visit_file(file);
    hidden.sites
}
