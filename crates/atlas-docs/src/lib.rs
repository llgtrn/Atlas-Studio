use atlas_model::DocsReport;
use std::{collections::BTreeMap, fs, io, path::{Path, PathBuf}};

const TYPES: &[&str] = &["decision","blueprint","architecture","contract","runbook","reference","generated-evidence"];
const REQUIRED: &[&str] = &["id","type","status","canonical"];

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
            if !TYPES.contains(&kind.as_str()) {
                invalid_type.push(rel.clone());
            }
        }

        if let Some(id) = meta.get("id") {
            ids.entry(id.clone()).or_default().push(rel.clone());
        }

        if meta.get("status").map(String::as_str) == Some("superseded")
            && !meta.contains_key("superseded_by")
        {
            superseded_without_successor.push(rel.clone());
        }

        for raw_link in markdown_links(&text) {
            if raw_link.is_empty()
                || raw_link.starts_with('#')
                || raw_link.starts_with("http://")
                || raw_link.starts_with("https://")
                || raw_link.starts_with("mailto:")
                || raw_link.starts_with('/')
            {
                continue;
            }
            let target = raw_link.split('#').next().unwrap_or_default().split('?').next().unwrap_or_default();
            if target.is_empty() || !target.ends_with(".md") { continue; }
            let resolved = path.parent().unwrap_or(&root).join(target);
            if !resolved.exists() {
                broken_internal_refs.push(format!("{rel}->{raw_link}"));
            }
        }
    }

    let duplicate_ids = ids.into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .map(|(id, paths)| format!("{id}:{}", paths.join(",")))
        .collect::<Vec<_>>();

    Ok(DocsReport {
        schema: "atlas.systemizer.docs-report.v1".into(),
        root: root.to_string_lossy().into_owned(),
        documents_total: docs.len(),
        canonical_frontmatter_total,
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
    fn detects_duplicate_ids_and_missing_successor() {
        let root = env::temp_dir().join(format!("atlas-docs-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let doc = "---\nid: same\ntype: decision\nstatus: superseded\ncanonical: true\n---\n";
        fs::write(root.join("a.md"), doc).unwrap();
        fs::write(root.join("b.md"), doc).unwrap();
        let report = audit(&root).unwrap();
        assert_eq!(report.duplicate_ids.len(), 1);
        assert_eq!(report.superseded_without_successor.len(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}
