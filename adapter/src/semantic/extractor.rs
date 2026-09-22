//! Semantic extractor boundary.
//!
//! `SourceFrontend` (see `super::super::source::frontend`) owns structural source-family
//! recognition. A `SemanticExtractor` is deeper, evidence-producing analysis behind that
//! boundary; neither owns canonical truth (`.atlas/decisions/0001-one-normalized-semantic-path.md`).
//! Extraction converts a pinned admitted artifact into typed raw observations and explicit
//! obligation accounting. It does not normalize identities globally, reconcile conflicts, invent
//! facts, or write the engineering graph directly — this module never imports the
//! engineering-graph module at all.

use atlas_core::{ArtifactId, ContentFingerprint, ExtractorIdentity, RepositoryId, RevisionRef};
use serde::{Deserialize, Serialize};

use super::batch::{ExtractionBatch, ObligationResult};

/// Re-exported from `atlas_core` (R4.3.2): `DiagnosticCode`/`ExtractionDiagnostic` are pure typed
/// data with no adapter-specific mechanics, so they live in the core semantic kernel alongside
/// `SemanticObservation` and friends -- this lets `CensusReport` (a `core` type) retain full
/// diagnostic objects without a reverse `core` -> `adapter` dependency. Existing callers importing
/// them from this module (`adapter::semantic::extractor::{DiagnosticCode, ExtractionDiagnostic}`)
/// are unaffected.
pub use atlas_core::{DiagnosticCode, ExtractionDiagnostic};

/// Every field a `SemanticExtractor` may consult to produce a deterministic, revision-pinned
/// `ExtractionBatch`. See `.atlas/contracts/SEMANTIC-EXTRACTION.md#extractioninput`.
///
/// Deliberately excludes mutable global state, UI state, wall-clock time and random identifiers:
/// nothing here may vary output for identical (repository, revision, artifact, extractor/version,
/// profile, requested dimensions).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtractionInput {
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub artifact: ArtifactId,
    pub artifact_path: String,
    /// The artifact's full source text, read once by the runtime orchestration layer and passed
    /// in here so `extract` stays a pure function of its input (no extractor-side filesystem I/O,
    /// better determinism/testability). Untrusted input: parsing it never authorizes executing
    /// it (build.rs, proc macros, `cargo build`/`test`, shell/install scripts, network calls).
    pub source_text: String,
    pub content_fingerprint: Option<ContentFingerprint>,
    pub source_frontend_id: String,
    pub language: String,
    /// Build/configuration profile when semantics depend on it (target triple, feature set, ...).
    pub build_profile: Option<String>,
    /// Scope/policy input needed by extraction (e.g. an admitted census scope policy id).
    pub scope_policy: Option<String>,
    pub requested_dimensions: Vec<atlas_core::SemanticDimension>,
}

impl ExtractionInput {
    /// Deterministic, order-independent encoding of every field that must pin extraction output.
    /// Used to derive `ExtractionBatch::input_fingerprint`; never derived from wall-clock time,
    /// random ids, or collection/iteration order.
    pub fn identity_key(&self, extractor: &ExtractorIdentity) -> String {
        let mut dimensions = self
            .requested_dimensions
            .iter()
            .map(atlas_core::SemanticDimension::as_str)
            .collect::<Vec<_>>();
        dimensions.sort_unstable();
        format!(
            "{}|{}:{}|{}|{}|{}|{}|{}|{}|{}|{}|{}:{}",
            self.repository.as_str(),
            self.revision.kind,
            self.revision.value,
            self.artifact.as_str(),
            self.artifact_path,
            atlas_core::stable_id("source-text", &self.source_text),
            self.content_fingerprint
                .as_ref()
                .map(|f| f.0.as_str())
                .unwrap_or(""),
            self.source_frontend_id,
            self.language,
            self.build_profile.as_deref().unwrap_or(""),
            self.scope_policy.as_deref().unwrap_or(""),
            extractor.id,
            extractor.version,
        ) + &format!("|{}", dimensions.join(","))
    }
}

/// `SourceFrontend` decides structural eligibility/language; a `SemanticExtractor` performs the
/// deeper analysis behind that boundary. Neither may write the engineering graph directly
/// (`.atlas/decisions/0001-one-normalized-semantic-path.md`): `extract` returns data, never a
/// graph mutation.
pub trait SemanticExtractor: Sync {
    /// Stable extractor id, e.g. `"atlas.rust.static-unsupported"`. Never randomly generated.
    fn id(&self) -> &'static str;
    /// Stable implementation version, e.g. `"0.1.0"`.
    fn version(&self) -> &'static str;
    fn supported_languages(&self) -> &'static [&'static str];
    fn supported_dimensions(&self) -> &'static [atlas_core::SemanticDimension];

    fn identity(&self) -> ExtractorIdentity {
        ExtractorIdentity {
            id: self.id().to_owned(),
            version: self.version().to_owned(),
        }
    }

    fn supports_language(&self, language: &str) -> bool {
        self.supported_languages()
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(language))
    }

    /// Extract every requested dimension in `input.requested_dimensions`. Implementations MUST
    /// return exactly one `ObligationResult` per requested dimension (see
    /// `ExtractionBatch::is_closed`) — a dimension outside `supported_dimensions()` is accounted
    /// as `UNSUPPORTED`, never silently dropped.
    fn extract(&self, input: &ExtractionInput) -> ExtractionBatch;

    /// The closed `ExtractionBatch` this extractor would produce when `input.source_text` could
    /// not be obtained at all — e.g. a filesystem read failure in the runtime orchestration layer
    /// upstream of extraction (reading content is that layer's job, never this trait's; see
    /// `ExtractionInput::source_text`). `reason` should typically carry
    /// `DiagnosticCode::InvalidInput`, but any diagnostic describing why the source was
    /// unavailable is accepted.
    ///
    /// Every dimension this extractor supports becomes `UNKNOWN` — evidence was never obtainable,
    /// which is a distinct cause from "obtained but insufficient" yet the same epistemic status.
    /// Every dimension it does not support stays `UNSUPPORTED` regardless: source availability
    /// never changes what an extractor is capable of analyzing. A read failure must never remove
    /// the artifact from accounting (`.atlas/contracts/SEMANTIC-EXTRACTION.md#failure-semantics`),
    /// so this default implementation always returns a batch closed over
    /// `input.requested_dimensions`, never propagates an error, and never panics.
    fn unavailable(
        &self,
        input: &ExtractionInput,
        reason: ExtractionDiagnostic,
    ) -> ExtractionBatch {
        let extractor = self.identity();
        let input_fingerprint = input.identity_key(&extractor);
        let mut diagnostics = vec![reason.clone()];
        let mut obligations = Vec::with_capacity(input.requested_dimensions.len());

        for &dimension in &input.requested_dimensions {
            if self.supported_dimensions().contains(&dimension) {
                obligations.push(ObligationResult::unknown(dimension, reason.id.clone()));
            } else {
                let unsupported = ExtractionDiagnostic::new(
                    DiagnosticCode::UnsupportedSemanticDimension,
                    Some(dimension),
                    format!(
                        "{} does not support {} regardless of source availability",
                        self.id(),
                        dimension.as_str()
                    ),
                );
                obligations.push(ObligationResult::unsupported(
                    dimension,
                    unsupported.id.clone(),
                ));
                diagnostics.push(unsupported);
            }
        }
        obligations.sort_by_key(|obligation| obligation.dimension.as_str());

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
