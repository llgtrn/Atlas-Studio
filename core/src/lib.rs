//! Atlas core: product-neutral engineering semantics.
//!
//! This crate owns the vocabulary that every repository compilation converges on: facts,
//! nodes, edges, bindings, revisions, evidence and policy. It performs no filesystem, Git,
//! provider, donor or UI work.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const CLI_API: &str = "atlas.systemizer.cli.v1";
pub const BINARY: &str = "atlas-systemizer";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Contract {
    pub schema: String,
    pub binary: String,
    pub subsystem_kind: String,
    pub runtime_dependency_allowed: bool,
    pub commands: Vec<String>,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            schema: CLI_API.to_owned(),
            binary: BINARY.to_owned(),
            subsystem_kind: "SYSTEM_INVENTION_FORGE".to_owned(),
            runtime_dependency_allowed: false,
            commands: vec![
                "contract".into(),
                "systemize".into(),
                "docs audit".into(),
                "code analyze".into(),
                "work prepare".into(),
                "check".into(),
                "parse".into(),
                "graph".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevisionRef {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Provenance {
    pub source_path: String,
    pub source_revision: Option<RevisionRef>,
    pub extractor: String,
    pub content_hash: Option<String>,
    pub span: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Node {
    pub id: String,
    pub kind: String,
    pub identity: String,
    pub attributes: BTreeMap<String, String>,
    pub provenance: Provenance,
    pub revision: Option<RevisionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Edge {
    pub id: String,
    pub kind: String,
    pub from: String,
    pub to: String,
    pub attributes: BTreeMap<String, String>,
    pub provenance: Provenance,
    pub revision: Option<RevisionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Binding {
    pub id: String,
    pub source: String,
    pub target: String,
    pub binding_kind: String,
    pub confidence: f32,
    pub evidence: Vec<String>,
    pub revision: Option<RevisionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Fact {
    pub id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub provenance: Provenance,
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Evidence {
    pub id: String,
    pub kind: String,
    pub path: String,
    pub summary: String,
    pub revision: Option<RevisionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepositorySnapshot {
    pub schema: String,
    pub root: String,
    pub head_sha: String,
    pub branch: Option<String>,
    pub dirty: bool,
    pub status_entries: Vec<String>,
}

impl RepositorySnapshot {
    pub fn revision(&self) -> RevisionRef {
        RevisionRef {
            kind: "git".into(),
            value: self.head_sha.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineeringGraph {
    pub schema: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub bindings: Vec<Binding>,
    pub facts: Vec<Fact>,
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoManifest {
    pub schema: String,
    pub repo: String,
    pub system_kind: String,
    pub backend_language: String,
    pub frontend_language: String,
    pub coding_requires_docs_gate: bool,
    pub graph_before_code_required: bool,
    pub exact_base_sha_required: bool,
    pub single_repository_target_required: bool,
    pub knowledge_root: String,
    pub temporary_root: String,
    pub provenance_root: String,
    pub license_root: String,
    pub source_roots: Vec<String>,
    pub backend_roots: Vec<String>,
    pub frontend_roots: Vec<String>,
    pub test_roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileFact {
    pub path: String,
    pub language: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceReport {
    pub schema: String,
    pub root: String,
    pub files_total: usize,
    pub languages: BTreeMap<String, usize>,
    pub files: Vec<FileFact>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocsReport {
    pub schema: String,
    pub standard: String,
    pub root: String,
    pub gate_ready: bool,
    pub hard_violations_total: usize,
    pub documents_total: usize,
    pub canonical_frontmatter_total: usize,
    pub required_control_docs_missing: Vec<String>,
    pub missing_frontmatter: Vec<String>,
    pub documents: Vec<DocumentFact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentFact {
    pub path: String,
    pub id: Option<String>,
    pub kind: Option<String>,
    pub status: Option<String>,
    pub canonical: bool,
    pub title: Option<String>,
    pub headings: Vec<String>,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoAudit {
    pub schema: String,
    pub archetype: String,
    pub manifest: Option<RepoManifest>,
    pub manifest_ready: bool,
    pub policy_violations: Vec<String>,
    pub missing_required_roles: Vec<String>,
    pub missing_mapped_paths: Vec<String>,
    pub forbidden_roots_present: Vec<String>,
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphSummary {
    pub schema: String,
    pub semantic_grade: String,
    pub nodes_total: usize,
    pub edges_total: usize,
    pub bindings_total: usize,
    pub facts_total: usize,
    pub language_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodingAdmission {
    pub schema: String,
    pub allowed: bool,
    pub docs_standard: String,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkRequest {
    pub schema: String,
    pub repository: String,
    pub base_revision: RevisionRef,
    pub goal: String,
    pub scope: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub required_verification: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkPrepareReport {
    pub schema: String,
    pub request: WorkRequest,
    pub repository: RepoAudit,
    pub snapshot: RepositorySnapshot,
    pub graph: GraphSummary,
    pub coding_admission: CodingAdmission,
    pub allowed: bool,
    pub blockers: Vec<String>,
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemizeReport {
    pub schema: String,
    pub cli_api: String,
    pub root: String,
    pub snapshot: RepositorySnapshot,
    pub repository: RepoAudit,
    pub docs: DocsReport,
    pub adl: AdlCompileReport,
    pub coding_admission: CodingAdmission,
    pub source: SourceReport,
    pub graph: GraphSummary,
    pub invariants: Vec<String>,
}

pub fn stable_id(prefix: &str, identity: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in identity.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{prefix}:{hash:016x}")
}

pub fn provenance(path: impl Into<String>, extractor: impl Into<String>) -> Provenance {
    Provenance {
        source_path: path.into(),
        source_revision: None,
        extractor: extractor.into(),
        content_hash: None,
        span: None,
    }
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

fn collect_block(lines: &[(usize, String)], start: usize) -> (Vec<(usize, String)>, usize) {
    let mut body = Vec::new();
    let mut depth = 0_i32;
    let mut index = start;
    while index < lines.len() {
        let clean = strip_comment(&lines[index].1);
        depth += clean.matches('{').count() as i32;
        depth -= clean.matches('}').count() as i32;
        if index > start {
            body.push(lines[index].clone());
        }
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

fn parse_constraint_check(lines: &[(usize, String)]) -> Vec<ConstraintCheck> {
    let joined = lines
        .iter()
        .map(|(_, line)| strip_comment(line).trim_matches('}').trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if let Some(target) = joined
        .split("require materialized")
        .nth(1)
        .and_then(|tail| tail.split_whitespace().next())
    {
        return vec![ConstraintCheck::MaterializationExists {
            target: target.trim().into(),
        }];
    }
    let words = joined.split_whitespace().collect::<Vec<_>>();
    let entity_kind = words
        .windows(3)
        .find(|window| window[0] == "forall" && window[1].ends_with(':'))
        .map(|window| window[2].to_owned());
    let where_pair = joined.split("where").nth(1).and_then(|tail| {
        let before_require = tail.split("require").next()?.trim();
        let (left, right) = before_require.split_once("==")?;
        let attr = left.split('.').nth(1)?.trim().to_owned();
        Some((attr, unquote(right)))
    });
    let require_pair = joined.split("require").nth(1).and_then(|tail| {
        let (left, right) = tail.split_once("==")?;
        let attr = left.split('.').nth(1)?.trim().to_owned();
        Some((attr, unquote(right)))
    });
    match (entity_kind, require_pair) {
        (Some(entity_kind), Some((require_attr, require_value))) => {
            vec![ConstraintCheck::AttributeEquals {
                entity_kind,
                where_attr: where_pair.as_ref().map(|pair| pair.0.clone()),
                where_value: where_pair.map(|pair| pair.1),
                require_attr,
                require_value,
            }]
        }
        _ => Vec::new(),
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
                let decl = ConstraintDecl {
                    name: (*name).into(),
                    checks: parse_constraint_check(&body),
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
                    if let Some(first) = names.get(&entity.name) {
                        diagnostics.push(AdlDiagnostic {
                            code: "ATLAS-E020".into(),
                            severity: "error".into(),
                            message: format!(
                                "duplicate entity `{}` first declared at {}:{}",
                                entity.name, first.path, first.line
                            ),
                            span: entity.span.clone(),
                        });
                    }
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
                    id: stable_id(
                        "declared-edge",
                        &format!("{}:{}:{}", relation.from, relation.relation, relation.to),
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
    for delta in &deltas {
        if delta.code == "MISSING_MATERIALIZATION" {
            constraint_results.push(ConstraintResult {
                name: format!("ObservedMaterialization:{}", delta.subject),
                passed: false,
                diagnostics: vec![adl_diag(
                    "ATLAS-E040",
                    &delta.message,
                    ".atlas/declared",
                    1,
                    1,
                )],
            });
        }
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
        .map(|constraint| {
            let mut diagnostics = Vec::new();
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
        let exists = observed.files.iter().any(|file| {
            file.path == prefix
                || file
                    .path
                    .strip_prefix(prefix)
                    .is_some_and(|tail| tail.starts_with('/'))
        });
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
                .any(|file| file.path.starts_with(prefix) && &file.language == language);
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

pub fn validate_manifest(manifest: &RepoManifest) -> Vec<String> {
    let mut violations = Vec::new();
    let expected = [
        ("schema", &manifest.schema, "atlas.repo.v2"),
        (
            "system_kind",
            &manifest.system_kind,
            "SYSTEM_INVENTION_FORGE",
        ),
        ("backend_language", &manifest.backend_language, "rust"),
        (
            "frontend_language",
            &manifest.frontend_language,
            "typescript",
        ),
        ("knowledge_root", &manifest.knowledge_root, ".atlas"),
        (
            "temporary_root",
            &manifest.temporary_root,
            ".atlas/temporary",
        ),
        (
            "provenance_root",
            &manifest.provenance_root,
            ".atlas/provenance",
        ),
        ("license_root", &manifest.license_root, ".atlas/licenses"),
    ];
    for (field, actual, required) in expected {
        if actual != required {
            violations.push(format!("{field} must be {required}"));
        }
    }
    let required_true = [
        (
            "coding_requires_docs_gate",
            manifest.coding_requires_docs_gate,
        ),
        (
            "graph_before_code_required",
            manifest.graph_before_code_required,
        ),
        ("exact_base_sha_required", manifest.exact_base_sha_required),
        (
            "single_repository_target_required",
            manifest.single_repository_target_required,
        ),
    ];
    for (field, actual) in required_true {
        if !actual {
            violations.push(format!("{field} must be true"));
        }
    }
    violations
}

pub fn build_source_graph(source: &SourceReport) -> EngineeringGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut facts = Vec::new();
    let repo_id = stable_id("node", &source.root);
    nodes.push(Node {
        id: repo_id.clone(),
        kind: "Repository".into(),
        identity: source.root.clone(),
        attributes: BTreeMap::new(),
        provenance: provenance(".atlas/repo.toml", "atlas-core.graph-bootstrap"),
        revision: None,
    });
    let mut language_ids = BTreeMap::new();
    for language in source.languages.keys() {
        let id = stable_id("node", &format!("technology:{language}"));
        language_ids.insert(language.clone(), id.clone());
        nodes.push(Node {
            id,
            kind: "Technology".into(),
            identity: language.clone(),
            attributes: BTreeMap::from([("domain".into(), "language".into())]),
            provenance: provenance(".atlas/repo.toml", "atlas-core.graph-bootstrap"),
            revision: None,
        });
    }
    for file in &source.files {
        let file_id = stable_id("node", &format!("file:{}", file.path));
        nodes.push(Node {
            id: file_id.clone(),
            kind: "File".into(),
            identity: file.path.clone(),
            attributes: BTreeMap::from([
                ("language".into(), file.language.clone()),
                ("bytes".into(), file.bytes.to_string()),
            ]),
            provenance: provenance(&file.path, "atlas-core.graph-bootstrap"),
            revision: None,
        });
        edges.push(Edge {
            id: stable_id("edge", &format!("{repo_id}:CONTAINS:{file_id}")),
            kind: "CONTAINS".into(),
            from: repo_id.clone(),
            to: file_id.clone(),
            attributes: BTreeMap::new(),
            provenance: provenance(&file.path, "atlas-core.graph-bootstrap"),
            revision: None,
        });
        if let Some(language_id) = language_ids.get(&file.language) {
            edges.push(Edge {
                id: stable_id("edge", &format!("{file_id}:USES:{language_id}")),
                kind: "USES".into(),
                from: file_id.clone(),
                to: language_id.clone(),
                attributes: BTreeMap::new(),
                provenance: provenance(&file.path, "atlas-core.graph-bootstrap"),
                revision: None,
            });
        }
        facts.push(Fact {
            id: stable_id("fact", &format!("{}:language:{}", file.path, file.language)),
            subject: file_id,
            predicate: "language".into(),
            object: file.language.clone(),
            provenance: provenance(&file.path, "atlas-core.graph-bootstrap"),
            confidence: Some(1.0),
        });
    }
    EngineeringGraph {
        schema: "atlas.engineering-graph.v1".into(),
        nodes,
        edges,
        bindings: Vec::new(),
        facts,
        evidence: Vec::new(),
    }
}

pub fn build_repository_graph(source: &SourceReport, docs: &DocsReport) -> EngineeringGraph {
    let mut graph = build_source_graph(source);
    let mut node_ids_by_identity = BTreeMap::new();
    for node in &graph.nodes {
        node_ids_by_identity.insert(node.identity.clone(), node.id.clone());
    }

    for document in &docs.documents {
        let document_id = stable_id("node", &format!("document:{}", document.path));
        let mut attributes = BTreeMap::new();
        if let Some(id) = &document.id {
            attributes.insert("document_id".into(), id.clone());
        }
        if let Some(kind) = &document.kind {
            attributes.insert("document_type".into(), kind.clone());
        }
        if let Some(status) = &document.status {
            attributes.insert("status".into(), status.clone());
        }
        if let Some(title) = &document.title {
            attributes.insert("title".into(), title.clone());
        }
        attributes.insert("canonical".into(), document.canonical.to_string());
        graph.nodes.push(Node {
            id: document_id.clone(),
            kind: "Document".into(),
            identity: document.path.clone(),
            attributes,
            provenance: provenance(&document.path, "atlas-core.document-graph"),
            revision: None,
        });

        for heading in &document.headings {
            let section_id = stable_id(
                "node",
                &format!("document-section:{}:{heading}", document.path),
            );
            graph.nodes.push(Node {
                id: section_id.clone(),
                kind: "DocumentSection".into(),
                identity: format!("{}#{heading}", document.path),
                attributes: BTreeMap::from([("heading".into(), heading.clone())]),
                provenance: provenance(&document.path, "atlas-core.document-graph"),
                revision: None,
            });
            graph.edges.push(Edge {
                id: stable_id("edge", &format!("{document_id}:CONTAINS:{section_id}")),
                kind: "CONTAINS".into(),
                from: document_id.clone(),
                to: section_id,
                attributes: BTreeMap::new(),
                provenance: provenance(&document.path, "atlas-core.document-graph"),
                revision: None,
            });
        }

        for reference in &document.references {
            if let Some(target_id) = node_ids_by_identity.get(reference) {
                let edge_id = stable_id("edge", &format!("{document_id}:DOCUMENTS:{target_id}"));
                graph.edges.push(Edge {
                    id: edge_id,
                    kind: "DOCUMENTS".into(),
                    from: document_id.clone(),
                    to: target_id.clone(),
                    attributes: BTreeMap::from([("reference".into(), reference.clone())]),
                    provenance: provenance(&document.path, "atlas-core.document-graph"),
                    revision: None,
                });
                graph.bindings.push(Binding {
                    id: stable_id(
                        "binding",
                        &format!("{document_id}:DocumentationBinding:{target_id}"),
                    ),
                    source: document_id.clone(),
                    target: target_id.clone(),
                    binding_kind: "DocumentationBinding".into(),
                    confidence: 0.85,
                    evidence: vec![document.path.clone(), reference.clone()],
                    revision: None,
                });
            }
        }
    }

    graph
}

fn span_attributes(span: &SourceSpan) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("source_path".into(), span.path.clone()),
        ("source_line".into(), span.line.to_string()),
        ("source_column".into(), span.column.to_string()),
    ])
}

fn declared_node_id(name: &str) -> String {
    stable_id("node", &format!("declared:{name}"))
}

pub fn build_system_graph(
    source: &SourceReport,
    docs: &DocsReport,
    adl: &AdlCompileReport,
) -> EngineeringGraph {
    let mut graph = build_repository_graph(source, docs);
    let mut file_ids_by_path = BTreeMap::new();
    for node in &graph.nodes {
        if node.kind == "File" {
            file_ids_by_path.insert(node.identity.clone(), node.id.clone());
        }
    }

    for declared in &adl.ir.declared.nodes {
        let mut attributes = declared.attributes.clone();
        attributes.insert("origin".into(), declared.origin.clone());
        attributes.insert("declared_name".into(), declared.name.clone());
        attributes.extend(span_attributes(&declared.span));
        graph.nodes.push(Node {
            id: declared_node_id(&declared.name),
            kind: declared.node_kind.clone(),
            identity: declared.name.clone(),
            attributes,
            provenance: provenance(&declared.span.path, "atlas-core.adl-graph"),
            revision: None,
        });
    }

    for declared in &adl.ir.declared.edges {
        let from = declared_node_id(&declared.from);
        let to = declared_node_id(&declared.to);
        graph.edges.push(Edge {
            id: stable_id(
                "edge",
                &format!(
                    "declared:{}:{}:{}",
                    declared.from, declared.relation, declared.to
                ),
            ),
            kind: declared.relation.to_ascii_uppercase(),
            from,
            to,
            attributes: BTreeMap::from([
                ("origin".into(), declared.origin.clone()),
                ("declared_relation".into(), declared.relation.clone()),
            ]),
            provenance: provenance(&declared.span.path, "atlas-core.adl-graph"),
            revision: None,
        });
    }

    for binding in &adl.ir.declared.bindings {
        graph.bindings.push(Binding {
            id: stable_id("binding", &format!("adl-binding:{}", binding.name)),
            source: declared_node_id(&binding.consumer),
            target: declared_node_id(&binding.provider),
            binding_kind: "DeclaredCapabilityBinding".into(),
            confidence: 1.0,
            evidence: vec![
                binding.span.path.clone(),
                binding.consumer.clone(),
                binding.provider.clone(),
                binding.capability.clone(),
            ],
            revision: None,
        });
        graph.edges.push(Edge {
            id: stable_id(
                "edge",
                &format!("{}:BINDS_TO:{}", binding.consumer, binding.provider),
            ),
            kind: "BINDS_TO".into(),
            from: declared_node_id(&binding.consumer),
            to: declared_node_id(&binding.provider),
            attributes: BTreeMap::from([
                ("binding".into(), binding.name.clone()),
                ("capability".into(), binding.capability.clone()),
                ("origin".into(), "declared".into()),
            ]),
            provenance: provenance(&binding.span.path, "atlas-core.adl-graph"),
            revision: None,
        });
    }

    for materialization in &adl.ir.declared.materializations {
        let materialization_id = stable_id(
            "node",
            &format!(
                "materialization:{}:{}",
                materialization.target, materialization.path
            ),
        );
        let mut attributes = BTreeMap::from([
            ("target".into(), materialization.target.clone()),
            ("path".into(), materialization.path.clone()),
            ("origin".into(), "declared".into()),
        ]);
        if let Some(language) = &materialization.source_language {
            attributes.insert("source_language".into(), language.clone());
        }
        if let Some(glob) = &materialization.source_glob {
            attributes.insert("source_glob".into(), glob.clone());
        }
        attributes.extend(span_attributes(&materialization.span));
        graph.nodes.push(Node {
            id: materialization_id.clone(),
            kind: "Materialization".into(),
            identity: format!("{}@{}", materialization.target, materialization.path),
            attributes,
            provenance: provenance(&materialization.span.path, "atlas-core.adl-graph"),
            revision: None,
        });
        graph.edges.push(Edge {
            id: stable_id(
                "edge",
                &format!(
                    "{}:MATERIALIZES:{}",
                    materialization.target, materialization.path
                ),
            ),
            kind: "MATERIALIZES".into(),
            from: declared_node_id(&materialization.target),
            to: materialization_id.clone(),
            attributes: BTreeMap::from([("origin".into(), "declared".into())]),
            provenance: provenance(&materialization.span.path, "atlas-core.adl-graph"),
            revision: None,
        });
        let prefix = materialization.path.trim_end_matches('/');
        for (path, file_id) in &file_ids_by_path {
            if path == prefix
                || path
                    .strip_prefix(prefix)
                    .is_some_and(|tail| tail.starts_with('/'))
            {
                graph.edges.push(Edge {
                    id: stable_id(
                        "edge",
                        &format!("{materialization_id}:MATERIALIZES_AS:{file_id}"),
                    ),
                    kind: "MATERIALIZES_AS".into(),
                    from: materialization_id.clone(),
                    to: file_id.clone(),
                    attributes: BTreeMap::from([("path".into(), path.clone())]),
                    provenance: provenance(&materialization.span.path, "atlas-core.adl-graph"),
                    revision: None,
                });
                graph.bindings.push(Binding {
                    id: stable_id(
                        "binding",
                        &format!("{materialization_id}:MaterializationBinding:{file_id}"),
                    ),
                    source: materialization_id.clone(),
                    target: file_id.clone(),
                    binding_kind: "MaterializationBinding".into(),
                    confidence: 1.0,
                    evidence: vec![materialization.span.path.clone(), path.clone()],
                    revision: None,
                });
            }
        }
    }

    for constraint in &adl.ir.declared.constraints {
        let constraint_id = stable_id("node", &format!("constraint:{}", constraint.name));
        graph.nodes.push(Node {
            id: constraint_id.clone(),
            kind: "Constraint".into(),
            identity: constraint.name.clone(),
            attributes: span_attributes(&constraint.span),
            provenance: provenance(&constraint.span.path, "atlas-core.adl-graph"),
            revision: None,
        });
        graph.facts.push(Fact {
            id: stable_id("fact", &format!("constraint:{}:declared", constraint.name)),
            subject: constraint_id,
            predicate: "origin".into(),
            object: "declared".into(),
            provenance: provenance(&constraint.span.path, "atlas-core.adl-graph"),
            confidence: Some(1.0),
        });
    }

    for result in &adl.constraint_results {
        let result_id = stable_id("node", &format!("constraint-result:{}", result.name));
        graph.nodes.push(Node {
            id: result_id.clone(),
            kind: "Diagnostic".into(),
            identity: format!("constraint-result:{}", result.name),
            attributes: BTreeMap::from([
                ("constraint".into(), result.name.clone()),
                ("passed".into(), result.passed.to_string()),
                ("origin".into(), "derived".into()),
            ]),
            provenance: provenance(".atlas/declared", "atlas-core.adl-constraint-evaluator"),
            revision: None,
        });
        graph.facts.push(Fact {
            id: stable_id("fact", &format!("constraint-result:{}:passed", result.name)),
            subject: result_id,
            predicate: "passed".into(),
            object: result.passed.to_string(),
            provenance: provenance(".atlas/declared", "atlas-core.adl-constraint-evaluator"),
            confidence: Some(1.0),
        });
    }

    for delta in &adl.deltas {
        let delta_id = stable_id("node", &format!("delta:{}:{}", delta.code, delta.subject));
        graph.nodes.push(Node {
            id: delta_id.clone(),
            kind: "Diagnostic".into(),
            identity: format!("{}:{}", delta.code, delta.subject),
            attributes: BTreeMap::from([
                ("code".into(), delta.code.clone()),
                ("message".into(), delta.message.clone()),
                ("subject".into(), delta.subject.clone()),
                ("origin".into(), "derived".into()),
            ]),
            provenance: provenance(".atlas/declared", "atlas-core.adl-delta"),
            revision: None,
        });
        graph.facts.push(Fact {
            id: stable_id("fact", &format!("delta:{}:{}", delta.code, delta.subject)),
            subject: delta_id,
            predicate: "delta".into(),
            object: delta.code.clone(),
            provenance: provenance(".atlas/declared", "atlas-core.adl-delta"),
            confidence: Some(1.0),
        });
    }

    graph
}

pub fn summarize_graph(source: &SourceReport) -> GraphSummary {
    let graph = build_source_graph(source);
    summarize_engineering_graph(&graph, source)
}

pub fn summarize_repository_graph(source: &SourceReport, docs: &DocsReport) -> GraphSummary {
    let graph = build_repository_graph(source, docs);
    summarize_engineering_graph(&graph, source)
}

pub fn summarize_system_graph(
    source: &SourceReport,
    docs: &DocsReport,
    adl: &AdlCompileReport,
) -> GraphSummary {
    let graph = build_system_graph(source, docs, adl);
    let mut summary = summarize_engineering_graph(&graph, source);
    summary.semantic_grade = "ADL_DECLARED_OBSERVED_GRAPH".into();
    summary
}

fn summarize_engineering_graph(graph: &EngineeringGraph, source: &SourceReport) -> GraphSummary {
    let mut language_nodes: Vec<_> = source.languages.keys().cloned().collect();
    language_nodes.sort();
    GraphSummary {
        schema: "atlas.systemizer.engineering-graph-summary.v1".into(),
        semantic_grade: "SOURCE_FACT_GRAPH".into(),
        nodes_total: graph.nodes.len(),
        edges_total: graph.edges.len(),
        bindings_total: graph.bindings.len(),
        facts_total: graph.facts.len(),
        language_nodes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_fact_compiles_to_graph_primitives() {
        let source = SourceReport {
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
        let graph = build_source_graph(&source);
        assert!(graph.nodes.iter().any(|node| node.kind == "Repository"));
        assert!(graph.nodes.iter().any(|node| node.kind == "File"));
        assert!(graph.edges.iter().any(|edge| edge.kind == "CONTAINS"));
        assert!(graph.edges.iter().any(|edge| edge.kind == "USES"));
        assert_eq!(graph.facts.len(), 1);
    }

    #[test]
    fn adl_parser_accepts_vertical_slice() {
        let source = AdlSource {
            path: ".atlas/declared/system.atlas".into(),
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
    fn adl_semantics_reports_unknown_relation_target() {
        let source = AdlSource {
            path: ".atlas/declared/broken.atlas".into(),
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
    fn adl_constraint_and_materialization_use_observed_graph() {
        let source = AdlSource {
            path: ".atlas/declared/system.atlas".into(),
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
    fn adl_declarations_materialize_into_engineering_graph() {
        let source = SourceReport {
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
        let docs = DocsReport {
            schema: "test".into(),
            standard: "test".into(),
            root: "/repo/.atlas".into(),
            gate_ready: true,
            hard_violations_total: 0,
            documents_total: 0,
            canonical_frontmatter_total: 0,
            required_control_docs_missing: Vec::new(),
            missing_frontmatter: Vec::new(),
            documents: Vec::new(),
        };
        let adl_source = AdlSource {
            path: ".atlas/declared/system.atlas".into(),
            text: r#"atlas 1
system Example
entity Runtime Core {
    kind = backend
    language = rust
}
capability CompileGraph {
    input = AST
    output = SystemGraph
}
Core ->provides-> CompileGraph
binding CoreCompiler {
    consumer = Core
    provider = Core
    capability = CompileGraph
}
materialize Core {
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
        let adl = compile_adl(&[adl_source], &source);
        let graph = build_system_graph(&source, &docs, &adl);
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == "Runtime" && node.identity == "Core")
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == "Materialization" && node.identity == "Core@core")
        );
        assert!(graph.edges.iter().any(|edge| edge.kind == "PROVIDES"));
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.kind == "MATERIALIZES_AS")
        );
        assert!(
            graph
                .bindings
                .iter()
                .any(|binding| binding.binding_kind == "MaterializationBinding")
        );
        assert!(
            graph
                .facts
                .iter()
                .any(|fact| fact.predicate == "passed" && fact.object == "true")
        );
    }
}
