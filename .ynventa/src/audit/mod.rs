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
        if segs
            .iter()
            .any(|s| matches!(*s, "architecture" | "blueprints" | "decisions" | "adr"))
            || name == "architecture.md"
        {
            let refs = referenced_paths(text);
            let dead = refs.iter().filter(|r| !files.exists(r)).count() as u64;
            if !refs.is_empty() && dead * 2 > refs.len() as u64 {
                issues.push(Issue::StaleArchitecture(dead, refs.len() as u64));
            }
        }
        let ls = &line_sets[p];
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

/// Extracts a document's knowledge as facts: statements, definitions, decisions, and the
/// document's own digest (which later licenses its deletion).
pub fn extract(path: &str, text: &str, seq: u64) -> Vec<Fact> {
    let mut facts = Vec::new();
    let mut headings: Vec<String> = Vec::new();
    let mut in_code = false;
    let mut code = String::new();
    let decision = path.split('/').any(|s| matches!(s, "decisions" | "adr"));
    let stem = path
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .trim_end_matches(".md")
        .to_string();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let prov = format!("doc:{path}#L{}", i + 1);
        if line.starts_with("```") {
            if in_code {
                let k = normalize(&code);
                if !k.is_empty() {
                    facts.push(Fact::new(
                        FactKind::Statement,
                        "",
                        &k,
                        code.trim_end(),
                        &prov,
                        seq,
                    ));
                }
                code.clear();
            }
            in_code = !in_code;
            continue;
        }
        if in_code {
            code.push_str(raw);
            code.push('\n');
            continue;
        }
        if let Some(h) = line.strip_prefix('#') {
            let level = 1 + h.chars().take_while(|c| *c == '#').count();
            let title = h.trim_start_matches('#').trim().to_string();
            headings.truncate(level.saturating_sub(1));
            headings.push(title.clone());
            if decision && level == 1 {
                facts.push(Fact::new(
                    FactKind::Decision,
                    &stem,
                    "title",
                    &title,
                    &prov,
                    seq,
                ));
            }
            continue;
        }
        let body = line.trim_start_matches(['-', '*', '+', '>', ' ']).trim();
        if body.is_empty()
            || body
                .chars()
                .all(|c| matches!(c, '-' | '|' | ':' | ' ' | '='))
        {
            continue;
        }
        if decision {
            if let Some(st) = body
                .strip_prefix("Status:")
                .or_else(|| body.strip_prefix("**Status**:"))
            {
                facts.push(Fact::new(
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
            Some((term, definition)) => facts.push(Fact::new(
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
                    facts.push(Fact::new(FactKind::Statement, "", &k, body, &prov, seq));
                }
            }
        }
    }
    facts.push(Fact::new(
        FactKind::Document,
        path,
        "digest",
        &content_digest(text.as_bytes()),
        &format!("doc:{path}"),
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
}
