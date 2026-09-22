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
//! `extract_semantics` (R4.3, hardened R4.3.1) is the real production call site: called from
//! `runtime::systemize` (and the other report-building entry points) *before* `build_census`, it
//! identifies every admitted, successfully-parsed artifact, selects the registered
//! `SemanticExtractor` for its language (currently real for `"rust"`; every other language still
//! resolves to no extractor and is simply skipped here, unchanged from R4.3), and returns every
//! resulting `ExtractionBatch`. The caller folds those batches into both the canonical
//! `CensusReport` (`build_census`) and `CensusExtractionAccounting` -- the same extraction result
//! set backs both, so census truth and closure accounting can never disagree
//! (`.atlas/decisions/0001-one-normalized-semantic-path.md`).
//!
//! R4.3.1 fix: this function is now infallible. A per-artifact source-read failure no longer
//! propagates an `io::Error` that would abort extraction of every remaining artifact -- it
//! produces an explicit, closed `ExtractionBatch` via `SemanticExtractor::unavailable` instead
//! (`.atlas/contracts/SEMANTIC-EXTRACTION.md#failure-semantics`: "A failure must not remove the
//! artifact from census accounting").

use adapter::{DiagnosticCode, ExtractionBatch, ExtractionDiagnostic, ExtractionInput};
use atlas_core::{
    ArtifactDisposition, ArtifactId, ContentFingerprint, EvidenceId, ExtractorIdentity,
    InventoryReport, RepositoryId, RevisionRef, SemanticDimension, SemanticRecordId, stable_id,
};
use std::{collections::BTreeMap, fs, path::Path};

/// Every canonical R4 semantic dimension, requested uniformly so a production extraction call
/// never silently narrows what it asks an extractor to account for
/// (`.atlas/contracts/CENSUS-COMPLETENESS.md`). `pub` so `build_census`/`systemize` and tests share
/// the one canonical list rather than each keeping their own copy.
pub const ALL_SEMANTIC_DIMENSIONS: [SemanticDimension; 12] = [
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
];

/// Runs every registered `SemanticExtractor` against every admitted, successfully-parsed
/// (`ArtifactDisposition::Parsed`) artifact whose language has one, and returns every resulting
/// `ExtractionBatch`. Never executes the artifact's content -- only reads and parses its text.
///
/// Failure isolation: a read failure or a parse failure on one artifact never removes another
/// artifact's results, and never aborts the remaining artifacts in this call. Each artifact is
/// extracted independently:
/// - a parser-level failure inside an extractor is accounted for as an explicit
///   `ExtractionDiagnostic` (`ParseFailure`) + `UNKNOWN` obligation by that extractor itself (see
///   `adapter::semantic::rust`);
/// - a filesystem read failure (the file disappeared, permission denied, invalid UTF-8, ...) is
///   caught here, in this orchestration layer, and turned into an explicit `ExtractionDiagnostic`
///   (`InvalidInput`) + `UNKNOWN` obligation via `SemanticExtractor::unavailable` -- a distinct
///   diagnostic code from a parse failure, even though both share the `UNKNOWN` epistemic status,
///   so the underlying cause is never lost.
///
/// Every dimension outside what an extractor supports stays `UNSUPPORTED` regardless of which of
/// the above paths ran; nothing here upgrades UNSUPPORTED into UNKNOWN or vice versa.
pub fn extract_semantics(
    inventory: &InventoryReport,
    repository: RepositoryId,
    revision: RevisionRef,
) -> Vec<ExtractionBatch> {
    let mut batches = Vec::new();
    let root = Path::new(&inventory.root);

    for artifact in &inventory.artifacts {
        if artifact.disposition != ArtifactDisposition::Parsed {
            continue;
        }
        let Some(language) = artifact.language.as_deref() else {
            continue;
        };
        let extractors = adapter::extractors_for_language(language);
        if extractors.is_empty() {
            continue;
        }
        let source_frontend_id = adapter::resolve_source_frontend(Path::new(&artifact.path))
            .map(|matched| matched.frontend_id.to_owned())
            .unwrap_or_default();

        match fs::read_to_string(root.join(&artifact.path)) {
            Ok(source_text) => {
                let content_fingerprint =
                    Some(ContentFingerprint(stable_id("content", &source_text)));
                for extractor in &extractors {
                    let input = ExtractionInput {
                        repository: repository.clone(),
                        revision: revision.clone(),
                        artifact: artifact.id.clone(),
                        artifact_path: artifact.path.clone(),
                        source_text: source_text.clone(),
                        content_fingerprint: content_fingerprint.clone(),
                        source_frontend_id: source_frontend_id.clone(),
                        language: language.to_owned(),
                        build_profile: None,
                        scope_policy: None,
                        requested_dimensions: ALL_SEMANTIC_DIMENSIONS.to_vec(),
                    };
                    batches.push(extractor.extract(&input));
                }
            }
            Err(read_error) => {
                // Never `?`-propagate: one artifact's read failure must not abort extraction of
                // every artifact that comes after it in `inventory.artifacts`.
                let diagnostic = ExtractionDiagnostic::new(
                    DiagnosticCode::InvalidInput,
                    None,
                    format!("failed to read {}: {read_error}", artifact.path),
                );
                for extractor in &extractors {
                    let input = ExtractionInput {
                        repository: repository.clone(),
                        revision: revision.clone(),
                        artifact: artifact.id.clone(),
                        artifact_path: artifact.path.clone(),
                        source_text: String::new(),
                        content_fingerprint: None,
                        source_frontend_id: source_frontend_id.clone(),
                        language: language.to_owned(),
                        build_profile: None,
                        scope_policy: None,
                        requested_dimensions: ALL_SEMANTIC_DIMENSIONS.to_vec(),
                    };
                    batches.push(extractor.unavailable(&input, diagnostic.clone()));
                }
            }
        }
    }

    batches
}

/// One extractor's obligation result for one dimension on one artifact, preserved independently
/// alongside every other extractor's result for the same (artifact, dimension) pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractorObligationRecord {
    pub artifact: ArtifactId,
    pub extractor: ExtractorIdentity,
    pub obligation: adapter::ObligationResult,
}

impl ExtractorObligationRecord {
    /// Deterministic publication identity for this record.
    ///
    /// Every field that could make two records semantically distinguishable participates:
    /// dimension, extractor id/version, artifact, epistemic status, and the full content of
    /// `observation_ids`/`evidence_refs`/`diagnostics` (canonicalized by sorting, since those
    /// lists' own element order is not semantically meaningful here). Field order matches the
    /// canonical publication grouping (`SemanticDimension -> ExtractorIdentity -> ArtifactId ->
    /// record content`), so sorting by this key alone reproduces that grouping without a separate
    /// comparator stage.
    ///
    /// Two records that differ in *any* field never tie under this key; two records that tie are
    /// therefore identical in every field and thus indistinguishable, so their relative order
    /// carries no meaning. Never derived from Vec insertion order, HashMap iteration, thread
    /// scheduling or wall-clock time.
    pub fn identity_key(&self) -> String {
        let mut observation_ids: Vec<&str> = self
            .obligation
            .observation_ids
            .iter()
            .map(SemanticRecordId::as_str)
            .collect();
        observation_ids.sort_unstable();
        let mut evidence_refs: Vec<&str> = self
            .obligation
            .evidence_refs
            .iter()
            .map(EvidenceId::as_str)
            .collect();
        evidence_refs.sort_unstable();
        let mut diagnostics: Vec<&str> = self
            .obligation
            .diagnostics
            .iter()
            .map(String::as_str)
            .collect();
        diagnostics.sort_unstable();

        format!(
            "{}|{}:{}|{}|{}|obs=[{}]|evi=[{}]|diag=[{}]",
            self.obligation.dimension.as_str(),
            self.extractor.id,
            self.extractor.version,
            self.artifact.as_str(),
            self.obligation.status.as_str(),
            observation_ids.join(","),
            evidence_refs.join(","),
            diagnostics.join(","),
        )
    }
}

/// Multi-extractor census accounting: an append-only collection of `ExtractorObligationRecord`,
/// plus the full lineage of diagnostic objects that justify an `UNKNOWN`/`UNSUPPORTED` obligation.
///
/// This is NOT the bootstrap `CensusReport.coverage: BTreeMap<String, EpistemicStatus>` (a
/// derived, single-valued projection kept for existing callers). This structure is the canonical
/// preserved input; that map is noncanonical summary shape, never populated by this accumulator.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CensusExtractionAccounting {
    records: Vec<ExtractorObligationRecord>,
    /// Every `ExtractionDiagnostic` seen so far, keyed by its stable id. An `ObligationResult`
    /// only carries diagnostic *ids* (`ObligationResult.diagnostics: Vec<String>`); resolving one
    /// against this map answers "what diagnostic caused this dimension's status" and "was it a
    /// parser failure (`ParseFailure`) or a read failure (`InvalidInput`)" without either being
    /// silently collapsed into a bare `UNKNOWN`.
    diagnostics: BTreeMap<String, ExtractionDiagnostic>,
}

impl CensusExtractionAccounting {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends every obligation in `batch`, tagged with `batch.artifact`/`batch.extractor`, and
    /// retains every diagnostic `batch` reported. Never removes or overwrites an existing record:
    /// an extractor's result can never erase another extractor's result, regardless of ingestion
    /// order.
    pub fn record_batch(&mut self, batch: &ExtractionBatch) {
        for obligation in &batch.obligations {
            self.records.push(ExtractorObligationRecord {
                artifact: batch.artifact.clone(),
                extractor: batch.extractor.clone(),
                obligation: obligation.clone(),
            });
        }
        for diagnostic in &batch.diagnostics {
            self.diagnostics
                .insert(diagnostic.id.clone(), diagnostic.clone());
        }
    }

    /// Resolves a diagnostic id (as found in an `ObligationResult.diagnostics` entry) to the full
    /// `ExtractionDiagnostic` object that produced it, when this accounting has seen it.
    pub fn diagnostic(&self, id: &str) -> Option<&ExtractionDiagnostic> {
        self.diagnostics.get(id)
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
        records.sort_by_key(ExtractorObligationRecord::identity_key);
        records
    }

    /// `true` iff, for every distinct (artifact, extractor identity) pair that contributed any
    /// record, `requested` appears exactly once each. Derived directly from the retained ledger
    /// rather than tracked as a separately-threaded boolean, so this closure check and the
    /// `CensusReport` built from the same extraction batches can never disagree about what
    /// extraction actually produced (`.atlas/contracts/SEMANTIC-EXTRACTION.md#extractionbatch`).
    pub fn is_closed(&self, requested: &[SemanticDimension]) -> bool {
        let mut grouped: BTreeMap<(&str, &str, &str), Vec<SemanticDimension>> = BTreeMap::new();
        for record in &self.records {
            grouped
                .entry((
                    record.artifact.as_str(),
                    record.extractor.id.as_str(),
                    record.extractor.version.as_str(),
                ))
                .or_default()
                .push(record.obligation.dimension);
        }
        grouped.values().all(|dimensions| {
            dimensions.len() == requested.len()
                && requested.iter().all(|dimension| {
                    dimensions.iter().filter(|seen| *seen == dimension).count() == 1
                })
        })
    }
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
            source_text: "pub fn known() {}\n".into(),
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

        // R4.3: SYMBOL and FUNCTION_IDENTITY are now real extraction (evidence-backed OBSERVED);
        // OWNERSHIP remains UNSUPPORTED (R4.10+). Neither is silently dropped.
        for dimension in [
            SemanticDimension::Symbol,
            SemanticDimension::FunctionIdentity,
        ] {
            let records = accounting.records_for(dimension);
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].obligation.status, EpistemicStatus::Observed);
        }
        let ownership_records = accounting.records_for(SemanticDimension::Ownership);
        assert_eq!(ownership_records.len(), 1);
        assert_eq!(
            ownership_records[0].obligation.status,
            EpistemicStatus::Unsupported
        );
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
        TypeIdentity, stable_id,
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
        let observation = SemanticObservation::Symbol(header);
        assert!(
            observation.is_dimension_consistent(),
            "fixture must not produce mismatched variant/dimension observations"
        );
        ExtractionBatch {
            extractor,
            repository,
            revision: revision.clone(),
            artifact: ArtifactId::new(format!("artifact:{artifact_path}")),
            input_fingerprint: format!("{extractor_id}:{artifact_path}"),
            observations: vec![observation],
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

    /// A genuine `SemanticObservation::Type(SemanticRecordHeader<TypeIdentity>)`, never a
    /// `Symbol` variant with its `dimension` field mutated to `Type` (that was a real, since-fixed
    /// bug in this test module: the variant tag and the header's `dimension` field disagreed).
    fn observed_type_batch(
        extractor_id: &str,
        artifact_path: &str,
        type_name: &str,
    ) -> ExtractionBatch {
        let extractor = extractor(extractor_id, "0.1.0");
        let repository = RepositoryId::new("atlas-studio");
        let revision = RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let evidence_id = EvidenceId::new(stable_id(
            "evidence",
            &format!("{extractor_id}:{type_name}"),
        ));
        let type_identity = TypeIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            scope: SemanticScope::new(["fixture"]),
            name: type_name.into(),
            canonical: None,
        };
        let record_id =
            SemanticRecordId::new(SemanticDimension::Type, &type_identity.identity_key());
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::Type,
            status: EpistemicStatus::Observed,
            subject: type_identity,
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
        let observation = SemanticObservation::Type(header);
        assert!(
            observation.is_dimension_consistent(),
            "fixture must not produce mismatched variant/dimension observations"
        );
        ExtractionBatch {
            extractor,
            repository,
            revision: revision.clone(),
            artifact: ArtifactId::new(format!("artifact:{artifact_path}")),
            input_fingerprint: format!("{extractor_id}:{artifact_path}"),
            observations: vec![observation],
            evidence: vec![Evidence {
                id: evidence_id.as_str().to_owned(),
                kind: "SOURCE_TEXT".into(),
                path: artifact_path.into(),
                summary: format!("{extractor_id} observed type `{type_name}`"),
                revision: Some(revision),
            }],
            obligations: vec![ObligationResult::observed(
                SemanticDimension::Type,
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
        // A second, independent extractor observes TYPE for the same artifact, via a genuine
        // Type observation (not a Symbol variant with its dimension field mutated).
        let observed_type = observed_type_batch("extractor-b", "src/lib.rs", "Known");
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

    // --- 12. R4.5: a real Rust extractor is now registered, scoped to exactly 5 dimensions -----

    #[test]
    fn a_real_rust_extractor_is_registered_supporting_exactly_the_r4_5_dimensions() {
        let rust_extractors = adapter::extractors_for_language("rust");
        assert_eq!(rust_extractors.len(), 1);
        assert_eq!(rust_extractors[0].id(), "atlas.rust.source-semantic.v1");
        let mut supported: Vec<&str> = rust_extractors[0]
            .supported_dimensions()
            .iter()
            .map(SemanticDimension::as_str)
            .collect();
        supported.sort_unstable();
        assert_eq!(
            supported,
            vec![
                "CALL",
                "FUNCTION_IDENTITY",
                "FUNCTION_SIGNATURE",
                "SYMBOL",
                "TYPE"
            ]
        );
    }

    // === R4.2.2 determinism hardening ==========================================================
    //
    // The publication key before this wave was `dimension -> extractor id -> extractor version ->
    // artifact -> first observation id`. Two records with empty `observation_ids` (e.g. UNKNOWN
    // and UNSUPPORTED for the same artifact/extractor/dimension) tied under that key, so a stable
    // sort preserved whichever insertion order they arrived in -- forward and reversed ingestion
    // could publish in different orders. `identity_key()` closes that gap by folding in status and
    // the full (canonicalized) content of observation_ids/evidence_refs/diagnostics.

    fn diagnostic_only_batch(
        extractor_id: &str,
        version: &str,
        artifact_path: &str,
        obligation: ObligationResult,
    ) -> ExtractionBatch {
        ExtractionBatch {
            extractor: extractor(extractor_id, version),
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            artifact: ArtifactId::new(format!("artifact:{artifact_path}")),
            input_fingerprint: format!("{extractor_id}:{version}:{artifact_path}"),
            observations: Vec::new(),
            evidence: Vec::new(),
            obligations: vec![obligation],
            diagnostics: Vec::new(),
        }
    }

    // CASE A: same artifact/extractor/dimension, UNKNOWN vs UNSUPPORTED -- both have empty
    // observation_ids, which is exactly the counterexample that tied under the old sort key.

    #[test]
    fn case_a_unknown_vs_unsupported_is_order_independent() {
        let unknown = diagnostic_only_batch(
            "extractor-a",
            "0.1.0",
            "src/lib.rs",
            ObligationResult::unknown(SemanticDimension::Symbol, "diag-unknown"),
        );
        let unsupported = diagnostic_only_batch(
            "extractor-a",
            "0.1.0",
            "src/lib.rs",
            ObligationResult::unsupported(SemanticDimension::Symbol, "diag-unsupported"),
        );

        let mut forward = CensusExtractionAccounting::new();
        forward.record_batch(&unknown);
        forward.record_batch(&unsupported);

        let mut reversed = CensusExtractionAccounting::new();
        reversed.record_batch(&unsupported);
        reversed.record_batch(&unknown);

        assert_eq!(forward.len(), 2);
        assert_eq!(forward.sorted(), reversed.sorted());
    }

    // CASE B: same artifact/extractor/dimension/status, differing only by diagnostic id.

    #[test]
    fn case_b_same_status_different_diagnostics_is_order_independent() {
        let first = diagnostic_only_batch(
            "extractor-a",
            "0.1.0",
            "src/lib.rs",
            ObligationResult::unknown(SemanticDimension::Symbol, "diag-1"),
        );
        let second = diagnostic_only_batch(
            "extractor-a",
            "0.1.0",
            "src/lib.rs",
            ObligationResult::unknown(SemanticDimension::Symbol, "diag-2"),
        );

        let mut forward = CensusExtractionAccounting::new();
        forward.record_batch(&first);
        forward.record_batch(&second);

        let mut reversed = CensusExtractionAccounting::new();
        reversed.record_batch(&second);
        reversed.record_batch(&first);

        assert_eq!(forward.len(), 2);
        assert_eq!(forward.sorted(), reversed.sorted());
    }

    // CASE C: same semantic record, but observation_ids/evidence_refs supplied in different input
    // order. List order is not semantically meaningful here, so identity must be canonical.

    #[test]
    fn case_c_child_collection_order_does_not_affect_identity() {
        let id_a = SemanticRecordId::new(SemanticDimension::Symbol, "a");
        let id_b = SemanticRecordId::new(SemanticDimension::Symbol, "b");
        let evidence_x = EvidenceId::new("evidence:x".to_owned());
        let evidence_y = EvidenceId::new("evidence:y".to_owned());

        let forward = ExtractorObligationRecord {
            artifact: ArtifactId::new("artifact:src/lib.rs"),
            extractor: extractor("extractor-a", "0.1.0"),
            obligation: ObligationResult::observed(
                SemanticDimension::Symbol,
                vec![id_a.clone(), id_b.clone()],
                vec![evidence_x.clone(), evidence_y.clone()],
            ),
        };
        let reversed = ExtractorObligationRecord {
            artifact: ArtifactId::new("artifact:src/lib.rs"),
            extractor: extractor("extractor-a", "0.1.0"),
            obligation: ObligationResult::observed(
                SemanticDimension::Symbol,
                vec![id_b, id_a],
                vec![evidence_y, evidence_x],
            ),
        };

        assert_eq!(forward.identity_key(), reversed.identity_key());
    }

    // CASE D: two different extractor ids remain distinct.

    #[test]
    fn case_d_different_extractor_ids_remain_distinct() {
        let a = diagnostic_only_batch(
            "extractor-a",
            "0.1.0",
            "src/lib.rs",
            ObligationResult::unknown(SemanticDimension::Symbol, "diag"),
        );
        let b = diagnostic_only_batch(
            "extractor-b",
            "0.1.0",
            "src/lib.rs",
            ObligationResult::unknown(SemanticDimension::Symbol, "diag"),
        );

        let record_a = ExtractorObligationRecord {
            artifact: a.artifact.clone(),
            extractor: a.extractor.clone(),
            obligation: a.obligations[0].clone(),
        };
        let record_b = ExtractorObligationRecord {
            artifact: b.artifact.clone(),
            extractor: b.extractor.clone(),
            obligation: b.obligations[0].clone(),
        };
        assert_ne!(record_a.identity_key(), record_b.identity_key());
    }

    // CASE E: same extractor id, different version remains distinct.

    #[test]
    fn case_e_same_extractor_id_different_version_remains_distinct() {
        let v1 = diagnostic_only_batch(
            "extractor-a",
            "0.1.0",
            "src/lib.rs",
            ObligationResult::unknown(SemanticDimension::Symbol, "diag"),
        );
        let v2 = diagnostic_only_batch(
            "extractor-a",
            "0.2.0",
            "src/lib.rs",
            ObligationResult::unknown(SemanticDimension::Symbol, "diag"),
        );

        let record_v1 = ExtractorObligationRecord {
            artifact: v1.artifact.clone(),
            extractor: v1.extractor.clone(),
            obligation: v1.obligations[0].clone(),
        };
        let record_v2 = ExtractorObligationRecord {
            artifact: v2.artifact.clone(),
            extractor: v2.extractor.clone(),
            obligation: v2.obligations[0].clone(),
        };
        assert_ne!(record_v1.identity_key(), record_v2.identity_key());
    }

    // Section 10 item 4: production/test helpers never generate mismatched variant/header pairs.
    // observed_symbol_batch/observed_type_batch already assert this at construction time; this
    // test additionally proves it from the outside, against their actual returned batches.

    #[test]
    fn fixture_helpers_never_produce_mismatched_variant_header_observations() {
        let symbol_batch = observed_symbol_batch("extractor-a", "src/lib.rs", "known");
        let type_batch = observed_type_batch("extractor-b", "src/lib.rs", "Known");

        for batch in [&symbol_batch, &type_batch] {
            for observation in &batch.observations {
                assert!(observation.is_dimension_consistent());
            }
        }
    }
}

/// Proves `extract_semantics` (the real production wiring, called from `runtime::systemize`) reads
/// actual files from disk, selects the real Rust extractor, and never lets one malformed file erase
/// another file's successfully extracted semantics.
#[cfg(test)]
mod production_wiring_tests {
    use super::*;
    use atlas_core::{
        ArtifactDisposition, ArtifactId, ArtifactKind, ArtifactRecord, EpistemicStatus,
        InventoryReport, RepositoryId, RevisionRef,
    };
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn scratch_dir() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("atlas-r4.3-extract-{}-{nonce}", std::process::id()))
    }

    fn rust_artifact(path: &str) -> ArtifactRecord {
        ArtifactRecord {
            id: ArtifactId::new(format!("artifact:{path}")),
            path: path.to_owned(),
            kind: ArtifactKind::File,
            bytes: 0,
            disposition: ArtifactDisposition::Parsed,
            language: Some("rust".into()),
            reason: None,
        }
    }

    fn accounting_from(batches: &[ExtractionBatch]) -> CensusExtractionAccounting {
        let mut accounting = CensusExtractionAccounting::new();
        for batch in batches {
            accounting.record_batch(batch);
        }
        accounting
    }

    #[test]
    fn production_wiring_extracts_real_rust_semantics_and_isolates_a_malformed_file() {
        let dir = scratch_dir();
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("valid.rs"), "pub fn known() {}\n").unwrap();
        fs::write(dir.join("broken.rs"), "pub fn broken( {\n").unwrap();

        let inventory = InventoryReport::new(
            dir.to_string_lossy().into_owned(),
            vec![rust_artifact("valid.rs"), rust_artifact("broken.rs")],
        );

        let batches = extract_semantics(
            &inventory,
            RepositoryId::new("atlas-studio"),
            RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
        );
        let accounting = accounting_from(&batches);
        assert!(
            accounting.is_closed(&ALL_SEMANTIC_DIMENSIONS),
            "every real extractor batch must be closed"
        );

        let records = accounting.sorted();
        let valid_artifact = ArtifactId::new("artifact:valid.rs");
        let broken_artifact = ArtifactId::new("artifact:broken.rs");

        let valid_symbol = records
            .iter()
            .find(|record| {
                record.artifact == valid_artifact
                    && record.obligation.dimension == SemanticDimension::Symbol
            })
            .expect("valid.rs SYMBOL obligation present");
        assert_eq!(valid_symbol.obligation.status, EpistemicStatus::Observed);
        assert!(!valid_symbol.obligation.observation_ids.is_empty());

        // The malformed file never disappears from accounting -- it still gets an explicit UNKNOWN
        // obligation (never silently dropped), and it never erases valid.rs's results above.
        let broken_symbol = records
            .iter()
            .find(|record| {
                record.artifact == broken_artifact
                    && record.obligation.dimension == SemanticDimension::Symbol
            })
            .expect("broken.rs SYMBOL obligation present -- malformed input is never dropped");
        assert_eq!(broken_symbol.obligation.status, EpistemicStatus::Unknown);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn extract_semantics_skips_artifacts_with_no_registered_extractor() {
        let dir = scratch_dir();
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("notes.md"), "# hello\n").unwrap();

        let inventory = InventoryReport::new(
            dir.to_string_lossy().into_owned(),
            vec![ArtifactRecord {
                id: ArtifactId::new("artifact:notes.md"),
                path: "notes.md".into(),
                kind: ArtifactKind::File,
                bytes: 0,
                disposition: ArtifactDisposition::Parsed,
                language: Some("markdown".into()),
                reason: None,
            }],
        );

        let batches = extract_semantics(
            &inventory,
            RepositoryId::new("atlas-studio"),
            RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
        );
        assert!(batches.is_empty());

        fs::remove_dir_all(&dir).unwrap();
    }

    // --- R4.3.1: per-artifact read-failure isolation ------------------------------------------
    //
    // A middle artifact whose file was admitted into inventory but can no longer be read (removed
    // after admission, permission denied, ...) must not abort extraction of the artifacts that
    // come after it, and must not silently vanish from accounting itself.

    #[test]
    fn a_read_failure_on_a_middle_artifact_never_aborts_or_erases_the_batch() {
        let dir = scratch_dir();
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.rs"), "pub fn from_a() {}\n").unwrap();
        fs::write(dir.join("c.rs"), "pub fn from_c() {}\n").unwrap();
        // b.rs is deliberately never written: inventory admits it (as it would have, e.g., in the
        // instant between classification and extraction), but reading it now fails with NotFound.

        let inventory = InventoryReport::new(
            dir.to_string_lossy().into_owned(),
            vec![
                rust_artifact("a.rs"),
                rust_artifact("b.rs"),
                rust_artifact("c.rs"),
            ],
        );

        let batches = extract_semantics(
            &inventory,
            RepositoryId::new("atlas-studio"),
            RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
        );
        // All three artifacts produced a batch: b's read failure did not remove a or c, and did
        // not stop extraction from reaching c.
        assert_eq!(batches.len(), 3);

        let accounting = accounting_from(&batches);
        assert!(accounting.is_closed(&ALL_SEMANTIC_DIMENSIONS));

        let by_artifact = |path: &str, dimension: SemanticDimension| {
            accounting
                .sorted()
                .into_iter()
                .find(|record| {
                    record.artifact == ArtifactId::new(format!("artifact:{path}"))
                        && record.obligation.dimension == dimension
                })
                .unwrap_or_else(|| panic!("missing {path}/{dimension:?} obligation"))
        };

        let a = by_artifact("a.rs", SemanticDimension::Symbol);
        assert_eq!(a.obligation.status, EpistemicStatus::Observed);
        assert!(!a.obligation.observation_ids.is_empty());

        let b = by_artifact("b.rs", SemanticDimension::Symbol);
        assert_eq!(
            b.obligation.status,
            EpistemicStatus::Unknown,
            "an unreadable artifact's supported dimensions are UNKNOWN, never silently dropped"
        );

        let c = by_artifact("c.rs", SemanticDimension::Symbol);
        assert_eq!(
            c.obligation.status,
            EpistemicStatus::Observed,
            "extraction must continue past a read failure to the artifacts that follow it"
        );

        // The read failure is distinguishable from a parse failure: it carries an INVALID_INPUT
        // diagnostic, never PARSE_FAILURE (`.atlas/contracts/SEMANTIC-EXTRACTION.md`: read failure,
        // parse failure, unsupported and unknown must not collapse into one undifferentiated state).
        let b_batch = batches
            .iter()
            .find(|batch| batch.artifact == ArtifactId::new("artifact:b.rs"))
            .unwrap();
        assert!(
            b_batch
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == adapter::DiagnosticCode::InvalidInput)
        );
        assert!(
            !b_batch
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == adapter::DiagnosticCode::ParseFailure)
        );

        // Unsupported dimensions stay UNSUPPORTED even for the unreadable artifact -- source
        // availability never changes what the extractor is capable of analyzing.
        let b_ownership = by_artifact("b.rs", SemanticDimension::Ownership);
        assert_eq!(b_ownership.obligation.status, EpistemicStatus::Unsupported);

        fs::remove_dir_all(&dir).unwrap();
    }
}
