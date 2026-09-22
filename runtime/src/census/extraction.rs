//! Bridges typed `ExtractionBatch` obligation accounting (`adapter`) into the bootstrap census
//! coverage shape, proving census can consume `SemanticExtractor` output without creating a second
//! semantic path (`.atlas/decisions/0001-one-normalized-semantic-path.md`).
//!
//! Full per-artifact wiring into `census::build_census`'s production path is intentionally
//! deferred: every extractor registered today is a `StaticUnsupportedExtractor`, so wiring it into
//! the main call graph right now would only reproduce the coverage fix already applied directly
//! in `build_census`. This is the real, tested integration point a real R4.3+ extractor attaches
//! to; it is not itself the production call site yet.

use adapter::ExtractionBatch;
use atlas_core::EpistemicStatus;
use std::collections::BTreeMap;

/// Folds every `ObligationResult` in `batch` into `coverage`, keyed by
/// `SemanticDimension::as_str()`. A dimension already present in `coverage` is superseded by the
/// batch's result for that dimension; dimensions the batch does not address are left untouched.
pub fn merge_batch_into_coverage(
    coverage: &mut BTreeMap<String, EpistemicStatus>,
    batch: &ExtractionBatch,
) {
    for obligation in &batch.obligations {
        coverage.insert(obligation.dimension.as_str().to_owned(), obligation.status);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapter::{ExtractionInput, extractors_for_language};
    use atlas_core::{
        ArtifactId, ContentFingerprint, RepositoryId, RevisionRef, SemanticDimension,
    };

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
    fn extractor_output_merges_into_census_coverage_without_a_second_semantic_path() {
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

        let mut coverage = BTreeMap::from([("SYMBOL".to_owned(), EpistemicStatus::Observed)]);
        merge_batch_into_coverage(&mut coverage, &batch);

        for dimension in &requested {
            assert_eq!(
                coverage.get(dimension.as_str()),
                Some(&EpistemicStatus::Unsupported)
            );
        }
    }
}
