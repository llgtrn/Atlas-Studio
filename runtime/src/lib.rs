//! Atlas runtime orchestration.

pub mod census;
pub mod inventory;
pub mod normalize;

use atlas_core::{
    AdlCompileReport, AdlProgram, CLI_API, CodingAdmission, Contract, DocsReport, EngineeringGraph,
    Evidence, RepoAudit, RepositoryId, RevisionRef, SystemizeReport, WorkPrepareReport,
    WorkRequest, build_system_graph, compile_adl, parse_adl_source, summarize_system_graph,
};
use std::{io, path::Path};

/// The `RepositoryId` extraction/census pin for `root`: the declared manifest repo name when one
/// exists, else the canonicalized root path. Shared by every entry point that runs extraction so
/// they all pin identity the same way.
fn resolve_repository_id(repository: &RepoAudit, root: &Path) -> RepositoryId {
    RepositoryId::new(
        repository
            .manifest
            .as_ref()
            .map(|manifest| manifest.repo.clone())
            .unwrap_or_else(|| root.to_string_lossy().into_owned()),
    )
}

pub fn systemize(root: impl AsRef<Path>) -> io::Result<SystemizeReport> {
    let root = root.as_ref();
    let snapshot = adapter::snapshot_git(root)?;
    let repository = adapter::audit_repository(root)?;
    let inventory = inventory::build_inventory(root, repository.manifest.as_ref())?;
    let source = adapter::source_report_from_inventory(&inventory);
    let docs = adapter::audit_docs(root.join(".atlas"))?;
    let adl_sources = adapter::read_adl_sources(root)?;
    let adl = compile_adl(&adl_sources, &source);

    // R4.3.1: semantic extraction runs BEFORE census construction, and its results flow directly
    // into the canonical `CensusReport` that `normalize`/`graph` below then consume -- extraction
    // is no longer a post-census accounting-only sidecar
    // (`.atlas/contracts/SEMANTIC-EXTRACTION.md`: "Census -> Normalize -> Reconcile -> Engineering
    // Graph" is the one normalized path). `extraction_accounting` and `census` are both built from
    // the exact same `extraction_batches`, so closure accounting and canonical census truth can
    // never disagree about what extraction produced.
    let repository_id = resolve_repository_id(&repository, root);
    let extraction_batches =
        census::extraction::extract_semantics(&inventory, repository_id, snapshot.revision());
    let mut extraction_accounting = census::CensusExtractionAccounting::new();
    for batch in &extraction_batches {
        extraction_accounting.record_batch(batch);
    }

    let mut census = census::build_census(&inventory, &source, &adl, &extraction_batches);
    let normalization = normalize::normalize(&census);
    let graph = summarize_system_graph(&source, &docs, &normalization);

    // `.atlas/contracts/DEPENDENCY-CENSUS.md`: census does not stop at the repository boundary.
    // `BUILD` starts life as a permanent `Unsupported` stub in `census::build_census` (an
    // accounting axis outside the R4 semantic dimension set, computed with no filesystem access);
    // promote it to real evidence here now that a real, closed Cargo dependency closure exists.
    let dependency_closure = adapter::census_cargo_workspace(root)?.unwrap_or_else(|| {
        atlas_core::DependencyClosureReport {
            schema: "atlas.dependency-closure-report.v1".into(),
            ecosystem: atlas_core::DependencyEcosystem::Cargo,
            root: root.to_string_lossy().into_owned(),
            edges_total: 0,
            instances_total: 0,
            edges: Vec::new(),
            dangling_references: Vec::new(),
        }
    });
    if dependency_closure.edges_total > 0 && dependency_closure.is_closed() {
        census
            .coverage
            .insert("BUILD".into(), atlas_core::EpistemicStatus::Observed);
    } else if !dependency_closure.dangling_references.is_empty() {
        census
            .coverage
            .insert("BUILD".into(), atlas_core::EpistemicStatus::Unknown);
    }

    let mut blockers = Vec::new();
    if !repository.ready {
        blockers.push("REPO_GATE_NOT_READY".to_owned());
    }
    if !docs.gate_ready {
        blockers.push("DOCS_GATE_NOT_READY".to_owned());
    }
    if !adl.diagnostics.is_empty() {
        blockers.push("ADL_DIAGNOSTICS_PRESENT".to_owned());
    }
    if !inventory.is_closed() {
        blockers.push("INVENTORY_ACCOUNTING_NOT_CLOSED".to_owned());
    }
    if !census.is_closed() {
        blockers.push("CENSUS_ACCOUNTING_NOT_CLOSED".to_owned());
    }
    if !normalization.is_closed() {
        blockers.push("NORMALIZATION_ACCOUNTING_NOT_CLOSED".to_owned());
    }
    if !extraction_accounting.is_closed(&census::extraction::ALL_SEMANTIC_DIMENSIONS) {
        blockers.push("SEMANTIC_EXTRACTION_ACCOUNTING_NOT_CLOSED".to_owned());
    }
    if !dependency_closure.is_closed() {
        blockers.push("DEPENDENCY_CLOSURE_NOT_CLOSED".to_owned());
    }

    Ok(SystemizeReport {
        schema: "atlas.systemizer.systemize-report.v12".into(),
        cli_api: CLI_API.into(),
        root: root.canonicalize()?.to_string_lossy().into_owned(),
        snapshot,
        repository,
        docs: docs.clone(),
        adl,
        coding_admission: CodingAdmission {
            schema: "atlas.systemizer.coding-admission.v1".into(),
            allowed: blockers.is_empty(),
            docs_standard: docs.standard,
            blockers,
        },
        inventory,
        source,
        census,
        normalization,
        graph,
        dependency_closure,
        invariants: vec![
            "CANONICAL_REPOSITORY_KNOWLEDGE_IS_IN_ATLAS_ROOT".into(),
            "FACTS_COMPILE_TO_ONE_ENGINEERING_GRAPH".into(),
            "GRAPH_BEFORE_CODE".into(),
            "EXACT_BASE_SHA_REQUIRED".into(),
            "ONE_CANONICAL_TARGET_PER_WORKRUN".into(),
            "DONORS_ARE_REFERENCE_AND_EVIDENCE_NOT_RUNTIME_OWNERS".into(),
            "INVENTORY_PRECEDES_SEMANTIC_DEPTH".into(),
            "UNKNOWN_OR_OVERSIZED_ARTIFACTS_CANNOT_DISAPPEAR".into(),
            "CENSUS_PRECEDES_NORMALIZATION".into(),
            "NORMALIZATION_PRESERVES_PROVENANCE".into(),
            "NORMALIZATION_MUST_NOT_DROP_CENSUS_FACTS".into(),
            "ENGINEERING_GRAPH_IS_PROJECTION_OF_NORMALIZED_FACTS".into(),
            "UNSUPPORTED_SEMANTIC_DIMENSIONS_ARE_EXPLICIT".into(),
            "SEMANTIC_EXTRACTION_NEVER_FABRICATES_COMPILER_RESOLVED_SEMANTICS".into(),
            "AI_OUTPUT_IS_PROPOSAL_NOT_CANONICAL_TRUTH".into(),
            "DEPENDENCY_CLOSURE_IS_CANONICAL_CENSUS_TRUTH".into(),
        ],
    })
}

pub fn check(root: impl AsRef<Path>) -> io::Result<AdlCompileReport> {
    let root = root.as_ref();
    let repository = adapter::audit_repository(root)?;
    let source = match repository.manifest.as_ref() {
        Some(manifest) => adapter::scan_declared_source(root, manifest)?,
        None => adapter::scan_source(root)?,
    };
    let adl_sources = adapter::read_adl_sources(root)?;
    Ok(compile_adl(&adl_sources, &source))
}

pub fn graph(root: impl AsRef<Path>) -> io::Result<EngineeringGraph> {
    let root = root.as_ref();
    let snapshot = adapter::snapshot_git(root)?;
    let repository = adapter::audit_repository(root)?;
    let inventory = inventory::build_inventory(root, repository.manifest.as_ref())?;
    let source = adapter::source_report_from_inventory(&inventory);
    let docs = adapter::audit_docs(root.join(".atlas"))?;
    let adl_sources = adapter::read_adl_sources(root)?;
    let adl = compile_adl(&adl_sources, &source);
    let repository_id = resolve_repository_id(&repository, root);
    let extraction_batches =
        census::extraction::extract_semantics(&inventory, repository_id, snapshot.revision());
    let census = census::build_census(&inventory, &source, &adl, &extraction_batches);
    let normalization = normalize::normalize(&census);
    Ok(build_system_graph(&source, &docs, &normalization))
}

pub fn contract() -> Contract {
    Contract::default()
}

pub fn docs_audit(root: impl AsRef<Path>) -> io::Result<DocsReport> {
    adapter::audit_docs(root)
}

pub fn code_analyze(root: impl AsRef<Path>) -> io::Result<serde_json::Value> {
    let root = root.as_ref();
    let snapshot = adapter::snapshot_git(root)?;
    let repository = adapter::audit_repository(root)?;
    let inventory = inventory::build_inventory(root, repository.manifest.as_ref())?;
    let source = adapter::source_report_from_inventory(&inventory);
    let docs = adapter::audit_docs(root.join(".atlas"))?;
    let adl_sources = adapter::read_adl_sources(root)?;
    let adl = compile_adl(&adl_sources, &source);
    let repository_id = resolve_repository_id(&repository, root);
    let extraction_batches =
        census::extraction::extract_semantics(&inventory, repository_id, snapshot.revision());
    let census = census::build_census(&inventory, &source, &adl, &extraction_batches);
    let normalization = normalize::normalize(&census);
    let graph = summarize_system_graph(&source, &docs, &normalization);

    Ok(serde_json::json!({
        "schema": "atlas.systemizer.code-analysis.v3",
        "inventory": inventory,
        "source": source,
        "adl": adl,
        "census": census,
        "normalization": normalization,
        "graph": graph,
        "source_of_truth": "derived engineering analysis; target repositories remain sovereign"
    }))
}

pub fn parse(root: impl AsRef<Path>) -> io::Result<Vec<AdlProgram>> {
    let sources = adapter::read_adl_sources(root)?;
    Ok(sources.iter().map(parse_adl_source).collect())
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

    if let Some(expected) = expected_base_sha
        && expected != system.snapshot.head_sha
    {
        blockers.push(format!(
            "BASE_SHA_DRIFT expected {expected} but checkout is {}",
            system.snapshot.head_sha
        ));
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
