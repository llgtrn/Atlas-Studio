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

/// What a `package.json` declares about its entry, with the `rootDir`/`outDir` of the
/// `tsconfig.json` beside it (G160). A manifest without a string `name` declares no package.
fn package_declaration(
    path: &str,
    text: &str,
    tsconfigs: &BTreeMap<String, String>,
) -> Option<adapter::PackageDeclaration> {
    let manifest = serde_json::from_str::<serde_json::Value>(text).ok()?;
    let name = manifest.get("name")?.as_str()?.to_owned();
    let field = |key: &str| {
        manifest
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::to_owned)
    };
    let mut outputs: Vec<String> = ["types", "typings", "module", "main"]
        .into_iter()
        .filter_map(field)
        .collect();
    // The `.` export: a string, a `.` entry, or conditions without subpaths; every string leaf.
    fn leaves(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::String(s) => out.push(s.clone()),
            serde_json::Value::Object(map) => map.values().for_each(|v| leaves(v, out)),
            serde_json::Value::Array(items) => items.iter().for_each(|v| leaves(v, out)),
            _ => {}
        }
    }
    match manifest.get("exports") {
        Some(serde_json::Value::Object(map)) if map.keys().any(|k| k.starts_with('.')) => {
            if let Some(dot) = map.get(".") {
                leaves(dot, &mut outputs);
            }
        }
        Some(other) => leaves(other, &mut outputs),
        None => {}
    }
    let dir = path.rsplit_once('/').map_or("", |(dir, _)| dir);
    let tsconfig = if dir.is_empty() {
        "tsconfig.json".to_owned()
    } else {
        format!("{dir}/tsconfig.json")
    };
    let options = tsconfigs
        .get(&tsconfig)
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&without_comments(text)).ok())
        .and_then(|config| config.get("compilerOptions").cloned());
    let option = |key: &str| {
        options
            .as_ref()
            .and_then(|o| o.get(key))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
    };
    Some(adapter::PackageDeclaration {
        manifest: path.to_owned(),
        name,
        source: field("source"),
        outputs,
        root_dir: option("rootDir"),
        out_dir: option("outDir"),
    })
}

/// `tsconfig.json` is JSON with comments and trailing commas: both removed outside strings.
fn without_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            match c {
                '\\' => {
                    if let Some(next) = chars.next() {
                        out.push(next);
                    }
                }
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match (c, chars.peek()) {
            ('"', _) => {
                in_string = true;
                out.push(c);
            }
            ('/', Some('/')) => {
                for next in chars.by_ref() {
                    if next == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut last = ' ';
                for next in chars.by_ref() {
                    if last == '*' && next == '/' {
                        break;
                    }
                    last = next;
                }
            }
            _ => out.push(c),
        }
    }
    // Trailing commas before a closing bracket or brace.
    let mut cleaned = String::with_capacity(out.len());
    let bytes: Vec<char> = out.chars().collect();
    let mut in_string = false;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            cleaned.push(c);
            if c == '\\' && i + 1 < bytes.len() {
                cleaned.push(bytes[i + 1]);
                i += 2;
                continue;
            }
            if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
            cleaned.push(c);
        } else if c == ',' {
            let next = bytes[i + 1..].iter().find(|n| !n.is_whitespace());
            if !matches!(next, Some('}' | ']')) {
                cleaned.push(c);
            }
        } else {
            cleaned.push(c);
        }
        i += 1;
    }
    cleaned
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
    let mut manifests = BTreeMap::new();
    let mut tsconfigs = BTreeMap::new();
    for artifact in &inventory.artifacts {
        if artifact.disposition != ArtifactDisposition::Parsed {
            continue;
        }
        let language = artifact.language.as_deref();
        let file = artifact.path.rsplit('/').next().unwrap_or(&artifact.path);
        let is_manifest = file == "package.json";
        let is_tsconfig = file == "tsconfig.json";
        if !is_manifest && !is_tsconfig && !matches!(language, Some("typescript" | "javascript")) {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(&artifact.path)) else {
            continue;
        };
        if is_manifest {
            manifests.insert(artifact.path.clone(), text);
            continue;
        }
        if is_tsconfig {
            tsconfigs.insert(artifact.path.clone(), text);
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
    let declared: Vec<adapter::PackageDeclaration> = manifests
        .iter()
        .filter_map(|(path, text)| package_declaration(path, text, &tsconfigs))
        .collect();
    let packages = adapter::workspace_packages(&declared, &facts.keys().cloned().collect());
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
