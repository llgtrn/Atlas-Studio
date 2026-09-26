//! Extraction diagnostic vocabulary.
//!
//! Pure typed data -- no filesystem/parsing mechanics (those remain `adapter`'s job). Lives here,
//! alongside the rest of the R4 semantic kernel, so the canonical census (`CensusReport`, a `core`
//! type) can retain full diagnostic objects -- not just their stable ids -- without a reverse
//! `core` -> `adapter` dependency. `adapter::semantic::extractor` re-exports these under the same
//! names for existing callers.

use super::SemanticDimension;
use crate::identity::stable_id;
use serde::{Deserialize, Serialize};

/// G162: the message prefix of an `INCOMPLETE_ANALYSIS` diagnostic naming the path calls of an
/// artifact the path-resolution engine withholds because a scope they are looked up in is open,
/// and why. The composed world model reads it back (`GAP-OPEN-SCOPE`, a component's `withheld`).
pub const OPEN_SCOPE_DIAGNOSTIC: &str = "open scope: ";

/// Minimum diagnostic causes an extractor may report; see
/// `.atlas/contracts/SEMANTIC-EXTRACTION.md#failure-semantics`. A diagnostic never removes the
/// artifact or the dimension from accounting — it always pairs with an explicit
/// `ObligationResult` (`UNKNOWN` or `UNSUPPORTED`, as appropriate).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticCode {
    UnsupportedLanguageOrProfile,
    UnsupportedSemanticDimension,
    ParseFailure,
    CompilerMetadataUnavailable,
    ResourceLimit,
    InvalidInput,
    IncompleteAnalysis,
    InternalExtractorFailure,
}

impl DiagnosticCode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::UnsupportedLanguageOrProfile => "UNSUPPORTED_LANGUAGE_OR_PROFILE",
            Self::UnsupportedSemanticDimension => "UNSUPPORTED_SEMANTIC_DIMENSION",
            Self::ParseFailure => "PARSE_FAILURE",
            Self::CompilerMetadataUnavailable => "COMPILER_METADATA_UNAVAILABLE",
            Self::ResourceLimit => "RESOURCE_LIMIT",
            Self::InvalidInput => "INVALID_INPUT",
            Self::IncompleteAnalysis => "INCOMPLETE_ANALYSIS",
            Self::InternalExtractorFailure => "INTERNAL_EXTRACTOR_FAILURE",
        }
    }
}

/// A typed extraction diagnostic. Stable, content-derived `id` — never a random UUID.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtractionDiagnostic {
    pub id: String,
    pub code: DiagnosticCode,
    pub dimension: Option<SemanticDimension>,
    pub message: String,
}

impl ExtractionDiagnostic {
    pub fn new(
        code: DiagnosticCode,
        dimension: Option<SemanticDimension>,
        message: impl Into<String>,
    ) -> Self {
        let message = message.into();
        let seed = format!(
            "{}|{}|{}",
            code.as_str(),
            dimension.map(|d| d.as_str()).unwrap_or(""),
            message
        );
        Self {
            id: stable_id("extraction-diagnostic", &seed),
            code,
            dimension,
            message,
        }
    }
}
