//! TypeScript/JavaScript semantic extractor: the bounded first profile of a second language (G152,
//! ADR 0068; DEBT-MULTILANGUAGE).
//!
//! tree-sitter (MIT) is the syntax substrate only: it turns source text into a concrete syntax
//! tree. A parse is never semantic support. Every record and every obligation below is Atlas's
//! own, under the same typed-record contract as Rust.
//!
//! Declared profile:
//! - FUNCTION_IDENTITY / FUNCTION_SIGNATURE: function declarations, `const f = () => ..` and
//!   `const f = function ..` bindings, class methods (static methods are associated functions),
//!   and anonymous functions as `{closure@line:column}` regions scoped under their enclosing
//!   function. Calls at module level are attributed to a generated `{module}` region. Overload
//!   signatures, interface and abstract method signatures declare no body and are not functions.
//! - SYMBOL: definitions of functions, classes, interfaces, type aliases, enums and module-level
//!   variables; imported names as declarations under `import <source>`.
//! - CALL: every call and `new` site, attributed to its innermost region, with its callee
//!   spelling (`name`, `.method`, `Class::constructor`). A bare identifier is resolved only to a
//!   function declared at module level in the same file whose name is bound exactly once in the
//!   file and never assigned: JavaScript's lexical scoping then fixes the callee (DERIVED).
//!   Everything else -- member calls, imported callees, callbacks, `this`, dynamic property
//!   access, JSX elements, getters and setters, decorators, tagged templates -- stays unresolved,
//!   so CALL is always UNKNOWN with its observations.
//! - Every other dimension is UNSUPPORTED, never guessed.
//!
//! A file with syntax errors keeps its observations, but no dimension of it is exhaustive.

use std::collections::{BTreeMap, BTreeSet};

use atlas_core::{
    CallDispatchKind, CallSiteIdentity, EpistemicStatus, Evidence, EvidenceId,
    FunctionDeclarationKind, FunctionIdentity, FunctionOwner, FunctionParameter, FunctionSignature,
    PlaceRef, Provenance, SemanticDimension, SemanticObservation, SemanticRecordHeader,
    SemanticRecordId, SemanticScope, SourceSpan, SymbolIdentity, SymbolRole, TypeIdentity,
    stable_id,
};
use tree_sitter::{Node, Parser};

use super::batch::{ExtractionBatch, ObligationResult};
use super::extractor::{DiagnosticCode, ExtractionDiagnostic, ExtractionInput, SemanticExtractor};

pub const TYPESCRIPT_SEMANTIC_EXTRACTOR_ID: &str = "atlas.typescript.source-semantic.v1";
pub const TYPESCRIPT_SEMANTIC_EXTRACTOR_VERSION: &str = "0.1.0";

pub const SUPPORTED_DIMENSIONS: &[SemanticDimension] = &[
    SemanticDimension::Symbol,
    SemanticDimension::FunctionIdentity,
    SemanticDimension::FunctionSignature,
    SemanticDimension::Call,
];

/// Deeper nesting than this is not walked: the file's dimensions stay UNKNOWN (a resource bound
/// against adversarial nesting, like the Rust extractor's recursion guard).
const MAX_NESTING: usize = 1_000;
const STACK_SIZE: usize = 256 * 1024 * 1024;

#[derive(Debug)]
pub struct TypeScriptSemanticExtractor;

impl SemanticExtractor for TypeScriptSemanticExtractor {
    fn id(&self) -> &'static str {
        TYPESCRIPT_SEMANTIC_EXTRACTOR_ID
    }

    fn version(&self) -> &'static str {
        TYPESCRIPT_SEMANTIC_EXTRACTOR_VERSION
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        &["typescript", "javascript"]
    }

    fn supported_dimensions(&self) -> &'static [SemanticDimension] {
        SUPPORTED_DIMENSIONS
    }

    fn extract(&self, input: &ExtractionInput) -> ExtractionBatch {
        let extractor = self.identity();
        std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(STACK_SIZE)
                .spawn_scoped(scope, || extract_file(input, extractor))
                .expect("spawning the extraction worker thread must not fail")
                .join()
                .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
        })
    }
}

/// The grammar for an artifact: TSX for `.tsx`, TypeScript for `.ts`/`.mts`/`.cts`, JavaScript
/// (JSX included) otherwise.
fn grammar(path: &str) -> tree_sitter::Language {
    let extension = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match extension.as_str() {
        "tsx" => tree_sitter_typescript::LANGUAGE_TSX.into(),
        "ts" | "mts" | "cts" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        _ => tree_sitter_javascript::LANGUAGE.into(),
    }
}

fn extract_file(
    input: &ExtractionInput,
    extractor: atlas_core::ExtractorIdentity,
) -> ExtractionBatch {
    let mut ctx = Context::new(input, extractor);
    let mut parser = Parser::new();
    if parser.set_language(&grammar(&input.artifact_path)).is_err() {
        return ctx.finish_failure(DiagnosticCode::ParseFailure, "grammar version mismatch");
    }
    let Some(tree) = parser.parse(&input.source_text, None) else {
        return ctx.finish_failure(DiagnosticCode::ParseFailure, "the parser returned no tree");
    };
    let root = tree.root_node();
    if root.has_error() {
        ctx.partial.insert(format!(
            "syntax errors in {} (tree-sitter error or missing nodes)",
            input.artifact_path
        ));
    }
    ctx.collect_bindings(root, 0);
    ctx.walk(root, &Walk::module());
    ctx.resolve_calls();
    ctx.finish_success()
}

/// Where a node sits while walking: its scope segments and the executable region its calls
/// belong to.
#[derive(Clone)]
struct Walk {
    scope: Vec<String>,
    region: Option<SemanticRecordId>,
    /// The scope of the region's body (`.. fn name`), where its closures belong.
    region_scope: Vec<String>,
    /// The class a method belongs to.
    class: Option<String>,
    exported: bool,
    depth: usize,
}

impl Walk {
    fn module() -> Self {
        Self {
            scope: Vec::new(),
            region: None,
            region_scope: Vec::new(),
            class: None,
            exported: false,
            depth: 0,
        }
    }

    fn deeper(&self) -> Self {
        Self {
            depth: self.depth + 1,
            exported: false,
            ..self.clone()
        }
    }
}

struct Context<'a> {
    input: &'a ExtractionInput,
    source: &'a [u8],
    extractor: atlas_core::ExtractorIdentity,
    fingerprint: String,
    observations: Vec<SemanticObservation>,
    evidence: Vec<Evidence>,
    diagnostics: Vec<ExtractionDiagnostic>,
    records: BTreeMap<SemanticDimension, (Vec<SemanticRecordId>, Vec<EvidenceId>)>,
    seen: BTreeSet<String>,
    /// How often each identifier is bound anywhere in the file (declarations, parameters,
    /// imports, catch clauses), and which identifiers are assigned to.
    bindings: BTreeMap<String, usize>,
    assigned: BTreeSet<String>,
    /// Module-level functions by name, for same-file resolution.
    module_functions: BTreeMap<String, SemanticRecordId>,
    /// Constructors by class name.
    constructors: BTreeMap<String, SemanticRecordId>,
    module_region: Option<SemanticRecordId>,
    /// Calls seen while walking, resolved once every declaration of the file is known
    /// (declarations are hoisted: a call may precede the function it names).
    pending: Vec<PendingCall>,
    /// Why no dimension of this file can claim exhaustive coverage (empty: exhaustive).
    partial: BTreeSet<String>,
}

impl<'a> Context<'a> {
    fn new(input: &'a ExtractionInput, extractor: atlas_core::ExtractorIdentity) -> Self {
        let fingerprint = input.identity_key(&extractor);
        Self {
            input,
            source: input.source_text.as_bytes(),
            extractor,
            fingerprint,
            observations: Vec::new(),
            evidence: Vec::new(),
            diagnostics: Vec::new(),
            records: BTreeMap::new(),
            seen: BTreeSet::new(),
            bindings: BTreeMap::new(),
            assigned: BTreeSet::new(),
            module_functions: BTreeMap::new(),
            constructors: BTreeMap::new(),
            module_region: None,
            pending: Vec::new(),
            partial: BTreeSet::new(),
        }
    }

    fn text(&self, node: Node) -> String {
        node.utf8_text(self.source).unwrap_or("").to_owned()
    }

    fn span(&self, node: Node) -> SourceSpan {
        let at = node.start_position();
        SourceSpan {
            path: self.input.artifact_path.clone(),
            line: at.row + 1,
            column: at.column,
        }
    }

    fn wants(&self, dimension: SemanticDimension) -> bool {
        self.input.requested_dimensions.contains(&dimension)
    }

    /// Pass one: every binding of an identifier in the file, and every assignment to one.
    fn collect_bindings(&mut self, node: Node, depth: usize) {
        if depth > MAX_NESTING {
            return;
        }
        let named = |field: &str| node.child_by_field_name(field);
        match node.kind() {
            "function_declaration"
            | "generator_function_declaration"
            | "class_declaration"
            | "abstract_class_declaration"
            | "variable_declarator"
            | "import_specifier"
            | "namespace_import"
            | "enum_declaration" => {
                let name = named("alias").or_else(|| named("name")).or_else(|| {
                    // `import * as ns` carries its identifier as a child.
                    let mut cursor = node.walk();
                    node.named_children(&mut cursor)
                        .find(|c| c.kind() == "identifier")
                });
                if let Some(name) = name {
                    self.bind_pattern(name);
                }
            }
            "import_clause" => {
                // The default import is the clause's bare identifier child.
                let mut cursor = node.walk();
                let defaults: Vec<Node> = node
                    .named_children(&mut cursor)
                    .filter(|c| c.kind() == "identifier")
                    .collect();
                for name in defaults {
                    self.bind_pattern(name);
                }
            }
            "formal_parameters" | "catch_clause" => {
                let mut cursor = node.walk();
                let children: Vec<Node> = node.named_children(&mut cursor).collect();
                for child in children {
                    if node.kind() == "catch_clause" && child.kind() == "statement_block" {
                        continue;
                    }
                    self.bind_pattern(child);
                }
            }
            "arrow_function" => {
                if let Some(parameter) = named("parameter") {
                    self.bind_pattern(parameter);
                }
            }
            "assignment_expression" | "augmented_assignment_expression" => {
                if let Some(left) = named("left")
                    && left.kind() == "identifier"
                {
                    self.assigned.insert(self.text(left));
                }
            }
            _ => {}
        }
        let mut cursor = node.walk();
        let children: Vec<Node> = node.named_children(&mut cursor).collect();
        for child in children {
            self.collect_bindings(child, depth + 1);
        }
    }

    /// Counts every identifier a binding pattern introduces (destructuring included).
    fn bind_pattern(&mut self, node: Node) {
        match node.kind() {
            "identifier" | "shorthand_property_identifier_pattern" | "type_identifier" => {
                *self.bindings.entry(self.text(node)).or_default() += 1;
            }
            // A parameter's type annotation and default value bind nothing.
            "type_annotation" => {}
            _ => {
                if let Some(pattern) = node
                    .child_by_field_name("pattern")
                    .or_else(|| node.child_by_field_name("left"))
                {
                    self.bind_pattern(pattern);
                    return;
                }
                let mut cursor = node.walk();
                let children: Vec<Node> = node.named_children(&mut cursor).collect();
                for child in children {
                    if child.kind() == "property_identifier" {
                        continue;
                    }
                    self.bind_pattern(child);
                }
            }
        }
    }

    fn walk(&mut self, node: Node, at: &Walk) {
        if at.depth > MAX_NESTING {
            self.partial.insert(format!(
                "nesting deeper than {MAX_NESTING} in {} was not walked",
                self.input.artifact_path
            ));
            return;
        }
        match node.kind() {
            "export_statement" => {
                let inner = Walk {
                    exported: true,
                    depth: at.depth + 1,
                    ..at.clone()
                };
                self.walk_children(node, &inner);
                return;
            }
            "function_declaration" | "generator_function_declaration" => {
                if let Some(name) = node.child_by_field_name("name") {
                    let name = self.text(name);
                    self.function(node, &name, FunctionDeclarationKind::FreeFunction, at);
                    return;
                }
            }
            "variable_declarator" => {
                let name = node.child_by_field_name("name");
                let value = node.child_by_field_name("value");
                if let (Some(name), Some(value)) = (name, value)
                    && name.kind() == "identifier"
                    && matches!(
                        value.kind(),
                        "arrow_function"
                            | "function_expression"
                            | "function"
                            | "generator_function"
                    )
                {
                    let name = self.text(name);
                    self.symbol(at, &name, SymbolRole::Definition, node);
                    self.function(value, &name, FunctionDeclarationKind::FreeFunction, at);
                    return;
                }
                if at.region.is_none()
                    && let Some(name) = name.filter(|n| n.kind() == "identifier")
                {
                    let name = self.text(name);
                    self.symbol(at, &name, SymbolRole::Definition, node);
                }
            }
            "class_declaration" | "abstract_class_declaration" | "class" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| self.text(n))
                    .unwrap_or_else(|| {
                        let span = self.span(node);
                        format!("{{class@{}:{}}}", span.line, span.column)
                    });
                self.symbol(at, &name, SymbolRole::Definition, node);
                let mut scope = at.scope.clone();
                scope.push(name.clone());
                let inner = Walk {
                    scope,
                    class: Some(name),
                    depth: at.depth + 1,
                    ..at.clone()
                };
                if let Some(body) = node.child_by_field_name("body") {
                    self.walk_children(body, &inner);
                }
                return;
            }
            "method_definition" if node.parent().map(|p| p.kind()) != Some("class_body") => {
                // An object-literal method is a value, like any anonymous function.
                let span = self.span(node);
                let name = format!("{{closure@{}:{}}}", span.line, span.column);
                self.function(node, &name, FunctionDeclarationKind::Closure, at);
                return;
            }
            "method_definition" => {
                let named = node.child_by_field_name("name").filter(|n| {
                    matches!(
                        n.kind(),
                        "property_identifier" | "private_property_identifier" | "identifier"
                    )
                });
                match named {
                    Some(name) => {
                        let name = self.text(name);
                        let is_static = {
                            let mut cursor = node.walk();
                            node.children(&mut cursor).any(|c| c.kind() == "static")
                        };
                        let kind = if is_static {
                            FunctionDeclarationKind::AssociatedFunction
                        } else {
                            FunctionDeclarationKind::InherentMethod
                        };
                        self.function(node, &name, kind, at);
                        return;
                    }
                    None => {
                        self.partial.insert(format!(
                            "a method with a computed or string name in {}",
                            self.input.artifact_path
                        ));
                    }
                }
            }
            "arrow_function" | "function_expression" | "function" | "generator_function" => {
                let span = self.span(node);
                let name = format!("{{closure@{}:{}}}", span.line, span.column);
                self.function(node, &name, FunctionDeclarationKind::Closure, at);
                return;
            }
            "interface_declaration" | "type_alias_declaration" | "enum_declaration" => {
                if let Some(name) = node.child_by_field_name("name") {
                    let name = self.text(name);
                    self.symbol(at, &name, SymbolRole::Definition, node);
                }
            }
            "import_statement" => {
                self.imports(node, at);
                return;
            }
            // `export const f = ..`: the export reaches the declarators.
            "lexical_declaration" | "variable_declaration" => {
                let inner = Walk {
                    depth: at.depth + 1,
                    ..at.clone()
                };
                self.walk_children(node, &inner);
                return;
            }
            "call_expression" => self.call(node, at),
            "new_expression" => self.construct(node, at),
            _ => {}
        }
        self.walk_children(node, &at.deeper());
    }

    fn walk_children(&mut self, node: Node, at: &Walk) {
        let mut cursor = node.walk();
        let children: Vec<Node> = node.named_children(&mut cursor).collect();
        for child in children {
            self.walk(child, at);
        }
    }

    fn imports(&mut self, node: Node, at: &Walk) {
        let source = node
            .child_by_field_name("source")
            .map(|s| {
                self.text(s)
                    .trim_matches(|c| c == '\'' || c == '"')
                    .to_owned()
            })
            .unwrap_or_default();
        let scope = Walk {
            scope: vec![format!("import {source}")],
            ..at.clone()
        };
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            let mut cursor = current.walk();
            for child in current.named_children(&mut cursor) {
                match child.kind() {
                    "import_specifier" => {
                        if let Some(local) = child
                            .child_by_field_name("alias")
                            .or_else(|| child.child_by_field_name("name"))
                        {
                            let local = self.text(local);
                            self.symbol(&scope, &local, SymbolRole::Declaration, child);
                        }
                    }
                    "identifier" if current.kind() == "import_clause" => {
                        let local = self.text(child);
                        self.symbol(&scope, &local, SymbolRole::Declaration, child);
                    }
                    "namespace_import" => {
                        let mut inner = child.walk();
                        let names: Vec<Node> = child
                            .named_children(&mut inner)
                            .filter(|c| c.kind() == "identifier")
                            .collect();
                        for local in names {
                            let local = self.text(local);
                            self.symbol(&scope, &local, SymbolRole::Declaration, child);
                        }
                    }
                    "import_clause" | "named_imports" => stack.push(child),
                    _ => {}
                }
            }
        }
    }

    /// A function-like node: its identity, signature and symbol, then its body as a region.
    fn function(&mut self, node: Node, name: &str, kind: FunctionDeclarationKind, at: &Walk) {
        // A closure outside every function (a module-level callback, a class field initializer)
        // runs in the module region: it is scoped under `fn {module}` so its enclosing region is
        // that one.
        if kind == FunctionDeclarationKind::Closure && at.region.is_none() {
            let module = self.region(at);
            let inner = Walk {
                scope: vec!["fn {module}".to_owned()],
                region: Some(module),
                region_scope: vec!["fn {module}".to_owned()],
                class: None,
                exported: false,
                depth: at.depth,
            };
            return self.function(node, name, kind, &inner);
        }
        // A closure in a class body inside a region (a field initializer of a class expression)
        // belongs to that region, not to the class.
        if kind == FunctionDeclarationKind::Closure
            && !at.scope.last().is_some_and(|s| s.starts_with("fn "))
        {
            let inner = Walk {
                scope: at.region_scope.clone(),
                class: None,
                ..at.clone()
            };
            return self.function(node, name, kind, &inner);
        }
        let span = self.span(node);
        let owner = match (&at.class, kind) {
            (
                Some(class),
                FunctionDeclarationKind::InherentMethod
                | FunctionDeclarationKind::AssociatedFunction,
            ) => FunctionOwner {
                target: Some(self.type_identity(&at.scope[..at.scope.len() - 1], class, &span)),
                trait_path: None,
            },
            _ => FunctionOwner::none(),
        };
        let scope = SemanticScope::new(at.scope.clone());
        let symbol = SymbolIdentity {
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            scope: scope.clone(),
            name: name.to_owned(),
            role: SymbolRole::Definition,
            path: self.input.artifact_path.clone(),
            documentation: None,
        };
        let generics = node
            .child_by_field_name("type_parameters")
            .map(|t| vec![self.text(t)])
            .unwrap_or_default();
        let identity = FunctionIdentity {
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            language: self.input.language.clone(),
            scope: scope.clone(),
            symbol,
            span: span.clone(),
            generated: false,
            declaration_kind: kind,
            owner,
            generics: generics.clone(),
        };
        let id = SemanticRecordId::new(
            SemanticDimension::FunctionIdentity,
            &identity.identity_key(),
        );
        if kind != FunctionDeclarationKind::Closure {
            self.symbol(at, name, SymbolRole::Definition, node);
        }
        if at.region.is_none()
            && at.class.is_none()
            && kind == FunctionDeclarationKind::FreeFunction
        {
            self.module_functions.insert(name.to_owned(), id.clone());
        }
        if name == "constructor"
            && let Some(class) = &at.class
        {
            self.constructors.insert(class.clone(), id.clone());
        }
        let signature = self.signature(node, identity.clone(), generics, at, kind);
        self.emit_function(identity, signature);
        let mut scope = at.scope.clone();
        scope.push(format!("fn {name}"));
        let inner = Walk {
            region_scope: scope.clone(),
            scope,
            region: Some(id),
            class: at
                .class
                .clone()
                .filter(|_| kind != FunctionDeclarationKind::Closure),
            exported: false,
            depth: at.depth + 1,
        };
        // Parameters (default values may call) and the body.
        if let Some(parameters) = node.child_by_field_name("parameters") {
            self.walk_children(parameters, &inner);
        }
        if let Some(body) = node.child_by_field_name("body") {
            self.walk(body, &inner);
        }
    }

    fn type_identity(&self, scope: &[String], name: &str, span: &SourceSpan) -> TypeIdentity {
        TypeIdentity {
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            scope: SemanticScope::new(scope.to_vec()),
            name: name.to_owned(),
            canonical: None,
            path: span.path.clone(),
        }
    }

    fn signature(
        &self,
        node: Node,
        function: FunctionIdentity,
        generics: Vec<String>,
        at: &Walk,
        kind: FunctionDeclarationKind,
    ) -> FunctionSignature {
        let span = function.span.clone();
        let mut parameters = Vec::new();
        let parameter_nodes: Vec<Node> = match node.child_by_field_name("parameters") {
            Some(list) => {
                let mut cursor = list.walk();
                list.named_children(&mut cursor)
                    .filter(|c| c.kind() != "comment")
                    .collect()
            }
            None => node.child_by_field_name("parameter").into_iter().collect(),
        };
        for parameter in parameter_nodes {
            let pattern = parameter
                .child_by_field_name("pattern")
                .or_else(|| parameter.child_by_field_name("left"))
                .unwrap_or(parameter);
            let name = match pattern.kind() {
                "identifier" => self.text(pattern),
                "rest_pattern" => format!("...{}", self.text(pattern).trim_start_matches("...")),
                _ => "{pattern}".to_owned(),
            };
            let annotated = parameter
                .child_by_field_name("type")
                .map(|t| self.text(t).trim_start_matches(':').trim().to_owned())
                .unwrap_or_else(|| "(untyped)".to_owned());
            parameters.push(FunctionParameter {
                name,
                type_identity: self.type_identity(&at.scope, &annotated, &span),
            });
        }
        let return_type = node.child_by_field_name("return_type").map(|t| {
            let spelled = self.text(t).trim_start_matches(':').trim().to_owned();
            self.type_identity(&at.scope, &spelled, &span)
        });
        let (is_async, accessibility) = {
            let mut cursor = node.walk();
            let mut is_async = false;
            let mut accessibility = None;
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "async" => is_async = true,
                    "accessibility_modifier" => accessibility = Some(self.text(child)),
                    _ => {}
                }
            }
            (is_async, accessibility)
        };
        let visibility = match (kind, accessibility) {
            (_, Some(a)) => a,
            (FunctionDeclarationKind::Closure, None) => "local".to_owned(),
            (_, None) if at.class.is_some() => "public".to_owned(),
            (_, None) if at.exported => "export".to_owned(),
            (_, None) if at.region.is_none() => "module".to_owned(),
            (_, None) => "local".to_owned(),
        };
        let body_fingerprint = node.child_by_field_name("body").map(|body| {
            let mut tokens = Vec::new();
            self.tokens(body, &mut tokens, 0);
            atlas_core::IntegrityDigest::of_bytes(tokens.join(" ").as_bytes())
                .as_str()
                .to_owned()
        });
        FunctionSignature {
            function,
            parameters,
            return_type,
            generics,
            abi: None,
            visibility,
            is_async,
            is_unsafe: false,
            is_extern: false,
            body_fingerprint,
        }
    }

    /// The body's tokens without comments: a fingerprint that survives reformatting and moves.
    fn tokens(&self, node: Node, out: &mut Vec<String>, depth: usize) {
        if node.kind() == "comment" || depth > MAX_NESTING {
            return;
        }
        if node.child_count() == 0 {
            out.push(self.text(node));
            return;
        }
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();
        for child in children {
            self.tokens(child, out, depth + 1);
        }
    }

    /// The region calls at this point belong to: the enclosing function, or the generated
    /// `{module}` region for top-level code.
    fn region(&mut self, at: &Walk) -> SemanticRecordId {
        if let Some(region) = &at.region {
            return region.clone();
        }
        if let Some(region) = &self.module_region {
            return region.clone();
        }
        let span = SourceSpan {
            path: self.input.artifact_path.clone(),
            line: 1,
            column: 0,
        };
        let scope = SemanticScope::new(Vec::<String>::new());
        let identity = FunctionIdentity {
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            language: self.input.language.clone(),
            scope: scope.clone(),
            symbol: SymbolIdentity {
                repository: self.input.repository.clone(),
                revision: self.input.revision.clone(),
                scope,
                name: "{module}".to_owned(),
                role: SymbolRole::Definition,
                path: self.input.artifact_path.clone(),
                documentation: None,
            },
            span,
            generated: true,
            declaration_kind: FunctionDeclarationKind::FreeFunction,
            owner: FunctionOwner::none(),
            generics: Vec::new(),
        };
        let id = SemanticRecordId::new(
            SemanticDimension::FunctionIdentity,
            &identity.identity_key(),
        );
        let signature = FunctionSignature {
            function: identity.clone(),
            parameters: Vec::new(),
            return_type: None,
            generics: Vec::new(),
            abi: None,
            visibility: "module".to_owned(),
            is_async: false,
            is_unsafe: false,
            is_extern: false,
            body_fingerprint: None,
        };
        self.emit_function(identity, signature);
        self.module_region = Some(id.clone());
        id
    }

    fn call(&mut self, node: Node, at: &Walk) {
        let Some(callee) = node.child_by_field_name("function") else {
            return;
        };
        let (spelling, target) = match callee.kind() {
            "identifier" => {
                let name = self.text(callee);
                (name.clone(), Target::Function(name))
            }
            "member_expression" => match callee.child_by_field_name("property") {
                Some(p)
                    if matches!(
                        p.kind(),
                        "property_identifier" | "private_property_identifier"
                    ) =>
                {
                    (format!(".{}", self.text(p)), Target::None)
                }
                _ => (compact(&self.text(callee)), Target::None),
            },
            _ => (compact(&self.text(callee)), Target::None),
        };
        self.defer_call(node, at, spelling, target);
    }

    fn construct(&mut self, node: Node, at: &Walk) {
        let Some(class) = node.child_by_field_name("constructor") else {
            return;
        };
        let (spelling, target) = if class.kind() == "identifier" {
            let name = self.text(class);
            (format!("{name}::constructor"), Target::Constructor(name))
        } else {
            (compact(&self.text(class)), Target::None)
        };
        self.defer_call(node, at, spelling, target);
    }

    fn defer_call(&mut self, node: Node, at: &Walk, spelling: String, target: Target) {
        if !self.wants(SemanticDimension::Call) {
            return;
        }
        let caller = self.region(at);
        self.pending.push(PendingCall {
            caller,
            span: self.span(node),
            scope: at.scope.clone(),
            spelling,
            target,
        });
    }

    /// Emits every deferred call with the callee its lexical binding fixes, if any.
    fn resolve_calls(&mut self) {
        for call in std::mem::take(&mut self.pending) {
            let callees = match &call.target {
                Target::Function(name) => self.resolve_module_function(name),
                Target::Constructor(name) => match self.constructors.get(name) {
                    Some(id)
                        if self.bindings.get(name) == Some(&1) && !self.assigned.contains(name) =>
                    {
                        vec![id.clone()]
                    }
                    _ => Vec::new(),
                },
                Target::None => Vec::new(),
            };
            self.emit_call(call, callees);
        }
    }

    /// A bare identifier names a module-level function of this file when that name is bound
    /// exactly once in the file and never assigned: lexical scoping then fixes the callee.
    fn resolve_module_function(&self, name: &str) -> Vec<SemanticRecordId> {
        match self.module_functions.get(name) {
            Some(id) if self.bindings.get(name) == Some(&1) && !self.assigned.contains(name) => {
                vec![id.clone()]
            }
            _ => Vec::new(),
        }
    }

    fn emit_call(&mut self, call: PendingCall, callees: Vec<SemanticRecordId>) {
        let dimension = SemanticDimension::Call;
        let PendingCall {
            caller,
            span,
            scope,
            spelling,
            ..
        } = call;
        let resolved = !callees.is_empty();
        let subject = CallSiteIdentity {
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            function: caller,
            span: span.clone(),
            dispatch: if resolved {
                CallDispatchKind::StaticResolved
            } else {
                CallDispatchKind::Unresolved
            },
            callees,
            arguments: Vec::new(),
            result: PlaceRef::Unresolved,
            callee_spelling: Some(spelling.clone()),
        };
        let record_id = SemanticRecordId::new(dimension, &subject.identity_key());
        let summary = if resolved {
            format!("resolved call to `{spelling}` by lexical scope")
        } else {
            format!("parsed call to `{spelling}`")
        };
        let status = if resolved {
            EpistemicStatus::Derived
        } else {
            EpistemicStatus::Observed
        };
        let scope = SemanticScope::new(scope);
        self.emit(
            dimension,
            record_id,
            &span,
            summary,
            status,
            scope,
            |header| SemanticObservation::Call(header.with(subject)),
        );
    }

    fn symbol(&mut self, at: &Walk, name: &str, role: SymbolRole, node: Node) {
        let dimension = SemanticDimension::Symbol;
        if !self.wants(dimension) {
            return;
        }
        let span = self.span(node);
        let scope = SemanticScope::new(at.scope.clone());
        let subject = SymbolIdentity {
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            scope: scope.clone(),
            name: name.to_owned(),
            role,
            path: self.input.artifact_path.clone(),
            documentation: None,
        };
        let record_id = SemanticRecordId::new(dimension, &subject.identity_key());
        let summary = format!("parsed {} `{name}`", role.as_str());
        self.emit(
            dimension,
            record_id,
            &span,
            summary,
            EpistemicStatus::Observed,
            scope,
            |header| SemanticObservation::Symbol(header.with(subject)),
        );
    }

    fn emit_function(&mut self, identity: FunctionIdentity, signature: FunctionSignature) {
        let span = identity.span.clone();
        let scope = identity.scope.clone();
        if self.wants(SemanticDimension::FunctionIdentity) {
            let record_id = SemanticRecordId::new(
                SemanticDimension::FunctionIdentity,
                &identity.identity_key(),
            );
            let summary = format!("parsed function identity `{}`", identity.symbol.name);
            self.emit(
                SemanticDimension::FunctionIdentity,
                record_id,
                &span,
                summary,
                EpistemicStatus::Observed,
                scope.clone(),
                |header| SemanticObservation::FunctionIdentity(Box::new(header.with(identity))),
            );
        }
        if self.wants(SemanticDimension::FunctionSignature) {
            let key = format!(
                "{}|signature|{}",
                signature.function.identity_key(),
                signature
                    .parameters
                    .iter()
                    .map(|p| format!("{}:{}", p.name, p.type_identity.name))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            let record_id = SemanticRecordId::new(SemanticDimension::FunctionSignature, &key);
            let summary = format!(
                "parsed function signature for `{}`",
                signature.function.symbol.name
            );
            self.emit(
                SemanticDimension::FunctionSignature,
                record_id,
                &span,
                summary,
                EpistemicStatus::Observed,
                scope,
                |header| SemanticObservation::FunctionSignature(Box::new(header.with(signature))),
            );
        }
    }

    /// Records one observation with its evidence, once per record id.
    #[allow(clippy::too_many_arguments)]
    fn emit(
        &mut self,
        dimension: SemanticDimension,
        record_id: SemanticRecordId,
        span: &SourceSpan,
        summary: String,
        status: EpistemicStatus,
        scope: SemanticScope,
        build: impl FnOnce(Header) -> SemanticObservation,
    ) {
        if !self.seen.insert(record_id.as_str().to_owned()) {
            return;
        }
        let evidence_id = EvidenceId::new(stable_id(
            "evidence",
            &format!(
                "{}:{}:{}",
                self.fingerprint,
                record_id.as_str(),
                dimension.as_str()
            ),
        ));
        self.evidence.push(Evidence {
            id: evidence_id.as_str().to_owned(),
            kind: "PARSER_OUTPUT".into(),
            path: self.input.artifact_path.clone(),
            summary: format!("{summary} at {}:{}:{}", span.path, span.line, span.column),
            revision: Some(self.input.revision.clone()),
        });
        let entry = self.records.entry(dimension).or_default();
        entry.0.push(record_id.clone());
        entry.1.push(evidence_id.clone());
        let header = Header {
            record_id,
            dimension,
            status,
            scope,
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            extractor: self.extractor.clone(),
            evidence_refs: vec![evidence_id],
            provenance: Provenance {
                source_path: self.input.artifact_path.clone(),
                source_revision: Some(self.input.revision.clone()),
                extractor: self.extractor.id.clone(),
                content_hash: None,
                span: Some(format!("{}:{}", span.line, span.column)),
            },
        };
        let observation = build(header);
        assert!(observation.is_dimension_consistent());
        self.observations.push(observation);
    }

    fn unsupported(&mut self, dimension: SemanticDimension) -> ObligationResult {
        let diagnostic = ExtractionDiagnostic::new(
            DiagnosticCode::UnsupportedSemanticDimension,
            Some(dimension),
            format!(
                "{TYPESCRIPT_SEMANTIC_EXTRACTOR_ID} does not extract {} for {} (bounded first profile)",
                dimension.as_str(),
                self.input.language
            ),
        );
        let obligation = ObligationResult::unsupported(dimension, diagnostic.id.clone());
        self.diagnostics.push(diagnostic);
        obligation
    }

    fn finish_success(mut self) -> ExtractionBatch {
        for (ids, refs) in self.records.values_mut() {
            ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            refs.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        }
        let mut obligations = Vec::new();
        for &dimension in &self.input.requested_dimensions.clone() {
            if !SUPPORTED_DIMENSIONS.contains(&dimension) {
                let obligation = self.unsupported(dimension);
                obligations.push(obligation);
                continue;
            }
            let records = self.records.remove(&dimension);
            let mut gaps: Vec<String> = self.partial.iter().cloned().collect();
            if dimension == SemanticDimension::Call {
                gaps.push(
                    "member, imported, callback, `this`, dynamic, JSX, accessor, decorator and \
                     tagged-template callees are not resolved"
                        .into(),
                );
            }
            if gaps.is_empty() {
                obligations.push(match records {
                    Some((ids, refs)) => ObligationResult::observed(dimension, ids, refs),
                    None => {
                        let evidence_id = EvidenceId::new(stable_id(
                            "evidence",
                            &format!(
                                "{}:{}:verified-absence",
                                self.fingerprint,
                                dimension.as_str()
                            ),
                        ));
                        self.evidence.push(Evidence {
                            id: evidence_id.as_str().to_owned(),
                            kind: "PARSER_OUTPUT".into(),
                            path: self.input.artifact_path.clone(),
                            summary: format!(
                                "error-free parse of {} found no {} declarations",
                                self.input.artifact_path,
                                dimension.as_str()
                            ),
                            revision: Some(self.input.revision.clone()),
                        });
                        ObligationResult::observed(dimension, Vec::new(), vec![evidence_id])
                    }
                });
                continue;
            }
            let diagnostic = ExtractionDiagnostic::new(
                DiagnosticCode::IncompleteAnalysis,
                Some(dimension),
                format!(
                    "{TYPESCRIPT_SEMANTIC_EXTRACTOR_ID} partial {} coverage for {}: {}",
                    dimension.as_str(),
                    self.input.artifact_path,
                    gaps.join("; ")
                ),
            );
            let diagnostic_id = diagnostic.id.clone();
            self.diagnostics.push(diagnostic);
            obligations.push(match records {
                Some((ids, refs)) => {
                    ObligationResult::unknown_with_observations(dimension, ids, refs, diagnostic_id)
                }
                None => ObligationResult::unknown(dimension, diagnostic_id),
            });
        }
        obligations.sort_by_key(|o| o.dimension.as_str());
        self.observations
            .sort_by(|a, b| a.record_id().as_str().cmp(b.record_id().as_str()));
        self.evidence.sort_by(|a, b| a.id.cmp(&b.id));
        ExtractionBatch {
            extractor: self.extractor,
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            artifact: self.input.artifact.clone(),
            input_fingerprint: self.fingerprint,
            observations: self.observations,
            evidence: self.evidence,
            obligations,
            diagnostics: self.diagnostics,
        }
    }

    fn finish_failure(mut self, code: DiagnosticCode, why: &str) -> ExtractionBatch {
        let diagnostic = ExtractionDiagnostic::new(
            code,
            None,
            format!("failed to parse {}: {why}", self.input.artifact_path),
        );
        let diagnostic_id = diagnostic.id.clone();
        self.diagnostics.push(diagnostic);
        let mut obligations = Vec::new();
        for &dimension in &self.input.requested_dimensions.clone() {
            if SUPPORTED_DIMENSIONS.contains(&dimension) {
                obligations.push(ObligationResult::unknown(dimension, diagnostic_id.clone()));
            } else {
                let obligation = self.unsupported(dimension);
                obligations.push(obligation);
            }
        }
        obligations.sort_by_key(|o| o.dimension.as_str());
        ExtractionBatch {
            extractor: self.extractor,
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            artifact: self.input.artifact.clone(),
            input_fingerprint: self.fingerprint,
            observations: self.observations,
            evidence: self.evidence,
            obligations,
            diagnostics: self.diagnostics,
        }
    }
}

/// What a call's spelling may be resolved to once the file's declarations are known.
enum Target {
    /// A bare identifier: a module-level function of this file.
    Function(String),
    /// `new Name(..)`: the constructor of a class of this file.
    Constructor(String),
    None,
}

struct PendingCall {
    caller: SemanticRecordId,
    span: SourceSpan,
    scope: Vec<String>,
    spelling: String,
    target: Target,
}

/// A record header without its subject yet.
struct Header {
    record_id: SemanticRecordId,
    dimension: SemanticDimension,
    status: EpistemicStatus,
    scope: SemanticScope,
    repository: atlas_core::RepositoryId,
    revision: atlas_core::RevisionRef,
    extractor: atlas_core::ExtractorIdentity,
    evidence_refs: Vec<EvidenceId>,
    provenance: Provenance,
}

impl Header {
    fn with<S>(self, subject: S) -> SemanticRecordHeader<S> {
        SemanticRecordHeader {
            record_id: self.record_id,
            dimension: self.dimension,
            status: self.status,
            subject,
            scope: self.scope,
            repository: self.repository,
            revision: self.revision,
            extractor: self.extractor,
            evidence_refs: self.evidence_refs,
            provenance: self.provenance,
        }
    }
}

/// A callee expression's spelling on one line, bounded.
fn compact(text: &str) -> String {
    let one_line: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    one_line.chars().take(80).collect()
}

#[cfg(test)]
mod tests;
