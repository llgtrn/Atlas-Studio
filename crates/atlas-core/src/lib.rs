use atlas_model::{SystemizeReport, CLI_API};
use std::{fs, io, path::Path};

pub fn validate_client_config(path: impl AsRef<Path>) -> io::Result<()> {
    let text = fs::read_to_string(path)?;
    if !text.contains("schema = \"atlas.systemizer.client.v1\"") {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "unsupported Atlas client config schema"));
    }
    if !text.contains("cli_api = \"atlas.systemizer.cli.v1\"") {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "unsupported Atlas CLI API"));
    }
    Ok(())
}

pub fn systemize(root: impl AsRef<Path>, config: impl AsRef<Path>) -> io::Result<SystemizeReport> {
    validate_client_config(config)?;
    let source = atlas_source::analyze(&root)?;
    let docs_root = root.as_ref().join("docs");
    let docs = if docs_root.exists() {
        atlas_docs::audit(&docs_root)?
    } else {
        atlas_model::DocsReport { schema:"atlas.systemizer.docs-report.v1".into(), root:docs_root.to_string_lossy().into_owned(), documents_total:0, canonical_frontmatter_total:0, missing_frontmatter:vec![], invalid_type:vec![] }
    };
    let graph = atlas_graph::summarize(&source);
    Ok(SystemizeReport {
        schema: "atlas.systemizer.systemize-report.v1".into(),
        cli_api: CLI_API.into(),
        root: root.as_ref().canonicalize()?.to_string_lossy().into_owned(),
        docs, source, graph,
        invariants: vec![
            "ATLAS_IS_DEV_TOOL".into(),
            "NO_CHRONICA_RUNTIME_DEPENDENCY".into(),
            "ANALYZE_NEVER_GRANTS_AUTHORITY".into(),
            "GENERATED_GRAPHS_ARE_REBUILDABLE".into(),
        ],
    })
}
