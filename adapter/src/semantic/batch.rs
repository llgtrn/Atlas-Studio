//! Typed extraction output and per-dimension obligation accounting.

use atlas_core::{
    ArtifactId, EpistemicStatus, Evidence, EvidenceId, ExtractorIdentity, RepositoryId,
    RevisionRef, SemanticDimension, SemanticObservation, SemanticRecordId,
};
use serde::{Deserialize, Serialize};

use super::extractor::ExtractionDiagnostic;

/// Per-dimension extraction accounting. Distinct from `EpistemicStatus`: `EpistemicStatus` is the
/// canonical knowledge-state taxonomy (`.atlas/contracts/SEMANTIC-FACTS.md`); `ObligationResult`
/// is the operational record of how one requested `SemanticDimension` was accounted for by one
/// extraction run, composed *from* an `EpistemicStatus` plus the observations/evidence/diagnostics
/// that justify it. Reusing `EpistemicStatus` as its `status` field (rather than inventing a
/// second epistemic enum) is intentional: `.atlas/contracts/SEMANTIC-EXTRACTION.md#obligation-result`
/// enumerates exactly the outcomes `EpistemicStatus` already names (OBSERVED, UNKNOWN, UNSUPPORTED,
/// IGNORED, CONFLICT).
///
/// - `status = Observed`, `observation_ids` non-empty: evidence-backed observation(s).
/// - `status = Observed`, `observation_ids` empty, `evidence_refs` non-empty: verified absence —
///   the obligation set was exhaustively checked and found nothing (never bare "not found").
/// - `status = Unknown`: evidence was insufficient to answer the obligation.
/// - `status = Unsupported`: this extractor/runtime cannot evaluate the dimension yet.
/// - `status = Ignored`: explicit policy exclusion (see `scope_policy` on `ExtractionInput`).
/// - `status = Conflict`: multiple incompatible candidate observations remain in `observation_ids`;
///   reconciliation, not this extraction run, picks a winner (`.atlas/contracts/NORMALIZATION.md`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObligationResult {
    pub dimension: SemanticDimension,
    pub status: EpistemicStatus,
    pub observation_ids: Vec<SemanticRecordId>,
    pub evidence_refs: Vec<EvidenceId>,
    pub diagnostics: Vec<String>,
}

impl ObligationResult {
    pub fn unsupported(dimension: SemanticDimension, diagnostic_id: impl Into<String>) -> Self {
        Self {
            dimension,
            status: EpistemicStatus::Unsupported,
            observation_ids: Vec::new(),
            evidence_refs: Vec::new(),
            diagnostics: vec![diagnostic_id.into()],
        }
    }

    pub fn unknown(dimension: SemanticDimension, diagnostic_id: impl Into<String>) -> Self {
        Self {
            dimension,
            status: EpistemicStatus::Unknown,
            observation_ids: Vec::new(),
            evidence_refs: Vec::new(),
            diagnostics: vec![diagnostic_id.into()],
        }
    }

    pub fn observed(
        dimension: SemanticDimension,
        observation_ids: Vec<SemanticRecordId>,
        evidence_refs: Vec<EvidenceId>,
    ) -> Self {
        Self {
            dimension,
            status: EpistemicStatus::Observed,
            observation_ids,
            evidence_refs,
            diagnostics: Vec::new(),
        }
    }
}

/// Typed extraction output for one artifact. See
/// `.atlas/contracts/SEMANTIC-EXTRACTION.md#extractionbatch`. Deliberately not
/// `Vec<SemanticFact { subject, predicate, object }>`: `observations` carries the typed R4.1
/// kernel records, and the bootstrap triple envelope is never reintroduced here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtractionBatch {
    pub extractor: ExtractorIdentity,
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub artifact: ArtifactId,
    pub input_fingerprint: String,
    pub observations: Vec<SemanticObservation>,
    pub evidence: Vec<Evidence>,
    pub obligations: Vec<ObligationResult>,
    pub diagnostics: Vec<ExtractionDiagnostic>,
}

impl ExtractionBatch {
    /// `true` iff every dimension in `requested` has exactly one `ObligationResult`, with no
    /// duplicates and no omissions. This is the structural proof that no requested dimension
    /// silently disappeared (`.atlas/contracts/CENSUS-COMPLETENESS.md`).
    pub fn is_closed(&self, requested: &[SemanticDimension]) -> bool {
        if self.obligations.len() != requested.len() {
            return false;
        }
        requested.iter().all(|dimension| {
            self.obligations
                .iter()
                .filter(|obligation| obligation.dimension == *dimension)
                .count()
                == 1
        })
    }

    pub fn obligation_for(&self, dimension: SemanticDimension) -> Option<&ObligationResult> {
        self.obligations
            .iter()
            .find(|obligation| obligation.dimension == dimension)
    }
}
