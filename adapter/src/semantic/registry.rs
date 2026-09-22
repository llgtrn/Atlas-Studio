//! Native extractor registry.
//!
//! Explicit typed registration only — this is not a plugin marketplace or a service locator.
//! Multiple extractors may eventually support the same language/dimension; the registry never
//! lets one overwrite another; each result stays attributable to its own `ExtractorIdentity` for
//! later reconciliation (`.atlas/contracts/NORMALIZATION.md#conflict-handling`).

use atlas_core::SemanticDimension;

use super::batch::{ExtractionBatch, ObligationResult};
use super::extractor::{DiagnosticCode, ExtractionDiagnostic, ExtractionInput, SemanticExtractor};

/// Placeholder extractor for a language with no deep semantic analyzer implemented yet. Declares
/// zero supported dimensions and answers every requested dimension with an explicit `UNSUPPORTED`
/// obligation plus diagnostic — this is what keeps FUNCTION_IDENTITY, FUNCTION_SIGNATURE,
/// OWNERSHIP, CONCURRENCY and PERSISTENCE from silently disappearing from accounting before a
/// real extractor (R4.3+) exists for `language`.
#[derive(Debug)]
pub struct StaticUnsupportedExtractor {
    id: &'static str,
    version: &'static str,
    languages: &'static [&'static str],
}

impl StaticUnsupportedExtractor {
    pub const fn new(
        id: &'static str,
        version: &'static str,
        languages: &'static [&'static str],
    ) -> Self {
        Self {
            id,
            version,
            languages,
        }
    }
}

impl SemanticExtractor for StaticUnsupportedExtractor {
    fn id(&self) -> &'static str {
        self.id
    }

    fn version(&self) -> &'static str {
        self.version
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        self.languages
    }

    fn supported_dimensions(&self) -> &'static [SemanticDimension] {
        &[]
    }

    fn extract(&self, input: &ExtractionInput) -> ExtractionBatch {
        let extractor = self.identity();
        let input_fingerprint = input.identity_key(&extractor);
        let mut diagnostics = Vec::new();
        let mut obligations = Vec::new();

        for &dimension in &input.requested_dimensions {
            let diagnostic = ExtractionDiagnostic::new(
                DiagnosticCode::UnsupportedSemanticDimension,
                Some(dimension),
                format!(
                    "no real semantic extractor implemented for {} in {} yet",
                    dimension.as_str(),
                    input.language
                ),
            );
            obligations.push(ObligationResult::unsupported(
                dimension,
                diagnostic.id.clone(),
            ));
            diagnostics.push(diagnostic);
        }

        ExtractionBatch {
            extractor,
            repository: input.repository.clone(),
            revision: input.revision.clone(),
            artifact: input.artifact.clone(),
            input_fingerprint,
            observations: Vec::new(),
            evidence: Vec::new(),
            obligations,
            diagnostics,
        }
    }
}

static RUST_STATIC_UNSUPPORTED: StaticUnsupportedExtractor =
    StaticUnsupportedExtractor::new("atlas.rust.static-unsupported.v1", "0.1.0", &["rust"]);

static BUILTIN_EXTRACTORS: [&'static dyn SemanticExtractor; 1] = [&RUST_STATIC_UNSUPPORTED];

pub fn semantic_extractors() -> &'static [&'static dyn SemanticExtractor] {
    &BUILTIN_EXTRACTORS
}

pub fn extractors_for_language(language: &str) -> Vec<&'static dyn SemanticExtractor> {
    semantic_extractors()
        .iter()
        .copied()
        .filter(|extractor| extractor.supports_language(language))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_extractor_ids_are_unique() {
        use std::collections::BTreeSet;
        let ids = semantic_extractors()
            .iter()
            .map(|extractor| extractor.id())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), semantic_extractors().len());
    }

    #[test]
    fn rust_resolves_to_the_static_unsupported_extractor() {
        let matches = extractors_for_language("rust");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id(), "atlas.rust.static-unsupported.v1");
        assert!(matches[0].supported_dimensions().is_empty());
    }

    #[test]
    fn unknown_language_resolves_to_no_extractor() {
        assert!(extractors_for_language("cobol").is_empty());
    }
}
