//! Atlas runtime orchestration.

pub mod census;
pub mod inventory;
pub mod normalize;

use atlas_core::{
    AdlCompileReport, AdlProgram, CLI_API, CodingAdmission, ConstraintResult, Contract,
    DependencyClosureReport, DependencyClosureState, DependencyEcosystem, DocsReport,
    EngineeringGraph, Evidence, RepoAudit, RepoManifest, RepositoryId, RevisionRef,
    SystemizeReport, WorkPrepareReport, WorkRequest, add_constraint_derivations,
    add_dependency_closure, build_system_graph, compile_adl, parse_adl_source,
    summarize_system_graph_with_dependencies,
};
use std::{io, path::Path};

/// The real, evidenced Cargo dependency closure for `root` (`census_cargo_workspace`), or a
/// `NotApplicable` report when no `Cargo.lock` exists at all -- never a fabricated empty-but-
/// `Closed` report (`.atlas/contracts/DEPENDENCY-CENSUS.md#implementation-status`). Shared by
/// every entry point that needs the closure (`systemize`, `graph`, `code_analyze`) so they can
/// never disagree about which state a given root resolves to.
fn resolve_dependency_closure(root: &Path) -> io::Result<DependencyClosureReport> {
    Ok(
        adapter::census_cargo_workspace(root)?.unwrap_or_else(|| DependencyClosureReport {
            schema: "atlas.dependency-closure-report.v3".into(),
            ecosystem: DependencyEcosystem::Cargo,
            root: root.to_string_lossy().into_owned(),
            state: DependencyClosureState::NotApplicable,
            edges_total: 0,
            instances_total: 0,
            edges: Vec::new(),
            dangling_references: Vec::new(),
            unsupported_constructs: Vec::new(),
            dynamic_obligations: Vec::new(),
        }),
    )
}

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

/// `WorkRequest.allowed_paths`: every root a manifest declares across `source_roots`,
/// `backend_roots`, `frontend_roots` and `test_roots` -- all four, per
/// `.atlas/contracts/EXTERNAL-PROVIDER-TRUST.md#capability-minimum`'s own documented rule that
/// these are "joined against the repository root before walking the filesystem" -- filtered
/// through the same `declared_root_is_contained` predicate `adapter::inventory_declared_source`
/// itself already enforces, sorted, and deduplicated. `backend_roots` was validated for
/// path-escape by `core::constraint::validate_manifest` from the day that field existed, but was
/// silently omitted from this list (and from `inventory_declared_source`'s own walk) until this
/// fix: a manifest declaring a `backend_roots` entry not already covered by one of the other three
/// fields produced zero artifacts for it and granted an external provider zero capability to touch
/// it, even though it is legitimate, admitted repository source.
///
/// A manifest-declared root that escapes the repository boundary is already caught by
/// `validate_manifest`/`repository.ready` (a `REPO_GATE_NOT_READY` blocker, which makes
/// `prepare_work`'s own `allowed` field `false`) and is never walked by `inventory_declared_source`
/// regardless. Filtered again here as defense in depth: `allowed_paths` is the literal
/// capability-scoping data an external provider reads to know what it may touch, so it must never
/// contain an escaping entry even if some future caller inspected this list without first checking
/// `allowed`/`coding_admission`.
///
/// Extracted as a pure function (matching `resolve_dependency_closure`/
/// `build_coverage_from_dependency_closure`'s own precedent) so this exact list-construction logic
/// is directly, cheaply unit-testable without running the full `systemize` pipeline against a real
/// on-disk repository -- the gap this function itself closes was previously invisible precisely
/// because no test exercised it in isolation.
fn work_allowed_paths(manifest: Option<&RepoManifest>) -> Vec<String> {
    let Some(manifest) = manifest else {
        return Vec::new();
    };
    let mut paths = manifest.source_roots.clone();
    paths.extend(manifest.backend_roots.clone());
    paths.extend(manifest.frontend_roots.clone());
    paths.extend(manifest.test_roots.clone());
    paths.retain(|path| atlas_core::declared_root_is_contained(path));
    paths.sort();
    paths.dedup();
    paths
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

/// Whether `coding_admission` must block on ADL constraint/invariant/materialization-delta
/// evaluation. Extracted as a pure function so the exact defect it replaces -- this gate
/// previously inspected only `adl.diagnostics` (parse/link diagnostics), never
/// `adl.constraint_results` (the sibling field `evaluate_constraints` and the declared/observed
/// materialization deltas actually report failures through), so a declared constraint, invariant,
/// or materialization could fail while `coding_admission.allowed` stayed `true` -- is directly,
/// cheaply falsifiable without running the full `systemize` pipeline.
fn adl_constraint_violation_blocks(constraint_results: &[ConstraintResult]) -> bool {
    constraint_results.iter().any(|result| !result.passed)
}

/// Every input `systemize`/`graph`/`code_analyze` need before they can build their own
/// entry-point-specific report: the repository snapshot/audit/inventory/source/docs, compiled ADL,
/// and real `SemanticExtractor` output for the admitted inventory. Extracted as one shared
/// pipeline stage so the three entry points -- each of which independently repeated this exact
/// eight-statement sequence before this fix -- can never silently diverge in how they gather it
/// (e.g. one adding a new admitted-source step the other two forget), the same "shared, not
/// re-derived" discipline already applied to `resolve_repository_id`/`resolve_dependency_closure`
/// above.
struct CensusInputs {
    snapshot: atlas_core::RepositorySnapshot,
    repository: RepoAudit,
    inventory: atlas_core::InventoryReport,
    source: atlas_core::SourceReport,
    docs: DocsReport,
    adl: AdlCompileReport,
    extraction_batches: Vec<adapter::ExtractionBatch>,
}

fn gather_census_inputs(root: &Path) -> io::Result<CensusInputs> {
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
    Ok(CensusInputs {
        snapshot,
        repository,
        inventory,
        source,
        docs,
        adl,
        extraction_batches,
    })
}

pub fn systemize(root: impl AsRef<Path>) -> io::Result<SystemizeReport> {
    let root = root.as_ref();
    let CensusInputs {
        snapshot,
        repository,
        inventory,
        source,
        docs,
        adl,
        extraction_batches,
    } = gather_census_inputs(root)?;

    // R4.3.1: semantic extraction runs BEFORE census construction, and its results flow directly
    // into the canonical `CensusReport` that `normalize`/`graph` below then consume -- extraction
    // is no longer a post-census accounting-only sidecar
    // (`.atlas/contracts/SEMANTIC-EXTRACTION.md`: "Census -> Normalize -> Reconcile -> Engineering
    // Graph" is the one normalized path). `extraction_accounting` and `census` are both built from
    // the exact same `extraction_batches`, so closure accounting and canonical census truth can
    // never disagree about what extraction produced.
    let mut extraction_accounting = census::CensusExtractionAccounting::new();
    for batch in &extraction_batches {
        extraction_accounting.record_batch(batch);
    }

    let mut census = census::build_census(&inventory, &source, &adl, &extraction_batches);
    let normalization = normalize::normalize(&census);

    // `.atlas/contracts/DEPENDENCY-CENSUS.md`: census does not stop at the repository boundary.
    // `BUILD` starts life as a permanent `Unsupported` stub in `census::build_census` (an
    // accounting axis outside the R4 semantic dimension set, computed with no filesystem access);
    // promote it to real evidence here now that a real, closed Cargo dependency closure exists.
    // Computed before `graph` below so the resolved edges can be projected into it
    // (`DEPENDENCY-CENSUS.md`: "the dependency graph is part of canonical census truth... feeds
    // query, graph, security..." -- previously `dependency_closure` reached only this report's own
    // sibling field, never `EngineeringGraph` itself).
    let dependency_closure = resolve_dependency_closure(root)?;
    let graph = summarize_system_graph_with_dependencies(
        &source,
        &docs,
        &normalization,
        &dependency_closure,
        &adl.constraint_results,
    );
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
    if adl_constraint_violation_blocks(&adl.constraint_results) {
        blockers.push("ADL_CONSTRAINT_VIOLATED".to_owned());
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
    let CensusInputs {
        inventory,
        source,
        docs,
        adl,
        extraction_batches,
        ..
    } = gather_census_inputs(root)?;
    let census = census::build_census(&inventory, &source, &adl, &extraction_batches);
    let normalization = normalize::normalize(&census);
    let mut graph = build_system_graph(&source, &docs, &normalization);
    add_constraint_derivations(&mut graph, &adl.constraint_results);
    add_dependency_closure(&mut graph, &resolve_dependency_closure(root)?);
    Ok(graph)
}

pub fn contract() -> Contract {
    Contract::default()
}

pub fn docs_audit(root: impl AsRef<Path>) -> io::Result<DocsReport> {
    adapter::audit_docs(root)
}

pub fn code_analyze(root: impl AsRef<Path>) -> io::Result<serde_json::Value> {
    let root = root.as_ref();
    let CensusInputs {
        inventory,
        source,
        docs,
        adl,
        extraction_batches,
        ..
    } = gather_census_inputs(root)?;
    let mut census = census::build_census(&inventory, &source, &adl, &extraction_batches);
    // Same promotion `systemize` applies (`.atlas/contracts/DEPENDENCY-CENSUS.md`): `BUILD`
    // starts life as a permanent `Unsupported` stub in `census::build_census` and must be
    // promoted to real evidence once a real, closed Cargo dependency closure exists -- otherwise
    // `code_analyze`'s own `census.coverage.BUILD` silently disagrees with the `dependency_closure`
    // this same response reports two fields below, contradicting real, observed state.
    let dependency_closure = resolve_dependency_closure(root)?;
    let (build_status, _) = build_coverage_from_dependency_closure(dependency_closure.state);
    if let Some(status) = build_status {
        census.coverage.insert("BUILD".into(), status);
    }
    let normalization = normalize::normalize(&census);
    let graph = summarize_system_graph_with_dependencies(
        &source,
        &docs,
        &normalization,
        &dependency_closure,
        &adl.constraint_results,
    );

    Ok(serde_json::json!({
        "schema": "atlas.systemizer.code-analysis.v3",
        "inventory": inventory,
        "source": source,
        "adl": adl,
        "census": census,
        "normalization": normalization,
        "dependency_closure": dependency_closure,
        "graph": graph,
        "source_of_truth": "derived engineering analysis; target repositories remain sovereign"
    }))
}

pub fn parse(root: impl AsRef<Path>) -> io::Result<Vec<AdlProgram>> {
    let sources = adapter::read_adl_sources(root)?;
    Ok(sources.iter().map(parse_adl_source).collect())
}

/// The exact command set the repository's own CI gate (`.github/workflows/ci.yml`) enforces on
/// every push/PR, in the order CI runs them. `WorkRequest.required_verification` must stay a
/// superset of this list: a candidate that only satisfies a weaker local list could pass
/// `prepare_work`'s admission and still fail CI, which is exactly the gap this closes (CI already
/// runs `cargo clippy --workspace --all-targets -- -D warnings`, but this list previously omitted
/// it and would have let a lint-violating candidate look admissible).
fn required_verification_commands() -> Vec<String> {
    vec![
        "cargo fmt --all --check".into(),
        "cargo clippy --workspace --all-targets -- -D warnings".into(),
        "cargo test --workspace".into(),
        "atlas-systemizer systemize".into(),
    ]
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
        allowed_paths: work_allowed_paths(system.repository.manifest.as_ref()),
        forbidden_paths: vec![
            ".atlas/temporary".into(),
            ".atlas/provenance".into(),
            ".atlas/licenses".into(),
        ],
        required_verification: required_verification_commands(),
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

    fn manifest_with_roots(
        source_roots: Vec<&str>,
        backend_roots: Vec<&str>,
        frontend_roots: Vec<&str>,
        test_roots: Vec<&str>,
    ) -> RepoManifest {
        RepoManifest {
            schema: "atlas.repo.v2".into(),
            repo: "org/repo".into(),
            system_kind: "SYSTEM_INVENTION_FORGE".into(),
            backend_language: "rust".into(),
            frontend_language: "typescript".into(),
            coding_requires_docs_gate: true,
            graph_before_code_required: true,
            exact_base_sha_required: true,
            single_repository_target_required: true,
            knowledge_root: ".atlas".into(),
            temporary_root: ".atlas/temporary".into(),
            provenance_root: ".atlas/provenance".into(),
            license_root: ".atlas/licenses".into(),
            source_roots: source_roots.into_iter().map(String::from).collect(),
            backend_roots: backend_roots.into_iter().map(String::from).collect(),
            frontend_roots: frontend_roots.into_iter().map(String::from).collect(),
            test_roots: test_roots.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn work_allowed_paths_includes_backend_roots_not_covered_by_any_other_field() {
        // The exact regression this function's own extraction fixes: `backend_roots` was declared
        // in the manifest schema, validated for path-escape, and documented
        // (`.atlas/contracts/EXTERNAL-PROVIDER-TRUST.md#capability-minimum`) as one of the four
        // fields joined into the filesystem walk / capability scope -- but silently never actually
        // included here, so a `backend_roots` entry not already covered by `source_roots`/
        // `frontend_roots`/`test_roots` granted an external provider zero capability to touch
        // legitimate, admitted repository source.
        let manifest = manifest_with_roots(vec!["shared"], vec!["services/api"], vec![], vec![]);
        let paths = work_allowed_paths(Some(&manifest));
        assert!(
            paths.iter().any(|path| path == "services/api"),
            "a backend_roots entry not covered by any other field must appear in allowed_paths: {paths:?}"
        );
    }

    #[test]
    fn work_allowed_paths_unions_all_four_fields_deduplicated_and_sorted() {
        let manifest = manifest_with_roots(
            vec!["core", "shared"],
            vec!["shared"],
            vec!["apps/web"],
            vec!["core/tests"],
        );
        let paths = work_allowed_paths(Some(&manifest));
        assert_eq!(
            paths,
            vec![
                "apps/web".to_owned(),
                "core".to_owned(),
                "core/tests".to_owned(),
                "shared".to_owned(),
            ],
            "a root declared in both source_roots and backend_roots must appear exactly once"
        );
    }

    #[test]
    fn work_allowed_paths_filters_an_escaping_backend_root() {
        let manifest = manifest_with_roots(vec![], vec!["../outside"], vec![], vec![]);
        let paths = work_allowed_paths(Some(&manifest));
        assert!(
            paths.is_empty(),
            "an escaping backend_roots entry must never reach allowed_paths: {paths:?}"
        );
    }

    #[test]
    fn work_allowed_paths_with_no_manifest_is_empty() {
        assert!(work_allowed_paths(None).is_empty());
    }

    // `.github/workflows/ci.yml` is the repository's real, authoritative CI gate. This is a
    // direct, file-based falsification: it reads the actual CI workflow rather than re-asserting
    // a hardcoded expectation, so it cannot silently drift the way the previous omission did
    // (required_verification was missing the clippy command CI has run all along).
    #[test]
    fn required_verification_commands_cover_every_command_ci_runs() {
        let ci_yaml = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows/ci.yml"),
        )
        .expect("repository's CI workflow file must exist and be readable");

        let ci_commands: Vec<&str> = ci_yaml
            .lines()
            .filter_map(|line| line.trim().strip_prefix("- run: cargo "))
            .map(|rest| rest.trim())
            .collect();
        assert!(
            !ci_commands.is_empty(),
            "expected to find at least one `- run: cargo ...` step in ci.yml; the parsing above \
             may have drifted from the workflow file's actual format"
        );

        let required = required_verification_commands();
        for ci_command in ci_commands {
            let full_command = format!("cargo {ci_command}");
            assert!(
                required.iter().any(|r| r == &full_command),
                "CI runs `{full_command}` but WorkRequest.required_verification does not \
                 require it -- a candidate could pass prepare_work's admission and still fail CI"
            );
        }
    }

    // `.atlas/contracts/cli/atlas-systemizer-cli-v1.json` is a machine-readable JSON Schema
    // describing this binary's own stable CLI surface -- but nothing in this workspace ever
    // validated the real CLI against it, so it silently drifted: it named a `"fleet connect"`
    // subcommand that has never existed in this codebase, was missing three real subcommands
    // (`check`/`graph`/`parse`), and named the wrong `subsystem_kind` -- the exact inversion a
    // "stable CLI contract" exists to prevent (a schema validator built against the stale file
    // would reject real, working subcommands while accepting a phantom one). Corrected the file's
    // content and added this permanent check so it can never drift unnoticed again.
    #[test]
    fn cli_contract_json_matches_the_real_contract_default() {
        let contract_text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../.atlas/contracts/cli/atlas-systemizer-cli-v1.json"),
        )
        .expect(".atlas/contracts/cli/atlas-systemizer-cli-v1.json must exist and be readable");
        let schema: serde_json::Value =
            serde_json::from_str(&contract_text).expect("the CLI contract must be valid JSON");

        let real = Contract::default();

        assert_eq!(
            schema["properties"]["schema"]["const"].as_str(),
            Some(real.schema.as_str())
        );
        assert_eq!(
            schema["properties"]["binary"]["const"].as_str(),
            Some(real.binary.as_str())
        );
        assert_eq!(
            schema["properties"]["subsystem_kind"]["const"].as_str(),
            Some(real.subsystem_kind.as_str()),
            "the contract's declared subsystem_kind must match Contract::default()'s real value"
        );
        assert_eq!(
            schema["properties"]["runtime_dependency_allowed"]["const"].as_bool(),
            Some(real.runtime_dependency_allowed)
        );

        let declared_commands: std::collections::BTreeSet<String> =
            schema["properties"]["commands"]["items"]["enum"]
                .as_array()
                .expect("commands.items.enum must be a JSON array")
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .expect("every command must be a string")
                        .to_owned()
                })
                .collect();
        let real_commands: std::collections::BTreeSet<String> = real.commands.into_iter().collect();
        assert_eq!(
            declared_commands, real_commands,
            "the CLI contract's declared command set must exactly match the real \
             Contract::default() command set -- a subcommand present in one but not the other is \
             exactly the drift this permanent check exists to catch"
        );
    }

    // `.atlas/evidence/verification/duplicate-classification-logic-swept-clean.json`: five
    // separate instances of the same defect class -- a small classification/helper function
    // copy-pasted into a second file (sometimes under a different name), with nothing to stop the
    // two copies silently drifting apart on a future edit to only one -- were found and fixed by a
    // one-off script this session. This test formalizes that script as a permanent, rerunning
    // regression check, per `.atlas/contracts/RECURSIVE-SELF-CENSUS.md`'s own preference for
    // machine-readable, durable evidence over a script that runs once and is discarded.
    //
    // Extracts every top-level `fn`'s brace-matched body from every `.rs` file in the four
    // workspace crates (skipping file names containing "test" and everything from a file's first
    // `#[cfg(test)]` onward, so real test fixtures/corpora are never flagged), normalizes
    // whitespace, and fails if any exact body text (long enough to be a real finding, not a
    // trivial one-liner) appears in more than one file. Deliberately coarse and over-inclusive in
    // one direction only: a byte-matching brace counter can be confused by an unbalanced brace
    // inside a string/comment, but this repository's own source has none (verified: this test
    // currently passes cleanly), and any future false positive is a loud, investigable test
    // failure, never a silent miss.
    mod duplicate_function_body_sweep {
        use std::path::{Path, PathBuf};

        fn workspace_root() -> PathBuf {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .canonicalize()
                .expect("workspace root must exist")
        }

        fn rust_files_under(dir: &Path, out: &mut Vec<PathBuf>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                let path = entry.path();
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if file_type.is_dir() {
                    rust_files_under(&path, out);
                } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                    out.push(path);
                }
            }
        }

        /// Every top-level `fn <name> ... { ... }` body in `text` (brace-matched from the first
        /// `{` after the `fn` keyword), whitespace-normalized to a single-spaced string. Only
        /// scans up to `text`'s first `#[cfg(test)]` occurrence, if any.
        fn top_level_fn_bodies(text: &str) -> Vec<String> {
            let scan_end = text.find("#[cfg(test)]").unwrap_or(text.len());
            let scan_text = &text[..scan_end];
            let bytes = scan_text.as_bytes();
            let mut bodies = Vec::new();
            let mut index = 0;
            while let Some(offset) = scan_text[index..].find("fn ") {
                let fn_at = index + offset;
                // Require a preceding word boundary so this never matches inside an identifier
                // merely ending in "fn " (e.g. none realistically exist, but stay precise).
                let boundary_ok = fn_at == 0
                    || !bytes[fn_at - 1].is_ascii_alphanumeric() && bytes[fn_at - 1] != b'_';
                let Some(brace_start) = scan_text[fn_at..].find('{') else {
                    break;
                };
                let brace_start = fn_at + brace_start;
                // A `;` before the first `{` means this "fn " was a trait method signature with
                // no body, or occurred inside a type/string this scan doesn't need to handle
                // specially -- either way, skip past it without counting a body.
                let semi_before_brace = scan_text[fn_at..brace_start].find(';');
                if !boundary_ok || semi_before_brace.is_some() {
                    index = fn_at + 3;
                    continue;
                }
                let mut depth = 0usize;
                let mut i = brace_start;
                while i < bytes.len() {
                    match bytes[i] {
                        b'{' => depth += 1,
                        b'}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
                if depth == 0 && i < bytes.len() {
                    let body = &scan_text[brace_start..=i];
                    let normalized = body.split_whitespace().collect::<Vec<_>>().join(" ");
                    if normalized.len() >= 60 {
                        bodies.push(normalized);
                    }
                    index = i + 1;
                } else {
                    // Unbalanced (should not happen for real Rust source) -- stop scanning this
                    // file rather than loop on a broken byte offset.
                    break;
                }
            }
            bodies
        }

        /// Exact, whitespace-normalized function bodies this sweep must NOT flag, each with its
        /// own justification -- deliberately narrow (an exact string match, not a pattern), so a
        /// future edit that changes the body even slightly makes the exemption stop matching and
        /// the sweep re-evaluate that spot fresh, rather than silently widening what it excuses.
        ///
        /// Unlike every real finding this sweep and its predecessor script already found and
        /// fixed this session (a classification/dispatch function accidentally copy-pasted into a
        /// second file), this one entry is a *coincidental* shape match between four independently
        /// meaningful concepts, not an accidental duplication of one concept:
        /// `DataFlowResolution`/`OwnershipResolution`/`PersistenceResolution`/`StateResolution`
        /// (`core::semantic::{data_flow,ownership,persistence,state}`) each document their own,
        /// genuinely different, dimension-specific meaning of "Resolved"/"Unresolved" in their own
        /// per-variant doc comments -- merging them into one shared type would trade away real
        /// type safety (today, passing an `OwnershipResolution` where a `DataFlowResolution` is
        /// expected is a compile error; a shared type would make that a silent, valid conversion)
        /// for a purely cosmetic reduction of a trivial two-arm `as_str` match, which is a much
        /// worse trade than the real fixes this sweep already produced.
        const KNOWN_ACCEPTABLE_DUPLICATE_BODIES: &[&str] = &[
            "{ match self { Self::Resolved => \"RESOLVED\", Self::Unresolved => \"UNRESOLVED\", } }",
        ];

        #[test]
        fn no_function_body_is_duplicated_verbatim_across_two_workspace_source_files() {
            let root = workspace_root();
            let mut files = Vec::new();
            for crate_dir in ["core/src", "adapter/src", "runtime/src", "apps/cli/src"] {
                rust_files_under(&root.join(crate_dir), &mut files);
            }
            assert!(
                files.len() > 20,
                "expected to find real source files under the workspace root {root:?}; found {}",
                files.len()
            );

            let mut bodies_by_text: std::collections::BTreeMap<String, Vec<PathBuf>> =
                std::collections::BTreeMap::new();
            for path in &files {
                if path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.contains("test"))
                {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(path) else {
                    continue;
                };
                for body in top_level_fn_bodies(&text) {
                    bodies_by_text.entry(body).or_default().push(path.clone());
                }
            }

            let mut violations = Vec::new();
            for (body, paths) in &bodies_by_text {
                if KNOWN_ACCEPTABLE_DUPLICATE_BODIES.contains(&body.as_str()) {
                    continue;
                }
                let unique_files: std::collections::BTreeSet<&PathBuf> = paths.iter().collect();
                if unique_files.len() > 1 {
                    violations.push(format!(
                        "identical function body appears in {} different files: {:?}\nbody: {}",
                        unique_files.len(),
                        unique_files,
                        &body[..body.len().min(160)]
                    ));
                }
            }
            assert!(
                violations.is_empty(),
                "found duplicated function bodies across workspace source files -- extract a \
                 shared function/method instead, per this session's own established fix pattern \
                 (see .atlas/evidence/verification/duplicate-classification-logic-swept-clean.json):\n\n{}",
                violations.join("\n\n")
            );
        }
    }

    // `.atlas/references/donor-corpus.toml` is this repository's own canonical donor tracker
    // (57 donors as of this writing), consulted and hand-edited repeatedly across this session's
    // donor-research generations. Every edit was verified ad hoc with a one-off
    // `python3 -c "import tomllib; ..."` shell command re-run by hand each time -- exactly the
    // "self-hosting pressure" this repository's own roadmap names: a script that runs once and is
    // discarded, per `.atlas/contracts/RECURSIVE-SELF-CENSUS.md`'s preference for durable,
    // machine-readable, permanently-rerunning evidence instead. This formalizes that check as a
    // real, permanent regression test, hand-parsed (no `toml` crate dependency exists anywhere in
    // this workspace, matching `adapter::dependency::cargo`'s own hand-rolled parsing precedent
    // rather than adding a new dependency for test-only use) rather than pulled in fresh.
    mod donor_corpus_integrity {
        use std::path::{Path, PathBuf};

        fn workspace_root() -> PathBuf {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .canonicalize()
                .expect("workspace root must exist")
        }

        struct DonorEntry {
            id: String,
            evidence: Vec<String>,
            license: Vec<String>,
            census_status: String,
            decision_status: String,
        }

        /// Hand-rolled, deliberately narrow parse: extracts only the first `id = "..."` line and
        /// the `evidence = [...]` line (confirmed single-line for every one of this file's current
        /// 57 entries) within each `[[donor]]` block. Does not attempt to parse TOML in general --
        /// exactly the same scope discipline `adapter::dependency::cargo`'s own hand-rolled parser
        /// already applies to `Cargo.lock`/`Cargo.toml`.
        /// Appends every `"..."` quoted substring found in `s` to `out`.
        fn extract_quoted_strings(s: &str, out: &mut Vec<String>) {
            let bytes = s.as_bytes();
            let mut index = 0;
            while index < bytes.len() {
                if bytes[index] == b'"' {
                    if let Some(end) = s[index + 1..].find('"') {
                        out.push(s[index + 1..index + 1 + end].to_owned());
                        index = index + 1 + end + 1;
                    } else {
                        break;
                    }
                } else {
                    index += 1;
                }
            }
        }

        /// Consumes a `key = [...]` array starting at `lines[i]` (already confirmed to start with
        /// `prefix`), which may be single-line (the common case) or span multiple lines -- one
        /// quoted path per line, closed by a bare `]` -- like the `zed`/`evidence` and
        /// `zed`/`license` entries' own arrays. Returns the parsed paths and the index of the line
        /// after the array's close.
        fn parse_quoted_array(lines: &[&str], i: usize, rest: &str) -> (Vec<String>, usize) {
            let mut paths = Vec::new();
            extract_quoted_strings(rest, &mut paths);
            let mut closed = rest.contains(']');
            let mut j = i;
            while !closed && j + 1 < lines.len() {
                j += 1;
                let next = lines[j].trim();
                extract_quoted_strings(next, &mut paths);
                closed = next.contains(']');
            }
            (paths, j + 1)
        }

        fn parse_donor_corpus(text: &str) -> Vec<DonorEntry> {
            let mut entries = Vec::new();
            let mut current_id: Option<String> = None;
            let mut current_evidence: Vec<String> = Vec::new();
            let mut current_license: Vec<String> = Vec::new();
            let mut current_census_status = String::new();
            let mut current_decision_status = String::new();
            let mut in_block = false;

            let lines: Vec<&str> = text.lines().collect();
            let mut i = 0;
            while i < lines.len() {
                let trimmed = lines[i].trim();
                if trimmed == "[[donor]]" {
                    if let Some(id) = current_id.take() {
                        entries.push(DonorEntry {
                            id,
                            evidence: std::mem::take(&mut current_evidence),
                            license: std::mem::take(&mut current_license),
                            census_status: std::mem::take(&mut current_census_status),
                            decision_status: std::mem::take(&mut current_decision_status),
                        });
                    }
                    in_block = true;
                    i += 1;
                    continue;
                }
                if !in_block {
                    i += 1;
                    continue;
                }
                if current_id.is_none()
                    && let Some(rest) = trimmed.strip_prefix("id = \"")
                    && let Some(end) = rest.find('"')
                {
                    current_id = Some(rest[..end].to_owned());
                    i += 1;
                    continue;
                }
                if let Some(rest) = trimmed.strip_prefix("census_status = \"")
                    && let Some(end) = rest.find('"')
                {
                    current_census_status = rest[..end].to_owned();
                    i += 1;
                    continue;
                }
                if let Some(rest) = trimmed.strip_prefix("decision_status = \"")
                    && let Some(end) = rest.find('"')
                {
                    current_decision_status = rest[..end].to_owned();
                    i += 1;
                    continue;
                }
                if let Some(rest) = trimmed.strip_prefix("evidence = [") {
                    let (paths, next_i) = parse_quoted_array(&lines, i, rest);
                    current_evidence = paths;
                    i = next_i;
                    continue;
                }
                // `license` is declared two ways across this file: an array of real, vendored
                // license-file paths (most donors, checked below), or a bare SPDX identifier
                // string (`license = "MIT"`, a label only, no file to verify) for donors recorded
                // without vendored license files. Only the array form is parsed here; the bare
                // string form is intentionally left unhandled (it falls through to the final
                // `i += 1` below) -- there is no path to check, and misreading it as a path list
                // would fabricate evidence this parser cannot verify.
                if let Some(rest) = trimmed.strip_prefix("license = [") {
                    let (paths, next_i) = parse_quoted_array(&lines, i, rest);
                    current_license = paths;
                    i = next_i;
                    continue;
                }
                i += 1;
            }
            if let Some(id) = current_id.take() {
                entries.push(DonorEntry {
                    id,
                    evidence: current_evidence,
                    license: current_license,
                    census_status: current_census_status,
                    decision_status: current_decision_status,
                });
            }
            entries
        }

        fn load_entries() -> Vec<DonorEntry> {
            let root = workspace_root();
            let text = std::fs::read_to_string(root.join(".atlas/references/donor-corpus.toml"))
                .expect("donor-corpus.toml must exist and be readable");
            let block_count = text
                .lines()
                .filter(|line| line.trim() == "[[donor]]")
                .count();
            let entries = parse_donor_corpus(&text);
            assert_eq!(
                entries.len(),
                block_count,
                "parser found {} DonorEntry values but the file has {} [[donor]] blocks -- the \
                 hand-rolled parser above has drifted from the file's actual format",
                entries.len(),
                block_count
            );
            entries
        }

        #[test]
        fn every_donor_has_a_unique_id() {
            let entries = load_entries();
            let mut seen = std::collections::HashSet::new();
            for entry in &entries {
                assert!(
                    !entry.id.is_empty(),
                    "a [[donor]] block has an empty or missing id"
                );
                assert!(
                    seen.insert(entry.id.clone()),
                    "duplicate donor id `{}` in donor-corpus.toml",
                    entry.id
                );
            }
        }

        #[test]
        fn every_donor_has_at_least_one_evidence_path_and_every_path_exists() {
            let root = workspace_root();
            let entries = load_entries();
            for entry in &entries {
                assert!(
                    !entry.evidence.is_empty(),
                    "donor `{}` has no evidence array -- every donor must cite at least one real \
                     evidence file (a census/provenance/genome record), never bare assertion",
                    entry.id
                );
                for path in &entry.evidence {
                    assert!(
                        root.join(path).exists(),
                        "donor `{}`'s evidence path `{}` does not exist on disk -- a dangling \
                         evidence reference is exactly the kind of unverifiable claim this \
                         repository's own donor-absorption discipline forbids",
                        entry.id,
                        path
                    );
                }
            }
        }

        /// The same dangling-reference discipline `every_donor_has_at_least_one_evidence_path_
        /// and_every_path_exists` already applies to `evidence`, applied to `license` -- a
        /// real, separate legal/provenance obligation (`.atlas/licenses/`), not previously checked
        /// at all: `DonorEntry` had no `license` field until this generation. Only the array-of-
        /// file-paths form is checked here (most donors); a bare SPDX-string `license = "MIT"`
        /// entry has no file path to verify and is correctly excluded by the parser itself, not
        /// silently skipped by this test. Confirmed clean today (73 real license paths, zero
        /// dangling) -- this closes a real, previously-unverified blind spot, not a currently-known
        /// defect.
        #[test]
        fn every_donor_license_path_that_is_a_file_reference_exists_on_disk() {
            let root = workspace_root();
            let entries = load_entries();
            let mut checked = 0usize;
            for entry in &entries {
                for path in &entry.license {
                    checked += 1;
                    assert!(
                        root.join(path).exists(),
                        "donor `{}`'s license path `{}` does not exist on disk -- a dangling \
                         license reference is exactly the kind of unverifiable legal/provenance \
                         claim this repository's own donor-absorption discipline forbids",
                        entry.id,
                        path
                    );
                }
            }
            assert!(
                checked > 0,
                "expected at least one donor to declare its license as a real file-path array \
                 (most donors do) -- zero checked means this test's own parsing has drifted"
            );
        }

        /// The reverse direction of the check above: every real, on-disk Technology Genome
        /// document must be cited by at least one donor's own `evidence` array, or it is an
        /// orphaned record no `donor-corpus.toml` entry actually points to -- invisible to anyone
        /// reading a donor's own evidence trail, even though the file itself exists. Confirmed
        /// clean today (this test was added, not because a defect was found, but because the
        /// blind spot it closes is real: nothing previously checked this direction at all), so
        /// this guards against a FUTURE genome document being written and never linked back, not
        /// a currently-known problem.
        #[test]
        fn every_genome_technology_document_is_referenced_by_some_donor() {
            let root = workspace_root();
            let genome_dir = root.join(".atlas/genome/technology");
            let entries = load_entries();
            let mut referenced: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for entry in &entries {
                for path in &entry.evidence {
                    if let Some(name) = path.strip_prefix(".atlas/genome/technology/") {
                        referenced.insert(name.to_owned());
                    }
                }
            }

            let mut genome_files: Vec<String> = std::fs::read_dir(&genome_dir)
                .expect(".atlas/genome/technology must exist and be readable")
                .filter_map(Result::ok)
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "md"))
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect();
            genome_files.sort();
            assert!(
                !genome_files.is_empty(),
                "expected at least one Technology Genome document under .atlas/genome/technology \
                 -- this session alone wrote several; an empty directory means this test's own \
                 path resolution has drifted"
            );

            for file in &genome_files {
                assert!(
                    referenced.contains(file),
                    "`.atlas/genome/technology/{file}` exists on disk but is not cited in any \
                     donor's evidence array in donor-corpus.toml -- an orphaned genome record \
                     nobody's evidence trail points back to"
                );
            }
        }

        /// A `decision_status` of bare `"PENDING"` is only an honest claim when the donor's own
        /// `census_status` says census depth genuinely never reached the point a decision could be
        /// made (`SKELETON`, or `PENDING_DEEP_CENSUS`). This session found 8 donors that violated
        /// that: their `census_status` already showed a completed (or lane-level-concluded) census
        /// -- each with a real, dated, per-mechanism disposition already recorded in its own
        /// census document (and, for the semantic-graph-language-lane donors, in a shared lane
        /// synthesis document's own `## Decision` section) -- yet `decision_status` still read the
        /// literal placeholder `"PENDING"`, silently misrepresenting an already-resolved judgment
        /// as still-open. One donor's own census doc (`duumbi.md`) explicitly names the missing
        /// step: "A coordinator must merge applicable Discoveries/target_owners into ...
        /// donor-corpus.toml separately" -- that merge was never done. This formalizes the
        /// invariant those 8 corrections restored, so a future census-complete donor can never
        /// again go unsynced silently.
        #[test]
        fn pending_decision_status_only_appears_on_a_genuinely_uncensused_donor() {
            const HONEST_PENDING_CENSUS_STATUSES: &[&str] = &["SKELETON", "PENDING_DEEP_CENSUS"];
            let entries = load_entries();
            let mut violations = Vec::new();
            for entry in &entries {
                if entry.decision_status == "PENDING"
                    && !HONEST_PENDING_CENSUS_STATUSES.contains(&entry.census_status.as_str())
                {
                    violations.push(format!(
                        "donor `{}` has decision_status = \"PENDING\" but census_status = \"{}\" \
                         -- census depth beyond {:?} means a real decision should already be \
                         recorded (in the donor's own census doc or a lane synthesis doc) and \
                         mirrored into decision_status, not left as an unsynced placeholder",
                        entry.id, entry.census_status, HONEST_PENDING_CENSUS_STATUSES
                    ));
                }
            }
            assert!(
                violations.is_empty(),
                "found donor-corpus.toml entries with a stale, unsynced PENDING decision_status:\n\n{}",
                violations.join("\n")
            );
        }
    }

    /// `.atlas/scripts/verify-donor-quarantine.sh` enforces `.atlas/contracts/
    /// DONOR-WORKBENCH-ISOLATION.md`'s invariant (no live agent-tooling-shaped path under donors).
    /// A delegated contract-vs-code cross-check found it used `find -type d`/`-type f`, which
    /// classify a symlink by its OWN type, never by what it resolves to -- so a `.claude`/
    /// `CLAUDE.md`-named SYMLINK to a real directory/file was invisible to every check, even though
    /// it is exactly as live and discoverable as a literal one. This module runs the real script
    /// (accepting an overridable donors-root argument added specifically for this test) against a
    /// scratch fixture, never against this repository's own real donor corpus, so it is a genuine
    /// permanent regression test rather than a one-off manual verification.
    #[cfg(unix)]
    mod donor_quarantine_script_symlink_detection {
        use std::{fs, os::unix::fs::symlink, path::PathBuf, process::Command};

        fn workspace_root() -> PathBuf {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .canonicalize()
                .expect("workspace root must exist")
        }

        fn scratch_dir(name: &str) -> PathBuf {
            std::env::temp_dir().join(format!(
                "atlas-donor-quarantine-test-{name}-{}",
                std::process::id()
            ))
        }

        fn run_quarantine_script(donors_root: &std::path::Path) -> std::process::Output {
            Command::new(workspace_root().join(".atlas/scripts/verify-donor-quarantine.sh"))
                .arg(donors_root)
                .output()
                .expect("verify-donor-quarantine.sh must be runnable")
        }

        #[test]
        fn a_symlinked_dot_claude_directory_is_detected_not_silently_missed() {
            let dir = scratch_dir("symlinked-dir");
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(dir.join("some-donor/real-target")).unwrap();
            symlink(
                dir.join("some-donor/real-target"),
                dir.join("some-donor/.claude"),
            )
            .unwrap();

            let output = run_quarantine_script(&dir);
            assert!(
                !output.status.success(),
                "a .claude directory that is a symlink to a real directory must be flagged, not \
                 silently treated as clean"
            );

            fs::remove_dir_all(&dir).unwrap();
        }

        #[test]
        fn a_symlinked_claude_md_file_is_detected_not_silently_missed() {
            let dir = scratch_dir("symlinked-file");
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(dir.join("some-donor")).unwrap();
            fs::write(dir.join("some-donor/real-instructions.md"), "content\n").unwrap();
            symlink(
                dir.join("some-donor/real-instructions.md"),
                dir.join("some-donor/CLAUDE.md"),
            )
            .unwrap();

            let output = run_quarantine_script(&dir);
            assert!(
                !output.status.success(),
                "a CLAUDE.md that is a symlink to a real file must be flagged, not silently \
                 treated as clean"
            );

            fs::remove_dir_all(&dir).unwrap();
        }

        #[test]
        fn a_donor_tree_with_no_quarantine_violations_still_scans_clean() {
            let dir = scratch_dir("clean");
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(dir.join("some-donor/src")).unwrap();
            fs::write(dir.join("some-donor/src/lib.rs"), "fn main() {}\n").unwrap();

            let output = run_quarantine_script(&dir);
            assert!(
                output.status.success(),
                "an ordinary donor tree with no agent-tooling-shaped paths must scan clean: {:?}",
                String::from_utf8_lossy(&output.stdout)
            );

            fs::remove_dir_all(&dir).unwrap();
        }
    }

    fn constraint_result(name: &str, passed: bool) -> ConstraintResult {
        ConstraintResult {
            name: name.into(),
            passed,
            diagnostics: Vec::new(),
            derivation: Vec::new(),
        }
    }

    #[test]
    fn no_constraint_results_never_blocks() {
        assert!(!adl_constraint_violation_blocks(&[]));
    }

    #[test]
    fn all_passing_constraint_results_never_block() {
        assert!(!adl_constraint_violation_blocks(&[
            constraint_result("A", true),
            constraint_result("B", true),
        ]));
    }

    #[test]
    fn a_single_failing_constraint_result_blocks() {
        // This is the exact defect this helper replaces: a declared constraint, invariant, or
        // materialization delta failing (passed: false) previously had no effect on
        // coding_admission at all, since only `adl.diagnostics` was inspected.
        assert!(adl_constraint_violation_blocks(&[
            constraint_result("A", true),
            constraint_result("ObservedMaterialization:WebUI", false),
        ]));
    }
}
