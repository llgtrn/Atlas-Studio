//! G177 (NA-DEPENDENCY-DECLARATIONS): what one registry item can bring into scope as a trait.
//!
//! G173 answers per package: the union over its lock closure, UNKNOWN when anything in it could
//! declare a trait unseen (serde's build-script `include!`). An import names one item, though,
//! and a name a module binds by a named item or a named `use` is certain whatever unseen
//! expansions sit beside it: an unseen item of that name in the same namespace would not compile
//! (E0255 against a `use`, E0252 between imports, E0428 between items). So the item is resolved
//! through its package's module tree, following named `use`s across locked packages, to its
//! definition, which answers its own trait methods (a supertrait's are not in scope through a
//! subtrait import), none when it is no trait, or UNKNOWN.
//!
//! A binding counts only when this reading sees it: an item or `use` written in the module, or one
//! that the transcriber of a local `macro_rules!` with a single empty rule spells where an empty
//! item-position invocation expands it (the definition in textual scope there, same file or a
//! `#[macro_use] mod`). A name reached only through a glob or an unseen expansion is UNKNOWN, and
//! so is one bound only in the value namespace: a `fn` beside an unseen `trait` of its name
//! compiles. A procedural macro crate exports nothing in the type namespace.
//!
//! `cfg` predicates are kept, and every assignment of the atoms a resolution depends on is
//! resolved (atoms taken as independent, a superset of the real builds): the answer is the union,
//! UNKNOWN if any assignment is. An assignment under which a module file on the path is missing
//! does not compile and answers nothing.

use super::{BUILTIN_ATTRIBUTES, TOOL_NAMESPACES, cfg_false, item_attrs, unraw};
use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    rc::Rc,
};
use syn::punctuated::Punctuated;

/// Assignments one answer may try before it is UNKNOWN.
const MAX_ASSIGNMENTS: usize = 256;
/// Nested expansions, module files and `use` hops one reading follows before it gives up.
const MAX_DEPTH: usize = 32;

/// A `cfg` predicate, kept for evaluation under each assignment of its atoms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Cfg {
    True,
    False,
    Atom(String),
    Not(Box<Cfg>),
    All(Vec<Cfg>),
    Any(Vec<Cfg>),
}

impl Cfg {
    fn and(self, other: Cfg) -> Cfg {
        match (self, other) {
            (Cfg::False, _) | (_, Cfg::False) => Cfg::False,
            (Cfg::True, c) | (c, Cfg::True) => c,
            (a, b) => Cfg::All(vec![a, b]),
        }
    }

    fn not(self) -> Cfg {
        match self {
            Cfg::True => Cfg::False,
            Cfg::False => Cfg::True,
            c => Cfg::Not(Box::new(c)),
        }
    }

    fn any(parts: Vec<Cfg>) -> Cfg {
        let mut parts: Vec<Cfg> = parts.into_iter().filter(|p| *p != Cfg::False).collect();
        if parts.contains(&Cfg::True) {
            Cfg::True
        } else if parts.len() <= 1 {
            parts.pop().unwrap_or(Cfg::False)
        } else {
            Cfg::Any(parts)
        }
    }

    /// Its value under `assumed`, or an atom it needs that `assumed` does not decide.
    fn eval(&self, assumed: &BTreeMap<String, bool>) -> Result<bool, String> {
        match self {
            Cfg::True => Ok(true),
            Cfg::False => Ok(false),
            Cfg::Atom(atom) => assumed.get(atom).copied().ok_or_else(|| atom.clone()),
            Cfg::Not(inner) => inner.eval(assumed).map(|v| !v),
            Cfg::All(parts) | Cfg::Any(parts) => {
                let decisive = matches!(self, Cfg::Any(_));
                let mut undecided = None;
                for part in parts {
                    match part.eval(assumed) {
                        Ok(value) if value == decisive => return Ok(decisive),
                        Ok(_) => {}
                        Err(atom) => {
                            undecided.get_or_insert(atom);
                        }
                    }
                }
                undecided.map_or(Ok(!decisive), Err)
            }
        }
    }
}

/// A `cfg(..)` predicate; a feature no build under this lock can enable is false.
fn cfg_of(meta: &syn::Meta, impossible: &BTreeSet<String>) -> Cfg {
    if cfg_false(meta, impossible) {
        return Cfg::False;
    }
    let atom = || Cfg::Atom(meta.to_token_stream().to_string());
    let syn::Meta::List(list) = meta else {
        return atom();
    };
    let Ok(parts) = list.parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
    else {
        return atom();
    };
    let parts: Vec<Cfg> = parts.iter().map(|p| cfg_of(p, impossible)).collect();
    match list.path.get_ident().map(|i| i.to_string()).as_deref() {
        Some("all") => parts.into_iter().fold(Cfg::True, Cfg::and),
        Some("any") => Cfg::any(parts),
        Some("not") if parts.len() == 1 => parts.into_iter().fold(Cfg::True, Cfg::and).not(),
        _ => atom(),
    }
}

/// What an item's attributes decide for this reading.
struct Attrs {
    /// When the item is compiled (`cfg`, and a `cfg` a `cfg_attr` applies).
    when: Cfg,
    /// Each `#[path]` value, with when it applies.
    paths: Vec<(Cfg, String)>,
    /// When a `#[macro_use]` applies.
    macro_use: Cfg,
    /// An attribute this reading does not know: a procedural macro may rewrite the item.
    rewrites: bool,
    /// A derive: its output is unseen and may define a `macro_rules!`.
    derives: bool,
}

fn attrs_of(attrs: &[syn::Attribute], impossible: &BTreeSet<String>) -> Attrs {
    let mut out = Attrs {
        when: Cfg::True,
        paths: Vec::new(),
        macro_use: Cfg::False,
        rewrites: false,
        derives: false,
    };
    for attr in attrs {
        attribute(&attr.meta, Cfg::True, impossible, &mut out);
    }
    out
}

fn attribute(meta: &syn::Meta, applies: Cfg, impossible: &BTreeSet<String>, out: &mut Attrs) {
    let path = meta.path();
    if path.segments.len() > 1 {
        out.rewrites |= !TOOL_NAMESPACES.contains(&path.segments[0].ident.to_string().as_str());
        return;
    }
    let Some(name) = path.get_ident().map(|i| unraw(&i.to_string())) else {
        out.rewrites = true;
        return;
    };
    let nested = || match meta {
        syn::Meta::List(list) => list
            .parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
            .ok(),
        _ => None,
    };
    match name.as_str() {
        "cfg" => match nested()
            .as_ref()
            .map(|p| p.iter().collect::<Vec<_>>())
            .as_deref()
        {
            Some([predicate]) => {
                let holds = Cfg::any(vec![applies.not(), cfg_of(predicate, impossible)]);
                out.when = std::mem::replace(&mut out.when, Cfg::True).and(holds);
            }
            _ => out.rewrites = true,
        },
        "cfg_attr" => match nested() {
            Some(inner) if !inner.is_empty() => {
                let applies = applies.and(cfg_of(&inner[0], impossible));
                for meta in inner.iter().skip(1) {
                    attribute(meta, applies.clone(), impossible, out);
                }
            }
            _ => out.rewrites = true,
        },
        "unsafe" => match nested() {
            Some(inner) => inner
                .iter()
                .for_each(|meta| attribute(meta, applies.clone(), impossible, out)),
            None => out.rewrites = true,
        },
        "path" => match meta {
            syn::Meta::NameValue(syn::MetaNameValue {
                value:
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(value),
                        ..
                    }),
                ..
            }) => out.paths.push((applies, value.value())),
            _ => out.rewrites = true,
        },
        "macro_use" => {
            out.macro_use = Cfg::any(vec![
                std::mem::replace(&mut out.macro_use, Cfg::False),
                applies,
            ])
        }
        "derive" => out.derives = true,
        other if BUILTIN_ATTRIBUTES.contains(&other) => {}
        _ => out.rewrites = true,
    }
}

/// A type-namespace binding this reading sees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Binding {
    /// A trait: its own method names, or `None` when its body is not fully read.
    Trait(Option<BTreeSet<String>>),
    /// An item that is no trait and holds nothing followed here: a struct, enum, union or type.
    Other,
    /// A module: its content under each `#[path]` alternative.
    Module(Vec<(Cfg, Content)>),
    /// `extern crate name`.
    Crate(String),
    /// A named `use`: the path it imports, written in this module.
    Use {
        segments: Vec<String>,
        leading_colon: bool,
    },
    /// Something whose meaning this reading does not see (an item an attribute may rewrite).
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Content {
    Module(usize),
    /// No such file: a build compiling the declaration fails.
    Missing,
    /// A file this reading does not read.
    Unread,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ModuleNode {
    parent: Option<usize>,
    /// Type-namespace bindings by name, each with when it is compiled.
    types: BTreeMap<String, Vec<(Cfg, Binding)>>,
}

/// One locked package's modules as the per-item reading sees them (module 0 is the root).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ItemTree {
    pub(super) modules: Vec<ModuleNode>,
    /// Crate-root `extern crate` items by the name they bind: each puts that name in the extern
    /// prelude of every module, over the dependency of that name (G177 review).
    pub(super) prelude: BTreeMap<String, Vec<(Cfg, Binding)>>,
    /// Extern-prelude names of its locked dependencies (renames included), to package indices.
    pub(super) externs: BTreeMap<String, usize>,
    pub(super) proc_macro: bool,
    /// Edition 2015: `use` paths start at the crate root.
    pub(super) edition_2015: bool,
    /// Why nothing in it is read.
    pub(super) unread: Option<String>,
}

impl ItemTree {
    pub(super) fn unread(reason: String) -> Self {
        Self {
            unread: Some(reason),
            ..Self::default()
        }
    }

    /// A tree whose root binds each `(name, methods)`: a trait, or `None` for an unknown item.
    #[cfg(test)]
    pub(super) fn stated(items: &[(&str, Option<&[&str]>)]) -> Self {
        let mut root = ModuleNode::default();
        for (name, methods) in items {
            let binding = match methods {
                Some(names) => Binding::Trait(Some(names.iter().map(|n| n.to_string()).collect())),
                None => Binding::Unknown,
            };
            root.types
                .entry(name.to_string())
                .or_default()
                .push((Cfg::True, binding));
        }
        Self {
            modules: vec![root],
            ..Self::default()
        }
    }
}

/// The package sources a tree is built from.
pub(super) struct Sources<'a> {
    pub(super) dir: &'a Path,
    /// Every file read, parsed, by its path relative to `dir`.
    pub(super) files: &'a BTreeMap<String, syn::File>,
    pub(super) impossible: &'a BTreeSet<String>,
    /// The package's index: its `cfg` atoms are its own (G177 review: a feature, a build-script
    /// `--cfg` or a profile setting holds per package, not across the packages a path crosses).
    pub(super) package: usize,
}

/// A `macro_rules!` name in textual scope, or an unseen expansion that may define any.
#[derive(Clone)]
enum Textual {
    Macro {
        name: String,
        when: Cfg,
        /// The items of its single empty rule's transcriber; `None` for any other macro.
        expansion: Option<Rc<Vec<syn::Item>>>,
    },
    Opaque(Cfg),
}

/// Where items are being read: their module, the directories its file modules resolve against,
/// and when they are compiled.
#[derive(Clone)]
struct Place {
    module: usize,
    /// Where `mod x;` looks for `x.rs` and `x/mod.rs`; `None` when this reading cannot tell.
    dir: Option<String>,
    /// The directory of the file being read (a top-level `#[path]` is relative to it).
    file_dir: String,
    /// Inside an inline module, where a `#[path]` is relative to `dir`.
    inline: bool,
    when: Cfg,
    depth: usize,
}

struct Builder<'a> {
    sources: &'a Sources<'a>,
    modules: Vec<ModuleNode>,
    prelude: BTreeMap<String, Vec<(Cfg, Binding)>>,
}

/// A module tree and its crate root's `extern crate` items.
pub(super) type Built = (Vec<ModuleNode>, BTreeMap<String, Vec<(Cfg, Binding)>>);

/// Makes every atom of `cfg` the package's own.
fn qualify(cfg: &mut Cfg, package: usize) {
    match cfg {
        Cfg::True | Cfg::False => {}
        Cfg::Atom(atom) => *atom = format!("{package}:{atom}"),
        Cfg::Not(inner) => qualify(inner, package),
        Cfg::All(parts) | Cfg::Any(parts) => parts.iter_mut().for_each(|p| qualify(p, package)),
    }
}

fn qualify_bindings(bindings: &mut BTreeMap<String, Vec<(Cfg, Binding)>>, package: usize) {
    for (when, binding) in bindings.values_mut().flatten() {
        qualify(when, package);
        if let Binding::Module(alternatives) = binding {
            for (applies, _) in alternatives {
                qualify(applies, package);
            }
        }
    }
}

/// The module tree of the library rooted at `lib`.
pub(super) fn build(lib: &str, sources: &Sources) -> Result<Built, String> {
    let file = sources
        .files
        .get(lib)
        .ok_or_else(|| format!("{lib} is not read"))?;
    let inner = attrs_of(&file.attrs, sources.impossible);
    if inner.rewrites {
        return Err(format!(
            "{lib} carries an inner attribute this reading does not know"
        ));
    }
    let mut builder = Builder {
        sources,
        modules: vec![ModuleNode::default()],
        prelude: BTreeMap::new(),
    };
    let dir = parent_dir(lib);
    let place = Place {
        module: 0,
        dir: Some(dir.clone()),
        file_dir: dir,
        inline: false,
        when: inner.when,
        depth: 0,
    };
    builder.walk(&file.items, &place, &mut Vec::new());
    for module in &mut builder.modules {
        qualify_bindings(&mut module.types, sources.package);
    }
    qualify_bindings(&mut builder.prelude, sources.package);
    Ok((builder.modules, builder.prelude))
}

impl Builder<'_> {
    fn bind(&mut self, module: usize, name: &str, when: &Cfg, binding: Binding) {
        self.modules[module]
            .types
            .entry(unraw(name))
            .or_default()
            .push((when.clone(), binding));
    }

    fn walk(&mut self, items: &[syn::Item], place: &Place, scope: &mut Vec<Textual>) {
        for item in items {
            let Some(attrs) = item_attrs(item) else {
                scope.push(Textual::Opaque(place.when.clone()));
                continue;
            };
            let attrs = attrs_of(attrs, self.sources.impossible);
            let when = place.when.clone().and(attrs.when.clone());
            if when == Cfg::False {
                continue;
            }
            if attrs.rewrites {
                if let Some(name) = item_name(item) {
                    self.bind(place.module, &name, &when, Binding::Unknown);
                }
                scope.push(Textual::Opaque(when));
                continue;
            }
            match item {
                syn::Item::Use(item_use) => {
                    let mut leaves = Vec::new();
                    use_leaves(&item_use.tree, &mut Vec::new(), &mut leaves);
                    for (name, segments) in leaves {
                        let binding = Binding::Use {
                            segments,
                            leading_colon: item_use.leading_colon.is_some(),
                        };
                        self.bind(place.module, &name, &when, binding);
                    }
                }
                syn::Item::Trait(item_trait) => {
                    let binding = Binding::Trait(own_methods(item_trait, self.sources.impossible));
                    self.bind(place.module, &item_trait.ident.to_string(), &when, binding);
                }
                syn::Item::Struct(_)
                | syn::Item::Enum(_)
                | syn::Item::Union(_)
                | syn::Item::Type(_)
                | syn::Item::TraitAlias(_) => {
                    let binding = match item {
                        syn::Item::TraitAlias(_) => Binding::Unknown,
                        _ => Binding::Other,
                    };
                    let name = item_name(item).expect("a type item is named");
                    self.bind(place.module, &name, &when, binding);
                }
                syn::Item::ExternCrate(extern_crate) => {
                    let name = extern_crate
                        .rename
                        .as_ref()
                        .map_or(&extern_crate.ident, |r| &r.1);
                    let binding = if extern_crate.ident == "self" {
                        Binding::Module(vec![(Cfg::True, Content::Module(0))])
                    } else {
                        Binding::Crate(unraw(&extern_crate.ident.to_string()))
                    };
                    if place.module == 0 {
                        self.prelude
                            .entry(unraw(&name.to_string()))
                            .or_default()
                            .push((when.clone(), binding.clone()));
                    }
                    self.bind(place.module, &name.to_string(), &when, binding);
                }
                syn::Item::Mod(module) => self.module(module, &attrs, &when, place, scope),
                syn::Item::Macro(item_macro) if item_macro.mac.path.is_ident("macro_rules") => {
                    match &item_macro.ident {
                        Some(name) => scope.push(Textual::Macro {
                            name: unraw(&name.to_string()),
                            when: when.clone(),
                            expansion: single_empty_rule(&item_macro.mac.tokens),
                        }),
                        None => scope.push(Textual::Opaque(when.clone())),
                    }
                }
                syn::Item::Macro(item_macro) => {
                    match (item_macro.mac.path.get_ident(), &item_macro.ident) {
                        (Some(name), None)
                            if item_macro.mac.tokens.is_empty() && place.depth < MAX_DEPTH =>
                        {
                            self.invoke(&unraw(&name.to_string()), &when, place, scope)
                        }
                        _ => scope.push(Textual::Opaque(when.clone())),
                    }
                }
                _ => {}
            }
            if attrs.derives {
                scope.push(Textual::Opaque(when));
            }
        }
    }

    /// An empty item-position invocation of `name`: the transcriber of each definition in textual
    /// scope that may be in effect (the last one compiled, with no unseen expansion after it) is
    /// read here, under when it is; any other case is an unseen expansion.
    fn invoke(&mut self, name: &str, when: &Cfg, place: &Place, scope: &mut Vec<Textual>) {
        let mut shown = Vec::new();
        for (index, entry) in scope.iter().enumerate() {
            let Textual::Macro {
                name: defined,
                when: compiled,
                expansion,
            } = entry
            else {
                continue;
            };
            if defined != name {
                continue;
            }
            let later = scope[index + 1..]
                .iter()
                .filter_map(|e| match e {
                    Textual::Macro {
                        name: other, when, ..
                    } if other == name => Some(when.clone()),
                    Textual::Opaque(when) => Some(when.clone()),
                    Textual::Macro { .. } => None,
                })
                .collect();
            let in_effect = compiled.clone().and(Cfg::any(later).not());
            if let Some(items) = expansion {
                shown.push((in_effect, Rc::clone(items)));
            }
        }
        let covered = Cfg::any(shown.iter().map(|(c, _)| c.clone()).collect());
        for (in_effect, items) in shown {
            let expanded = Place {
                when: when.clone().and(in_effect),
                depth: place.depth + 1,
                ..place.clone()
            };
            self.walk(&items, &expanded, scope);
        }
        let unseen = when.clone().and(covered.not());
        if unseen != Cfg::False {
            scope.push(Textual::Opaque(unseen));
        }
    }

    fn module(
        &mut self,
        module: &syn::ItemMod,
        attrs: &Attrs,
        when: &Cfg,
        place: &Place,
        scope: &mut Vec<Textual>,
    ) {
        let name = unraw(&module.ident.to_string());
        let mut contents = Vec::new();
        if let Some((_, items)) = &module.content {
            // A `#[path]` on an inline module moves its file modules where this reading does not
            // follow.
            let dir = match attrs.paths.is_empty() {
                true => place.dir.as_deref().and_then(|d| join(d, &name)),
                false => None,
            };
            let child = Place {
                module: self.child(place.module),
                dir,
                file_dir: place.file_dir.clone(),
                inline: true,
                when: when.clone(),
                depth: place.depth,
            };
            self.enter(items, &child, &attrs.macro_use, scope);
            contents.push((Cfg::True, Content::Module(child.module)));
        } else {
            let alternatives = match attrs.paths.as_slice() {
                [] => vec![(Cfg::True, self.default_file(place, &name))],
                [(applies, path)] => vec![
                    (applies.clone(), self.path_file(place, path)),
                    (applies.clone().not(), self.default_file(place, &name)),
                ],
                _ => vec![(Cfg::True, Err(Content::Unread))],
            };
            for (applies, file) in alternatives {
                let content = match file {
                    Err(content) => content,
                    Ok(_) if place.depth >= MAX_DEPTH => Content::Unread,
                    Ok((path, dir)) => match self.sources.files.get(&path) {
                        None => Content::Unread,
                        Some(file) => {
                            let inner = attrs_of(&file.attrs, self.sources.impossible);
                            if inner.rewrites {
                                Content::Unread
                            } else {
                                let child = Place {
                                    module: self.child(place.module),
                                    dir: Some(dir),
                                    file_dir: parent_dir(&path),
                                    inline: false,
                                    when: when.clone().and(applies.clone()).and(inner.when),
                                    depth: place.depth + 1,
                                };
                                let uses = attrs.macro_use.clone().and(applies.clone());
                                self.enter(&file.items, &child, &uses, scope);
                                Content::Module(child.module)
                            }
                        }
                    },
                };
                // G177 review: a module this reading does not walk may define any macro; under
                // `#[macro_use]` it stays in scope after the module.
                if content == Content::Unread {
                    let unseen = when
                        .clone()
                        .and(applies.clone())
                        .and(attrs.macro_use.clone());
                    if unseen != Cfg::False {
                        scope.push(Textual::Opaque(unseen));
                    }
                }
                contents.push((applies, content));
            }
        }
        self.bind(place.module, &name, when, Binding::Module(contents));
    }

    fn child(&mut self, parent: usize) -> usize {
        self.modules.push(ModuleNode {
            parent: Some(parent),
            ..ModuleNode::default()
        });
        self.modules.len() - 1
    }

    /// Reads a child module, which sees the textual scope so far; under `#[macro_use]` the
    /// definitions it adds stay in scope after it.
    fn enter(
        &mut self,
        items: &[syn::Item],
        child: &Place,
        macro_use: &Cfg,
        scope: &mut Vec<Textual>,
    ) {
        let mut inner = scope.clone();
        let before = scope.len();
        self.walk(items, child, &mut inner);
        for entry in inner.into_iter().skip(before) {
            scope.push(match entry {
                Textual::Macro {
                    name,
                    when,
                    expansion,
                } => Textual::Macro {
                    name,
                    when: when.and(macro_use.clone()),
                    expansion,
                },
                Textual::Opaque(when) => Textual::Opaque(when.and(macro_use.clone())),
            });
        }
    }

    /// `mod name;` without `#[path]`: `name.rs` or `name/mod.rs` in the module's directory, and
    /// the directory its own file modules resolve against.
    fn default_file(&self, place: &Place, name: &str) -> Result<(String, String), Content> {
        let dir = place.dir.as_deref().ok_or(Content::Unread)?;
        let flat = join(dir, &format!("{name}.rs")).ok_or(Content::Unread)?;
        let nested = join(dir, &format!("{name}/mod.rs")).ok_or(Content::Unread)?;
        let child_dir = join(dir, name).ok_or(Content::Unread)?;
        match (self.exists(&flat), self.exists(&nested)) {
            (true, true) => Err(Content::Unread),
            (true, false) => Ok((flat, child_dir)),
            (false, true) => Ok((nested, child_dir)),
            (false, false) => Err(Content::Missing),
        }
    }

    /// `#[path = "p"] mod name;`: relative to the file's directory, or to the module's inside an
    /// inline module. The file is read like a `mod.rs`.
    fn path_file(&self, place: &Place, path: &str) -> Result<(String, String), Content> {
        let base = match place.inline {
            true => place.dir.as_deref().ok_or(Content::Unread)?,
            false => place.file_dir.as_str(),
        };
        let file = join(base, path).ok_or(Content::Unread)?;
        if !file.ends_with(".rs") {
            return Err(Content::Unread);
        }
        match self.exists(&file) {
            true => Ok((file.clone(), parent_dir(&file))),
            false => Err(Content::Missing),
        }
    }

    fn exists(&self, relative: &str) -> bool {
        self.sources.dir.join(relative).exists()
    }
}

/// `dir/relative`, normalized; `None` when it leaves the package.
fn join(dir: &str, relative: &str) -> Option<String> {
    if relative.starts_with('/') || relative.contains('\\') {
        return None;
    }
    let mut parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    for part in relative.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

fn parent_dir(file: &str) -> String {
    file.rsplit_once('/')
        .map(|(dir, _)| dir.to_owned())
        .unwrap_or_default()
}

fn item_name(item: &syn::Item) -> Option<String> {
    let ident = match item {
        syn::Item::Struct(i) => &i.ident,
        syn::Item::Enum(i) => &i.ident,
        syn::Item::Union(i) => &i.ident,
        syn::Item::Type(i) => &i.ident,
        syn::Item::Trait(i) => &i.ident,
        syn::Item::TraitAlias(i) => &i.ident,
        syn::Item::Mod(i) => &i.ident,
        _ => return None,
    };
    Some(ident.to_string())
}

/// Each name a `use` tree binds, with the path it imports; a glob and `as _` bind none.
fn use_leaves(tree: &syn::UseTree, prefix: &mut Vec<String>, out: &mut Vec<(String, Vec<String>)>) {
    let leaf = |ident: &syn::Ident, prefix: &[String]| {
        let mut path = prefix.to_vec();
        if ident != "self" {
            path.push(unraw(&ident.to_string()));
        }
        path
    };
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(unraw(&path.ident.to_string()));
            use_leaves(&path.tree, prefix, out);
            prefix.pop();
        }
        syn::UseTree::Name(name) if name.ident == "self" => {
            if let Some(last) = prefix.last() {
                out.push((last.clone(), prefix.clone()));
            }
        }
        syn::UseTree::Name(name) => out.push((name.ident.to_string(), leaf(&name.ident, prefix))),
        syn::UseTree::Rename(rename) if rename.rename == "_" => {}
        syn::UseTree::Rename(rename) => {
            out.push((rename.rename.to_string(), leaf(&rename.ident, prefix)))
        }
        syn::UseTree::Glob(_) => {}
        syn::UseTree::Group(group) => group.items.iter().for_each(|t| use_leaves(t, prefix, out)),
    }
}

/// A trait's own method names, when every item of its body is read.
fn own_methods(item: &syn::ItemTrait, impossible: &BTreeSet<String>) -> Option<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    for member in &item.items {
        let (attrs, name) = match member {
            syn::TraitItem::Fn(f) => (&f.attrs, Some(&f.sig.ident)),
            syn::TraitItem::Const(c) => (&c.attrs, None),
            syn::TraitItem::Type(t) => (&t.attrs, None),
            _ => return None,
        };
        if attrs_of(attrs, impossible).rewrites {
            return None;
        }
        if let Some(name) = name {
            names.insert(unraw(&name.to_string()));
        }
    }
    Some(names)
}

/// The items of a `macro_rules!` body with exactly one rule, whose matcher is empty.
fn single_empty_rule(body: &TokenStream) -> Option<Rc<Vec<syn::Item>>> {
    let trees: Vec<TokenTree> = body.clone().into_iter().collect();
    let rule = match trees.as_slice() {
        [rule @ .., TokenTree::Punct(semi)] if semi.as_char() == ';' => rule,
        rule => rule,
    };
    let [
        TokenTree::Group(matcher),
        TokenTree::Punct(eq),
        TokenTree::Punct(gt),
        TokenTree::Group(transcriber),
    ] = rule
    else {
        return None;
    };
    if !matcher.stream().is_empty() || eq.as_char() != '=' || gt.as_char() != '>' {
        return None;
    }
    let file = syn::parse2::<syn::File>(transcriber.stream()).ok()?;
    file.attrs.is_empty().then(|| Rc::new(file.items))
}

/// What a name or path means in the type namespace under one assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Found {
    Trait(BTreeSet<String>),
    /// A module of a package's tree, or `None` for one whose content is not read.
    Module(Option<(usize, usize)>),
    /// No trait, and nothing a path continues through.
    Other,
    /// Nothing in the type namespace: a procedural macro crate exports only macros.
    Absent,
    /// No binding this reading sees.
    Unbound,
    /// This build does not compile: a module file on the path is missing.
    Infeasible,
    Unknown,
}

/// The per-item reading over every locked package's tree.
pub(super) struct Reading<'a> {
    pub(super) trees: &'a [ItemTree],
}

impl Reading<'_> {
    /// The method names the item at `path` below package `tree`'s root can bring into scope as a
    /// trait, over every assignment of the atoms it depends on; `None` when any is UNKNOWN.
    pub(super) fn methods(&self, tree: usize, path: &[String]) -> Option<BTreeSet<String>> {
        let mut pending = vec![BTreeMap::new()];
        let mut names = BTreeSet::new();
        let mut feasible = false;
        let mut tried = 0;
        while let Some(assumed) = pending.pop() {
            tried += 1;
            if tried > MAX_ASSIGNMENTS {
                return None;
            }
            match self.walk(Found::Module(Some((tree, 0))), path, &assumed, 0) {
                Err(atom) => {
                    for value in [true, false] {
                        let mut next = assumed.clone();
                        next.insert(atom.clone(), value);
                        pending.push(next);
                    }
                }
                Ok(Found::Trait(methods)) => {
                    feasible = true;
                    names.extend(methods);
                }
                Ok(Found::Module(_) | Found::Other | Found::Absent) => feasible = true,
                Ok(Found::Infeasible) => {}
                Ok(Found::Unbound | Found::Unknown) => return None,
            }
        }
        feasible.then_some(names)
    }

    /// Follows `segments` from `start`, each in the type namespace.
    fn walk(
        &self,
        start: Found,
        segments: &[String],
        assumed: &BTreeMap<String, bool>,
        hops: usize,
    ) -> Result<Found, String> {
        let mut at = start;
        for segment in segments {
            at = match at {
                Found::Module(Some((tree, module))) => {
                    match self.lookup(tree, module, segment, assumed, hops)? {
                        Found::Unbound => Found::Unknown,
                        found => found,
                    }
                }
                // An enum's variant.
                Found::Other => Found::Other,
                Found::Absent | Found::Infeasible => return Ok(at),
                _ => return Ok(Found::Unknown),
            };
        }
        Ok(at)
    }

    /// The type-namespace binding of `name` in a module: the one binding compiled under
    /// `assumed` that this reading sees, `Unbound` when there is none.
    fn lookup(
        &self,
        tree: usize,
        module: usize,
        name: &str,
        assumed: &BTreeMap<String, bool>,
        hops: usize,
    ) -> Result<Found, String> {
        let package = &self.trees[tree];
        if package.proc_macro {
            return Ok(Found::Absent);
        }
        if package.unread.is_some() || hops > MAX_DEPTH {
            return Ok(Found::Unknown);
        }
        let bindings = package.modules[module].types.get(name);
        self.among(tree, module, bindings, assumed, hops)
    }

    /// The one of `bindings` (written in `module`) compiled under `assumed`, `Unbound` when
    /// there is none.
    fn among(
        &self,
        tree: usize,
        module: usize,
        bindings: Option<&Vec<(Cfg, Binding)>>,
        assumed: &BTreeMap<String, bool>,
        hops: usize,
    ) -> Result<Found, String> {
        let package = &self.trees[tree];
        let mut found = Vec::new();
        for (when, binding) in bindings.into_iter().flatten() {
            if !when.eval(assumed)? {
                continue;
            }
            let meaning = match binding {
                Binding::Trait(Some(methods)) => Found::Trait(methods.clone()),
                Binding::Trait(None) | Binding::Unknown => Found::Unknown,
                Binding::Other => Found::Other,
                Binding::Module(alternatives) => {
                    let mut content = None;
                    for (applies, alternative) in alternatives {
                        if applies.eval(assumed)? {
                            content = Some(alternative);
                            break;
                        }
                    }
                    match content {
                        Some(Content::Module(child)) => Found::Module(Some((tree, *child))),
                        Some(Content::Missing) => Found::Infeasible,
                        Some(Content::Unread) | None => Found::Module(None),
                    }
                }
                Binding::Crate(krate) => {
                    Found::Module(package.externs.get(krate).map(|&dep| (dep, 0)))
                }
                Binding::Use {
                    segments,
                    leading_colon,
                } => self.path(tree, module, segments, *leading_colon, assumed, hops + 1)?,
            };
            match meaning {
                // A `use` of a name the target binds only elsewhere binds none here.
                Found::Absent => {}
                Found::Infeasible => return Ok(Found::Infeasible),
                meaning => found.push(meaning),
            }
        }
        Ok(match found.len() {
            0 => Found::Unbound,
            // Two bindings of one name would not compile.
            1 => found.pop().expect("one binding"),
            _ => Found::Unknown,
        })
    }

    /// A name in the extern prelude: what a crate-root `extern crate` compiled under `assumed`
    /// binds it to, else the dependency of that name.
    fn extern_prelude(
        &self,
        tree: usize,
        name: &str,
        assumed: &BTreeMap<String, bool>,
        hops: usize,
    ) -> Result<Found, String> {
        let package = &self.trees[tree];
        Ok(
            match self.among(tree, 0, package.prelude.get(name), assumed, hops)? {
                Found::Unbound => match package.externs.get(name) {
                    Some(&dep) => Found::Module(Some((dep, 0))),
                    None => Found::Unknown,
                },
                found => found,
            },
        )
    }

    /// What a `use` path written in `module` names in the type namespace.
    fn path(
        &self,
        tree: usize,
        module: usize,
        segments: &[String],
        leading_colon: bool,
        assumed: &BTreeMap<String, bool>,
        hops: usize,
    ) -> Result<Found, String> {
        let package = &self.trees[tree];
        let root = Found::Module(Some((tree, 0)));
        let Some((first, rest)) = segments.split_first() else {
            return Ok(Found::Unknown);
        };
        let (start, rest) = match first.as_str() {
            "crate" if !leading_colon => (root, rest),
            "self" if !leading_colon => (Found::Module(Some((tree, module))), rest),
            "super" if !leading_colon => {
                let mut at = module;
                let mut rest = segments;
                while let Some(("super", tail)) = rest.split_first().map(|(s, t)| (s.as_str(), t)) {
                    match package.modules[at].parent {
                        Some(parent) => at = parent,
                        None => return Ok(Found::Unknown),
                    }
                    rest = tail;
                }
                (Found::Module(Some((tree, at))), rest)
            }
            _ if package.edition_2015 => (root, segments),
            _ if leading_colon => (self.extern_prelude(tree, first, assumed, hops)?, rest),
            // A name the module binds comes first; else the extern prelude. A local binding
            // this reading does not see (a glob, an expansion) would be ambiguous with a crate
            // (E0659).
            _ => match self.lookup(tree, module, first, assumed, hops)? {
                Found::Unbound => (self.extern_prelude(tree, first, assumed, hops)?, rest),
                found => (found, rest),
            },
        };
        self.walk(start, rest, assumed, hops)
    }
}
