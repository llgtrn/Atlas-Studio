//! Runtime semantic normalization.
//!
//! Normalization gives observed/declared facts a deterministic vocabulary without upgrading their
//! epistemic status or dropping provenance. It is intentionally lossless at this stage.
//!
//! R4.3.2/R4.3.3: normalization is typed-semantic aware. `normalize_typed_records` is the real N0
//! normalization step for `CensusReport.typed_semantic_records` -- deterministic ordering only, no
//! restructuring, no field mutation, no dedup beyond EXACT raw-observation collapse (there are no
//! further normalization/reconciliation rules to apply yet; see
//! `.atlas/contracts/NORMALIZATION.md`). `normalize_typed_obligations` carries
//! `CensusReport.typed_obligations` through the same N0 discipline. The pre-existing
//! `normalize_fact`/`canonical_predicate` logic remains, unchanged, for the `SemanticFact`
//! compatibility projection that `facts` carries.

use atlas_core::{
    CensusReport, Evidence, ExtractionDiagnostic, NormalizationReport, SemanticFact,
    SemanticObligationRecord, SemanticObservation, TypedClosureAccounting, stable_id,
};
use std::collections::BTreeMap;

/// N0 typed normalization: deterministic ordering, everything else preserved verbatim.
/// `normalize(record) == record` up to this ordering -- never a lossy record-to-string-to-record
/// round trip. De-duplication is by `raw_observation_id` (an EXACT raw-content match), never by
/// `record_id` alone: two independent extractors reporting the same semantic claim share a
/// `record_id` by design and must both survive
/// (`.atlas/contracts/SEMANTIC-EXTRACTION.md#multi-engine-extraction`).
fn normalize_typed_records(records: &[SemanticObservation]) -> Vec<SemanticObservation> {
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
    normalized
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

    let typed_semantic_records = normalize_typed_records(&census.typed_semantic_records);
    let typed_obligations = normalize_typed_obligations(&census.typed_obligations);
    let normalized_typed_closure = TypedClosureAccounting {
        typed_observations_total: typed_semantic_records.len(),
        typed_obligations_total: typed_obligations.len(),
    };

    NormalizationReport {
        schema: "atlas.normalization-report.v3".into(),
        input_facts_total: census.facts_total,
        normalized_facts_total: facts.len(),
        kinds,
        typed_semantic_records,
        evidence: normalize_evidence(&census.evidence),
        diagnostics: normalize_diagnostics(&census.diagnostics),
        typed_obligations,
        input_typed_closure: census.typed_closure,
        normalized_typed_closure,
        facts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::{EpistemicStatus, Provenance, SemanticFact, SemanticFactKind};

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
}
