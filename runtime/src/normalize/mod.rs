//! Runtime semantic normalization.
//!
//! Normalization gives observed/declared facts a deterministic vocabulary without upgrading their
//! epistemic status or dropping provenance. It is intentionally lossless at this stage.
//!
//! R4.3.2/R4.3.3: normalization is typed-semantic aware. `normalize_typed_records` is the real N0
//! normalization step for `CensusReport.typed_semantic_records` -- deterministic ordering, no
//! restructuring, no field mutation, and dedup ONLY on an exact `raw_observation_id` match (a
//! byte-identical raw observation), reported as `NormalizationReport::exact_duplicates_merged`
//! (`.atlas/contracts/NORMALIZATION.md#deduplication`). R4.12 adds `detect_conflict_candidates`:
//! records sharing a `record_id` (the same semantic claim identity) whose typed payload actually
//! disagrees are surfaced as `NormalizationReport::conflict_candidates` -- detected, never resolved
//! (`.atlas/contracts/NORMALIZATION.md#conflict-handling`); reconciliation (R6) owns resolution.
//! `normalize_typed_obligations` carries `CensusReport.typed_obligations` through the same N0
//! discipline. The pre-existing `normalize_fact`/`canonical_predicate` logic remains, unchanged,
//! for the `SemanticFact` compatibility projection that `facts` carries.

use atlas_core::{
    CensusReport, ConflictCandidate, Evidence, ExtractionDiagnostic, NormalizationReport,
    SemanticFact, SemanticObligationRecord, SemanticObservation, TypedClosureAccounting, stable_id,
};
use std::collections::BTreeMap;

/// N0 typed normalization: deterministic ordering, everything else preserved verbatim.
/// `normalize(record) == record` up to this ordering -- never a lossy record-to-string-to-record
/// round trip. De-duplication is by `raw_observation_id` (an EXACT raw-content match), never by
/// `record_id` alone: two independent extractors reporting the same semantic claim share a
/// `record_id` by design and must both survive
/// (`.atlas/contracts/SEMANTIC-EXTRACTION.md#multi-engine-extraction`).
///
/// Returns the deduplicated records plus how many raw observations the exact-identity collapse
/// removed (`NormalizationReport::exact_duplicates_merged`).
fn normalize_typed_records(records: &[SemanticObservation]) -> (Vec<SemanticObservation>, usize) {
    let input_len = records.len();
    let mut normalized = records.to_vec();
    normalized.sort_by(|a, b| {
        a.record_id()
            .as_str()
            .cmp(b.record_id().as_str())
            .then_with(|| {
                a.raw_observation_id()
                    .as_str()
                    .cmp(b.raw_observation_id().as_str())
            })
    });
    normalized.dedup_by(|a, b| a.raw_observation_id() == b.raw_observation_id());
    let exact_duplicates_merged = input_len - normalized.len();
    (normalized, exact_duplicates_merged)
}

/// Groups typed observations by `record_id` (the same semantic claim identity, guaranteed same
/// dimension since `SemanticRecordId` embeds it) and flags any group whose members disagree on
/// `subject_repr()` -- see `ConflictCandidate`'s doc comment for why this is deliberately
/// over-inclusive rather than attempting fine-grained per-dimension disagreement rules.
fn detect_conflict_candidates(records: &[SemanticObservation]) -> Vec<ConflictCandidate> {
    let mut by_record: BTreeMap<&str, Vec<&SemanticObservation>> = BTreeMap::new();
    for record in records {
        by_record
            .entry(record.record_id().as_str())
            .or_default()
            .push(record);
    }

    let mut candidates = Vec::new();
    for group in by_record.values() {
        let first_repr = group[0].subject_repr();
        if group.iter().any(|o| o.subject_repr() != first_repr) {
            candidates.push(ConflictCandidate {
                record_id: group[0].record_id().clone(),
                dimension: group[0].dimension(),
                raw_observation_ids: group.iter().map(|o| o.raw_observation_id()).collect(),
            });
        }
    }
    candidates
}

/// N0 typed normalization for the obligation ledger: deterministic ordering by `obligation_id`,
/// full-equality dedup (never merges two obligations that legitimately differ in content), no
/// reconciliation, no winner selection, no epistemic promotion, no evidence/diagnostic loss.
fn normalize_typed_obligations(
    obligations: &[SemanticObligationRecord],
) -> Vec<SemanticObligationRecord> {
    let mut normalized = obligations.to_vec();
    normalized.sort_by(|a, b| a.obligation_id.as_str().cmp(b.obligation_id.as_str()));
    normalized.dedup_by(|a, b| a == b);
    normalized
}

fn normalize_evidence(evidence: &[Evidence]) -> Vec<Evidence> {
    let mut normalized = evidence.to_vec();
    normalized.sort_by(|a, b| a.id.cmp(&b.id));
    normalized
}

fn normalize_diagnostics(diagnostics: &[ExtractionDiagnostic]) -> Vec<ExtractionDiagnostic> {
    let mut normalized = diagnostics.to_vec();
    normalized.sort_by(|a, b| a.id.cmp(&b.id));
    normalized
}

fn canonical_predicate(value: &str) -> String {
    let mut out = String::new();
    let mut previous_separator = false;
    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            previous_separator = false;
        } else if !previous_separator && !out.is_empty() {
            out.push('_');
            previous_separator = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    out
}

fn normalize_fact(fact: &SemanticFact) -> SemanticFact {
    let mut normalized = fact.clone();
    normalized.subject = normalized.subject.trim().to_owned();
    normalized.predicate = canonical_predicate(&normalized.predicate);
    normalized.object = normalized.object.trim().to_owned();
    normalized.id = stable_id(
        "normalized-fact",
        &format!(
            "{}:{}:{}:{}:{}",
            fact.id,
            fact.kind.as_str(),
            normalized.subject,
            normalized.predicate,
            normalized.object
        ),
    );
    normalized
}

pub fn normalize(census: &CensusReport) -> NormalizationReport {
    let mut facts = census.facts.iter().map(normalize_fact).collect::<Vec<_>>();
    facts.sort_by(|a, b| {
        a.kind
            .as_str()
            .cmp(b.kind.as_str())
            .then_with(|| a.subject.cmp(&b.subject))
            .then_with(|| a.predicate.cmp(&b.predicate))
            .then_with(|| a.object.cmp(&b.object))
            .then_with(|| a.id.cmp(&b.id))
    });

    let mut kinds = BTreeMap::new();
    for fact in &facts {
        *kinds.entry(fact.kind.as_str().to_owned()).or_insert(0) += 1;
    }

    let (typed_semantic_records, exact_duplicates_merged) =
        normalize_typed_records(&census.typed_semantic_records);
    let conflict_candidates = detect_conflict_candidates(&typed_semantic_records);
    let typed_obligations = normalize_typed_obligations(&census.typed_obligations);
    let normalized_typed_closure = TypedClosureAccounting {
        typed_observations_total: typed_semantic_records.len(),
        typed_obligations_total: typed_obligations.len(),
    };

    NormalizationReport {
        schema: "atlas.normalization-report.v4".into(),
        input_facts_total: census.facts_total,
        normalized_facts_total: facts.len(),
        kinds,
        typed_semantic_records,
        evidence: normalize_evidence(&census.evidence),
        diagnostics: normalize_diagnostics(&census.diagnostics),
        typed_obligations,
        input_typed_closure: census.typed_closure,
        normalized_typed_closure,
        exact_duplicates_merged,
        conflict_candidates,
        facts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::{
        EpistemicStatus, Evidence, ExtractorIdentity, Provenance, RepositoryId, RevisionRef,
        SemanticDimension, SemanticFact, SemanticFactKind, SemanticRecordHeader, SemanticRecordId,
        SemanticScope, SymbolIdentity, SymbolRole, provenance,
    };

    fn symbol_observation(
        name: &str,
        extractor_id: &str,
        evidence: Vec<atlas_core::EvidenceId>,
    ) -> SemanticObservation {
        let repository = RepositoryId::new("atlas-studio");
        let revision = RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let scope = SemanticScope::new(["core"]);
        let subject = SymbolIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            scope: scope.clone(),
            name: name.into(),
            role: SymbolRole::Definition,
        };
        let record_id = SemanticRecordId::new(SemanticDimension::Symbol, &subject.identity_key());
        SemanticObservation::Symbol(SemanticRecordHeader {
            record_id,
            dimension: SemanticDimension::Symbol,
            status: EpistemicStatus::Observed,
            subject,
            scope,
            repository,
            revision,
            extractor: ExtractorIdentity {
                id: extractor_id.into(),
                version: "0.1.0".into(),
            },
            evidence_refs: evidence,
            provenance: provenance("core/src/lib.rs", extractor_id),
        })
    }

    fn evidence_record(id: &str) -> Evidence {
        Evidence {
            id: id.into(),
            kind: "PARSER_OUTPUT".into(),
            path: "core/src/lib.rs".into(),
            summary: "test".into(),
            revision: None,
        }
    }

    /// Builds a `CensusReport` carrying `typed_semantic_records` plus a matching `Evidence` entry
    /// for every distinct `evidence_refs` id they cite, so `typed_semantics_closed()`'s reference
    /// check has something real to resolve against.
    fn census_with(typed_semantic_records: Vec<SemanticObservation>) -> CensusReport {
        let mut evidence_ids: std::collections::BTreeSet<String> =
            std::collections::BTreeSet::new();
        for record in &typed_semantic_records {
            for id in record.evidence_refs() {
                evidence_ids.insert(id.as_str().to_owned());
            }
        }
        let evidence = evidence_ids
            .into_iter()
            .map(|id| evidence_record(&id))
            .collect();

        CensusReport {
            schema: "test".into(),
            artifacts_total: 0,
            artifacts_accounted_total: 0,
            facts_total: 0,
            coverage: BTreeMap::new(),
            typed_closure: TypedClosureAccounting {
                typed_observations_total: typed_semantic_records.len(),
                typed_obligations_total: 0,
            },
            typed_semantic_records,
            evidence,
            diagnostics: Vec::new(),
            typed_obligations: Vec::new(),
            facts: Vec::new(),
        }
    }

    #[test]
    fn normalization_is_lossless_and_preserves_provenance() {
        let census = CensusReport {
            schema: "test".into(),
            artifacts_total: 0,
            artifacts_accounted_total: 0,
            facts_total: 1,
            coverage: BTreeMap::new(),
            typed_semantic_records: Vec::new(),
            evidence: Vec::new(),
            diagnostics: Vec::new(),
            typed_obligations: Vec::new(),
            typed_closure: TypedClosureAccounting {
                typed_observations_total: 0,
                typed_obligations_total: 0,
            },
            facts: vec![SemanticFact {
                id: "raw:1".into(),
                kind: SemanticFactKind::DeclaredEdge,
                status: EpistemicStatus::Declared,
                subject: " Core ".into(),
                predicate: "PROVIDES capability".into(),
                object: " Compile ".into(),
                provenance: Provenance {
                    source_path: ".atlas/declared/system.adl".into(),
                    source_revision: None,
                    extractor: "atlas.adl.compiler.v1".into(),
                    content_hash: None,
                    span: Some("3:1".into()),
                },
            }],
        };

        let report = normalize(&census);
        assert!(report.is_closed());
        assert_eq!(report.facts[0].subject, "Core");
        assert_eq!(report.facts[0].predicate, "provides_capability");
        assert_eq!(
            report.facts[0].provenance.source_path,
            ".atlas/declared/system.adl"
        );
    }

    // --- exact-duplicate collapse is reported, and closure survives it -----------------------

    #[test]
    fn byte_identical_raw_observations_collapse_and_are_counted_as_exact_duplicates() {
        // Two literally identical raw observations reaching `normalize()` directly (bypassing
        // `build_census`'s own pre-dedup, e.g. a caller merging raw observations from more than
        // one census) must still collapse to one record here, and the collapse must be an
        // explicit, countable fact -- not a silent count mismatch against `input_typed_closure`.
        let evidence = vec![atlas_core::EvidenceId::new("evidence:known")];
        let a = symbol_observation("known", "extractor-a", evidence.clone());
        let b = symbol_observation("known", "extractor-a", evidence);
        let census = census_with(vec![a, b]);

        let report = normalize(&census);

        assert_eq!(report.typed_semantic_records.len(), 1);
        assert_eq!(report.exact_duplicates_merged, 1);
        assert!(
            report.typed_semantics_closed(),
            "closure must account for the exact-duplicate collapse, not just raw counts"
        );
        assert!(report.conflict_candidates.is_empty());
    }

    #[test]
    fn two_extractors_reporting_the_same_symbol_are_not_exact_duplicates_or_a_conflict() {
        // Different extractor identity means these are independent corroborating observations
        // (`.atlas/contracts/SEMANTIC-EXTRACTION.md#multi-engine-extraction`), not a duplicate to
        // collapse and not a disagreement to flag: both survive, unmerged, uncounted as conflict.
        let a = symbol_observation(
            "known",
            "extractor-a",
            vec![atlas_core::EvidenceId::new("evidence:a")],
        );
        let b = symbol_observation(
            "known",
            "extractor-b",
            vec![atlas_core::EvidenceId::new("evidence:b")],
        );
        let census = census_with(vec![a, b]);

        let report = normalize(&census);

        assert_eq!(report.typed_semantic_records.len(), 2);
        assert_eq!(report.exact_duplicates_merged, 0);
        assert!(report.conflict_candidates.is_empty());
        assert!(report.typed_semantics_closed());
    }

    fn call_observation(
        dispatch: atlas_core::CallDispatchKind,
        callees: Vec<SemanticRecordId>,
        extractor_id: &str,
        evidence: Vec<atlas_core::EvidenceId>,
    ) -> SemanticObservation {
        let repository = RepositoryId::new("atlas-studio");
        let revision = RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        };
        let scope = SemanticScope::new(["core"]);
        let subject = atlas_core::CallSiteIdentity {
            repository: repository.clone(),
            revision: revision.clone(),
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "caller-fn-key"),
            span: atlas_core::SourceSpan {
                path: "core/src/lib.rs".into(),
                line: 10,
                column: 5,
            },
            dispatch,
            callees,
        };
        let record_id = SemanticRecordId::new(SemanticDimension::Call, &subject.identity_key());
        SemanticObservation::Call(SemanticRecordHeader {
            record_id,
            dimension: SemanticDimension::Call,
            status: EpistemicStatus::Inferred,
            subject,
            scope,
            repository,
            revision,
            extractor: ExtractorIdentity {
                id: extractor_id.into(),
                version: "0.1.0".into(),
            },
            evidence_refs: evidence,
            provenance: provenance("core/src/lib.rs", extractor_id),
        })
    }

    #[test]
    fn two_extractors_disagreeing_about_a_calls_resolved_targets_are_a_conflict_candidate() {
        // Same call site identity (same caller/span, per `CallSiteIdentity::identity_key()`, which
        // deliberately excludes `dispatch`/`callees`) but two extractors claiming DIFFERENT
        // resolved target sets for it -- exactly the "extractor call-target sets" example
        // `.atlas/contracts/NORMALIZATION.md#conflict-handling` names. Normalization must surface
        // this as a conflict candidate, not silently keep two unrelated-looking records.
        let a = call_observation(
            atlas_core::CallDispatchKind::StaticResolved,
            vec![SemanticRecordId::new(
                SemanticDimension::FunctionIdentity,
                "callee-foo",
            )],
            "extractor-a",
            vec![atlas_core::EvidenceId::new("evidence:a")],
        );
        let b = call_observation(
            atlas_core::CallDispatchKind::StaticResolved,
            vec![SemanticRecordId::new(
                SemanticDimension::FunctionIdentity,
                "callee-bar",
            )],
            "extractor-b",
            vec![atlas_core::EvidenceId::new("evidence:b")],
        );
        assert_eq!(
            a.record_id(),
            b.record_id(),
            "dispatch/callees must not affect the call site's identity"
        );

        let census = census_with(vec![a, b]);
        let report = normalize(&census);

        assert_eq!(report.typed_semantic_records.len(), 2);
        assert_eq!(report.exact_duplicates_merged, 0);
        assert_eq!(report.conflict_candidates.len(), 1);
        let candidate = &report.conflict_candidates[0];
        assert_eq!(candidate.dimension, SemanticDimension::Call);
        assert_eq!(candidate.raw_observation_ids.len(), 2);
        assert!(report.typed_semantics_closed());
    }

    #[test]
    fn two_extractors_agreeing_about_a_calls_resolved_target_are_not_a_conflict() {
        // Same identity, same resolved target -- genuine corroboration, not disagreement.
        let a = call_observation(
            atlas_core::CallDispatchKind::StaticResolved,
            vec![SemanticRecordId::new(
                SemanticDimension::FunctionIdentity,
                "callee-foo",
            )],
            "extractor-a",
            vec![atlas_core::EvidenceId::new("evidence:a")],
        );
        let b = call_observation(
            atlas_core::CallDispatchKind::StaticResolved,
            vec![SemanticRecordId::new(
                SemanticDimension::FunctionIdentity,
                "callee-foo",
            )],
            "extractor-b",
            vec![atlas_core::EvidenceId::new("evidence:b")],
        );

        let census = census_with(vec![a, b]);
        let report = normalize(&census);

        assert_eq!(report.typed_semantic_records.len(), 2);
        assert!(report.conflict_candidates.is_empty());
        assert!(report.typed_semantics_closed());
    }
}
