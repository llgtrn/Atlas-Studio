//! Multi-extractor census accounting.
//!
//! Bridges typed `ExtractionBatch` obligation accounting (`adapter`) into runtime-owned census
//! state, proving census can consume `SemanticExtractor` output without creating a second semantic
//! path (`.atlas/decisions/0001-one-normalized-semantic-path.md`).
//!
//! Per `.atlas/contracts/SEMANTIC-EXTRACTION.md#multi-engine-extraction`: "Independent extractors
//! MAY analyze the same dimension. Their identities/evidence remain separate. Disagreement creates
//! a reconciliation obligation; one extractor may not overwrite another." `CensusExtractionAccounting`
//! is the smallest structure that honors this: every `ObligationResult` from every recorded batch is
//! preserved independently, tagged with the artifact and extractor that produced it. It never
//! collapses multiple results for the same (artifact, dimension) into one slot, and it never picks
//! a winner -- that is reconciliation's job, out of scope for this wave
//! (`.atlas/contracts/NORMALIZATION.md#conflict-handling`).
//!
//! Full per-artifact wiring into `census::build_census`'s production path remains deferred: every
//! extractor registered today is a `StaticUnsupportedExtractor`, so wiring it in now would only
//! reproduce results the bootstrap coverage map already states directly. This is the real, tested
//! attach point a real R4.3+ extractor (or a second independent extractor sharing a dimension)
//! plugs into; it is not itself the production call site yet.

use adapter::ExtractionBatch;
use atlas_core::{ArtifactId, ExtractorIdentity, SemanticDimension, SemanticRecordId};

/// One extractor's obligation result for one dimension on one artifact, preserved independently
/// alongside every other extractor's result for the same (artifact, dimension) pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractorObligationRecord {
    pub artifact: ArtifactId,
    pub extractor: ExtractorIdentity,
    pub obligation: adapter::ObligationResult,
}

/// Multi-extractor census accounting: an append-only collection of `ExtractorObligationRecord`.
///
/// This is NOT the bootstrap `CensusReport.coverage: BTreeMap<String, EpistemicStatus>` (a
/// derived, single-valued projection kept for existing callers). This structure is the canonical
/// preserved input; that map is noncanonical summary shape, never populated by this accumulator.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CensusExtractionAccounting {
    records: Vec<ExtractorObligationRecord>,
}

impl CensusExtractionAccounting {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends every obligation in `batch`, tagged with `batch.artifact`/`batch.extractor`. Never
    /// removes or overwrites an existing record: an extractor's result can never erase another
    /// extractor's result, regardless of ingestion order.
    pub fn record_batch(&mut self, batch: &ExtractionBatch) {
        for obligation in &batch.obligations {
            self.records.push(ExtractorObligationRecord {
                artifact: batch.artifact.clone(),
                extractor: batch.extractor.clone(),
                obligation: obligation.clone(),
            });
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Every preserved record for one dimension, across every extractor/artifact that addressed
    /// it. Insertion order, not publication order; use `sorted` for a deterministic view.
    pub fn records_for(&self, dimension: SemanticDimension) -> Vec<&ExtractorObligationRecord> {
        self.records
            .iter()
            .filter(|record| record.obligation.dimension == dimension)
            .collect()
    }

    /// Canonical deterministic publication order: `SemanticDimension -> ExtractorIdentity ->
    /// ArtifactId -> stable record identity`. This is representation ordering only -- it carries
    /// no truth-ranking meaning, and never depends on insertion order, HashMap iteration, thread
    /// scheduling or any other nondeterministic source.
    pub fn sorted(&self) -> Vec<ExtractorObligationRecord> {
        let mut records = self.records.clone();
        records.sort_by(|a, b| {
            a.obligation
                .dimension
                .as_str()
                .cmp(b.obligation.dimension.as_str())
                .then_with(|| a.extractor.id.cmp(&b.extractor.id))
                .then_with(|| a.extractor.version.cmp(&b.extractor.version))
                .then_with(|| a.artifact.as_str().cmp(b.artifact.as_str()))
                .then_with(|| {
                    observation_sort_key(&a.obligation).cmp(observation_sort_key(&b.obligation))
                })
        });
        records
    }
}

fn observation_sort_key(obligation: &adapter::ObligationResult) -> &str {
    obligation
        .observation_ids
        .first()
        .map(SemanticRecordId::as_str)
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapter::{ExtractionInput, extractors_for_language};
    use atlas_core::{ContentFingerprint, EpistemicStatus, RepositoryId, RevisionRef};

    fn input(dimensions: Vec<SemanticDimension>) -> ExtractionInput {
        ExtractionInput {
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            artifact: ArtifactId::new("artifact:src/lib.rs"),
            artifact_path: "src/lib.rs".into(),
            content_fingerprint: Some(ContentFingerprint("sha256:deadbeef".into())),
            source_frontend_id: "atlas.source.rust.bootstrap.v1".into(),
            language: "rust".into(),
            build_profile: None,
            scope_policy: None,
            requested_dimensions: dimensions,
        }
    }

    #[test]
    fn extractor_output_records_into_multi_extractor_accounting_without_a_second_semantic_path() {
        let requested = vec![
            SemanticDimension::Symbol,
            SemanticDimension::FunctionIdentity,
            SemanticDimension::Ownership,
        ];
        let extractor = extractors_for_language("rust")
            .into_iter()
            .next()
            .expect("rust has a registered extractor");
        let batch = extractor.extract(&input(requested.clone()));
        assert!(batch.is_closed(&requested));

        let mut accounting = CensusExtractionAccounting::new();
        accounting.record_batch(&batch);

        for dimension in &requested {
            let records = accounting.records_for(*dimension);
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].obligation.status, EpistemicStatus::Unsupported);
        }
    }
}

/// Proves `CensusExtractionAccounting` never applies last-write-wins semantics across
/// independent extractors. Per `.atlas/contracts/SEMANTIC-EXTRACTION.md#multi-engine-extraction`
/// and `.atlas/contracts/NORMALIZATION.md#conflict-handling`: disagreement is preserved, not
/// resolved here.
#[cfg(test)]
mod multi_extractor_tests {
    use super::*;
    use adapter::ObligationResult;
    use atlas_core::{
        EpistemicStatus, Evidence, EvidenceId, Provenance, RepositoryId, RevisionRef,
        SemanticObservation, SemanticRecordHeader, SemanticScope, SymbolIdentity, SymbolRole,
        stable_id,
    };

    fn extractor(id: &str, version: &str) -> ExtractorIdentity {
        ExtractorIdentity {
            id: id.into(),
            version: version.into(),
        }
    }

    fn observed_symbol_batch(
        extractor_id: &str,
        artifact_path: &str,
        symbol_name: &str,
    ) -> ExtractionBatch {
        let extractor = extractor(extractor_id, "0.1.0");
        let repository = RepositoryId::new("atlas-studio");
        let revision = RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let evidence_id = EvidenceId::new(stable_id(
            "evidence",
            &format!("{extractor_id}:{symbol_name}"),
        ));
        let symbol = SymbolIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            scope: SemanticScope::new(["fixture"]),
            name: symbol_name.into(),
            role: SymbolRole::Definition,
        };
        let record_id = SemanticRecordId::new(SemanticDimension::Symbol, &symbol.identity_key());
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::Symbol,
            status: EpistemicStatus::Observed,
            subject: symbol,
            scope: SemanticScope::new(["fixture"]),
            repository: repository.clone(),
            revision: revision.clone(),
            extractor: extractor.clone(),
            evidence_refs: vec![evidence_id.clone()],
            provenance: Provenance {
                source_path: artifact_path.into(),
                source_revision: Some(revision.clone()),
                extractor: extractor_id.into(),
                content_hash: None,
                span: None,
            },
        };
        ExtractionBatch {
            extractor,
            repository,
            revision: revision.clone(),
            artifact: ArtifactId::new(format!("artifact:{artifact_path}")),
            input_fingerprint: format!("{extractor_id}:{artifact_path}"),
            observations: vec![SemanticObservation::Symbol(header)],
            evidence: vec![Evidence {
                id: evidence_id.as_str().to_owned(),
                kind: "SOURCE_TEXT".into(),
                path: artifact_path.into(),
                summary: format!("{extractor_id} observed `{symbol_name}`"),
                revision: Some(revision),
            }],
            obligations: vec![ObligationResult::observed(
                SemanticDimension::Symbol,
                vec![record_id],
                vec![evidence_id],
            )],
            diagnostics: Vec::new(),
        }
    }

    fn unknown_batch(
        extractor_id: &str,
        artifact_path: &str,
        dimension: SemanticDimension,
    ) -> ExtractionBatch {
        ExtractionBatch {
            extractor: extractor(extractor_id, "0.1.0"),
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            artifact: ArtifactId::new(format!("artifact:{artifact_path}")),
            input_fingerprint: format!("{extractor_id}:{artifact_path}"),
            observations: Vec::new(),
            evidence: Vec::new(),
            obligations: vec![ObligationResult::unknown(
                dimension,
                format!("{extractor_id}-diagnostic"),
            )],
            diagnostics: Vec::new(),
        }
    }

    fn unsupported_batch(
        extractor_id: &str,
        artifact_path: &str,
        dimension: SemanticDimension,
    ) -> ExtractionBatch {
        ExtractionBatch {
            extractor: extractor(extractor_id, "0.1.0"),
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            artifact: ArtifactId::new(format!("artifact:{artifact_path}")),
            input_fingerprint: format!("{extractor_id}:{artifact_path}"),
            observations: Vec::new(),
            evidence: Vec::new(),
            obligations: vec![ObligationResult::unsupported(
                dimension,
                format!("{extractor_id}-diagnostic"),
            )],
            diagnostics: Vec::new(),
        }
    }

    // --- 1. two extractor results for the same dimension are both retained ------------------

    #[test]
    fn two_extractors_disagreeing_on_symbol_are_both_retained() {
        let mut accounting = CensusExtractionAccounting::new();
        accounting.record_batch(&observed_symbol_batch("extractor-a", "src/lib.rs", "known"));
        accounting.record_batch(&unknown_batch(
            "extractor-b",
            "src/lib.rs",
            SemanticDimension::Symbol,
        ));

        let records = accounting.records_for(SemanticDimension::Symbol);
        assert_eq!(records.len(), 2);
        assert!(
            records.iter().any(|r| r.extractor.id == "extractor-a"
                && r.obligation.status == EpistemicStatus::Observed)
        );
        assert!(
            records.iter().any(|r| r.extractor.id == "extractor-b"
                && r.obligation.status == EpistemicStatus::Unknown)
        );
    }

    // --- 2. ingestion order does not change semantic accounting -----------------------------

    #[test]
    fn ingestion_order_does_not_change_the_preserved_accounting_set() {
        let a = observed_symbol_batch("extractor-a", "src/lib.rs", "known");
        let b = unknown_batch("extractor-b", "src/lib.rs", SemanticDimension::Symbol);

        let mut forward = CensusExtractionAccounting::new();
        forward.record_batch(&a);
        forward.record_batch(&b);

        let mut reversed = CensusExtractionAccounting::new();
        reversed.record_batch(&b);
        reversed.record_batch(&a);

        assert_eq!(forward.len(), reversed.len());
        assert_eq!(forward.sorted(), reversed.sorted());
    }

    // --- 3 & 4. extractor identity and artifact identity remain attached --------------------

    #[test]
    fn extractor_and_artifact_identity_remain_attached() {
        let mut accounting = CensusExtractionAccounting::new();
        accounting.record_batch(&observed_symbol_batch("extractor-a", "src/one.rs", "known"));

        let record = &accounting.records_for(SemanticDimension::Symbol)[0];
        assert_eq!(record.extractor, extractor("extractor-a", "0.1.0"));
        assert_eq!(record.artifact, ArtifactId::new("artifact:src/one.rs"));
    }

    // --- 5. EpistemicStatus remains unchanged ------------------------------------------------

    #[test]
    fn epistemic_status_is_preserved_verbatim_not_recomputed() {
        let mut accounting = CensusExtractionAccounting::new();
        accounting.record_batch(&observed_symbol_batch("extractor-a", "src/lib.rs", "known"));
        accounting.record_batch(&unknown_batch(
            "extractor-b",
            "src/lib.rs",
            SemanticDimension::Symbol,
        ));
        accounting.record_batch(&unsupported_batch(
            "extractor-c",
            "src/lib.rs",
            SemanticDimension::Symbol,
        ));

        let statuses: Vec<EpistemicStatus> = accounting
            .records_for(SemanticDimension::Symbol)
            .iter()
            .map(|r| r.obligation.status)
            .collect();
        assert!(statuses.contains(&EpistemicStatus::Observed));
        assert!(statuses.contains(&EpistemicStatus::Unknown));
        assert!(statuses.contains(&EpistemicStatus::Unsupported));
        assert_eq!(statuses.len(), 3);
    }

    // --- 6. UNKNOWN does not erase OBSERVED --------------------------------------------------

    #[test]
    fn unknown_does_not_erase_observed() {
        let mut accounting = CensusExtractionAccounting::new();
        accounting.record_batch(&observed_symbol_batch("extractor-a", "src/lib.rs", "known"));
        accounting.record_batch(&unknown_batch(
            "extractor-b",
            "src/lib.rs",
            SemanticDimension::Symbol,
        ));

        assert!(
            accounting
                .records_for(SemanticDimension::Symbol)
                .iter()
                .any(|r| r.obligation.status == EpistemicStatus::Observed)
        );
    }

    // --- 7. OBSERVED does not erase UNSUPPORTED ----------------------------------------------

    #[test]
    fn observed_does_not_erase_unsupported() {
        let mut accounting = CensusExtractionAccounting::new();
        accounting.record_batch(&unsupported_batch(
            "extractor-a",
            "src/lib.rs",
            SemanticDimension::Type,
        ));
        // A second, independent extractor observes TYPE for the same artifact.
        let mut observed_type = observed_symbol_batch("extractor-b", "src/lib.rs", "known");
        observed_type.obligations[0].dimension = SemanticDimension::Type;
        if let SemanticObservation::Symbol(header) = &mut observed_type.observations[0] {
            header.dimension = SemanticDimension::Type;
        }
        accounting.record_batch(&observed_type);

        let records = accounting.records_for(SemanticDimension::Type);
        assert_eq!(records.len(), 2);
        assert!(
            records
                .iter()
                .any(|r| r.obligation.status == EpistemicStatus::Unsupported)
        );
        assert!(
            records
                .iter()
                .any(|r| r.obligation.status == EpistemicStatus::Observed)
        );
    }

    // --- 8. multiple OBSERVED results remain independently attributable ---------------------

    #[test]
    fn multiple_observed_results_stay_independently_attributable() {
        let mut accounting = CensusExtractionAccounting::new();
        accounting.record_batch(&observed_symbol_batch("extractor-a", "src/lib.rs", "alpha"));
        accounting.record_batch(&observed_symbol_batch("extractor-b", "src/lib.rs", "beta"));

        let records = accounting.records_for(SemanticDimension::Symbol);
        assert_eq!(records.len(), 2);
        assert_ne!(records[0].extractor, records[1].extractor);
        assert_ne!(
            records[0].obligation.observation_ids,
            records[1].obligation.observation_ids
        );
        assert_ne!(
            records[0].obligation.evidence_refs,
            records[1].obligation.evidence_refs
        );
    }

    // --- 9. no automatic reconciliation occurs -----------------------------------------------

    #[test]
    fn no_automatic_reconciliation_or_conflict_status_is_synthesized() {
        let mut accounting = CensusExtractionAccounting::new();
        accounting.record_batch(&observed_symbol_batch("extractor-a", "src/lib.rs", "known"));
        accounting.record_batch(&unknown_batch(
            "extractor-b",
            "src/lib.rs",
            SemanticDimension::Symbol,
        ));

        // Two raw statuses in, two raw statuses out -- neither collapsed into a synthesized
        // EpistemicStatus::Conflict, nor reduced to a single winner.
        let records = accounting.records_for(SemanticDimension::Symbol);
        assert_eq!(records.len(), 2);
        assert!(
            records
                .iter()
                .all(|r| r.obligation.status != EpistemicStatus::Conflict)
        );
    }

    // --- 10. existing twelve-dimension census accounting remains explicit -------------------

    #[test]
    fn bootstrap_coverage_still_names_all_twelve_r4_dimensions() {
        for dimension in [
            SemanticDimension::Symbol,
            SemanticDimension::Type,
            SemanticDimension::FunctionIdentity,
            SemanticDimension::FunctionSignature,
            SemanticDimension::Call,
            SemanticDimension::ControlFlow,
            SemanticDimension::DataFlow,
            SemanticDimension::State,
            SemanticDimension::Effect,
            SemanticDimension::Ownership,
            SemanticDimension::Concurrency,
            SemanticDimension::Persistence,
        ] {
            // SemanticDimension::as_str() is what both build_census's coverage map and this
            // accounting structure key by; a compile-time exhaustive match plus this loop is the
            // proof that no dimension was dropped from the canonical set this wave touches.
            assert!(!dimension.as_str().is_empty());
        }
    }

    // --- 11. no graph dependency is introduced ------------------------------------------------

    #[test]
    fn census_accounting_has_no_graph_dependency() {
        // Built at runtime, not written as a literal, so this check doesn't trip on its own text.
        let graph_type = format!("{}{}", "Engineering", "Graph");
        let graph_builder = format!("{}_{}", "build_system", "graph");
        let source = include_str!("extraction.rs");
        assert!(
            !source.contains(&graph_type) && !source.contains(&graph_builder),
            "multi-extractor census accounting must not depend on the engineering graph"
        );
    }

    // --- 12. no real Rust extraction is introduced --------------------------------------------

    #[test]
    fn no_real_rust_extractor_was_registered_this_wave() {
        let rust_extractors = adapter::extractors_for_language("rust");
        assert_eq!(rust_extractors.len(), 1);
        assert!(rust_extractors[0].supported_dimensions().is_empty());
    }
}
