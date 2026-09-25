//! The Rust path-call resolution engine (G75): the second CALL engine.
//!
//! `adapter::resolve_path_calls` (rust-analyzer #4's `hir-def` name resolution, absorbed natively)
//! resolves every path call of every workspace crate target; this module lays out those targets
//! from the Cargo manifests in the inventory, runs the resolver, and turns each resolution into an
//! observation of the SAME CALL claim the syntactic extractor made at that anchor -- same
//! `record_id`, `dispatch: STATIC_RESOLVED`, `callees: [the callee's FunctionIdentity record]` --
//! under its own extractor identity, so the two engines stay separately attributable
//! (`.atlas/contracts/SEMANTIC-EXTRACTION.md#multi-engine-extraction`). The callee identity is
//! the syntactic extractor's own FunctionIdentity record for the definition the resolver found,
//! never a re-derived one.
//!
//! The engine evaluates only CALL, and only path calls: its obligation is UNKNOWN (method calls,
//! closure bodies and macro arguments are outside it), and a resolution that cannot be attached --
//! no syntactic claim at the anchor, or no FunctionIdentity for the definition -- is a diagnosed
//! disagreement, never a fabricated record.

use adapter::join_path as join;
use adapter::{
    CrateInput, DiagnosticCode, ExtractionBatch, ExtractionDiagnostic, ObligationResult,
    PathCallOutcome,
};
use atlas_core::{
    ArtifactDisposition, CallDispatchKind, EpistemicStatus, Evidence, EvidenceId,
    ExtractorIdentity, InventoryReport, Provenance, SemanticDimension, SemanticObservation,
    SemanticRecordHeader, SemanticRecordId, stable_id,
};
use std::collections::BTreeMap;
use std::{fs, path::Path};

pub const RUST_PATH_RESOLUTION_ID: &str = "atlas.resolution.rust-paths";
pub const RUST_PATH_RESOLUTION_VERSION: &str = "1";

/// The dimensions the resolution engine is asked to evaluate (accounting closure is checked
/// against these, not against every dimension).
pub const RUST_PATH_RESOLUTION_DIMENSIONS: [SemanticDimension; 1] = [SemanticDimension::Call];

fn identity() -> ExtractorIdentity {
    ExtractorIdentity {
        id: RUST_PATH_RESOLUTION_ID.into(),
        version: RUST_PATH_RESOLUTION_VERSION.into(),
    }
}

/// Every workspace crate target (library, binaries, build script) with the extern-prelude names
/// of the workspace libraries it may use, from the package manifests in `sources`' inventory.
pub fn crate_targets(
    manifests: &BTreeMap<String, String>,
    sources: &BTreeMap<String, String>,
) -> Vec<CrateInput> {
    struct Package {
        dir: String,
        targets: adapter::ManifestTargets,
    }
    let packages: Vec<Package> = manifests
        .iter()
        .filter_map(|(path, text)| {
            let dir = path
                .strip_suffix("Cargo.toml")?
                .trim_end_matches('/')
                .to_owned();
            let targets =
                adapter::manifest_targets(text, |rel| sources.contains_key(&join(&dir, rel)))?;
            Some(Package { dir, targets })
        })
        .collect();

    let mut crates: Vec<CrateInput> = Vec::new();
    let mut libs: BTreeMap<String, usize> = BTreeMap::new();
    let mut kinds: Vec<(usize, bool)> = Vec::new(); // (package index, is build script)
    for (index, package) in packages.iter().enumerate() {
        if let Some(lib) = &package.targets.lib {
            libs.insert(package.targets.package.clone(), crates.len());
            crates.push(CrateInput {
                root: join(&package.dir, lib),
                externs: BTreeMap::new(),
            });
            kinds.push((index, false));
        }
    }
    for (index, package) in packages.iter().enumerate() {
        for bin in &package.targets.bins {
            crates.push(CrateInput {
                root: join(&package.dir, bin),
                externs: BTreeMap::new(),
            });
            kinds.push((index, false));
        }
        if let Some(build) = &package.targets.build {
            crates.push(CrateInput {
                root: join(&package.dir, build),
                externs: BTreeMap::new(),
            });
            kinds.push((index, true));
        }
    }
    for (krate, (package_index, is_build)) in kinds.into_iter().enumerate() {
        let package = &packages[package_index].targets;
        let mut externs = BTreeMap::new();
        for (key, crate_name, role) in &package.dependencies {
            let wanted = if is_build {
                *role == atlas_core::DependencyRole::Build
            } else {
                *role != atlas_core::DependencyRole::Build
            };
            if let (true, Some(&lib)) = (wanted, libs.get(crate_name)) {
                externs.insert(key.replace('-', "_"), lib);
            }
        }
        // A binary (never the library itself or its build script) sees its own package's library.
        if !is_build
            && let Some(&lib) = libs.get(&package.package)
            && lib != krate
        {
            externs.insert(package.package.replace('-', "_"), lib);
        }
        crates[krate].externs = externs;
    }
    crates
}

/// The resolution engine's batches: one per Rust artifact (a file no crate root reaches says it
/// was not evaluated), each observing the CALL claims it resolved.
pub fn resolve_rust_path_calls(
    inventory: &InventoryReport,
    batches: &[ExtractionBatch],
) -> Vec<ExtractionBatch> {
    let root = Path::new(&inventory.root);
    let mut sources = BTreeMap::new();
    let mut manifests = BTreeMap::new();
    let mut artifacts = BTreeMap::new();
    for artifact in &inventory.artifacts {
        if artifact.disposition != ArtifactDisposition::Parsed {
            continue;
        }
        let is_manifest = artifact.path == "Cargo.toml" || artifact.path.ends_with("/Cargo.toml");
        let is_rust = artifact.language.as_deref() == Some("rust");
        if !is_manifest && !is_rust {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(&artifact.path)) else {
            continue;
        };
        if is_manifest {
            manifests.insert(artifact.path.clone(), text);
        } else {
            artifacts.insert(artifact.path.clone(), artifact.id.clone());
            sources.insert(artifact.path.clone(), text);
        }
    }
    let crates = crate_targets(&manifests, &sources);
    let workspace = adapter::resolve_workspace(&crates, &sources);
    let resolutions = &workspace.calls;

    // The syntactic extractor's claims: CALL sites by anchor, FunctionIdentity by item start.
    let mut claims = BTreeMap::new();
    let mut functions = BTreeMap::new();
    let (mut repository, mut revision) = (None, None);
    for batch in batches {
        if batch.extractor.id != adapter::RUST_SEMANTIC_EXTRACTOR_ID {
            continue;
        }
        repository.get_or_insert_with(|| batch.repository.clone());
        revision.get_or_insert_with(|| batch.revision.clone());
        for observation in &batch.observations {
            match observation {
                SemanticObservation::Call(header) => {
                    let span = &header.subject.span;
                    claims.insert((span.path.clone(), span.line, span.column), header);
                }
                SemanticObservation::FunctionIdentity(header) => {
                    let span = &header.subject.span;
                    functions.insert(
                        (
                            span.path.clone(),
                            span.line,
                            span.column,
                            header.subject.symbol.name.clone(),
                        ),
                        header.record_id.clone(),
                    );
                }
                _ => {}
            }
        }
    }
    let (Some(repository), Some(revision)) = (repository, revision) else {
        return Vec::new();
    };

    let extractor = identity();
    let mut per_artifact: BTreeMap<String, (Vec<SemanticObservation>, Vec<Evidence>, usize)> =
        artifacts
            .keys()
            .map(|path| (path.clone(), Default::default()))
            .collect();
    for resolution in resolutions {
        let entry = per_artifact.entry(resolution.path.clone()).or_default();
        let PathCallOutcome::Resolved(target) = &resolution.outcome else {
            continue;
        };
        let claim = claims.get(&(resolution.path.clone(), resolution.line, resolution.column));
        let callee = functions.get(&(
            target.path.clone(),
            target.line,
            target.column,
            target.name.clone(),
        ));
        let (Some(claim), Some(callee)) = (claim, callee) else {
            entry.2 += 1;
            continue;
        };
        let evidence_id = EvidenceId::new(stable_id(
            "evidence",
            &format!("{RUST_PATH_RESOLUTION_ID}:{}", claim.record_id.as_str()),
        ));
        entry.1.push(Evidence {
            id: evidence_id.as_str().to_owned(),
            kind: "NAME_RESOLUTION".into(),
            path: resolution.path.clone(),
            summary: format!(
                "resolved `{}` at {}:{}:{} to `{}` at {}:{}:{}",
                resolution.callee,
                resolution.path,
                resolution.line,
                resolution.column,
                target.name,
                target.path,
                target.line,
                target.column
            ),
            revision: Some(revision.clone()),
        });
        let mut subject = claim.subject.clone();
        subject.dispatch = CallDispatchKind::StaticResolved;
        subject.callees = vec![callee.clone()];
        let header = SemanticRecordHeader {
            record_id: claim.record_id.clone(),
            dimension: SemanticDimension::Call,
            status: EpistemicStatus::Derived,
            scope: claim.scope.clone(),
            repository: repository.clone(),
            revision: revision.clone(),
            extractor: extractor.clone(),
            evidence_refs: vec![evidence_id],
            provenance: Provenance {
                source_path: resolution.path.clone(),
                source_revision: Some(revision.clone()),
                extractor: RUST_PATH_RESOLUTION_ID.into(),
                content_hash: None,
                span: Some(format!("{}:{}", resolution.line, resolution.column)),
            },
            subject,
        };
        let observation = SemanticObservation::Call(header);
        assert!(observation.is_dimension_consistent());
        entry.0.push(observation);
    }

    let mut out = Vec::new();
    for (path, (observations, evidence, unattached)) in per_artifact {
        let Some(artifact) = artifacts.get(&path) else {
            continue;
        };
        let scope = if workspace.reached.contains(&path) {
            format!(
                "{RUST_PATH_RESOLUTION_ID} resolves path calls only ({path}); method calls, closure bodies and macro arguments are outside it"
            )
        } else {
            format!(
                "{RUST_PATH_RESOLUTION_ID} did not evaluate {path}: no Cargo target's module tree reaches it"
            )
        };
        let mut diagnostics = vec![ExtractionDiagnostic::new(
            DiagnosticCode::IncompleteAnalysis,
            Some(SemanticDimension::Call),
            scope,
        )];
        if unattached > 0 {
            diagnostics.push(ExtractionDiagnostic::new(
                DiagnosticCode::IncompleteAnalysis,
                Some(SemanticDimension::Call),
                format!(
                    "{unattached} resolved path call(s) in {path} match no syntactic CALL claim or no FunctionIdentity (engine disagreement)"
                ),
            ));
        }
        let mut obligation = ObligationResult::unknown_with_observations(
            SemanticDimension::Call,
            observations
                .iter()
                .map(|o| o.record_id().clone())
                .collect::<Vec<SemanticRecordId>>(),
            evidence
                .iter()
                .map(|e| EvidenceId::new(e.id.clone()))
                .collect(),
            diagnostics[0].id.clone(),
        );
        obligation
            .diagnostics
            .extend(diagnostics[1..].iter().map(|d| d.id.clone()));
        out.push(ExtractionBatch {
            extractor: extractor.clone(),
            repository: repository.clone(),
            revision: revision.clone(),
            artifact: artifact.clone(),
            input_fingerprint: stable_id(
                "resolution-input",
                &format!("{RUST_PATH_RESOLUTION_ID}:{}:{path}", revision.value),
            ),
            observations,
            evidence,
            obligations: vec![obligation],
            diagnostics,
        });
    }
    out
}

#[cfg(test)]
mod tests;
