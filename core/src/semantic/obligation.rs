//! Canonical typed obligation ledger (R4.3.3).
//!
//! R4.3.2 preserved `CensusReport.evidence`/`diagnostics`, but the full relation an obligation
//! represents -- artifact + extractor + dimension + status + which observations satisfy it, which
//! evidence supports it, which diagnostics explain an UNKNOWN/UNSUPPORTED status -- lived only in
//! the runtime-local `CensusExtractionAccounting`, never in a serialized, canonical report. A
//! `SystemizeReport` could therefore contain diagnostics and a compatibility `SemanticObligation`
//! fact saying "artifact X, SYMBOL, UNKNOWN" with no durable typed record connecting that status to
//! the extractor/evidence/diagnostics that produced it. `SemanticObligationRecord` is that missing
//! typed record: the canonical Census (`CensusReport.typed_obligations`) must be able to answer,
//! from itself alone -- without the original transient `ExtractionBatch`es -- which extractor
//! evaluated which dimension, for which artifact, at which revision, with what status, which raw
//! observations satisfy it, which evidence supports it, and which diagnostics explain it.

use super::{ExtractorIdentity, SemanticDimension, SemanticRecordId};
use crate::identity::{ArtifactId, EvidenceId, RepositoryId, SemanticObligationId, stable_id};
use crate::schema::EpistemicStatus;
use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

/// One extractor's obligation result for one `SemanticDimension`, on one artifact, at one
/// revision, preserved as a canonical typed record rather than only inside a runtime-local
/// accounting structure.
///
/// `obligation_id` is a coordinate identity -- `(repository, revision, artifact, extractor,
/// dimension)` -- not a content hash: by construction (`ExtractionBatch::is_closed`), exactly one
/// `ObligationResult` exists per requested dimension per batch, and exactly one batch exists per
/// (artifact, extractor) pair in one extraction run, so this coordinate is already the obligation's
/// natural primary key. Two records that legitimately share every coordinate field always carry
/// identical content too; if that invariant is ever violated, callers must preserve both records
/// rather than let one silently overwrite the other (`.atlas/contracts/SEMANTIC-EXTRACTION.md#multi-engine-extraction`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticObligationRecord {
    pub obligation_id: SemanticObligationId,
    pub artifact: ArtifactId,
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub extractor: ExtractorIdentity,
    pub dimension: SemanticDimension,
    pub status: EpistemicStatus,
    /// Which raw observations (in `CensusReport.typed_semantic_records`, by `record_id`) satisfy
    /// this obligation. Empty is valid for `UNKNOWN`/`UNSUPPORTED`/`IGNORED` statuses, or for a
    /// verified-absence `OBSERVED` result backed only by `evidence_refs`.
    pub observation_ids: Vec<SemanticRecordId>,
    /// Which `Evidence` records (in `CensusReport.evidence`, by id) support this obligation.
    pub evidence_refs: Vec<EvidenceId>,
    /// Which `ExtractionDiagnostic`s (in `CensusReport.diagnostics`, by id) explain this
    /// obligation's status -- e.g. distinguishing `ParseFailure` from `InvalidInput` for two
    /// obligations that both report `UNKNOWN`.
    pub diagnostic_ids: Vec<String>,
}

impl SemanticObligationRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        artifact: ArtifactId,
        repository: RepositoryId,
        revision: RevisionRef,
        extractor: ExtractorIdentity,
        dimension: SemanticDimension,
        status: EpistemicStatus,
        observation_ids: Vec<SemanticRecordId>,
        evidence_refs: Vec<EvidenceId>,
        diagnostic_ids: Vec<String>,
    ) -> Self {
        let obligation_id = SemanticObligationId::new(stable_id(
            "semantic-obligation",
            &format!(
                "{}|{}:{}|{}|{}:{}|{}",
                repository.as_str(),
                revision.kind,
                revision.value,
                artifact.as_str(),
                extractor.id,
                extractor.version,
                dimension.as_str(),
            ),
        ));
        Self {
            obligation_id,
            artifact,
            repository,
            revision,
            extractor,
            dimension,
            status,
            observation_ids,
            evidence_refs,
            diagnostic_ids,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(dimension: SemanticDimension, status: EpistemicStatus) -> SemanticObligationRecord {
        SemanticObligationRecord::new(
            ArtifactId::new("artifact:src/lib.rs"),
            RepositoryId::new("atlas-studio"),
            RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            ExtractorIdentity {
                id: "atlas.rust.source-semantic.v1".into(),
                version: "0.1.0".into(),
            },
            dimension,
            status,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    }

    #[test]
    fn obligation_id_is_deterministic_for_identical_coordinates() {
        let a = record(SemanticDimension::Symbol, EpistemicStatus::Observed);
        let b = record(SemanticDimension::Symbol, EpistemicStatus::Observed);
        assert_eq!(a.obligation_id, b.obligation_id);
    }

    #[test]
    fn obligation_id_differs_by_dimension() {
        let a = record(SemanticDimension::Symbol, EpistemicStatus::Observed);
        let b = record(SemanticDimension::Type, EpistemicStatus::Observed);
        assert_ne!(a.obligation_id, b.obligation_id);
    }

    #[test]
    fn obligation_id_does_not_depend_on_status() {
        // obligation_id is a coordinate identity, not a content hash: the same (artifact,
        // extractor, dimension) always yields the same id regardless of status, so a status
        // transition never orphans the obligation's own identity.
        let a = record(SemanticDimension::Symbol, EpistemicStatus::Unknown);
        let b = record(SemanticDimension::Symbol, EpistemicStatus::Observed);
        assert_eq!(a.obligation_id, b.obligation_id);
    }
}
