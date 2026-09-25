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
//! Method calls need the receiver's type and are not resolved here at all.

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
    Unresolved(&'static str),
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

/// Every path call of the workspace, and the files the crate roots reach (a file no root reaches
/// was never evaluated).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceResolution {
    pub reached: BTreeSet<String>,
    pub calls: Vec<PathCallResolution>,
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
    for (file, module) in map.file_modules.clone() {
        if let Some(ast) = parsed.get(&file) {
            let mut walker = CallWalker {
                map: &map,
                crates,
                file: &file,
                scopes: vec![module],
                impl_self: Vec::new(),
                fn_ctx: Vec::new(),
                out: &mut out,
            };
            walker.visit_file(ast);
        }
    }
    out.sort_by(|a, b| (&a.path, a.line, a.column).cmp(&(&b.path, b.line, b.column)));
    WorkspaceResolution {
        reached: parsed.into_keys().collect(),
        calls: out,
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
    External,
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
}

#[derive(Debug, Clone)]
struct TypeDef {
    scope: ModId,
    variants: BTreeSet<String>,
    is_trait: bool,
    alias_of: Option<(Vec<String>, bool)>,
}

#[derive(Debug, Clone)]
struct ImplDef {
    scope: ModId,
    self_path: Vec<String>,
    self_leading_colon: bool,
    is_trait_impl: bool,
    fns: Vec<(String, FnTarget)>,
    self_type: Option<TypeId>,
}

#[derive(Default)]
struct DefMap {
    modules: Vec<Module>,
    types: Vec<TypeDef>,
    impls: Vec<ImplDef>,
    /// Each parsed file's top module.
    file_modules: BTreeMap<String, ModId>,
    /// Block scopes by (file, line, column) of the block's opening brace.
    blocks: BTreeMap<(String, usize, usize), ModId>,
    crate_roots: BTreeMap<usize, ModId>,
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
                    self.add_item(module, Ns::Values, &name, Def::Fn(target), &item_fn.vis);
                    self.collect_block(module, file, &item_fn.block);
                }
                syn::Item::Struct(item) => {
                    let ty = self.new_type(module, BTreeSet::new(), false, None);
                    let name = item.ident.to_string();
                    self.add_item(module, Ns::Types, &name, Def::Type(ty), &item.vis);
                    if !matches!(item.fields, syn::Fields::Named(_)) {
                        self.add_item(module, Ns::Values, &name, Def::Ctor, &item.vis);
                    }
                }
                syn::Item::Enum(item) => {
                    let variants = item.variants.iter().map(|v| v.ident.to_string()).collect();
                    let ty = self.new_type(module, variants, false, None);
                    self.add_item(
                        module,
                        Ns::Types,
                        &item.ident.to_string(),
                        Def::Type(ty),
                        &item.vis,
                    );
                }
                syn::Item::Union(item) => {
                    let ty = self.new_type(module, BTreeSet::new(), false, None);
                    self.add_item(
                        module,
                        Ns::Types,
                        &item.ident.to_string(),
                        Def::Type(ty),
                        &item.vis,
                    );
                }
                syn::Item::Trait(item) => {
                    let ty = self.new_type(module, BTreeSet::new(), true, None);
                    self.add_item(
                        module,
                        Ns::Types,
                        &item.ident.to_string(),
                        Def::Type(ty),
                        &item.vis,
                    );
                    for trait_item in &item.items {
                        if let syn::TraitItem::Fn(method) = trait_item
                            && let Some(block) = &method.default
                        {
                            self.collect_block(module, file, block);
                        }
                    }
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
                    let ty = self.new_type(module, BTreeSet::new(), false, alias);
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
                    self.add_item(module, Ns::Types, &name, Def::External, &item.vis);
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
        let mut fns = Vec::new();
        for impl_item in &item.items {
            if let syn::ImplItem::Fn(method) = impl_item {
                let (line, column) = start(method.span());
                let name = method.sig.ident.to_string();
                fns.push((
                    name.clone(),
                    FnTarget {
                        path: file.to_owned(),
                        line,
                        column,
                        name,
                    },
                ));
                self.collect_block(scope, file, &method.block);
            }
        }
        self.impls.push(ImplDef {
            scope,
            self_path,
            self_leading_colon,
            is_trait_impl: item.trait_.is_some(),
            fns,
            self_type: None,
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
        variants: BTreeSet<String>,
        is_trait: bool,
        alias_of: Option<(Vec<String>, bool)>,
    ) -> TypeId {
        self.types.push(TypeDef {
            scope,
            variants,
            is_trait,
            alias_of,
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
                        Def::External => Some(Def::External),
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
                Def::External => Def::External,
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
            None => Def::External,
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
                .filter(|(fn_name, _)| fn_name == name)
                .map(|(_, target)| target)
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
}

struct CallWalker<'a> {
    map: &'a DefMap,
    crates: &'a [CrateInput],
    file: &'a str,
    scopes: Vec<ModId>,
    /// The enclosing impl's self type (`None` inside a trait or an impl of a non-path type).
    impl_self: Vec<Option<(Option<TypeId>, BTreeSet<String>)>>,
    fn_ctx: Vec<FnCtx>,
    out: &'a mut Vec<PathCallResolution>,
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

    fn enter_fn(&mut self, sig: &syn::Signature, block: &syn::Block) {
        let mut bindings = Bindings(BTreeSet::new());
        for input in &sig.inputs {
            bindings.visit_fn_arg(input);
        }
        bindings.visit_block(block);
        let mut generics = generic_names(&sig.generics);
        if let Some(Some((_, outer))) = self.impl_self.last() {
            generics.extend(outer.iter().cloned());
        }
        self.fn_ctx.push(FnCtx {
            locals: bindings.0,
            generics,
        });
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
                Lookup::Missing => PathCallOutcome::Unresolved("external"),
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
                Some(Some((Some(ty), _))) => match self.map.associated(*ty, last) {
                    Ok(target) => PathCallOutcome::Resolved(target),
                    Err(reason) => PathCallOutcome::Unresolved(reason),
                },
                Some(Some((None, _))) => PathCallOutcome::Unresolved("unresolved-self-type"),
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
            Def::External => PathCallOutcome::Unresolved("external"),
            Def::Ambiguous => PathCallOutcome::Unresolved("ambiguous"),
            Def::Unknown => PathCallOutcome::Unresolved("unresolved-import"),
            _ => PathCallOutcome::Unresolved("not-a-path-prefix"),
        }
    }
}

fn outcome_of(def: Def) -> PathCallOutcome {
    match def {
        Def::Fn(target) => PathCallOutcome::Resolved(target),
        Def::Ctor => PathCallOutcome::Unresolved("constructor"),
        Def::Ambiguous => PathCallOutcome::Unresolved("ambiguous"),
        Def::External => PathCallOutcome::Unresolved("external"),
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
        self.impl_self
            .push(Some((resolved, generic_names(&item.generics))));
        for impl_item in &item.items {
            if let syn::ImplItem::Fn(method) = impl_item {
                self.enter_fn(&method.sig, &method.block);
                self.visit_block(&method.block);
                self.fn_ctx.pop();
            }
        }
        self.impl_self.pop();
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.impl_self.push(None);
        for trait_item in &item.items {
            if let syn::TraitItem::Fn(method) = trait_item
                && let Some(block) = &method.default
            {
                self.enter_fn(&method.sig, block);
                self.visit_block(block);
                self.fn_ctx.pop();
            }
        }
        self.impl_self.pop();
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

    // The CALL profile's exclusions: deferred executable regions and initializers with no caller.
    fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}
    fn visit_expr_async(&mut self, _: &'ast syn::ExprAsync) {}
    fn visit_item_const(&mut self, _: &'ast syn::ItemConst) {}
    fn visit_item_static(&mut self, _: &'ast syn::ItemStatic) {}
}

#[cfg(test)]
mod tests;
