use atlas_model::{CodingAdmission, RepoAuditSummary, SystemizeReport, CLI_API};
use std::{io, path::Path};

pub fn systemize(root: impl AsRef<Path>) -> io::Result<SystemizeReport> {
    let repository = atlas_repo::audit(&root)?;
    let source = atlas_source::analyze(&root)?;
    let docs = atlas_docs::audit(root.as_ref().join(".atlas"))?;
    let graph = atlas_graph::summarize(&source);

    let mut blockers = Vec::new();
    if !repository.ready { blockers.push("REPO_GATE_NOT_READY".to_owned()); }
    if !docs.gate_ready { blockers.push("ATLAS_KNOWLEDGE_GATE_NOT_READY".to_owned()); }
    let coding_admission = CodingAdmission {
        schema: "atlas.systemizer.coding-admission.v1".into(),
        allowed: blockers.is_empty(),
        docs_standard: docs.standard.clone(),
        blockers,
    };

    Ok(SystemizeReport {
        schema: "atlas.systemizer.systemize-report.v3".into(),
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
            "ATLAS_IS_EXTERNAL_ENGINEERING_FORGE".into(),
            "ATLAS_KNOWLEDGE_ROOT_IS_DOT_ATLAS".into(),
            "LEGACY_DOCS_ROOT_IS_FORBIDDEN".into(),
            "ATLAS_OUTPUT_IS_HUMAN_READABLE".into(),
            "ANALYZE_NEVER_GRANTS_AUTHORITY".into(),
            "NO_COMPLETE_ATLAS_KNOWLEDGE_NO_CODING".into(),
            "GRAPH_BEFORE_CODE".into(),
            "ONE_CANONICAL_TARGET_PER_SESSION".into(),
        ],
    })
}
