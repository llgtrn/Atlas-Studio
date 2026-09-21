//! Contract for semantic extractors.
//!
//! Structural SourceFrontend admission is intentionally separate from semantic extraction. An
//! extractor produces typed observations and coverage; it never writes canonical graph state.

use atlas_core::{
    ArtifactId, ContentFingerprint, EpistemicStatus, RevisionRef, SemanticObligation,
    TypedSemanticFact,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticSourceUnit {
    pub artifact_id: ArtifactId,
    pub path: String,
    pub language: String,
    pub revision: Option<RevisionRef>,
    pub content_hash: Option<ContentFingerprint>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticObligationCoverage {
    pub obligation: SemanticObligation,
    pub status: EpistemicStatus,
    pub facts_total: usize,
    pub closure_proven: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticExtractionBatch {
    pub schema: String,
    pub extractor_id: String,
    pub extractor_version: String,
    pub source_artifact_id: ArtifactId,
    pub source_path: String,
    pub coverage: Vec<SemanticObligationCoverage>,
    pub facts: Vec<TypedSemanticFact>,
    pub diagnostics: Vec<String>,
}

impl SemanticExtractionBatch {
    pub fn is_accounted(&self) -> bool {
        self.coverage.iter().all(|coverage| match coverage.status {
            EpistemicStatus::Unknown
            | EpistemicStatus::Unsupported
            | EpistemicStatus::Ignored => true,
            EpistemicStatus::Hypothesis | EpistemicStatus::Conflict => false,
            EpistemicStatus::Observed
            | EpistemicStatus::Declared
            | EpistemicStatus::Derived
            | EpistemicStatus::Inferred => coverage.closure_proven,
        }) && self
            .facts
            .iter()
            .all(TypedSemanticFact::is_epistemically_valid)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticExtractionError {
    pub code: String,
    pub message: String,
}

pub trait SemanticExtractor: Sync {
    fn id(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn language(&self) -> &'static str;
    fn obligations(&self) -> &'static [SemanticObligation];

    fn extract(
        &self,
        unit: &SemanticSourceUnit,
    ) -> Result<SemanticExtractionBatch, SemanticExtractionError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch(status: EpistemicStatus, closure_proven: bool) -> SemanticExtractionBatch {
        SemanticExtractionBatch {
            schema: "test".into(),
            extractor_id: "test.extractor".into(),
            extractor_version: "1".into(),
            source_artifact_id: ArtifactId::new("artifact:test"),
            source_path: "src/lib.rs".into(),
            coverage: vec![SemanticObligationCoverage {
                obligation: SemanticObligation::Symbol,
                status,
                facts_total: 0,
                closure_proven,
                reason: None,
            }],
            facts: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    #[test]
    fn explicit_unsupported_obligation_is_accounted() {
        assert!(batch(EpistemicStatus::Unsupported, false).is_accounted());
    }

    #[test]
    fn positive_obligation_requires_closure_proof() {
        assert!(!batch(EpistemicStatus::Observed, false).is_accounted());
        assert!(batch(EpistemicStatus::Observed, true).is_accounted());
    }
}
