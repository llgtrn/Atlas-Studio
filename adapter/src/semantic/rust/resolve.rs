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
    map.resolve_imports(crates);
    map.resolve_impls(crates);
    let mut out = Vec::new();
    let mut types = Vec::new();
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
            };
            walker.visit_file(ast);
        }
    }
    out.sort_by(|a, b| (&a.path, a.line, a.column).cmp(&(&b.path, b.line, b.column)));
    types.sort_by(|a, b| (&a.path, a.line, a.column).cmp(&(&b.path, b.line, b.column)));
    WorkspaceResolution {
        reached: parsed.into_keys().collect(),
        calls: out,
        types,
    }
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
    /// Names may exist that this pass cannot see (item macros, an unresolvable glob, a missing
    /// or unparsable module file).
    open: bool,
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

fn impl_shape(item: &syn::ItemImpl) -> String {
    use quote::ToTokens;
    let arguments = match item.self_ty.as_ref() {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.arguments.to_token_stream().to_string())
            .unwrap_or_default(),
        other => other.to_token_stream().to_string(),
    };
    format!(
        "{} {} | {arguments}",
        item.generics.to_token_stream(),
        item.generics.where_clause.to_token_stream()
    )
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
    /// An inherent impl on a trait object (`impl dyn Tr {}`) exists somewhere: its methods would
    /// compete with the trait's, so no call through `dyn` is claimed.
    dyn_inherent_impl: bool,
    /// Each parsed file's top module.
    file_modules: BTreeMap<String, ModId>,
    /// Block scopes by (file, line, column) of the block's opening brace.
    blocks: BTreeMap<(String, usize, usize), ModId>,
    crate_roots: BTreeMap<usize, ModId>,
    crate_packages: BTreeMap<usize, String>,
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
                // An item-position macro may define any name.
                syn::Item::Macro(item) if item.ident.is_none() => self.modules[module].open = true,
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
    fn resolve_imports(&mut self, crates: &[CrateInput]) {
        for module in &mut self.modules {
            module.scope = module.items.clone();
        }
        for _ in 0..64 {
            let mut next: Vec<Scope> = self.modules.iter().map(|m| m.items.clone()).collect();
            let mut opened = vec![false; self.modules.len()];
            for (id, module) in self.modules.iter().enumerate() {
                for import in &module.imports {
                    self.apply_import(crates, id, import, &mut next[id], &mut opened[id]);
                }
            }
            let mut changed = false;
            for (id, scope) in next.into_iter().enumerate() {
                if self.modules[id].scope != scope {
                    changed = true;
                    self.modules[id].scope = scope;
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
                if entry.origin == Origin::Glob && module.open {
                    return Lookup::Open;
                }
                return Lookup::Found(entry.def.clone());
            }
            if module.open {
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
    /// path without generic arguments to a non-trait workspace type that is not a generic name in
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
                .any(|s| !matches!(s.arguments, syn::PathArguments::None))
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

    /// G142: whether the probe's autoref step can be claimed for `ty.name(..)` from `scope`: no
    /// by-value method of that name can come first. A by-value workspace trait method, a std
    /// blanket by-value method (`into`, `try_into`, `into_iter`), a non-std import or an unseen
    /// name in the scope chain, or an explicit impl of a std trait with by-value methods on `ty`
    /// could each supply one; any of them withholds the claim.
    fn autoref_allowed(&self, scope: ModId, ty: TypeId, name: &str) -> bool {
        const BLANKET: [&str; 3] = ["into", "try_into", "into_iter"];
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
        if self.impls.iter().any(|imp| {
            imp.self_type == Some(ty)
                && imp
                    .trait_name
                    .as_deref()
                    .is_some_and(|t| BY_VALUE_TRAITS.contains(&t))
        }) {
            return false;
        }
        let mut cursor = Some(scope);
        while let Some(id) = cursor {
            let module = &self.modules[id];
            if module.open
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
}

/// What a declared type says about a receiver.
#[derive(Clone)]
enum LocalType {
    /// A plain workspace type (G139).
    Concrete(TypeId),
    /// A type known only through workspace traits (G140).
    Bounded(Vec<TypeId>),
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
                    let open = self.map.modules[module].open;
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
        let mut found = Vec::new();
        for stmt in &block.stmts {
            let syn::Stmt::Local(local) = stmt else {
                continue;
            };
            let syn::Pat::Ident(ident) = &local.pat else {
                continue;
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
            let Some(init) = &local.init else {
                continue;
            };
            if let Some(ty) = self.value_type(&init.expr) {
                found.push((name, ty, start(local.let_token.span)));
            }
        }
        if let Some(ctx) = self.fn_ctx.last_mut() {
            for (name, ty, from) in found {
                ctx.typed.insert(
                    name,
                    TypedLocal {
                        ty: LocalType::Concrete(ty),
                        form: Receiver::Value,
                        from,
                    },
                );
            }
        }
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
                typed.insert(name, TypedLocal { ty, form, from });
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
        if path.qself.is_some()
            || path
                .path
                .segments
                .iter()
                .any(|s| !matches!(s.arguments, syn::PathArguments::None))
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
                    if entry.origin == Origin::Glob && self.map.modules[module].open {
                        PathCallOutcome::Unresolved("open-scope")
                    } else {
                        outcome_of(entry.def.clone())
                    }
                }
                _ if self.map.modules[module].open => PathCallOutcome::Unresolved("open-scope"),
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

    /// `self.m(..)` inside an impl method (G79): the receiver's type is the impl's self type.
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if let syn::Expr::Path(receiver) = call.receiver.as_ref()
            && receiver.path.is_ident("self")
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
            let (line, column) = start(call.method.span());
            let name = call.method.to_string();
            self.out.push(PathCallResolution {
                path: self.file.to_owned(),
                line,
                column,
                callee: format!("self.{name}"),
                outcome: self.map.method_on_self(
                    *ty,
                    shape,
                    &name,
                    *form,
                    self.map.autoref_allowed(self.scope(), *ty, &name),
                ),
            });
        } else if let syn::Expr::Path(receiver) = call.receiver.as_ref()
            && receiver.qself.is_none()
            && let Some(local) = receiver.path.get_ident()
            && let Some(ctx) = self.fn_ctx.last()
            && let Some(typed) = ctx.typed.get(&local.to_string())
        {
            // G139 (NA-CALL-TYPE-RESIDUAL): a local whose declared type is a plain workspace
            // type. The probe's first step is the same as for `self`: the inherent method whose
            // receiver form equals the local's declared form wins, ahead of every trait.
            let (line, column) = start(call.method.span());
            if (line, column) > typed.from {
                let name = call.method.to_string();
                let outcome = match &typed.ty {
                    LocalType::Concrete(ty) => self.map.method_on_self(
                        *ty,
                        PLAIN_IMPL_SHAPE,
                        &name,
                        typed.form,
                        self.map.autoref_allowed(self.scope(), *ty, &name),
                    ),
                    LocalType::Bounded(bounds) => {
                        self.map.method_on_bounds(bounds, &name, typed.form)
                    }
                };
                self.out.push(PathCallResolution {
                    path: self.file.to_owned(),
                    line,
                    column,
                    callee: format!("{local}.{name}"),
                    outcome,
                });
            }
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
