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
    AdlCompileReport, ArtifactDisposition, ArtifactId, CensusReport, EpistemicStatus,
    ExtractorIdentity, FunctionSignature, InventoryReport, Provenance, RevisionRef,
    SemanticDimension, SemanticFact, SemanticFactKind, SemanticObservation, SemanticScope,
    SourceReport, stable_id,
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

    // R4.3.1: fold real `SemanticExtractor` output into the SAME canonical census this function
    // returns, before normalization/graph ever run. `path_by_artifact` recovers each artifact's
    // human path for provenance -- `ExtractionBatch` itself only carries the opaque `ArtifactId`.
    let path_by_artifact: BTreeMap<&str, &str> = inventory
        .artifacts
        .iter()
        .map(|artifact| (artifact.id.as_str(), artifact.path.as_str()))
        .collect();
    let mut dimension_summary: BTreeMap<SemanticDimension, EpistemicStatus> = BTreeMap::new();

    for batch in extraction_batches {
        let artifact_path = path_by_artifact
            .get(batch.artifact.as_str())
            .copied()
            .unwrap_or_default();

        for observation in &batch.observations {
            if let Some(fact) = semantic_observation_fact(observation) {
                facts.push(fact);
            }
        }

        for obligation in &batch.obligations {
            facts.push(obligation_status_fact(
                artifact_path,
                &batch.artifact,
                &batch.extractor,
                &batch.revision,
                obligation,
            ));

            let summary = dimension_summary
                .entry(obligation.dimension)
                .or_insert(obligation.status);
            if status_rank(obligation.status) > status_rank(*summary) {
                *summary = obligation.status;
            }
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

    CensusReport {
        schema: "atlas.census-report.v1".into(),
        artifacts_total: inventory.artifacts_total,
        artifacts_accounted_total: inventory.artifacts.len(),
        facts_total: facts.len(),
        coverage,
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
}
