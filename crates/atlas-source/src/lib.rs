use atlas_model::{FileFact, SourceReport};
use std::{collections::BTreeMap, fs, io, path::{Path, PathBuf}};

fn language(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|x| x.to_str()).unwrap_or_default().to_ascii_lowercase().as_str() {
        "rs" => Some("rust"), "ts" | "tsx" => Some("typescript"), "js" | "jsx" | "mjs" | "cjs" => Some("javascript"),
        "py" => Some("python"), "go" => Some("go"), "c" | "h" => Some("c"), "cc" | "cpp" | "cxx" | "hpp" | "hh" => Some("cpp"),
        "java" => Some("java"), "kt" | "kts" => Some("kotlin"), "md" => Some("markdown"), "toml" => Some("toml"), "json" => Some("json"),
        _ => None,
    }
}
fn ignored(name: &str) -> bool { matches!(name, ".git" | "target" | "node_modules" | "dist" | "build" | ".next" | "coverage" | ".atlas") }
fn visit(root: &Path, dir: &Path, out: &mut Vec<FileFact>) -> io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type()?.is_dir() { if !ignored(&name) { visit(root, &path, out)?; } continue; }
        if !entry.file_type()?.is_file() { continue; }
        let Some(lang) = language(&path) else { continue };
        let bytes = entry.metadata()?.len();
        if bytes > 4 * 1024 * 1024 { continue; }
        let relative = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\', "/");
        out.push(FileFact { path: relative, language: lang.into(), bytes });
    }
    Ok(())
}
pub fn analyze(root: impl AsRef<Path>) -> io::Result<SourceReport> {
    let root: PathBuf = root.as_ref().canonicalize()?;
    let mut files = Vec::new();
    visit(&root, &root, &mut files)?;
    files.sort_by(|a,b| a.path.cmp(&b.path));
    let mut languages = BTreeMap::new();
    for file in &files { *languages.entry(file.language.clone()).or_insert(0) += 1; }
    Ok(SourceReport { schema: "atlas.systemizer.source-report.v1".into(), root: root.to_string_lossy().into_owned(), files_total: files.len(), languages, files })
}
