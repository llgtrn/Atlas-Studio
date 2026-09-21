//! Runtime semantic normalization.
//!
//! Normalization removes representational variance without deciding truth. Equivalent propositions
//! converge on one semantic identity while each input row remains present so provenance is not lost.

use atlas_core::{
    CensusReport, EpistemicStatus, NormalizationConflict, NormalizationReport, SemanticFact,
    stable_id,
};
use std::collections::BTreeMap;

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

fn revision_scope(fact: &SemanticFact) -> String {
    fact.provenance
        .source_revision
        .as_ref()
        .map(|revision| format!("{}:{}", revision.kind.trim(), revision.value.trim()))
        .unwrap_or_else(|| "UNVERSIONED".into())
}

fn proposition_seed(fact: &SemanticFact, subject: &str, predicate: &str, object: &str) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        revision_scope(fact),
        fact.kind.as_str(),
        subject,
        predicate,
        object
    )
}

fn conflict_slot(fact: &SemanticFact) -> String {
    format!(
        "{}:{}:{}:{}",
        revision_scope(fact),
        fact.kind.as_str(),
        fact.subject,
        fact.predicate
    )
}

fn normalize_fact(fact: &SemanticFact) -> SemanticFact {
    let mut normalized = fact.clone();
    normalized.subject = normalized.subject.trim().to_owned();
    normalized.predicate = canonical_predicate(&normalized.predicate);
    normalized.object = normalized.object.trim().to_owned();
    normalized.id = stable_id(
        "normalized-fact",
        &proposition_seed(
            fact,
            &normalized.subject,
            &normalized.predicate,
            &normalized.object,
        ),
    );
    normalized
}

fn participates_in_positive_conflict(status: EpistemicStatus) -> bool {
    matches!(
        status,
        EpistemicStatus::Observed
            | EpistemicStatus::Declared
            | EpistemicStatus::Derived
            | EpistemicStatus::Inferred
    )
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
            .then_with(|| a.provenance.source_path.cmp(&b.provenance.source_path))
    });

    let mut kinds = BTreeMap::new();
    let mut equivalence_classes = BTreeMap::<String, usize>::new();
    let mut conflict_slots =
        BTreeMap::<String, BTreeMap<String, Vec<String>>>::new();

    for fact in &facts {
        *kinds.entry(fact.kind.as_str().to_owned()).or_insert(0) += 1;
        *equivalence_classes.entry(fact.id.clone()).or_insert(0) += 1;

        if participates_in_positive_conflict(fact.status) {
            conflict_slots
                .entry(conflict_slot(fact))
                .or_default()
                .entry(fact.object.clone())
                .or_default()
                .push(fact.id.clone());
        }
    }

    let equivalence_classes_total = equivalence_classes.len();
    let duplicate_observations_total = equivalence_classes
        .values()
        .map(|count| count.saturating_sub(1))
        .sum();

    let mut conflict_candidates = conflict_slots
        .into_iter()
        .filter_map(|(slot, objects)| {
            if objects.len() <= 1 {
                return None;
            }
            let mut fact_ids = objects
                .values()
                .flatten()
                .cloned()
                .collect::<Vec<_>>();
            fact_ids.sort();
            fact_ids.dedup();

            let objects = objects.into_keys().collect::<Vec<_>>();
            Some(NormalizationConflict {
                id: stable_id("normalization-conflict", &slot),
                slot,
                fact_ids,
                objects,
            })
        })
        .collect::<Vec<_>>();
    conflict_candidates.sort_by(|a, b| a.id.cmp(&b.id));

    NormalizationReport {
        schema: "atlas.normalization-report.v2".into(),
        input_facts_total: census.facts_total,
        normalized_facts_total: facts.len(),
        equivalence_classes_total,
        duplicate_observations_total,
        conflict_candidates_total: conflict_candidates.len(),
        conflict_candidates,
        kinds,
        facts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::{Provenance, RevisionRef, SemanticFactKind};

    fn fact(id: &str, object: &str, extractor: &str) -> SemanticFact {
        SemanticFact {
            id: id.into(),
            kind: SemanticFactKind::DeclaredEdge,
            status: EpistemicStatus::Declared,
            subject: " Core ".into(),
            predicate: "PROVIDES capability".into(),
            object: object.into(),
            provenance: Provenance {
                source_path: ".atlas/declared/system.adl".into(),
                source_revision: None,
                extractor: extractor.into(),
                content_hash: None,
                span: Some("3:1".into()),
            },
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
            facts: vec![fact("raw:1", " Compile ", "atlas.adl.compiler.v1")],
        };

        let report = normalize(&census);
        assert!(report.is_closed());
        assert_eq!(report.facts[0].subject, "Core");
        assert_eq!(report.facts[0].predicate, "provides_capability");
        assert_eq!(report.facts[0].object, "Compile");
        assert_eq!(
            report.facts[0].provenance.source_path,
            ".atlas/declared/system.adl"
        );
    }

    #[test]
    fn equivalent_raw_facts_converge_on_identity_without_dropping_rows() {
        let census = CensusReport {
            schema: "test".into(),
            artifacts_total: 0,
            artifacts_accounted_total: 0,
            facts_total: 2,
            coverage: BTreeMap::new(),
            facts: vec![
                fact("raw:one", "Compile", "extractor:one"),
                fact("raw:two", "Compile", "extractor:two"),
            ],
        };

        let report = normalize(&census);
        assert!(report.is_closed());
        assert_eq!(report.facts.len(), 2);
        assert_eq!(report.facts[0].id, report.facts[1].id);
        assert_eq!(report.equivalence_classes_total, 1);
        assert_eq!(report.duplicate_observations_total, 1);
    }

    #[test]
    fn identical_propositions_from_different_revisions_do_not_share_identity() {
        let mut first = fact("raw:one", "Compile", "extractor:one");
        first.provenance.source_revision = Some(RevisionRef {
            kind: "git".into(),
            value: "revision-a".into(),
        });
        let mut second = fact("raw:two", "Compile", "extractor:two");
        second.provenance.source_revision = Some(RevisionRef {
            kind: "git".into(),
            value: "revision-b".into(),
        });
        let census = CensusReport {
            schema: "test".into(),
            artifacts_total: 0,
            artifacts_accounted_total: 0,
            facts_total: 2,
            coverage: BTreeMap::new(),
            facts: vec![first, second],
        };

        let report = normalize(&census);
        assert!(report.is_closed());
        assert_ne!(report.facts[0].id, report.facts[1].id);
        assert_eq!(report.equivalence_classes_total, 2);
    }

    #[test]
    fn conflicting_positive_objects_are_surfaced_not_collapsed() {
        let census = CensusReport {
            schema: "test".into(),
            artifacts_total: 0,
            artifacts_accounted_total: 0,
            facts_total: 2,
            coverage: BTreeMap::new(),
            facts: vec![
                fact("raw:one", "CompileA", "extractor:one"),
                fact("raw:two", "CompileB", "extractor:two"),
            ],
        };

        let report = normalize(&census);
        assert!(report.is_closed());
        assert_eq!(report.conflict_candidates_total, 1);
        assert_eq!(report.conflict_candidates[0].objects.len(), 2);
        assert_eq!(report.facts.len(), 2);
    }
}
