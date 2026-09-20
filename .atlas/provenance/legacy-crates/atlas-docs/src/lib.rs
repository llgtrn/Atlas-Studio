use atlas_model::DocsReport;
use std::{collections::BTreeMap, fs, io, path::{Path, PathBuf}};

const STANDARD: &str = "atlas.docs.v1";
const TYPES: &[&str] = &["decision","blueprint","architecture","contract","runbook","reference","generated-evidence"];
const REQUIRED: &[&str] = &["id","type","status","canonical"];
const FORBIDDEN_PLACEHOLDERS: &[&str] = &["TODO","TBD","PLACEHOLDER","FILL_ME"];
const REQUIRED_SECTION_MIN_CHARS: usize = 40;
const CONTROL_DOCS: &[&str] = &[
    "README.md",
    "INDEX.md",
    "TEMPLATE.md",
    "architecture/README.md",
    "architecture/constitution/NORTH-STAR.md",
    "architecture/SYSTEM.md",
    "blueprints/SYSTEM-BLUEPRINT.md",
    "contracts/SYSTEM-CONTRACT.md",
    "decisions/README.md",
    "guides/DEVELOPMENT.md",
    "references/README.md",
];

fn required_headings(path: &str) -> &'static [&'static str] {
    match path {
        "README.md" => &["## Purpose","## Start Here","## Documentation Gate"],
        "INDEX.md" => &["## Start Here","## Canonical Documents","## Coding Route"],
        "TEMPLATE.md" => &["## Required Frontmatter","## Required Control Documents","## Authoring Rules"],
        "architecture/README.md" => &["## System Model","## Responsibilities","## Boundaries","## Runtime Ownership","## State and Effects","## Dependencies"],
        "architecture/constitution/NORTH-STAR.md" => &["## Mission","## North Star","## Non-Negotiable Invariants","## Boundaries","## Sequencing","## Non-Goals"],
        "architecture/SYSTEM.md" => &["## System Model","## Responsibilities","## Boundaries","## Runtime Ownership","## Data and Effect Flow","## Failure and Recovery","## Evidence","## Verification"],
        "blueprints/SYSTEM-BLUEPRINT.md" => &["## Objective","## Inputs","## Flow","## Authority","## State","## Failure and Recovery","## Evidence","## Verification"],
        "contracts/SYSTEM-CONTRACT.md" => &["## Hard Invariants","## Interfaces","## State and Durability","## Authority","## Evidence","## Recovery","## Verification"],
        "decisions/README.md" => &["## Decision Rules","## Active Decisions","## Supersession"],
        "guides/DEVELOPMENT.md" => &["## Preconditions","## Documentation Gate","## Implementation","## Verification","## Reconciliation","## Rollback"],
        "references/README.md" => &["## Reference Rules","## Provenance","## Freshness"],
        _ => &[],
    }
}

fn section_body<'a>(text: &'a str, heading: &str) -> Option<&'a str> {
    let start = text.find(heading)? + heading.len();
    let tail = &text[start..];
    let end = tail.find("\n## ").unwrap_or(tail.len());
    Some(tail[..end].trim())
}

fn visit(dir: &Path, docs: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_dir() { visit(&path, docs)?; }
        else if path.extension().and_then(|x| x.to_str()) == Some("md") { docs.push(path); }
    }
    Ok(())
}

fn frontmatter(text: &str) -> Option<BTreeMap<String, String>> {
    if !text.starts_with("---\n") { return None; }
    let rest = &text[4..];
    let end = rest.find("\n---")?;
    let mut map = BTreeMap::new();
    for raw in rest[..end].lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some((key, value)) = line.split_once(':') {
            map.insert(key.trim().to_owned(), value.trim().trim_matches('"').to_owned());
        }
    }
    Some(map)
}

fn markdown_links(text: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = text[cursor..].find("](") {
        let start = cursor + offset + 2;
        let Some(end_rel) = text[start..].find(')') else { break };
        let end = start + end_rel;
        links.push(text[start..end].trim().trim_matches('<').trim_matches('>').to_owned());
        cursor = end + 1;
    }
    links
}

pub fn audit(root: impl AsRef<Path>) -> io::Result<DocsReport> {
    let root = root.as_ref().canonicalize()?;
    let mut docs = Vec::new();
    visit(&root, &mut docs)?;
    docs.sort();

    let mut required_control_docs_missing = Vec::new();
    let mut required_headings_missing = Vec::new();
    let mut insufficient_sections = Vec::new();
    let mut forbidden_placeholders = Vec::new();
    for relative in CONTROL_DOCS {
        let path = root.join(relative);
        if !path.is_file() {
            required_control_docs_missing.push((*relative).to_owned());
            continue;
        }
        let text = fs::read_to_string(&path)?;
        for heading in required_headings(relative) {
            if !text.lines().any(|line| line.trim() == *heading) {
                required_headings_missing.push(format!("{relative}:{heading}"));
                continue;
            }
            match section_body(&text, heading) {
                Some(body) if body.chars().filter(|c| !c.is_whitespace()).count() >= REQUIRED_SECTION_MIN_CHARS => {}
                _ => insufficient_sections.push(format!("{relative}:{heading}")),
            }
        }
        for token in FORBIDDEN_PLACEHOLDERS {
            if text.contains(token) {
                forbidden_placeholders.push(format!("{relative}:{token}"));
            }
        }
    }

    let mut canonical_frontmatter_total = 0;
    let mut missing_frontmatter = Vec::new();
    let mut missing_required_fields = Vec::new();
    let mut invalid_type = Vec::new();
    let mut superseded_without_successor = Vec::new();
    let mut broken_internal_refs = Vec::new();
    let mut ids: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for path in &docs {
        let text = fs::read_to_string(path)?;
        let rel = path.strip_prefix(&root).unwrap_or(path).to_string_lossy().replace('\\', "/");
        let Some(meta) = frontmatter(&text) else {
            missing_frontmatter.push(rel.clone());
            continue;
        };
        canonical_frontmatter_total += 1;
        for field in REQUIRED {
            if !meta.contains_key(*field) {
                missing_required_fields.push(format!("{rel}:{field}"));
            }
        }
        if let Some(kind) = meta.get("type") {
            if !TYPES.contains(&kind.as_str()) { invalid_type.push(rel.clone()); }
        }
        if let Some(id) = meta.get("id") {
            ids.entry(id.clone()).or_default().push(rel.clone());
        }
        if meta.get("status").map(String::as_str) == Some("superseded") && !meta.contains_key("superseded_by") {
            superseded_without_successor.push(rel.clone());
        }
        for raw_link in markdown_links(&text) {
            if raw_link.is_empty() || raw_link.starts_with('#') || raw_link.starts_with("http://")
                || raw_link.starts_with("https://") || raw_link.starts_with("mailto:") || raw_link.starts_with('/') {
                continue;
            }
            let target = raw_link.split('#').next().unwrap_or_default().split('?').next().unwrap_or_default();
            if target.is_empty() || !target.ends_with(".md") { continue; }
            let resolved = path.parent().unwrap_or(&root).join(target);
            if !resolved.exists() { broken_internal_refs.push(format!("{rel}->{raw_link}")); }
        }
    }

    let duplicate_ids = ids.into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .map(|(id, paths)| format!("{id}:{}", paths.join(",")))
        .collect::<Vec<_>>();

    let hard_violations_total =
        required_control_docs_missing.len()
        + required_headings_missing.len()
        + insufficient_sections.len()
        + forbidden_placeholders.len()
        + missing_frontmatter.len()
        + missing_required_fields.len()
        + invalid_type.len()
        + duplicate_ids.len()
        + superseded_without_successor.len()
        + broken_internal_refs.len();

    Ok(DocsReport {
        schema: "atlas.systemizer.docs-report.v1".into(),
        standard: STANDARD.into(),
        root: root.to_string_lossy().into_owned(),
        gate_ready: hard_violations_total == 0,
        hard_violations_total,
        documents_total: docs.len(),
        canonical_frontmatter_total,
        required_control_docs_missing,
        required_headings_missing,
        insufficient_sections,
        forbidden_placeholders,
        missing_frontmatter,
        missing_required_fields,
        invalid_type,
        duplicate_ids,
        superseded_without_successor,
        broken_internal_refs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn missing_control_docs_blocks_gate() {
        let root = env::temp_dir().join(format!("atlas-docs-gate-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("README.md"), "---\nid: x\ntype: reference\nstatus: active\ncanonical: true\n---\n# X\n").unwrap();
        let report = audit(&root).unwrap();
        assert!(!report.gate_ready);
        assert!(!report.required_control_docs_missing.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
