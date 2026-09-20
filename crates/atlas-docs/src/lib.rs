use atlas_model::DocsReport;
use std::{fs, io, path::{Path, PathBuf}};

const TYPES: &[&str] = &["decision","blueprint","architecture","contract","runbook","reference","generated-evidence"];
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
pub fn audit(root: impl AsRef<Path>) -> io::Result<DocsReport> {
    let root = root.as_ref().canonicalize()?;
    let mut docs = Vec::new();
    visit(&root, &mut docs)?;
    docs.sort();
    let mut canonical_frontmatter_total = 0;
    let mut missing_frontmatter = Vec::new();
    let mut invalid_type = Vec::new();
    for path in &docs {
        let text = fs::read_to_string(path)?;
        let rel = path.strip_prefix(&root).unwrap_or(path).to_string_lossy().replace('\', "/");
        if !text.starts_with("---\n") { missing_frontmatter.push(rel); continue; }
        canonical_frontmatter_total += 1;
        let frontmatter = text.split("---").nth(1).unwrap_or_default();
        if let Some(line) = frontmatter.lines().find(|line| line.trim_start().starts_with("type:")) {
            let value = line.split_once(':').map(|(_,v)| v.trim()).unwrap_or_default();
            if !TYPES.contains(&value) { invalid_type.push(rel); }
        }
    }
    Ok(DocsReport { schema: "atlas.systemizer.docs-report.v1".into(), root: root.to_string_lossy().into_owned(), documents_total: docs.len(), canonical_frontmatter_total, missing_frontmatter, invalid_type })
}
