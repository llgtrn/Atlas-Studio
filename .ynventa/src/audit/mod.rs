//! The document budget. Markdown is never a source of truth; a repository keeps only its human
//! entry points. Everything else is detected, classified, extracted into compacted knowledge
//! facts, and only then deleted — never before its knowledge is preserved.

use crate::compact::facts::{normalize, Fact, Knowledge};
use crate::declare::Declaration;
use crate::digest::content_digest;
use crate::repository::files::Files;
use crate::schema::{is_physical, FactKind};
use std::collections::{BTreeMap, BTreeSet};

/// Documents always within budget.
pub const ALLOWED_DOCUMENTS: &[&str] = &[
    "README.md",
    "AGENTS.md",
    "CLAUDE.md",
    "THIRD-PARTY-NOTICES.md",
    ".ynventa/README.md",
];

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Issue {
    OverBudget,
    GeneratedCommitted,
    SessionLog,
    Duplicate(String),
    NearDuplicate(String, u64),
    Superseded,
    DonorNote,
    CensusDescription,
    StaleArchitecture(u64, u64),
    Represented(u64),
}

impl Issue {
    pub fn wire(&self) -> String {
        match self {
            Issue::OverBudget => "OVER_BUDGET".into(),
            Issue::GeneratedCommitted => "GENERATED_REPORT_COMMITTED".into(),
            Issue::SessionLog => "SESSION_OR_PROGRESS_LOG".into(),
            Issue::Duplicate(o) => format!("DUPLICATE_OF {o}"),
            Issue::NearDuplicate(o, p) => format!("NEAR_DUPLICATE_OF {o} ({p}% shared lines)"),
            Issue::Superseded => "SUPERSEDED_DOCUMENT".into(),
            Issue::DonorNote => "DONOR_NOTE".into(),
            Issue::CensusDescription => "CENSUS_DESCRIPTION".into(),
            Issue::StaleArchitecture(dead, all) => {
                format!("STALE_ARCHITECTURE ({dead}/{all} referenced paths missing)")
            }
            Issue::Represented(p) => {
                format!("REPRESENTED_CANONICALLY ({p}% of statements are knowledge facts)")
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct DocReport {
    pub path: String,
    pub allowed: bool,
    pub issues: Vec<Issue>,
    pub digest: String,
    /// Knowledge was extracted from exactly these bytes; the document may be deleted.
    pub extracted: bool,
}

#[derive(Clone, Debug, Default)]
pub struct DocAudit {
    pub docs: Vec<DocReport>,
}

impl DocAudit {
    pub fn over_budget(&self) -> impl Iterator<Item = &DocReport> {
        self.docs.iter().filter(|d| !d.allowed)
    }
}

pub fn is_document(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".md") || lower.ends_with(".markdown") || lower.ends_with(".mdx")
}

/// Directories whose documents are decision records (compared case-insensitively).
pub const DECISION_RECORD_DIRS: &[&str] = &["decisions", "decision-records", "adr", "adrs"];

/// Index documents of a decision directory: living navigation, not decision records.
pub const DECISION_INDEX_NAMES: &[&str] = &["readme.md", "index.md", "_index.md"];

/// The one definition of a decision record (an ADR): a document with a directory segment in
/// [`DECISION_RECORD_DIRS`] (`.atlas/decisions/0015-x.md`, `docs/adr/0003-y.md`), other than
/// that directory's index. A decision record is history: it keeps the paths of its time, so
/// paths it names that were later moved are neither stale architecture nor stale references.
pub fn is_decision_record(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let Some((dir, name)) = lower.rsplit_once('/') else {
        return false;
    };
    is_document(&lower)
        && !DECISION_INDEX_NAMES.contains(&name)
        && dir.split('/').any(|s| DECISION_RECORD_DIRS.contains(&s))
}

fn allowed(path: &str, node_paths: &BTreeSet<String>) -> bool {
    if ALLOWED_DOCUMENTS.contains(&path) || path.starts_with(".github/") {
        return true;
    }
    match path.rsplit_once('/') {
        Some((dir, "README.md")) => node_paths.contains(dir),
        _ => false,
    }
}

/// Non-trivial normalised lines of a document.
fn lines(text: &str) -> BTreeSet<String> {
    text.lines()
        .map(normalize)
        .filter(|l| l.len() >= 12 && l.chars().any(|c| c.is_alphabetic()))
        .collect()
}

pub fn audit(files: &Files, d: &Declaration, knowledge: &Knowledge) -> DocAudit {
    let excluded = crate::census::excluded_roots(d);
    let donor_sources: Vec<String> = excluded
        .iter()
        .filter(|e| *e != ".atlas")
        .cloned()
        .collect();
    let node_paths: BTreeSet<String> = d
        .repository
        .nodes
        .iter()
        .filter(|n| is_physical(n.kind) && !n.path.is_empty())
        .map(|n| n.path.clone())
        .collect();
    let donor_words: BTreeSet<String> = d
        .donors
        .iter()
        .flat_map(|x| [x.key.to_ascii_lowercase(), x.name.to_ascii_lowercase()])
        .filter(|w| w.len() >= 3)
        .collect();
    let statements: BTreeSet<&str> = knowledge
        .facts
        .iter()
        .filter(|f| f.kind == FactKind::Statement)
        .map(|f| f.key.as_str())
        .collect();

    let docs: Vec<&String> = files
        .paths
        .iter()
        .filter(|p| is_document(p) && !crate::census::is_excluded(p, &donor_sources))
        .filter(|p| !p.split('/').any(|s| s == "node_modules"))
        .collect();
    let texts: BTreeMap<&String, String> = docs
        .iter()
        .map(|p| (*p, files.read(p).unwrap_or_default()))
        .collect();
    let line_sets: BTreeMap<&String, BTreeSet<String>> =
        texts.iter().map(|(p, t)| (*p, lines(t))).collect();

    // Exact duplicates by normalised content.
    let mut by_content: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    for (p, t) in &texts {
        by_content
            .entry(content_digest(normalize(t).as_bytes()))
            .or_default()
            .push(p);
    }
    // Near duplicates via an inverted index of lines (common boilerplate lines skipped).
    let mut postings: BTreeMap<&str, Vec<&String>> = BTreeMap::new();
    for (p, ls) in &line_sets {
        for l in ls {
            postings.entry(l.as_str()).or_default().push(p);
        }
    }
    let mut shared: BTreeMap<(&String, &String), u64> = BTreeMap::new();
    for ps in postings.values() {
        if ps.len() < 2 || ps.len() > 50 {
            continue;
        }
        for i in 0..ps.len() {
            for j in i + 1..ps.len() {
                *shared.entry((ps[i], ps[j])).or_default() += 1;
            }
        }
    }
    let mut near: BTreeMap<&String, (String, u64)> = BTreeMap::new();
    for ((a, b), n) in shared {
        let (la, lb) = (line_sets[a].len() as u64, line_sets[b].len() as u64);
        // The smaller document is the redundant one.
        let (small, big, ls) = if la <= lb { (a, b, la) } else { (b, a, lb) };
        if ls >= 3 {
            let pct = n * 100 / ls;
            if pct >= 80 && near.get(small).is_none_or(|(_, p)| pct > *p) {
                near.insert(small, (big.clone(), pct));
            }
        }
    }

    let mut out = DocAudit::default();
    for p in docs {
        let text = &texts[p];
        let digest = content_digest(text.as_bytes());
        let lower = p.to_ascii_lowercase();
        let segs: Vec<&str> = lower.split('/').collect();
        let name = segs.last().copied().unwrap_or("");
        let ok = allowed(p, &node_paths);
        let mut issues = Vec::new();
        if !ok {
            issues.push(Issue::OverBudget);
        }
        let head: String = text
            .lines()
            .take(8)
            .collect::<Vec<_>>()
            .join("\n")
            .to_ascii_lowercase();
        if head.contains("@generated")
            || head.contains("do not edit")
            || head.contains("generated by")
            || segs.contains(&"generated")
        {
            issues.push(Issue::GeneratedCommitted);
        }
        let dated = name.len() > 10
            && name[..10].bytes().enumerate().all(|(i, c)| {
                if i == 4 || i == 7 {
                    c == b'-'
                } else {
                    c.is_ascii_digit()
                }
            });
        if dated
            || [
                "session",
                "progress",
                "handoff",
                "status",
                "iteration",
                "journal",
                "worklog",
                "notes-",
                "standup",
            ]
            .iter()
            .any(|w| name.contains(w))
        {
            issues.push(Issue::SessionLog);
        }
        if let Some(group) = by_content.get(&content_digest(normalize(text).as_bytes())) {
            if group.len() > 1 && group[0] != p {
                issues.push(Issue::Duplicate(group[0].clone()));
            }
        }
        if let Some((other, pct)) = near.get(p) {
            issues.push(Issue::NearDuplicate(other.clone(), *pct));
        }
        let head20: String = text
            .lines()
            .take(20)
            .collect::<Vec<_>>()
            .join("\n")
            .to_ascii_lowercase();
        if head20.contains("superseded")
            || head20.contains("status: deprecated")
            || head20.contains("status: retired")
        {
            issues.push(Issue::Superseded);
        }
        if segs
            .iter()
            .any(|s| matches!(*s, "donors" | "provenance" | "licenses" | "license"))
            || donor_words
                .iter()
                .any(|w| name.trim_end_matches(".md").contains(w.as_str()))
        {
            issues.push(Issue::DonorNote);
        }
        if segs.contains(&"census") {
            issues.push(Issue::CensusDescription);
        }
        // Living architecture must name paths that exist; a decision record keeps its history.
        if (segs.iter().any(|s| {
            matches!(*s, "architecture" | "blueprints") || DECISION_RECORD_DIRS.contains(s)
        }) || name == "architecture.md")
            && !is_decision_record(p)
        {
            let refs = referenced_paths(text);
            let dead = refs.iter().filter(|r| !files.exists(r)).count() as u64;
            if !refs.is_empty() && dead * 2 > refs.len() as u64 {
                issues.push(Issue::StaleArchitecture(dead, refs.len() as u64));
            }
        }
        let ls = statement_units(text);
        if !ls.is_empty() && !statements.is_empty() {
            let represented = ls
                .iter()
                .filter(|l| statements.contains(l.as_str()))
                .count() as u64;
            let pct = represented * 100 / ls.len() as u64;
            if pct >= 90 {
                issues.push(Issue::Represented(pct));
            }
        }
        let extracted = knowledge.document_digest(p) == Some(digest.as_str());
        out.docs.push(DocReport {
            path: p.clone(),
            allowed: ok,
            issues,
            digest,
            extracted,
        });
    }
    out
}

/// Backticked repository-relative paths (`dir/file.ext`) mentioned by a document.
fn referenced_paths(text: &str) -> Vec<String> {
    let mut out = BTreeSet::new();
    for chunk in text.split('`').skip(1).step_by(2) {
        let c = chunk.trim().trim_end_matches('/');
        if c.contains('/')
            && !c.contains(' ')
            && !c.contains("://")
            && !c.starts_with('.')
            && !c.contains("::")
            && c.len() < 120
        {
            out.insert(c.to_string());
        }
    }
    out.into_iter().collect()
}

/// A logical block of a document: a heading, a fenced code block, a table row, or a paragraph
/// or list item together with its wrapped continuation lines. Lines are 1-based.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub first: usize,
    pub last: usize,
    pub kind: BlockKind,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Heading(usize),
    Code,
    Text,
}

/// Whether a line opens a list item: `- `, `* `, `+ ` or `1. ` / `1) `, after any quote marks.
fn opens_item(line: &str) -> bool {
    let l = line.trim_start_matches(['>', ' ']);
    if l.starts_with("- ") || l.starts_with("* ") || l.starts_with("+ ") || l == "-" {
        return true;
    }
    let digits = l.chars().take_while(char::is_ascii_digit).count();
    digits > 0 && (l[digits..].starts_with(". ") || l[digits..].starts_with(") "))
}

/// Whether a line is a `Field: value` line (`Status: Accepted`, `**Date**: …`), which starts
/// its own block even without a blank line before it.
fn opens_field(line: &str) -> bool {
    let l = line.trim_start_matches(['>', ' ', '*']);
    let Some((term, _)) = l.split_once(": ").or_else(|| l.split_once(":** ")) else {
        return false;
    };
    let term = term.trim_end_matches('*');
    let words = term.split_whitespace().count();
    (1..=3).contains(&words)
        && !term.contains('`')
        && term.chars().next().is_some_and(char::is_uppercase)
}

/// A rule or table separator: only `-`, `|`, `:`, `=` and spaces.
fn is_rule(line: &str) -> bool {
    !line.is_empty()
        && line
            .chars()
            .all(|c| matches!(c, '-' | '|' | ':' | ' ' | '=' | '*' | '_'))
        && line.chars().any(|c| c != ' ')
        && line.len() >= 3
}

/// Splits a document into logical blocks. A wrapped paragraph or list item is one block: its
/// continuation lines are joined with single spaces, so it is classified as one statement.
pub fn blocks(text: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut open: Option<Block> = None;
    let mut in_code = false;
    let mut code = String::new();
    let mut code_start = 0;
    for (i, raw) in text.lines().enumerate() {
        let n = i + 1;
        let line = raw.trim();
        if line.starts_with("```") {
            out.extend(open.take());
            if in_code {
                out.push(Block {
                    first: code_start,
                    last: n,
                    kind: BlockKind::Code,
                    text: std::mem::take(&mut code),
                });
            } else {
                code_start = n;
            }
            in_code = !in_code;
            continue;
        }
        if in_code {
            code.push_str(raw);
            code.push('\n');
            continue;
        }
        if line.is_empty() || is_rule(line) {
            out.extend(open.take());
            continue;
        }
        if let Some(h) = line.strip_prefix('#') {
            out.extend(open.take());
            let level = 1 + h.chars().take_while(|c| *c == '#').count();
            out.push(Block {
                first: n,
                last: n,
                kind: BlockKind::Heading(level),
                text: h.trim_start_matches('#').trim().to_string(),
            });
            continue;
        }
        let row = line.starts_with('|');
        match open.as_mut() {
            Some(b)
                if !row && !b.text.starts_with('|') && !opens_item(line) && !opens_field(line) =>
            {
                b.text.push(' ');
                b.text.push_str(line.trim_start_matches(['>', ' ']).trim());
                b.last = n;
            }
            _ => {
                out.extend(open.take());
                open = Some(Block {
                    first: n,
                    last: n,
                    kind: BlockKind::Text,
                    text: line.to_string(),
                });
            }
        }
    }
    out.extend(open);
    out
}

/// The text of a block without its list or quote marker.
fn block_body(b: &Block) -> &str {
    b.text.trim_start_matches(['-', '*', '+', '>', ' ']).trim()
}

/// Normalised statement keys of a document, as [`extract`] would key them.
fn statement_units(text: &str) -> BTreeSet<String> {
    blocks(text)
        .iter()
        .filter_map(|b| match b.kind {
            BlockKind::Heading(_) => None,
            BlockKind::Code => Some(normalize(&b.text)),
            BlockKind::Text => Some(normalize(block_body(b))),
        })
        .filter(|l| l.len() >= 12 && l.chars().any(|c| c.is_alphabetic()))
        .collect()
}

/// Where a block was found: `doc:<path>#L<a>[-L<b>]`, and with a commit also
/// `git:<sha>:<path>#L<a>[-L<b>]`.
fn provenance(path: &str, first: usize, last: usize, commit: Option<&str>) -> Vec<String> {
    let lines = if first == last {
        format!("#L{first}")
    } else {
        format!("#L{first}-L{last}")
    };
    let mut out = vec![format!("doc:{path}{lines}")];
    if let Some(c) = commit.filter(|c| !c.is_empty()) {
        out.push(format!("git:{c}:{path}{lines}"));
    }
    out
}

fn fact(kind: FactKind, subject: &str, key: &str, value: &str, prov: &[String], seq: u64) -> Fact {
    let mut f = Fact::new(kind, subject, key, value, &prov[0], seq);
    f.provenance.extend(prov.iter().cloned());
    f
}

/// Extracts a document's knowledge as facts: statements, definitions, decisions, and the
/// document's own digest (which later licenses its deletion).
pub fn extract(path: &str, text: &str, seq: u64) -> Vec<Fact> {
    extract_at(path, text, seq, None)
}

/// [`extract`], with provenance that also names the commit the document was read at.
pub fn extract_at(path: &str, text: &str, seq: u64, commit: Option<&str>) -> Vec<Fact> {
    let mut facts = Vec::new();
    let mut headings: Vec<String> = Vec::new();
    let decision = is_decision_record(path);
    let stem = path
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .trim_end_matches(".md")
        .to_string();
    for b in blocks(text) {
        let prov = provenance(path, b.first, b.last, commit);
        match b.kind {
            BlockKind::Code => {
                let k = normalize(&b.text);
                if !k.is_empty() {
                    facts.push(fact(
                        FactKind::Statement,
                        "",
                        &k,
                        b.text.trim_end(),
                        &prov,
                        seq,
                    ));
                }
            }
            BlockKind::Heading(level) => {
                headings.truncate(level.saturating_sub(1));
                headings.push(b.text.clone());
                if decision && level == 1 {
                    facts.push(fact(
                        FactKind::Decision,
                        &stem,
                        "title",
                        &b.text,
                        &prov,
                        seq,
                    ));
                }
            }
            BlockKind::Text => {
                let body = block_body(&b);
                if body.is_empty() || is_rule(body) {
                    continue;
                }
                if decision {
                    if let Some(st) = body
                        .strip_prefix("Status:")
                        .or_else(|| body.strip_prefix("**Status**:"))
                        .or_else(|| body.strip_prefix("Status**:"))
                    {
                        facts.push(fact(
                            FactKind::Decision,
                            &stem,
                            "status",
                            st.trim(),
                            &prov,
                            seq,
                        ));
                        continue;
                    }
                }
                let def = body.split_once(": ").filter(|(t, d)| {
                    !t.is_empty()
                        && t.split_whitespace().count() <= 5
                        && !d.trim().is_empty()
                        && !t.contains('`')
                });
                match def {
                    Some((term, definition)) => facts.push(fact(
                        FactKind::Definition,
                        &headings.join(" / "),
                        &normalize(term),
                        definition.trim(),
                        &prov,
                        seq,
                    )),
                    None => {
                        let k = normalize(body);
                        if !k.is_empty() {
                            facts.push(fact(FactKind::Statement, "", &k, body, &prov, seq));
                        }
                    }
                }
            }
        }
    }
    let mut whole = vec![format!("doc:{path}")];
    if let Some(c) = commit.filter(|c| !c.is_empty()) {
        whole.push(format!("git:{c}:{path}"));
    }
    facts.push(fact(
        FactKind::Document,
        path,
        "digest",
        &content_digest(text.as_bytes()),
        &whole,
        seq,
    ));
    facts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extraction_keeps_definitions_decisions_and_digest() {
        let text = "# Use native hashing\n\nStatus: Accepted\n\n- Census: a deterministic observation\n- The log is append-only.\n\n```\nfn x() {}\n```\n";
        let facts = extract(".atlas/decisions/0001-hash.md", text, 1);
        assert!(facts.iter().any(|f| f.kind == FactKind::Decision
            && f.key == "title"
            && f.value == "Use native hashing"));
        assert!(facts
            .iter()
            .any(|f| f.kind == FactKind::Decision && f.key == "status" && f.value == "Accepted"));
        assert!(facts
            .iter()
            .any(|f| f.kind == FactKind::Definition && f.key == "census"));
        assert!(facts
            .iter()
            .any(|f| f.kind == FactKind::Statement && f.value == "The log is append-only."));
        assert!(facts
            .iter()
            .any(|f| f.kind == FactKind::Statement && f.value == "fn x() {}"));
        assert!(facts
            .iter()
            .any(|f| f.kind == FactKind::Document && f.subject == ".atlas/decisions/0001-hash.md"));
    }
    #[test]
    fn wrapped_bullet_is_one_fact() {
        let text = "# Notes\n\n- The page store is append-only and every\n  record is addressed by its digest.\n- Census: a deterministic observation\n  of committed content.\n1. First numbered item that\n   wraps onto a second line.\n\nA plain paragraph that is\nwrapped by the editor\n> and quoted.\nStatus: Draft\nDate: 2026-10-01\n\n| a | b |\n|---|---|\n| c | d |\n";
        let facts = extract("docs/notes.md", text, 1);
        let statements: Vec<&str> = facts
            .iter()
            .filter(|f| f.kind == FactKind::Statement)
            .map(|f| f.value.as_str())
            .collect();
        assert!(statements.contains(
            &"The page store is append-only and every record is addressed by its digest."
        ));
        assert!(statements.contains(&"1. First numbered item that wraps onto a second line."));
        assert!(statements.contains(&"A plain paragraph that is wrapped by the editor and quoted."));
        assert!(
            !statements
                .iter()
                .any(|s| s.starts_with("record is addressed")),
            "no fragment of a wrapped bullet: {statements:?}"
        );
        let census = facts
            .iter()
            .find(|f| f.kind == FactKind::Definition && f.key == "census")
            .unwrap();
        assert_eq!(
            census.value,
            "a deterministic observation of committed content."
        );
        assert_eq!(
            census.provenance.iter().cloned().collect::<Vec<_>>(),
            vec!["doc:docs/notes.md#L5-L6".to_string()]
        );
        // Field lines start their own block; table rows stay separate.
        assert!(facts
            .iter()
            .any(|f| f.kind == FactKind::Definition && f.key == "status" && f.value == "Draft"));
        assert!(facts
            .iter()
            .any(|f| f.kind == FactKind::Definition && f.key == "date"));
        assert!(statements.contains(&"| a | b |") && statements.contains(&"| c | d |"));
    }

    #[test]
    fn extraction_provenance_names_the_commit() {
        let sha = "0123456789abcdef0123456789abcdef01234567";
        let facts = extract_at(
            ".atlas/decisions/0002-x.md",
            "# X\n\nStatus: Accepted\n\n- A statement that\n  wraps.\n",
            3,
            Some(sha),
        );
        let st = facts
            .iter()
            .find(|f| f.kind == FactKind::Statement)
            .unwrap();
        assert!(st
            .provenance
            .contains(&format!("git:{sha}:.atlas/decisions/0002-x.md#L5-L6")));
        assert!(st
            .provenance
            .contains("doc:.atlas/decisions/0002-x.md#L5-L6"));
        let status = facts.iter().find(|f| f.key == "status").unwrap();
        assert!(status
            .provenance
            .contains(&format!("git:{sha}:.atlas/decisions/0002-x.md#L3")));
        let doc = facts.iter().find(|f| f.kind == FactKind::Document).unwrap();
        assert!(doc
            .provenance
            .contains(&format!("git:{sha}:.atlas/decisions/0002-x.md")));
        // Without a commit, provenance is the document alone.
        assert!(extract(".atlas/decisions/0002-x.md", "- a statement\n", 1)
            .iter()
            .all(|f| f.provenance.iter().all(|p| p.starts_with("doc:"))));
    }
}
