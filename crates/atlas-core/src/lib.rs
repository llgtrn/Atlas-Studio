use atlas_model::{CodingAdmission, RepoAuditSummary, SystemizeReport, CLI_API};
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
    let repository = atlas_repo::audit(&root)?;
    let source = atlas_source::analyze(&root)?;
    let docs = atlas_docs::audit(root.as_ref().join("docs"))?;
    let graph = atlas_graph::summarize(&source);

    let mut blockers = Vec::new();
    if !repository.ready { blockers.push("REPO_GATE_NOT_READY".to_owned()); }
    if !docs.gate_ready { blockers.push("DOCS_GATE_NOT_READY".to_owned()); }
    let coding_admission = CodingAdmission {
        schema: "atlas.systemizer.coding-admission.v1".into(),
        allowed: blockers.is_empty(),
        docs_standard: docs.standard.clone(),
        blockers,
    };

    Ok(SystemizeReport {
        schema: "atlas.systemizer.systemize-report.v1".into(),
        cli_api: CLI_API.into(),
        root: root.as_ref().canonicalize()?.to_string_lossy().into_owned(),
        repository: RepoAuditSummary {
            schema: repository.schema,
            archetype: repository.archetype,
            ready: repository.ready,
            missing_required_roles: repository.missing_required_roles,
            missing_mapped_paths: repository.missing_mapped_paths,
            forbidden_roots_present: repository.forbidden_roots_present,
        },
        docs,
        coding_admission,
        source,
        graph,
        invariants: vec![
            "ATLAS_IS_DEV_TOOL".into(),
            "NO_CHRONICA_RUNTIME_DEPENDENCY".into(),
            "ANALYZE_NEVER_GRANTS_AUTHORITY".into(),
            "GENERATED_GRAPHS_ARE_REBUILDABLE".into(),
            "NO_COMPLETE_DOCS_NO_CODING_ADMISSION".into(),
            "ONE_REPOSITORY_PER_CODING_SESSION".into(),
        ],
    })
}
