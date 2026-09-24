use crate::{
    identity::{escape_identity_field, stable_id},
    schema::{FileFact, SourceReport},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdlSource {
    pub path: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceSpan {
    pub path: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdlDiagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdlToken {
    pub kind: String,
    pub lexeme: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdlProgram {
    pub schema: String,
    pub version: u32,
    pub system: Option<String>,
    pub declarations: Vec<AdlDeclaration>,
    pub diagnostics: Vec<AdlDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "data")]
pub enum AdlDeclaration {
    Entity(EntityDecl),
    Relation(RelationDecl),
    Capability(CapabilityDecl),
    Binding(BindingDecl),
    Constraint(ConstraintDecl),
    Invariant(ConstraintDecl),
    Transform(TransformDecl),
    Materialization(MaterializationDecl),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntityDecl {
    pub entity_kind: String,
    pub name: String,
    pub attributes: BTreeMap<String, String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelationDecl {
    pub from: String,
    pub relation: String,
    pub to: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityDecl {
    pub name: String,
    pub input: Option<String>,
    pub output: Option<String>,
    pub attributes: BTreeMap<String, String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BindingDecl {
    pub name: String,
    pub consumer: String,
    pub provider: String,
    pub capability: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConstraintDecl {
    pub name: String,
    pub checks: Vec<ConstraintCheck>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind")]
pub enum ConstraintCheck {
    AttributeEquals {
        entity_kind: String,
        where_attr: Option<String>,
        where_value: Option<String>,
        require_attr: String,
        require_value: String,
    },
    MaterializationExists {
        target: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransformDecl {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MaterializationDecl {
    pub target: String,
    pub path: String,
    pub source_language: Option<String>,
    pub source_glob: Option<String>,
    pub test_glob: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtlasIr {
    pub schema: String,
    pub version: u32,
    pub system: Option<String>,
    pub declared: DeclaredGraph,
    pub diagnostics: Vec<AdlDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeclaredGraph {
    pub nodes: Vec<DeclaredNode>,
    pub edges: Vec<DeclaredEdge>,
    pub bindings: Vec<BindingDecl>,
    pub constraints: Vec<ConstraintDecl>,
    pub invariants: Vec<ConstraintDecl>,
    pub transforms: Vec<TransformDecl>,
    pub materializations: Vec<MaterializationDecl>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeclaredNode {
    pub id: String,
    pub name: String,
    pub node_kind: String,
    pub attributes: BTreeMap<String, String>,
    pub origin: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeclaredEdge {
    pub id: String,
    pub from: String,
    pub relation: String,
    pub to: String,
    pub origin: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConstraintResult {
    pub name: String,
    pub passed: bool,
    pub diagnostics: Vec<AdlDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeclaredObservedDelta {
    pub code: String,
    pub message: String,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdlCompileReport {
    pub schema: String,
    pub sources_total: usize,
    pub declared_nodes_total: usize,
    pub declared_edges_total: usize,
    pub materializations_total: usize,
    pub diagnostics: Vec<AdlDiagnostic>,
    pub constraint_results: Vec<ConstraintResult>,
    pub deltas: Vec<DeclaredObservedDelta>,
    pub ir: AtlasIr,
}

fn adl_diag(
    code: &str,
    message: impl Into<String>,
    path: &str,
    line: usize,
    column: usize,
) -> AdlDiagnostic {
    AdlDiagnostic {
        code: code.into(),
        severity: "error".into(),
        message: message.into(),
        span: SourceSpan {
            path: path.into(),
            line,
            column,
        },
    }
}

pub fn lex_adl(path: &str, text: &str) -> Vec<AdlToken> {
    let mut tokens = Vec::new();
    for (line_idx, raw_line) in text.lines().enumerate() {
        let line_no = line_idx + 1;
        let mut col = 1;
        let mut chars = raw_line.chars().peekable();
        while let Some(ch) = chars.peek().copied() {
            if ch == '#' {
                break;
            }
            if ch.is_whitespace() {
                chars.next();
                col += 1;
                continue;
            }
            let start_col = col;
            if ch == '"' {
                chars.next();
                col += 1;
                let mut value = String::new();
                for next in chars.by_ref() {
                    col += 1;
                    if next == '"' {
                        break;
                    }
                    value.push(next);
                }
                tokens.push(AdlToken {
                    kind: "string".into(),
                    lexeme: value,
                    span: SourceSpan {
                        path: path.into(),
                        line: line_no,
                        column: start_col,
                    },
                });
                continue;
            }
            if matches!(ch, '{' | '}' | '=' | ':' | '.') {
                chars.next();
                col += 1;
                tokens.push(AdlToken {
                    kind: ch.to_string(),
                    lexeme: ch.to_string(),
                    span: SourceSpan {
                        path: path.into(),
                        line: line_no,
                        column: start_col,
                    },
                });
                continue;
            }
            if ch == '-' {
                let mut value = String::new();
                while let Some(next) = chars.peek().copied() {
                    if next.is_whitespace() {
                        break;
                    }
                    value.push(next);
                    chars.next();
                    col += 1;
                }
                tokens.push(AdlToken {
                    kind: "arrow".into(),
                    lexeme: value,
                    span: SourceSpan {
                        path: path.into(),
                        line: line_no,
                        column: start_col,
                    },
                });
                continue;
            }
            let mut value = String::new();
            while let Some(next) = chars.peek().copied() {
                if next.is_whitespace() || matches!(next, '{' | '}' | '=' | ':' | '.') {
                    break;
                }
                value.push(next);
                chars.next();
                col += 1;
            }
            tokens.push(AdlToken {
                kind: "ident".into(),
                lexeme: value,
                span: SourceSpan {
                    path: path.into(),
                    line: line_no,
                    column: start_col,
                },
            });
        }
    }
    tokens
}

fn strip_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or_default().trim()
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').to_owned()
}

fn parse_assignments(lines: &[(usize, String)]) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    for (_, line) in lines {
        let clean = strip_comment(line);
        if clean.is_empty() || clean.ends_with('{') || clean == "}" {
            continue;
        }
        if let Some((key, value)) = clean.split_once('=') {
            attrs.insert(key.trim().into(), unquote(value));
        }
    }
    attrs
}

/// Splits `content` into whitespace-delimited tokens, treating a `"..."`-quoted span as one
/// token regardless of internal whitespace -- shared logic every `key = value`/`key = "value"`
/// attribute in this DSL relies on.
fn quote_aware_tokens(content: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in content.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            current.push(ch);
        } else if ch.is_whitespace() && !in_quotes {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Recovers zero or more `"key = value"` lines from `content`, a single-line block's inline text
/// (between its opening `{` and closing `}`, e.g. `path = "core" language = rust`). Every
/// attribute value in this DSL is exactly one whitespace-delimited token (a bareword identifier
/// or a `"..."`-quoted string), so a `(key, "=", value)` token triple, repeated, recovers every
/// assignment regardless of how many share the line -- the same shape `parse_assignments` already
/// expects one-per-line for a multi-line block body.
fn split_inline_assignments(content: &str) -> Vec<String> {
    let tokens = quote_aware_tokens(content);
    let mut assignments = Vec::new();
    let mut index = 0;
    while index + 2 < tokens.len() {
        if tokens[index + 1] == "=" {
            assignments.push(format!("{} = {}", tokens[index], tokens[index + 2]));
            index += 3;
        } else {
            index += 1;
        }
    }
    assignments
}

fn collect_block(lines: &[(usize, String)], start: usize) -> (Vec<(usize, String)>, usize) {
    let mut body = Vec::new();
    let mut depth = 0_i32;
    let mut index = start;
    while index < lines.len() {
        let (line_no, raw) = &lines[index];
        let clean = strip_comment(raw);
        if index == start {
            // A single-line block (`capability Foo { input = A }`, or `materialize Compiler {
            // path = "core" language = rust }`) carries its real content on the SAME line as its
            // opening (and possibly closing) brace -- legal-looking ADL syntax with no grammar
            // rule forbidding it. Extract whatever sits between the FIRST `{` and the LAST `}`
            // (or end of line, if the block continues past this one) as inline assignments,
            // instead of unconditionally discarding the header line the way this function used to
            // -- which silently dropped every field of a single-line declaration with zero
            // diagnostic.
            if let Some(open_pos) = clean.find('{') {
                let after_open = &clean[open_pos + 1..];
                let inline = match after_open.rfind('}') {
                    Some(close_pos) => &after_open[..close_pos],
                    None => after_open,
                };
                for assignment in split_inline_assignments(inline) {
                    body.push((*line_no, assignment));
                }
            }
        } else {
            body.push(lines[index].clone());
        }
        depth += clean.matches('{').count() as i32;
        depth -= clean.matches('}').count() as i32;
        index += 1;
        if depth <= 0 {
            break;
        }
    }
    (body, index)
}

fn parse_relation(line: &str, path: &str, line_no: usize) -> Result<RelationDecl, AdlDiagnostic> {
    let Some((from, rest)) = line.split_once("->") else {
        return Err(adl_diag(
            "ATLAS-E030",
            "invalid relation syntax",
            path,
            line_no,
            1,
        ));
    };
    let Some((relation, to)) = rest.split_once("->") else {
        return Err(adl_diag(
            "ATLAS-E030",
            "invalid relation syntax",
            path,
            line_no,
            1,
        ));
    };
    Ok(RelationDecl {
        from: from.trim().into(),
        relation: relation.trim().into(),
        to: to.trim().into(),
        span: SourceSpan {
            path: path.into(),
            line: line_no,
            column: 1,
        },
    })
}

/// Returns `Err(joined_body)` when the body does not match any recognized
/// constraint syntax (including an empty body), so the caller can diagnose
/// it instead of silently admitting a constraint/invariant that checks nothing.
fn parse_constraint_check(lines: &[(usize, String)]) -> Result<Vec<ConstraintCheck>, String> {
    let joined = lines
        .iter()
        .map(|(_, line)| strip_comment(line).trim_matches('}').trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let words = joined.split_whitespace().collect::<Vec<_>>();
    // Every keyword below is located by exact whole-word match against `words`, never a substring
    // search on `joined` itself: an ADL attribute name that merely CONTAINS a keyword as a
    // substring (`elsewhere`, `requires_review`, `required_by`) is a real, plausible declared
    // attribute name, not an adversarial input, and previously collided with `str::split("where")`/
    // `str::split("require")` -- corrupting or silently dropping the clause built around it.
    if let Some(index) = words
        .windows(2)
        .position(|pair| pair[0] == "require" && pair[1] == "materialized")
        && let Some(target) = words.get(index + 2)
    {
        return Ok(vec![ConstraintCheck::MaterializationExists {
            target: (*target).into(),
        }]);
    }
    let entity_kind = words
        .windows(3)
        .find(|window| window[0] == "forall" && window[1].ends_with(':'))
        .map(|window| window[2].to_owned());
    let where_index = words.iter().position(|word| *word == "where");
    let require_index = words.iter().position(|word| *word == "require");
    let where_pair = where_index.and_then(|start| {
        let end = require_index
            .filter(|&require_index| require_index > start)
            .unwrap_or(words.len());
        let clause = words[start + 1..end].join(" ");
        let (left, right) = clause.split_once("==")?;
        let attr = left.split('.').nth(1)?.trim().to_owned();
        Some((attr, unquote(right)))
    });
    let require_pair = require_index.and_then(|start| {
        let clause = words[start + 1..].join(" ");
        let (left, right) = clause.split_once("==")?;
        let attr = left.split('.').nth(1)?.trim().to_owned();
        Some((attr, unquote(right)))
    });
    match (entity_kind, require_pair) {
        (Some(entity_kind), Some((require_attr, require_value))) => {
            Ok(vec![ConstraintCheck::AttributeEquals {
                entity_kind,
                where_attr: where_pair.as_ref().map(|pair| pair.0.clone()),
                where_value: where_pair.map(|pair| pair.1),
                require_attr,
                require_value,
            }])
        }
        _ => Err(joined),
    }
}

pub fn parse_adl_source(source: &AdlSource) -> AdlProgram {
    let _tokens = lex_adl(&source.path, &source.text);
    let mut diagnostics = Vec::new();
    let mut version = 0;
    let mut system = None;
    let mut declarations = Vec::new();
    let lines = source
        .text
        .lines()
        .enumerate()
        .map(|(idx, line)| (idx + 1, line.to_owned()))
        .collect::<Vec<_>>();
    let mut index = 0;
    while index < lines.len() {
        let (line_no, raw) = &lines[index];
        let clean = strip_comment(raw);
        if clean.is_empty() {
            index += 1;
            continue;
        }
        let parts = clean.split_whitespace().collect::<Vec<_>>();
        match parts.as_slice() {
            ["atlas", value] => {
                version = value.parse::<u32>().unwrap_or(0);
                if version != 1 {
                    diagnostics.push(adl_diag(
                        "ATLAS-E001",
                        "only ADL version 1 is supported",
                        &source.path,
                        *line_no,
                        1,
                    ));
                }
                index += 1;
            }
            ["system", name] => {
                system = Some((*name).into());
                index += 1;
            }
            ["entity", entity_kind, name, ..] => {
                let (body, next) = collect_block(&lines, index);
                declarations.push(AdlDeclaration::Entity(EntityDecl {
                    entity_kind: (*entity_kind).into(),
                    name: (*name).into(),
                    attributes: parse_assignments(&body),
                    span: SourceSpan {
                        path: source.path.clone(),
                        line: *line_no,
                        column: 1,
                    },
                }));
                index = next;
            }
            ["capability", name, ..] => {
                let (body, next) = collect_block(&lines, index);
                let attrs = parse_assignments(&body);
                declarations.push(AdlDeclaration::Capability(CapabilityDecl {
                    name: (*name).into(),
                    input: attrs.get("input").cloned(),
                    output: attrs.get("output").cloned(),
                    attributes: attrs,
                    span: SourceSpan {
                        path: source.path.clone(),
                        line: *line_no,
                        column: 1,
                    },
                }));
                index = next;
            }
            ["binding", name, ..] => {
                let (body, next) = collect_block(&lines, index);
                let attrs = parse_assignments(&body);
                declarations.push(AdlDeclaration::Binding(BindingDecl {
                    name: (*name).into(),
                    consumer: attrs.get("consumer").cloned().unwrap_or_default(),
                    provider: attrs.get("provider").cloned().unwrap_or_default(),
                    capability: attrs.get("capability").cloned().unwrap_or_default(),
                    span: SourceSpan {
                        path: source.path.clone(),
                        line: *line_no,
                        column: 1,
                    },
                }));
                index = next;
            }
            ["constraint", name, ..] | ["invariant", name, ..] => {
                let is_invariant = parts[0] == "invariant";
                let (body, next) = collect_block(&lines, index);
                let checks = match parse_constraint_check(&body) {
                    Ok(checks) => checks,
                    Err(unrecognized) => {
                        let kind = if is_invariant {
                            "invariant"
                        } else {
                            "constraint"
                        };
                        let snippet = if unrecognized.is_empty() {
                            "<empty body>".to_string()
                        } else {
                            unrecognized
                        };
                        diagnostics.push(adl_diag(
                            "ATLAS-E052",
                            format!(
                                "{kind} `{name}` body did not match any recognized constraint syntax: `{snippet}`"
                            ),
                            &source.path,
                            *line_no,
                            1,
                        ));
                        Vec::new()
                    }
                };
                let decl = ConstraintDecl {
                    name: (*name).into(),
                    checks,
                    span: SourceSpan {
                        path: source.path.clone(),
                        line: *line_no,
                        column: 1,
                    },
                };
                if is_invariant {
                    declarations.push(AdlDeclaration::Invariant(decl));
                } else {
                    declarations.push(AdlDeclaration::Constraint(decl));
                }
                index = next;
            }
            ["transform", name, ..] => {
                let (_, next) = collect_block(&lines, index);
                declarations.push(AdlDeclaration::Transform(TransformDecl {
                    name: name.split('<').next().unwrap_or(name).into(),
                    span: SourceSpan {
                        path: source.path.clone(),
                        line: *line_no,
                        column: 1,
                    },
                }));
                index = next;
            }
            ["materialize", target, ..] => {
                let (body, next) = collect_block(&lines, index);
                let attrs = parse_assignments(&body);
                declarations.push(AdlDeclaration::Materialization(MaterializationDecl {
                    target: (*target).into(),
                    path: attrs.get("path").cloned().unwrap_or_default(),
                    source_language: attrs.get("language").cloned(),
                    source_glob: attrs.get("glob").cloned(),
                    test_glob: attrs.get("tests.glob").cloned(),
                    span: SourceSpan {
                        path: source.path.clone(),
                        line: *line_no,
                        column: 1,
                    },
                }));
                index = next;
            }
            _ if clean.contains("->") => match parse_relation(clean, &source.path, *line_no) {
                Ok(relation) => {
                    declarations.push(AdlDeclaration::Relation(relation));
                    index += 1;
                }
                Err(error) => {
                    diagnostics.push(error);
                    index += 1;
                }
            },
            _ => {
                diagnostics.push(adl_diag(
                    "ATLAS-E010",
                    format!("unrecognized ADL declaration `{clean}`"),
                    &source.path,
                    *line_no,
                    1,
                ));
                index += 1;
            }
        }
    }
    if version == 0 {
        diagnostics.push(adl_diag(
            "ATLAS-E000",
            "missing `atlas 1` language version",
            &source.path,
            1,
            1,
        ));
    }
    AdlProgram {
        schema: "atlas.adl.program.v1".into(),
        version,
        system,
        declarations,
        diagnostics,
    }
}

/// A `name` shared across an `entity`/`capability` declaration silently collided in the graph
/// layer before this check existed: `DeclaredNode.id` (`stable_id("declared-node", &name)`) and
/// `engineering_graph::declared_node_id` are BOTH keyed purely by `name`, and `ensure_node` never
/// overwrites a node that already has that id -- it silently keeps only the FIRST of two
/// same-named declarations and drops the second entirely from the canonical engineering graph, no
/// diagnostic anywhere. Before this fix, only entity-vs-entity duplicates were checked
/// (`ATLAS-E020`); an entity colliding with a capability's own name, or two capabilities sharing a
/// name, produced zero diagnostics while silently losing one declaration's own node_kind/attributes
/// from the graph. `names` (this function's own shared map, already populated by every
/// entity/capability regardless of kind) is exactly the right structure to check against -- it was
/// simply never consulted from the Capability arm.
fn diagnose_duplicate_name(
    diagnostics: &mut Vec<AdlDiagnostic>,
    names: &BTreeMap<String, SourceSpan>,
    name: &str,
    span: &SourceSpan,
) {
    if let Some(first) = names.get(name) {
        diagnostics.push(AdlDiagnostic {
            code: "ATLAS-E020".into(),
            severity: "error".into(),
            message: format!(
                "duplicate declared name `{name}` first declared at {}:{}",
                first.path, first.line
            ),
            span: span.clone(),
        });
    }
}

pub fn compile_adl(sources: &[AdlSource], observed: &SourceReport) -> AdlCompileReport {
    let mut diagnostics = Vec::new();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut bindings = Vec::new();
    let mut constraints = Vec::new();
    let mut invariants = Vec::new();
    let mut transforms = Vec::new();
    let mut materializations = Vec::new();
    let mut names: BTreeMap<String, SourceSpan> = BTreeMap::new();
    let mut system = None;
    let mut version = 1;

    for source in sources {
        let program = parse_adl_source(source);
        version = program.version;
        if system.is_none() {
            system = program.system.clone();
        }
        diagnostics.extend(program.diagnostics);
        for decl in program.declarations {
            match decl {
                AdlDeclaration::Entity(entity) => {
                    diagnose_duplicate_name(&mut diagnostics, &names, &entity.name, &entity.span);
                    names.insert(entity.name.clone(), entity.span.clone());
                    nodes.push(DeclaredNode {
                        id: stable_id("declared-node", &entity.name),
                        name: entity.name,
                        node_kind: entity.entity_kind,
                        attributes: entity.attributes,
                        origin: "declared".into(),
                        span: entity.span,
                    });
                }
                AdlDeclaration::Capability(capability) => {
                    diagnose_duplicate_name(
                        &mut diagnostics,
                        &names,
                        &capability.name,
                        &capability.span,
                    );
                    names.insert(capability.name.clone(), capability.span.clone());
                    let mut attributes = capability.attributes;
                    if let Some(input) = capability.input {
                        attributes.insert("input".into(), input);
                    }
                    if let Some(output) = capability.output {
                        attributes.insert("output".into(), output);
                    }
                    nodes.push(DeclaredNode {
                        id: stable_id("declared-node", &capability.name),
                        name: capability.name,
                        node_kind: "Capability".into(),
                        attributes,
                        origin: "declared".into(),
                        span: capability.span,
                    });
                }
                AdlDeclaration::Relation(relation) => edges.push(DeclaredEdge {
                    // `from`/`relation`/`to` are extracted from raw source text by
                    // `parse_relation`'s simple `split_once("->")`, not through a restrictive
                    // lexer -- any of the three can contain a literal `:`, so each is escaped
                    // before joining to keep two genuinely different relations from ever
                    // computing the same edge id.
                    id: stable_id(
                        "declared-edge",
                        &format!(
                            "{}:{}:{}",
                            escape_identity_field(&relation.from, ':'),
                            escape_identity_field(&relation.relation, ':'),
                            escape_identity_field(&relation.to, ':'),
                        ),
                    ),
                    from: relation.from,
                    relation: relation.relation,
                    to: relation.to,
                    origin: "declared".into(),
                    span: relation.span,
                }),
                AdlDeclaration::Binding(binding) => bindings.push(binding),
                AdlDeclaration::Constraint(constraint) => constraints.push(constraint),
                AdlDeclaration::Invariant(invariant) => invariants.push(invariant),
                AdlDeclaration::Transform(transform) => transforms.push(transform),
                AdlDeclaration::Materialization(materialization) => {
                    materializations.push(materialization)
                }
            }
        }
    }

    for edge in &edges {
        if !names.contains_key(&edge.from) {
            diagnostics.push(adl_diag(
                "ATLAS-E021",
                format!("unknown relation source `{}`", edge.from),
                &edge.span.path,
                edge.span.line,
                edge.span.column,
            ));
        }
        if !names.contains_key(&edge.to) {
            diagnostics.push(adl_diag(
                "ATLAS-E022",
                format!("unknown relation target `{}`", edge.to),
                &edge.span.path,
                edge.span.line,
                edge.span.column,
            ));
        }
    }
    for binding in &bindings {
        for (role, name) in [
            ("consumer", &binding.consumer),
            ("provider", &binding.provider),
            ("capability", &binding.capability),
        ] {
            if !names.contains_key(name) {
                diagnostics.push(adl_diag(
                    "ATLAS-E023",
                    format!("unknown binding {role} `{name}`"),
                    &binding.span.path,
                    binding.span.line,
                    binding.span.column,
                ));
            }
        }
    }

    let declared = DeclaredGraph {
        nodes,
        edges,
        bindings,
        constraints,
        invariants,
        transforms,
        materializations,
    };
    let mut constraint_results = evaluate_constraints(&declared);
    let deltas = compare_declared_observed(&declared, observed);
    // A materialization whose path is entirely missing already gets an ATLAS-E040 entry below;
    // skip the language-mismatch delta for the same subject so one root cause does not produce
    // two redundant failing constraint_results entries (the language check inherently also fails
    // when the whole path is absent, since no files at all match the declared prefix).
    let missing_materialization_subjects: std::collections::BTreeSet<&str> = deltas
        .iter()
        .filter(|delta| delta.code == "MISSING_MATERIALIZATION")
        .map(|delta| delta.subject.as_str())
        .collect();
    for delta in &deltas {
        let code = match delta.code.as_str() {
            "MISSING_MATERIALIZATION" => "ATLAS-E040",
            // Previously computed by `compare_declared_observed` and recorded only as an inert
            // `Diagnostic`-kind census fact -- never turned into a failing `ConstraintResult`, so
            // a declared materialization whose observed files existed but were the wrong language
            // never blocked `coding_admission` or `atlas-cli check`.
            "MATERIALIZATION_LANGUAGE_NOT_OBSERVED"
                if !missing_materialization_subjects.contains(delta.subject.as_str()) =>
            {
                "ATLAS-E054"
            }
            _ => continue,
        };
        constraint_results.push(ConstraintResult {
            name: format!("ObservedMaterialization:{}", delta.subject),
            passed: false,
            diagnostics: vec![adl_diag(code, &delta.message, ".atlas/declared", 1, 1)],
        });
    }
    let ir = AtlasIr {
        schema: "atlas.ir.v1".into(),
        version,
        system,
        declared,
        diagnostics,
    };
    AdlCompileReport {
        schema: "atlas.adl.compile-report.v1".into(),
        sources_total: sources.len(),
        declared_nodes_total: ir.declared.nodes.len(),
        declared_edges_total: ir.declared.edges.len(),
        materializations_total: ir.declared.materializations.len(),
        diagnostics: ir.diagnostics.clone(),
        constraint_results,
        deltas,
        ir,
    }
}

fn evaluate_constraints(declared: &DeclaredGraph) -> Vec<ConstraintResult> {
    declared
        .constraints
        .iter()
        .chain(declared.invariants.iter())
        .map(|constraint| {
            let mut diagnostics = Vec::new();
            if constraint.checks.is_empty() {
                // A required constraint/invariant that was never successfully
                // parsed into an evaluable check must never report as
                // trivially passed: `.atlas/contracts/ARCHITECTURAL-INTEGRITY.md`
                // requires UNKNOWN/INCOMPLETE, not PASS, when evaluation is
                // impossible. `ATLAS-E052` (if present) already explains why.
                diagnostics.push(adl_diag(
                    "ATLAS-E053",
                    format!(
                        "constraint `{}` has no evaluable checks and cannot be verified",
                        constraint.name
                    ),
                    &constraint.span.path,
                    constraint.span.line,
                    constraint.span.column,
                ));
            }
            for check in &constraint.checks {
                match check {
                    ConstraintCheck::AttributeEquals {
                        entity_kind,
                        where_attr,
                        where_value,
                        require_attr,
                        require_value,
                    } => {
                        for node in declared
                            .nodes
                            .iter()
                            .filter(|node| &node.node_kind == entity_kind)
                        {
                            if let (Some(attr), Some(value)) = (where_attr, where_value)
                                && node.attributes.get(attr) != Some(value)
                            {
                                continue;
                            }
                            if node.attributes.get(require_attr) != Some(require_value) {
                                diagnostics.push(adl_diag(
                                    "ATLAS-E050",
                                    format!(
                                        "constraint `{}` expected {}.{} == {}",
                                        constraint.name, node.name, require_attr, require_value
                                    ),
                                    &node.span.path,
                                    node.span.line,
                                    node.span.column,
                                ));
                            }
                        }
                    }
                    ConstraintCheck::MaterializationExists { target } => {
                        if !declared
                            .materializations
                            .iter()
                            .any(|materialization| &materialization.target == target)
                        {
                            diagnostics.push(adl_diag(
                                "ATLAS-E051",
                                format!("`{target}` has no materialization"),
                                &constraint.span.path,
                                constraint.span.line,
                                constraint.span.column,
                            ));
                        }
                    }
                }
            }
            ConstraintResult {
                name: constraint.name.clone(),
                passed: diagnostics.is_empty(),
                diagnostics,
            }
        })
        .collect()
}

fn compare_declared_observed(
    declared: &DeclaredGraph,
    observed: &SourceReport,
) -> Vec<DeclaredObservedDelta> {
    let mut deltas = Vec::new();
    for materialization in &declared.materializations {
        let prefix = materialization.path.trim_end_matches('/');
        let exists = observed
            .files
            .iter()
            .any(|file| observed_prefix_matches(file, prefix));
        if !exists {
            deltas.push(DeclaredObservedDelta {
                code: "MISSING_MATERIALIZATION".into(),
                message: format!(
                    "declared materialization `{}` points at missing observed path `{}`",
                    materialization.target, materialization.path
                ),
                subject: materialization.target.clone(),
            });
        }
        if let Some(language) = &materialization.source_language {
            let has_language = observed
                .files
                .iter()
                .any(|file| observed_prefix_matches(file, prefix) && &file.language == language);
            if !has_language {
                deltas.push(DeclaredObservedDelta {
                    code: "MATERIALIZATION_LANGUAGE_NOT_OBSERVED".into(),
                    message: format!(
                        "declared materialization `{}` expected language `{}` under `{}`",
                        materialization.target, language, materialization.path
                    ),
                    subject: materialization.target.clone(),
                });
            }
        }
    }
    deltas
}

fn observed_prefix_matches(file: &FileFact, prefix: &str) -> bool {
    file.path == prefix
        || file
            .path
            .strip_prefix(prefix)
            .is_some_and(|tail| tail.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_accepts_vertical_slice() {
        let source = AdlSource {
            path: ".atlas/declared/system.adl".into(),
            text: r#"atlas 1
system Example

entity Runtime Compiler {
    kind = backend
    language = rust
}

capability CompileGraph {
    input = AST
    output = SystemGraph
}

Compiler ->provides-> CompileGraph

binding CompilerBinding {
    consumer = Compiler
    provider = Compiler
    capability = CompileGraph
}

materialize Compiler {
    path = "core"
    language = rust
}

constraint BackendIsRust {
    forall x: Runtime
        where x.kind == backend

    require x.language == rust
}
"#
            .into(),
        };
        let program = parse_adl_source(&source);
        assert!(program.diagnostics.is_empty());
        assert_eq!(program.version, 1);
        assert_eq!(program.system.as_deref(), Some("Example"));
        assert_eq!(program.declarations.len(), 6);
    }

    #[test]
    fn a_single_line_capability_block_does_not_silently_drop_its_attribute() {
        // `collect_block` used to unconditionally skip the header line (`if index > start`), so a
        // block whose opening AND closing brace both sit on the same line as the keyword --
        // legal-looking ADL syntax with no grammar rule forbidding it -- had its entire content
        // silently discarded: `body` came back empty, with zero diagnostic anywhere.
        let source = AdlSource {
            path: ".atlas/declared/system.adl".into(),
            text: "atlas 1\nsystem Example\ncapability Foo { input = A }\n".into(),
        };
        let program = parse_adl_source(&source);
        assert!(program.diagnostics.is_empty());
        let capability = program
            .declarations
            .iter()
            .find_map(|decl| match decl {
                AdlDeclaration::Capability(c) if c.name == "Foo" => Some(c),
                _ => None,
            })
            .expect("capability declaration is recorded");
        assert_eq!(
            capability.input.as_deref(),
            Some("A"),
            "a single-line block's own attribute must not be silently dropped"
        );
    }

    #[test]
    fn a_single_line_materialize_block_with_multiple_attributes_keeps_every_one() {
        // Same defect, worse consequence: a one-line `materialize` block with TWO attributes on
        // it. Before the fix, `path` silently defaulted to "" (`unwrap_or_default()`), which can
        // flip a real `check`/`systemize` MISSING_MATERIALIZATION result based on a parser
        // artifact rather than the actual repository state.
        let source = AdlSource {
            path: ".atlas/declared/system.adl".into(),
            text: "atlas 1\nsystem Example\nmaterialize Compiler { path = \"core\" language = rust }\n"
                .into(),
        };
        let program = parse_adl_source(&source);
        assert!(program.diagnostics.is_empty());
        let materialization = program
            .declarations
            .iter()
            .find_map(|decl| match decl {
                AdlDeclaration::Materialization(m) if m.target == "Compiler" => Some(m),
                _ => None,
            })
            .expect("materialize declaration is recorded");
        assert_eq!(materialization.path, "core");
        assert_eq!(materialization.source_language.as_deref(), Some("rust"));
    }

    // `.atlas/evidence/verification/large-stack-worker-mitigates-recursion-dos-residual-risk.json`
    // named, as an explicitly open question for a future generation, whether the same
    // recursion-depth-driven stack-overflow DoS class `adapter::semantic::rust`'s own
    // `max_structural_recursion_risk`/`EXTRACTION_STACK_SIZE` mitigate for `syn::parse_file` also
    // applies to this ADL parser. Code review shows `collect_block` tracks nesting with a flat
    // `i32` depth counter incremented/decremented once per line via a substring count, never
    // recursing per brace or per declaration -- structurally unlike `syn`'s recursive-descent
    // `Expr`/`Item` parsing. This test confirms that empirically, not merely by inspection: 200,000
    // nested braces (two orders of magnitude past the deepest confirmed `syn` crash vector, 3,000
    // levels) on adjacent lines must not overflow the default test-thread stack or hang.
    #[test]
    fn parser_handles_adversarially_deep_brace_nesting_without_recursion_or_crash() {
        let opens = "{".repeat(200_000);
        let closes = "}".repeat(200_001);
        let source = AdlSource {
            path: "adversarial.adl".into(),
            text: format!(
                "atlas 1\nsystem Example\n\nentity Runtime Stress {{\n{opens}\n{closes}\n"
            ),
        };
        let program = parse_adl_source(&source);
        assert_eq!(program.version, 1);
        assert_eq!(program.system.as_deref(), Some("Example"));
    }

    fn empty_source_report() -> SourceReport {
        SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 0,
            languages: BTreeMap::new(),
            files: Vec::new(),
        }
    }

    // ATLAS-E020 previously fired only for entity-vs-entity duplicates. `names` (the shared map
    // both entity and capability declarations populate, and the same map edges/bindings resolve
    // against) makes every node-shaped declaration's name comparable regardless of kind -- the
    // check just was not consulted from the Capability arm. This matters beyond a missing
    // diagnostic: `DeclaredNode.id` and `engineering_graph::declared_node_id` are BOTH keyed purely
    // by `name`, and graph construction's `ensure_node` silently keeps only the first of two
    // same-named declarations -- an entity/capability name collision silently dropped one
    // declaration's own node_kind/attributes from the canonical engineering graph before this fix,
    // with zero diagnostic anywhere in the pipeline.

    #[test]
    fn duplicate_entity_names_are_still_diagnosed() {
        let source = AdlSource {
            path: "a.adl".into(),
            text: "atlas 1\nsystem X\nentity Runtime Foo {\n    kind = backend\n}\nentity Runtime Foo {\n    kind = frontend\n}\n".into(),
        };
        let report = compile_adl(&[source], &empty_source_report());
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "ATLAS-E020"),
            "a duplicate entity name must still be diagnosed exactly as before this generation"
        );
    }

    #[test]
    fn duplicate_capability_names_are_now_diagnosed() {
        let source = AdlSource {
            path: "a.adl".into(),
            text: "atlas 1\nsystem X\ncapability Foo {\n    input = A\n}\ncapability Foo {\n    input = B\n}\n".into(),
        };
        let report = compile_adl(&[source], &empty_source_report());
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "ATLAS-E020"),
            "two capabilities sharing a name must be diagnosed, not silently drop one from the \
             graph with no diagnostic at all"
        );
    }

    #[test]
    fn an_entity_and_capability_sharing_a_name_are_now_diagnosed() {
        let source = AdlSource {
            path: "a.adl".into(),
            text: "atlas 1\nsystem X\nentity Runtime Foo {\n    kind = backend\n}\ncapability Foo {\n    input = A\n}\n".into(),
        };
        let report = compile_adl(&[source], &empty_source_report());
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "ATLAS-E020"),
            "an entity and a capability sharing a name is exactly the cross-kind collision this \
             fix closes -- ATLAS-E020 previously only ever compared entities against entities"
        );
    }

    #[test]
    fn semantics_reports_unknown_relation_target() {
        let source = AdlSource {
            path: ".atlas/declared/broken.adl".into(),
            text: "atlas 1\nsystem Broken\nentity Runtime Compiler {}\nCompiler ->depends_on-> Missing\n"
                .into(),
        };
        let observed = SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 0,
            languages: BTreeMap::new(),
            files: Vec::new(),
        };
        let report = compile_adl(&[source], &observed);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "ATLAS-E022")
        );
    }

    #[test]
    fn two_genuinely_different_relations_never_collide_into_the_same_edge_id() {
        // `parse_relation` extracts `from`/`relation`/`to` from raw source text via simple
        // `split_once("->")`, not through a restrictive lexer -- any of the three can contain a
        // literal `:`. Two genuinely different relations, joined unescaped with `:` for the
        // edge's `stable_id`, could otherwise produce the identical id.
        let source = AdlSource {
            path: ".atlas/declared/broken.adl".into(),
            text: "atlas 1\nsystem Broken\nA ->r:B-> C\nA ->r-> B:C\n".into(),
        };
        let observed = SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 0,
            languages: BTreeMap::new(),
            files: Vec::new(),
        };
        let report = compile_adl(&[source], &observed);
        assert_eq!(
            report.ir.declared.edges.len(),
            2,
            "both relations must be recorded as distinct declared edges"
        );
        assert_ne!(
            report.ir.declared.edges[0].id, report.ir.declared.edges[1].id,
            "two genuinely different (from, relation, to) triples must never share an edge id"
        );
    }

    #[test]
    fn constraint_and_materialization_use_observed_graph() {
        let source = AdlSource {
            path: ".atlas/declared/system.adl".into(),
            text: r#"atlas 1
system Example
entity Runtime Compiler {
    kind = backend
    language = rust
}
materialize Compiler {
    path = "core"
    language = rust
}
constraint BackendIsRust {
    forall x: Runtime
        where x.kind == backend
    require x.language == rust
}
"#
            .into(),
        };
        let observed = SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 1,
            languages: BTreeMap::from([("rust".into(), 1)]),
            files: vec![FileFact {
                path: "core/src/lib.rs".into(),
                language: "rust".into(),
                bytes: 10,
            }],
        };
        let report = compile_adl(&[source], &observed);
        assert!(report.diagnostics.is_empty());
        assert!(report.deltas.is_empty());
        assert!(report.constraint_results.iter().all(|result| result.passed));
    }

    #[test]
    fn unrecognized_constraint_syntax_is_diagnosed_not_silently_admitted() {
        let source = AdlSource {
            path: ".atlas/declared/broken.adl".into(),
            text: "atlas 1\nsystem Broken\nconstraint Nonsense {\n    this is not valid constraint syntax\n}\n"
                .into(),
        };
        let program = parse_adl_source(&source);
        assert!(
            program
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "ATLAS-E052"),
            "unrecognized constraint body must be diagnosed: {:?}",
            program.diagnostics
        );
        let constraint = program
            .declarations
            .iter()
            .find_map(|decl| match decl {
                AdlDeclaration::Constraint(c) if c.name == "Nonsense" => Some(c),
                _ => None,
            })
            .expect("constraint declaration is still recorded for provenance");
        assert!(
            constraint.checks.is_empty(),
            "unrecognized syntax must not fabricate a check"
        );
    }

    #[test]
    fn a_require_clause_attribute_name_containing_require_as_a_substring_still_parses() {
        // `requires_review` is an ordinary, plausible declared attribute name that CONTAINS
        // "require" as a substring. A naive `joined.split("require")` splits inside it as well as
        // at the real `require` keyword, corrupting the clause and rejecting an otherwise valid
        // constraint as unrecognized syntax.
        let source = AdlSource {
            path: ".atlas/declared/substring.adl".into(),
            text: "atlas 1\nsystem Example\nconstraint C {\n    forall x: Runtime\n        \
                   where x.requires_review == true\n    require x.language == rust\n}\n"
                .into(),
        };
        let program = parse_adl_source(&source);
        assert!(
            !program
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "ATLAS-E052"),
            "a where-attribute merely containing the word `require` must not corrupt parsing: {:?}",
            program.diagnostics
        );
        let constraint = program
            .declarations
            .iter()
            .find_map(|decl| match decl {
                AdlDeclaration::Constraint(c) if c.name == "C" => Some(c),
                _ => None,
            })
            .expect("constraint declaration is recorded");
        assert_eq!(
            constraint.checks,
            vec![ConstraintCheck::AttributeEquals {
                entity_kind: "Runtime".into(),
                where_attr: Some("requires_review".into()),
                where_value: Some("true".into()),
                require_attr: "language".into(),
                require_value: "rust".into(),
            }]
        );
    }

    #[test]
    fn a_where_clause_attribute_name_containing_where_as_a_substring_is_not_silently_dropped() {
        // `elsewhere` is an ordinary, plausible declared attribute name that CONTAINS "where" as a
        // substring. A naive `joined.split("where")` matches inside it too, silently resolving the
        // where-filter to `None` -- with no diagnostic anywhere -- which corrupts the constraint's
        // real meaning from "Runtime nodes where elsewhere == true must have language == rust" to
        // "ALL Runtime nodes must have language == rust", a silent semantic change that can flip a
        // real `check`/`systemize` pass/fail outcome.
        let source = AdlSource {
            path: ".atlas/declared/substring.adl".into(),
            text: "atlas 1\nsystem Example\nconstraint C {\n    forall x: Runtime\n        \
                   where x.elsewhere == true\n    require x.language == rust\n}\n"
                .into(),
        };
        let program = parse_adl_source(&source);
        let constraint = program
            .declarations
            .iter()
            .find_map(|decl| match decl {
                AdlDeclaration::Constraint(c) if c.name == "C" => Some(c),
                _ => None,
            })
            .expect("constraint declaration is recorded");
        assert_eq!(
            constraint.checks,
            vec![ConstraintCheck::AttributeEquals {
                entity_kind: "Runtime".into(),
                where_attr: Some("elsewhere".into()),
                where_value: Some("true".into()),
                require_attr: "language".into(),
                require_value: "rust".into(),
            }],
            "the where-filter naming an attribute that merely contains the word `where` must be \
             preserved, not silently dropped"
        );
    }

    #[test]
    fn unrecognized_constraint_syntax_never_reports_as_passed() {
        let source = AdlSource {
            path: ".atlas/declared/broken.adl".into(),
            text: "atlas 1\nsystem Broken\nconstraint Nonsense {\n    this is not valid constraint syntax\n}\n"
                .into(),
        };
        let observed = SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 0,
            languages: BTreeMap::new(),
            files: Vec::new(),
        };
        let report = compile_adl(&[source], &observed);
        let result = report
            .constraint_results
            .iter()
            .find(|result| result.name == "Nonsense")
            .expect("constraint result is still reported even when unevaluable");
        assert!(
            !result.passed,
            "a constraint that was never successfully parsed must not report passed:true"
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "ATLAS-E053")
        );
        assert!(
            !report.diagnostics.is_empty(),
            "top-level diagnostics must also flag it"
        );
    }

    #[test]
    fn empty_constraint_body_is_diagnosed_not_treated_as_vacuously_true() {
        let source = AdlSource {
            path: ".atlas/declared/empty.adl".into(),
            text: "atlas 1\nsystem Empty\nconstraint DoesNothing {\n}\n".into(),
        };
        let program = parse_adl_source(&source);
        assert!(
            program
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "ATLAS-E052")
        );
    }

    #[test]
    fn invariants_are_evaluated_just_like_constraints_not_silently_skipped() {
        let source = AdlSource {
            path: ".atlas/declared/system.adl".into(),
            text: r#"atlas 1
system Example
entity Runtime Compiler {
    kind = backend
    language = python
}
invariant BackendMustBeRust {
    forall x: Runtime
        where x.kind == backend
    require x.language == rust
}
"#
            .into(),
        };
        let observed = SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 0,
            languages: BTreeMap::new(),
            files: Vec::new(),
        };
        let report = compile_adl(&[source], &observed);
        assert_eq!(
            report.ir.declared.invariants.len(),
            1,
            "invariant declaration must still be recorded"
        );
        let result = report
            .constraint_results
            .iter()
            .find(|result| result.name == "BackendMustBeRust")
            .expect("invariant must produce a constraint_results entry, not be silently skipped");
        assert!(
            !result.passed,
            "a genuinely violated invariant (language=python, required rust) must be reported as failed"
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "ATLAS-E050")
        );
    }

    #[test]
    fn a_satisfied_invariant_passes_exactly_like_a_satisfied_constraint() {
        let source = AdlSource {
            path: ".atlas/declared/system.adl".into(),
            text: r#"atlas 1
system Example
entity Runtime Compiler {
    kind = backend
    language = rust
}
invariant BackendMustBeRust {
    forall x: Runtime
        where x.kind == backend
    require x.language == rust
}
"#
            .into(),
        };
        let observed = SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 0,
            languages: BTreeMap::new(),
            files: Vec::new(),
        };
        let report = compile_adl(&[source], &observed);
        assert!(report.diagnostics.is_empty());
        let result = report
            .constraint_results
            .iter()
            .find(|result| result.name == "BackendMustBeRust")
            .expect("invariant must produce a constraint_results entry");
        assert!(result.passed);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn materialization_language_mismatch_is_a_failing_constraint_result_not_only_an_inert_delta() {
        let source = AdlSource {
            path: ".atlas/declared/system.adl".into(),
            text: r#"atlas 1
system Example
materialize Compiler {
    path = "core"
    language = rust
}
"#
            .into(),
        };
        let observed = SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 1,
            languages: BTreeMap::from([("python".into(), 1)]),
            files: vec![FileFact {
                path: "core/src/lib.py".into(),
                language: "python".into(),
                bytes: 10,
            }],
        };
        let report = compile_adl(&[source], &observed);
        assert!(
            report
                .deltas
                .iter()
                .any(|delta| delta.code == "MATERIALIZATION_LANGUAGE_NOT_OBSERVED")
        );
        let result = report
            .constraint_results
            .iter()
            .find(|result| result.name == "ObservedMaterialization:Compiler")
            .expect(
                "a declared-but-wrong-language materialization must produce a failing \
                 constraint_results entry, not only an inert delta",
            );
        assert!(!result.passed);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "ATLAS-E054")
        );
    }

    #[test]
    fn a_wholly_missing_materialization_path_produces_exactly_one_failing_result_not_two() {
        let source = AdlSource {
            path: ".atlas/declared/system.adl".into(),
            text: r#"atlas 1
system Example
materialize Compiler {
    path = "core"
    language = rust
}
"#
            .into(),
        };
        let observed = SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 0,
            languages: BTreeMap::new(),
            files: Vec::new(),
        };
        let report = compile_adl(&[source], &observed);
        let matching = report
            .constraint_results
            .iter()
            .filter(|result| result.name == "ObservedMaterialization:Compiler")
            .collect::<Vec<_>>();
        assert_eq!(
            matching.len(),
            1,
            "MISSING_MATERIALIZATION and the redundant MATERIALIZATION_LANGUAGE_NOT_OBSERVED \
             delta for the same subject must not both become separate failing results"
        );
        assert!(!matching[0].passed);
        assert!(
            matching[0]
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "ATLAS-E040")
        );
    }
}
