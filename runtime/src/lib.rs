//! Atlas runtime orchestration.

use atlas_core::{
    CLI_API, CodingAdmission, Evidence, RevisionRef, SystemizeReport, WorkPrepareReport,
    WorkRequest, summarize_repository_graph,
};
use std::{io, path::Path};

pub fn systemize(root: impl AsRef<Path>) -> io::Result<SystemizeReport> {
    let root = root.as_ref();
    let snapshot = adapter::snapshot_git(root)?;
    let repository = adapter::audit_repository(root)?;
    let source = match repository.manifest.as_ref() {
        Some(manifest) => adapter::scan_declared_source(root, manifest)?,
        None => adapter::scan_source(root)?,
    };
    let docs = adapter::audit_docs(root.join(".atlas"))?;
    let graph = summarize_repository_graph(&source, &docs);

    let mut blockers = Vec::new();
    if !repository.ready {
        blockers.push("REPO_GATE_NOT_READY".to_owned());
    }
    if !docs.gate_ready {
        blockers.push("DOCS_GATE_NOT_READY".to_owned());
    }

    Ok(SystemizeReport {
        schema: "atlas.systemizer.systemize-report.v6".into(),
        cli_api: CLI_API.into(),
        root: root.canonicalize()?.to_string_lossy().into_owned(),
        snapshot,
        repository,
        docs: docs.clone(),
        coding_admission: CodingAdmission {
            schema: "atlas.systemizer.coding-admission.v1".into(),
            allowed: blockers.is_empty(),
            docs_standard: docs.standard,
            blockers,
        },
        source,
        graph,
        invariants: vec![
            "CANONICAL_REPOSITORY_KNOWLEDGE_IS_IN_ATLAS_ROOT".into(),
            "FACTS_COMPILE_TO_ONE_ENGINEERING_GRAPH".into(),
            "GRAPH_BEFORE_CODE".into(),
            "EXACT_BASE_SHA_REQUIRED".into(),
            "ONE_CANONICAL_TARGET_PER_WORKRUN".into(),
            "DONORS_ARE_REFERENCE_AND_EVIDENCE_NOT_RUNTIME_OWNERS".into(),
            "AI_OUTPUT_IS_PROPOSAL_NOT_CANONICAL_TRUTH".into(),
        ],
    })
}

pub fn prepare_work(
    root: impl AsRef<Path>,
    goal: impl Into<String>,
    expected_base_sha: Option<String>,
) -> io::Result<WorkPrepareReport> {
    let root = root.as_ref();
    let system = systemize(root)?;
    let base_revision = RevisionRef {
        kind: "git".into(),
        value: system.snapshot.head_sha.clone(),
    };
    let mut blockers = system.coding_admission.blockers.clone();

    if let Some(expected) = expected_base_sha {
        if expected != system.snapshot.head_sha {
            blockers.push(format!(
                "BASE_SHA_DRIFT expected {expected} but checkout is {}",
                system.snapshot.head_sha
            ));
        }
    }
    if system.snapshot.dirty {
        blockers.push("WORKTREE_HAS_UNCOMMITTED_CHANGES".into());
    }

    let request = WorkRequest {
        schema: "atlas.work-request.v1".into(),
        repository: system.root.clone(),
        base_revision: base_revision.clone(),
        goal: goal.into(),
        scope: vec!["single-repository".into()],
        allowed_paths: system
            .repository
            .manifest
            .as_ref()
            .map(|manifest| {
                let mut paths = manifest.source_roots.clone();
                paths.extend(manifest.frontend_roots.clone());
                paths.extend(manifest.test_roots.clone());
                paths.sort();
                paths.dedup();
                paths
            })
            .unwrap_or_default(),
        forbidden_paths: vec![
            ".atlas/temporary".into(),
            ".atlas/provenance".into(),
            ".atlas/licenses".into(),
        ],
        required_verification: vec![
            "cargo fmt --all --check".into(),
            "cargo test --workspace".into(),
            "atlas-systemizer systemize".into(),
        ],
    };

    let allowed = blockers.is_empty();
    Ok(WorkPrepareReport {
        schema: "atlas.work-prepare-report.v1".into(),
        request,
        repository: system.repository,
        snapshot: system.snapshot.clone(),
        graph: system.graph,
        coding_admission: system.coding_admission,
        allowed,
        blockers,
        evidence: vec![Evidence {
            id: "evidence:work-prepare:git-head".into(),
            kind: "RepositorySnapshot".into(),
            path: system.root,
            summary: format!(
                "Prepared work against exact Git revision {}",
                base_revision.value
            ),
            revision: Some(base_revision),
        }],
    })
}
