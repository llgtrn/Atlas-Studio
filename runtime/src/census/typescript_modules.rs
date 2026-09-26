//! The TypeScript/JavaScript module-resolution engine (G158, FULL_OSS_REPLAY R5, ADR 0073): the
//! second CALL engine for TypeScript and JavaScript.
//!
//! The syntactic extractor (`atlas.typescript.source-semantic.v1`) resolves a bare identifier
//! only to a function of its own file. This engine links modules
//! (`adapter::typescript::modules`): imports, exports, re-exports and star exports across the
//! files of the inventory, and bare specifiers naming a workspace package whose `package.json`
//! declares its `name` and `source` entry. A CALL claim whose callee is a bare identifier bound
//! once in its file, by an import this engine resolves to a module-level function, is observed
//! again under this engine's identity -- the same `record_id`, `dispatch: STATIC_RESOLVED`,
//! `callees: [that function's FunctionIdentity record]` -- so the two engines stay separately
//! attributable. Every CALL obligation of the engine is UNKNOWN: member calls, namespace imports,
//! JSX elements, dynamic imports, `require` and path aliases are outside it.

use adapter::{
    DiagnosticCode, ExtractionBatch, ExtractionDiagnostic, ExtractionInput, ObligationResult,
};
use atlas_core::{
    ArtifactDisposition, CallDispatchKind, EpistemicStatus, Evidence, EvidenceId,
    ExtractorIdentity, InventoryReport, Provenance, SemanticDimension, SemanticObservation,
    SemanticRecordHeader, stable_id,
};
use std::collections::BTreeMap;
use std::{fs, path::Path};

pub const TYPESCRIPT_MODULE_RESOLUTION_ID: &str = "atlas.resolution.typescript-modules";
pub const TYPESCRIPT_MODULE_RESOLUTION_VERSION: &str = "1";

/// The dimensions the engine is asked to evaluate.
pub const TYPESCRIPT_MODULE_RESOLUTION_DIMENSIONS: [SemanticDimension; 1] =
    [SemanticDimension::Call];

fn identity() -> ExtractorIdentity {
    ExtractorIdentity {
        id: TYPESCRIPT_MODULE_RESOLUTION_ID.into(),
        version: TYPESCRIPT_MODULE_RESOLUTION_VERSION.into(),
    }
}

/// Whether a callee spelling is a bare identifier (not a member, constructor or expression).
fn bare_identifier(spelling: &str) -> bool {
    let mut chars = spelling.chars();
    chars
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

/// The engine's batches: one per TypeScript/JavaScript artifact, each observing the CALL claims
/// whose imported callee it resolved.
pub fn resolve_typescript_modules(
    inventory: &InventoryReport,
    batches: &[ExtractionBatch],
) -> Vec<ExtractionBatch> {
    let (mut repository, mut revision) = (None, None);
    let mut claims = Vec::new();
    for batch in batches {
        if batch.extractor.id != adapter::TYPESCRIPT_SEMANTIC_EXTRACTOR_ID {
            continue;
        }
        repository.get_or_insert_with(|| batch.repository.clone());
        revision.get_or_insert_with(|| batch.revision.clone());
        for observation in &batch.observations {
            if let SemanticObservation::Call(header) = observation {
                claims.push(header);
            }
        }
    }
    let (Some(repository), Some(revision)) = (repository, revision) else {
        return Vec::new();
    };
    let root = Path::new(&inventory.root);
    let mut facts = BTreeMap::new();
    let mut artifacts = BTreeMap::new();
    let mut declared = Vec::new();
    for artifact in &inventory.artifacts {
        if artifact.disposition != ArtifactDisposition::Parsed {
            continue;
        }
        let language = artifact.language.as_deref();
        let is_manifest =
            artifact.path == "package.json" || artifact.path.ends_with("/package.json");
        if !is_manifest && !matches!(language, Some("typescript" | "javascript")) {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(&artifact.path)) else {
            continue;
        };
        if is_manifest {
            let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&text) else {
                continue;
            };
            if let (Some(name), Some(source)) = (
                manifest.get("name").and_then(|n| n.as_str()),
                manifest.get("source").and_then(|s| s.as_str()),
            ) {
                declared.push((artifact.path.clone(), name.to_owned(), source.to_owned()));
            }
            continue;
        }
        artifacts.insert(artifact.path.clone(), artifact.id.clone());
        let input = ExtractionInput {
            repository: repository.clone(),
            revision: revision.clone(),
            artifact: artifact.id.clone(),
            artifact_path: artifact.path.clone(),
            source_text: text,
            content_fingerprint: None,
            source_frontend_id: String::new(),
            language: language.unwrap_or_default().to_owned(),
            build_profile: None,
            scope_policy: None,
            requested_dimensions: super::extraction::ALL_SEMANTIC_DIMENSIONS.to_vec(),
        };
        if let Some(module) = adapter::module_facts(&input) {
            facts.insert(artifact.path.clone(), module);
        }
    }
    let packages = adapter::workspace_packages(&declared);
    let bindings: BTreeMap<(String, String), adapter::ImportBinding> =
        adapter::resolve_imports(&facts, &packages)
            .into_iter()
            .map(|b| ((b.path.clone(), b.local.clone()), b))
            .collect();

    let extractor = identity();
    let mut calls: BTreeMap<String, (Vec<SemanticObservation>, Vec<Evidence>)> = BTreeMap::new();
    for claim in claims {
        let subject = &claim.subject;
        let Some(spelling) = subject.callee_spelling.as_deref() else {
            continue;
        };
        if subject.dispatch != CallDispatchKind::Unresolved || !bare_identifier(spelling) {
            continue;
        }
        let path = &subject.span.path;
        let Some(binding) = bindings.get(&(path.clone(), spelling.to_owned())) else {
            continue;
        };
        let entry = calls.entry(path.clone()).or_default();
        let evidence_id = EvidenceId::new(stable_id(
            "evidence",
            &format!(
                "{TYPESCRIPT_MODULE_RESOLUTION_ID}:{}",
                claim.record_id.as_str()
            ),
        ));
        entry.1.push(Evidence {
            id: evidence_id.as_str().to_owned(),
            kind: "NAME_RESOLUTION".into(),
            path: path.clone(),
            summary: format!(
                "`{spelling}` at {path}:{}:{} is an import resolved through {} to a module-level function",
                subject.span.line,
                subject.span.column,
                binding.chain.join(" -> ")
            ),
            revision: Some(revision.clone()),
        });
        let mut resolved = subject.clone();
        resolved.dispatch = CallDispatchKind::StaticResolved;
        resolved.callees = vec![binding.function.clone()];
        let observation = SemanticObservation::Call(SemanticRecordHeader {
            record_id: claim.record_id.clone(),
            dimension: SemanticDimension::Call,
            status: EpistemicStatus::Derived,
            scope: claim.scope.clone(),
            repository: repository.clone(),
            revision: revision.clone(),
            extractor: extractor.clone(),
            evidence_refs: vec![evidence_id],
            provenance: Provenance {
                source_path: path.clone(),
                source_revision: Some(revision.clone()),
                extractor: TYPESCRIPT_MODULE_RESOLUTION_ID.into(),
                content_hash: None,
                span: Some(format!("{}:{}", subject.span.line, subject.span.column)),
            },
            subject: resolved,
        });
        assert!(observation.is_dimension_consistent());
        entry.0.push(observation);
    }

    let mut out = Vec::new();
    for (path, artifact) in artifacts {
        let (observations, evidence) = calls.remove(&path).unwrap_or_default();
        let diagnostic = ExtractionDiagnostic::new(
            DiagnosticCode::IncompleteAnalysis,
            Some(SemanticDimension::Call),
            format!(
                "{TYPESCRIPT_MODULE_RESOLUTION_ID} resolves calls of bare identifiers imported by name or default through exports, re-exports and unambiguous star exports to module-level functions, across relative specifiers and workspace packages declaring a `source` entry ({path}); member calls, namespace imports, JSX elements, dynamic imports, require, path aliases and package export maps are outside it"
            ),
        );
        let obligation = ObligationResult::unknown_with_observations(
            SemanticDimension::Call,
            observations.iter().map(|o| o.record_id().clone()).collect(),
            evidence
                .iter()
                .map(|e| EvidenceId::new(e.id.clone()))
                .collect(),
            diagnostic.id.clone(),
        );
        out.push(ExtractionBatch {
            extractor: extractor.clone(),
            repository: repository.clone(),
            revision: revision.clone(),
            artifact,
            input_fingerprint: stable_id(
                "resolution-input",
                &format!(
                    "{TYPESCRIPT_MODULE_RESOLUTION_ID}:{}:{path}",
                    revision.value
                ),
            ),
            observations,
            evidence,
            obligations: vec![obligation],
            diagnostics: vec![diagnostic],
        });
    }
    out
}

#[cfg(test)]
mod tests;
