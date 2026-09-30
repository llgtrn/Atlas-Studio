//! Cross-module call resolution for TypeScript/JavaScript (G158, FULL_OSS_REPLAY R5, ADR 0073;
//! DEBT-MULTILANGUAGE).
//!
//! The G152 profile resolves a bare identifier only to a function of the same file. A call to
//! an imported function stayed unresolved even when the import names a file of the same
//! repository. This module adds the part of ECMAScript module linking that fixes such a callee
//! statically:
//!
//! - **Module facts** of each file ([`module_facts`]): its named and default imports, its exports
//!   (declarations, `export { a as b }`, `export default f`), its re-exports
//!   (`export { a } from 's'`), its `export * from 's'`, and its module-level functions whose
//!   name is bound exactly once in the file and never assigned (the extractor's own rule).
//! - **Specifiers** ([`resolve_specifier`]): a relative specifier names a file by the first
//!   existing candidate in a fixed order (the path itself, `.js` spelled for a `.ts` source, the
//!   script extensions, then `index` files); a bare specifier names a workspace package only when
//!   an inventoried `package.json` declares exactly that `name` and a `source` entry that exists,
//!   or (G160) compiled entries its sibling `tsconfig.json` maps back to one source through the
//!   declared `outDir` -> `rootDir`.
//!   Anything else -- a registry package, a subpath, a non-script file -- is outside.
//! - **Bindings** ([`resolve_imports`]): an imported name follows exports, re-exports and star
//!   exports to the module-level function that defines it. Two star exports providing the same
//!   name are ambiguous and resolve to nothing; cycles and chains deeper than
//!   [`MAX_LINK_DEPTH`] resolve to nothing.
//!
//! Namespace imports (`import * as ns`), member calls, dynamic `import()`, CommonJS `require`,
//! `tsconfig` path aliases and package `exports` maps stay unresolved: they are never guessed.

use std::collections::{BTreeMap, BTreeSet};

use atlas_core::SemanticRecordId;
use tree_sitter::{Node, Parser};

use super::super::extractor::ExtractionInput;
use super::{Context, Walk, grammar};

/// How many export hops a binding may follow before it is left unresolved.
pub const MAX_LINK_DEPTH: usize = 16;

/// What one file imports, exports and defines, for linking.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModuleFacts {
    /// Local name -> (specifier, imported name); a default import imports `default`.
    pub imports: BTreeMap<String, (String, String)>,
    /// Exported name -> local name (`default` for a default export).
    pub exports: BTreeMap<String, String>,
    /// Exported name -> (specifier, imported name), for `export { a as b } from 's'`.
    pub re_exports: BTreeMap<String, (String, String)>,
    /// Specifiers of `export * from 's'`, in source order.
    pub stars: Vec<String>,
    /// Module-level functions a name fixes: bound exactly once in the file, never assigned.
    pub functions: BTreeMap<String, SemanticRecordId>,
    /// Names bound exactly once in the file and never assigned.
    pub unique: BTreeSet<String>,
}

/// The module facts of one TypeScript/JavaScript file, or `None` when it does not parse.
pub fn module_facts(input: &ExtractionInput) -> Option<ModuleFacts> {
    let extractor = atlas_core::ExtractorIdentity {
        id: super::TYPESCRIPT_SEMANTIC_EXTRACTOR_ID.into(),
        version: super::TYPESCRIPT_SEMANTIC_EXTRACTOR_VERSION.into(),
    };
    let mut ctx = Context::new(input, extractor);
    let mut parser = Parser::new();
    parser.set_language(&grammar(&input.artifact_path)).ok()?;
    let tree = parser.parse(&input.source_text, None)?;
    let root = tree.root_node();
    ctx.collect_bindings(root, 0);
    ctx.walk(root, &Walk::module());
    let unique: BTreeSet<String> = ctx
        .bindings
        .keys()
        .filter(|name| ctx.fixed(name) && !ctx.expression_names.iter().any(|(n, ..)| n == *name))
        .cloned()
        .collect();
    let mut facts = ModuleFacts {
        functions: ctx
            .module_functions
            .iter()
            .filter(|(name, visible)| visible.top && unique.contains(*name))
            .map(|(name, visible)| (name.clone(), visible.id.clone()))
            .collect(),
        unique,
        ..ModuleFacts::default()
    };
    let text = |node: Node| node.utf8_text(ctx.source).unwrap_or("").to_owned();
    let mut cursor = root.walk();
    for statement in root.named_children(&mut cursor) {
        match statement.kind() {
            "import_statement" => import_facts(statement, &text, &mut facts),
            "export_statement" => export_facts(statement, &text, &mut facts),
            _ => {}
        }
    }
    Some(facts)
}

fn unquote(text: String) -> String {
    text.trim_matches(|c| c == '\'' || c == '"' || c == '`')
        .to_owned()
}

fn has_token(node: Node, token: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|c| c.kind() == token)
}

fn import_facts(node: Node, text: &impl Fn(Node) -> String, facts: &mut ModuleFacts) {
    // `import type { .. }` imports no value.
    if has_token(node, "type") {
        return;
    }
    let Some(source) = node.child_by_field_name("source").map(|s| unquote(text(s))) else {
        return;
    };
    let mut cursor = node.walk();
    for clause in node.named_children(&mut cursor) {
        if clause.kind() != "import_clause" {
            continue;
        }
        let mut inner = clause.walk();
        for part in clause.named_children(&mut inner) {
            match part.kind() {
                "identifier" => {
                    facts
                        .imports
                        .insert(text(part), (source.clone(), "default".into()));
                }
                "named_imports" => {
                    let mut specifiers = part.walk();
                    for specifier in part.named_children(&mut specifiers) {
                        if specifier.kind() != "import_specifier" || has_token(specifier, "type") {
                            continue;
                        }
                        let Some(name) = specifier.child_by_field_name("name") else {
                            continue;
                        };
                        let imported = unquote(text(name));
                        let local = specifier
                            .child_by_field_name("alias")
                            .map(text)
                            .unwrap_or_else(|| imported.clone());
                        facts.imports.insert(local, (source.clone(), imported));
                    }
                }
                // `import * as ns`: member calls through a namespace are outside.
                _ => {}
            }
        }
    }
}

fn export_facts(node: Node, text: &impl Fn(Node) -> String, facts: &mut ModuleFacts) {
    if has_token(node, "type") {
        return;
    }
    let source = node.child_by_field_name("source").map(|s| unquote(text(s)));
    let default = has_token(node, "default");
    if let Some(declaration) = node.child_by_field_name("declaration") {
        let names: Vec<String> = declared_names(declaration).into_iter().map(text).collect();
        for name in names {
            let exported = if default {
                "default".into()
            } else {
                name.clone()
            };
            facts.exports.insert(exported, name);
        }
        return;
    }
    if default {
        // `export default name;`
        if let Some(value) = node.child_by_field_name("value")
            && value.kind() == "identifier"
        {
            facts.exports.insert("default".into(), text(value));
        }
        return;
    }
    let mut cursor = node.walk();
    let clause = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "export_clause");
    match (clause, source) {
        (Some(clause), source) => {
            let mut specifiers = clause.walk();
            for specifier in clause.named_children(&mut specifiers) {
                if specifier.kind() != "export_specifier" || has_token(specifier, "type") {
                    continue;
                }
                let Some(name) = specifier.child_by_field_name("name") else {
                    continue;
                };
                let local = unquote(text(name));
                let exported = specifier
                    .child_by_field_name("alias")
                    .map(|a| unquote(text(a)))
                    .unwrap_or_else(|| local.clone());
                match &source {
                    Some(source) => {
                        facts.re_exports.insert(exported, (source.clone(), local));
                    }
                    None => {
                        facts.exports.insert(exported, local);
                    }
                }
            }
        }
        // `export * from 's'`; `export * as ns from 's'` nests its `*` in a `namespace_export`.
        (None, Some(source)) => {
            if has_token(node, "*") {
                facts.stars.push(source);
            }
        }
        (None, None) => {}
    }
}

/// The value names an exported declaration binds: a function, class, enum or namespace name,
/// every identifier of a variable's destructuring pattern (`export const { a, b: [c] } = o`),
/// and the names of an ambient declaration (`export declare const x`, `export declare function
/// f()`), and an `export import` alias. A local export shadows `export *`, so a missed name
/// would let a star export resolve it to another module's function (G188, ADR 0101).
/// Interfaces and type aliases bind no value. An ambient declaration, a non-instantiated
/// namespace or a `const enum` shadows the star only in the type system -- it emits no
/// JavaScript, so at run time the star may supply the name -- and the import is left
/// unresolved: withheld, never a wrong edge.
fn declared_names(declaration: Node) -> Vec<Node> {
    match declaration.kind() {
        "function_declaration"
        | "generator_function_declaration"
        | "function_signature"
        | "class_declaration"
        | "abstract_class_declaration"
        | "enum_declaration" => declaration
            .child_by_field_name("name")
            .into_iter()
            .collect(),
        // `export namespace N {}`, `export module N {}`, `export namespace N.B.C {}` (binds `N`;
        // tree-sitter nests `N.B` as a member expression inside the dotted name);
        // a string-named ambient module binds no name.
        "internal_module" | "module" => {
            let mut name = declaration.child_by_field_name("name");
            while let Some(nested) =
                name.filter(|n| matches!(n.kind(), "nested_identifier" | "member_expression"))
            {
                name = nested.child_by_field_name("object");
            }
            name.filter(|n| n.kind() == "identifier")
                .into_iter()
                .collect()
        }
        // `export import M = N.X`: the alias is the first identifier.
        "import_alias" => {
            let mut cursor = declaration.walk();
            declaration
                .named_children(&mut cursor)
                .find(|c| c.kind() == "identifier")
                .into_iter()
                .collect()
        }
        "lexical_declaration" | "variable_declaration" => {
            let mut cursor = declaration.walk();
            declaration
                .named_children(&mut cursor)
                .filter(|declarator| declarator.kind() == "variable_declarator")
                .filter_map(|declarator| declarator.child_by_field_name("name"))
                .flat_map(super::pattern_identifiers)
                .collect()
        }
        "ambient_declaration" => {
            let mut cursor = declaration.walk();
            let inner: Vec<Node> = declaration.named_children(&mut cursor).collect();
            inner.into_iter().flat_map(declared_names).collect()
        }
        _ => Vec::new(),
    }
}

/// The directory part of a repository-relative path.
fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

/// `dir` joined with a relative path, `.` and `..` resolved; `None` above the root.
fn join(dir: &str, relative: &str) -> Option<String> {
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

const SCRIPT_EXTENSIONS: [&str; 8] = ["ts", "tsx", "js", "jsx", "mts", "mjs", "cts", "cjs"];

/// What one inventoried `package.json` and the `tsconfig.json` beside it declare about the
/// package's entry (G158 `source`; G160 compiled outputs mapped back through the compiler's
/// declared `outDir` -> `rootDir`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageDeclaration {
    /// The manifest's repository-relative path.
    pub manifest: String,
    pub name: String,
    /// The `source` field, when declared.
    pub source: Option<String>,
    /// The compiled entries the manifest declares (`types`, `module`, `main`, the `.` export).
    pub outputs: Vec<String>,
    /// `compilerOptions.rootDir` and `outDir` of the sibling `tsconfig.json`, when declared.
    pub root_dir: Option<String>,
    pub out_dir: Option<String>,
}

/// The source extensions a compiled entry's extension is emitted from.
fn emitted_from(output: &str) -> Option<(&str, &'static [&'static str])> {
    [
        (".d.ts", &["ts", "tsx"][..]),
        (".d.mts", &["mts"][..]),
        (".d.cts", &["cts"][..]),
        (".js", &["ts", "tsx"][..]),
        (".mjs", &["mts"][..]),
        (".cjs", &["cts"][..]),
    ]
    .into_iter()
    .find_map(|(extension, sources)| output.strip_suffix(extension).map(|stem| (stem, sources)))
}

fn trimmed(path: &str) -> &str {
    path.trim_start_matches("./").trim_end_matches('/')
}

/// The source file a package's entry is: its declared `source`, or (G160) the one existing file
/// every compiled entry under the declared `outDir` is emitted from, found under the declared
/// `rootDir`. Nothing is guessed: without both directories, or when the entries disagree, the
/// package has no entry.
pub fn package_entry(declaration: &PackageDeclaration, files: &BTreeSet<String>) -> Option<String> {
    let dir = dir_of(&declaration.manifest);
    if let Some(source) = &declaration.source {
        return join(dir, source);
    }
    let (root, out) = (
        trimmed(declaration.root_dir.as_deref()?),
        trimmed(declaration.out_dir.as_deref()?),
    );
    let mut entries = BTreeSet::new();
    for output in &declaration.outputs {
        let Some(rest) = trimmed(output)
            .strip_prefix(out)
            .and_then(|r| r.strip_prefix('/'))
        else {
            continue;
        };
        let Some((stem, sources)) = emitted_from(rest) else {
            continue;
        };
        let found = sources.iter().find_map(|extension| {
            let candidate = join(dir, &format!("{root}/{stem}.{extension}"))?;
            files.contains(&candidate).then_some(candidate)
        });
        entries.insert(found?);
    }
    match entries.len() {
        1 => entries.into_iter().next(),
        _ => None,
    }
}

/// The workspace packages among inventoried manifests: package name -> entry file. A name
/// declared by two manifests names nothing.
pub fn workspace_packages(
    declarations: &[PackageDeclaration],
    files: &BTreeSet<String>,
) -> BTreeMap<String, String> {
    let mut by_name: BTreeMap<&str, Vec<Option<String>>> = BTreeMap::new();
    for declaration in declarations {
        by_name
            .entry(&declaration.name)
            .or_default()
            .push(package_entry(declaration, files));
    }
    by_name
        .into_iter()
        .filter_map(|(name, entries)| match entries.as_slice() {
            [Some(entry)] => Some((name.to_owned(), entry.clone())),
            _ => None,
        })
        .collect()
}

/// The file `specifier` names from `from`, among `files`, or `None` when it names none of them.
pub fn resolve_specifier(
    from: &str,
    specifier: &str,
    files: &BTreeSet<String>,
    packages: &BTreeMap<String, String>,
) -> Option<String> {
    if !(specifier.starts_with("./") || specifier.starts_with("../") || specifier == ".") {
        return packages
            .get(specifier)
            .filter(|entry| files.contains(*entry))
            .cloned();
    }
    let base = join(dir_of(from), specifier)?;
    let mut candidates = vec![base.clone()];
    // TypeScript sources import each other by their emitted `.js` names.
    for (emitted, source) in [("js", "ts"), ("jsx", "tsx"), ("mjs", "mts"), ("cjs", "cts")] {
        if let Some(stem) = base.strip_suffix(&format!(".{emitted}")) {
            candidates.push(format!("{stem}.{source}"));
        }
    }
    candidates.extend(SCRIPT_EXTENSIONS.iter().map(|e| format!("{base}.{e}")));
    candidates.extend(
        SCRIPT_EXTENSIONS
            .iter()
            .map(|e| format!("{base}/index.{e}")),
    );
    candidates.into_iter().find(|candidate| {
        files.contains(candidate)
            && candidate
                .rsplit_once('.')
                .is_some_and(|(_, e)| SCRIPT_EXTENSIONS.contains(&e))
    })
}

/// One import binding resolved to the module-level function that defines it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportBinding {
    pub path: String,
    pub local: String,
    pub function: SemanticRecordId,
    /// The files the binding crossed, from the importing file's first target to the defining one.
    pub chain: Vec<String>,
}

struct Linker<'a> {
    facts: &'a BTreeMap<String, ModuleFacts>,
    files: BTreeSet<String>,
    packages: &'a BTreeMap<String, String>,
}

impl Linker<'_> {
    fn target(&self, from: &str, specifier: &str) -> Option<String> {
        resolve_specifier(from, specifier, &self.files, self.packages)
    }

    /// The function a name bound locally in `file` fixes: a function of the file, or an import.
    fn local(
        &self,
        file: &str,
        name: &str,
        depth: usize,
        chain: &mut Vec<String>,
    ) -> Option<SemanticRecordId> {
        let facts = self.facts.get(file)?;
        if let Some(id) = facts.functions.get(name) {
            return Some(id.clone());
        }
        let (specifier, imported) = facts.imports.get(name)?;
        if !facts.unique.contains(name) {
            return None;
        }
        let target = self.target(file, specifier)?;
        self.export(&target, imported, depth + 1, chain)
    }

    /// The function `file` exports as `name`.
    fn export(
        &self,
        file: &str,
        name: &str,
        depth: usize,
        chain: &mut Vec<String>,
    ) -> Option<SemanticRecordId> {
        if depth > MAX_LINK_DEPTH || chain.iter().any(|seen| seen == file) {
            return None;
        }
        chain.push(file.to_owned());
        let facts = self.facts.get(file)?;
        if let Some(local) = facts.exports.get(name) {
            return self.local(file, local, depth, chain);
        }
        if let Some((specifier, imported)) = facts.re_exports.get(name) {
            let target = self.target(file, specifier)?;
            return self.export(&target, imported, depth + 1, chain);
        }
        if name == "default" {
            return None;
        }
        // Star exports: exactly one of them may provide the name.
        let mut found: Option<(SemanticRecordId, Vec<String>)> = None;
        for specifier in &facts.stars {
            let Some(target) = self.target(file, specifier) else {
                continue;
            };
            let mut branch = chain.clone();
            if let Some(id) = self.export(&target, name, depth + 1, &mut branch) {
                match &found {
                    Some((existing, _)) if *existing != id => return None,
                    Some(_) => {}
                    None => found = Some((id, branch)),
                }
            }
        }
        let (id, branch) = found?;
        *chain = branch;
        Some(id)
    }
}

/// Every import of every file bound to the module-level function it names, across files and
/// workspace packages.
pub fn resolve_imports(
    facts: &BTreeMap<String, ModuleFacts>,
    packages: &BTreeMap<String, String>,
) -> Vec<ImportBinding> {
    let linker = Linker {
        facts,
        files: facts.keys().cloned().collect(),
        packages,
    };
    let mut out = Vec::new();
    for (path, file) in facts {
        for local in file.imports.keys() {
            let mut chain = Vec::new();
            if let Some(function) = linker.local(path, local, 0, &mut chain) {
                out.push(ImportBinding {
                    path: path.clone(),
                    local: local.clone(),
                    function,
                    chain,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
