//! Runtime census stage.
//!
//! Census converts admitted inventory, declared ADL and real `SemanticExtractor` output into typed
//! semantic facts while preserving provenance and explicit unsupported/unknown states. It does not
//! grant truth to model output.
//!
//! R4.3.1: extraction is no longer a post-census accounting-only sidecar. `build_census` folds
//! every `ExtractionBatch` its caller passes in directly into the `CensusReport` it returns --
//! before normalization or graph construction ever run -- so `Inventory -> SemanticExtractor ->
//! Census -> Normalize -> Reconcile -> Engineering Graph` is genuinely one path, not two
//! (`.atlas/decisions/0001-one-normalized-semantic-path.md`,
//! `.atlas/contracts/SEMANTIC-EXTRACTION.md`).

pub mod extraction;

pub use extraction::{
    ALL_SEMANTIC_DIMENSIONS, CensusExtractionAccounting, ExtractorObligationRecord,
};

use adapter::{ExtractionBatch, ObligationResult};
use atlas_core::{
    AdlCompileReport, ArtifactDisposition, ArtifactId, CensusReport, EpistemicStatus, Evidence,
    ExtractionDiagnostic, ExtractorIdentity, FunctionSignature, InventoryReport, Provenance,
    RevisionRef, SemanticDimension, SemanticFact, SemanticFactKind, SemanticObligationRecord,
    SemanticObservation, SemanticScope, SourceReport, TypedClosureAccounting, stable_id,
};
use std::{collections::BTreeMap, path::Path};

fn fact_id(seed: &str) -> String {
    stable_id("semantic-fact", seed)
}

fn scoped_name(scope: &SemanticScope, name: &str) -> String {
    if scope.segments.is_empty() {
        name.to_owned()
    } else {
        format!("{}::{}", scope.join(), name)
    }
}

fn function_signature_summary(signature: &FunctionSignature) -> String {
    let params = signature
        .parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, parameter.type_identity.name))
        .collect::<Vec<_>>()
        .join(", ");
    let return_type = signature
        .return_type
        .as_ref()
        .map(|type_identity| type_identity.name.clone())
        .unwrap_or_else(|| "()".to_owned());
    let asyncness = if signature.is_async { "async " } else { "" };
    let unsafety = if signature.is_unsafe { "unsafe " } else { "" };
    format!("{asyncness}{unsafety}fn({params}) -> {return_type}")
}

/// Projects one real typed `SemanticObservation` (produced by a `SemanticExtractor`, never
/// invented here) into the bootstrap `SemanticFact` triple envelope that `normalize`/graph
/// construction already consume. `None` for dimensions with no typed kernel record and no
/// extractor producing them yet (CALL/CONTROL_FLOW/DATA_FLOW/STATE/EFFECT -- R4.4+): there is
/// nothing to project because nothing was observed.
///
/// This is a lossy compatibility projection, not a second source of truth: the typed
/// `SemanticObservation` (retrievable from the `ExtractionBatch`es a caller passed to
/// `build_census`) remains the canonical record this fact is derived *from*
/// (`.atlas/contracts/SEMANTIC-EXTRACTION.md#r4-acceptance-matrix`: "SemanticFact remains a
/// compatibility envelope, not the only semantic type system"). `TypeIdentity.canonical` never
/// participates here -- only source spelling ever reaches `object`, never a fabricated
/// compiler-resolved identity.
fn semantic_observation_fact(observation: &SemanticObservation) -> Option<SemanticFact> {
    match observation {
        SemanticObservation::Symbol(header) => Some(SemanticFact {
            id: fact_id(&format!("extraction:symbol:{}", header.record_id.as_str())),
            kind: SemanticFactKind::Symbol,
            status: header.status,
            subject: header.record_id.as_str().to_owned(),
            predicate: "declares_symbol".into(),
            object: scoped_name(&header.subject.scope, &header.subject.name),
            provenance: header.provenance.clone(),
        }),
        SemanticObservation::Type(header) => Some(SemanticFact {
            id: fact_id(&format!("extraction:type:{}", header.record_id.as_str())),
            kind: SemanticFactKind::Type,
            status: header.status,
            subject: header.record_id.as_str().to_owned(),
            predicate: "type_spelling".into(),
            object: header.subject.name.clone(),
            provenance: header.provenance.clone(),
        }),
        SemanticObservation::FunctionIdentity(header) => Some(SemanticFact {
            id: fact_id(&format!(
                "extraction:function-identity:{}",
                header.record_id.as_str()
            )),
            kind: SemanticFactKind::FunctionIdentity,
            status: header.status,
            subject: header.record_id.as_str().to_owned(),
            predicate: "declares_function".into(),
            object: scoped_name(&header.subject.scope, &header.subject.symbol.name),
            provenance: header.provenance.clone(),
        }),
        SemanticObservation::FunctionSignature(header) => Some(SemanticFact {
            id: fact_id(&format!(
                "extraction:function-signature:{}",
                header.record_id.as_str()
            )),
            kind: SemanticFactKind::FunctionSignature,
            status: header.status,
            subject: header.record_id.as_str().to_owned(),
            predicate: "function_signature".into(),
            object: function_signature_summary(&header.subject),
            provenance: header.provenance.clone(),
        }),
        SemanticObservation::Call(_)
        | SemanticObservation::ControlFlow(_)
        | SemanticObservation::DataFlow(_)
        | SemanticObservation::State(_)
        | SemanticObservation::Effect(_) => None,
    }
}

/// One requested dimension's obligation status for one (artifact, extractor) pair, made visible
/// per-artifact in the canonical census rather than only as a single repository-wide summary
/// (see `CensusReport.coverage`'s own doc comment).
fn obligation_status_fact(
    artifact_path: &str,
    artifact: &ArtifactId,
    extractor: &ExtractorIdentity,
    revision: &RevisionRef,
    obligation: &ObligationResult,
) -> SemanticFact {
    SemanticFact {
        id: fact_id(&format!(
            "extraction:obligation:{}:{}:{}:{}",
            artifact_path,
            extractor.id,
            extractor.version,
            obligation.dimension.as_str()
        )),
        kind: SemanticFactKind::SemanticObligation,
        status: obligation.status,
        subject: artifact.as_str().to_owned(),
        predicate: obligation.dimension.as_str().to_ascii_lowercase(),
        object: obligation.status.as_str().into(),
        provenance: Provenance {
            source_path: artifact_path.to_owned(),
            source_revision: Some(revision.clone()),
            extractor: extractor.id.clone(),
            content_hash: None,
            span: None,
        },
    }
}

/// Most-informative-wins precedence used only to collapse per-artifact statuses into
/// `CensusReport.coverage`'s single repository-wide summary value per dimension. Never used to
/// pick a "winning" observation, discard disagreement, or synthesize `CONFLICT` -- every
/// per-artifact status survives untouched in `facts` regardless of this ranking
/// (`.atlas/contracts/NORMALIZATION.md#conflict-handling`).
fn status_rank(status: EpistemicStatus) -> u8 {
    match status {
        EpistemicStatus::Unsupported => 0,
        EpistemicStatus::Ignored => 1,
        EpistemicStatus::Unknown => 2,
        EpistemicStatus::Conflict => 3,
        EpistemicStatus::Observed
        | EpistemicStatus::Declared
        | EpistemicStatus::Derived
        | EpistemicStatus::Inferred
        | EpistemicStatus::Hypothesis => 4,
    }
}

fn source_provenance(path: &str) -> Provenance {
    let extractor = adapter::resolve_source_frontend(Path::new(path))
        .map(|matched| matched.frontend_id.to_owned())
        .unwrap_or_else(|| "atlas.inventory.v1".to_owned());
    Provenance {
        source_path: path.to_owned(),
        source_revision: None,
        extractor,
        content_hash: None,
        span: None,
    }
}

fn adl_provenance(path: &str, line: usize, column: usize) -> Provenance {
    Provenance {
        source_path: path.to_owned(),
        source_revision: None,
        extractor: "atlas.adl.compiler.v1".into(),
        content_hash: None,
        span: Some(format!("{line}:{column}")),
    }
}

fn disposition_status(disposition: &ArtifactDisposition) -> EpistemicStatus {
    match disposition {
        ArtifactDisposition::Parsed
        | ArtifactDisposition::BinaryDescribed
        | ArtifactDisposition::Generated
        | ArtifactDisposition::ExternalizedWithEvidence => EpistemicStatus::Observed,
        ArtifactDisposition::IgnoredByExplicitPolicy => EpistemicStatus::Ignored,
        ArtifactDisposition::Unsupported => EpistemicStatus::Unsupported,
        ArtifactDisposition::Unknown => EpistemicStatus::Unknown,
    }
}

pub fn build_census(
    inventory: &InventoryReport,
    source: &SourceReport,
    adl: &AdlCompileReport,
    extraction_batches: &[ExtractionBatch],
) -> CensusReport {
    let mut facts = Vec::new();

    for artifact in &inventory.artifacts {
        let subject = artifact.id.as_str().to_owned();
        facts.push(SemanticFact {
            id: fact_id(&format!("artifact:{}:disposition", artifact.path)),
            kind: SemanticFactKind::SourceArtifact,
            status: disposition_status(&artifact.disposition),
            subject: subject.clone(),
            predicate: "disposition".into(),
            object: artifact.disposition.as_str().into(),
            provenance: source_provenance(&artifact.path),
        });

        if let Some(language) = &artifact.language {
            facts.push(SemanticFact {
                id: fact_id(&format!("artifact:{}:language:{language}", artifact.path)),
                kind: SemanticFactKind::SourceArtifact,
                status: EpistemicStatus::Observed,
                subject,
                predicate: "language".into(),
                object: language.clone(),
                provenance: source_provenance(&artifact.path),
            });
        }
    }

    for node in &adl.ir.declared.nodes {
        facts.push(SemanticFact {
            id: fact_id(&format!("declared-node:{}:{}", node.id, node.node_kind)),
            kind: SemanticFactKind::DeclaredNode,
            status: EpistemicStatus::Declared,
            subject: node.name.clone(),
            predicate: "node_kind".into(),
            object: node.node_kind.clone(),
            provenance: adl_provenance(&node.span.path, node.span.line, node.span.column),
        });
    }

    for edge in &adl.ir.declared.edges {
        facts.push(SemanticFact {
            id: fact_id(&format!(
                "declared-edge:{}:{}:{}",
                edge.from, edge.relation, edge.to
            )),
            kind: SemanticFactKind::DeclaredEdge,
            status: EpistemicStatus::Declared,
            subject: edge.from.clone(),
            predicate: edge.relation.clone(),
            object: edge.to.clone(),
            provenance: adl_provenance(&edge.span.path, edge.span.line, edge.span.column),
        });
    }

    for binding in &adl.ir.declared.bindings {
        facts.push(SemanticFact {
            id: fact_id(&format!(
                "binding:{}:{}:{}",
                binding.consumer, binding.capability, binding.provider
            )),
            kind: SemanticFactKind::Binding,
            status: EpistemicStatus::Declared,
            subject: binding.consumer.clone(),
            predicate: "binds_to".into(),
            object: binding.provider.clone(),
            provenance: adl_provenance(&binding.span.path, binding.span.line, binding.span.column),
        });
        facts.push(SemanticFact {
            id: fact_id(&format!("binding:{}:capability", binding.name)),
            kind: SemanticFactKind::Binding,
            status: EpistemicStatus::Declared,
            subject: binding.name.clone(),
            predicate: "capability".into(),
            object: binding.capability.clone(),
            provenance: adl_provenance(&binding.span.path, binding.span.line, binding.span.column),
        });
    }

    for constraint in &adl.ir.declared.constraints {
        facts.push(SemanticFact {
            id: fact_id(&format!("constraint:{}", constraint.name)),
            kind: SemanticFactKind::Constraint,
            status: EpistemicStatus::Declared,
            subject: constraint.name.clone(),
            predicate: "declared".into(),
            object: "constraint".into(),
            provenance: adl_provenance(
                &constraint.span.path,
                constraint.span.line,
                constraint.span.column,
            ),
        });
    }

    for invariant in &adl.ir.declared.invariants {
        facts.push(SemanticFact {
            id: fact_id(&format!("invariant:{}", invariant.name)),
            kind: SemanticFactKind::Invariant,
            status: EpistemicStatus::Declared,
            subject: invariant.name.clone(),
            predicate: "declared".into(),
            object: "invariant".into(),
            provenance: adl_provenance(
                &invariant.span.path,
                invariant.span.line,
                invariant.span.column,
            ),
        });
    }

    for result in &adl.constraint_results {
        facts.push(SemanticFact {
            id: fact_id(&format!(
                "constraint-result:{}:{}",
                result.name, result.passed
            )),
            kind: SemanticFactKind::ConstraintResult,
            status: EpistemicStatus::Derived,
            subject: result.name.clone(),
            predicate: "passed".into(),
            object: result.passed.to_string(),
            provenance: Provenance {
                source_path: ".atlas/declared".into(),
                source_revision: None,
                extractor: "atlas.adl.constraint-evaluator.v1".into(),
                content_hash: None,
                span: None,
            },
        });
    }

    for delta in &adl.deltas {
        facts.push(SemanticFact {
            id: fact_id(&format!("diagnostic:{}:{}", delta.code, delta.subject)),
            kind: SemanticFactKind::Diagnostic,
            status: EpistemicStatus::Derived,
            subject: delta.subject.clone(),
            predicate: delta.code.clone(),
            object: delta.message.clone(),
            provenance: Provenance {
                source_path: ".atlas/declared".into(),
                source_revision: None,
                extractor: "atlas.adl.delta.v1".into(),
                content_hash: None,
                span: None,
            },
        });
    }

    for transform in &adl.ir.declared.transforms {
        facts.push(SemanticFact {
            id: fact_id(&format!("transform:{}", transform.name)),
            kind: SemanticFactKind::Transform,
            status: EpistemicStatus::Declared,
            subject: transform.name.clone(),
            predicate: "declared".into(),
            object: "transform".into(),
            provenance: adl_provenance(
                &transform.span.path,
                transform.span.line,
                transform.span.column,
            ),
        });
    }

    for materialization in &adl.ir.declared.materializations {
        facts.push(SemanticFact {
            id: fact_id(&format!(
                "materialization:{}:{}",
                materialization.target, materialization.path
            )),
            kind: SemanticFactKind::Materialization,
            status: EpistemicStatus::Declared,
            subject: materialization.target.clone(),
            predicate: "materializes_at".into(),
            object: materialization.path.clone(),
            provenance: adl_provenance(
                &materialization.span.path,
                materialization.span.line,
                materialization.span.column,
            ),
        });
    }

    let parsed_artifacts = source.files_total;

    // R4.3.2: retain the real `SemanticExtractor` output LOSSLESSLY -- `typed_semantic_records`/
    // `evidence`/`diagnostics` are the canonical carriers; `facts` (built below, from
    // `typed_semantic_records`) is a derived compatibility projection, never populated
    // independently from the raw batches (`.atlas/contracts/SEMANTIC-EXTRACTION.md#r4-acceptance-matrix`).
    // `path_by_artifact` recovers each artifact's human path for provenance -- `ExtractionBatch`
    // itself only carries the opaque `ArtifactId`.
    let path_by_artifact: BTreeMap<&str, &str> = inventory
        .artifacts
        .iter()
        .map(|artifact| (artifact.id.as_str(), artifact.path.as_str()))
        .collect();
    let mut dimension_summary: BTreeMap<SemanticDimension, EpistemicStatus> = BTreeMap::new();
    let mut typed_semantic_records: Vec<SemanticObservation> = Vec::new();
    let mut evidence_by_id: BTreeMap<String, Evidence> = BTreeMap::new();
    let mut diagnostics_by_id: BTreeMap<String, ExtractionDiagnostic> = BTreeMap::new();
    let mut typed_obligations: Vec<SemanticObligationRecord> = Vec::new();

    for batch in extraction_batches {
        let artifact_path = path_by_artifact
            .get(batch.artifact.as_str())
            .copied()
            .unwrap_or_default();

        typed_semantic_records.extend(batch.observations.iter().cloned());
        for item in &batch.evidence {
            evidence_by_id.insert(item.id.clone(), item.clone());
        }
        for diagnostic in &batch.diagnostics {
            diagnostics_by_id.insert(diagnostic.id.clone(), diagnostic.clone());
        }

        for obligation in &batch.obligations {
            facts.push(obligation_status_fact(
                artifact_path,
                &batch.artifact,
                &batch.extractor,
                &batch.revision,
                obligation,
            ));

            // R4.3.3: the canonical typed obligation ledger, so a Census-only reader can answer
            // "which extractor evaluated this dimension, for which artifact, at which revision,
            // with what status, backed by which observations/evidence/diagnostics" without the
            // original transient `ExtractionBatch`es (`.atlas/contracts/CENSUS-COMPLETENESS.md`).
            typed_obligations.push(SemanticObligationRecord::new(
                batch.artifact.clone(),
                batch.repository.clone(),
                batch.revision.clone(),
                batch.extractor.clone(),
                obligation.dimension,
                obligation.status,
                obligation.observation_ids.clone(),
                obligation.evidence_refs.clone(),
                obligation.diagnostics.clone(),
            ));

            let summary = dimension_summary
                .entry(obligation.dimension)
                .or_insert(obligation.status);
            if status_rank(obligation.status) > status_rank(*summary) {
                *summary = obligation.status;
            }
        }
    }

    // Deterministic ordering: never derived from Vec insertion/HashMap/traversal order. De-duped
    // by `raw_observation_id` -- an EXACT match on claim identity, extractor, status, evidence,
    // provenance AND typed payload -- so only a genuinely identical raw observation collapses.
    // Sorting/deduping by `record_id` alone (the pre-R4.3.3 behavior) is unsafe: two independent
    // extractors reporting the SAME semantic claim share a `record_id` by design, and collapsing
    // on it would silently erase one extractor's observation
    // (`.atlas/contracts/SEMANTIC-EXTRACTION.md#multi-engine-extraction`: "Independent extractors
    // MAY analyze the same dimension. Their identities/evidence remain separate ... one extractor
    // may not overwrite another").
    typed_semantic_records.sort_by(|a, b| {
        a.record_id()
            .as_str()
            .cmp(b.record_id().as_str())
            .then_with(|| {
                a.raw_observation_id()
                    .as_str()
                    .cmp(b.raw_observation_id().as_str())
            })
    });
    typed_semantic_records.dedup_by(|a, b| a.raw_observation_id() == b.raw_observation_id());
    let evidence: Vec<Evidence> = evidence_by_id.into_values().collect();
    let diagnostics: Vec<ExtractionDiagnostic> = diagnostics_by_id.into_values().collect();

    // Obligations use the same "preserve two independent records rather than merge" discipline:
    // `obligation_id` is a coordinate identity (artifact/extractor/dimension), unique by
    // construction, so a genuine duplicate key always carries identical content; dedup_by full
    // equality still never silently drops a content-differing collision if that invariant is ever
    // violated.
    typed_obligations.sort_by(|a, b| a.obligation_id.as_str().cmp(b.obligation_id.as_str()));
    typed_obligations.dedup_by(|a, b| a == b);

    // The compatibility `SemanticFact` projection is derived FROM `typed_semantic_records` (never
    // computed independently from the raw batches): one is always clearly downstream of the other.
    for observation in &typed_semantic_records {
        if let Some(fact) = semantic_observation_fact(observation) {
            facts.push(fact);
        }
    }

    // SOURCE_ARTIFACT/DECLARED_ADL/BUILD are accounting axes outside the canonical R4 semantic
    // dimension set (SEMANTIC-EXTRACTION.md). The twelve R4 dimensions are keyed by
    // `SemanticDimension::as_str()` and derived from `dimension_summary` above (falling back to
    // UNSUPPORTED only when no extraction batch touched that dimension at all this run), so this
    // map can never silently drift from -- or disagree with -- what extraction actually produced.
    let mut coverage = BTreeMap::from([
        ("SOURCE_ARTIFACT".into(), EpistemicStatus::Observed),
        ("DECLARED_ADL".into(), EpistemicStatus::Declared),
        ("BUILD".into(), EpistemicStatus::Unsupported),
    ]);
    for &dimension in &ALL_SEMANTIC_DIMENSIONS {
        let status = dimension_summary
            .get(&dimension)
            .copied()
            .unwrap_or(EpistemicStatus::Unsupported);
        coverage.insert(dimension.as_str().into(), status);
    }
    if parsed_artifacts == 0 {
        coverage.insert("SOURCE_ARTIFACT".into(), EpistemicStatus::Unknown);
    }

    facts.sort_by(|a, b| {
        a.provenance
            .source_path
            .cmp(&b.provenance.source_path)
            .then_with(|| a.id.cmp(&b.id))
    });

    let typed_closure = TypedClosureAccounting {
        typed_observations_total: typed_semantic_records.len(),
        typed_obligations_total: typed_obligations.len(),
    };

    CensusReport {
        schema: "atlas.census-report.v3".into(),
        artifacts_total: inventory.artifacts_total,
        artifacts_accounted_total: inventory.artifacts.len(),
        facts_total: facts.len(),
        coverage,
        typed_semantic_records,
        evidence,
        diagnostics,
        typed_obligations,
        typed_closure,
        facts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::{
        ArtifactId, ArtifactKind, ArtifactRecord, FileFact, InventoryReport, SourceReport,
        compile_adl,
    };

    #[test]
    fn census_accounts_for_every_inventory_artifact() {
        let inventory = InventoryReport::new(
            "/repo",
            vec![
                ArtifactRecord {
                    id: ArtifactId::new("artifact:rs"),
                    path: "src/lib.rs".into(),
                    kind: ArtifactKind::File,
                    bytes: 10,
                    disposition: ArtifactDisposition::Parsed,
                    language: Some("rust".into()),
                    reason: None,
                },
                ArtifactRecord {
                    id: ArtifactId::new("artifact:unknown"),
                    path: "src/data.xyz".into(),
                    kind: ArtifactKind::File,
                    bytes: 3,
                    disposition: ArtifactDisposition::Unknown,
                    language: None,
                    reason: Some("no-registered-source-frontend".into()),
                },
            ],
        );
        let source = SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 1,
            languages: BTreeMap::from([("rust".into(), 1)]),
            files: vec![FileFact {
                path: "src/lib.rs".into(),
                language: "rust".into(),
                bytes: 10,
            }],
        };
        let adl = compile_adl(&[], &source);
        let report = build_census(&inventory, &source, &adl, &[]);

        assert!(report.is_closed());
        assert_eq!(report.artifacts_accounted_total, 2);
        assert_eq!(
            report.coverage.get("SYMBOL"),
            Some(&EpistemicStatus::Unsupported)
        );
        assert!(
            report
                .facts
                .iter()
                .any(|fact| fact.status == EpistemicStatus::Unknown)
        );
    }

    // --- R4.3.1: canonical flow -----------------------------------------------------------------
    //
    // Real extraction observations must be present in `census.facts` BEFORE normalization, and the
    // normalized/graph path must be demonstrably downstream of them. This assertion is FALSE under
    // the pre-R4.3.1 "accounting sidecar" architecture: `build_census` had no fourth parameter at
    // all, so no `ExtractionBatch` could ever reach `census.facts` regardless of what was extracted
    // -- `extract_semantics` only ever fed a closure boolean into `coding_admission.blockers`.

    fn extraction_batch_with_one_symbol(artifact_path: &str) -> ExtractionBatch {
        use atlas_core::{
            Evidence, EvidenceId, RepositoryId, SemanticRecordHeader, SemanticRecordId,
            SymbolIdentity, SymbolRole, provenance,
        };

        let repository = RepositoryId::new("atlas-studio");
        let revision = atlas_core::RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let extractor = ExtractorIdentity {
            id: "atlas.rust.source-semantic.v1".into(),
            version: "0.1.0".into(),
        };
        let symbol = SymbolIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            scope: SemanticScope::new(Vec::<String>::new()),
            name: "known".into(),
            role: SymbolRole::Definition,
        };
        let record_id = SemanticRecordId::new(SemanticDimension::Symbol, &symbol.identity_key());
        let evidence_id = EvidenceId::new("evidence:known-symbol".to_owned());
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::Symbol,
            status: EpistemicStatus::Observed,
            subject: symbol,
            scope: SemanticScope::new(Vec::<String>::new()),
            repository: repository.clone(),
            revision: revision.clone(),
            extractor: extractor.clone(),
            evidence_refs: vec![evidence_id.clone()],
            provenance: provenance(artifact_path, &extractor.id),
        };
        let observation = SemanticObservation::Symbol(header);
        assert!(observation.is_dimension_consistent());

        ExtractionBatch {
            extractor,
            repository,
            revision,
            artifact: ArtifactId::new(format!("artifact:{artifact_path}")),
            input_fingerprint: format!("test:{artifact_path}"),
            observations: vec![observation],
            evidence: vec![Evidence {
                id: evidence_id.as_str().to_owned(),
                kind: "PARSER_OUTPUT".into(),
                path: artifact_path.into(),
                summary: "test-observed symbol `known`".into(),
                revision: None,
            }],
            obligations: vec![ObligationResult::observed(
                SemanticDimension::Symbol,
                vec![record_id],
                vec![evidence_id],
            )],
            diagnostics: Vec::new(),
        }
    }

    /// Like `extraction_batch_with_one_symbol`, but with a caller-chosen extractor identity and
    /// evidence id, so two batches can report the SAME semantic claim (same `SymbolIdentity`,
    /// hence same `record_id`) while differing in exactly the fields that make them independent
    /// raw observations.
    fn extraction_batch_with_symbol_from(
        artifact_path: &str,
        extractor_id: &str,
        evidence_id: &str,
        evidence_summary: &str,
    ) -> ExtractionBatch {
        use atlas_core::{
            Evidence, EvidenceId, RepositoryId, SemanticRecordHeader, SemanticRecordId,
            SymbolIdentity, SymbolRole, provenance,
        };

        let repository = RepositoryId::new("atlas-studio");
        let revision = atlas_core::RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let extractor = ExtractorIdentity {
            id: extractor_id.into(),
            version: "0.1.0".into(),
        };
        let symbol = SymbolIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            scope: SemanticScope::new(Vec::<String>::new()),
            name: "known".into(),
            role: SymbolRole::Definition,
        };
        let record_id = SemanticRecordId::new(SemanticDimension::Symbol, &symbol.identity_key());
        let evidence_id = EvidenceId::new(evidence_id.to_owned());
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::Symbol,
            status: EpistemicStatus::Observed,
            subject: symbol,
            scope: SemanticScope::new(Vec::<String>::new()),
            repository: repository.clone(),
            revision: revision.clone(),
            extractor: extractor.clone(),
            evidence_refs: vec![evidence_id.clone()],
            provenance: provenance(artifact_path, &extractor.id),
        };
        let observation = SemanticObservation::Symbol(header);
        assert!(observation.is_dimension_consistent());

        ExtractionBatch {
            extractor,
            repository,
            revision,
            artifact: ArtifactId::new(format!("artifact:{artifact_path}")),
            input_fingerprint: format!("test:{artifact_path}"),
            observations: vec![observation],
            evidence: vec![Evidence {
                id: evidence_id.as_str().to_owned(),
                kind: "PARSER_OUTPUT".into(),
                path: artifact_path.into(),
                summary: evidence_summary.into(),
                revision: None,
            }],
            obligations: vec![ObligationResult::observed(
                SemanticDimension::Symbol,
                vec![record_id],
                vec![evidence_id],
            )],
            diagnostics: Vec::new(),
        }
    }

    /// R4.4: a strengthened `FunctionIdentity` observation (an inherent method `Foo::get`, with a
    /// declared generic parameter, exactly like `extraction_batch_with_symbol_from` but for the
    /// `FunctionIdentity` dimension), from a caller-chosen extractor identity -- so two batches can
    /// report the SAME strengthened semantic claim while differing in exactly the fields that make
    /// them independent raw observations.
    fn extraction_batch_with_function_identity_from(
        artifact_path: &str,
        extractor_id: &str,
        evidence_id: &str,
        evidence_summary: &str,
    ) -> ExtractionBatch {
        use atlas_core::{
            Evidence, EvidenceId, FunctionDeclarationKind, FunctionIdentity, FunctionOwner,
            RepositoryId, SemanticRecordHeader, SemanticRecordId, SymbolIdentity, SymbolRole,
            TypeIdentity, provenance,
        };

        let repository = RepositoryId::new("atlas-studio");
        let revision = atlas_core::RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let extractor = ExtractorIdentity {
            id: extractor_id.into(),
            version: "0.1.0".into(),
        };
        let scope = SemanticScope::new(["impl:Foo"]);
        let identity = FunctionIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            language: "rust".into(),
            scope: scope.clone(),
            symbol: SymbolIdentity {
                repository: repository.clone(),
                revision: revision.clone(),
                scope: scope.clone(),
                name: "get".into(),
                role: SymbolRole::Definition,
            },
            span: atlas_core::SourceSpan {
                path: artifact_path.into(),
                line: 1,
                column: 1,
            },
            generated: false,
            declaration_kind: FunctionDeclarationKind::InherentMethod,
            owner: FunctionOwner {
                target: Some(TypeIdentity {
                    repository: repository.clone(),
                    revision: revision.clone(),
                    scope: SemanticScope::new(Vec::<String>::new()),
                    name: "Foo".into(),
                    canonical: None,
                }),
                trait_path: None,
            },
            generics: vec!["T".into()],
        };
        let record_id = SemanticRecordId::new(
            SemanticDimension::FunctionIdentity,
            &identity.identity_key(),
        );
        let evidence_id = EvidenceId::new(evidence_id.to_owned());
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::FunctionIdentity,
            status: EpistemicStatus::Observed,
            scope: scope.clone(),
            repository: repository.clone(),
            revision: revision.clone(),
            extractor: extractor.clone(),
            evidence_refs: vec![evidence_id.clone()],
            provenance: provenance(artifact_path, &extractor.id),
            subject: identity,
        };
        let observation = SemanticObservation::FunctionIdentity(Box::new(header));
        assert!(observation.is_dimension_consistent());

        ExtractionBatch {
            extractor,
            repository,
            revision,
            artifact: ArtifactId::new(format!("artifact:{artifact_path}")),
            input_fingerprint: format!("test:{artifact_path}"),
            observations: vec![observation],
            evidence: vec![Evidence {
                id: evidence_id.as_str().to_owned(),
                kind: "PARSER_OUTPUT".into(),
                path: artifact_path.into(),
                summary: evidence_summary.into(),
                revision: None,
            }],
            obligations: vec![ObligationResult::observed(
                SemanticDimension::FunctionIdentity,
                vec![record_id],
                vec![evidence_id],
            )],
            diagnostics: Vec::new(),
        }
    }

    /// R4.5: a Call observation (`CallSiteIdentity`), attributed to `extractor_id`, whose `function`
    /// (caller) is a fixed synthetic `FunctionIdentity` record_id -- the test only needs the CALL
    /// claim itself to be independently attributable, not a paired FunctionIdentity observation.
    fn extraction_batch_with_call_from(
        artifact_path: &str,
        extractor_id: &str,
        evidence_id: &str,
        evidence_summary: &str,
    ) -> ExtractionBatch {
        use atlas_core::{
            CallDispatchKind, CallSiteIdentity, Evidence, EvidenceId, RepositoryId,
            SemanticRecordHeader, SemanticRecordId, provenance,
        };

        let repository = RepositoryId::new("atlas-studio");
        let revision = atlas_core::RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let extractor = ExtractorIdentity {
            id: extractor_id.into(),
            version: "0.1.0".into(),
        };
        let scope = SemanticScope::new(Vec::<String>::new());
        let caller = SemanticRecordId::new(SemanticDimension::FunctionIdentity, "caller-fn-key");
        let subject = CallSiteIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            function: caller,
            span: atlas_core::SourceSpan {
                path: artifact_path.into(),
                line: 3,
                column: 5,
            },
            dispatch: CallDispatchKind::Unresolved,
            callees: Vec::new(),
        };
        let record_id = SemanticRecordId::new(SemanticDimension::Call, &subject.identity_key());
        let evidence_id = EvidenceId::new(evidence_id.to_owned());
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::Call,
            status: EpistemicStatus::Observed,
            scope,
            repository: repository.clone(),
            revision: revision.clone(),
            extractor: extractor.clone(),
            evidence_refs: vec![evidence_id.clone()],
            provenance: provenance(artifact_path, &extractor.id),
            subject,
        };
        let observation = SemanticObservation::Call(header);
        assert!(observation.is_dimension_consistent());

        ExtractionBatch {
            extractor,
            repository,
            revision,
            artifact: ArtifactId::new(format!("artifact:{artifact_path}")),
            input_fingerprint: format!("test:{artifact_path}"),
            observations: vec![observation],
            evidence: vec![Evidence {
                id: evidence_id.as_str().to_owned(),
                kind: "PARSER_OUTPUT".into(),
                path: artifact_path.into(),
                summary: evidence_summary.into(),
                revision: None,
            }],
            obligations: vec![ObligationResult::observed(
                SemanticDimension::Call,
                vec![record_id],
                vec![evidence_id],
            )],
            diagnostics: Vec::new(),
        }
    }

    /// R4.6: a ControlFlow block observation, attributed to `extractor_id`, whose `function`
    /// (owner) is a fixed synthetic `FunctionIdentity` record_id -- the test only needs the
    /// CONTROL_FLOW claim itself to be independently attributable, not a paired FunctionIdentity
    /// observation.
    fn extraction_batch_with_control_flow_from(
        artifact_path: &str,
        extractor_id: &str,
        evidence_id: &str,
        evidence_summary: &str,
    ) -> ExtractionBatch {
        use atlas_core::{
            ControlFlowBlockIdentity, ControlFlowBlockKind, ControlFlowEdge, ControlFlowEdgeKind,
            Evidence, EvidenceId, RepositoryId, SemanticRecordHeader, SemanticRecordId, provenance,
        };

        let repository = RepositoryId::new("atlas-studio");
        let revision = atlas_core::RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let extractor = ExtractorIdentity {
            id: extractor_id.into(),
            version: "0.1.0".into(),
        };
        let scope = SemanticScope::new(Vec::<String>::new());
        let function = SemanticRecordId::new(SemanticDimension::FunctionIdentity, "owner-fn-key");
        let subject = ControlFlowBlockIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            function,
            block_index: 0,
            kind: ControlFlowBlockKind::FunctionEntry,
            is_entry: true,
            successors: vec![ControlFlowEdge {
                kind: ControlFlowEdgeKind::Return,
                target: None,
            }],
        };
        let record_id =
            SemanticRecordId::new(SemanticDimension::ControlFlow, &subject.identity_key());
        let evidence_id = EvidenceId::new(evidence_id.to_owned());
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::ControlFlow,
            status: EpistemicStatus::Observed,
            scope,
            repository: repository.clone(),
            revision: revision.clone(),
            extractor: extractor.clone(),
            evidence_refs: vec![evidence_id.clone()],
            provenance: provenance(artifact_path, &extractor.id),
            subject,
        };
        let observation = SemanticObservation::ControlFlow(header);
        assert!(observation.is_dimension_consistent());

        ExtractionBatch {
            extractor,
            repository,
            revision,
            artifact: ArtifactId::new(format!("artifact:{artifact_path}")),
            input_fingerprint: format!("test:{artifact_path}"),
            observations: vec![observation],
            evidence: vec![Evidence {
                id: evidence_id.as_str().to_owned(),
                kind: "PARSER_OUTPUT".into(),
                path: artifact_path.into(),
                summary: evidence_summary.into(),
                revision: None,
            }],
            obligations: vec![ObligationResult::observed(
                SemanticDimension::ControlFlow,
                vec![record_id],
                vec![evidence_id],
            )],
            diagnostics: Vec::new(),
        }
    }

    /// R4.7: a DataFlow value observation, attributed to `extractor_id`, whose `function` (owner)
    /// is a fixed synthetic `FunctionIdentity` record_id -- the test only needs the DATA_FLOW
    /// claim itself to be independently attributable, not a paired FunctionIdentity observation.
    fn extraction_batch_with_data_flow_from(
        artifact_path: &str,
        extractor_id: &str,
        evidence_id: &str,
        evidence_summary: &str,
    ) -> ExtractionBatch {
        use atlas_core::{
            DataFlowResolution, Evidence, EvidenceId, RepositoryId, SemanticRecordHeader,
            SemanticRecordId, ValueIdentity, ValueRole, provenance,
        };

        let repository = RepositoryId::new("atlas-studio");
        let revision = atlas_core::RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let extractor = ExtractorIdentity {
            id: extractor_id.into(),
            version: "0.1.0".into(),
        };
        let scope = SemanticScope::new(Vec::<String>::new());
        let function = SemanticRecordId::new(SemanticDimension::FunctionIdentity, "owner-fn-key");
        let subject = ValueIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            function,
            name: "x".into(),
            span: atlas_core::SourceSpan {
                path: artifact_path.into(),
                line: 3,
                column: 5,
            },
            role: ValueRole::Definition,
            is_parameter: false,
            is_return_flow: false,
            resolution: DataFlowResolution::Unresolved,
            resolved_definition: None,
        };
        let record_id = SemanticRecordId::new(SemanticDimension::DataFlow, &subject.identity_key());
        let evidence_id = EvidenceId::new(evidence_id.to_owned());
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::DataFlow,
            status: EpistemicStatus::Observed,
            scope,
            repository: repository.clone(),
            revision: revision.clone(),
            extractor: extractor.clone(),
            evidence_refs: vec![evidence_id.clone()],
            provenance: provenance(artifact_path, &extractor.id),
            subject,
        };
        let observation = SemanticObservation::DataFlow(header);
        assert!(observation.is_dimension_consistent());

        ExtractionBatch {
            extractor,
            repository,
            revision,
            artifact: ArtifactId::new(format!("artifact:{artifact_path}")),
            input_fingerprint: format!("test:{artifact_path}"),
            observations: vec![observation],
            evidence: vec![Evidence {
                id: evidence_id.as_str().to_owned(),
                kind: "PARSER_OUTPUT".into(),
                path: artifact_path.into(),
                summary: evidence_summary.into(),
                revision: None,
            }],
            obligations: vec![ObligationResult::observed(
                SemanticDimension::DataFlow,
                vec![record_id],
                vec![evidence_id],
            )],
            diagnostics: Vec::new(),
        }
    }

    fn single_rust_file_context() -> (InventoryReport, SourceReport, AdlCompileReport) {
        let inventory = InventoryReport::new(
            "/repo",
            vec![ArtifactRecord {
                id: ArtifactId::new("artifact:src/lib.rs"),
                path: "src/lib.rs".into(),
                kind: ArtifactKind::File,
                bytes: 10,
                disposition: ArtifactDisposition::Parsed,
                language: Some("rust".into()),
                reason: None,
            }],
        );
        let source = SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 1,
            languages: BTreeMap::from([("rust".into(), 1)]),
            files: vec![FileFact {
                path: "src/lib.rs".into(),
                language: "rust".into(),
                bytes: 10,
            }],
        };
        let adl = compile_adl(&[], &source);
        (inventory, source, adl)
    }

    #[test]
    fn real_extraction_observations_reach_census_before_normalization() {
        let (inventory, source, adl) = single_rust_file_context();
        let batch = extraction_batch_with_one_symbol("src/lib.rs");
        let batches = [batch];

        let census = build_census(&inventory, &source, &adl, &batches);

        // 1. Present in the canonical census itself, tagged with the real record identity.
        let symbol_fact = census
            .facts
            .iter()
            .find(|fact| fact.kind == SemanticFactKind::Symbol)
            .expect("a real Symbol observation must appear in census.facts");
        assert_eq!(symbol_fact.status, EpistemicStatus::Observed);
        assert_eq!(symbol_fact.object, "known");
        assert_eq!(symbol_fact.provenance.source_path, "src/lib.rs");

        // 2. Repository-wide SYMBOL coverage reflects the real result, not a hardcoded constant.
        assert_eq!(
            census.coverage.get("SYMBOL"),
            Some(&EpistemicStatus::Observed)
        );

        // 3. Normalization is genuinely downstream: the same fact (canonicalized) survives.
        let normalization = crate::normalize::normalize(&census);
        assert!(
            normalization
                .facts
                .iter()
                .any(|fact| fact.kind == SemanticFactKind::Symbol && fact.object == "known")
        );

        // 4. Graph construction is genuinely downstream: a Symbol node exists.
        let docs = atlas_core::DocsReport {
            schema: "test".into(),
            standard: "test".into(),
            root: "/repo/.atlas".into(),
            gate_ready: true,
            hard_violations_total: 0,
            documents_total: 0,
            canonical_frontmatter_total: 0,
            required_control_docs_missing: Vec::new(),
            missing_frontmatter: Vec::new(),
            documents: Vec::new(),
        };
        let graph = atlas_core::build_system_graph(&source, &docs, &normalization);
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == "Symbol" && node.identity == "known"),
            "the engineering graph must derive from real extraction, not only declared ADL/artifact facts"
        );

        // No fabricated canonical resolution anywhere along the path: the raw batch's own
        // TypeIdentity/observations never gained a `canonical` value by passing through census.
        for observation in &batches[0].observations {
            if let SemanticObservation::Type(header) = observation {
                assert!(header.subject.canonical.is_none());
            }
        }
    }

    #[test]
    fn extraction_is_deterministic_across_repeated_census_builds() {
        let (inventory, source, adl) = single_rust_file_context();
        let batches = [extraction_batch_with_one_symbol("src/lib.rs")];

        let a = build_census(&inventory, &source, &adl, &batches);
        let b = build_census(&inventory, &source, &adl, &batches);
        assert_eq!(a, b);
    }

    #[test]
    fn unknown_and_unsupported_obligations_remain_distinct_after_canonical_integration() {
        let (inventory, source, adl) = single_rust_file_context();
        let extractor = ExtractorIdentity {
            id: "atlas.rust.source-semantic.v1".into(),
            version: "0.1.0".into(),
        };
        let revision = atlas_core::RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let batch = ExtractionBatch {
            extractor,
            repository: atlas_core::RepositoryId::new("atlas-studio"),
            revision,
            artifact: ArtifactId::new("artifact:src/lib.rs"),
            input_fingerprint: "test:src/lib.rs".into(),
            observations: Vec::new(),
            evidence: Vec::new(),
            obligations: vec![
                ObligationResult::unknown(SemanticDimension::Symbol, "diag-unknown".to_owned()),
                ObligationResult::unsupported(
                    SemanticDimension::Call,
                    "diag-unsupported".to_owned(),
                ),
            ],
            diagnostics: Vec::new(),
        };

        let census = build_census(&inventory, &source, &adl, &[batch]);

        let symbol_obligation_fact = census
            .facts
            .iter()
            .find(|fact| {
                fact.kind == SemanticFactKind::SemanticObligation && fact.predicate == "symbol"
            })
            .unwrap();
        assert_eq!(symbol_obligation_fact.status, EpistemicStatus::Unknown);

        let call_obligation_fact = census
            .facts
            .iter()
            .find(|fact| {
                fact.kind == SemanticFactKind::SemanticObligation && fact.predicate == "call"
            })
            .unwrap();
        assert_eq!(call_obligation_fact.status, EpistemicStatus::Unsupported);

        assert_ne!(symbol_obligation_fact.status, call_obligation_fact.status);
        assert_eq!(
            census.coverage.get("SYMBOL"),
            Some(&EpistemicStatus::Unknown)
        );
        assert_eq!(
            census.coverage.get("CALL"),
            Some(&EpistemicStatus::Unsupported)
        );
    }

    // === R4.3.2: lossless typed semantic path ====================================================
    //
    // Every test below FAILS under the pre-R4.3.2 architecture: `CensusReport`/`NormalizationReport`
    // had no `typed_semantic_records`/`evidence`/`diagnostics` fields at all, so a `FunctionSignature`'s
    // parameters/generics/abi/visibility/is_async/is_unsafe/is_extern, an observation's evidence, and
    // an obligation's causing diagnostic were all unrecoverable once `semantic_observation_fact`
    // collapsed them into a subject/predicate/object string triple.

    fn function_signature_fixture_batch() -> ExtractionBatch {
        use adapter::ExtractionInput;
        use atlas_core::ContentFingerprint;

        // Modifier order matches syn's `Signature` grammar (const, async, unsafe/safe, extern):
        // this is syntactically valid (parses cleanly), even though `async` + `extern "C"` would be
        // semantically rejected by rustc -- our extractor only ever runs `syn`, never `rustc`.
        const SOURCE: &str = r#"
pub async unsafe extern "C" fn example<T>(x: Vec<T>, y: &mut usize) -> Result<T, Error> {
    todo!()
}
"#;
        let extractor = adapter::extractors_for_language("rust")
            .into_iter()
            .next()
            .expect("rust has a registered extractor");
        let requested = vec![
            SemanticDimension::Symbol,
            SemanticDimension::Type,
            SemanticDimension::FunctionIdentity,
            SemanticDimension::FunctionSignature,
        ];
        let input = ExtractionInput {
            repository: atlas_core::RepositoryId::new("atlas-studio"),
            revision: atlas_core::RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            artifact: ArtifactId::new("artifact:src/lib.rs"),
            artifact_path: "src/lib.rs".into(),
            source_text: SOURCE.into(),
            content_fingerprint: Some(ContentFingerprint("sha256:fixture".into())),
            source_frontend_id: "atlas.source.rust.bootstrap.v1".into(),
            language: "rust".into(),
            build_profile: None,
            scope_policy: None,
            requested_dimensions: requested.clone(),
        };
        let batch = extractor.extract(&input);
        assert!(batch.is_closed(&requested), "fixture batch must be closed");
        batch
    }

    fn find_function_signature(records: &[SemanticObservation]) -> &atlas_core::FunctionSignature {
        records
            .iter()
            .find_map(|observation| match observation {
                SemanticObservation::FunctionSignature(header) => Some(&header.subject),
                _ => None,
            })
            .expect("a FunctionSignature observation must be present")
    }

    // --- Required test 1: FunctionSignature round-trip preservation --------------------------

    #[test]
    fn function_signature_round_trips_losslessly_through_census_and_normalization() {
        let (inventory, source, adl) = single_rust_file_context();
        let batch = function_signature_fixture_batch();
        let batches = [batch];

        let census = build_census(&inventory, &source, &adl, &batches);
        let signature = find_function_signature(&census.typed_semantic_records);

        assert_eq!(signature.function.symbol.name, "example");
        assert_eq!(signature.visibility, "pub");
        assert!(signature.is_async, "async must survive");
        assert!(signature.is_unsafe, "unsafe must survive");
        assert!(signature.is_extern, "extern must survive");
        assert_eq!(signature.abi.as_deref(), Some("C"));
        assert_eq!(signature.generics, vec!["T".to_owned()]);
        assert_eq!(signature.parameters.len(), 2);
        assert_eq!(signature.parameters[0].name, "x");
        assert_eq!(signature.parameters[0].type_identity.name, "Vec<T>");
        assert_eq!(signature.parameters[1].name, "y");
        assert_eq!(signature.parameters[1].type_identity.name, "&mut usize");
        let return_type = signature.return_type.as_ref().expect("return type present");
        assert_eq!(return_type.name, "Result<T, Error>");
        assert!(
            return_type.canonical.is_none(),
            "no compiler-resolved canonical type identity is ever fabricated"
        );

        let signature_header = census
            .typed_semantic_records
            .iter()
            .find(|observation| matches!(observation, SemanticObservation::FunctionSignature(_)))
            .unwrap();
        assert_eq!(
            signature_header.dimension(),
            SemanticDimension::FunctionSignature
        );

        // Survives normalization not as a reconstructed string, but as the SAME typed record
        // (up to canonical ordering) -- normalize(typed record) -> equivalent typed record.
        let normalization = crate::normalize::normalize(&census);
        let normalized_signature_header = normalization
            .typed_semantic_records
            .iter()
            .find(|observation| matches!(observation, SemanticObservation::FunctionSignature(_)))
            .expect("FunctionSignature observation must survive normalization");
        assert_eq!(normalized_signature_header, signature_header);
    }

    // --- Required test 2: identity preservation -----------------------------------------------

    #[test]
    fn identity_and_attribution_fields_survive_census_and_normalization() {
        let (inventory, source, adl) = single_rust_file_context();
        let batch = extraction_batch_with_one_symbol("src/lib.rs");
        let batches = [batch];

        let census = build_census(&inventory, &source, &adl, &batches);
        let symbol_header = census
            .typed_semantic_records
            .iter()
            .find(|observation| matches!(observation, SemanticObservation::Symbol(_)))
            .expect("Symbol observation present");
        let SemanticObservation::Symbol(header) = symbol_header else {
            unreachable!()
        };

        assert_eq!(
            header.repository,
            atlas_core::RepositoryId::new("atlas-studio")
        );
        assert_eq!(header.revision.value, "abc123");
        assert_eq!(header.scope.segments, Vec::<String>::new());
        assert_eq!(
            header.extractor,
            ExtractorIdentity {
                id: "atlas.rust.source-semantic.v1".into(),
                version: "0.1.0".into(),
            }
        );
        assert_eq!(header.status, EpistemicStatus::Observed);

        let normalization = crate::normalize::normalize(&census);
        let normalized = normalization
            .typed_semantic_records
            .iter()
            .find(|observation| matches!(observation, SemanticObservation::Symbol(_)))
            .expect("Symbol observation must survive normalization");
        assert_eq!(
            normalized, symbol_header,
            "identity/attribution must be byte-for-byte identical after normalization"
        );
    }

    // --- Required test 3: evidence lineage preservation ---------------------------------------

    #[test]
    fn evidence_lineage_survives_census_and_normalization() {
        let (inventory, source, adl) = single_rust_file_context();
        let batch = extraction_batch_with_one_symbol("src/lib.rs");
        let batches = [batch];

        let census = build_census(&inventory, &source, &adl, &batches);
        let SemanticObservation::Symbol(header) = census
            .typed_semantic_records
            .iter()
            .find(|observation| matches!(observation, SemanticObservation::Symbol(_)))
            .unwrap()
        else {
            unreachable!()
        };
        assert!(!header.evidence_refs.is_empty());
        let evidence_id = header.evidence_refs[0].as_str();
        let evidence = census
            .evidence
            .iter()
            .find(|item| item.id == evidence_id)
            .expect("evidence backing the observation must be retained in census.evidence");
        assert_eq!(evidence.summary, "test-observed symbol `known`");
        assert_eq!(evidence.kind, "PARSER_OUTPUT");

        let normalization = crate::normalize::normalize(&census);
        let normalized_evidence = normalization
            .evidence
            .iter()
            .find(|item| item.id == evidence_id)
            .expect("evidence must survive normalization");
        assert_eq!(normalized_evidence, evidence);
    }

    // --- Required test 4: diagnostic preservation (read failure vs parse failure) -------------

    #[test]
    fn diagnostic_lineage_distinguishes_parse_failure_from_read_failure_after_build_census() {
        use adapter::ExtractionInput;
        use atlas_core::{ContentFingerprint, DiagnosticCode, ExtractionDiagnostic};

        let (inventory, source, adl) = single_rust_file_context();
        let extractor_impl = adapter::extractors_for_language("rust")
            .into_iter()
            .next()
            .unwrap();
        let requested = vec![SemanticDimension::Symbol];

        let parse_input = ExtractionInput {
            repository: atlas_core::RepositoryId::new("atlas-studio"),
            revision: atlas_core::RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            artifact: ArtifactId::new("artifact:broken.rs"),
            artifact_path: "broken.rs".into(),
            source_text: "pub fn broken( {".into(),
            content_fingerprint: Some(ContentFingerprint("sha256:broken".into())),
            source_frontend_id: "atlas.source.rust.bootstrap.v1".into(),
            language: "rust".into(),
            build_profile: None,
            scope_policy: None,
            requested_dimensions: requested.clone(),
        };
        let parse_failure_batch = extractor_impl.extract(&parse_input);

        let read_input = ExtractionInput {
            repository: atlas_core::RepositoryId::new("atlas-studio"),
            revision: atlas_core::RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            artifact: ArtifactId::new("artifact:missing.rs"),
            artifact_path: "missing.rs".into(),
            source_text: String::new(),
            content_fingerprint: None,
            source_frontend_id: "atlas.source.rust.bootstrap.v1".into(),
            language: "rust".into(),
            build_profile: None,
            scope_policy: None,
            requested_dimensions: requested,
        };
        let read_diagnostic = ExtractionDiagnostic::new(
            DiagnosticCode::InvalidInput,
            None,
            "failed to read missing.rs".to_owned(),
        );
        let read_failure_batch = extractor_impl.unavailable(&read_input, read_diagnostic);

        let batches = [parse_failure_batch, read_failure_batch];
        let census = build_census(&inventory, &source, &adl, &batches);

        let parse_diag = census
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagnosticCode::ParseFailure)
            .expect("ParseFailure diagnostic must survive into census.diagnostics");
        let read_diag = census
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagnosticCode::InvalidInput)
            .expect("InvalidInput diagnostic must survive into census.diagnostics");
        assert_ne!(parse_diag.code, read_diag.code);

        // Both artifacts' SYMBOL obligation is UNKNOWN -- it is NOT sufficient for both to merely
        // say UNKNOWN; the accounting ledger must resolve each to its OWN distinct cause.
        let mut accounting = CensusExtractionAccounting::new();
        for batch in &batches {
            accounting.record_batch(batch);
        }
        let broken_obligation = accounting
            .records_for(SemanticDimension::Symbol)
            .into_iter()
            .find(|record| record.artifact == ArtifactId::new("artifact:broken.rs"))
            .unwrap();
        let missing_obligation = accounting
            .records_for(SemanticDimension::Symbol)
            .into_iter()
            .find(|record| record.artifact == ArtifactId::new("artifact:missing.rs"))
            .unwrap();
        assert_eq!(
            broken_obligation.obligation.status,
            EpistemicStatus::Unknown
        );
        assert_eq!(
            missing_obligation.obligation.status,
            EpistemicStatus::Unknown
        );

        let broken_diagnostic = accounting
            .diagnostic(&broken_obligation.obligation.diagnostics[0])
            .expect("diagnostic must be resolvable from the accounting ledger");
        let missing_diagnostic = accounting
            .diagnostic(&missing_obligation.obligation.diagnostics[0])
            .expect("diagnostic must be resolvable from the accounting ledger");
        assert_eq!(broken_diagnostic.code, DiagnosticCode::ParseFailure);
        assert_eq!(missing_diagnostic.code, DiagnosticCode::InvalidInput);
        assert_ne!(
            broken_diagnostic.code, missing_diagnostic.code,
            "both obligations are UNKNOWN, but their causes must remain distinguishable"
        );
    }

    // --- Required test 6: compatibility projection is downstream, never the sole carrier ------

    #[test]
    fn removing_the_compatibility_fact_projection_would_not_erase_typed_semantic_truth() {
        let (inventory, source, adl) = single_rust_file_context();
        let batch = function_signature_fixture_batch();
        let batches = [batch];
        let census = build_census(&inventory, &source, &adl, &batches);

        let signature_fact = census
            .facts
            .iter()
            .find(|fact| fact.kind == SemanticFactKind::FunctionSignature)
            .expect("compatibility FunctionSignature fact present");
        // The compatibility projection is a lossy display summary: it never mentions the ABI or
        // visibility at all (proving `facts` alone cannot answer "what is this function's ABI").
        assert!(!signature_fact.object.contains("extern"));
        assert!(!signature_fact.object.contains("pub"));

        // The typed world retains everything, independently of `facts`.
        let signature = find_function_signature(&census.typed_semantic_records);
        assert_eq!(signature.abi.as_deref(), Some("C"));
        assert_eq!(signature.visibility, "pub");

        // Concretely: clearing `facts` (simulating "no compatibility projection exists") leaves
        // `typed_semantic_records` completely untouched -- it was never derived FROM `facts`.
        let mut without_compatibility_projection = census.clone();
        without_compatibility_projection.facts.clear();
        without_compatibility_projection.facts_total = 0;
        assert_eq!(
            without_compatibility_projection.typed_semantic_records,
            census.typed_semantic_records
        );
        assert!(
            !without_compatibility_projection
                .typed_semantic_records
                .is_empty()
        );
    }

    // --- typed_semantic_records de-duplicates by record_id, matching evidence/diagnostics --------

    #[test]
    fn typed_semantic_records_collapse_an_exact_record_id_repeat_across_batches() {
        let (inventory, source, adl) = single_rust_file_context();
        // The exact same observation (identical record_id) reported by two batches -- e.g. a
        // legitimate re-observation via a shared extraction dependency -- must collapse to one
        // entry, matching how `evidence`/`diagnostics` already de-duplicate by id.
        let batches = [
            extraction_batch_with_one_symbol("src/lib.rs"),
            extraction_batch_with_one_symbol("src/lib.rs"),
        ];

        let census = build_census(&inventory, &source, &adl, &batches);
        let symbol_records: Vec<_> = census
            .typed_semantic_records
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::Symbol(_)))
            .collect();
        assert_eq!(
            symbol_records.len(),
            1,
            "an exact record_id repeat across batches must not appear as a literal duplicate"
        );

        let normalization = crate::normalize::normalize(&census);
        let normalized_symbol_records: Vec<_> = normalization
            .typed_semantic_records
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::Symbol(_)))
            .collect();
        assert_eq!(normalized_symbol_records.len(), 1);
    }

    // === R4.3.3: multi-extractor raw observation survival + canonical obligation lineage =======
    //
    // Every test below FAILS under the pre-R4.3.3 architecture: `typed_semantic_records` deduped
    // by `record_id` alone (erasing one extractor's observation of a shared semantic claim), and
    // no canonical, serializable obligation ledger existed at all (only the runtime-local
    // `CensusExtractionAccounting`).

    fn two_extractor_symbol_batches() -> [ExtractionBatch; 2] {
        [
            extraction_batch_with_symbol_from(
                "src/lib.rs",
                "extractor-a",
                "evidence:a",
                "extractor-a observed `known`",
            ),
            extraction_batch_with_symbol_from(
                "src/lib.rs",
                "extractor-b",
                "evidence:b",
                "extractor-b observed `known`",
            ),
        ]
    }

    // === R4.4: strengthened FunctionIdentity claims stay multi-extractor safe ===================
    //
    // Same proof as the R4.3.3 Symbol tests above, but for the R4.4-strengthened FunctionIdentity
    // dimension specifically: the new declaration_kind/owner/generics fields must not have
    // reintroduced any record_id-vs-raw_observation_id conflation, and the semantic claim identity
    // (record_id) must remain untouched by which extractor reported it.

    #[test]
    fn strengthened_function_identity_claim_from_two_extractors_survives_census_and_normalization()
    {
        let (inventory, source, adl) = single_rust_file_context();
        let batches = [
            extraction_batch_with_function_identity_from(
                "src/lib.rs",
                "extractor-a",
                "evidence:a",
                "extractor-a observed `Foo::get`",
            ),
            extraction_batch_with_function_identity_from(
                "src/lib.rs",
                "extractor-b",
                "evidence:b",
                "extractor-b observed `Foo::get`",
            ),
        ];

        let census = build_census(&inventory, &source, &adl, &batches);
        let function_identities: Vec<_> = census
            .typed_semantic_records
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::FunctionIdentity(_)))
            .collect();
        assert_eq!(
            function_identities.len(),
            2,
            "independent extractors' observations of the same strengthened FunctionIdentity \
             claim must both survive Census"
        );

        // Required test 17: the semantic claim identity (record_id) is not polluted by extractor
        // id -- both observations report the SAME record_id (they describe the same claim)...
        let claim_ids: std::collections::BTreeSet<&str> = function_identities
            .iter()
            .map(|observation| observation.record_id().as_str())
            .collect();
        assert_eq!(
            claim_ids.len(),
            1,
            "both observations share the same FunctionIdentity claim"
        );

        // ...while raw_observation_id (extractor-attributed) remains distinct per extractor.
        let raw_ids: std::collections::BTreeSet<_> = function_identities
            .iter()
            .map(|observation| observation.raw_observation_id())
            .collect();
        assert_eq!(raw_ids.len(), 2);

        let extractor_ids: std::collections::BTreeSet<&str> = function_identities
            .iter()
            .map(|observation| {
                let SemanticObservation::FunctionIdentity(header) = observation else {
                    unreachable!()
                };
                header.extractor.id.as_str()
            })
            .collect();
        assert_eq!(
            extractor_ids,
            std::collections::BTreeSet::from(["extractor-a", "extractor-b"])
        );

        // Survives normalization too, still independently attributable.
        let normalization = crate::normalize::normalize(&census);
        let normalized: Vec<_> = normalization
            .typed_semantic_records
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::FunctionIdentity(_)))
            .collect();
        assert_eq!(normalized.len(), 2);
    }

    // === R4.5: Call claims stay multi-extractor safe =============================================
    //
    // Same proof, for the new CALL dimension: two independent extractors observing the exact same
    // call site must both survive Census/Normalization, share one claim identity (record_id), and
    // remain distinctly attributable by raw_observation_id/extractor id.

    #[test]
    fn same_call_site_claim_from_two_extractors_survives_census_and_normalization() {
        let (inventory, source, adl) = single_rust_file_context();
        let batches = [
            extraction_batch_with_call_from(
                "src/lib.rs",
                "extractor-a",
                "evidence:call-a",
                "extractor-a observed a call at src/lib.rs:3:5",
            ),
            extraction_batch_with_call_from(
                "src/lib.rs",
                "extractor-b",
                "evidence:call-b",
                "extractor-b observed a call at src/lib.rs:3:5",
            ),
        ];

        let census = build_census(&inventory, &source, &adl, &batches);
        let calls: Vec<_> = census
            .typed_semantic_records
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::Call(_)))
            .collect();
        assert_eq!(
            calls.len(),
            2,
            "independent extractors' observations of the same call site must both survive Census"
        );

        let claim_ids: std::collections::BTreeSet<&str> = calls
            .iter()
            .map(|observation| observation.record_id().as_str())
            .collect();
        assert_eq!(
            claim_ids.len(),
            1,
            "both observations share the same Call claim"
        );

        let raw_ids: std::collections::BTreeSet<_> = calls
            .iter()
            .map(|observation| observation.raw_observation_id())
            .collect();
        assert_eq!(raw_ids.len(), 2);

        let extractor_ids: std::collections::BTreeSet<&str> = calls
            .iter()
            .map(|observation| {
                let SemanticObservation::Call(header) = observation else {
                    unreachable!()
                };
                header.extractor.id.as_str()
            })
            .collect();
        assert_eq!(
            extractor_ids,
            std::collections::BTreeSet::from(["extractor-a", "extractor-b"])
        );

        let normalization = crate::normalize::normalize(&census);
        let normalized: Vec<_> = normalization
            .typed_semantic_records
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::Call(_)))
            .collect();
        assert_eq!(normalized.len(), 2);
    }

    // === R4.6: ControlFlow block claims stay multi-extractor safe =================================
    //
    // Same proof, for the new CONTROL_FLOW dimension: two independent extractors observing the
    // exact same CFG block must both survive Census/Normalization, share one claim identity
    // (record_id), and remain distinctly attributable by raw_observation_id/extractor id.

    #[test]
    fn same_control_flow_block_claim_from_two_extractors_survives_census_and_normalization() {
        let (inventory, source, adl) = single_rust_file_context();
        let batches = [
            extraction_batch_with_control_flow_from(
                "src/lib.rs",
                "extractor-a",
                "evidence:cfg-a",
                "extractor-a observed a control-flow block at src/lib.rs",
            ),
            extraction_batch_with_control_flow_from(
                "src/lib.rs",
                "extractor-b",
                "evidence:cfg-b",
                "extractor-b observed a control-flow block at src/lib.rs",
            ),
        ];

        let census = build_census(&inventory, &source, &adl, &batches);
        let blocks: Vec<_> = census
            .typed_semantic_records
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::ControlFlow(_)))
            .collect();
        assert_eq!(
            blocks.len(),
            2,
            "independent extractors' observations of the same CFG block must both survive Census"
        );

        let claim_ids: std::collections::BTreeSet<&str> = blocks
            .iter()
            .map(|observation| observation.record_id().as_str())
            .collect();
        assert_eq!(
            claim_ids.len(),
            1,
            "both observations share the same ControlFlow claim"
        );

        let raw_ids: std::collections::BTreeSet<_> = blocks
            .iter()
            .map(|observation| observation.raw_observation_id())
            .collect();
        assert_eq!(raw_ids.len(), 2);

        let extractor_ids: std::collections::BTreeSet<&str> = blocks
            .iter()
            .map(|observation| {
                let SemanticObservation::ControlFlow(header) = observation else {
                    unreachable!()
                };
                header.extractor.id.as_str()
            })
            .collect();
        assert_eq!(
            extractor_ids,
            std::collections::BTreeSet::from(["extractor-a", "extractor-b"])
        );

        let normalization = crate::normalize::normalize(&census);
        let normalized: Vec<_> = normalization
            .typed_semantic_records
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::ControlFlow(_)))
            .collect();
        assert_eq!(normalized.len(), 2);
    }

    // === R4.7: DataFlow value claims stay multi-extractor safe =====================================
    //
    // Same proof, for the new DATA_FLOW dimension: two independent extractors observing the exact
    // same value must both survive Census/Normalization, share one claim identity (record_id), and
    // remain distinctly attributable by raw_observation_id/extractor id.

    #[test]
    fn same_data_flow_value_claim_from_two_extractors_survives_census_and_normalization() {
        let (inventory, source, adl) = single_rust_file_context();
        let batches = [
            extraction_batch_with_data_flow_from(
                "src/lib.rs",
                "extractor-a",
                "evidence:dataflow-a",
                "extractor-a observed a data-flow value at src/lib.rs",
            ),
            extraction_batch_with_data_flow_from(
                "src/lib.rs",
                "extractor-b",
                "evidence:dataflow-b",
                "extractor-b observed a data-flow value at src/lib.rs",
            ),
        ];

        let census = build_census(&inventory, &source, &adl, &batches);
        let values: Vec<_> = census
            .typed_semantic_records
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::DataFlow(_)))
            .collect();
        assert_eq!(
            values.len(),
            2,
            "independent extractors' observations of the same value must both survive Census"
        );

        let claim_ids: std::collections::BTreeSet<&str> = values
            .iter()
            .map(|observation| observation.record_id().as_str())
            .collect();
        assert_eq!(
            claim_ids.len(),
            1,
            "both observations share the same DataFlow claim"
        );

        let raw_ids: std::collections::BTreeSet<_> = values
            .iter()
            .map(|observation| observation.raw_observation_id())
            .collect();
        assert_eq!(raw_ids.len(), 2);

        let extractor_ids: std::collections::BTreeSet<&str> = values
            .iter()
            .map(|observation| {
                let SemanticObservation::DataFlow(header) = observation else {
                    unreachable!()
                };
                header.extractor.id.as_str()
            })
            .collect();
        assert_eq!(
            extractor_ids,
            std::collections::BTreeSet::from(["extractor-a", "extractor-b"])
        );

        let normalization = crate::normalize::normalize(&census);
        let normalized: Vec<_> = normalization
            .typed_semantic_records
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::DataFlow(_)))
            .collect();
        assert_eq!(normalized.len(), 2);
    }

    // --- Required test 1: same semantic claim from two extractors survives Census -------------

    #[test]
    fn same_semantic_claim_from_two_extractors_survives_independently_through_census() {
        let (inventory, source, adl) = single_rust_file_context();
        let batches = two_extractor_symbol_batches();

        let census = build_census(&inventory, &source, &adl, &batches);
        let symbol_records: Vec<_> = census
            .typed_semantic_records
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::Symbol(_)))
            .collect();
        assert_eq!(
            symbol_records.len(),
            2,
            "independent extractors' observations of the same semantic claim must both survive"
        );

        // Same semantic claim identity...
        let claim_ids: std::collections::BTreeSet<&str> = symbol_records
            .iter()
            .map(|observation| observation.record_id().as_str())
            .collect();
        assert_eq!(
            claim_ids.len(),
            1,
            "both observations share the same SymbolIdentity claim"
        );

        // ...but distinct, independently attributable extractors.
        let extractor_ids: std::collections::BTreeSet<&str> = symbol_records
            .iter()
            .map(|observation| {
                let SemanticObservation::Symbol(header) = observation else {
                    unreachable!()
                };
                header.extractor.id.as_str()
            })
            .collect();
        assert_eq!(
            extractor_ids,
            std::collections::BTreeSet::from(["extractor-a", "extractor-b"])
        );
    }

    // --- Required test 2: same, but through normalization --------------------------------------

    #[test]
    fn same_semantic_claim_from_two_extractors_survives_independently_through_normalization() {
        let (inventory, source, adl) = single_rust_file_context();
        let batches = two_extractor_symbol_batches();

        let census = build_census(&inventory, &source, &adl, &batches);
        let normalization = crate::normalize::normalize(&census);
        let symbol_records: Vec<_> = normalization
            .typed_semantic_records
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::Symbol(_)))
            .collect();
        assert_eq!(
            symbol_records.len(),
            2,
            "normalization must not collapse two independent extractors' observations"
        );
        let extractor_ids: std::collections::BTreeSet<&str> = symbol_records
            .iter()
            .map(|observation| {
                let SemanticObservation::Symbol(header) = observation else {
                    unreachable!()
                };
                header.extractor.id.as_str()
            })
            .collect();
        assert_eq!(
            extractor_ids,
            std::collections::BTreeSet::from(["extractor-a", "extractor-b"])
        );
    }

    // --- Required test 3: both evidence sets remain resolvable ---------------------------------

    #[test]
    fn both_extractors_evidence_sets_remain_resolvable_through_census_and_normalization() {
        let (inventory, source, adl) = single_rust_file_context();
        let batches = two_extractor_symbol_batches();

        let census = build_census(&inventory, &source, &adl, &batches);
        assert!(census.evidence.iter().any(|item| item.id == "evidence:a"));
        assert!(census.evidence.iter().any(|item| item.id == "evidence:b"));

        let normalization = crate::normalize::normalize(&census);
        assert!(
            normalization
                .evidence
                .iter()
                .any(|item| item.id == "evidence:a")
        );
        assert!(
            normalization
                .evidence
                .iter()
                .any(|item| item.id == "evidence:b")
        );
    }

    // --- Required tests 5/6/7: typed obligation survives Census+normalization and resolves -----

    #[test]
    fn typed_obligation_survives_census_and_normalization_and_resolves_its_references() {
        let (inventory, source, adl) = single_rust_file_context();
        let batch = extraction_batch_with_one_symbol("src/lib.rs");
        let batches = [batch];

        let census = build_census(&inventory, &source, &adl, &batches);
        let obligation = census
            .typed_obligations
            .iter()
            .find(|obligation| obligation.dimension == SemanticDimension::Symbol)
            .expect("a typed SYMBOL obligation must survive census");
        assert_eq!(obligation.status, EpistemicStatus::Observed);
        assert_eq!(obligation.artifact, ArtifactId::new("artifact:src/lib.rs"));
        assert_eq!(obligation.extractor.id, "atlas.rust.source-semantic.v1");
        assert!(!obligation.observation_ids.is_empty());
        assert!(!obligation.evidence_refs.is_empty());

        for id in &obligation.observation_ids {
            assert!(
                census
                    .typed_semantic_records
                    .iter()
                    .any(|observation| observation.record_id() == id),
                "obligation.observation_ids must resolve within census.typed_semantic_records"
            );
        }
        for id in &obligation.evidence_refs {
            assert!(
                census.evidence.iter().any(|item| item.id == id.as_str()),
                "obligation.evidence_refs must resolve within census.evidence"
            );
        }

        let normalization = crate::normalize::normalize(&census);
        let normalized_obligation = normalization
            .typed_obligations
            .iter()
            .find(|candidate| candidate.obligation_id == obligation.obligation_id)
            .expect("the typed obligation must survive normalization");
        assert_eq!(normalized_obligation, obligation);
        for id in &normalized_obligation.observation_ids {
            assert!(
                normalization
                    .typed_semantic_records
                    .iter()
                    .any(|observation| observation.record_id() == id)
            );
        }
    }

    // --- Required test 8: parse vs read failure distinguishable from CANONICAL Census alone ----

    #[test]
    fn canonical_census_alone_distinguishes_parse_failure_from_read_failure() {
        use adapter::ExtractionInput;
        use atlas_core::{ContentFingerprint, DiagnosticCode, ExtractionDiagnostic};

        let (inventory, source, adl) = single_rust_file_context();
        let extractor_impl = adapter::extractors_for_language("rust")
            .into_iter()
            .next()
            .unwrap();
        let requested = vec![SemanticDimension::Symbol];

        let parse_input = ExtractionInput {
            repository: atlas_core::RepositoryId::new("atlas-studio"),
            revision: atlas_core::RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            artifact: ArtifactId::new("artifact:broken.rs"),
            artifact_path: "broken.rs".into(),
            source_text: "pub fn broken( {".into(),
            content_fingerprint: Some(ContentFingerprint("sha256:broken".into())),
            source_frontend_id: "atlas.source.rust.bootstrap.v1".into(),
            language: "rust".into(),
            build_profile: None,
            scope_policy: None,
            requested_dimensions: requested.clone(),
        };
        let parse_failure_batch = extractor_impl.extract(&parse_input);

        let read_input = ExtractionInput {
            repository: atlas_core::RepositoryId::new("atlas-studio"),
            revision: atlas_core::RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            artifact: ArtifactId::new("artifact:missing.rs"),
            artifact_path: "missing.rs".into(),
            source_text: String::new(),
            content_fingerprint: None,
            source_frontend_id: "atlas.source.rust.bootstrap.v1".into(),
            language: "rust".into(),
            build_profile: None,
            scope_policy: None,
            requested_dimensions: requested,
        };
        let read_diagnostic = ExtractionDiagnostic::new(
            DiagnosticCode::InvalidInput,
            None,
            "failed to read missing.rs".to_owned(),
        );
        let read_failure_batch = extractor_impl.unavailable(&read_input, read_diagnostic);

        let batches = [parse_failure_batch, read_failure_batch];
        let census = build_census(&inventory, &source, &adl, &batches);

        // Using ONLY the canonical, serializable Census fields (typed_obligations + diagnostics)
        // -- never `runtime::census::CensusExtractionAccounting` -- prove the two artifacts'
        // UNKNOWN SYMBOL obligations remain distinguishable by cause.
        let broken_obligation = census
            .typed_obligations
            .iter()
            .find(|obligation| {
                obligation.artifact == ArtifactId::new("artifact:broken.rs")
                    && obligation.dimension == SemanticDimension::Symbol
            })
            .expect("broken.rs SYMBOL obligation must be present");
        let missing_obligation = census
            .typed_obligations
            .iter()
            .find(|obligation| {
                obligation.artifact == ArtifactId::new("artifact:missing.rs")
                    && obligation.dimension == SemanticDimension::Symbol
            })
            .expect("missing.rs SYMBOL obligation must be present");
        assert_eq!(broken_obligation.status, EpistemicStatus::Unknown);
        assert_eq!(missing_obligation.status, EpistemicStatus::Unknown);

        let broken_diag_id = broken_obligation
            .diagnostic_ids
            .first()
            .expect("broken.rs obligation must reference a diagnostic");
        let missing_diag_id = missing_obligation
            .diagnostic_ids
            .first()
            .expect("missing.rs obligation must reference a diagnostic");
        let broken_diag = census
            .diagnostics
            .iter()
            .find(|diagnostic| &diagnostic.id == broken_diag_id)
            .expect("broken.rs diagnostic must resolve within census.diagnostics");
        let missing_diag = census
            .diagnostics
            .iter()
            .find(|diagnostic| &diagnostic.id == missing_diag_id)
            .expect("missing.rs diagnostic must resolve within census.diagnostics");
        assert_eq!(broken_diag.code, DiagnosticCode::ParseFailure);
        assert_eq!(missing_diag.code, DiagnosticCode::InvalidInput);
        assert_ne!(
            broken_diag.code, missing_diag.code,
            "both obligations are UNKNOWN, but their causes must remain distinguishable using \
             only canonical Census data"
        );
    }

    // --- Required test 9: serialized SystemizeReport (via CensusReport) retains lineage --------

    #[test]
    fn serialized_census_report_retains_obligation_lineage_after_round_trip() {
        let (inventory, source, adl) = single_rust_file_context();
        let batch = extraction_batch_with_one_symbol("src/lib.rs");
        let batches = [batch];
        let census = build_census(&inventory, &source, &adl, &batches);

        // `CensusReport`/`NormalizationReport` are direct fields of `SystemizeReport`, so a
        // lossless serde round trip here proves the same for the full serialized report.
        let json = serde_json::to_string(&census).expect("CensusReport must serialize");
        let deserialized: atlas_core::CensusReport =
            serde_json::from_str(&json).expect("CensusReport must deserialize");
        assert_eq!(deserialized, census, "serde round trip must be lossless");

        // Answer an obligation-lineage query using ONLY the deserialized copy.
        let obligation = deserialized
            .typed_obligations
            .iter()
            .find(|obligation| obligation.dimension == SemanticDimension::Symbol)
            .expect("obligation must survive serialization");
        assert_eq!(obligation.extractor.id, "atlas.rust.source-semantic.v1");
        assert_eq!(obligation.artifact, ArtifactId::new("artifact:src/lib.rs"));
        for id in &obligation.observation_ids {
            assert!(
                deserialized
                    .typed_semantic_records
                    .iter()
                    .any(|observation| observation.record_id() == id)
            );
        }
        for id in &obligation.evidence_refs {
            assert!(
                deserialized
                    .evidence
                    .iter()
                    .any(|item| item.id == id.as_str())
            );
        }
    }

    // --- Task 3: CensusExtractionAccounting stays consistent with the canonical obligation ledger

    #[test]
    fn census_extraction_accounting_agrees_with_typed_obligations_on_the_same_batches() {
        let (inventory, source, adl) = single_rust_file_context();
        let batches = two_extractor_symbol_batches();

        let census = build_census(&inventory, &source, &adl, &batches);
        let mut accounting = CensusExtractionAccounting::new();
        for batch in &batches {
            accounting.record_batch(batch);
        }

        let census_coordinates: std::collections::BTreeSet<(String, String, String)> = census
            .typed_obligations
            .iter()
            .map(|obligation| {
                (
                    obligation.artifact.as_str().to_owned(),
                    obligation.extractor.id.clone(),
                    obligation.dimension.as_str().to_owned(),
                )
            })
            .collect();
        let accounting_coordinates: std::collections::BTreeSet<(String, String, String)> =
            accounting
                .sorted()
                .iter()
                .map(|record| {
                    (
                        record.artifact.as_str().to_owned(),
                        record.extractor.id.clone(),
                        record.obligation.dimension.as_str().to_owned(),
                    )
                })
                .collect();
        assert_eq!(
            census_coordinates, accounting_coordinates,
            "both views are built from the identical extraction batches and must never disagree \
             about which (artifact, extractor, dimension) coordinates were addressed"
        );
    }
}
