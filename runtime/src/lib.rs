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

/// `BUILD` coverage promotion and coding-admission blocking, derived purely from a
/// `DependencyClosureReport`'s `state` (`.atlas/contracts/DEPENDENCY-CENSUS.md#implementation-status`).
/// Returns `(coverage update, whether this state blocks coding admission)`. Extracted as a pure
/// function so the exact defect it replaces -- a fabricated `NotApplicable` report whose
/// `dangling_references.is_empty()` made `is_closed()` silently read `true`, so a repository with
/// no Cargo workspace at all never raised `DEPENDENCY_CLOSURE_NOT_CLOSED` and looked identical to
/// a genuinely verified empty closure -- is directly, cheaply falsifiable without running the full
/// `systemize` pipeline.
fn build_coverage_from_dependency_closure(
    state: atlas_core::DependencyClosureState,
) -> (Option<atlas_core::EpistemicStatus>, bool) {
    use atlas_core::{DependencyClosureState as State, EpistemicStatus as Status};
    match state {
        State::Closed => (Some(Status::Observed), false),
        State::Partial | State::Blocked => (Some(Status::Unknown), true),
        State::NotApplicable => (None, false),
    }
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
    //
    // `census_cargo_workspace` returning `None` means no `Cargo.lock` exists at all -- this
    // ecosystem is `NotApplicable` here, never a fabricated empty-but-`Closed` report. Previously
    // this fallback built a bare zero-edge report whose `dangling_references.is_empty()` made
    // `is_closed()` silently `true` for a repository where Cargo dependency census never ran at
    // all -- indistinguishable from a genuinely verified empty closure. `DependencyClosureState`
    // now makes that distinction explicit end to end.
    let dependency_closure = adapter::census_cargo_workspace(root)?.unwrap_or_else(|| {
        atlas_core::DependencyClosureReport {
            schema: "atlas.dependency-closure-report.v3".into(),
            ecosystem: atlas_core::DependencyEcosystem::Cargo,
            root: root.to_string_lossy().into_owned(),
            state: atlas_core::DependencyClosureState::NotApplicable,
            edges_total: 0,
            instances_total: 0,
            edges: Vec::new(),
            dangling_references: Vec::new(),
            unsupported_constructs: Vec::new(),
            dynamic_obligations: Vec::new(),
        }
    });
    // `NotApplicable` (no Cargo.lock at all -- e.g. a non-Rust admitted repository) is not a
    // failure and leaves `BUILD` at whatever `census::build_census` already set (`Unsupported`):
    // this ecosystem was never observed here, not incompletely observed. Only `Partial`/`Blocked`
    // (a real Cargo workspace census actually attempted and failed to fully close) demote `BUILD`
    // to `Unknown` and raise a coding-admission blocker.
    let (build_status, dependency_closure_blocks) =
        build_coverage_from_dependency_closure(dependency_closure.state);
    if let Some(status) = build_status {
        census.coverage.insert("BUILD".into(), status);
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
    // `NotApplicable` never blocks: absence of a Cargo workspace at this root is not itself a
    // failure (see `build_coverage_from_dependency_closure` above). Only an actually-attempted-
    // but-incomplete Cargo census (`Partial`/`Blocked`) withholds coding admission.
    if dependency_closure_blocks {
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

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::{DependencyClosureState, EpistemicStatus};

    // `.atlas/contracts/DEPENDENCY-CENSUS.md`: BUILD coverage/coding-admission must derive from
    // evidence state, not default structure values. One falsification case per state transition.

    #[test]
    fn not_applicable_promotes_no_coverage_and_never_blocks() {
        let (status, blocks) =
            build_coverage_from_dependency_closure(DependencyClosureState::NotApplicable);
        assert_eq!(status, None);
        assert!(
            !blocks,
            "a repository with no Cargo workspace at all must not be treated as a failed dependency census"
        );
    }

    #[test]
    fn closed_promotes_build_to_observed_and_never_blocks() {
        let (status, blocks) =
            build_coverage_from_dependency_closure(DependencyClosureState::Closed);
        assert_eq!(status, Some(EpistemicStatus::Observed));
        assert!(!blocks);
    }

    #[test]
    fn partial_demotes_build_to_unknown_and_blocks() {
        let (status, blocks) =
            build_coverage_from_dependency_closure(DependencyClosureState::Partial);
        assert_eq!(status, Some(EpistemicStatus::Unknown));
        assert!(blocks);
    }

    #[test]
    fn blocked_demotes_build_to_unknown_and_blocks() {
        let (status, blocks) =
            build_coverage_from_dependency_closure(DependencyClosureState::Blocked);
        assert_eq!(status, Some(EpistemicStatus::Unknown));
        assert!(
            blocks,
            "a present but unparseable Cargo.lock must not be treated as a closed dependency census"
        );
    }

    #[test]
    fn not_applicable_and_closed_are_never_conflated() {
        // The exact regression this module's helper replaces: both states previously produced
        // `is_closed() == true` from a fabricated zero-edge report, making a repository with no
        // Cargo workspace indistinguishable from one with a genuinely verified empty closure.
        let (not_applicable_status, not_applicable_blocks) =
            build_coverage_from_dependency_closure(DependencyClosureState::NotApplicable);
        let (closed_status, closed_blocks) =
            build_coverage_from_dependency_closure(DependencyClosureState::Closed);
        assert_ne!(not_applicable_status, closed_status);
        assert_eq!(
            not_applicable_blocks, closed_blocks,
            "neither blocks, but for different reasons"
        );
    }
}
