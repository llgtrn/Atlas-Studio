//! Contract for semantic extractors.
//!
//! Structural SourceFrontend admission is intentionally separate from semantic extraction. An
//! extractor produces typed observations and coverage; it never writes canonical graph state.

use atlas_core::{
    ArtifactId, EpistemicStatus, SemanticObligation, TypedSemanticFact,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticSourceUnit {
    pub artifact_id: ArtifactId,
    pub path: String,
    pub language: String,
    pub revision: Option<String>,
    pub content_hash: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticObligationCoverage {
    pub obligation: SemanticObligation,
    pub status: EpistemicStatus,
    pub facts_total: usize,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticExtractionBatch {
    pub schema: String,
    pub extractor_id: String,
    pub source_path: String,
    pub coverage: Vec<SemanticObligationCoverage>,
    pub facts: Vec<TypedSemanticFact>,
    pub diagnostics: Vec<String>,
}

impl SemanticExtractionBatch {
    pub fn is_accounted(&self) -> bool {
        self.coverage.iter().all(|coverage| {
            !matches!(
                coverage.status,
                EpistemicStatus::Hypothesis | EpistemicStatus::Conflict
            )
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticExtractionError {
    pub code: String,
    pub message: String,
}

pub trait SemanticExtractor: Sync {
    fn id(&self) -> &'static str;
    fn language(&self) -> &'static str;
    fn obligations(&self) -> &'static [SemanticObligation];

    fn extract(
        &self,
        unit: &SemanticSourceUnit,
    ) -> Result<SemanticExtractionBatch, SemanticExtractionError>;
}
