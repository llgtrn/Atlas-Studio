//! Runtime semantic normalization.
//!
//! Normalization gives observed/declared facts a deterministic vocabulary without upgrading their
//! epistemic status or dropping provenance. It is intentionally lossless at this stage.

use atlas_core::{
    CensusReport, NormalizationReport, SemanticFact, SemanticFactKind, stable_id,
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

    NormalizationReport {
        schema: "atlas.normalization-report.v1".into(),
        input_facts_total: census.facts_total,
        normalized_facts_total: facts.len(),
        kinds,
        facts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::{EpistemicStatus, Provenance, SemanticFact};

    #[test]
    fn normalization_is_lossless_and_preserves_provenance() {
        let census = CensusReport {
            schema: "test".into(),
            artifacts_total: 0,
            artifacts_accounted_total: 0,
            facts_total: 1,
            coverage: BTreeMap::new(),
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
