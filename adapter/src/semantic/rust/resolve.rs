//! Native Rust name resolution for path calls (G75; absorbed from rust-analyzer #4, `hir-def`'s
//! `nameres`).
//!
//! The mechanism is `hir-def`'s DefMap reduced to what resolving a *path call* from source alone
//! needs: every crate target's module tree (`mod x;` files located the way `mod_resolution.rs`
//! does), each module's item scope in two namespaces (types, values) with visibility, `use`
//! imports (named, renamed, grouped, `self`, globs) resolved to a fixed point in which an item
//! shadows a named import and a named import shadows a glob, block scopes for items declared inside
//! a function body, the extern prelude of workspace dependencies, and the crate-wide inherent/trait
//! impl lookup `Type::assoc()` needs. Macro expansion, cfg evaluation and type inference are out of
//! scope, so every call this module cannot resolve *soundly* is left unresolved with a reason --
//! it never guesses:
//!
//! - a single-segment callee that names a local binding or parameter anywhere in the enclosing
//!   function (a closure or fn pointer, `let f = ..; f()`), or a path through a generic parameter
//!   (`T::new()`), is a value this pass does not track;
//! - a module whose scope has an unresolvable glob (`use std::collections::*`) or item-position
//!   macro invocations may hold names this pass cannot see, so a miss there never falls through
//!   to an outer scope, and a glob-provided hit is not trusted;
//! - two definitions of one name in one namespace (cfg alternates, conflicting globs) are
//!   ambiguous, never a pick;
//! - a path through a trait (`Trait::f(x)`, `Self::f()` inside a trait) is dynamic dispatch.
//!
//! G83: every type occurrence is canonicalized the same way, bottom-up -- a workspace type to its
//! defining module path, a standard-library type to its std path, references, slices, arrays,
//! tuples and generic arguments structurally -- so every spelling of one type shares one canonical
//! identity, without an e-graph (all its equalities come from resolution). Generic parameters, a
//! generic `Self`, aliases whose arguments a bare path cannot carry, qualified paths, trait objects
//! and glob names of open modules have none.
//!
//! Method calls need the receiver's type, with one exception (G79): `self.m()` inside an impl
//! method, whose receiver has the impl's self type. The method probe's first step tries that type
//! by value, inherent methods before trait methods, so the unique inherent method whose receiver
//! form (`self`/`&self`/`&mut self`) equals the caller's is the one called, when its impl is
//! shaped (generics, self-type arguments) exactly like the caller's. Every other method call is
//! left to type inference.

use std::collections::{BTreeMap, BTreeSet};

use syn::spanned::Spanned;
use syn::visit::Visit;

/// One crate target (a library, binary or build script) of a workspace package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateInput {
    /// Artifact path of the crate root file (`core/src/lib.rs`, `adapter/build.rs`, ...).
    pub root: String,
    /// Extern-prelude names of *workspace* crates this target may use, to the index of that
    /// crate's own `CrateInput` (`atlas_core` -> the `core` library). Every other dependency is
    /// external and resolves nothing.
    pub externs: BTreeMap<String, usize>,
}

/// A function definition a call resolved to, located the way the syntactic extractor locates its
/// `FunctionIdentity` (the item's own start position).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FnTarget {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathCallOutcome {
    Resolved(FnTarget),
    /// G140: a method of a workspace trait, called through a receiver whose type is only known
    /// by its bounds (`&dyn Tr`, `T: Tr`, `impl Tr`, `self` in a default body). The trait's
    /// declaration is the callee; the implementation is chosen at run time or instantiation.
    Dynamic(FnTarget),
    /// A path outside the workspace. The canonical path is spelled through imports and renames
    /// when it is rooted at `std`, `core` or `alloc` (`std::fs::write`), and empty otherwise (a
    /// registry crate or a prelude name this pass does not follow).
    External(String),
    Unresolved(&'static str),
}

impl PathCallOutcome {
    /// Whether the call goes through a trait bound (G140).
    pub fn is_dynamic(&self) -> bool {
        matches!(self, Self::Dynamic(_))
    }
}

/// One path call site (`f()`, `a::f()`, `Type::f()`), anchored exactly like the CALL claim the
/// syntactic extractor makes for it (its last path segment).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathCallResolution {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub callee: String,
    pub outcome: PathCallOutcome,
}

/// Resolves every path call inside a function body of every file reachable from `crates`' roots.
/// `sources` maps artifact paths to their text; a file that is missing or fails to parse leaves
/// the modules it would define empty and *open* (nothing resolves through them).
pub fn resolve_path_calls(
    crates: &[CrateInput],
    sources: &BTreeMap<String, String>,
) -> Vec<PathCallResolution> {
    resolve_workspace(crates, sources).calls
}

/// Every path call of the workspace, every type occurrence with its canonical identity, and the
/// files the crate roots reach (a file no root reaches was never evaluated).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceResolution {
    pub reached: BTreeSet<String>,
    pub calls: Vec<PathCallResolution>,
    pub types: Vec<TypeResolution>,
    /// G157: where let-bound resources are given back.
    pub releases: Vec<ReleaseResolution>,
}

/// One type occurrence (G83): its spelling as the syntactic extractor spells it
/// (`spelling::type_spelling`), and its canonical identity when every part of it resolves --
/// workspace types as `<package> <module path>/<Name>#`, standard-library types by their std
/// path, composed structurally (references, slices, arrays, tuples, generic arguments). `None`
/// whenever any part does not resolve soundly (a generic parameter, a generic `Self`, a generic
/// alias, a qualified path, `impl`/`dyn` traits, an open scope).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeResolution {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub spelling: String,
    pub canonical: Option<String>,
}

pub fn resolve_workspace(
    crates: &[CrateInput],
    sources: &BTreeMap<String, String>,
) -> WorkspaceResolution {
    // The same recursion discipline as the syntactic extractor: parse and walk on its large
    // dedicated stack, and never hand `syn` a file the recursion-risk pre-scan refuses.
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(super::EXTRACTION_STACK_SIZE)
            .spawn_scoped(scope, || resolve_on_this_stack(crates, sources))
            .expect("spawning the resolution worker thread must not fail")
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}

fn resolve_on_this_stack(
    crates: &[CrateInput],
    sources: &BTreeMap<String, String>,
) -> WorkspaceResolution {
    let mut map = DefMap::default();
    let mut parsed: BTreeMap<String, syn::File> = BTreeMap::new();
    for (index, krate) in crates.iter().enumerate() {
        map.collect_crate(index, krate, sources, &mut parsed);
    }
    map.resolve_macro_calls();
    map.resolve_imports(crates);
    map.resolve_impls(crates);
    let mut out = Vec::new();
    let mut types = Vec::new();
    let mut releases = Vec::new();
    for (file, module) in map.file_modules.clone() {
        if let Some(ast) = parsed.get(&file) {
            let mut walker = CallWalker {
                map: &map,
                crates,
                file: &file,
                scopes: vec![module],
                impl_self: Vec::new(),
                fn_ctx: Vec::new(),
                item_generics: Vec::new(),
                out: &mut out,
                types: &mut types,
                releases: &mut releases,
            };
            walker.visit_file(ast);
        }
    }
    out.sort_by(|a, b| (&a.path, a.line, a.column).cmp(&(&b.path, b.line, b.column)));
    types.sort_by(|a, b| (&a.path, a.line, a.column).cmp(&(&b.path, b.line, b.column)));
    releases.sort_by(|a, b| {
        (&a.path, a.acquired, a.line, a.column).cmp(&(&b.path, b.acquired, b.line, b.column))
    });
    WorkspaceResolution {
        reached: parsed.into_keys().collect(),
        calls: out,
        types,
        releases,
    }
}

/// G157: where a resource bound by a `let` is given back (`std::mem::drop`, `JoinHandle::join`
/// or the end of the holder's block), with the position of the call that acquired it -- the
/// position its `PathCallResolution` is recorded at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseResolution {
    pub path: String,
    pub acquired: (usize, usize),
    pub holder: String,
    pub line: usize,
    pub column: usize,
    pub kind: atlas_core::ResourceKind,
    pub release: atlas_core::ResourceRelease,
}

/// Every `let` of a body in source order with the block it is a statement of and that block's
/// end (closure bodies included, nested items not).
fn lets_of(block: &syn::Block) -> Vec<(&syn::Local, &syn::Block, (usize, usize))> {
    struct Lets<'b> {
        blocks: Vec<&'b syn::Block>,
        found: Vec<(&'b syn::Local, &'b syn::Block, (usize, usize))>,
    }
    impl<'b> Visit<'b> for Lets<'b> {
        fn visit_block(&mut self, block: &'b syn::Block) {
            self.blocks.push(block);
            syn::visit::visit_block(self, block);
            self.blocks.pop();
        }
        fn visit_local(&mut self, local: &'b syn::Local) {
            if let Some(block) = self.blocks.last() {
                self.found
                    .push((local, block, start(block.brace_token.span.close())));
            }
            syn::visit::visit_local(self, local);
        }
        fn visit_item(&mut self, _: &'b syn::Item) {}
    }
    let mut lets = Lets {
        blocks: Vec::new(),
        found: Vec::new(),
    };
    lets.visit_block(block);
    lets.found
}

/// Formatting and assertion macros take their arguments by reference.
const BORROWING_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "eprint",
    "eprintln",
    "format",
    "panic",
    "print",
    "println",
    "write",
    "writeln",
];

/// G157: whether a method of that name takes a std resource holder by value: `Read::take`,
/// `Read::bytes`, `Read::chain`, `Into::into`, `TryInto::try_into` and every `into_*` conversion
/// consume a file, a socket or a handle (a guard's methods reach its data through `Deref`, where
/// these names are treated the same way, conservatively). A workspace trait method taking `self`
/// by value is not seen here, which is one reason a scope-end release stays INFERRED.
fn consumes_receiver(method: &str) -> bool {
    matches!(method, "take" | "bytes" | "chain" | "into" | "try_into")
        || method.starts_with("into_")
}

/// G157: the uses of a resource holder after its `let`, in the block it lives in.
struct HolderUses<'w, 'a> {
    walker: &'w CallWalker<'a>,
    name: &'w str,
    after: (usize, usize),
    until: (usize, usize),
    /// Nested blocks, closures and match arms entered: a release there is conditional.
    depth: usize,
    moved: bool,
    /// Inside a `move` closure: every mention takes the holder.
    taking: bool,
    released: Vec<(atlas_core::ResourceRelease, (usize, usize))>,
}

impl HolderUses<'_, '_> {
    fn is_holder(&self, expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Paren(inner) => self.is_holder(&inner.expr),
            syn::Expr::Path(path) if path.qself.is_none() => {
                let at = start(path.span());
                path.path.is_ident(self.name) && at > self.after && at < self.until
            }
            _ => false,
        }
    }

    fn release(&mut self, how: atlas_core::ResourceRelease, at: (usize, usize)) {
        if self.depth == 0 {
            self.released.push((how, at));
        } else {
            self.moved = true;
        }
    }
}

impl<'ast> Visit<'ast> for HolderUses<'_, '_> {
    fn visit_expr(&mut self, expr: &'ast syn::Expr) {
        if self.taking {
            if self.is_holder(expr) {
                self.moved = true;
            } else {
                syn::visit::visit_expr(self, expr);
            }
            return;
        }
        match expr {
            syn::Expr::Reference(r) if self.is_holder(&r.expr) => {}
            syn::Expr::Field(f) if self.is_holder(&f.base) => {}
            syn::Expr::Unary(u)
                if matches!(u.op, syn::UnOp::Deref(_)) && self.is_holder(&u.expr) => {}
            syn::Expr::Index(i) if self.is_holder(&i.expr) => self.visit_expr(&i.index),
            syn::Expr::MethodCall(call) if self.is_holder(&call.receiver) => {
                let at = start(call.method.span());
                match self.walker.method_outcome(call) {
                    Some((_, PathCallOutcome::External(std)))
                        if std == "std::thread::JoinHandle::join" =>
                    {
                        self.release(atlas_core::ResourceRelease::Join, at);
                    }
                    _ if consumes_receiver(&call.method.to_string()) => self.moved = true,
                    _ => {}
                }
                for arg in &call.args {
                    self.visit_expr(arg);
                }
            }
            syn::Expr::Call(call) if call.args.len() == 1 && self.is_holder(&call.args[0]) => {
                let dropped = match call.func.as_ref() {
                    syn::Expr::Path(path) => matches!(
                        self.walker.resolve_call(path),
                        PathCallOutcome::External(std) if std == "std::mem::drop"
                    )
                    .then(|| {
                        start(
                            path.path
                                .segments
                                .last()
                                .map_or(path.span(), |s| s.ident.span()),
                        )
                    }),
                    _ => None,
                };
                match dropped {
                    Some(at) => self.release(atlas_core::ResourceRelease::ExplicitDrop, at),
                    None => self.moved = true,
                }
            }
            syn::Expr::Closure(closure) => {
                self.depth += 1;
                // A `move` closure naming the holder takes it, however it uses it.
                let taking = self.taking;
                self.taking |= closure.capture.is_some();
                syn::visit::visit_expr_closure(self, closure);
                self.taking = taking;
                self.depth -= 1;
            }
            syn::Expr::Path(_) if self.is_holder(expr) => self.moved = true,
            _ => syn::visit::visit_expr(self, expr),
        }
    }
    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.depth += 1;
        syn::visit::visit_block(self, block);
        self.depth -= 1;
    }
    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        self.depth += 1;
        syn::visit::visit_arm(self, arm);
        self.depth -= 1;
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let mut names = BTreeSet::new();
        stream_idents(mac.tokens.clone(), &mut names);
        if !names.contains(self.name) {
            return;
        }
        let borrowing = mac
            .path
            .segments
            .last()
            .is_some_and(|s| BORROWING_MACROS.contains(&s.ident.to_string().as_str()));
        if self.taking || !borrowing {
            self.moved = true;
        }
    }
    fn visit_item(&mut self, _: &'ast syn::Item) {}
}

type ModId = usize;
type TypeId = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Def {
    Module(ModId),
    Fn(FnTarget),
    Type(TypeId),
    /// A tuple/unit struct or enum variant constructor: a call, but not of a function.
    Ctor,
    /// A const/static or other non-function value.
    Value,
    /// Outside the workspace, with its path when it is rooted at a standard crate (else empty).
    External(Vec<String>),
    Ambiguous,
    /// A named import this pass could not resolve: it still binds its name, so a lookup must not
    /// fall through to an outer scope's definition of the same name.
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Origin {
    Glob,
    Named,
    Item,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Vis {
    Public,
    Crate(usize),
    /// Visible inside this module (and every module nested in it).
    Within(ModId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    def: Def,
    vis: Vis,
    origin: Origin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ns {
    Types,
    Values,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Scope {
    types: BTreeMap<String, Entry>,
    values: BTreeMap<String, Entry>,
}

impl Scope {
    fn ns(&self, ns: Ns) -> &BTreeMap<String, Entry> {
        match ns {
            Ns::Types => &self.types,
            Ns::Values => &self.values,
        }
    }

    fn ns_mut(&mut self, ns: Ns) -> &mut BTreeMap<String, Entry> {
        match ns {
            Ns::Types => &mut self.types,
            Ns::Values => &mut self.values,
        }
    }

    /// Adds `entry` under `name`: a higher-priority origin shadows, an equal-priority different
    /// definition makes the name ambiguous.
    fn insert(&mut self, ns: Ns, name: &str, entry: Entry) {
        let slot = self.ns_mut(ns);
        match slot.get_mut(name) {
            None => {
                slot.insert(name.to_owned(), entry);
            }
            Some(existing) if entry.origin > existing.origin => *existing = entry,
            Some(existing) if entry.origin == existing.origin && existing.def != entry.def => {
                existing.def = Def::Ambiguous;
            }
            Some(_) => {}
        }
    }
}

#[derive(Debug, Clone)]
struct Import {
    segments: Vec<String>,
    leading_colon: bool,
    /// `None` for a glob.
    name: Option<String>,
    /// `use a::{self}` imports the module `a` itself (types namespace only).
    self_import: bool,
    vis: Vis,
}

#[derive(Debug, Clone)]
struct Module {
    krate: usize,
    /// The enclosing module (for a block scope: the module whose `self`/`super` it uses).
    parent: Option<ModId>,
    /// The scope names fall back to: a block's enclosing block or module; `None` for a module
    /// (a module never sees its parent's names).
    lexical_parent: Option<ModId>,
    is_block: bool,
    items: Scope,
    scope: Scope,
    imports: Vec<Import>,
    /// Names may exist that this pass cannot see (an unbounded item macro, an unresolvable glob,
    /// a missing or unparsable module file).
    open: bool,
    /// G145: the names this module's own item-position `macro_rules!` invocations may define.
    macro_names: BTreeSet<String>,
    /// G145: `macro_names` and those a glob import brings in -- a lookup of one of these names is
    /// uncertain in this module, every other name is not.
    shadow: BTreeSet<String>,
    /// The module's own name (empty for a crate root and a block).
    name: String,
}

#[derive(Debug, Clone)]
struct TypeDef {
    scope: ModId,
    name: String,
    variants: BTreeSet<String>,
    is_trait: bool,
    alias_of: Option<(Vec<String>, bool)>,
    /// A type alias whose meaning a bare path cannot carry (generic, or naming a type with
    /// arguments or a non-path type): never canonicalized.
    opaque_alias: bool,
}

/// G142: a function's declared output as spelled, with what is needed to read it: the scope it
/// is spelled in, the impl whose `Self` it may name, and the generic names in force.
#[derive(Clone)]
struct FnOutput {
    scope: ModId,
    impl_index: Option<usize>,
    generics: BTreeSet<String>,
    ty: syn::Type,
}

#[derive(Debug, Clone)]
struct ImplDef {
    /// G142: the implemented trait's name (its path's last segment), for a trait impl.
    trait_name: Option<String>,
    scope: ModId,
    self_path: Vec<String>,
    self_leading_colon: bool,
    is_trait_impl: bool,
    fns: Vec<ImplFn>,
    self_type: Option<TypeId>,
    /// The impl's generics, where clause and self-type arguments as spelled: two impls of one
    /// type are interchangeable for method lookup only when these are identical.
    shape: String,
}

#[derive(Debug, Clone)]
struct ImplFn {
    name: String,
    target: FnTarget,
    receiver: Option<Receiver>,
}

/// A method's `self` parameter: by value, `&self` or `&mut self`. A typed receiver
/// (`self: Box<Self>`) is not modeled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Receiver {
    Value,
    Ref,
    RefMut,
}

fn receiver_of(sig: &syn::Signature) -> Option<Receiver> {
    match &sig.receiver()?.kind {
        syn::ReceiverKind::Value => Some(Receiver::Value),
        syn::ReceiverKind::Reference(_, _, None) => Some(Receiver::Ref),
        syn::ReceiverKind::Reference(_, _, Some(_)) => Some(Receiver::RefMut),
        _ => None,
    }
}

/// G151: lifetimes are erased. Which method a call dispatches to never depends on a lifetime,
/// so `impl<'a> Walker<'a>` is shaped like `impl Walker` (and is plain when nothing else is
/// generic); a lifetime predicate in a where clause is dropped with it.
///
/// G156 (replay R4, salsa): the shape is the set of types the impl applies to, not its spelling.
/// Generic parameters are renamed by position, every bound -- inline (`impl<C: Config>`) or in
/// the where clause (`where C: Config`), `A + B` split into one predicate each -- becomes a sorted
/// predicate, so `impl<C: Config> S<C>` and `impl<T> S<T> where T: Config` share a shape and
/// their inherent methods are one namespace, as rustc sees them. Different bound sets still
/// differ.
fn impl_shape(item: &syn::ItemImpl) -> String {
    use quote::ToTokens;
    let mut names: BTreeMap<String, String> = BTreeMap::new();
    let mut params: Vec<(String, Option<syn::Type>)> = Vec::new();
    let mut bounds: Vec<(proc_macro2::TokenStream, proc_macro2::TokenStream)> = Vec::new();
    let mut push_bounds =
        |subject: proc_macro2::TokenStream,
         list: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>| {
            for bound in list {
                if !matches!(bound, syn::TypeParamBound::Lifetime(_)) {
                    bounds.push((subject.clone(), bound.to_token_stream()));
                }
            }
        };
    for param in &item.generics.params {
        match param {
            syn::GenericParam::Lifetime(_) => {}
            syn::GenericParam::Type(ty) => {
                let positional = format!("P{}", names.len());
                names.insert(ty.ident.to_string(), positional.clone());
                params.push((positional, None));
                push_bounds(ty.ident.to_token_stream(), &ty.bounds);
            }
            syn::GenericParam::Const(konst) => {
                let positional = format!("P{}", names.len());
                names.insert(konst.ident.to_string(), positional.clone());
                params.push((positional, Some(konst.ty.clone())));
            }
        }
    }
    if let Some(clause) = &item.generics.where_clause {
        for predicate in &clause.predicates {
            if let syn::WherePredicate::Type(predicate) = predicate {
                let mut subject = proc_macro2::TokenStream::new();
                if let Some(lifetimes) = &predicate.lifetimes {
                    lifetimes.to_tokens(&mut subject);
                }
                predicate.bounded_ty.to_tokens(&mut subject);
                push_bounds(subject, &predicate.bounds);
            }
        }
    }
    let rename = |tokens: proc_macro2::TokenStream| renamed(tokens, &names).to_string();
    let predicates: BTreeSet<String> = bounds
        .into_iter()
        .map(|(subject, bound)| format!("{} : {}", rename(subject), rename(bound)))
        .collect();
    let arguments = match item.self_ty.as_ref() {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| rename(without_lifetimes(&segment.arguments).to_token_stream()))
            .unwrap_or_default(),
        other => rename(other.to_token_stream()),
    };
    let generics = if params.is_empty() {
        String::new()
    } else {
        let list: Vec<String> = params
            .iter()
            .map(|(name, ty)| match ty {
                Some(ty) => format!("const {name} : {}", rename(ty.to_token_stream())),
                None => name.clone(),
            })
            .collect();
        format!("< {} >", list.join(" , "))
    };
    let clause = if predicates.is_empty() {
        String::new()
    } else {
        format!(
            "where {}",
            predicates.into_iter().collect::<Vec<_>>().join(" , ")
        )
    };
    format!("{generics} {clause} | {arguments}")
}

/// `tokens` with every identifier in `names` replaced by its positional name.
fn renamed(
    tokens: proc_macro2::TokenStream,
    names: &BTreeMap<String, String>,
) -> proc_macro2::TokenStream {
    tokens
        .into_iter()
        .map(|tree| match tree {
            proc_macro2::TokenTree::Ident(ident) => match names.get(&ident.to_string()) {
                Some(name) => {
                    proc_macro2::TokenTree::Ident(proc_macro2::Ident::new(name, ident.span()))
                }
                None => proc_macro2::TokenTree::Ident(ident),
            },
            proc_macro2::TokenTree::Group(group) => {
                let mut inner =
                    proc_macro2::Group::new(group.delimiter(), renamed(group.stream(), names));
                inner.set_span(group.span());
                proc_macro2::TokenTree::Group(inner)
            }
            other => other,
        })
        .collect()
}

/// Path arguments with every lifetime argument removed; `None` when only lifetimes were given.
fn without_lifetimes(arguments: &syn::PathArguments) -> syn::PathArguments {
    match arguments {
        syn::PathArguments::AngleBracketed(angle) => {
            let mut kept = angle.clone();
            kept.args = angle
                .args
                .iter()
                .filter(|a| !matches!(a, syn::GenericArgument::Lifetime(_)))
                .cloned()
                .collect();
            if kept.args.is_empty() {
                syn::PathArguments::None
            } else {
                syn::PathArguments::AngleBracketed(kept)
            }
        }
        other => other.clone(),
    }
}

#[derive(Default)]
struct DefMap {
    modules: Vec<Module>,
    types: Vec<TypeDef>,
    impls: Vec<ImplDef>,
    /// G140: each workspace trait's own methods (supertraits' are not followed).
    trait_fns: BTreeMap<TypeId, Vec<ImplFn>>,
    /// G142: each workspace function's declared output, by its target's (path, line, column).
    fn_outputs: BTreeMap<(String, usize, usize), FnOutput>,
    /// G143: the named fields of each workspace struct without generics, as declared.
    struct_fields: BTreeMap<TypeId, BTreeMap<String, (ModId, syn::Type)>>,
    /// An inherent impl on a trait object (`impl dyn Tr {}`) exists somewhere: its methods would
    /// compete with the trait's, so no call through `dyn` is claimed.
    dyn_inherent_impl: bool,
    /// Each parsed file's top module.
    file_modules: BTreeMap<String, ModId>,
    /// Block scopes by (file, line, column) of the block's opening brace.
    blocks: BTreeMap<(String, usize, usize), ModId>,
    crate_roots: BTreeMap<usize, ModId>,
    crate_packages: BTreeMap<usize, String>,
    /// G145: every workspace `macro_rules!` definition and item-position macro invocation.
    macro_defs: Vec<MacroDef>,
    macro_calls: Vec<MacroCall>,
}

/// G145: a workspace `macro_rules!` definition and the names its expansions may define.
#[derive(Debug, Clone)]
struct MacroDef {
    module: ModId,
    file: String,
    line: usize,
    name: String,
    exported: bool,
    names: MacroNames,
}

/// G145: what an expansion of a `macro_rules!` may define, read from its transcribers without
/// expanding: an item is named by the identifier after its keyword, which is either spelled in the
/// transcriber or substituted for a metavariable from the invocation's tokens. Anything that could
/// name items another way (an import, a macro outside the expression allowlist, an attribute
/// outside the attribute allowlist) leaves the expansion unbounded.
#[derive(Debug, Clone, Default)]
struct MacroNames {
    literal: BTreeSet<String>,
    from_invocation: bool,
    unbounded: bool,
}

/// G145: an item-position macro invocation, resolved once every definition is collected.
#[derive(Debug, Clone)]
struct MacroCall {
    module: ModId,
    file: String,
    line: usize,
    segments: Vec<String>,
    leading_colon: bool,
    idents: BTreeSet<String>,
    attributes_allowed: bool,
}

/// Keywords whose next identifier names an item.
const ITEM_KEYWORDS: [&str; 9] = [
    "struct", "enum", "union", "trait", "type", "fn", "mod", "const", "static",
];

/// Standard macros that expand to expressions or nothing: never to a named item.
const EXPRESSION_MACROS: [&str; 24] = [
    "assert",
    "assert_eq",
    "assert_ne",
    "cfg",
    "column",
    "concat",
    "dbg",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "env",
    "file",
    "format",
    "format_args",
    "line",
    "matches",
    "panic",
    "print",
    "println",
    "stringify",
    "todo",
    "unimplemented",
    "unreachable",
    "vec",
];

/// Attributes that never introduce a name, and the derives whose output names nothing (std's, and
/// serde's, which expand to anonymous `const _` items).
const ATTRIBUTES: [&str; 14] = [
    "allow",
    "cfg",
    "cfg_attr",
    "default",
    "deny",
    "derive",
    "doc",
    "expect",
    "inline",
    "must_use",
    "non_exhaustive",
    "repr",
    "serde",
    "warn",
];
const DERIVES: [&str; 12] = [
    "Clone",
    "Copy",
    "Debug",
    "Default",
    "Deserialize",
    "Eq",
    "Hash",
    "Ord",
    "PartialEq",
    "PartialOrd",
    "Serialize",
    "serde",
];

fn ident_text(ident: &proc_macro2::Ident) -> String {
    let text = ident.to_string();
    text.strip_prefix("r#").map_or(text.clone(), str::to_owned)
}

/// Every identifier in a token stream, groups included.
fn stream_idents(tokens: proc_macro2::TokenStream, out: &mut BTreeSet<String>) {
    for token in tokens {
        match token {
            proc_macro2::TokenTree::Ident(ident) => {
                out.insert(ident_text(&ident));
            }
            proc_macro2::TokenTree::Group(group) => stream_idents(group.stream(), out),
            _ => {}
        }
    }
}

/// Whether every `#[..]` in a token stream is an allowlisted attribute (a leading metavariable,
/// `#[$meta]`, stands for the invocation's attributes, which are checked there).
fn attributes_allowed(tokens: proc_macro2::TokenStream) -> bool {
    use proc_macro2::TokenTree;
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Punct(punct) if punct.as_char() == '#' => {
                let Some(TokenTree::Group(group)) = tokens.get(index + 1) else {
                    continue;
                };
                if group.delimiter() != proc_macro2::Delimiter::Bracket {
                    continue;
                }
                let inner: Vec<TokenTree> = group.stream().into_iter().collect();
                match inner.first() {
                    Some(TokenTree::Punct(punct)) if punct.as_char() == '$' => {}
                    Some(TokenTree::Ident(ident)) => {
                        let name = ident_text(ident);
                        if !ATTRIBUTES.contains(&name.as_str()) {
                            return false;
                        }
                        if name == "derive" {
                            let mut derived = BTreeSet::new();
                            for part in &inner[1..] {
                                stream_idents(part.clone().into(), &mut derived);
                            }
                            if derived.iter().any(|d| !DERIVES.contains(&d.as_str())) {
                                return false;
                            }
                        }
                    }
                    _ => return false,
                }
            }
            TokenTree::Group(group) if !attributes_allowed(group.stream()) => return false,
            _ => {}
        }
    }
    true
}

/// The names a transcriber may define (see `MacroNames`).
fn transcriber_names(tokens: proc_macro2::TokenStream, names: &mut MacroNames) {
    use proc_macro2::TokenTree;
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Ident(ident) => {
                let text = ident_text(ident);
                let previous = index
                    .checked_sub(1)
                    .map(|i| tokens[i].to_string())
                    .unwrap_or_default();
                // `$name` is a metavariable and `'static` a lifetime, never a keyword.
                let after_dollar = previous == "$";
                let lifetime = previous == "'";
                if lifetime {
                    continue;
                }
                if text == "use" || text == "extern" {
                    names.unbounded = true;
                } else if ITEM_KEYWORDS.contains(&text.as_str()) && !after_dollar {
                    let mut next = index + 1;
                    while tokens.get(next).is_some_and(|t| t.to_string() == "mut") {
                        next += 1;
                    }
                    match tokens.get(next) {
                        Some(TokenTree::Ident(name)) => {
                            names.literal.insert(ident_text(name));
                        }
                        Some(TokenTree::Punct(punct)) if punct.as_char() == '$' => {
                            names.from_invocation = true;
                        }
                        _ => {}
                    }
                }
                let bang = tokens.get(index + 1).is_some_and(|t| t.to_string() == "!");
                if bang
                    && text != "macro_rules"
                    && (after_dollar || !EXPRESSION_MACROS.contains(&text.as_str()))
                {
                    names.unbounded = true;
                }
            }
            TokenTree::Group(group) => transcriber_names(group.stream(), names),
            _ => {}
        }
    }
}

/// The names a `macro_rules!` body's arms may define: every transcriber, since the arm an
/// invocation selects is not decided here.
fn macro_rules_names(body: proc_macro2::TokenStream) -> MacroNames {
    use proc_macro2::TokenTree;
    let mut names = MacroNames::default();
    let tokens: Vec<TokenTree> = body.into_iter().collect();
    let mut index = 0;
    let mut arms = 0;
    while index < tokens.len() {
        // matcher `=` `>` transcriber [`;`]
        let (Some(TokenTree::Group(_)), Some(TokenTree::Punct(eq)), Some(TokenTree::Punct(gt))) = (
            tokens.get(index),
            tokens.get(index + 1),
            tokens.get(index + 2),
        ) else {
            names.unbounded = true;
            return names;
        };
        let Some(TokenTree::Group(transcriber)) = tokens.get(index + 3) else {
            names.unbounded = true;
            return names;
        };
        if eq.as_char() != '=' || gt.as_char() != '>' {
            names.unbounded = true;
            return names;
        }
        transcriber_names(transcriber.stream(), &mut names);
        if !attributes_allowed(transcriber.stream()) {
            names.unbounded = true;
        }
        arms += 1;
        index += 4;
        if tokens.get(index).is_some_and(|t| t.to_string() == ";") {
            index += 1;
        }
    }
    if arms == 0 {
        names.unbounded = true;
    }
    names
}

fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

/// `dir/rest`, or `rest` at the root.
pub fn join(dir: &str, rest: &str) -> String {
    if dir.is_empty() {
        rest.to_owned()
    } else {
        format!("{dir}/{rest}")
    }
}

/// Normalizes `a/b/../c` to `a/c` (a `#[path]` attribute may climb).
fn normalize(path: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out.join("/")
}

fn path_attr(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| match &attr.meta {
        syn::Meta::NameValue(nv) if nv.path.is_ident("path") => match &nv.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) => Some(s.value()),
            _ => None,
        },
        _ => None,
    })
}

/// G154: the file an item-position `include!("path")` names, relative to the including file's
/// directory, when its argument is one string literal; `None` for a computed path.
fn included_path(file: &str, mac: &syn::Macro) -> Option<String> {
    let literal: syn::LitStr = syn::parse2(mac.tokens.clone()).ok()?;
    Some(normalize(&join(dir_of(file), &literal.value())))
}

fn start(span: proc_macro2::Span) -> (usize, usize) {
    let start = span.start();
    (start.line, start.column)
}

impl DefMap {
    fn new_module(&mut self, krate: usize, parent: Option<ModId>, lexical: Option<ModId>) -> ModId {
        self.modules.push(Module {
            krate,
            parent,
            lexical_parent: lexical,
            is_block: lexical.is_some(),
            items: Scope::default(),
            scope: Scope::default(),
            imports: Vec::new(),
            open: false,
            macro_names: BTreeSet::new(),
            shadow: BTreeSet::new(),
            name: String::new(),
        });
        self.modules.len() - 1
    }

    /// The module whose `self`/`super` a scope uses (a block uses its enclosing module's).
    fn module_of(&self, mut scope: ModId) -> ModId {
        while self.modules[scope].is_block {
            scope = self.modules[scope]
                .parent
                .expect("a block scope has an enclosing module");
        }
        scope
    }

    fn is_within(&self, mut scope: ModId, ancestor: ModId) -> bool {
        loop {
            if scope == ancestor {
                return true;
            }
            match self.modules[scope]
                .lexical_parent
                .or(self.modules[scope].parent)
            {
                Some(next) => scope = next,
                None => return false,
            }
        }
    }

    fn visible(&self, vis: &Vis, from: ModId) -> bool {
        match vis {
            Vis::Public => true,
            Vis::Crate(krate) => self.modules[from].krate == *krate,
            Vis::Within(owner) => self.is_within(from, *owner),
        }
    }

    fn vis(&self, vis: &syn::Visibility, owner: ModId) -> Vis {
        match vis {
            syn::Visibility::Public(_) => Vis::Public,
            syn::Visibility::Inherited => Vis::Within(owner),
            syn::Visibility::Restricted(restricted) => {
                let path = &restricted.path;
                if path.is_ident("self") {
                    Vis::Within(self.module_of(owner))
                } else if path.is_ident("super") {
                    let module = self.module_of(owner);
                    Vis::Within(self.modules[module].parent.unwrap_or(module))
                } else {
                    // `pub(crate)` and `pub(in path)` (approximated by the whole crate).
                    Vis::Crate(self.modules[owner].krate)
                }
            }
        }
    }

    fn collect_crate(
        &mut self,
        krate: usize,
        input: &CrateInput,
        sources: &BTreeMap<String, String>,
        parsed: &mut BTreeMap<String, syn::File>,
    ) {
        let root = self.new_module(krate, None, None);
        self.crate_roots.insert(krate, root);
        // The package a crate belongs to: the directory holding `src/` (or the build script).
        let package = match input.root.split_once("/src/") {
            Some((package, _)) => package.to_owned(),
            None if input.root.starts_with("src/") => String::new(),
            None => dir_of(&input.root).to_owned(),
        };
        self.crate_packages.insert(krate, package);
        // A crate root is a "mod-rs" file: its children live next to it.
        let dir = dir_of(&input.root).to_owned();
        self.collect_file(root, &input.root, &dir, sources, parsed);
    }

    fn collect_file(
        &mut self,
        module: ModId,
        file: &str,
        child_dir: &str,
        sources: &BTreeMap<String, String>,
        parsed: &mut BTreeMap<String, syn::File>,
    ) {
        let Some(ast) = sources
            .get(file)
            .filter(|text| {
                super::max_structural_recursion_risk(text) <= super::MAX_STRUCTURAL_RECURSION_RISK
            })
            .and_then(|text| syn::parse_file(text).ok())
        else {
            self.modules[module].open = true;
            return;
        };
        if self.file_modules.contains_key(file) {
            // The same file reached twice (a `#[path]` cycle or two crates): resolve it once.
            self.modules[module].open = true;
            return;
        }
        self.file_modules.insert(file.to_owned(), module);
        self.collect_items(module, file, child_dir, false, &ast.items, sources, parsed);
        parsed.insert(file.to_owned(), ast);
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_items(
        &mut self,
        module: ModId,
        file: &str,
        child_dir: &str,
        inline: bool,
        items: &[syn::Item],
        sources: &BTreeMap<String, String>,
        parsed: &mut BTreeMap<String, syn::File>,
    ) {
        for item in items {
            match item {
                syn::Item::Fn(item_fn) => {
                    let (line, column) = start(item_fn.span());
                    let name = item_fn.sig.ident.to_string();
                    let target = FnTarget {
                        path: file.to_owned(),
                        line,
                        column,
                        name: name.clone(),
                    };
                    if let syn::ReturnType::Type(_, ty) = &item_fn.sig.output {
                        self.fn_outputs.insert(
                            (file.to_owned(), line, column),
                            FnOutput {
                                scope: module,
                                impl_index: None,
                                generics: generic_names(&item_fn.sig.generics),
                                ty: ty.as_ref().clone(),
                            },
                        );
                    }
                    self.add_item(module, Ns::Values, &name, Def::Fn(target), &item_fn.vis);
                    self.collect_block(module, file, &item_fn.block);
                }
                syn::Item::Struct(item) => {
                    let name = item.ident.to_string();
                    let ty = self.new_type(module, &name, BTreeSet::new(), false, None);
                    self.add_item(module, Ns::Types, &name, Def::Type(ty), &item.vis);
                    if let syn::Fields::Named(named) = &item.fields
                        && item.generics.params.is_empty()
                    {
                        let fields = named
                            .named
                            .iter()
                            .filter_map(|f| {
                                Some((f.ident.as_ref()?.to_string(), (module, f.ty.clone())))
                            })
                            .collect();
                        self.struct_fields.insert(ty, fields);
                    }
                    if !matches!(item.fields, syn::Fields::Named(_)) {
                        self.add_item(module, Ns::Values, &name, Def::Ctor, &item.vis);
                    }
                }
                syn::Item::Enum(item) => {
                    let variants = item.variants.iter().map(|v| v.ident.to_string()).collect();
                    let ty = self.new_type(module, &item.ident.to_string(), variants, false, None);
                    self.add_item(
                        module,
                        Ns::Types,
                        &item.ident.to_string(),
                        Def::Type(ty),
                        &item.vis,
                    );
                }
                syn::Item::Union(item) => {
                    let ty = self.new_type(
                        module,
                        &item.ident.to_string(),
                        BTreeSet::new(),
                        false,
                        None,
                    );
                    self.add_item(
                        module,
                        Ns::Types,
                        &item.ident.to_string(),
                        Def::Type(ty),
                        &item.vis,
                    );
                }
                syn::Item::Trait(item) => {
                    let ty =
                        self.new_type(module, &item.ident.to_string(), BTreeSet::new(), true, None);
                    self.add_item(
                        module,
                        Ns::Types,
                        &item.ident.to_string(),
                        Def::Type(ty),
                        &item.vis,
                    );
                    let mut fns = Vec::new();
                    for trait_item in &item.items {
                        if let syn::TraitItem::Fn(method) = trait_item {
                            let (line, column) = start(method.span());
                            let name = method.sig.ident.to_string();
                            fns.push(ImplFn {
                                name: name.clone(),
                                target: FnTarget {
                                    path: file.to_owned(),
                                    line,
                                    column,
                                    name,
                                },
                                receiver: receiver_of(&method.sig),
                            });
                            if let Some(block) = &method.default {
                                self.collect_block(module, file, block);
                            }
                        }
                    }
                    self.trait_fns.insert(ty, fns);
                }
                syn::Item::Type(item) => {
                    let alias = match item.ty.as_ref() {
                        syn::Type::Path(path) if path.qself.is_none() => Some((
                            path.path
                                .segments
                                .iter()
                                .map(|s| s.ident.to_string())
                                .collect(),
                            path.path.leading_colon.is_some(),
                        )),
                        _ => None,
                    };
                    let opaque = !item.generics.params.is_empty()
                        || match item.ty.as_ref() {
                            syn::Type::Path(path) => {
                                path.path.segments.iter().any(|s| !s.arguments.is_empty())
                            }
                            _ => true,
                        };
                    let ty = self.new_type(
                        module,
                        &item.ident.to_string(),
                        BTreeSet::new(),
                        false,
                        alias,
                    );
                    self.types[ty].opaque_alias = opaque;
                    self.add_item(
                        module,
                        Ns::Types,
                        &item.ident.to_string(),
                        Def::Type(ty),
                        &item.vis,
                    );
                }
                syn::Item::Const(item) => {
                    self.add_item(
                        module,
                        Ns::Values,
                        &item.ident.to_string(),
                        Def::Value,
                        &item.vis,
                    );
                }
                syn::Item::Static(item) => {
                    self.add_item(
                        module,
                        Ns::Values,
                        &item.ident.to_string(),
                        Def::Value,
                        &item.vis,
                    );
                }
                syn::Item::Mod(item) => {
                    let name = item.ident.to_string();
                    let krate = self.modules[module].krate;
                    let child = self.new_module(krate, Some(module), None);
                    self.modules[child].name.clone_from(&name);
                    self.add_item(module, Ns::Types, &name, Def::Module(child), &item.vis);
                    let explicit = path_attr(&item.attrs);
                    // `#[path]` is relative to the declaring file's directory at file level, and
                    // to the inline module's directory inside an inline module.
                    let attr_base = if inline { child_dir } else { dir_of(file) };
                    match &item.content {
                        Some((_, items)) => {
                            let dir = match &explicit {
                                Some(path) => normalize(&join(attr_base, path)),
                                None => join(child_dir, &name),
                            };
                            self.collect_items(child, file, &dir, true, items, sources, parsed);
                        }
                        None => {
                            let located = match &explicit {
                                Some(path) => {
                                    let path = normalize(&join(attr_base, path));
                                    sources.contains_key(&path).then_some(path)
                                }
                                None => {
                                    let flat = join(child_dir, &format!("{name}.rs"));
                                    let nested = join(child_dir, &format!("{name}/mod.rs"));
                                    match (
                                        sources.contains_key(&flat),
                                        sources.contains_key(&nested),
                                    ) {
                                        (true, false) => Some(flat),
                                        (false, true) => Some(nested),
                                        _ => None,
                                    }
                                }
                            };
                            match located {
                                Some(path) => {
                                    // Children of a mod-rs file live next to it; of `x.rs`, in `x/`.
                                    let dir = if path.ends_with("/mod.rs") || explicit.is_some() {
                                        dir_of(&path).to_owned()
                                    } else {
                                        path.trim_end_matches(".rs").to_owned()
                                    };
                                    self.collect_file(child, &path, &dir, sources, parsed);
                                }
                                None => self.modules[child].open = true,
                            }
                        }
                    }
                }
                syn::Item::Use(item) => {
                    let vis = self.vis(&item.vis, module);
                    let leading = item.leading_colon.is_some();
                    let mut imports = Vec::new();
                    flatten_use(&item.tree, Vec::new(), leading, &vis, &mut imports);
                    self.modules[module].imports.extend(imports);
                }
                syn::Item::ExternCrate(item) => {
                    let name = item
                        .rename
                        .as_ref()
                        .map_or_else(|| item.ident.to_string(), |(_, rename)| rename.to_string());
                    let def = self.external_root(&item.ident.to_string());
                    self.add_item(module, Ns::Types, &name, def, &item.vis);
                }
                syn::Item::Impl(item) => self.collect_impl(module, file, item),
                // G145: an item-position macro defines what its expansion names; a workspace
                // `macro_rules!` bounds that (`resolve_macro_calls`), anything else may define any
                // name.
                // G154 (replay R3): `include!("file.rs")` at item position splices that file's
                // items into this module, as rustc does. Only a literal path relative to this
                // file is followed; a computed one (`concat!`, `env!`) stays a macro call.
                syn::Item::Macro(item)
                    if item.ident.is_none()
                        && item.mac.path.is_ident("include")
                        && included_path(file, &item.mac)
                            .is_some_and(|path| sources.contains_key(&path)) =>
                {
                    if let Some(path) = included_path(file, &item.mac) {
                        self.collect_file(module, &path, child_dir, sources, parsed);
                    }
                }
                // G154 (replay R3): functions and statics declared by `extern "ABI" { ... }`.
                syn::Item::ForeignMod(foreign) => {
                    for foreign_item in &foreign.items {
                        match foreign_item {
                            syn::ForeignItem::Fn(item_fn) => {
                                let (line, column) = start(item_fn.span());
                                let name = item_fn.sig.ident.to_string();
                                let target = FnTarget {
                                    path: file.to_owned(),
                                    line,
                                    column,
                                    name: name.clone(),
                                };
                                self.add_item(
                                    module,
                                    Ns::Values,
                                    &name,
                                    Def::Fn(target),
                                    &item_fn.vis,
                                );
                            }
                            syn::ForeignItem::Static(item_static) => {
                                let name = item_static.ident.to_string();
                                self.add_item(
                                    module,
                                    Ns::Values,
                                    &name,
                                    Def::Value,
                                    &item_static.vis,
                                );
                            }
                            _ => {}
                        }
                    }
                }
                syn::Item::Macro(item) if item.ident.is_none() => {
                    let mut idents = BTreeSet::new();
                    stream_idents(item.mac.tokens.clone(), &mut idents);
                    self.macro_calls.push(MacroCall {
                        module,
                        file: file.to_owned(),
                        line: start(item.span()).0,
                        segments: item
                            .mac
                            .path
                            .segments
                            .iter()
                            .map(|s| s.ident.to_string())
                            .collect(),
                        leading_colon: item.mac.path.leading_colon.is_some(),
                        idents,
                        attributes_allowed: attributes_allowed(item.mac.tokens.clone()),
                    });
                }
                syn::Item::Macro(item) if item.mac.path.is_ident("macro_rules") => {
                    if let Some(ident) = &item.ident {
                        self.macro_defs.push(MacroDef {
                            module,
                            file: file.to_owned(),
                            line: start(item.span()).0,
                            name: ident.to_string(),
                            exported: item.attrs.iter().any(|a| a.path().is_ident("macro_export")),
                            names: macro_rules_names(item.mac.tokens.clone()),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_impl(&mut self, scope: ModId, file: &str, item: &syn::ItemImpl) {
        let (self_path, self_leading_colon) = match item.self_ty.as_ref() {
            syn::Type::Path(path) if path.qself.is_none() => (
                path.path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect(),
                path.path.leading_colon.is_some(),
            ),
            _ => (Vec::new(), false),
        };
        if item.trait_.is_none() && matches!(item.self_ty.as_ref(), syn::Type::TraitObject(_)) {
            self.dyn_inherent_impl = true;
        }
        let mut fns = Vec::new();
        for impl_item in &item.items {
            if let syn::ImplItem::Fn(method) = impl_item {
                let (line, column) = start(method.span());
                let name = method.sig.ident.to_string();
                if let syn::ReturnType::Type(_, ty) = &method.sig.output {
                    let mut generics = generic_names(&item.generics);
                    generics.extend(generic_names(&method.sig.generics));
                    self.fn_outputs.insert(
                        (file.to_owned(), line, column),
                        FnOutput {
                            scope,
                            impl_index: Some(self.impls.len()),
                            generics,
                            ty: ty.as_ref().clone(),
                        },
                    );
                }
                fns.push(ImplFn {
                    name: name.clone(),
                    target: FnTarget {
                        path: file.to_owned(),
                        line,
                        column,
                        name,
                    },
                    receiver: receiver_of(&method.sig),
                });
                self.collect_block(scope, file, &method.block);
            }
        }
        self.impls.push(ImplDef {
            trait_name: item
                .trait_
                .as_ref()
                .and_then(|(path, _)| path.segments.last())
                .map(|segment| segment.ident.to_string()),
            scope,
            self_path,
            self_leading_colon,
            is_trait_impl: item.trait_.is_some(),
            fns,
            self_type: None,
            shape: impl_shape(item),
        });
    }

    /// Items declared inside a function body live in a block scope of their own, nested in the
    /// scope the block appears in; blocks without items need none.
    fn collect_block(&mut self, scope: ModId, file: &str, block: &syn::Block) {
        struct Blocks<'b>(Vec<&'b syn::Block>);
        impl<'b> Visit<'b> for Blocks<'b> {
            fn visit_block(&mut self, block: &'b syn::Block) {
                self.0.push(block);
            }
            fn visit_item(&mut self, _: &'b syn::Item) {}
        }
        let has_items = block
            .stmts
            .iter()
            .any(|stmt| matches!(stmt, syn::Stmt::Item(_)));
        let inner = if has_items {
            let krate = self.modules[scope].krate;
            let parent = self.module_of(scope);
            let id = self.new_module(krate, Some(parent), Some(scope));
            let (line, column) = start(block.brace_token.span.open());
            self.blocks.insert((file.to_owned(), line, column), id);
            let items: Vec<syn::Item> = block
                .stmts
                .iter()
                .filter_map(|stmt| match stmt {
                    syn::Stmt::Item(item) => Some(item.clone()),
                    _ => None,
                })
                .collect();
            // Block items cannot declare file modules (`mod x;` inside a fn is an error).
            let empty = BTreeMap::new();
            let mut unused = BTreeMap::new();
            self.collect_items(id, file, "", true, &items, &empty, &mut unused);
            id
        } else {
            scope
        };
        // Nested blocks (inside expressions and statements, not inside nested items, which
        // `collect_items` already walked with their own scopes).
        for stmt in &block.stmts {
            let mut nested = Blocks(Vec::new());
            match stmt {
                syn::Stmt::Item(_) => continue,
                syn::Stmt::Local(local) => nested.visit_local(local),
                syn::Stmt::Expr(expr, _) => nested.visit_expr(expr),
                syn::Stmt::Macro(_) => continue,
            }
            for block in nested.0 {
                self.collect_block(inner, file, block);
            }
        }
    }

    fn new_type(
        &mut self,
        scope: ModId,
        name: &str,
        variants: BTreeSet<String>,
        is_trait: bool,
        alias_of: Option<(Vec<String>, bool)>,
    ) -> TypeId {
        self.types.push(TypeDef {
            scope,
            name: name.to_owned(),
            variants,
            is_trait,
            alias_of,
            opaque_alias: false,
        });
        self.types.len() - 1
    }

    fn add_item(&mut self, module: ModId, ns: Ns, name: &str, def: Def, vis: &syn::Visibility) {
        let vis = self.vis(vis, module);
        self.modules[module].items.insert(
            ns,
            name,
            Entry {
                def,
                vis,
                origin: Origin::Item,
            },
        );
    }

    /// The fixed point: every iteration recomputes every scope from its items plus its imports,
    /// resolved against the previous iteration's scopes, until no scope changes.
    /// G145: an item-position invocation of a workspace `macro_rules!` -- `name!` defined earlier
    /// in the same module of the same file (textual scope, which precedes any path-based macro),
    /// or `crate::name!` naming the crate's `#[macro_export]` definitions -- may define only the
    /// names its definitions' transcribers bound; every other invocation opens its module.
    fn resolve_macro_calls(&mut self) {
        for call in std::mem::take(&mut self.macro_calls) {
            let krate = self.modules[call.module].krate;
            let candidates: Vec<&MacroDef> = match call.segments.as_slice() {
                [name] if !call.leading_colon => self
                    .macro_defs
                    .iter()
                    .filter(|d| {
                        d.module == call.module
                            && d.file == call.file
                            && d.line < call.line
                            && d.name == *name
                    })
                    .collect(),
                [root, name] if !call.leading_colon && root == "crate" => self
                    .macro_defs
                    .iter()
                    .filter(|d| {
                        d.exported && self.modules[d.module].krate == krate && d.name == *name
                    })
                    .collect(),
                _ => Vec::new(),
            };
            let bounded = !candidates.is_empty()
                && call.attributes_allowed
                && candidates.iter().all(|d| !d.names.unbounded);
            if !bounded {
                self.modules[call.module].open = true;
                continue;
            }
            let mut names = BTreeSet::new();
            for definition in candidates {
                names.extend(definition.names.literal.iter().cloned());
                if definition.names.from_invocation {
                    names.extend(call.idents.iter().cloned());
                }
            }
            self.modules[call.module].macro_names.extend(names);
        }
    }

    /// Whether a lookup of `name` in `module` is uncertain: the module is open, or an item macro
    /// there (or one a glob import reaches) may define `name` (G145).
    fn uncertain(&self, module: ModId, name: &str) -> bool {
        let module = &self.modules[module];
        module.open || module.shadow.contains(name)
    }

    fn resolve_imports(&mut self, crates: &[CrateInput]) {
        for module in &mut self.modules {
            module.scope = module.items.clone();
            module.shadow = module.macro_names.clone();
        }
        for _ in 0..64 {
            let mut next: Vec<Scope> = self.modules.iter().map(|m| m.items.clone()).collect();
            let mut opened = vec![false; self.modules.len()];
            let mut shadows: Vec<BTreeSet<String>> =
                self.modules.iter().map(|m| m.macro_names.clone()).collect();
            for (id, module) in self.modules.iter().enumerate() {
                for import in &module.imports {
                    self.apply_import(
                        crates,
                        id,
                        import,
                        &mut next[id],
                        &mut opened[id],
                        &mut shadows[id],
                    );
                }
            }
            let mut changed = false;
            for (id, (scope, shadow)) in next.into_iter().zip(shadows).enumerate() {
                if self.modules[id].scope != scope {
                    changed = true;
                    self.modules[id].scope = scope;
                }
                if self.modules[id].shadow != shadow {
                    changed = true;
                    self.modules[id].shadow = shadow;
                }
                if opened[id] && !self.modules[id].open {
                    changed = true;
                    self.modules[id].open = true;
                }
            }
            if !changed {
                return;
            }
        }
        // No fixed point within the bound: nothing imported may be trusted.
        for module in &mut self.modules {
            module.open = true;
        }
    }

    fn apply_import(
        &self,
        crates: &[CrateInput],
        id: ModId,
        import: &Import,
        into: &mut Scope,
        opened: &mut bool,
        shadow: &mut BTreeSet<String>,
    ) {
        let whole_is_prefix = import.name.is_none() || import.self_import;
        let Some(prefix) = self.resolve_prefix(
            crates,
            id,
            &import.segments,
            import.leading_colon,
            whole_is_prefix,
        ) else {
            match &import.name {
                None => *opened = true,
                Some(name) => {
                    for ns in [Ns::Types, Ns::Values] {
                        into.insert(
                            ns,
                            name,
                            Entry {
                                def: Def::Unknown,
                                vis: import.vis.clone(),
                                origin: Origin::Named,
                            },
                        );
                    }
                }
            }
            return;
        };
        match (&import.name, prefix) {
            (None, Def::Module(target)) => {
                if self.modules[target].open {
                    *opened = true;
                }
                shadow.extend(self.modules[target].shadow.iter().cloned());
                for ns in [Ns::Types, Ns::Values] {
                    for (name, entry) in self.modules[target].scope.ns(ns) {
                        if self.visible(&entry.vis, id) {
                            into.insert(
                                ns,
                                name,
                                Entry {
                                    def: entry.def.clone(),
                                    vis: import.vis.clone(),
                                    origin: Origin::Glob,
                                },
                            );
                        }
                    }
                }
            }
            (None, Def::Type(ty)) => {
                for variant in &self.types[ty].variants {
                    for ns in [Ns::Types, Ns::Values] {
                        into.insert(
                            ns,
                            variant,
                            Entry {
                                def: Def::Ctor,
                                vis: import.vis.clone(),
                                origin: Origin::Glob,
                            },
                        );
                    }
                }
            }
            (None, _) => *opened = true,
            (Some(name), Def::External(path)) if import.self_import => {
                into.insert(
                    Ns::Types,
                    name,
                    Entry {
                        def: Def::External(path),
                        vis: import.vis.clone(),
                        origin: Origin::Named,
                    },
                );
            }
            (Some(name), Def::Module(target)) if import.self_import => {
                into.insert(
                    Ns::Types,
                    name,
                    Entry {
                        def: Def::Module(target),
                        vis: import.vis.clone(),
                        origin: Origin::Named,
                    },
                );
            }
            (Some(name), prefix) => {
                let last = import
                    .segments
                    .last()
                    .expect("a named import has a last segment");
                let mut bound = false;
                for ns in [Ns::Types, Ns::Values] {
                    let def = match &prefix {
                        Def::Module(target) => self.modules[*target]
                            .scope
                            .ns(ns)
                            .get(last)
                            .filter(|entry| self.visible(&entry.vis, id))
                            .map(|entry| entry.def.clone()),
                        Def::Type(ty) if self.types[*ty].variants.contains(last) => Some(Def::Ctor),
                        Def::External(path) => Some(Def::External(extend(path, last))),
                        _ => None,
                    };
                    if let Some(def) = def {
                        bound = true;
                        into.insert(
                            ns,
                            name,
                            Entry {
                                def,
                                vis: import.vis.clone(),
                                origin: Origin::Named,
                            },
                        );
                    }
                }
                if !bound {
                    for ns in [Ns::Types, Ns::Values] {
                        into.insert(
                            ns,
                            name,
                            Entry {
                                def: Def::Unknown,
                                vis: import.vis.clone(),
                                origin: Origin::Named,
                            },
                        );
                    }
                }
            }
        }
    }

    /// Resolves all but the last segment of a path (the whole path for a glob or `self` import)
    /// to a module, type or external crate, in the types namespace.
    fn resolve_prefix(
        &self,
        crates: &[CrateInput],
        scope: ModId,
        segments: &[String],
        leading_colon: bool,
        whole_is_prefix: bool,
    ) -> Option<Def> {
        let prefix = if whole_is_prefix {
            segments
        } else {
            &segments[..segments.len().saturating_sub(1)]
        };
        let (first, rest) = prefix.split_first()?;
        let mut def = self.resolve_first_type_segment(crates, scope, first, leading_colon)?;
        for segment in rest {
            def = match def {
                Def::Module(module) if segment == "super" => {
                    Def::Module(self.modules[module].parent?)
                }
                Def::Module(module) => {
                    let entry = self.modules[module].scope.types.get(segment);
                    match entry {
                        Some(entry)
                            if entry.origin == Origin::Glob && self.uncertain(module, segment) =>
                        {
                            return None;
                        }
                        Some(entry) if self.visible(&entry.vis, scope) => entry.def.clone(),
                        _ => return None,
                    }
                }
                Def::External(path) => Def::External(extend(&path, segment)),
                // `Enum::Variant::..` and associated types are not modules.
                _ => return None,
            };
        }
        Some(def)
    }

    fn resolve_first_type_segment(
        &self,
        crates: &[CrateInput],
        scope: ModId,
        first: &str,
        leading_colon: bool,
    ) -> Option<Def> {
        let krate = self.modules[scope].krate;
        if leading_colon {
            return Some(self.extern_crate(crates, krate, first));
        }
        match first {
            "crate" => return Some(Def::Module(self.crate_roots[&krate])),
            "self" => return Some(Def::Module(self.module_of(scope))),
            "super" => return self.modules[self.module_of(scope)].parent.map(Def::Module),
            _ => {}
        }
        match self.lexical_lookup(scope, Ns::Types, first) {
            Lookup::Found(def) => Some(def),
            Lookup::Open => None,
            Lookup::Missing => Some(self.extern_crate(crates, krate, first)),
        }
    }

    /// A name no scope defines: a workspace crate from the extern prelude, else something outside
    /// the workspace (a registry dependency, `std`, a prelude type) -- external either way.
    fn extern_crate(&self, crates: &[CrateInput], krate: usize, name: &str) -> Def {
        match crates[krate].externs.get(name) {
            Some(index) => Def::Module(self.crate_roots[index]),
            None => self.external_root(name),
        }
    }

    /// An external root: a standard crate keeps its name as the start of a canonical path; any
    /// other (a registry crate, a prelude name) is external with an unknown path.
    fn external_root(&self, name: &str) -> Def {
        if matches!(name, "std" | "core" | "alloc") {
            Def::External(vec![name.to_owned()])
        } else {
            Def::External(Vec::new())
        }
    }

    fn lexical_lookup(&self, mut scope: ModId, ns: Ns, name: &str) -> Lookup {
        loop {
            let module = &self.modules[scope];
            if let Some(entry) = module.scope.ns(ns).get(name) {
                // A glob-provided name in an open scope may be shadowed by one we cannot see.
                if entry.origin == Origin::Glob && self.uncertain(scope, name) {
                    return Lookup::Open;
                }
                return Lookup::Found(entry.def.clone());
            }
            if self.uncertain(scope, name) {
                return Lookup::Open;
            }
            match module.lexical_parent {
                Some(parent) => scope = parent,
                None => return Lookup::Missing,
            }
        }
    }

    fn resolve_impls(&mut self, crates: &[CrateInput]) {
        for index in 0..self.impls.len() {
            let imp = &self.impls[index];
            if imp.self_path.is_empty() {
                continue;
            }
            let resolved = self.resolve_type_path(
                crates,
                imp.scope,
                &imp.self_path,
                imp.self_leading_colon,
                0,
            );
            self.impls[index].self_type = resolved;
        }
    }

    fn resolve_type_path(
        &self,
        crates: &[CrateInput],
        scope: ModId,
        segments: &[String],
        leading_colon: bool,
        depth: usize,
    ) -> Option<TypeId> {
        if depth > 8 {
            return None;
        }
        let def = if segments.len() == 1 && !leading_colon {
            match self.lexical_lookup(scope, Ns::Types, &segments[0]) {
                Lookup::Found(def) => def,
                _ => return None,
            }
        } else {
            let prefix = self.resolve_prefix(crates, scope, segments, leading_colon, false)?;
            let Def::Module(module) = prefix else {
                return None;
            };
            let entry = self.modules[module].scope.types.get(segments.last()?)?;
            entry.def.clone()
        };
        let Def::Type(ty) = def else {
            return None;
        };
        match &self.types[ty].alias_of {
            Some((path, leading)) => {
                self.resolve_type_path(crates, self.types[ty].scope, path, *leading, depth + 1)
            }
            None => Some(ty),
        }
    }

    /// `Type::name`: an enum variant, else the one inherent associated function of that name,
    /// else the one trait-impl function of that name (a concrete type's trait method is statically
    /// dispatched). Several candidates are ambiguous.
    fn associated(&self, ty: TypeId, name: &str) -> Result<FnTarget, &'static str> {
        if self.types[ty].is_trait {
            return Err("trait-method");
        }
        if self.types[ty].variants.contains(name) {
            return Err("constructor");
        }
        let candidates = |trait_impl: bool| -> Vec<&FnTarget> {
            self.impls
                .iter()
                .filter(|imp| imp.self_type == Some(ty) && imp.is_trait_impl == trait_impl)
                .flat_map(|imp| imp.fns.iter())
                .filter(|f| f.name == name)
                .map(|f| &f.target)
                .collect()
        };
        match candidates(false).as_slice() {
            [one] => return Ok((*one).clone()),
            [] => {}
            _ => return Err("ambiguous-associated"),
        }
        match candidates(true).as_slice() {
            [one] => Ok((*one).clone()),
            [] => Err("associated-not-found"),
            _ => Err("ambiguous-associated"),
        }
    }
}

impl DefMap {
    /// A workspace type's canonical identity: `<package> <module path>/<Name>#` (the G66
    /// descriptor shape, SCIP's type suffix). A type declared in a block has no stable path.
    fn type_descriptor(&self, ty: TypeId) -> Option<String> {
        let def = &self.types[ty];
        let mut names = Vec::new();
        let mut module = def.scope;
        loop {
            let m = &self.modules[module];
            if m.is_block {
                return None;
            }
            match m.parent {
                Some(parent) => {
                    names.push(m.name.clone());
                    module = parent;
                }
                None => break,
            }
        }
        names.reverse();
        names.push(format!("{}#", def.name));
        let package = match self.crate_packages[&self.modules[module].krate].as_str() {
            "" => ".",
            package => package,
        };
        Some(format!("{package} {}", names.join("/")))
    }

    /// The canonical identity of a resolved type definition, following transparent aliases.
    fn canonical_of_type(&self, crates: &[CrateInput], ty: TypeId, depth: usize) -> Option<String> {
        let def = &self.types[ty];
        if def.is_trait || def.opaque_alias || depth > 8 {
            return None;
        }
        match &def.alias_of {
            Some((path, leading)) => {
                let target = self.resolve_type_path(crates, def.scope, path, *leading, 0)?;
                self.canonical_of_type(crates, target, depth + 1)
            }
            None if self.types[ty].name.is_empty() => None,
            None => self.type_descriptor(ty),
        }
    }
}

/// The std path of a prelude type name (the types every Rust module sees unqualified).
fn prelude_type(name: &str) -> Option<&'static str> {
    Some(match name {
        "Vec" => "std::vec::Vec",
        "String" => "std::string::String",
        "Box" => "std::boxed::Box",
        "Option" => "std::option::Option",
        "Result" => "std::result::Result",
        _ => return None,
    })
}

fn is_primitive(name: &str) -> bool {
    matches!(
        name,
        "bool"
            | "char"
            | "str"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "f32"
            | "f64"
    )
}

impl DefMap {
    /// G142: the workspace type a function's declared output names, when it is a plain one: a
    /// path without generic arguments (lifetimes erased, G151) to a non-trait workspace type that is not a generic name in
    /// force, or `Self` of an impl of such a type without generics.
    fn output_type(&self, crates: &[CrateInput], target: &FnTarget) -> Option<TypeId> {
        let output = self
            .fn_outputs
            .get(&(target.path.clone(), target.line, target.column))?;
        let syn::Type::Path(path) = &output.ty else {
            return None;
        };
        if path.qself.is_some()
            || path
                .path
                .segments
                .iter()
                .any(|s| !matches!(without_lifetimes(&s.arguments), syn::PathArguments::None))
        {
            return None;
        }
        let segments: Vec<String> = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        if let [only] = segments.as_slice() {
            if output.generics.contains(only) {
                return None;
            }
            if only == "Self" {
                let imp = &self.impls[output.impl_index?];
                return (imp.shape == PLAIN_IMPL_SHAPE).then_some(imp.self_type?);
            }
        }
        let ty = self.resolve_type_path(
            crates,
            output.scope,
            &segments,
            path.path.leading_colon.is_some(),
            0,
        )?;
        (!self.types[ty].is_trait).then_some(ty)
    }

    /// G143: the type of field `name` of the workspace struct `ty` (declared without generics)
    /// with the form its spelling gives (`T`, `&T`, `&mut T`), when `T` is a plain workspace type.
    fn field_type(
        &self,
        crates: &[CrateInput],
        ty: TypeId,
        name: &str,
    ) -> Option<(TypeId, Receiver)> {
        let (scope, spelled) = self.struct_fields.get(&ty)?.get(name)?;
        let (form, inner) = match spelled {
            syn::Type::Reference(reference) => (
                if reference.mutability.is_some() {
                    Receiver::RefMut
                } else {
                    Receiver::Ref
                },
                reference.elem.as_ref(),
            ),
            other => (Receiver::Value, other),
        };
        let syn::Type::Path(path) = inner else {
            return None;
        };
        if path.qself.is_some()
            || path
                .path
                .segments
                .iter()
                .any(|s| !matches!(without_lifetimes(&s.arguments), syn::PathArguments::None))
        {
            return None;
        }
        let segments: Vec<String> = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        let field = self.resolve_type_path(
            crates,
            *scope,
            &segments,
            path.path.leading_colon.is_some(),
            0,
        )?;
        (!self.types[field].is_trait).then_some((field, form))
    }

    /// G142: whether the probe's autoref step can be claimed for `ty.name(..)` from `scope`: no
    /// by-value method of that name can come first. A by-value workspace trait method, a std
    /// blanket by-value method (`into`, `try_into`, `into_iter`), a non-std import or an unseen
    /// name in the scope chain, or an explicit impl of a std trait with by-value methods on `ty`
    /// could each supply one; any of them withholds the claim.
    fn autoref_allowed(&self, scope: ModId, ty: TypeId, name: &str) -> bool {
        const BY_VALUE_TRAITS: [&str; 8] = [
            "Iterator",
            "IntoIterator",
            "DoubleEndedIterator",
            "Read",
            "BufRead",
            "Write",
            "Future",
            "Stream",
        ];
        self.autoref_name_allowed(scope, name)
            && !self.impls.iter().any(|imp| {
                imp.self_type == Some(ty)
                    && imp
                        .trait_name
                        .as_deref()
                        .is_some_and(|t| BY_VALUE_TRAITS.contains(&t))
            })
    }

    /// The part of [`Self::autoref_allowed`] that does not depend on a workspace type (G144: a
    /// std receiver has no workspace impl).
    fn autoref_name_allowed(&self, scope: ModId, name: &str) -> bool {
        const BLANKET: [&str; 3] = ["into", "try_into", "into_iter"];
        if BLANKET.contains(&name) {
            return false;
        }
        let by_value_trait_method = self
            .trait_fns
            .values()
            .flatten()
            .any(|f| f.name == name && f.receiver == Some(Receiver::Value));
        if by_value_trait_method {
            return false;
        }
        let mut cursor = Some(scope);
        while let Some(id) = cursor {
            let module = &self.modules[id];
            if module.open
                || !module.shadow.is_empty()
                || module
                    .scope
                    .types
                    .values()
                    .any(|entry| matches!(&entry.def, Def::External(path) if path.is_empty()))
            {
                return false;
            }
            cursor = module
                .lexical_parent
                .or(module.parent.filter(|_| module.is_block));
        }
        true
    }

    /// `self.name(..)` in a method with receiver `form` of an impl of `ty` shaped `shape`. The
    /// method probe's first step tries the receiver's own type by value, inherent methods before
    /// trait methods; an inherent method whose receiver form equals the caller's is therefore
    /// chosen there, ahead of every trait. Anything else is left to type inference.
    fn method_on_self(
        &self,
        ty: TypeId,
        shape: &str,
        name: &str,
        form: Receiver,
        autoref: bool,
    ) -> PathCallOutcome {
        let candidates: Vec<(&ImplDef, &ImplFn)> = self
            .impls
            .iter()
            .filter(|imp| imp.self_type == Some(ty) && !imp.is_trait_impl)
            .flat_map(|imp| imp.fns.iter().map(move |f| (imp, f)))
            .filter(|(_, f)| f.name == name && f.receiver.is_some())
            .collect();
        match candidates.as_slice() {
            [] => PathCallOutcome::Unresolved("method-not-inherent"),
            [(imp, _)] if imp.shape != shape => PathCallOutcome::Unresolved("generic-impl"),
            // G142: the autoref step -- a value taking `&self` or `&mut self`, or `&mut T`
            // taking `&self` -- when nothing by value can come first.
            [(_, f)]
                if autoref
                    && matches!(
                        (form, f.receiver),
                        (Receiver::Value, Some(Receiver::Ref | Receiver::RefMut))
                            | (Receiver::RefMut, Some(Receiver::Ref))
                    ) =>
            {
                PathCallOutcome::Resolved(f.target.clone())
            }
            [(_, f)] if f.receiver != Some(form) => {
                PathCallOutcome::Unresolved("receiver-form-differs")
            }
            [(_, f)] => PathCallOutcome::Resolved(f.target.clone()),
            _ => PathCallOutcome::Unresolved("ambiguous-associated"),
        }
    }
}

impl DefMap {
    /// G140: `x.name(..)` where `x`'s type is known only through the workspace traits `bounds`.
    /// Their methods are the probe's inherent-like candidates (object and parameter candidates),
    /// ahead of every other trait: the one bound method of that name with the caller's receiver
    /// form is called through its declaration. Two bound methods of one name would not compile.
    fn method_on_bounds(&self, bounds: &[TypeId], name: &str, form: Receiver) -> PathCallOutcome {
        let candidates: Vec<&ImplFn> = bounds
            .iter()
            .flat_map(|t| self.trait_fns.get(t).into_iter().flatten())
            .filter(|f| f.name == name)
            .collect();
        match candidates.as_slice() {
            [] => PathCallOutcome::Unresolved("method-not-in-bounds"),
            [f] if f.receiver != Some(form) => PathCallOutcome::Unresolved("receiver-form-differs"),
            [f] => PathCallOutcome::Dynamic(f.target.clone()),
            _ => PathCallOutcome::Unresolved("ambiguous-associated"),
        }
    }
}

enum Lookup {
    Found(Def),
    Missing,
    /// The scope may define the name through something this pass cannot see.
    Open,
}

fn flatten_use(
    tree: &syn::UseTree,
    mut prefix: Vec<String>,
    leading_colon: bool,
    vis: &Vis,
    out: &mut Vec<Import>,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            flatten_use(&path.tree, prefix, leading_colon, vis, out);
        }
        syn::UseTree::Name(name) => {
            let ident = name.ident.to_string();
            if ident == "self" {
                let Some(last) = prefix.last().cloned() else {
                    return;
                };
                out.push(Import {
                    segments: prefix,
                    leading_colon,
                    name: Some(last),
                    self_import: true,
                    vis: vis.clone(),
                });
            } else {
                prefix.push(ident.clone());
                out.push(Import {
                    segments: prefix,
                    leading_colon,
                    name: Some(ident),
                    self_import: false,
                    vis: vis.clone(),
                });
            }
        }
        syn::UseTree::Rename(rename) => {
            let ident = rename.ident.to_string();
            let alias = rename.rename.to_string();
            if alias == "_" {
                return;
            }
            let self_import = ident == "self";
            if !self_import {
                prefix.push(ident);
            }
            if prefix.is_empty() {
                return;
            }
            out.push(Import {
                segments: prefix,
                leading_colon,
                name: Some(alias),
                self_import,
                vis: vis.clone(),
            });
        }
        syn::UseTree::Glob(_) => out.push(Import {
            segments: prefix,
            leading_colon,
            name: None,
            self_import: false,
            vis: vis.clone(),
        }),
        syn::UseTree::Group(group) => {
            for tree in &group.items {
                flatten_use(tree, prefix.clone(), leading_colon, vis, out);
            }
        }
    }
}

struct FnCtx {
    locals: BTreeSet<String>,
    generics: BTreeSet<String>,
    receiver: Option<Receiver>,
    /// G139: locals whose type is declared (`x: T`, `x: &T`, `x: &mut T` for a workspace type
    /// without generic arguments) and bound exactly once in the function, with the position
    /// from which the binding is in scope (a parameter's is the start of the function).
    typed: BTreeMap<String, TypedLocal>,
}

/// A local whose declared type decides a method call on it (G139).
#[derive(Clone)]
struct TypedLocal {
    ty: LocalType,
    form: Receiver,
    from: (usize, usize),
    /// G144: the end of the block the binding lives in (`MAX` for a parameter).
    until: (usize, usize),
}

/// The end of the whole function (a parameter's or top-level binding's scope).
const FN_END: (usize, usize) = (usize::MAX, usize::MAX);

/// G144: standard-library functions whose documented output a receiver may take, by the canonical
/// path the resolver spells (`std::fs::File::create`): the std type returned and whether it comes
/// in a `Result` (`?` unwraps it). Declared knowledge, like the std-path effect tables; a path
/// absent here types nothing.
const STD_RETURNS: &[(&str, &str, bool)] = &[
    ("std::fs::File::create", "std::fs::File", true),
    ("std::fs::File::open", "std::fs::File", true),
    ("std::sync::Mutex::new", "std::sync::Mutex", false),
    ("std::sync::RwLock::new", "std::sync::RwLock", false),
    ("std::thread::spawn", "std::thread::JoinHandle", false),
];

/// The std type a declared std function returns (`fallible`: inside a `Result`).
fn std_return(path: &str, fallible: bool) -> Option<&'static str> {
    STD_RETURNS
        .iter()
        .find(|(function, _, wrapped)| *function == path && *wrapped == fallible)
        .map(|(_, ty, _)| *ty)
}

/// G144: inherent methods of std types, with their documented receiver form. An inherent method
/// wins the probe step its form matches, ahead of every trait; only these are claimed on a std
/// receiver, as the std path `<type>::<method>` the declared effect, persistence and concurrency
/// tables read.
const STD_INHERENT: &[(&str, &str, Receiver)] = &[
    ("std::fs::File", "sync_all", Receiver::Ref),
    ("std::fs::File", "sync_data", Receiver::Ref),
    ("std::sync::Mutex", "lock", Receiver::Ref),
    ("std::sync::mpsc::Receiver", "recv", Receiver::Ref),
    ("std::sync::mpsc::Sender", "send", Receiver::Ref),
    ("std::sync::RwLock", "read", Receiver::Ref),
    ("std::sync::RwLock", "write", Receiver::Ref),
    ("std::thread::JoinHandle", "join", Receiver::Value),
];

/// A receiver expression's type as the method probe first sees it (G143).
struct ReceiverType {
    ty: LocalType,
    form: Receiver,
    /// The impl shape the called method's impl must have (`self` in its own impl; else plain).
    shape: String,
    label: String,
}

/// What a declared type says about a receiver.
#[derive(Clone)]
enum LocalType {
    /// A plain workspace type (G139).
    Concrete(TypeId),
    /// A type known only through workspace traits (G140).
    Bounded(Vec<TypeId>),
    /// A standard-library type, by its canonical path (G144).
    Std(String),
}

/// `impl_shape` of an impl without generics or a where clause, of a type without arguments.
const PLAIN_IMPL_SHAPE: &str = "  | ";

/// How many times each identifier is bound in a function body, closures included (G139): a name
/// bound once is the same local wherever it is used.
#[derive(Default)]
struct BindingCounts(BTreeMap<String, usize>);

impl<'ast> Visit<'ast> for BindingCounts {
    fn visit_pat_ident(&mut self, pat: &'ast syn::PatIdent) {
        *self.0.entry(pat.ident.to_string()).or_default() += 1;
        syn::visit::visit_pat_ident(self, pat);
    }
    fn visit_item(&mut self, _: &'ast syn::Item) {}
}

/// The impl a method body sits in.
struct ImplCtx {
    ty: Option<TypeId>,
    generics: BTreeSet<String>,
    shape: String,
}

struct CallWalker<'a> {
    map: &'a DefMap,
    crates: &'a [CrateInput],
    file: &'a str,
    scopes: Vec<ModId>,
    /// The enclosing impl's self type (`None` inside a trait or an impl of a non-path type).
    impl_self: Vec<Option<ImplCtx>>,
    fn_ctx: Vec<FnCtx>,
    /// Generic parameters of enclosing structs, enums, unions, traits and type aliases.
    item_generics: Vec<BTreeSet<String>>,
    out: &'a mut Vec<PathCallResolution>,
    types: &'a mut Vec<TypeResolution>,
    releases: &'a mut Vec<ReleaseResolution>,
}

/// Every identifier a pattern binds.
struct Bindings(BTreeSet<String>);

impl<'ast> Visit<'ast> for Bindings {
    fn visit_pat_ident(&mut self, pat: &'ast syn::PatIdent) {
        self.0.insert(pat.ident.to_string());
        syn::visit::visit_pat_ident(self, pat);
    }
    fn visit_item(&mut self, _: &'ast syn::Item) {}
    fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}
}

fn generic_names(generics: &syn::Generics) -> BTreeSet<String> {
    generics
        .params
        .iter()
        .filter_map(|param| match param {
            syn::GenericParam::Type(ty) => Some(ty.ident.to_string()),
            _ => None,
        })
        .collect()
}

impl CallWalker<'_> {
    fn scope(&self) -> ModId {
        *self.scopes.last().expect("a scope is always active")
    }

    fn is_generic(&self, name: &str) -> bool {
        self.fn_ctx
            .last()
            .is_some_and(|f| f.generics.contains(name))
            || matches!(self.impl_self.last(), Some(Some(ctx)) if ctx.generics.contains(name))
            || self.item_generics.iter().any(|g| g.contains(name))
    }

    /// The canonical identity of `ty` in the current scope (G83), or `None` when any part of it
    /// does not resolve soundly.
    fn canonical_type(&self, ty: &syn::Type) -> Option<String> {
        match ty {
            syn::Type::Path(path) if path.qself.is_none() => self.canonical_path_type(&path.path),
            syn::Type::Reference(reference) => {
                let inner = self.canonical_type(&reference.elem)?;
                let mutability = if reference.mutability.is_some() {
                    "mut "
                } else {
                    ""
                };
                Some(format!("&{mutability}{inner}"))
            }
            syn::Type::Slice(slice) => Some(format!("[{}]", self.canonical_type(&slice.elem)?)),
            syn::Type::Array(array) => {
                use quote::ToTokens;
                let len = array.len.to_token_stream().to_string();
                Some(format!("[{}; {len}]", self.canonical_type(&array.elem)?))
            }
            syn::Type::Tuple(tuple) => {
                let elems: Option<Vec<String>> =
                    tuple.elems.iter().map(|t| self.canonical_type(t)).collect();
                let elems = elems?;
                Some(match elems.as_slice() {
                    [one] => format!("({one},)"),
                    _ => format!("({})", elems.join(", ")),
                })
            }
            syn::Type::Paren(paren) => self.canonical_type(&paren.elem),
            syn::Type::Group(group) => self.canonical_type(&group.elem),
            syn::Type::Ptr(ptr) => {
                let kind = match ptr.mutability {
                    syn::PointerMutability::Mut(_) => "mut",
                    syn::PointerMutability::Const(_) => "const",
                };
                Some(format!("*{kind} {}", self.canonical_type(&ptr.elem)?))
            }
            syn::Type::Never(_) => Some("!".into()),
            _ => None,
        }
    }

    fn canonical_path_type(&self, path: &syn::Path) -> Option<String> {
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        let leading = path.leading_colon.is_some();
        let last = path.segments.last()?;
        // Arguments anywhere but the last segment (`Vec::<T>::X`) are not modeled.
        if path
            .segments
            .iter()
            .rev()
            .skip(1)
            .any(|s| !s.arguments.is_empty())
        {
            return None;
        }
        let head = if segments.len() == 1 && !leading {
            let name = &segments[0];
            if self.is_generic(name) {
                return None;
            }
            if name == "Self" {
                return match self.impl_self.last() {
                    Some(Some(ImplCtx {
                        ty: Some(ty),
                        generics,
                        ..
                    })) if generics.is_empty() && last.arguments.is_empty() => {
                        self.map.canonical_of_type(self.crates, *ty, 0)
                    }
                    _ => None,
                };
            }
            match self.map.lexical_lookup(self.scope(), Ns::Types, name) {
                Lookup::Found(Def::Type(ty)) => self.map.canonical_of_type(self.crates, ty, 0)?,
                Lookup::Found(Def::External(path)) if !path.is_empty() => path.join("::"),
                Lookup::Found(_) | Lookup::Open => return None,
                Lookup::Missing if is_primitive(name) => name.clone(),
                Lookup::Missing => prelude_type(name)?.to_owned(),
            }
        } else {
            match self
                .map
                .resolve_prefix(self.crates, self.scope(), &segments, leading, false)?
            {
                Def::Module(module) => {
                    // In an open module only a glob-provided name is uncertain: an explicit item
                    // or named import cannot be redefined by what this pass does not see.
                    let open = self.map.uncertain(module, &last.ident.to_string());
                    match self.map.modules[module]
                        .scope
                        .types
                        .get(&last.ident.to_string())
                    {
                        Some(entry) if open && entry.origin == Origin::Glob => return None,
                        Some(entry) if self.map.visible(&entry.vis, self.scope()) => {
                            match &entry.def {
                                Def::Type(ty) => self.map.canonical_of_type(self.crates, *ty, 0)?,
                                Def::External(path) if !path.is_empty() => path.join("::"),
                                _ => return None,
                            }
                        }
                        _ => return None,
                    }
                }
                Def::External(path) if !path.is_empty() => {
                    extend(&path, &last.ident.to_string()).join("::")
                }
                _ => return None,
            }
        };
        match &last.arguments {
            syn::PathArguments::None => Some(head),
            syn::PathArguments::AngleBracketed(arguments) => {
                let mut parts = Vec::new();
                for argument in &arguments.args {
                    match argument {
                        syn::GenericArgument::Type(ty) => parts.push(self.canonical_type(ty)?),
                        // Lifetimes do not distinguish types for identity.
                        syn::GenericArgument::Lifetime(_) => {}
                        _ => return None,
                    }
                }
                Some(if parts.is_empty() {
                    head
                } else {
                    format!("{head}<{}>", parts.join(", "))
                })
            }
            syn::PathArguments::Parenthesized(_) => None,
        }
    }

    fn enter_fn(&mut self, sig: &syn::Signature, block: &syn::Block) {
        let mut bindings = Bindings(BTreeSet::new());
        for input in &sig.inputs {
            bindings.visit_fn_arg(input);
        }
        bindings.visit_block(block);
        let mut generics = generic_names(&sig.generics);
        if let Some(Some(outer)) = self.impl_self.last() {
            generics.extend(outer.generics.iter().cloned());
        }
        let typed = self.typed_locals(sig, block, &generics);
        self.fn_ctx.push(FnCtx {
            locals: bindings.0,
            generics,
            receiver: receiver_of(sig),
            typed,
        });
        self.type_lets(sig, block);
        let releases = self.resource_releases(sig, block);
        self.releases.extend(releases);
    }

    /// G143: the type of a receiver expression as the method probe first sees it, with the impl
    /// shape its methods must match and a label for the call: `self` (G79; `Self: Trait` in a
    /// default body, G140), a typed local (G139, G140, G142), a field of a typed receiver whose
    /// struct declares it, the result of a resolved call whose callee declares a plain output,
    /// or any of these in parentheses. `at` is the call's position (a local counts after its
    /// binding).
    fn receiver_type(&self, expr: &syn::Expr, at: (usize, usize)) -> Option<ReceiverType> {
        match expr {
            syn::Expr::Path(path) if path.qself.is_none() => {
                let local = path.path.get_ident()?.to_string();
                if local == "self"
                    && let Some(FnCtx {
                        receiver: Some(form),
                        ..
                    }) = self.fn_ctx.last()
                    && let Some(Some(ImplCtx {
                        ty: Some(ty),
                        shape,
                        ..
                    })) = self.impl_self.last()
                {
                    return Some(ReceiverType {
                        ty: LocalType::Concrete(*ty),
                        form: *form,
                        shape: shape.clone(),
                        label: local,
                    });
                }
                let typed = self.fn_ctx.last()?.typed.get(&local)?;
                (at > typed.from && at < typed.until).then(|| ReceiverType {
                    ty: typed.ty.clone(),
                    form: typed.form,
                    shape: PLAIN_IMPL_SHAPE.into(),
                    label: local,
                })
            }
            syn::Expr::Field(field) => {
                let syn::Member::Named(name) = &field.member else {
                    return None;
                };
                let base = self.receiver_type(&field.base, at)?;
                let LocalType::Concrete(ty) = base.ty else {
                    return None;
                };
                let (ty, form) = self.map.field_type(self.crates, ty, &name.to_string())?;
                Some(ReceiverType {
                    ty: LocalType::Concrete(ty),
                    form,
                    shape: PLAIN_IMPL_SHAPE.into(),
                    label: format!("{}.{name}", base.label),
                })
            }
            syn::Expr::MethodCall(inner) => {
                let (label, PathCallOutcome::Resolved(target)) = self.method_outcome(inner)? else {
                    return None;
                };
                Some(ReceiverType {
                    ty: LocalType::Concrete(self.map.output_type(self.crates, &target)?),
                    form: Receiver::Value,
                    shape: PLAIN_IMPL_SHAPE.into(),
                    label: format!("{label}()"),
                })
            }
            syn::Expr::Call(call) => {
                let syn::Expr::Path(path) = call.func.as_ref() else {
                    return None;
                };
                match self.resolve_call(path) {
                    PathCallOutcome::Resolved(target) => Some(ReceiverType {
                        ty: LocalType::Concrete(self.map.output_type(self.crates, &target)?),
                        form: Receiver::Value,
                        shape: PLAIN_IMPL_SHAPE.into(),
                        label: format!("{}()", target.name),
                    }),
                    // G144: a std function whose declared output is a std type, not in a Result.
                    PathCallOutcome::External(std) => {
                        std_return(&std, false).map(|ty| ReceiverType {
                            ty: LocalType::Std(ty.into()),
                            form: Receiver::Value,
                            shape: PLAIN_IMPL_SHAPE.into(),
                            label: format!("{std}()"),
                        })
                    }
                    _ => None,
                }
            }
            // G144: `?` on a std function whose declared output comes in a `Result`.
            syn::Expr::Try(fallible) => {
                let syn::Expr::Call(call) = fallible.expr.as_ref() else {
                    return None;
                };
                let syn::Expr::Path(path) = call.func.as_ref() else {
                    return None;
                };
                let PathCallOutcome::External(std) = self.resolve_call(path) else {
                    return None;
                };
                std_return(&std, true).map(|ty| ReceiverType {
                    ty: LocalType::Std(ty.into()),
                    form: Receiver::Value,
                    shape: PLAIN_IMPL_SHAPE.into(),
                    label: format!("{std}()?"),
                })
            }
            syn::Expr::Paren(inner) => self.receiver_type(&inner.expr, at),
            _ => None,
        }
    }

    /// The probe's decision for a method call whose receiver's type is known.
    fn method_outcome(&self, call: &syn::ExprMethodCall) -> Option<(String, PathCallOutcome)> {
        let at = start(call.method.span());
        let receiver = self.receiver_type(&call.receiver, at)?;
        let name = call.method.to_string();
        let outcome = match &receiver.ty {
            LocalType::Concrete(ty) => self.map.method_on_self(
                *ty,
                &receiver.shape,
                &name,
                receiver.form,
                self.map.autoref_allowed(self.scope(), *ty, &name),
            ),
            LocalType::Bounded(bounds) => self.map.method_on_bounds(bounds, &name, receiver.form),
            LocalType::Std(ty) => self.std_method(ty, &name, receiver.form),
        };
        Some((format!("{}.{name}", receiver.label), outcome))
    }

    /// G144: a declared inherent method of a std type is the std path `<type>::<method>` when the
    /// receiver's form matches, or when the autoref step can be claimed; nothing else is claimed.
    fn std_method(&self, ty: &str, name: &str, form: Receiver) -> PathCallOutcome {
        let Some((_, _, declared)) = STD_INHERENT
            .iter()
            .find(|(t, method, _)| *t == ty && *method == name)
        else {
            return PathCallOutcome::Unresolved("std-method-undeclared");
        };
        let autoref = matches!(
            (form, *declared),
            (Receiver::Value, Receiver::Ref | Receiver::RefMut) | (Receiver::RefMut, Receiver::Ref)
        );
        if form == *declared || (autoref && self.map.autoref_name_allowed(self.scope(), name)) {
            PathCallOutcome::External(format!("{ty}::{name}"))
        } else {
            PathCallOutcome::Unresolved("receiver-form-differs")
        }
    }

    /// G142: a top-level `let x = ..` bound once whose value's type the resolved callee declares:
    /// a function whose output is a plain workspace type (`Self` of a plain impl included), a
    /// struct literal, a tuple-struct or enum-variant constructor of such a type. The local holds
    /// the value (receiver form by value).
    fn type_lets(&mut self, sig: &syn::Signature, block: &syn::Block) {
        let mut counts = BindingCounts::default();
        for input in &sig.inputs {
            counts.visit_fn_arg(input);
        }
        counts.visit_block(block);
        // G144: every `let` of the body in source order, with the end of the block it lives in
        // (closure bodies included, nested items not).
        struct Lets<'b> {
            ends: Vec<(usize, usize)>,
            found: Vec<(&'b syn::Local, (usize, usize))>,
        }
        impl<'b> Visit<'b> for Lets<'b> {
            fn visit_block(&mut self, block: &'b syn::Block) {
                self.ends.push(start(block.brace_token.span.close()));
                syn::visit::visit_block(self, block);
                self.ends.pop();
            }
            fn visit_local(&mut self, local: &'b syn::Local) {
                self.found
                    .push((local, self.ends.last().copied().unwrap_or(FN_END)));
                syn::visit::visit_local(self, local);
            }
            fn visit_item(&mut self, _: &'b syn::Item) {}
        }
        let mut lets = Lets {
            ends: Vec::new(),
            found: Vec::new(),
        };
        lets.visit_block(block);
        let generics = self
            .fn_ctx
            .last()
            .map(|c| c.generics.clone())
            .unwrap_or_default();
        for (local, until) in lets.found {
            let (ident, declared) = match &local.pat {
                syn::Pat::Ident(ident) => (ident, None),
                syn::Pat::Type(typed) => match typed.pat.as_ref() {
                    syn::Pat::Ident(ident) => (ident, Some(typed.ty.as_ref())),
                    _ => continue,
                },
                _ => continue,
            };
            let name = ident.ident.to_string();
            if ident.by_ref.is_some()
                || ident.subpat.is_some()
                || counts.0.get(&name) != Some(&1)
                || self
                    .fn_ctx
                    .last()
                    .is_some_and(|c| c.typed.contains_key(&name))
            {
                continue;
            }
            let from = start(local.let_token.span);
            let typed = match declared {
                // A declared type in a nested block (top-level ones are G139's).
                Some(ty) => self.declared_receiver_type(ty, &generics, &sig.generics),
                None => {
                    let Some(init) = &local.init else {
                        continue;
                    };
                    match self.value_type(&init.expr) {
                        Some(ty) => Some((LocalType::Concrete(ty), Receiver::Value)),
                        // G143: a field or a call result; G144: a declared std output.
                        None => self
                            .receiver_type(&init.expr, from)
                            .map(|receiver| (receiver.ty, receiver.form)),
                    }
                }
            };
            // Each binding is typed in order, so a later `let` may use an earlier one.
            if let (Some((ty, form)), Some(ctx)) = (typed, self.fn_ctx.last_mut()) {
                ctx.typed.insert(
                    name,
                    TypedLocal {
                        ty,
                        form,
                        from,
                        until,
                    },
                );
            }
        }
    }

    /// G157 (NA-RESOURCE-DIMENSION): where a resource bound once by a `let` is given back. The
    /// initializer, through `?`, `.unwrap()` or `.expect(..)`, is a call resolved to a std path
    /// the declared resource table names; the local is then followed through the block it lives
    /// in. A borrow, a field, an index, a dereference, a method receiver (except a method taking
    /// it by value, `consumes_receiver`) and a formatting macro leave it in place; `drop(local)`
    /// resolved to `std::mem::drop` or `local.join()` resolved to `JoinHandle::join`, as a
    /// statement of that block, releases it there; any other use -- a value passed, returned,
    /// stored, captured by a `move` closure, named inside another macro, or a release inside a
    /// branch -- moves it, and nothing is claimed. A local never moved is released at the end of
    /// its block when dropping it releases the resource (not a thread: a dropped `JoinHandle`
    /// detaches).
    fn resource_releases(
        &self,
        sig: &syn::Signature,
        block: &syn::Block,
    ) -> Vec<ReleaseResolution> {
        let mut counts = BindingCounts::default();
        for input in &sig.inputs {
            counts.visit_fn_arg(input);
        }
        counts.visit_block(block);
        let mut out = Vec::new();
        for (local, holder_block, until) in lets_of(block) {
            let syn::Pat::Ident(ident) = &local.pat else {
                continue;
            };
            let name = ident.ident.to_string();
            let Some(init) = &local.init else {
                continue;
            };
            if ident.by_ref.is_some()
                || ident.subpat.is_some()
                || init.diverge.is_some()
                || counts.0.get(&name) != Some(&1)
            {
                continue;
            }
            let Some((acquired, kind)) = self.acquisition(&init.expr) else {
                continue;
            };
            let mut uses = HolderUses {
                walker: self,
                name: &name,
                after: start(local.semi_token.span),
                until,
                depth: 0,
                moved: false,
                taking: false,
                released: Vec::new(),
            };
            for stmt in &holder_block.stmts {
                uses.visit_stmt(stmt);
            }
            if uses.moved {
                continue;
            }
            let (how, at) = match uses.released.as_slice() {
                [] if kind.released_by_drop() && until != FN_END => {
                    (atlas_core::ResourceRelease::ScopeEnd, until)
                }
                [(atlas_core::ResourceRelease::ExplicitDrop, at)] if kind.released_by_drop() => {
                    (atlas_core::ResourceRelease::ExplicitDrop, *at)
                }
                [(atlas_core::ResourceRelease::Join, at)]
                    if kind == atlas_core::ResourceKind::Thread =>
                {
                    (atlas_core::ResourceRelease::Join, *at)
                }
                _ => continue,
            };
            out.push(ReleaseResolution {
                path: self.file.to_owned(),
                acquired,
                holder: name,
                line: at.0,
                column: at.1,
                kind,
                release: how,
            });
        }
        out
    }

    /// The acquiring call under `?`, `.unwrap()` and `.expect(..)`: its position (where its
    /// resolution is recorded) and the resource the declared table names for its std path.
    fn acquisition(&self, expr: &syn::Expr) -> Option<((usize, usize), atlas_core::ResourceKind)> {
        match expr {
            syn::Expr::Try(fallible) => self.acquisition(&fallible.expr),
            syn::Expr::Paren(inner) => self.acquisition(&inner.expr),
            syn::Expr::MethodCall(call)
                if matches!(call.method.to_string().as_str(), "unwrap" | "expect") =>
            {
                self.acquisition(&call.receiver)
            }
            other => self.acquired_by(other),
        }
    }

    /// The resource a call itself acquires, if its resolution is a std path the table names.
    fn acquired_by(&self, expr: &syn::Expr) -> Option<((usize, usize), atlas_core::ResourceKind)> {
        let (at, outcome) = match expr {
            syn::Expr::Call(call) => {
                let syn::Expr::Path(path) = call.func.as_ref() else {
                    return None;
                };
                let last = path.path.segments.last()?;
                (start(last.ident.span()), self.resolve_call(path))
            }
            syn::Expr::MethodCall(call) => {
                (start(call.method.span()), self.method_outcome(call)?.1)
            }
            _ => return None,
        };
        let PathCallOutcome::External(std) = outcome else {
            return None;
        };
        atlas_core::std_path_resource(&std).map(|kind| (at, kind))
    }

    /// The plain workspace type of a call's or literal's value, when its callee declares it.
    fn value_type(&self, expr: &syn::Expr) -> Option<TypeId> {
        let plain = |path: &syn::Path| {
            path.segments
                .iter()
                .all(|s| matches!(s.arguments, syn::PathArguments::None))
        };
        let segments = |path: &syn::Path| -> Vec<String> {
            path.segments.iter().map(|s| s.ident.to_string()).collect()
        };
        let non_trait = |ty: TypeId| (!self.map.types[ty].is_trait).then_some(ty);
        match expr {
            syn::Expr::Call(call) => {
                let syn::Expr::Path(path) = call.func.as_ref() else {
                    return None;
                };
                if path.qself.is_some() || !plain(&path.path) {
                    return None;
                }
                match self.resolve_call(path) {
                    PathCallOutcome::Resolved(target) => self.map.output_type(self.crates, &target),
                    PathCallOutcome::Unresolved("constructor") => {
                        let all = segments(&path.path);
                        // A tuple struct names its type; an enum variant, its enum.
                        let type_path = if all.len() == 1 {
                            &all[..]
                        } else {
                            &all[..all.len() - 1]
                        };
                        non_trait(self.map.resolve_type_path(
                            self.crates,
                            self.scope(),
                            type_path,
                            path.path.leading_colon.is_some(),
                            0,
                        )?)
                    }
                    _ => None,
                }
            }
            syn::Expr::Struct(literal) if literal.qself.is_none() && plain(&literal.path) => {
                non_trait(self.map.resolve_type_path(
                    self.crates,
                    self.scope(),
                    &segments(&literal.path),
                    literal.path.leading_colon.is_some(),
                    0,
                )?)
            }
            _ => None,
        }
    }

    /// G139: the parameters and top-level `let` bindings of a function whose declared type
    /// decides a method call on them -- bound exactly once in the whole body, closures included.
    fn typed_locals(
        &self,
        sig: &syn::Signature,
        block: &syn::Block,
        generics: &BTreeSet<String>,
    ) -> BTreeMap<String, TypedLocal> {
        let mut counts = BindingCounts::default();
        for input in &sig.inputs {
            counts.visit_fn_arg(input);
        }
        counts.visit_block(block);
        let mut declared: Vec<(&syn::Pat, &syn::Type, (usize, usize))> = sig
            .inputs
            .iter()
            .filter_map(|input| match input {
                syn::FnArg::Typed(arg) => Some((arg.pat.as_ref(), arg.ty.as_ref(), (0, 0))),
                syn::FnArg::Receiver(_) => None,
            })
            .collect();
        for stmt in &block.stmts {
            if let syn::Stmt::Local(local) = stmt
                && let syn::Pat::Type(typed) = &local.pat
            {
                declared.push((
                    typed.pat.as_ref(),
                    typed.ty.as_ref(),
                    start(local.let_token.span),
                ));
            }
        }
        let mut typed = BTreeMap::new();
        for (pat, ty, from) in declared {
            let syn::Pat::Ident(ident) = pat else {
                continue;
            };
            if ident.by_ref.is_some() || ident.subpat.is_some() {
                continue;
            }
            let name = ident.ident.to_string();
            if counts.0.get(&name) != Some(&1) {
                continue;
            }
            if let Some((ty, form)) = self.declared_receiver_type(ty, generics, &sig.generics) {
                typed.insert(
                    name,
                    TypedLocal {
                        ty,
                        form,
                        from,
                        until: FN_END,
                    },
                );
            }
        }
        typed
    }

    /// `T`, `&T` or `&mut T` for a workspace type `T` spelled without generic arguments (or
    /// `Self` in an impl of such a type), with the receiver form the spelling gives; (G140) the
    /// same forms of `dyn Tr`, `impl Tr` or a function generic `T: Tr`, known by their bounds.
    fn declared_receiver_type(
        &self,
        ty: &syn::Type,
        generics: &BTreeSet<String>,
        fn_generics: &syn::Generics,
    ) -> Option<(LocalType, Receiver)> {
        let (form, inner) = match ty {
            syn::Type::Reference(reference) => (
                if reference.mutability.is_some() {
                    Receiver::RefMut
                } else {
                    Receiver::Ref
                },
                reference.elem.as_ref(),
            ),
            other => (Receiver::Value, other),
        };
        let path = match inner {
            syn::Type::TraitObject(object) if form != Receiver::Value => {
                if self.map.dyn_inherent_impl {
                    return None;
                }
                return Some((LocalType::Bounded(self.trait_bounds(&object.bounds)?), form));
            }
            syn::Type::ImplTrait(opaque) => {
                return Some((LocalType::Bounded(self.trait_bounds(&opaque.bounds)?), form));
            }
            syn::Type::Path(path) => path,
            _ => return None,
        };
        if path.qself.is_none()
            && let Some(param) = path.path.get_ident()
            && generics.contains(&param.to_string())
        {
            // Only the function's own generics carry their bounds here.
            let mut bounds =
                syn::punctuated::Punctuated::<syn::TypeParamBound, syn::Token![+]>::new();
            let mut own = false;
            for generic in &fn_generics.params {
                if let syn::GenericParam::Type(tp) = generic
                    && tp.ident == *param
                {
                    own = true;
                    bounds.extend(tp.bounds.iter().cloned());
                }
            }
            for predicate in fn_generics.where_clause.iter().flat_map(|w| &w.predicates) {
                if let syn::WherePredicate::Type(pt) = predicate
                    && let syn::Type::Path(bounded) = &pt.bounded_ty
                    && bounded.path.is_ident(param)
                {
                    bounds.extend(pt.bounds.iter().cloned());
                }
            }
            if !own {
                return None;
            }
            return Some((LocalType::Bounded(self.trait_bounds(&bounds)?), form));
        }
        // G144: a std type (its arguments do not change which inherent method a name denotes).
        if path.qself.is_none()
            && path
                .path
                .segments
                .iter()
                .rev()
                .skip(1)
                .all(|s| s.arguments.is_empty())
        {
            let mut bare = path.path.clone();
            if let Some(last) = bare.segments.last_mut() {
                last.arguments = syn::PathArguments::None;
            }
            if let Some(canonical) = self.canonical_path_type(&bare)
                && STD_INHERENT.iter().any(|(t, _, _)| *t == canonical)
            {
                return Some((LocalType::Std(canonical), form));
            }
        }
        if path.qself.is_some()
            || path
                .path
                .segments
                .iter()
                .any(|s| !matches!(without_lifetimes(&s.arguments), syn::PathArguments::None))
        {
            return None;
        }
        let segments: Vec<String> = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        if let [only] = segments.as_slice() {
            if generics.contains(only) {
                return None;
            }
            if only == "Self" {
                return match self.impl_self.last() {
                    Some(Some(ImplCtx {
                        ty: Some(ty),
                        shape,
                        ..
                    })) if shape == PLAIN_IMPL_SHAPE => Some((LocalType::Concrete(*ty), form)),
                    _ => None,
                };
            }
        }
        let ty = self.map.resolve_type_path(
            self.crates,
            self.scope(),
            &segments,
            path.path.leading_colon.is_some(),
            0,
        )?;
        if self.map.types[ty].is_trait {
            return None;
        }
        Some((LocalType::Concrete(ty), form))
    }

    /// The workspace traits of a bound list, when every bound is one or a method-less marker
    /// (`Send`, `Sync`, `Unpin`, `Sized`, `Copy`, `?Sized`, a lifetime). Any other bound could
    /// supply a method of the same name, so nothing is claimed.
    fn trait_bounds(
        &self,
        bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>,
    ) -> Option<Vec<TypeId>> {
        const MARKERS: [&str; 5] = ["Send", "Sync", "Unpin", "Sized", "Copy"];
        let mut traits = Vec::new();
        for bound in bounds {
            match bound {
                syn::TypeParamBound::Lifetime(_) => {}
                syn::TypeParamBound::Trait(tb) => {
                    if tb.maybe.is_some() {
                        continue;
                    }
                    let segments: Vec<String> = tb
                        .path
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect();
                    match self.map.resolve_type_path(
                        self.crates,
                        self.scope(),
                        &segments,
                        tb.path.leading_colon.is_some(),
                        0,
                    ) {
                        Some(ty) if self.map.types[ty].is_trait => traits.push(ty),
                        Some(_) => return None,
                        None if MARKERS.contains(&segments.last()?.as_str()) => {}
                        None => return None,
                    }
                }
                _ => return None,
            }
        }
        (!traits.is_empty()).then_some(traits)
    }

    fn resolve_call(&self, path: &syn::ExprPath) -> PathCallOutcome {
        if path.qself.is_some() {
            return PathCallOutcome::Unresolved("qualified-self");
        }
        let segments: Vec<String> = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        let leading = path.path.leading_colon.is_some();
        let Some(ctx) = self.fn_ctx.last() else {
            return PathCallOutcome::Unresolved("outside-function");
        };
        let scope = self.scope();
        if segments.len() == 1 && !leading {
            if segments[0] == "Self" {
                return PathCallOutcome::Unresolved("constructor");
            }
            if ctx.locals.contains(&segments[0]) {
                return PathCallOutcome::Unresolved("local-binding");
            }
            return match self.map.lexical_lookup(scope, Ns::Values, &segments[0]) {
                Lookup::Found(def) => outcome_of(def),
                Lookup::Open => PathCallOutcome::Unresolved("open-scope"),
                // G157: `drop` is the prelude's `std::mem::drop` when no scope defines it.
                Lookup::Missing if segments[0] == "drop" => {
                    PathCallOutcome::External("std::mem::drop".into())
                }
                Lookup::Missing => PathCallOutcome::External(String::new()),
            };
        }
        let first = &segments[0];
        if ctx.generics.contains(first) {
            return PathCallOutcome::Unresolved("generic-parameter");
        }
        let last = segments.last().expect("non-empty path");
        if first == "Self" && !leading {
            if segments.len() != 2 {
                return PathCallOutcome::Unresolved("associated-path");
            }
            return match self.impl_self.last() {
                Some(Some(ImplCtx { ty: Some(ty), .. })) => match self.map.associated(*ty, last) {
                    Ok(target) => PathCallOutcome::Resolved(target),
                    Err(reason) => PathCallOutcome::Unresolved(reason),
                },
                Some(Some(ImplCtx { ty: None, .. })) => {
                    PathCallOutcome::Unresolved("unresolved-self-type")
                }
                _ => PathCallOutcome::Unresolved("trait-method"),
            };
        }
        let Some(prefix) = self
            .map
            .resolve_prefix(self.crates, scope, &segments, leading, false)
        else {
            return PathCallOutcome::Unresolved("unresolved-prefix");
        };
        match prefix {
            Def::Module(module) => match self.map.modules[module].scope.values.get(last) {
                Some(entry) if self.map.visible(&entry.vis, scope) => {
                    if entry.origin == Origin::Glob && self.map.uncertain(module, last) {
                        PathCallOutcome::Unresolved("open-scope")
                    } else {
                        outcome_of(entry.def.clone())
                    }
                }
                _ if self.map.uncertain(module, last) => PathCallOutcome::Unresolved("open-scope"),
                _ => PathCallOutcome::Unresolved("not-found"),
            },
            Def::Type(_) => {
                let type_segments = &segments[..segments.len() - 1];
                match self
                    .map
                    .resolve_type_path(self.crates, scope, type_segments, leading, 0)
                {
                    Some(ty) => match self.map.associated(ty, last) {
                        Ok(target) => PathCallOutcome::Resolved(target),
                        Err(reason) => PathCallOutcome::Unresolved(reason),
                    },
                    None => PathCallOutcome::Unresolved("unresolved-type"),
                }
            }
            Def::External(path) => external(&extend(&path, last)),
            Def::Ambiguous => PathCallOutcome::Unresolved("ambiguous"),
            Def::Unknown => PathCallOutcome::Unresolved("unresolved-import"),
            _ => PathCallOutcome::Unresolved("not-a-path-prefix"),
        }
    }
}

/// `path` followed by `segment`; an unknown (empty) path stays unknown.
fn extend(path: &[String], segment: &str) -> Vec<String> {
    if path.is_empty() {
        Vec::new()
    } else {
        let mut path = path.to_vec();
        path.push(segment.to_owned());
        path
    }
}

fn external(path: &[String]) -> PathCallOutcome {
    PathCallOutcome::External(path.join("::"))
}

fn outcome_of(def: Def) -> PathCallOutcome {
    match def {
        Def::Fn(target) => PathCallOutcome::Resolved(target),
        Def::Ctor => PathCallOutcome::Unresolved("constructor"),
        Def::Ambiguous => PathCallOutcome::Unresolved("ambiguous"),
        Def::External(path) => external(&path),
        Def::Value => PathCallOutcome::Unresolved("non-function-value"),
        Def::Unknown => PathCallOutcome::Unresolved("unresolved-import"),
        Def::Module(_) | Def::Type(_) => PathCallOutcome::Unresolved("not-a-value"),
    }
}

impl<'ast> Visit<'ast> for CallWalker<'_> {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let Some((_, items)) = &item.content else {
            return;
        };
        let Some(Entry {
            def: Def::Module(child),
            ..
        }) = self.map.modules[self.scope()]
            .items
            .types
            .get(&item.ident.to_string())
        else {
            return;
        };
        self.scopes.push(*child);
        for nested in items {
            self.visit_item(nested);
        }
        self.scopes.pop();
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.impl_self.push(None);
        self.enter_fn(&item.sig, &item.block);
        self.visit_signature(&item.sig);
        self.visit_block(&item.block);
        self.fn_ctx.pop();
        self.impl_self.pop();
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        let resolved = match item.self_ty.as_ref() {
            syn::Type::Path(path) if path.qself.is_none() => {
                let segments: Vec<String> = path
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                self.map.resolve_type_path(
                    self.crates,
                    self.scope(),
                    &segments,
                    path.path.leading_colon.is_some(),
                    0,
                )
            }
            _ => None,
        };
        self.impl_self.push(Some(ImplCtx {
            ty: resolved,
            generics: generic_names(&item.generics),
            shape: impl_shape(item),
        }));
        self.visit_type(&item.self_ty);
        for impl_item in &item.items {
            match impl_item {
                syn::ImplItem::Fn(method) => {
                    self.enter_fn(&method.sig, &method.block);
                    self.visit_signature(&method.sig);
                    self.visit_block(&method.block);
                    self.fn_ctx.pop();
                }
                syn::ImplItem::Const(constant) => self.visit_type(&constant.ty),
                syn::ImplItem::Type(assoc) => self.visit_type(&assoc.ty),
                _ => {}
            }
        }
        self.impl_self.pop();
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        let this_trait = self
            .map
            .resolve_type_path(
                self.crates,
                self.scope(),
                &[item.ident.to_string()],
                false,
                0,
            )
            .filter(|ty| self.map.types[*ty].is_trait);
        self.impl_self.push(None);
        self.item_generics.push(generic_names(&item.generics));
        for trait_item in &item.items {
            match trait_item {
                syn::TraitItem::Fn(method) => match &method.default {
                    Some(block) => {
                        self.enter_fn(&method.sig, block);
                        // G140: `self` in a default body is known only as `Self: ThisTrait`.
                        if let (Some(ty), Some(form), Some(ctx)) =
                            (this_trait, receiver_of(&method.sig), self.fn_ctx.last_mut())
                        {
                            ctx.typed.insert(
                                "self".into(),
                                TypedLocal {
                                    ty: LocalType::Bounded(vec![ty]),
                                    form,
                                    from: (0, 0),
                                    until: FN_END,
                                },
                            );
                        }
                        self.visit_signature(&method.sig);
                        self.visit_block(block);
                        self.fn_ctx.pop();
                    }
                    None => {
                        self.item_generics.push(generic_names(&method.sig.generics));
                        self.visit_signature(&method.sig);
                        self.item_generics.pop();
                    }
                },
                syn::TraitItem::Const(constant) => self.visit_type(&constant.ty),
                syn::TraitItem::Type(assoc) => {
                    if let Some((_, default)) = &assoc.default {
                        self.visit_type(default);
                    }
                }
                _ => {}
            }
        }
        self.item_generics.pop();
        self.impl_self.pop();
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        self.item_generics.push(generic_names(&item.generics));
        syn::visit::visit_item_struct(self, item);
        self.item_generics.pop();
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        self.item_generics.push(generic_names(&item.generics));
        syn::visit::visit_item_enum(self, item);
        self.item_generics.pop();
    }

    fn visit_item_union(&mut self, item: &'ast syn::ItemUnion) {
        self.item_generics.push(generic_names(&item.generics));
        syn::visit::visit_item_union(self, item);
        self.item_generics.pop();
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        self.item_generics.push(generic_names(&item.generics));
        syn::visit::visit_item_type(self, item);
        self.item_generics.pop();
    }

    /// Every type occurrence (G83), outermost first; nested types are recorded too.
    fn visit_type(&mut self, ty: &'ast syn::Type) {
        let (line, column) = start(ty.span());
        self.types.push(TypeResolution {
            path: self.file.to_owned(),
            line,
            column,
            spelling: super::spelling::type_spelling(ty),
            canonical: self.canonical_type(ty),
        });
        syn::visit::visit_type(self, ty);
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        let (line, column) = start(block.brace_token.span.open());
        let scoped = self
            .map
            .blocks
            .get(&(self.file.to_owned(), line, column))
            .copied();
        if let Some(scope) = scoped {
            self.scopes.push(scope);
        }
        syn::visit::visit_block(self, block);
        if scoped.is_some() {
            self.scopes.pop();
        }
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = call.func.as_ref()
            && self.fn_ctx.last().is_some()
            && let Some(last) = path.path.segments.last()
        {
            let (line, column) = start(last.ident.span());
            let callee = path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            self.out.push(PathCallResolution {
                path: self.file.to_owned(),
                line,
                column,
                callee,
                outcome: self.resolve_call(path),
            });
        }
        syn::visit::visit_expr_call(self, call);
    }

    /// A method call whose receiver's type is known: `self.m(..)` in an impl method (G79), a typed
    /// local (G139, G140, G142), a field of one or a call result (G143).
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if let Some((callee, outcome)) = self.method_outcome(call) {
            let (line, column) = start(call.method.span());
            self.out.push(PathCallResolution {
                path: self.file.to_owned(),
                line,
                column,
                callee,
                outcome,
            });
        }
        syn::visit::visit_expr_method_call(self, call);
    }

    // G133 (NA-CLOSURE-REGIONS): a closure body is its own executable region inside the CALL
    // profile. Its locals are the enclosing region's (captures) plus its own parameters and
    // bindings, so a parameter shadowing a function name stays a local.
    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        let Some(enclosing) = self.fn_ctx.last() else {
            return;
        };
        let mut bindings = Bindings(enclosing.locals.clone());
        for input in &closure.inputs {
            bindings.visit_pat(input);
        }
        bindings.visit_expr(&closure.body);
        let ctx = FnCtx {
            locals: bindings.0,
            generics: enclosing.generics.clone(),
            receiver: enclosing.receiver,
            // Bound once in the whole function, closures included: the same local here.
            typed: enclosing.typed.clone(),
        };
        self.fn_ctx.push(ctx);
        for input in &closure.inputs {
            self.visit_pat(input);
        }
        if let syn::ReturnType::Type(_, ty) = &closure.output {
            self.visit_type(ty);
        }
        self.visit_expr(&closure.body);
        self.fn_ctx.pop();
    }
    // The CALL profile's exclusions: `async` blocks and initializers with no caller.
    fn visit_expr_async(&mut self, _: &'ast syn::ExprAsync) {}
    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        self.visit_type(&item.ty);
    }
    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        self.visit_type(&item.ty);
    }
}

#[cfg(test)]
mod tests;
