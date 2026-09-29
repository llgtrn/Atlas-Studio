//! G173 (NA-CALL-TYPE-RESIDUAL, ADR 0087): the method names the traits of a locked Cargo
//! dependency closure can declare, read from the registry sources Cargo itself compiles.
//!
//! The path-call resolver's autoref guard (G142) refuses to claim `value.name()` for a `&self`
//! method whenever a scope on the lookup chain imports anything from a registry crate: a trait of
//! that crate in scope could supply a by-value `name`, which the method probe tries before the
//! autoref step. A registry crate's traits come from its own sources and its dependencies', all
//! pinned by `Cargo.lock`, and Cargo compiles them from the directory it extracted them to
//! (`<CARGO_HOME>/registry/src/<index>/<name>-<version>`, complete once `.cargo-ok` exists). This
//! module reads those sources as data (parsed; never built, run or expanded) and answers, for a
//! package, the union of the method names every trait in its lock closure declares, or UNKNOWN
//! when anything in the closure could declare a trait method the reading does not see:
//!
//! - a package that is not a crates.io package, whose sources are not extracted, that a
//!   `[replace]` entry may redirect, or that holds a symlink, a file that does not parse, or one
//!   the recursion pre-scan refuses;
//! - an unparsed item (`macro` 2.0), a macro invocation among a trait's items, or a `#[path]` or
//!   `include!` of anything but a `.rs` file inside the package (every such file is read; the
//!   top-level `tests/`, `benches/` and `examples/` crates are skipped unless the library itself
//!   sits at the package root);
//! - an expansion this reading does not see. Only a procedural macro turns an attribute, a derive
//!   or an item-position invocation into new items (the built-in derives emit impls; `include!`
//!   is refused outright), and a package can only invoke one that its own closure exports. So a
//!   module-level item or trait item attribute, a derive or an item-position invocation whose
//!   name a procedural macro of the package's closure exports is unknown, as is a `use` renaming
//!   such a macro or `include`, and a procedural macro named like a built-in attribute, derive or
//!   item macro. Any other such name is built in, or a feature-gated one whose crate is not
//!   locked (it would not compile). Any identifier a macro's tokens or a `use` tree spell that
//!   names such a procedural macro counts too: it may be invoked through a metavariable;
//! - in a `macro_rules!` transcriber or an item-position invocation, a trait body (the brace
//!   group after a literal `trait`, or after a header a metavariable may complete) whose own
//!   items include a metavariable other than `$crate`, a method named by one, or a macro
//!   invocation: its method names come from elsewhere;
//! - a `pub use` or `pub extern crate` naming a crate outside the lock (`proc_macro`): its traits
//!   would reach dependants unread.
//!
//! Standard-library traits a dependency re-exports are the traits a direct `std` import names:
//! the guard withholds a claim when a workspace impl of any trait on the receiver defines the
//! name, and for the blanket and by-value std methods it knows, either way.
//!
//! G177: the same sources also answer per item (see [`items`]): an import names one item, which
//! is read through its package's module tree even where the closure is UNKNOWN.

use super::{MAX_STRUCTURAL_RECURSION_RISK, max_structural_recursion_risk};
use crate::dependency::cargo::{
    DeclaredSource, LockPackage, parse_cargo_lock, resolve_dependency_ref,
};
use atlas_core::{DependencyRole, IntegrityDigest};
use items::ItemTree;
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::ToTokens;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
use syn::{punctuated::Punctuated, visit::Visit};

/// The crates.io sources Cargo writes in a lockfile.
const CRATES_IO: &[&str] = &[
    "registry+https://github.com/rust-lang/crates.io-index",
    "sparse+https://index.crates.io/",
];

/// Built-in attributes, which never rewrite the item they sit on into another item.
const BUILTIN_ATTRIBUTES: &[&str] = &[
    "allow",
    "automatically_derived",
    "bench",
    "cfg",
    "cold",
    "collapse_debuginfo",
    "coverage",
    "crate_name",
    "crate_type",
    "debugger_visualizer",
    "deny",
    "deprecated",
    "doc",
    "expect",
    "export_name",
    "feature",
    "forbid",
    "global_allocator",
    "ignore",
    "inline",
    "instruction_set",
    "link",
    "link_name",
    "link_ordinal",
    "link_section",
    "macro_export",
    "macro_use",
    "must_use",
    "naked",
    "no_builtins",
    "no_implicit_prelude",
    "no_link",
    "no_main",
    "no_mangle",
    "no_std",
    "non_exhaustive",
    "panic_handler",
    "proc_macro",
    "proc_macro_attribute",
    "recursion_limit",
    "repr",
    "should_panic",
    "target_feature",
    "test",
    "track_caller",
    "type_length_limit",
    "used",
    "warn",
    "windows_subsystem",
];

/// Tool attribute namespaces (`#[rustfmt::skip]`, `#[diagnostic::on_unimplemented]`).
const TOOL_NAMESPACES: &[&str] = &["clippy", "diagnostic", "rustdoc", "rustfmt"];

/// The standard derives: each emits one impl of its own trait, never a new trait.
const STD_DERIVES: &[&str] = &[
    "Clone",
    "Copy",
    "Debug",
    "Default",
    "Eq",
    "Hash",
    "Ord",
    "PartialEq",
    "PartialOrd",
];

/// Built-in macros allowed in item position: none of them emits a trait.
const BUILTIN_ITEM_MACROS: &[&str] = &["compile_error", "global_asm", "thread_local"];

/// Keywords or punctuation that make a brace group's header something other than a trait.
const NOT_A_TRAIT: &[&str] = &[
    "async", "const", "else", "enum", "extern", "fn", "for", "if", "impl", "let", "loop", "match",
    "mod", "move", "return", "static", "struct", "union", "while",
];

/// Keywords after which `!` is negation, not a macro invocation.
const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "yield",
];

/// What one package's sources say, before its closure is consulted.
#[derive(Debug, Clone, Default)]
struct PackageFacts {
    trait_methods: BTreeSet<String>,
    macro_rules: BTreeSet<String>,
    /// Item-position invocations of anything but `macro_rules!`, by the invoked name.
    item_macros: BTreeSet<String>,
    /// Module-level item and trait item attributes and derives that are not built in, by the
    /// last segment of their path: inert unless a procedural macro of the closure exports it.
    attributes: BTreeSet<String>,
    /// The file being read (a `#[path]` is relative to it).
    file: String,
    /// Features no build under this lock can enable (see [`impossible_features`]).
    impossible: BTreeSet<String>,
    /// A literal `trait` inside a macro invocation's input, at module level or in a transcriber:
    /// only then can a metavariable bind the keyword.
    keyword_in_input: bool,
    /// Macros invoked with exactly `trait` as their input (`Token![trait]`): the keyword binds
    /// only if a rule of the macro could bind it to a metavariable.
    keyword_inputs: BTreeSet<String>,
    /// Per `macro_rules!` name: whether its first rule able to match a lone `trait` matches it
    /// literally (a lone-token matcher `trait`), never through a metavariable.
    keyword_literal: BTreeMap<String, bool>,
    /// Brace groups whose header a metavariable may complete into a trait: their method names,
    /// or why they are unknown. They count only when the closure passes the keyword to a macro.
    completed_methods: BTreeSet<String>,
    completed_unknown: Option<String>,
    /// Names this package exports as procedural macros (function-like, attribute or derive).
    proc_macros: BTreeSet<String>,
    /// `extern crate` names, and every `(name, alias)` an `extern crate` or `use` renames.
    extern_crates: BTreeSet<String>,
    renames: Vec<(String, String)>,
    /// Every identifier a `pub use` tree or a `pub extern crate` names.
    public_use_idents: BTreeSet<String>,
    /// Every identifier any macro's tokens or any `use` tree spell: a procedural macro of the
    /// closure named anywhere may be invoked through a metavariable or a rename.
    mentioned: BTreeSet<String>,
    unknown: Option<String>,
}

impl PackageFacts {
    fn refuse(&mut self, reason: impl FnOnce() -> String) {
        if self.unknown.is_none() {
            self.unknown = Some(reason());
        }
    }
}

/// One locked package as read: its method names, or why they are unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageReading {
    pub name: String,
    pub version: String,
    pub outcome: PackageRead,
}

/// How a locked package's own sources read (its closure is decided apart).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageRead {
    /// Read: the number of trait method names its own sources declare.
    Read(usize),
    /// Refused, with why: nothing it could declare is known.
    Refused(String),
}

/// The locked dependency closures' trait method names (see the module documentation).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LockedTraitMethods {
    /// Per package name: the union over the closure of every locked package of that name, or
    /// `None` when any of them is unknown.
    closures: BTreeMap<String, Result<BTreeSet<String>, String>>,
    /// Per path (workspace) package name: the package names it depends on directly.
    members: BTreeMap<String, BTreeSet<String>>,
    pub readings: Vec<PackageReading>,
    /// BLAKE3 over the lockfile, every file read, and every answer: what the table rests on.
    pub digest: Option<IntegrityDigest>,
    /// G177: every locked package's module tree for the per-item reading, and each package
    /// name's indices into it.
    trees: Vec<ItemTree>,
    tree_indices: BTreeMap<String, Vec<usize>>,
    /// Each locked package's library name (`[lib] name`, else its package name, `-` as `_`),
    /// `None` when its manifest is not read.
    lib_names: Vec<Option<String>>,
}

impl LockedTraitMethods {
    /// Whether no trait of `package`'s locked closure can declare a method `name`. False when
    /// the package or its closure is unknown or unread.
    pub fn lacks(&self, package: &str, name: &str) -> bool {
        let name = name.strip_prefix("r#").unwrap_or(name);
        matches!(self.closures.get(package), Some(Ok(names)) if !names.contains(name))
    }

    /// A table stating each package's closure answer outright (resolver tests).
    #[cfg(test)]
    pub(crate) fn stated(closures: &[(&str, Option<&[&str]>)]) -> Self {
        Self {
            closures: closures
                .iter()
                .map(|(package, names)| {
                    let answer = names
                        .map(|names| names.iter().map(|n| n.to_string()).collect())
                        .ok_or_else(|| "stated unknown".to_owned());
                    (package.to_string(), answer)
                })
                .collect(),
            ..Self::default()
        }
    }

    /// A table stating, beside [`Self::stated`], the items at the root of `package`: each a
    /// trait with its own methods, or `None` for an item this reading cannot decide.
    #[cfg(test)]
    pub(crate) fn with_items(mut self, package: &str, items: &[(&str, Option<&[&str]>)]) -> Self {
        self.tree_indices
            .entry(package.to_owned())
            .or_default()
            .push(self.trees.len());
        self.trees.push(ItemTree::stated(items));
        self.lib_names.push(None);
        self
    }

    /// G177: the method names the item at `path` below `package`'s root can bring into scope as
    /// a trait -- its own, none when it is no trait -- over every locked package of that name;
    /// `None` when any of them is UNKNOWN or none is read.
    pub fn item_methods(&self, package: &str, path: &[String]) -> Option<BTreeSet<String>> {
        let indices = self.tree_indices.get(package)?;
        let reading = items::Reading { trees: &self.trees };
        let mut names = BTreeSet::new();
        for &tree in indices {
            names.extend(reading.methods(tree, path)?);
        }
        Some(names)
    }

    /// G177 (review): the library names of every locked package named `package`: the name Cargo
    /// passes to `--extern` for an unrenamed dependency. `None` when any of them is not read or
    /// none is locked.
    pub fn library_names(&self, package: &str) -> Option<BTreeSet<String>> {
        let indices = self.tree_indices.get(package)?;
        indices.iter().map(|&i| self.lib_names[i].clone()).collect()
    }

    /// G177: whether importing the item at `path` below `package`'s root cannot bring a trait
    /// method `name` into scope: by the item itself, else (UNKNOWN) by the closure's union.
    pub fn item_lacks(&self, package: &str, path: &[String], name: &str) -> bool {
        match self.item_methods(package, path) {
            Some(names) => !names.contains(name.strip_prefix("r#").unwrap_or(name)),
            None => self.lacks(package, name),
        }
    }

    /// Why `package`'s closure is unknown, when it is.
    pub fn unknown(&self, package: &str) -> Option<&str> {
        match self.closures.get(package) {
            Some(Ok(_)) => None,
            Some(Err(reason)) => Some(reason),
            None => Some("not locked"),
        }
    }

    /// The locked package a workspace package's dependency named `package` resolves to, when the
    /// lockfile records that direct dependency (by package name) for exactly this member.
    pub fn direct_dependency(&self, member: &str, package: &str) -> bool {
        self.members
            .get(member)
            .is_some_and(|deps| deps.contains(package))
    }
}

/// Reads the trait method names of every registry package `lock` pins, from the extracted sources
/// under `registry_src` (`<CARGO_HOME>/registry/src`). Runs on a large dedicated stack: `syn`'s
/// recursive descent is bounded only by the pre-scan.
pub fn read_locked_trait_methods(lock: &str, registry_src: &Path) -> LockedTraitMethods {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(super::EXTRACTION_STACK_SIZE)
            .spawn_scoped(scope, || read_on_this_stack(lock, registry_src))
            .expect("spawning the dependency reading thread must not fail")
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}

fn read_on_this_stack(lock: &str, registry_src: &Path) -> LockedTraitMethods {
    let packages = parse_cargo_lock(lock);
    let mut by_name: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, package) in packages.iter().enumerate() {
        by_name.entry(&package.name).or_default().push(index);
    }
    // Direct dependencies, by index; an unresolvable reference makes its dependent unknown.
    let mut deps: Vec<Result<Vec<usize>, String>> = Vec::with_capacity(packages.len());
    for package in &packages {
        let mut resolved = Ok(Vec::new());
        for reference in &package.dependencies {
            let candidates: Vec<_> = by_name
                .get(reference.name.as_str())
                .map(|indices| indices.iter().map(|&i| &packages[i]).collect())
                .unwrap_or_default();
            match (
                resolve_dependency_ref(&candidates, reference),
                &mut resolved,
            ) {
                (Ok(found), Ok(list)) => {
                    let index = packages
                        .iter()
                        .position(|p| std::ptr::eq(p, found))
                        .expect("a resolved candidate is a locked package");
                    list.push(index);
                }
                (Err(error), Ok(_)) => resolved = Err(format!("unresolved lock entry {error}")),
                (_, Err(_)) => {}
            }
        }
        deps.push(resolved);
    }

    let mut digest_input = format!("atlas.locked-trait-methods.v1\nlock {}\n", {
        IntegrityDigest::of_bytes(lock.as_bytes())
    });
    let replaced = lock
        .lines()
        .any(|l| l.trim_start().starts_with("replace = "));
    let mut facts: Vec<PackageFacts> = Vec::with_capacity(packages.len());
    let mut readings = Vec::new();
    let mut trees = Vec::with_capacity(packages.len());
    // G177 (review): each package's library name, and the normal dependencies each tree's
    // extern prelude is built from once every library name is known.
    let mut lib_names: Vec<Option<String>> = vec![None; packages.len()];
    let mut dependency_keys: Vec<Vec<(String, String)>> = vec![Vec::new(); packages.len()];
    for (index, package) in packages.iter().enumerate() {
        let mut package_facts = PackageFacts::default();
        // G177: the per-item reading needs the one extraction, every file of it read.
        let mut tree = ItemTree::unread("sources not read".into());
        // Only crates.io, whose extraction directories are named after its index; a `[replace]`
        // entry (deprecated) redirects a package this reading does not follow.
        let registry = package
            .source
            .as_deref()
            .is_some_and(|s| CRATES_IO.contains(&s));
        if replaced {
            package_facts.refuse(|| "the lockfile carries a [replace] entry".into());
        } else if !registry {
            package_facts.refuse(|| match &package.source {
                Some(source) => format!("not a registry package ({source})"),
                None => "a path package (not read here)".into(),
            });
        } else {
            let dirs = extracted_dirs(registry_src, &package.name, &package.version);
            if dirs.is_empty() {
                package_facts.refuse(|| "sources not extracted under the registry".into());
            }
            let single = dirs.len() == 1;
            for dir in dirs {
                let manifest = fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
                let targets = crate::manifest_targets(&manifest, |p| dir.join(p).is_file());
                let locked: BTreeSet<String> = match &deps[index] {
                    Ok(direct) => direct.iter().map(|&d| packages[d].name.clone()).collect(),
                    Err(_) => BTreeSet::new(),
                };
                if let Some(targets) = &targets {
                    package_facts.impossible = impossible_features(&manifest, targets, &locked);
                }
                let lib = targets.as_ref().and_then(|targets| targets.lib.clone());
                lib_names[index] = Some(
                    manifest_field(&manifest, "[lib]", &["name"])
                        .unwrap_or_else(|| package.name.clone())
                        .replace('-', "_"),
                );
                if !lib
                    .as_deref()
                    .is_some_and(|lib| stays_inside("Cargo.toml", lib))
                {
                    package_facts.refuse(|| "no library target inside its sources".into());
                }
                // A library rooted at the package root reaches `tests/` by plain `mod` paths.
                let skip_own_targets = lib.as_deref().is_some_and(|lib| lib.contains('/'));
                let mut files = Vec::new();
                let mut unread = None;
                if let Err(reason) = rust_files(&dir, &dir, skip_own_targets, &mut files) {
                    unread = Some(reason.clone());
                    package_facts.refuse(|| reason);
                }
                files.sort();
                let mut parsed = BTreeMap::new();
                for (relative, text) in files {
                    digest_input.push_str(&format!(
                        "file {} {} {relative} {}\n",
                        package.name,
                        package.version,
                        IntegrityDigest::of_bytes(text.as_bytes())
                    ));
                    match read_file(&relative, &text, &mut package_facts) {
                        Some(file) => {
                            parsed.insert(relative, file);
                        }
                        None => {
                            unread.get_or_insert_with(|| format!("{relative} is not read"));
                        }
                    }
                }
                tree = match (unread, &lib, &targets, &deps[index]) {
                    (None, Some(lib), Some(targets), Ok(_)) if single => {
                        let sources = items::Sources {
                            dir: &dir,
                            files: &parsed,
                            impossible: &package_facts.impossible,
                            package: index,
                        };
                        dependency_keys[index] = targets
                            .dependencies
                            .iter()
                            .filter(|(_, _, role, ..)| *role == DependencyRole::Runtime)
                            // G178 (review 4): an entry the manifest reading could not read
                            // names no package; the empty name matches none, so its key binds
                            // nothing (nor does any other entry passing the same name).
                            .map(|(key, package, _, source, _)| match source {
                                DeclaredSource::Unknown => (key.clone(), String::new()),
                                _ => (key.clone(), package.clone()),
                            })
                            .collect();
                        match items::build(lib, &sources) {
                            Ok((modules, prelude)) => ItemTree {
                                modules,
                                prelude,
                                externs: BTreeMap::new(),
                                proc_macro: lib_proc_macro(&manifest),
                                edition_2015: edition_2015(&manifest),
                                unread: None,
                            },
                            Err(reason) => ItemTree::unread(reason),
                        }
                    }
                    (Some(reason), ..) => ItemTree::unread(reason),
                    _ => ItemTree::unread("no single library target with a resolved lock".into()),
                };
            }
        }
        trees.push(tree);
        readings.push(PackageReading {
            name: package.name.clone(),
            version: package.version.clone(),
            outcome: match &package_facts.unknown {
                Some(reason) => PackageRead::Refused(reason.clone()),
                None => PackageRead::Read(package_facts.trait_methods.len()),
            },
        });
        facts.push(package_facts);
    }

    // Each package's closure answer.
    let crate_names: Vec<String> = packages.iter().map(|p| p.name.replace('-', "_")).collect();
    let answers: Vec<Result<BTreeSet<String>, String>> = (0..packages.len())
        .map(|index| closure_answer(index, &crate_names, &deps, &facts))
        .collect();
    // Every locked version of a name: the union, unknown if any of them is.
    let mut closures: BTreeMap<String, Result<BTreeSet<String>, String>> = BTreeMap::new();
    for (index, package) in packages.iter().enumerate() {
        let slot = closures
            .entry(package.name.clone())
            .or_insert_with(|| Ok(BTreeSet::new()));
        match (&answers[index], slot.as_mut()) {
            (Ok(names), Ok(union)) => union.extend(names.iter().cloned()),
            (Err(reason), Ok(_)) => *slot = Err(reason.clone()),
            (_, Err(_)) => {}
        }
    }
    let mut members: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut ambiguous = BTreeSet::new();
    for (index, package) in packages.iter().enumerate() {
        if package.source.is_some() {
            continue;
        }
        let Ok(direct) = &deps[index] else {
            ambiguous.insert(package.name.clone());
            continue;
        };
        let names = direct.iter().map(|&d| packages[d].name.clone()).collect();
        if members.insert(package.name.clone(), names).is_some() {
            ambiguous.insert(package.name.clone());
        }
    }
    for name in ambiguous {
        members.remove(&name);
    }
    for (name, answer) in &closures {
        match answer {
            Ok(names) => digest_input.push_str(&format!(
                "closure {name} {}\n",
                names.iter().cloned().collect::<Vec<_>>().join(",")
            )),
            Err(reason) => digest_input.push_str(&format!("closure {name} UNKNOWN {reason}\n")),
        }
    }
    let mut tree_indices: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, package) in packages.iter().enumerate() {
        if let (None, Ok(direct)) = (&trees[index].unread, &deps[index]) {
            trees[index].externs = externs(&dependency_keys[index], direct, &packages, &lib_names);
        }
        digest_input.push_str(&format!(
            "items {} {} {:?} {}\n",
            package.name,
            package.version,
            lib_names[index],
            IntegrityDigest::of_bytes(format!("{:?}", trees[index]).as_bytes())
        ));
        tree_indices
            .entry(package.name.clone())
            .or_default()
            .push(index);
    }
    LockedTraitMethods {
        closures,
        members,
        readings,
        digest: Some(IntegrityDigest::of_bytes(digest_input.as_bytes())),
        trees,
        tree_indices,
        lib_names,
    }
}

/// G177: a package's extern-prelude names for its locked normal dependencies (the manifest key,
/// so a rename), each to the one direct lock dependency of that package name. An unrenamed
/// dependency is named after its library (`[lib] name`), so one whose library name is not its
/// key is left out (G177 review).
fn externs(
    dependencies: &[(String, String)],
    direct: &[usize],
    packages: &[LockPackage],
    lib_names: &[Option<String>],
) -> BTreeMap<String, usize> {
    let mut names: BTreeMap<String, Option<usize>> = BTreeMap::new();
    for (key, package) in dependencies {
        let name = key.replace('-', "_");
        let locked: Vec<usize> = direct
            .iter()
            .copied()
            .filter(|&d| packages[d].name == *package)
            .collect();
        let found = match locked.as_slice() {
            [only] if key != package || lib_names[*only].as_ref().is_none_or(|l| *l == name) => {
                Some(*only)
            }
            _ => None,
        };
        let slot = names.entry(name).or_insert(found);
        if *slot != found {
            *slot = None;
        }
    }
    names
        .into_iter()
        .filter_map(|(name, index)| Some((name, index?)))
        .collect()
}

/// G177: `proc-macro = true` in the `[lib]` table: the crate exports only macros.
fn lib_proc_macro(manifest: &str) -> bool {
    manifest_field(manifest, "[lib]", &["proc-macro", "proc_macro"]).as_deref() == Some("true")
}

/// G177: the library's edition is 2015 (the default when neither `[lib]` nor `[package]` says).
fn edition_2015(manifest: &str) -> bool {
    let edition = manifest_field(manifest, "[lib]", &["edition"])
        .or_else(|| manifest_field(manifest, "[package]", &["edition"]));
    edition.is_none_or(|e| e == "2015")
}

/// The unquoted value of the first `key = value` in the `table` section.
fn manifest_field(manifest: &str, table: &str, keys: &[&str]) -> Option<String> {
    let mut inside = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == table;
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if inside && keys.contains(&key.trim()) {
            return Some(value.trim().trim_matches('"').to_owned());
        }
    }
    None
}

/// The union of trait method names over `root`'s closure, or `None` when any package in it is
/// unknown or breaks a closure-level rule.
fn closure_answer(
    root: usize,
    crate_names: &[String],
    deps: &[Result<Vec<usize>, String>],
    facts: &[PackageFacts],
) -> Result<BTreeSet<String>, String> {
    let unresolved = |i: usize| format!("{} has an unresolved lock entry", crate_names[i]);
    let closure = closure_of(root, deps).ok_or_else(|| unresolved(root))?;
    let keyword_bound = closure.iter().any(|&i| {
        facts[i].keyword_in_input
            || facts[i].keyword_inputs.iter().any(|name| {
                let procedural = closure.iter().any(|&j| facts[j].proc_macros.contains(name));
                let definitions: Vec<bool> = closure
                    .iter()
                    .filter_map(|&j| facts[j].keyword_literal.get(name).copied())
                    .collect();
                // A name no `macro_rules!` of the closure defines (a rename, a re-export) may be
                // any macro: the keyword may bind.
                procedural || definitions.is_empty() || definitions.iter().any(|literal| !literal)
            })
    });
    let mut names = BTreeSet::new();
    for &member in &closure {
        let own = closure_of(member, deps).ok_or_else(|| unresolved(member))?;
        let package = &facts[member];
        let at = &crate_names[member];
        if let Some(reason) = &package.unknown {
            return Err(format!("{at}: {reason}"));
        }
        // A crate never invokes its own procedural macros.
        let mut expanding: BTreeSet<&str> = own
            .iter()
            .filter(|&&i| i != member)
            .flat_map(|&i| &facts[i].proc_macros)
            .map(String::as_str)
            .collect();
        if let Some(name) = expanding.iter().find(|name| {
            STD_DERIVES.contains(name)
                || BUILTIN_ATTRIBUTES.contains(name)
                || BUILTIN_ITEM_MACROS.contains(name)
        }) {
            return Err(format!(
                "{at}: a procedural macro of its closure is named `{name}`"
            ));
        }
        expanding.insert("include");
        if let Some(name) = package
            .item_macros
            .iter()
            .chain(&package.attributes)
            .chain(&package.mentioned)
            .chain(package.renames.iter().map(|(from, _)| from))
            .find(|name| expanding.contains(name.as_str()))
        {
            return Err(format!("{at}: `{name}` may expand unseen"));
        }
        // A crate outside the lock (`proc_macro`, `test`) may be used, never re-exported.
        let locked: BTreeSet<&str> = own.iter().map(|&i| crate_names[i].as_str()).collect();
        let mut outside: BTreeSet<String> = package
            .extern_crates
            .iter()
            .filter(|name| {
                !matches!(name.as_str(), "std" | "core" | "alloc" | "self")
                    && !locked.contains(name.as_str())
            })
            .cloned()
            .collect();
        outside.insert("proc_macro".into());
        loop {
            let before = outside.len();
            for (from, to) in &package.renames {
                if outside.contains(from) {
                    outside.insert(to.clone());
                }
            }
            if outside.len() == before {
                break;
            }
        }
        if let Some(ident) = package
            .public_use_idents
            .iter()
            .find(|ident| outside.contains(ident.as_str()))
        {
            return Err(format!(
                "{at}: re-exports `{ident}`, a crate outside the lock"
            ));
        }
        names.extend(package.trait_methods.iter().cloned());
        if keyword_bound {
            if let Some(reason) = &package.completed_unknown {
                return Err(format!("{at}: {reason} (a metavariable may bind `trait`)"));
            }
            names.extend(package.completed_methods.iter().cloned());
        }
    }
    Ok(names)
}

fn closure_of(root: usize, deps: &[Result<Vec<usize>, String>]) -> Option<BTreeSet<usize>> {
    let mut seen = BTreeSet::from([root]);
    let mut stack = vec![root];
    while let Some(next) = stack.pop() {
        for &dep in deps[next].as_ref().ok()? {
            if seen.insert(dep) {
                stack.push(dep);
            }
        }
    }
    Some(seen)
}

/// Every extracted directory of `name-version` under any registry index, when it is complete.
fn extracted_dirs(registry_src: &Path, name: &str, version: &str) -> Vec<std::path::PathBuf> {
    let Ok(indices) = fs::read_dir(registry_src) else {
        return Vec::new();
    };
    let mut dirs: Vec<_> = indices
        .filter_map(Result::ok)
        .filter(|index| {
            let name = index.file_name().to_string_lossy().into_owned();
            name.starts_with("index.crates.io-") || name.starts_with("github.com-")
        })
        .map(|index| index.path().join(format!("{name}-{version}")))
        .filter(|dir| dir.join(".cargo-ok").is_file())
        .collect();
    dirs.sort();
    dirs
}

/// `include!("file.rs")` of a Rust file inside the package: a file read here too.
fn included_inside(file: &str, mac: &syn::Macro) -> bool {
    syn::parse2::<syn::LitStr>(mac.tokens.clone()).is_ok_and(|path| {
        let path = path.value();
        path.ends_with(".rs") && stays_inside(file, &path)
    })
}

/// Directories of targets that are their own crates (integration tests, benchmarks, examples):
/// nothing in them reaches a dependant, so they are not read, and no `#[path]` may enter them.
const OWN_TARGET_DIRS: &[&str] = &["tests", "benches", "examples"];

/// Every `.rs` file under `dir` (but not under a top-level [`OWN_TARGET_DIRS`] one), relative to
/// `base`; a symlink refuses the package.
fn rust_files(
    base: &Path,
    dir: &Path,
    skip_own_targets: bool,
    out: &mut Vec<(String, String)>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("unreadable directory: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("unreadable directory entry: {e}"))?;
        let kind = entry
            .file_type()
            .map_err(|e| format!("unreadable file type: {e}"))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        if kind.is_symlink() {
            return Err(format!("a symlink ({relative})"));
        } else if kind.is_dir() && skip_own_targets && OWN_TARGET_DIRS.contains(&relative.as_str())
        {
            continue;
        } else if kind.is_dir() {
            rust_files(base, &path, skip_own_targets, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text =
                fs::read_to_string(&path).map_err(|e| format!("unreadable {relative}: {e}"))?;
            out.push((relative, text));
        }
    }
    Ok(())
}

/// Reads one file into `facts`; the parsed file, unless it is refused.
fn read_file(relative: &str, text: &str, facts: &mut PackageFacts) -> Option<syn::File> {
    facts.file = relative.to_owned();
    let risk = max_structural_recursion_risk(text);
    if risk > MAX_STRUCTURAL_RECURSION_RISK {
        facts.refuse(|| format!("{relative} refused by the recursion pre-scan ({risk})"));
        return None;
    }
    let file = match syn::parse_file(text) {
        Ok(file) => file,
        Err(error) => {
            facts.refuse(|| format!("{relative} does not parse: {error}"));
            return None;
        }
    };
    for attr in &file.attrs {
        check_attribute(&attr.meta, facts);
    }
    module_items(&file.items, facts);
    let mut definitions = MacroRules { facts };
    definitions.visit_file(&file);
    Some(file)
}

/// Every `macro_rules!` definition, at any depth (a `#[macro_export]` one in a block is exported).
struct MacroRules<'f> {
    facts: &'f mut PackageFacts,
}

impl<'ast> Visit<'ast> for MacroRules<'_> {
    /// An item no build under this lock compiles is skipped, as the module walk skips it.
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if !cfg_impossible(item, &self.facts.impossible) {
            syn::visit::visit_item(self, item);
        }
    }

    /// Every other macro, at any depth (a statement or expression macro may expand to a
    /// `#[macro_export] macro_rules!`): its tokens are an input, its name an invocation.
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if !mac.path.is_ident("macro_rules") {
            let invoked = mac
                .path
                .segments
                .last()
                .map(|s| unraw(&s.ident.to_string()))
                .unwrap_or_default();
            if !(invoked == "include" && included_inside(&self.facts.file, mac)) {
                self.facts.item_macros.insert(invoked.clone());
            }
            scan_tokens(mac.tokens.clone(), true, Some(&invoked), self.facts);
        }
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if item.mac.path.is_ident("macro_rules") {
            match &item.ident {
                Some(name) => {
                    self.facts.macro_rules.insert(unraw(&name.to_string()));
                }
                None => self
                    .facts
                    .refuse(|| "a `macro_rules!` without a name".into()),
            }
            let literal = transcribers(item.mac.tokens.clone(), self.facts);
            if let Some(name) = &item.ident {
                let slot = self
                    .facts
                    .keyword_literal
                    .entry(unraw(&name.to_string()))
                    .or_insert(true);
                *slot &= literal;
            }
        }
        syn::visit::visit_item_macro(self, item);
    }
}

/// The module-level items (a block's items are local and never reach a dependant).
fn module_items(items: &[syn::Item], facts: &mut PackageFacts) {
    for item in items {
        if cfg_impossible(item, &facts.impossible) {
            continue;
        }
        let attrs: &[syn::Attribute] = match item {
            syn::Item::Const(i) => &i.attrs,
            syn::Item::Enum(i) => &i.attrs,
            syn::Item::ExternCrate(i) => &i.attrs,
            syn::Item::Fn(i) => &i.attrs,
            syn::Item::ForeignMod(i) => &i.attrs,
            syn::Item::Impl(i) => &i.attrs,
            syn::Item::Macro(i) => &i.attrs,
            syn::Item::Mod(i) => &i.attrs,
            syn::Item::Static(i) => &i.attrs,
            syn::Item::Struct(i) => &i.attrs,
            syn::Item::Trait(i) => &i.attrs,
            syn::Item::TraitAlias(i) => &i.attrs,
            syn::Item::Type(i) => &i.attrs,
            syn::Item::Union(i) => &i.attrs,
            syn::Item::Use(i) => &i.attrs,
            _ => {
                facts.refuse(|| "an item this reading does not parse (verbatim tokens)".into());
                continue;
            }
        };
        for attr in attrs {
            check_attribute(&attr.meta, facts);
        }
        match item {
            syn::Item::Fn(function) => {
                // Through `cfg_attr` too: any spelling of the export attributes counts.
                let exported = attrs.iter().any(|a| {
                    let mut names = BTreeSet::new();
                    stream_names(a.meta.to_token_stream(), &mut names);
                    names.contains("proc_macro") || names.contains("proc_macro_attribute")
                });
                if exported {
                    facts
                        .proc_macros
                        .insert(unraw(&function.sig.ident.to_string()));
                }
            }
            syn::Item::Trait(trait_item) => {
                for member in &trait_item.items {
                    let attrs = match member {
                        syn::TraitItem::Fn(f) => {
                            facts.trait_methods.insert(unraw(&f.sig.ident.to_string()));
                            &f.attrs
                        }
                        syn::TraitItem::Const(c) => &c.attrs,
                        syn::TraitItem::Type(t) => &t.attrs,
                        _ => {
                            facts.refuse(|| {
                                format!(
                                    "a macro or unparsed item in the body of trait `{}`",
                                    trait_item.ident
                                )
                            });
                            continue;
                        }
                    };
                    for attr in attrs {
                        check_attribute(&attr.meta, facts);
                    }
                }
            }
            syn::Item::Macro(item_macro) if !item_macro.mac.path.is_ident("macro_rules") => {
                let invoked = item_macro
                    .mac
                    .path
                    .segments
                    .last()
                    .map(|s| unraw(&s.ident.to_string()))
                    .unwrap_or_default();
                // `include!("file.rs")` of a file inside the package reads a file read here too.
                if !(invoked == "include" && included_inside(&facts.file, &item_macro.mac)) {
                    facts.item_macros.insert(invoked.clone());
                }
                scan_tokens(item_macro.mac.tokens.clone(), true, Some(&invoked), facts);
            }
            syn::Item::Mod(module) => {
                if let Some((_, items)) = &module.content {
                    module_items(items, facts);
                }
            }
            syn::Item::Use(item_use) => {
                use_renames(&item_use.tree, facts);
                let mut leaves = BTreeSet::new();
                use_idents(&item_use.tree, &mut leaves);
                facts.mentioned.extend(leaves);
                if matches!(item_use.vis, syn::Visibility::Public(_)) {
                    let mut idents = BTreeSet::new();
                    use_idents(&item_use.tree, &mut idents);
                    facts.public_use_idents.extend(idents);
                }
            }
            syn::Item::ExternCrate(extern_crate) => {
                let name = unraw(&extern_crate.ident.to_string());
                facts.extern_crates.insert(name.clone());
                if let Some((_, alias)) = &extern_crate.rename {
                    let alias = unraw(&alias.to_string());
                    facts.renames.push((name.clone(), alias.clone()));
                    if matches!(extern_crate.vis, syn::Visibility::Public(_)) {
                        facts.public_use_idents.insert(alias);
                    }
                }
                if matches!(extern_crate.vis, syn::Visibility::Public(_)) {
                    facts.public_use_idents.insert(name);
                }
            }
            _ => {}
        }
    }
}

/// Whether a `cfg` on `item` requires a feature no build under this lock can enable.
fn cfg_impossible(item: &syn::Item, impossible: &BTreeSet<String>) -> bool {
    item_attrs(item).is_some_and(|attrs| {
        attrs.iter().any(|a| {
            a.path().is_ident("cfg")
                && a.parse_args::<syn::Meta>()
                    .is_ok_and(|p| cfg_false(&p, impossible))
        })
    })
}

pub(super) fn item_attrs(item: &syn::Item) -> Option<&[syn::Attribute]> {
    Some(match item {
        syn::Item::Const(i) => &i.attrs,
        syn::Item::Enum(i) => &i.attrs,
        syn::Item::ExternCrate(i) => &i.attrs,
        syn::Item::Fn(i) => &i.attrs,
        syn::Item::ForeignMod(i) => &i.attrs,
        syn::Item::Impl(i) => &i.attrs,
        syn::Item::Macro(i) => &i.attrs,
        syn::Item::Mod(i) => &i.attrs,
        syn::Item::Static(i) => &i.attrs,
        syn::Item::Struct(i) => &i.attrs,
        syn::Item::Trait(i) => &i.attrs,
        syn::Item::TraitAlias(i) => &i.attrs,
        syn::Item::Type(i) => &i.attrs,
        syn::Item::Union(i) => &i.attrs,
        syn::Item::Use(i) => &i.attrs,
        _ => return None,
    })
}

/// Whether a `cfg` predicate is false in every build under this lock: it requires a feature no
/// such build can enable. Anything else may hold.
fn cfg_false(predicate: &syn::Meta, impossible: &BTreeSet<String>) -> bool {
    match predicate {
        syn::Meta::NameValue(syn::MetaNameValue {
            path,
            value:
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(value),
                    ..
                }),
            ..
        }) if path.is_ident("feature") => impossible.contains(&value.value()),
        syn::Meta::List(list) if list.path.is_ident("all") || list.path.is_ident("any") => {
            let Ok(parts) =
                list.parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
            else {
                return false;
            };
            if list.path.is_ident("all") {
                parts.iter().any(|p| cfg_false(p, impossible))
            } else {
                parts.iter().all(|p| cfg_false(p, impossible))
            }
        }
        _ => false,
    }
}

/// The features of a package no build under this lock can enable: enabling one activates a
/// dependency (`dep:x`, `x`, `x/feature`, or an optional dependency's implicit feature) whose
/// package the lock does not record for this package -- Cargo would have had to lock it. A name
/// that is neither a feature nor a dependency cannot be enabled at all.
fn impossible_features(
    manifest: &str,
    targets: &crate::ManifestTargets,
    locked: &BTreeSet<String>,
) -> BTreeSet<String> {
    let dependencies: BTreeMap<&str, &str> = targets
        .dependencies
        .iter()
        // G178 (review 4): an entry the manifest reading could not read blocks no feature.
        .filter(|(_, _, _, source, _)| *source != DeclaredSource::Unknown)
        .map(|(key, package, ..)| (key.as_str(), package.as_str()))
        .collect();
    let features = feature_table(manifest);
    let mut names: BTreeSet<String> = features.keys().cloned().collect();
    names.extend(dependencies.keys().map(|k| k.to_string()));
    // Only a dependency this manifest reading knows and the lock lacks blocks a feature: target
    // tables it does not read, and unknown names, leave the feature possible.
    let unlocked = |key: &str| dependencies.get(key).is_some_and(|p| !locked.contains(*p));
    let mut impossible = BTreeSet::new();
    for name in &names {
        let mut stack = vec![name.clone()];
        let mut seen = BTreeSet::new();
        let mut blocked = false;
        while let Some(feature) = stack.pop() {
            if !seen.insert(feature.clone()) {
                continue;
            }
            let Some(entries) = features.get(&feature) else {
                blocked |= unlocked(&feature);
                continue;
            };
            for entry in entries {
                if let Some(dep) = entry.strip_prefix("dep:") {
                    blocked |= unlocked(dep);
                } else if let Some((dep, _)) = entry.split_once('/') {
                    if !dep.ends_with('?') && !features.contains_key(dep) {
                        blocked |= unlocked(dep);
                    }
                } else {
                    stack.push(entry.clone());
                }
            }
        }
        if blocked {
            impossible.insert(name.clone());
        }
    }
    impossible
}

/// The `[features]` table of a normalized (published) manifest: `name = ["entry", ...]`.
fn feature_table(manifest: &str) -> BTreeMap<String, Vec<String>> {
    let mut section = String::new();
    let mut inside = false;
    for line in manifest.lines() {
        if line.starts_with('[') {
            inside = line.trim() == "[features]";
            continue;
        }
        if inside {
            section.push_str(line);
            section.push('\n');
        }
    }
    let mut table = BTreeMap::new();
    for chunk in section.split(']') {
        let Some((key, list)) = chunk.split_once('=') else {
            continue;
        };
        let key = key.trim().trim_matches('"').to_owned();
        let Some(list) = list.split_once('[').map(|(_, rest)| rest) else {
            continue;
        };
        let entries = list
            .split(',')
            .map(|e| e.trim().trim_matches('"').to_owned())
            .filter(|e| !e.is_empty())
            .collect();
        table.insert(key, entries);
    }
    table
}

fn use_idents(tree: &syn::UseTree, out: &mut BTreeSet<String>) {
    match tree {
        syn::UseTree::Path(path) => {
            out.insert(unraw(&path.ident.to_string()));
            use_idents(&path.tree, out);
        }
        syn::UseTree::Name(name) => {
            out.insert(unraw(&name.ident.to_string()));
        }
        syn::UseTree::Rename(rename) => {
            out.insert(unraw(&rename.ident.to_string()));
            out.insert(unraw(&rename.rename.to_string()));
        }
        syn::UseTree::Glob(_) => {}
        syn::UseTree::Group(group) => group.items.iter().for_each(|t| use_idents(t, out)),
    }
}

/// Every `a as b` of a `use` tree, at any depth; `a::{self as b}` renames `a`.
fn use_renames(tree: &syn::UseTree, facts: &mut PackageFacts) {
    match tree {
        syn::UseTree::Rename(rename) => facts.renames.push((
            unraw(&rename.ident.to_string()),
            unraw(&rename.rename.to_string()),
        )),
        syn::UseTree::Path(path) => {
            if let syn::UseTree::Group(group) = &*path.tree {
                for item in &group.items {
                    if let syn::UseTree::Rename(rename) = item
                        && rename.ident == "self"
                    {
                        facts.renames.push((
                            unraw(&path.ident.to_string()),
                            unraw(&rename.rename.to_string()),
                        ));
                    }
                }
            }
            use_renames(&path.tree, facts);
        }
        syn::UseTree::Group(group) => group.items.iter().for_each(|t| use_renames(t, facts)),
        _ => {}
    }
}

fn check_attribute(meta: &syn::Meta, facts: &mut PackageFacts) {
    let path = meta.path();
    if path.segments.len() > 1 {
        let namespace = path.segments[0].ident.to_string();
        if !TOOL_NAMESPACES.contains(&namespace.as_str())
            && let Some(last) = path.segments.last()
        {
            facts.attributes.insert(unraw(&last.ident.to_string()));
        }
        return;
    }
    let Some(name) = path.get_ident().map(|i| unraw(&i.to_string())) else {
        facts.refuse(|| "an attribute without a name".into());
        return;
    };
    match name.as_str() {
        "derive" => {
            let parsed = match meta {
                syn::Meta::List(list) => list
                    .parse_args_with(Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)
                    .ok(),
                _ => None,
            };
            let Some(paths) = parsed else {
                facts.refuse(|| "an unreadable derive".into());
                return;
            };
            for derived in paths {
                let last = derived
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();
                let std_path = derived.segments.len() == 1
                    || derived
                        .segments
                        .first()
                        .is_some_and(|s| s.ident == "std" || s.ident == "core");
                if !(std_path && STD_DERIVES.contains(&last.as_str())) {
                    facts.attributes.insert(unraw(&last));
                }
            }
        }
        "cfg_attr" | "unsafe" => {
            let parsed = match meta {
                syn::Meta::List(list) => list
                    .parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
                    .ok(),
                _ => None,
            };
            let Some(inner) = parsed else {
                facts.refuse(|| format!("an unreadable `{name}` attribute"));
                return;
            };
            if name == "cfg_attr"
                && inner
                    .first()
                    .is_some_and(|p| cfg_false(p, &facts.impossible))
            {
                return;
            }
            let skip = usize::from(name == "cfg_attr");
            for nested in inner.iter().skip(skip) {
                check_attribute(nested, facts);
            }
        }
        "path" => {
            let value = match meta {
                syn::Meta::NameValue(syn::MetaNameValue {
                    value:
                        syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(value),
                            ..
                        }),
                    ..
                }) => Some(value.value()),
                _ => None,
            };
            if !value
                .is_some_and(|value| value.ends_with(".rs") && stays_inside(&facts.file, &value))
            {
                let file = facts.file.clone();
                facts.refuse(|| format!("a `#[path]` in {file} that may leave the package"));
            }
        }
        "proc_macro_derive" => {
            let exported = match meta {
                syn::Meta::List(list) => list
                    .parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
                    .ok()
                    .and_then(|m| m.first().and_then(|m| m.path().get_ident().cloned())),
                _ => None,
            };
            match exported {
                Some(derive) => {
                    facts.proc_macros.insert(unraw(&derive.to_string()));
                }
                None => facts.refuse(|| "an unreadable `proc_macro_derive`".into()),
            }
        }
        other if BUILTIN_ATTRIBUTES.contains(&other) => {}
        other => {
            facts.attributes.insert(other.to_owned());
        }
    }
}

/// Whether `value`, relative to `file`'s directory (the shallowest base a `#[path]` resolves
/// against; a deeper one only climbs less), stays inside the package's read files.
fn stays_inside(file: &str, value: &str) -> bool {
    if value.starts_with('/') || value.contains('\\') {
        return false;
    }
    let mut depth: Vec<&str> = file.split('/').collect();
    depth.pop();
    for part in value.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if depth.pop().is_none() {
                    return false;
                }
            }
            other => depth.push(other),
        }
    }
    depth
        .first()
        .is_none_or(|first| !OWN_TARGET_DIRS.contains(first))
}

/// The transcribers of a `macro_rules!` body (`(matcher) => { transcriber };`...). Answers
/// whether a lone `trait` input is matched literally (see `PackageFacts::keyword_literal`).
fn transcribers(body: TokenStream, facts: &mut PackageFacts) -> bool {
    let trees: Vec<TokenTree> = body.into_iter().collect();
    let mut keyword: Option<bool> = None;
    let mut index = 0;
    while index < trees.len() {
        let rule = (
            trees.get(index),
            trees.get(index + 1),
            trees.get(index + 2),
            trees.get(index + 3),
        );
        match rule {
            (
                Some(TokenTree::Group(matcher)),
                Some(TokenTree::Punct(eq)),
                Some(TokenTree::Punct(gt)),
                Some(TokenTree::Group(transcriber)),
            ) if eq.as_char() == '=' && gt.as_char() == '>' => {
                if keyword.is_none() {
                    let tokens: Vec<TokenTree> = matcher.stream().into_iter().collect();
                    match tokens.as_slice() {
                        [] => {}
                        [TokenTree::Ident(only)] if only == "trait" => keyword = Some(true),
                        [TokenTree::Ident(_) | TokenTree::Literal(_)] => {}
                        [TokenTree::Punct(p)] if p.as_char() != '$' => {}
                        _ => keyword = Some(false),
                    }
                }
                scan_tokens(transcriber.stream(), false, None, facts);
                index += 4;
                if let Some(TokenTree::Punct(semi)) = trees.get(index)
                    && semi.as_char() == ';'
                {
                    index += 1;
                }
            }
            _ => {
                facts.refuse(|| "a `macro_rules!` body this reading does not follow".into());
                return false;
            }
        }
    }
    keyword.unwrap_or(true)
}

/// Finds trait bodies in expansion tokens: after a literal `trait`, or after a header holding a
/// metavariable and nothing that makes it another item or an expression. `input` is whether the
/// tokens are a macro invocation's input, and `invoked` that macro when they are its whole input.
/// Angle brackets are tracked, so a const-generic block (`Other<{ 1 }>`) is never a body and a
/// default (`<X = u8>`) never ends a header.
fn scan_tokens(stream: TokenStream, input: bool, invoked: Option<&str>, facts: &mut PackageFacts) {
    let trees: Vec<TokenTree> = stream.into_iter().collect();
    for tree in &trees {
        if let TokenTree::Ident(ident) = tree {
            facts.mentioned.insert(unraw(&ident.to_string()));
        }
    }
    if let (true, Some(name), [TokenTree::Ident(only)]) = (input, invoked, trees.as_slice())
        && only == "trait"
    {
        facts.keyword_inputs.insert(name.to_owned());
        return;
    }
    let depths = angle_depths(&trees);
    let mut header_start = 0;
    for (index, tree) in trees.iter().enumerate() {
        // `name!(..)`, `$crate::name!{..}`, `$callback!(..)`: the group is another macro's input.
        let callee = match (
            index.checked_sub(2).map(|i| &trees[i]),
            index.checked_sub(1).map(|i| &trees[i]),
        ) {
            (Some(TokenTree::Ident(name)), Some(TokenTree::Punct(bang)))
                if bang.as_char() == '!' && !KEYWORDS.contains(&name.to_string().as_str()) =>
            {
                Some(unraw(&name.to_string()))
            }
            _ => None,
        };
        // Expansion tokens spell attributes and macro invocations the item walk never sees: each
        // name counts like a module-level one (a procedural macro of the closure is unseen).
        if let Some(name) = &callee {
            facts.item_macros.insert(name.clone());
        }
        let attribute = matches!(tree, TokenTree::Group(g) if g.delimiter() == Delimiter::Bracket)
            && index >= 1
            && match &trees[index - 1] {
                TokenTree::Punct(p) if p.as_char() == '#' => true,
                TokenTree::Punct(p) if p.as_char() == '!' => {
                    matches!(index.checked_sub(2).map(|i| &trees[i]), Some(TokenTree::Punct(h)) if h.as_char() == '#')
                }
                _ => false,
            };
        if let (true, TokenTree::Group(group)) = (attribute, tree) {
            stream_names(group.stream(), &mut facts.attributes);
        }
        match tree {
            TokenTree::Ident(ident) if ident == "trait" && input => facts.keyword_in_input = true,
            TokenTree::Ident(ident) if ident == "trait" => {
                if let Err(reason) = literal_trait_header(&trees, &depths, index) {
                    let file = facts.file.clone();
                    facts.refuse(|| format!("a `trait` {reason} in {file}"));
                }
            }
            TokenTree::Group(group)
                if group.delimiter() == Delimiter::Brace && depths[index] == 0 =>
            {
                match trait_header(&trees[header_start..index]) {
                    Header::Literal => trait_body(group.stream(), false, facts),
                    Header::Completed => trait_body(group.stream(), true, facts),
                    Header::None => {}
                }
                scan_tokens(
                    group.stream(),
                    input || callee.is_some(),
                    callee.as_deref(),
                    facts,
                );
                header_start = index + 1;
            }
            TokenTree::Group(group) => scan_tokens(
                group.stream(),
                input || callee.is_some(),
                callee.as_deref(),
                facts,
            ),
            TokenTree::Punct(p) if p.as_char() == ';' && depths[index] == 0 => {
                header_start = index + 1
            }
            _ => {}
        }
    }
}

/// The angle-bracket depth before each token (`->` and `=>` close nothing; `;` resets).
fn angle_depths(trees: &[TokenTree]) -> Vec<usize> {
    let mut depths = Vec::with_capacity(trees.len());
    let mut depth = 0usize;
    for (index, tree) in trees.iter().enumerate() {
        depths.push(depth);
        if let TokenTree::Punct(p) = tree {
            match p.as_char() {
                '<' => depth += 1,
                '>' => {
                    let arrow = matches!(
                        index.checked_sub(1).map(|i| &trees[i]),
                        Some(TokenTree::Punct(q)) if matches!(q.as_char(), '-' | '=')
                            && q.spacing() == proc_macro2::Spacing::Joint
                    );
                    if !arrow {
                        depth = depth.saturating_sub(1);
                    }
                }
                ';' => depth = 0,
                _ => {}
            }
        }
    }
    depths
}

/// Keywords that start another item: between a literal `trait` and its body, one means the
/// brace group is that item's and the trait's body comes from elsewhere.
const ITEM_STARTS: &[&str] = &[
    "const",
    "enum",
    "extern",
    "fn",
    "impl",
    "macro_rules",
    "mod",
    "static",
    "struct",
    "trait",
    "type",
    "union",
    "use",
];

/// A transcribed or spelled `trait` at `index` must be followed by its own body: the first brace
/// group outside angle brackets, with no other item, macro invocation or free-standing
/// metavariable (one that could be the body itself) before it. A metavariable may name the trait
/// or stand in a bound or a where clause.
fn literal_trait_header(
    trees: &[TokenTree],
    depths: &[usize],
    index: usize,
) -> Result<(), &'static str> {
    for at in index + 1..trees.len() {
        let top = depths[at] == 0;
        match &trees[at] {
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace && top => return Ok(()),
            TokenTree::Punct(p) if p.as_char() == ';' && top => break,
            TokenTree::Punct(p) if p.as_char() == '!' && top => {
                return Err("whose header holds a macro invocation");
            }
            TokenTree::Ident(i) if top && ITEM_STARTS.contains(&i.to_string().as_str()) => {
                return Err("followed by another item before its body");
            }
            TokenTree::Punct(p) if p.as_char() == '$' && top => {
                let crate_path =
                    matches!(trees.get(at + 1), Some(TokenTree::Ident(i)) if i == "crate");
                let placed = match &trees[at - 1] {
                    TokenTree::Ident(i) => i == "trait" || i == "where",
                    TokenTree::Punct(q) => matches!(q.as_char(), ':' | '+' | ','),
                    _ => false,
                };
                if !(crate_path || placed) {
                    return Err("whose body may be a metavariable");
                }
            }
            _ => {}
        }
    }
    Err("whose body comes from elsewhere")
}

/// Every identifier in `stream`, at any depth: an attribute's path, derives and arguments.
fn stream_names(stream: TokenStream, out: &mut BTreeSet<String>) {
    for tree in stream {
        match tree {
            TokenTree::Ident(ident) => {
                out.insert(unraw(&ident.to_string()));
            }
            TokenTree::Group(group) => stream_names(group.stream(), out),
            _ => {}
        }
    }
}

enum Header {
    /// A literal `trait`.
    Literal,
    /// A metavariable, and nothing that makes it another item or an expression.
    Completed,
    None,
}

fn trait_header(header: &[TokenTree]) -> Header {
    let depths = angle_depths(header);
    let mut metavariable = false;
    for (index, tree) in header.iter().enumerate() {
        let top = depths[index] == 0;
        match tree {
            TokenTree::Ident(ident) if ident == "trait" => return Header::Literal,
            // `for<'a>` in a where clause is a bound, not a loop.
            TokenTree::Ident(ident)
                if ident == "for"
                    && matches!(header.get(index + 1), Some(TokenTree::Punct(p)) if p.as_char() == '<') =>
                {}
            TokenTree::Ident(ident) if top && NOT_A_TRAIT.contains(&ident.to_string().as_str()) => {
                return Header::None;
            }
            TokenTree::Punct(p) if top && matches!(p.as_char(), '=' | '|' | '!') => {
                return Header::None;
            }
            TokenTree::Punct(p) if p.as_char() == '$' => {
                let crate_path =
                    matches!(header.get(index + 1), Some(TokenTree::Ident(i)) if i == "crate");
                metavariable |= !crate_path;
            }
            _ => {}
        }
    }
    if metavariable {
        Header::Completed
    } else {
        Header::None
    }
}

/// A trait body in expansion tokens: its `fn` names, refused when a metavariable or a macro
/// invocation among its own items could supply more. Only the body's top level holds items; a
/// nested group is a signature or a default body, whose tokens declare no trait method.
/// `completed` bodies (a metavariable header) record apart, counted only if the keyword can bind.
fn trait_body(stream: TokenStream, completed: bool, facts: &mut PackageFacts) {
    let trees: Vec<TokenTree> = stream.into_iter().collect();
    let mut names = BTreeSet::new();
    let mut unknown = None;
    for (index, tree) in trees.iter().enumerate() {
        match tree {
            TokenTree::Ident(ident) if ident == "fn" => match trees.get(index + 1) {
                Some(TokenTree::Ident(name)) => {
                    names.insert(unraw(&name.to_string()));
                }
                Some(TokenTree::Punct(p)) if p.as_char() == '$' => {
                    unknown.get_or_insert("a trait method named by a metavariable".to_owned());
                }
                _ => {}
            },
            TokenTree::Ident(ident)
                if !KEYWORDS.contains(&ident.to_string().as_str())
                    && matches!(trees.get(index + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!')
                    && matches!(trees.get(index + 2), Some(TokenTree::Group(_))) =>
            {
                unknown.get_or_insert(format!("a macro invocation `{ident}!` in a trait body"));
            }
            TokenTree::Punct(p) if p.as_char() == '$' => {
                let crate_path =
                    matches!(trees.get(index + 1), Some(TokenTree::Ident(i)) if i == "crate");
                if !crate_path {
                    unknown.get_or_insert("a metavariable in a trait body".to_owned());
                }
            }
            _ => {}
        }
    }
    let unknown = unknown.map(|reason| format!("{reason} in {}", facts.file));
    if completed {
        facts.completed_methods.extend(names);
        if facts.completed_unknown.is_none() {
            facts.completed_unknown = unknown;
        }
    } else {
        facts.trait_methods.extend(names);
        if let Some(reason) = unknown {
            facts.refuse(|| reason);
        }
    }
}

fn unraw(name: &str) -> String {
    name.strip_prefix("r#").unwrap_or(name).to_owned()
}

mod items;

#[cfg(test)]
mod tests;
