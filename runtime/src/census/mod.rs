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
pub mod resolution;

pub use extraction::{
    ALL_SEMANTIC_DIMENSIONS, CensusExtractionAccounting, ExtractorObligationRecord,
};

use adapter::{ExtractionBatch, ObligationResult};
use atlas_core::{
    AdlCompileReport, ArtifactDisposition, ArtifactId, CensusReport, EpistemicStatus, Evidence,
    ExtractionDiagnostic, ExtractorIdentity, InventoryReport, Provenance, RevisionRef,
    SemanticDimension, SemanticFact, SemanticFactKind, SemanticObligationRecord,
    SemanticObservation, SourceReport, TypedClosureAccounting, escape_identity_field, stable_id,
};
use std::{collections::BTreeMap, path::Path};

fn fact_id(seed: &str) -> String {
    stable_id("semantic-fact", seed)
}

/// Projects one real typed `SemanticObservation` (produced by a `SemanticExtractor`, never
/// invented here) into the bootstrap `SemanticFact` triple envelope that `normalize`/graph
/// construction already consume. `None` for dimensions that already have a typed kernel/extractor
/// (CALL/CONTROL_FLOW/DATA_FLOW/STATE/EFFECT/OWNERSHIP/CONCURRENCY/PERSISTENCE -- R4.4+): the typed
/// `SemanticObservation` variant itself is the canonical record; this bootstrap compatibility
/// envelope was never extended to cover them and there is no reason to start now.
///
/// This is a lossy compatibility projection, not a second source of truth: the typed
/// `SemanticObservation` (retrievable from the `ExtractionBatch`es a caller passed to
/// `build_census`) remains the canonical record this fact is derived *from*
/// (`.atlas/contracts/SEMANTIC-EXTRACTION.md#r4-acceptance-matrix`: "SemanticFact remains a
/// compatibility envelope, not the only semantic type system"). `TypeIdentity.canonical` never
/// participates here -- only source spelling ever reaches `object`, never a fabricated
/// compiler-resolved identity.
///
/// Each variant's `id` seed folds in `header.extractor.id`/`header.extractor.version` alongside
/// `record_id`, matching `obligation_status_fact`'s own established discipline (extractor identity
/// must be part of a fact's `id`, never `record_id` alone): `record_id` is deliberately SHARED
/// across independent extractors that observe the same semantic claim
/// (`.atlas/contracts/SEMANTIC-EXTRACTION.md#multi-engine-extraction`), so two extractors agreeing
/// on the same Symbol/Type/FunctionIdentity/FunctionSignature claim produce two real, independent
/// `SemanticFact`s with different provenance -- they must never collide on `id`, the one field
/// whose entire purpose is stable per-fact identity.
fn semantic_observation_fact(observation: &SemanticObservation) -> Option<SemanticFact> {
    match observation {
        SemanticObservation::Symbol(header) => Some(SemanticFact {
            id: fact_id(&format!(
                "extraction:symbol:{}:{}:{}",
                header.record_id.as_str(),
                header.extractor.id,
                header.extractor.version
            )),
            kind: SemanticFactKind::Symbol,
            status: header.status,
            subject: header.record_id.as_str().to_owned(),
            predicate: "declares_symbol".into(),
            object: header.subject.scope.scoped_name(&header.subject.name),
            provenance: header.provenance.clone(),
        }),
        SemanticObservation::Type(header) => Some(SemanticFact {
            id: fact_id(&format!(
                "extraction:type:{}:{}:{}",
                header.record_id.as_str(),
                header.extractor.id,
                header.extractor.version
            )),
            kind: SemanticFactKind::Type,
            status: header.status,
            subject: header.record_id.as_str().to_owned(),
            predicate: "type_spelling".into(),
            object: header.subject.name.clone(),
            provenance: header.provenance.clone(),
        }),
        SemanticObservation::FunctionIdentity(header) => Some(SemanticFact {
            id: fact_id(&format!(
                "extraction:function-identity:{}:{}:{}",
                header.record_id.as_str(),
                header.extractor.id,
                header.extractor.version
            )),
            kind: SemanticFactKind::FunctionIdentity,
            status: header.status,
            subject: header.record_id.as_str().to_owned(),
            predicate: "declares_function".into(),
            object: header
                .subject
                .scope
                .scoped_name(&header.subject.symbol.name),
            provenance: header.provenance.clone(),
        }),
        SemanticObservation::FunctionSignature(header) => Some(SemanticFact {
            id: fact_id(&format!(
                "extraction:function-signature:{}:{}:{}",
                header.record_id.as_str(),
                header.extractor.id,
                header.extractor.version
            )),
            kind: SemanticFactKind::FunctionSignature,
            status: header.status,
            subject: header.record_id.as_str().to_owned(),
            predicate: "function_signature".into(),
            object: header.subject.summary(),
            provenance: header.provenance.clone(),
        }),
        SemanticObservation::Call(_)
        | SemanticObservation::ControlFlow(_)
        | SemanticObservation::DataFlow(_)
        | SemanticObservation::State(_)
        | SemanticObservation::Effect(_)
        | SemanticObservation::Ownership(_)
        | SemanticObservation::Concurrency(_)
        | SemanticObservation::Persistence(_) => None,
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
        | EpistemicStatus::Hypothesis
        | EpistemicStatus::Simulated => 4,
    }
}

fn source_provenance(path: &str, language: Option<&str>, revision: &RevisionRef) -> Provenance {
    let extractor = adapter::resolve_source_frontend(Path::new(path))
        .or_else(|| language.and_then(adapter::source_frontend_for_language))
        .map(|matched| matched.frontend_id.to_owned())
        .unwrap_or_else(|| "atlas.inventory.v1".to_owned());
    Provenance {
        source_path: path.to_owned(),
        source_revision: Some(revision.clone()),
        extractor,
        content_hash: None,
        span: None,
    }
}

fn adl_provenance(path: &str, line: usize, column: usize, revision: &RevisionRef) -> Provenance {
    Provenance {
        source_path: path.to_owned(),
        source_revision: Some(revision.clone()),
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
    revision: &RevisionRef,
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
            provenance: source_provenance(&artifact.path, artifact.language.as_deref(), revision),
        });

        if let Some(language) = &artifact.language {
            facts.push(SemanticFact {
                id: fact_id(&format!("artifact:{}:language:{language}", artifact.path)),
                kind: SemanticFactKind::SourceArtifact,
                status: EpistemicStatus::Observed,
                subject,
                predicate: "language".into(),
                object: language.clone(),
                provenance: source_provenance(
                    &artifact.path,
                    artifact.language.as_deref(),
                    revision,
                ),
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
            provenance: adl_provenance(&node.span.path, node.span.line, node.span.column, revision),
        });
    }

    // `edge.from`/`.relation`/`.to` and `binding.consumer`/`.capability`/`.provider` are
    // ADL-authored free text, extracted by `parse_relation`'s simple `split_once("->")`, not a
    // restrictive lexer -- the same field set `core::language::adl::mod`'s own `DeclaredEdge.id`
    // already escapes for this exact reason. Escaped here too before joining into `fact_id`'s
    // seed, so two genuinely different declared relations/bindings can never collapse onto one
    // `SemanticFact.id`.
    for edge in &adl.ir.declared.edges {
        facts.push(SemanticFact {
            id: fact_id(&format!(
                "declared-edge:{}:{}:{}",
                escape_identity_field(&edge.from, ':'),
                escape_identity_field(&edge.relation, ':'),
                escape_identity_field(&edge.to, ':'),
            )),
            kind: SemanticFactKind::DeclaredEdge,
            status: EpistemicStatus::Declared,
            subject: edge.from.clone(),
            predicate: edge.relation.clone(),
            object: edge.to.clone(),
            provenance: adl_provenance(&edge.span.path, edge.span.line, edge.span.column, revision),
        });
    }

    for binding in &adl.ir.declared.bindings {
        facts.push(SemanticFact {
            id: fact_id(&format!(
                "binding:{}:{}:{}",
                escape_identity_field(&binding.consumer, ':'),
                escape_identity_field(&binding.capability, ':'),
                escape_identity_field(&binding.provider, ':'),
            )),
            kind: SemanticFactKind::Binding,
            status: EpistemicStatus::Declared,
            subject: binding.consumer.clone(),
            predicate: "binds_to".into(),
            object: binding.provider.clone(),
            provenance: adl_provenance(
                &binding.span.path,
                binding.span.line,
                binding.span.column,
                revision,
            ),
        });
        facts.push(SemanticFact {
            id: fact_id(&format!("binding:{}:capability", binding.name)),
            kind: SemanticFactKind::Binding,
            status: EpistemicStatus::Declared,
            subject: binding.name.clone(),
            predicate: "capability".into(),
            object: binding.capability.clone(),
            provenance: adl_provenance(
                &binding.span.path,
                binding.span.line,
                binding.span.column,
                revision,
            ),
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
                revision,
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
                revision,
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
                source_revision: Some(revision.clone()),
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
                source_revision: Some(revision.clone()),
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
                revision,
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
                revision,
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
    // G119: per (dimension, artifact), the best status any engine reports for that artifact.
    let mut artifact_coverage: BTreeMap<(SemanticDimension, String), EpistemicStatus> =
        BTreeMap::new();
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

            let best = artifact_coverage
                .entry((obligation.dimension, batch.artifact.as_str().to_owned()))
                .or_insert(obligation.status);
            if status_rank(obligation.status) > status_rank(*best) {
                *best = obligation.status;
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
    // `raw_observation_id()` is a bare 64-bit FNV-1a hash of attacker-influenced content -- a
    // non-cryptographic hash with no collision-resistance guarantee against a deliberately
    // adversarial donor repository. Hash equality alone is a fast pre-filter, never sufficient
    // proof of "genuinely identical" on its own: verifying full struct equality alongside it
    // (`SemanticObservation` already derives `PartialEq`) makes this comparator strictly MORE
    // conservative than before -- it can only prevent an unsound merge a hash collision would
    // have caused, never merge two records the old hash-only check would not have.
    typed_semantic_records
        .dedup_by(|a, b| a.raw_observation_id() == b.raw_observation_id() && a == b);
    let evidence: Vec<Evidence> = evidence_by_id.into_values().collect();
    let diagnostics: Vec<ExtractionDiagnostic> = diagnostics_by_id.into_values().collect();

    // Obligations use the same "preserve two independent records rather than merge" discipline:
    // `obligation_id` is a coordinate identity (artifact/extractor/dimension), unique by
    // construction, so a genuine duplicate key always carries identical content; dedup_by full
    // equality still never silently drops a content-differing collision if that invariant is ever
    // violated. But sorting by `obligation_id` ALONE is coarser than the full-equality `dedup_by`
    // comparator, and `Vec::dedup_by` only ever compares an element to the immediately preceding
    // one it kept, never a full pairwise scan within a key-group -- so if that invariant IS ever
    // violated (a caller merging obligations from more than one census run, exactly the scenario
    // this pipeline supports) and a third, content-differing record separates two equal ones in
    // input order, the equal pair can fail to land adjacent after a stable sort, silently
    // under-deduplicating in an input-order-dependent way. Tie-breaking on the full derived
    // `Debug` representation (every field, in declaration order, no custom/lossy impls anywhere in
    // this crate) makes the sort key exactly as fine as the equality comparator, mirroring
    // `typed_semantic_records`' own `(record_id, raw_observation_id)` compound key above.
    typed_obligations.sort_by(|a, b| {
        a.obligation_id
            .as_str()
            .cmp(b.obligation_id.as_str())
            .then_with(|| format!("{a:?}").cmp(&format!("{b:?}")))
    });
    typed_obligations.dedup_by(|a, b| a == b);

    // The compatibility `SemanticFact` projection is derived FROM `typed_semantic_records` (never
    // computed independently from the raw batches): one is always clearly downstream of the other.
    for observation in &typed_semantic_records {
        if let Some(fact) = semantic_observation_fact(observation) {
            facts.push(fact);
        }
    }

    // G119: a dimension's scope coverage is the WORST per-artifact coverage among the artifacts
    // some engine can evaluate for it (per artifact, the best engine: one engine exhaustively
    // covering an artifact covers it). One covered artifact never makes the scope covered; an
    // UNSUPPORTED artifact (no extractor for its language) is accounted separately
    // (UNSUPPORTED_FACTS) and is the scope summary only when no artifact is supported at all.
    let mut dimension_summary: BTreeMap<SemanticDimension, EpistemicStatus> = BTreeMap::new();
    for ((dimension, _), status) in &artifact_coverage {
        let summary = dimension_summary.entry(*dimension).or_insert(*status);
        let supported = |s: EpistemicStatus| s != EpistemicStatus::Unsupported;
        if !supported(*summary)
            || (supported(*status) && status_rank(*status) < status_rank(*summary))
        {
            *summary = *status;
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

    fn test_revision() -> RevisionRef {
        RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        }
    }
    use atlas_core::{
        ArtifactId, ArtifactKind, ArtifactRecord, BindingDecl, DeclaredEdge, FileFact,
        InventoryReport, SemanticScope, SourceReport, SourceSpan, compile_adl,
    };

    /// Every census-level fact -- artifact disposition/language, every ADL declaration, constraint
    /// result and declared/observed delta -- carries the census revision and a named extractor
    /// (G62). Before, all of them were revision-less, so no census could be RECONCILED.
    #[test]
    fn every_census_level_fact_carries_the_census_revision() {
        let inventory = InventoryReport::new(
            "/repo",
            vec![ArtifactRecord {
                id: ArtifactId::new("artifact:rs"),
                path: "src/lib.rs".into(),
                kind: ArtifactKind::File,
                bytes: 10,
                disposition: ArtifactDisposition::Parsed,
                language: Some("rust".into()),
                reason: None,
                content_digest: None,
                content_digest_withheld: None,
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
        let adl = compile_adl(
            &[atlas_core::AdlSource {
                path: ".atlas/declared/system.adl".into(),
                text: "atlas 1\nsystem Example\n\nentity Runtime Compiler {\n    kind = backend\n    language = rust\n}\n\ncapability CompileGraph {\n    input = AST\n    output = SystemGraph\n}\n\nCompiler ->provides-> CompileGraph\n\nbinding CompilerBinding {\n    consumer = Compiler\n    provider = Compiler\n    capability = CompileGraph\n}\n\nmaterialize Compiler {\n    path = \"core\"\n    language = rust\n}\n\nconstraint BackendIsRust {\n    forall x: Runtime\n        where x.kind == backend\n\n    require x.language == rust\n}\n".into(),
            }],
            &source,
        );
        assert!(
            !adl.deltas.is_empty(),
            "`core` is declared but not observed"
        );
        assert!(!adl.constraint_results.is_empty());
        let report = build_census(&inventory, &source, &adl, &[], &test_revision());
        for kind in [
            SemanticFactKind::SourceArtifact,
            SemanticFactKind::DeclaredNode,
            SemanticFactKind::DeclaredEdge,
            SemanticFactKind::Binding,
            SemanticFactKind::Materialization,
            SemanticFactKind::Constraint,
            SemanticFactKind::ConstraintResult,
            SemanticFactKind::Diagnostic,
        ] {
            assert!(
                report.facts.iter().any(|f| f.kind == kind),
                "fixture produces a {kind:?} fact"
            );
        }
        for fact in &report.facts {
            assert_eq!(
                fact.provenance.source_revision,
                Some(test_revision()),
                "{fact:#?}"
            );
            assert!(!fact.provenance.extractor.is_empty(), "{fact:#?}");
        }
    }

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
                    content_digest: None,
                    content_digest_withheld: None,
                },
                ArtifactRecord {
                    id: ArtifactId::new("artifact:unknown"),
                    path: "src/data.xyz".into(),
                    kind: ArtifactKind::File,
                    bytes: 3,
                    disposition: ArtifactDisposition::Unknown,
                    language: None,
                    reason: Some("no-registered-source-frontend".into()),
                    content_digest: None,
                    content_digest_withheld: None,
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
        let report = build_census(&inventory, &source, &adl, &[], &test_revision());

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
            path: String::new(),
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

    /// G119: a dimension's scope coverage is the worst per-artifact coverage, where an artifact is
    /// covered when any engine covers it. One covered artifact never covers the scope (the rule was
    /// best-of-all-obligations before G119, which would have reported CALL OBSERVED at 75 of 98).
    #[test]
    fn scope_coverage_is_the_worst_artifact_and_the_best_engine_per_artifact() {
        let artifact = |id: &str, path: &str| ArtifactRecord {
            id: ArtifactId::new(id),
            path: path.into(),
            kind: ArtifactKind::File,
            bytes: 10,
            disposition: ArtifactDisposition::Parsed,
            language: Some("rust".into()),
            reason: None,
            content_digest: None,
            content_digest_withheld: None,
        };
        let inventory = InventoryReport::new(
            "/repo",
            vec![
                artifact("artifact:src/a.rs", "src/a.rs"),
                artifact("artifact:src/b.rs", "src/b.rs"),
            ],
        );
        let source = SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 2,
            languages: BTreeMap::from([("rust".into(), 2)]),
            files: Vec::new(),
        };
        let adl = compile_adl(&[], &source);
        let covered = extraction_batch_with_one_symbol("src/a.rs");
        let mut open = extraction_batch_with_one_symbol("src/b.rs");
        let diagnostic = atlas_core::ExtractionDiagnostic::new(
            atlas_core::DiagnosticCode::IncompleteAnalysis,
            Some(SemanticDimension::Symbol),
            "test: not exhaustive",
        );
        open.obligations = vec![ObligationResult::unknown_with_observations(
            SemanticDimension::Symbol,
            open.observations
                .iter()
                .map(|o| o.record_id().clone())
                .collect(),
            Vec::new(),
            diagnostic.id.clone(),
        )];
        open.diagnostics = vec![diagnostic];
        let report = build_census(
            &inventory,
            &source,
            &adl,
            &[covered.clone(), open.clone()],
            &test_revision(),
        );
        assert_eq!(
            report.coverage.get("SYMBOL"),
            Some(&EpistemicStatus::Unknown),
            "one uncovered artifact keeps the scope UNKNOWN"
        );
        // A second engine that covers b.rs covers it: the scope is then covered.
        let second = extraction_batch_with_symbol_from(
            "src/b.rs",
            "test.second-engine",
            "evidence:second",
            "second engine",
        );
        let report = build_census(
            &inventory,
            &source,
            &adl,
            &[covered, open, second],
            &test_revision(),
        );
        assert_eq!(
            report.coverage.get("SYMBOL"),
            Some(&EpistemicStatus::Observed)
        );
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
            path: String::new(),
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
                path: String::new(),
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
                    path: String::new(),
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
            CallDispatchKind, CallSiteIdentity, Evidence, EvidenceId, PlaceRef, RepositoryId,
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
            arguments: Vec::new(),
            result: PlaceRef::Unresolved,
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

    /// R4.8: a State access observation, attributed to `extractor_id`, whose `function` (owner)
    /// is a fixed synthetic `FunctionIdentity` record_id -- the test only needs the STATE claim
    /// itself to be independently attributable, not a paired FunctionIdentity observation.
    fn extraction_batch_with_state_from(
        artifact_path: &str,
        extractor_id: &str,
        evidence_id: &str,
        evidence_summary: &str,
    ) -> ExtractionBatch {
        use atlas_core::{
            Evidence, EvidenceId, RepositoryId, SemanticRecordHeader, SemanticRecordId,
            StateAccessIdentity, StateAccessKind, StateResolution, provenance,
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
        let scope = SemanticScope::new(["impl:Owner"]);
        let function = SemanticRecordId::new(SemanticDimension::FunctionIdentity, "owner-fn-key");
        let subject = StateAccessIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            function,
            scope: scope.clone(),
            name: "value".into(),
            span: atlas_core::SourceSpan {
                path: artifact_path.into(),
                line: 3,
                column: 5,
            },
            kind: StateAccessKind::Read,
            resolution: StateResolution::Resolved,
        };
        let record_id = SemanticRecordId::new(SemanticDimension::State, &subject.identity_key());
        let evidence_id = EvidenceId::new(evidence_id.to_owned());
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::State,
            status: EpistemicStatus::Observed,
            scope,
            repository: repository.clone(),
            revision: revision.clone(),
            extractor: extractor.clone(),
            evidence_refs: vec![evidence_id.clone()],
            provenance: provenance(artifact_path, &extractor.id),
            subject,
        };
        let observation = SemanticObservation::State(header);
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
                SemanticDimension::State,
                vec![record_id],
                vec![evidence_id],
            )],
            diagnostics: Vec::new(),
        }
    }

    /// R4.8: a Panic Effect observation, attributed to `extractor_id`, whose `function` (owner)
    /// is a fixed synthetic `FunctionIdentity` record_id.
    fn extraction_batch_with_effect_from(
        artifact_path: &str,
        extractor_id: &str,
        evidence_id: &str,
        evidence_summary: &str,
    ) -> ExtractionBatch {
        use atlas_core::{
            EffectCategory, EffectIdentity, Evidence, EvidenceId, RepositoryId,
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
        let function = SemanticRecordId::new(SemanticDimension::FunctionIdentity, "owner-fn-key");
        let subject = EffectIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            function,
            category: EffectCategory::Panic,
            span: atlas_core::SourceSpan {
                path: artifact_path.into(),
                line: 3,
                column: 5,
            },
        };
        let record_id = SemanticRecordId::new(SemanticDimension::Effect, &subject.identity_key());
        let evidence_id = EvidenceId::new(evidence_id.to_owned());
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::Effect,
            status: EpistemicStatus::Observed,
            scope,
            repository: repository.clone(),
            revision: revision.clone(),
            extractor: extractor.clone(),
            evidence_refs: vec![evidence_id.clone()],
            provenance: provenance(artifact_path, &extractor.id),
            subject,
        };
        let observation = SemanticObservation::Effect(header);
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
                SemanticDimension::Effect,
                vec![record_id],
                vec![evidence_id],
            )],
            diagnostics: Vec::new(),
        }
    }

    /// R4.9: an Ownership operation observation, attributed to `extractor_id`, whose `function`
    /// (owner) is a fixed synthetic `FunctionIdentity` record_id -- the test only needs the
    /// OWNERSHIP claim itself to be independently attributable, not a paired FunctionIdentity
    /// observation.
    fn extraction_batch_with_ownership_from(
        artifact_path: &str,
        extractor_id: &str,
        evidence_id: &str,
        evidence_summary: &str,
    ) -> ExtractionBatch {
        use atlas_core::{
            Evidence, EvidenceId, OwnershipIdentity, OwnershipKind, OwnershipResolution,
            RepositoryId, SemanticRecordHeader, SemanticRecordId, provenance,
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
        let subject = OwnershipIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            function,
            name: "w".into(),
            span: atlas_core::SourceSpan {
                path: artifact_path.into(),
                line: 3,
                column: 5,
            },
            kind: OwnershipKind::BorrowShared,
            resolution: OwnershipResolution::Resolved,
        };
        let record_id =
            SemanticRecordId::new(SemanticDimension::Ownership, &subject.identity_key());
        let evidence_id = EvidenceId::new(evidence_id.to_owned());
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::Ownership,
            status: EpistemicStatus::Observed,
            scope,
            repository: repository.clone(),
            revision: revision.clone(),
            extractor: extractor.clone(),
            evidence_refs: vec![evidence_id.clone()],
            provenance: provenance(artifact_path, &extractor.id),
            subject,
        };
        let observation = SemanticObservation::Ownership(header);
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
                SemanticDimension::Ownership,
                vec![record_id],
                vec![evidence_id],
            )],
            diagnostics: Vec::new(),
        }
    }

    fn extraction_batch_with_concurrency_from(
        artifact_path: &str,
        extractor_id: &str,
        evidence_id: &str,
        evidence_summary: &str,
    ) -> ExtractionBatch {
        use atlas_core::{
            ConcurrencyIdentity, ConcurrencyKind, Evidence, EvidenceId, RepositoryId,
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
        let function = SemanticRecordId::new(SemanticDimension::FunctionIdentity, "owner-fn-key");
        let subject = ConcurrencyIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            function,
            kind: ConcurrencyKind::Await,
            span: atlas_core::SourceSpan {
                path: artifact_path.into(),
                line: 3,
                column: 5,
            },
        };
        let record_id =
            SemanticRecordId::new(SemanticDimension::Concurrency, &subject.identity_key());
        let evidence_id = EvidenceId::new(evidence_id.to_owned());
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::Concurrency,
            status: EpistemicStatus::Observed,
            scope,
            repository: repository.clone(),
            revision: revision.clone(),
            extractor: extractor.clone(),
            evidence_refs: vec![evidence_id.clone()],
            provenance: provenance(artifact_path, &extractor.id),
            subject,
        };
        let observation = SemanticObservation::Concurrency(header);
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
                SemanticDimension::Concurrency,
                vec![record_id],
                vec![evidence_id],
            )],
            diagnostics: Vec::new(),
        }
    }

    fn extraction_batch_with_persistence_from(
        artifact_path: &str,
        extractor_id: &str,
        evidence_id: &str,
        evidence_summary: &str,
    ) -> ExtractionBatch {
        use atlas_core::{
            Evidence, EvidenceId, PersistenceIdentity, PersistenceKind, PersistenceResolution,
            PlaceRef, RepositoryId, SemanticRecordHeader, SemanticRecordId, provenance,
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
        let subject = PersistenceIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            function,
            kind: PersistenceKind::Commit,
            span: atlas_core::SourceSpan {
                path: artifact_path.into(),
                line: 3,
                column: 5,
            },
            place: PlaceRef::Unresolved,
            resolution: PersistenceResolution::Unresolved,
        };
        let record_id =
            SemanticRecordId::new(SemanticDimension::Persistence, &subject.identity_key());
        let evidence_id = EvidenceId::new(evidence_id.to_owned());
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::Persistence,
            status: EpistemicStatus::Inferred,
            scope,
            repository: repository.clone(),
            revision: revision.clone(),
            extractor: extractor.clone(),
            evidence_refs: vec![evidence_id.clone()],
            provenance: provenance(artifact_path, &extractor.id),
            subject,
        };
        let observation = SemanticObservation::Persistence(header);
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
            obligations: vec![ObligationResult::unknown_with_observations(
                SemanticDimension::Persistence,
                vec![record_id],
                vec![evidence_id],
                "diagnostic:test-persistence-partial-coverage",
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
                content_digest: None,
                content_digest_withheld: None,
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

    fn span() -> SourceSpan {
        SourceSpan {
            path: "atlas.adl".into(),
            line: 1,
            column: 1,
        }
    }

    #[test]
    fn two_different_declared_edges_never_collapse_into_one_fact_via_an_unescaped_join() {
        // `edge.from`/`edge.relation`/`edge.to` are ADL-authored free text, extracted by
        // `parse_relation`'s simple `split_once("->")`, not a restrictive lexer -- the same field
        // set `core::language::adl::mod`'s own `DeclaredEdge.id` already escapes for this exact
        // reason. `fact_id` here joins them again, independently, with no escaping: two genuinely
        // different declared relations must never collapse onto one `SemanticFact.id`.
        let (inventory, source, mut adl) = single_rust_file_context();
        adl.ir.declared.edges = vec![
            DeclaredEdge {
                id: "edge-a".into(),
                from: "Foo:bar".into(),
                relation: "baz".into(),
                to: "X".into(),
                origin: "declared".into(),
                span: span(),
            },
            DeclaredEdge {
                id: "edge-b".into(),
                from: "Foo".into(),
                relation: "bar:baz".into(),
                to: "X".into(),
                origin: "declared".into(),
                span: span(),
            },
        ];
        let census = build_census(&inventory, &source, &adl, &[], &test_revision());
        let ids: std::collections::BTreeSet<_> = census
            .facts
            .iter()
            .filter(|fact| fact.kind == SemanticFactKind::DeclaredEdge)
            .map(|fact| fact.id.clone())
            .collect();
        assert_eq!(
            ids.len(),
            2,
            "two genuinely different declared edges must never collapse onto one fact id"
        );
    }

    #[test]
    fn two_different_bindings_never_collapse_into_one_fact_via_an_unescaped_join() {
        // Same defect class as the DeclaredEdge case above, for `binding.consumer`/`.capability`/
        // `.provider` feeding the `binds_to` fact's id (joined as `"{consumer}:{capability}:
        // {provider}"`, so the crafted collision shifts the boundary between `consumer` and
        // `capability`, holding `provider` fixed).
        let (inventory, source, mut adl) = single_rust_file_context();
        adl.ir.declared.bindings = vec![
            BindingDecl {
                name: "bind-a".into(),
                consumer: "Foo:bar".into(),
                capability: "baz".into(),
                provider: "X".into(),
                span: span(),
            },
            BindingDecl {
                name: "bind-b".into(),
                consumer: "Foo".into(),
                capability: "bar:baz".into(),
                provider: "X".into(),
                span: span(),
            },
        ];
        let census = build_census(&inventory, &source, &adl, &[], &test_revision());
        let ids: std::collections::BTreeSet<_> = census
            .facts
            .iter()
            .filter(|fact| fact.kind == SemanticFactKind::Binding && fact.predicate == "binds_to")
            .map(|fact| fact.id.clone())
            .collect();
        assert_eq!(
            ids.len(),
            2,
            "two genuinely different bindings must never collapse onto one fact id"
        );
    }

    #[test]
    fn real_extraction_observations_reach_census_before_normalization() {
        let (inventory, source, adl) = single_rust_file_context();
        let batch = extraction_batch_with_one_symbol("src/lib.rs");
        let batches = [batch];

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());

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

        let a = build_census(&inventory, &source, &adl, &batches, &test_revision());
        let b = build_census(&inventory, &source, &adl, &batches, &test_revision());
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

        let census = build_census(&inventory, &source, &adl, &[batch], &test_revision());

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

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
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

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
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

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
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
        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());

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
        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());

        let signature_fact = census
            .facts
            .iter()
            .find(|fact| fact.kind == SemanticFactKind::FunctionSignature)
            .expect("compatibility FunctionSignature fact present");
        // The compatibility projection's display summary now DOES represent ABI/visibility (a
        // real gap this session's own fix closed -- `FunctionSignature::summary()` previously
        // silently dropped them). This test's real point survives regardless: the typed world
        // below is never DERIVED from `facts`, so its correctness never depended on this string's
        // completeness in the first place -- proven concretely by the `facts.clear()` step below,
        // not by this string being incomplete.
        assert!(signature_fact.object.contains("extern"));
        assert!(signature_fact.object.contains("pub"));

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

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
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

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
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

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
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

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
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

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
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

    // === R4.8: State/Effect claims stay multi-extractor safe =======================================
    //
    // Same proof, for the two new R4.8 dimensions: two independent extractors observing the exact
    // same claim must both survive Census/Normalization, share one claim identity (record_id), and
    // remain distinctly attributable by raw_observation_id/extractor id.

    #[test]
    fn same_state_access_claim_from_two_extractors_survives_census_and_normalization() {
        let (inventory, source, adl) = single_rust_file_context();
        let batches = [
            extraction_batch_with_state_from(
                "src/lib.rs",
                "extractor-a",
                "evidence:state-a",
                "extractor-a observed a state access at src/lib.rs",
            ),
            extraction_batch_with_state_from(
                "src/lib.rs",
                "extractor-b",
                "evidence:state-b",
                "extractor-b observed a state access at src/lib.rs",
            ),
        ];

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
        let accesses: Vec<_> = census
            .typed_semantic_records
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::State(_)))
            .collect();
        assert_eq!(
            accesses.len(),
            2,
            "independent extractors' observations of the same state access must both survive Census"
        );

        let claim_ids: std::collections::BTreeSet<&str> = accesses
            .iter()
            .map(|observation| observation.record_id().as_str())
            .collect();
        assert_eq!(
            claim_ids.len(),
            1,
            "both observations share the same State claim"
        );

        let raw_ids: std::collections::BTreeSet<_> = accesses
            .iter()
            .map(|observation| observation.raw_observation_id())
            .collect();
        assert_eq!(raw_ids.len(), 2);

        let extractor_ids: std::collections::BTreeSet<&str> = accesses
            .iter()
            .map(|observation| {
                let SemanticObservation::State(header) = observation else {
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
            .filter(|observation| matches!(observation, SemanticObservation::State(_)))
            .collect();
        assert_eq!(normalized.len(), 2);
    }

    #[test]
    fn same_effect_claim_from_two_extractors_survives_census_and_normalization() {
        let (inventory, source, adl) = single_rust_file_context();
        let batches = [
            extraction_batch_with_effect_from(
                "src/lib.rs",
                "extractor-a",
                "evidence:effect-a",
                "extractor-a observed a panic effect at src/lib.rs",
            ),
            extraction_batch_with_effect_from(
                "src/lib.rs",
                "extractor-b",
                "evidence:effect-b",
                "extractor-b observed a panic effect at src/lib.rs",
            ),
        ];

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
        let effects: Vec<_> = census
            .typed_semantic_records
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::Effect(_)))
            .collect();
        assert_eq!(
            effects.len(),
            2,
            "independent extractors' observations of the same effect must both survive Census"
        );

        let claim_ids: std::collections::BTreeSet<&str> = effects
            .iter()
            .map(|observation| observation.record_id().as_str())
            .collect();
        assert_eq!(
            claim_ids.len(),
            1,
            "both observations share the same Effect claim"
        );

        let raw_ids: std::collections::BTreeSet<_> = effects
            .iter()
            .map(|observation| observation.raw_observation_id())
            .collect();
        assert_eq!(raw_ids.len(), 2);

        let extractor_ids: std::collections::BTreeSet<&str> = effects
            .iter()
            .map(|observation| {
                let SemanticObservation::Effect(header) = observation else {
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
            .filter(|observation| matches!(observation, SemanticObservation::Effect(_)))
            .collect();
        assert_eq!(normalized.len(), 2);
    }

    // === R4.9: Ownership claims stay multi-extractor safe ===========================================
    //
    // Same proof, for the new R4.9 dimension: two independent extractors observing the exact same
    // claim must both survive Census/Normalization, share one claim identity (record_id), and
    // remain distinctly attributable by raw_observation_id/extractor id.

    #[test]
    fn same_ownership_claim_from_two_extractors_survives_census_and_normalization() {
        let (inventory, source, adl) = single_rust_file_context();
        let batches = [
            extraction_batch_with_ownership_from(
                "src/lib.rs",
                "extractor-a",
                "evidence:ownership-a",
                "extractor-a observed a borrow at src/lib.rs",
            ),
            extraction_batch_with_ownership_from(
                "src/lib.rs",
                "extractor-b",
                "evidence:ownership-b",
                "extractor-b observed a borrow at src/lib.rs",
            ),
        ];

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
        let ops: Vec<_> = census
            .typed_semantic_records
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::Ownership(_)))
            .collect();
        assert_eq!(
            ops.len(),
            2,
            "independent extractors' observations of the same ownership op must both survive Census"
        );

        let claim_ids: std::collections::BTreeSet<&str> = ops
            .iter()
            .map(|observation| observation.record_id().as_str())
            .collect();
        assert_eq!(
            claim_ids.len(),
            1,
            "both observations share the same Ownership claim"
        );

        let raw_ids: std::collections::BTreeSet<_> = ops
            .iter()
            .map(|observation| observation.raw_observation_id())
            .collect();
        assert_eq!(raw_ids.len(), 2);

        let extractor_ids: std::collections::BTreeSet<&str> = ops
            .iter()
            .map(|observation| {
                let SemanticObservation::Ownership(header) = observation else {
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
            .filter(|observation| matches!(observation, SemanticObservation::Ownership(_)))
            .collect();
        assert_eq!(normalized.len(), 2);
    }

    // === R4.10: Concurrency claims stay multi-extractor safe ========================================
    //
    // Same proof, for the new R4.10 dimension: two independent extractors observing the exact same
    // claim must both survive Census/Normalization, share one claim identity (record_id), and
    // remain distinctly attributable by raw_observation_id/extractor id.

    #[test]
    fn same_concurrency_claim_from_two_extractors_survives_census_and_normalization() {
        let (inventory, source, adl) = single_rust_file_context();
        let batches = [
            extraction_batch_with_concurrency_from(
                "src/lib.rs",
                "extractor-a",
                "evidence:concurrency-a",
                "extractor-a observed an await at src/lib.rs",
            ),
            extraction_batch_with_concurrency_from(
                "src/lib.rs",
                "extractor-b",
                "evidence:concurrency-b",
                "extractor-b observed an await at src/lib.rs",
            ),
        ];

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
        let ops: Vec<_> = census
            .typed_semantic_records
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::Concurrency(_)))
            .collect();
        assert_eq!(
            ops.len(),
            2,
            "independent extractors' observations of the same concurrency op must both survive Census"
        );

        let claim_ids: std::collections::BTreeSet<&str> = ops
            .iter()
            .map(|observation| observation.record_id().as_str())
            .collect();
        assert_eq!(
            claim_ids.len(),
            1,
            "both observations share the same Concurrency claim"
        );

        let raw_ids: std::collections::BTreeSet<_> = ops
            .iter()
            .map(|observation| observation.raw_observation_id())
            .collect();
        assert_eq!(raw_ids.len(), 2);

        let extractor_ids: std::collections::BTreeSet<&str> = ops
            .iter()
            .map(|observation| {
                let SemanticObservation::Concurrency(header) = observation else {
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
            .filter(|observation| matches!(observation, SemanticObservation::Concurrency(_)))
            .collect();
        assert_eq!(normalized.len(), 2);
    }

    // Same proof, for the new R4.11 dimension: two independent extractors observing the exact same
    // persistence candidate claim must both survive Census/Normalization, share one claim identity
    // (record_id), and remain distinctly attributable by raw_observation_id/extractor id.

    #[test]
    fn same_persistence_claim_from_two_extractors_survives_census_and_normalization() {
        let (inventory, source, adl) = single_rust_file_context();
        let batches = [
            extraction_batch_with_persistence_from(
                "src/lib.rs",
                "extractor-a",
                "evidence:persistence-a",
                "extractor-a observed a commit candidate at src/lib.rs",
            ),
            extraction_batch_with_persistence_from(
                "src/lib.rs",
                "extractor-b",
                "evidence:persistence-b",
                "extractor-b observed a commit candidate at src/lib.rs",
            ),
        ];

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
        let ops: Vec<_> = census
            .typed_semantic_records
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::Persistence(_)))
            .collect();
        assert_eq!(
            ops.len(),
            2,
            "independent extractors' observations of the same persistence candidate must both survive Census"
        );

        let claim_ids: std::collections::BTreeSet<&str> = ops
            .iter()
            .map(|observation| observation.record_id().as_str())
            .collect();
        assert_eq!(
            claim_ids.len(),
            1,
            "both observations share the same Persistence claim"
        );

        let raw_ids: std::collections::BTreeSet<_> = ops
            .iter()
            .map(|observation| observation.raw_observation_id())
            .collect();
        assert_eq!(raw_ids.len(), 2);

        let extractor_ids: std::collections::BTreeSet<&str> = ops
            .iter()
            .map(|observation| {
                let SemanticObservation::Persistence(header) = observation else {
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
            .filter(|observation| matches!(observation, SemanticObservation::Persistence(_)))
            .collect();
        assert_eq!(normalized.len(), 2);
    }

    // --- Required test 1: same semantic claim from two extractors survives Census -------------

    #[test]
    fn same_semantic_claim_from_two_extractors_survives_independently_through_census() {
        let (inventory, source, adl) = single_rust_file_context();
        let batches = two_extractor_symbol_batches();

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
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

    // `record_id` is deliberately SHARED across independent extractors observing the same claim
    // (the whole point of the assertion just above), so `semantic_observation_fact`'s own `id`
    // seed must fold in extractor identity too -- otherwise two real, independently-provenanced
    // `SemanticFact`s collide on the one field whose entire purpose is stable per-fact identity,
    // exactly the discipline `obligation_status_fact` already correctly applies.
    #[test]
    fn two_extractors_agreeing_on_the_same_symbol_produce_two_facts_with_distinct_ids() {
        let (inventory, source, adl) = single_rust_file_context();
        let batches = two_extractor_symbol_batches();

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
        let symbol_fact_ids: Vec<&str> = census
            .facts
            .iter()
            .filter(|fact| fact.kind == SemanticFactKind::Symbol)
            .map(|fact| fact.id.as_str())
            .collect();
        assert_eq!(
            symbol_fact_ids.len(),
            2,
            "both extractors' Symbol facts must survive into the compatibility projection"
        );
        assert_ne!(
            symbol_fact_ids[0], symbol_fact_ids[1],
            "two facts with different provenance/extractor must never share an id, even when \
             they agree on the same underlying claim (record_id)"
        );
    }

    // --- Required test 2: same, but through normalization --------------------------------------

    #[test]
    fn same_semantic_claim_from_two_extractors_survives_independently_through_normalization() {
        let (inventory, source, adl) = single_rust_file_context();
        let batches = two_extractor_symbol_batches();

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
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

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
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

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
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
        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());

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
        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());

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

        let census = build_census(&inventory, &source, &adl, &batches, &test_revision());
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

    // === R4.12: the named R4 Rust reference profile, run through the FULL canonical path ==========
    //
    // `.atlas/contracts/SEMANTIC-EXTRACTION.md#r4-definition-of-done` (item 1) requires naming the
    // reference corpus a project-wide R4-complete claim is proven against.
    // `adapter::semantic::rust::tests` already proves this exact corpus is evidence-producing for
    // every mandatory dimension at the extractor's own boundary; this test proves the same corpus
    // survives the REST of the canonical path -- Census -> Normalize -- untouched: no exact
    // duplicates spuriously collapsed, no conflict candidates from a single, internally consistent
    // extractor's own output, and `is_closed()`/`typed_semantics_closed()` both hold on the real
    // `NormalizationReport` this pipeline produces (not a hand-built fixture).

    const R4_REFERENCE_PROFILE_CORPUS: &str = r#"
pub struct Widget {
    pub value: u64,
}

impl Widget {
    pub fn new(value: u64) -> Self {
        Widget { value }
    }

    pub fn get(&self) -> u64 {
        self.value
    }

    pub fn set(&mut self, new_value: u64) {
        self.value = new_value;
    }

    pub fn maybe_panic(&self, ok: bool) -> u64 {
        if !ok {
            panic!("not ok");
        }
        self.value
    }
}

pub fn helper(x: u64) -> u64 {
    x
}

pub fn caller(w: &Widget) -> u64 {
    let doubled = helper(w.get()) * 2;
    doubled
}

pub fn borrow_widget(w: &Widget) -> u64 {
    w.value
}

pub async fn awaits_something(x: u64) -> u64 {
    x.await
}

fn do_work() {}

pub fn spawns_work() {
    thread::spawn(do_work);
}

pub struct Store;
impl Store {
    pub fn commit(&mut self) {}
}

pub fn commits_a_store(store: &mut Store) {
    store.commit();
}
"#;

    fn extract_reference_profile() -> ExtractionBatch {
        use adapter::ExtractionInput;
        use atlas_core::ContentFingerprint;

        let extractor = adapter::extractors_for_language("rust")
            .into_iter()
            .next()
            .expect("rust has a registered extractor");
        let input = ExtractionInput {
            repository: atlas_core::RepositoryId::new("atlas-studio"),
            revision: atlas_core::RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            artifact: ArtifactId::new("artifact:src/lib.rs"),
            artifact_path: "src/lib.rs".into(),
            source_text: R4_REFERENCE_PROFILE_CORPUS.into(),
            content_fingerprint: Some(ContentFingerprint("sha256:fixture".into())),
            source_frontend_id: "atlas.source.rust.bootstrap.v1".into(),
            language: "rust".into(),
            build_profile: None,
            scope_policy: None,
            requested_dimensions: ALL_SEMANTIC_DIMENSIONS.to_vec(),
        };
        extractor.extract(&input)
    }

    #[test]
    fn reference_profile_survives_census_and_normalization_with_no_duplicates_or_conflicts() {
        let (inventory, source, adl) = single_rust_file_context();
        let batch = extract_reference_profile();
        assert!(batch.is_closed(&ALL_SEMANTIC_DIMENSIONS));

        let census = build_census(&inventory, &source, &adl, &[batch], &test_revision());
        assert!(census.is_closed());
        for &dimension in &ALL_SEMANTIC_DIMENSIONS {
            assert_ne!(
                census.coverage.get(dimension.as_str()),
                Some(&EpistemicStatus::Unsupported),
                "{dimension:?} must not be Unsupported in the R4 reference profile"
            );
            let observed = census
                .typed_semantic_records
                .iter()
                .any(|observation| observation.dimension() == dimension);
            assert!(
                observed,
                "{dimension:?} produced zero observations reaching Census in the R4 reference \
                 profile"
            );
        }

        let normalization = crate::normalize::normalize(&census);
        assert!(normalization.is_closed());
        assert_eq!(
            normalization.exact_duplicates_merged, 0,
            "a single, internally consistent extractor's own output must not collapse against \
             itself"
        );
        assert!(
            normalization.conflict_candidates.is_empty(),
            "a single extractor disagreeing with itself would be a real bug, not the \
             multi-extractor scenario conflict detection exists for"
        );
    }

    #[test]
    fn build_census_obligation_dedup_is_order_independent_even_when_the_same_key_carries_differing_content()
     {
        // `obligation_id` is a coordinate identity (repository/revision/artifact/extractor/
        // dimension), so three batches sharing that coordinate but reporting DIFFERENT obligation
        // content for the same dimension is exactly the "coordinate-identity invariant violated"
        // scenario this dedup exists to handle (a caller merging results from more than one
        // extraction run for the same artifact/extractor pair). The first and third batches report
        // byte-identical `ObligationResult::observed(Symbol, [], [])`; the second (sandwiched
        // between them) reports a genuinely different `unknown` result for the SAME dimension.
        // Sorting by `obligation_id` alone is coarser than the full-equality `dedup_by`
        // comparator, so whether the equal pair ends up adjacent after a stable sort -- and thus
        // whether it collapses -- must not depend on which of two equivalent input orderings this
        // function happens to receive.
        let (inventory, source, adl) = single_rust_file_context();
        let repository = atlas_core::RepositoryId::new("atlas-studio");
        let revision = atlas_core::RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let extractor = ExtractorIdentity {
            id: "atlas.rust.source-semantic.v1".into(),
            version: "0.1.0".into(),
        };
        let artifact = ArtifactId::new("artifact:src/lib.rs");
        let observed_batch = ExtractionBatch {
            extractor: extractor.clone(),
            repository: repository.clone(),
            revision: revision.clone(),
            artifact: artifact.clone(),
            input_fingerprint: "test:src/lib.rs:observed".into(),
            observations: Vec::new(),
            evidence: Vec::new(),
            obligations: vec![ObligationResult::observed(
                SemanticDimension::Symbol,
                Vec::new(),
                Vec::new(),
            )],
            diagnostics: Vec::new(),
        };
        let unknown_batch = ExtractionBatch {
            extractor: extractor.clone(),
            repository: repository.clone(),
            revision: revision.clone(),
            artifact: artifact.clone(),
            input_fingerprint: "test:src/lib.rs:unknown".into(),
            observations: Vec::new(),
            evidence: Vec::new(),
            obligations: vec![ObligationResult::unknown(
                SemanticDimension::Symbol,
                "test-diagnostic",
            )],
            diagnostics: Vec::new(),
        };

        let sandwiched = [
            observed_batch.clone(),
            unknown_batch.clone(),
            observed_batch.clone(),
        ];
        let adjacent = [
            observed_batch.clone(),
            observed_batch.clone(),
            unknown_batch.clone(),
        ];

        let sandwiched_census =
            build_census(&inventory, &source, &adl, &sandwiched, &test_revision());
        let adjacent_census = build_census(&inventory, &source, &adl, &adjacent, &test_revision());

        assert_eq!(
            sandwiched_census.typed_obligations.len(),
            2,
            "the two byte-identical `observed` obligations must collapse to one, leaving the \
             `unknown` one distinct: {:?}",
            sandwiched_census.typed_obligations
        );
        assert_eq!(
            sandwiched_census.typed_obligations.len(),
            adjacent_census.typed_obligations.len(),
            "the same logical multiset of obligations must dedup to the same count regardless \
             of input order: sandwiched={:?}, adjacent={:?}",
            sandwiched_census.typed_obligations,
            adjacent_census.typed_obligations
        );
    }
}
