//! End-to-end boundary tests proving R4.2's accounting/determinism guarantees.
//!
//! Fixture extractors here exist only to exercise the boundary (`.atlas/contracts/
//! SEMANTIC-EXTRACTION.md` section "No Rust deep extraction yet"); they perform no real semantic
//! analysis. That is R4.3+.

use super::batch::{ExtractionBatch, ObligationResult};
use super::extractor::{DiagnosticCode, ExtractionDiagnostic, ExtractionInput, SemanticExtractor};
use atlas_core::{
    ArtifactId, ContentFingerprint, EpistemicStatus, Evidence, EvidenceId, RepositoryId,
    RevisionRef, SemanticDimension, SemanticObservation, SemanticRecordHeader, SemanticRecordId,
    SemanticScope, SymbolIdentity, SymbolRole, stable_id,
};

fn fixture_input(artifact_path: &str, dimensions: Vec<SemanticDimension>) -> ExtractionInput {
    ExtractionInput {
        repository: RepositoryId::new("atlas-studio"),
        revision: RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        },
        artifact: ArtifactId::new(format!("artifact:{artifact_path}")),
        artifact_path: artifact_path.to_owned(),
        source_text: String::new(),
        content_fingerprint: Some(ContentFingerprint(format!("sha256:{artifact_path}"))),
        source_frontend_id: "atlas.test.fixture-frontend.v1".into(),
        language: "fixture".into(),
        build_profile: None,
        scope_policy: None,
        requested_dimensions: dimensions,
    }
}

/// Observes a SYMBOL when the artifact path contains `"known"`; explicit UNKNOWN otherwise (case
/// 5: insufficient evidence). Any other requested dimension is explicit UNSUPPORTED (this fixture
/// declares support for SYMBOL only).
struct FixtureSymbolExtractor {
    id: &'static str,
    version: &'static str,
}

impl SemanticExtractor for FixtureSymbolExtractor {
    fn id(&self) -> &'static str {
        self.id
    }

    fn version(&self) -> &'static str {
        self.version
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        &["fixture"]
    }

    fn supported_dimensions(&self) -> &'static [SemanticDimension] {
        &[SemanticDimension::Symbol]
    }

    fn extract(&self, input: &ExtractionInput) -> ExtractionBatch {
        let extractor = self.identity();
        let input_fingerprint = input.identity_key(&extractor);
        let mut observations = Vec::new();
        let mut evidence = Vec::new();
        let mut obligations = Vec::new();
        let mut diagnostics = Vec::new();

        for &dimension in &input.requested_dimensions {
            if dimension != SemanticDimension::Symbol {
                let diagnostic = ExtractionDiagnostic::new(
                    DiagnosticCode::UnsupportedSemanticDimension,
                    Some(dimension),
                    format!(
                        "fixture extractor only supports SYMBOL, not {}",
                        dimension.as_str()
                    ),
                );
                obligations.push(ObligationResult::unsupported(
                    dimension,
                    diagnostic.id.clone(),
                ));
                diagnostics.push(diagnostic);
                continue;
            }

            if input.artifact_path.contains("known") {
                let evidence_id = EvidenceId::new(stable_id(
                    "evidence",
                    &format!("{input_fingerprint}:symbol"),
                ));
                evidence.push(Evidence {
                    id: evidence_id.as_str().to_owned(),
                    kind: "SOURCE_TEXT".into(),
                    path: input.artifact_path.clone(),
                    summary: "fixture-observed top-level symbol `known`".into(),
                    revision: Some(input.revision.clone()),
                });
                let symbol = SymbolIdentity {
                    path: String::new(),
                    repository: input.repository.clone(),
                    revision: input.revision.clone(),
                    scope: SemanticScope::new(["fixture"]),
                    name: "known".into(),
                    role: SymbolRole::Definition,
                    documentation: None,
                    declaration: None,
                };
                let record_id =
                    SemanticRecordId::new(SemanticDimension::Symbol, &symbol.identity_key());
                observations.push(SemanticObservation::Symbol(SemanticRecordHeader {
                    record_id: record_id.clone(),
                    dimension: SemanticDimension::Symbol,
                    status: EpistemicStatus::Observed,
                    subject: symbol,
                    scope: SemanticScope::new(["fixture"]),
                    repository: input.repository.clone(),
                    revision: input.revision.clone(),
                    extractor: extractor.clone(),
                    evidence_refs: vec![evidence_id.clone()],
                    provenance: atlas_core::provenance(&input.artifact_path, &extractor.id),
                }));
                obligations.push(ObligationResult::observed(
                    dimension,
                    vec![record_id],
                    vec![evidence_id],
                ));
            } else {
                let diagnostic = ExtractionDiagnostic::new(
                    DiagnosticCode::IncompleteAnalysis,
                    Some(dimension),
                    "fixture extractor found no evidence for this artifact".to_owned(),
                );
                obligations.push(ObligationResult::unknown(dimension, diagnostic.id.clone()));
                diagnostics.push(diagnostic);
            }
        }

        ExtractionBatch {
            extractor,
            repository: input.repository.clone(),
            revision: input.revision.clone(),
            artifact: input.artifact.clone(),
            input_fingerprint,
            observations,
            evidence,
            obligations,
            diagnostics,
        }
    }
}

/// Simulates an internal extractor crash (case 6: extractor failure). Every requested dimension
/// still ends up explicitly accounted (`UNKNOWN`) with a diagnostic -- never silently dropped.
struct FixtureFailingExtractor;

impl SemanticExtractor for FixtureFailingExtractor {
    fn id(&self) -> &'static str {
        "atlas.test.fixture-failing"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        &["fixture"]
    }

    fn supported_dimensions(&self) -> &'static [SemanticDimension] {
        &[SemanticDimension::Symbol]
    }

    fn extract(&self, input: &ExtractionInput) -> ExtractionBatch {
        let extractor = self.identity();
        let input_fingerprint = input.identity_key(&extractor);
        let mut obligations = Vec::new();
        let mut diagnostics = Vec::new();

        for &dimension in &input.requested_dimensions {
            let diagnostic = ExtractionDiagnostic::new(
                DiagnosticCode::InternalExtractorFailure,
                Some(dimension),
                "fixture extractor crashed before producing any observation".to_owned(),
            );
            obligations.push(ObligationResult::unknown(dimension, diagnostic.id.clone()));
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

const FIXTURE_A: FixtureSymbolExtractor = FixtureSymbolExtractor {
    id: "atlas.test.fixture-symbol-a",
    version: "0.1.0",
};
const FIXTURE_B: FixtureSymbolExtractor = FixtureSymbolExtractor {
    id: "atlas.test.fixture-symbol-b",
    version: "0.2.0",
};

// --- 1. extractor identity stability ---------------------------------------------------------

#[test]
fn extractor_identity_is_stable_across_calls() {
    assert_eq!(FIXTURE_A.identity(), FIXTURE_A.identity());
    let a = FIXTURE_A.extract(&fixture_input("known.rs", vec![SemanticDimension::Symbol]));
    let b = FIXTURE_A.extract(&fixture_input("known.rs", vec![SemanticDimension::Symbol]));
    assert_eq!(a.extractor, b.extractor);
}

// --- 2. identical ExtractionInput -> deterministic equivalent ExtractionBatch ------------------

#[test]
fn identical_input_produces_equal_batches() {
    let input = fixture_input(
        "known.rs",
        vec![SemanticDimension::Symbol, SemanticDimension::Type],
    );
    let a = FIXTURE_A.extract(&input);
    let b = FIXTURE_A.extract(&input);
    assert_eq!(a, b);
}

// --- 3 & 7. every requested dimension accounted; no silent omission ---------------------------

#[test]
fn every_requested_dimension_is_accounted_with_no_omission() {
    let requested = vec![
        SemanticDimension::Symbol,
        SemanticDimension::Type,
        SemanticDimension::FunctionIdentity,
        SemanticDimension::FunctionSignature,
        SemanticDimension::Call,
        SemanticDimension::ControlFlow,
        SemanticDimension::DataFlow,
        SemanticDimension::State,
        SemanticDimension::Effect,
        SemanticDimension::Ownership,
        SemanticDimension::Concurrency,
        SemanticDimension::Persistence,
    ];
    let batch = FIXTURE_A.extract(&fixture_input("known.rs", requested.clone()));
    assert!(batch.is_closed(&requested));
    for dimension in &requested {
        assert!(
            batch.obligation_for(*dimension).is_some(),
            "{dimension:?} missing from batch"
        );
    }
}

// --- 4. unsupported dimension -> explicit UNSUPPORTED ------------------------------------------

#[test]
fn unsupported_dimension_is_explicit() {
    let batch = FIXTURE_A.extract(&fixture_input("known.rs", vec![SemanticDimension::Call]));
    let obligation = batch.obligation_for(SemanticDimension::Call).unwrap();
    assert_eq!(obligation.status, EpistemicStatus::Unsupported);
    assert!(!obligation.diagnostics.is_empty());
}

// --- 5. insufficient evidence -> explicit UNKNOWN ----------------------------------------------

#[test]
fn insufficient_evidence_is_explicit_unknown() {
    let batch = FIXTURE_A.extract(&fixture_input(
        "unrelated.rs",
        vec![SemanticDimension::Symbol],
    ));
    let obligation = batch.obligation_for(SemanticDimension::Symbol).unwrap();
    assert_eq!(obligation.status, EpistemicStatus::Unknown);
    assert!(obligation.observation_ids.is_empty());
    assert!(!obligation.diagnostics.is_empty());
    assert!(batch.observations.is_empty());
}

// --- 6. extractor failure -> diagnostic + accounted result -------------------------------------

#[test]
fn extractor_failure_still_accounts_for_every_dimension() {
    let requested = vec![SemanticDimension::Symbol, SemanticDimension::Type];
    let batch = FixtureFailingExtractor.extract(&fixture_input("known.rs", requested.clone()));
    assert!(batch.is_closed(&requested));
    for dimension in &requested {
        let obligation = batch.obligation_for(*dimension).unwrap();
        assert_eq!(obligation.status, EpistemicStatus::Unknown);
        assert_eq!(obligation.diagnostics.len(), 1);
    }
    assert_eq!(batch.diagnostics.len(), 2);
    assert!(
        batch
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code == DiagnosticCode::InternalExtractorFailure)
    );
}

// --- 8. SourceFrontend and SemanticExtractor remain separate layers ----------------------------

#[test]
fn source_frontend_and_extractor_are_decoupled_layers() {
    // The extractor only ever sees the frontend's *output* (plain id/language strings); it never
    // takes a `dyn SourceFrontend`, and `crate::source` is never imported by `crate::semantic`.
    let matched = crate::source::resolve_source_frontend(std::path::Path::new("src/lib.rs"))
        .expect("rust is a recognized source family");
    let mut input = fixture_input("known.rs", vec![SemanticDimension::Symbol]);
    input.source_frontend_id = matched.frontend_id.to_owned();
    input.language = "fixture".into(); // extraction still keyed by the extractor's own language
    let batch = FIXTURE_A.extract(&input);
    assert!(batch.obligation_for(SemanticDimension::Symbol).is_some());
}

// --- 9. extractor cannot mutate the engineering graph directly ---------------------------------

#[test]
fn extractor_boundary_has_no_graph_dependency() {
    for source in [
        include_str!("mod.rs"),
        include_str!("extractor.rs"),
        include_str!("batch.rs"),
        include_str!("registry.rs"),
    ] {
        assert!(
            !source.contains("EngineeringGraph") && !source.contains("atlas_core::graph"),
            "semantic extractor boundary must not depend on the engineering graph"
        );
    }
}

// --- 10. canonical EpistemicStatus reused (compile-time: ObligationResult.status : EpistemicStatus) --

#[test]
fn obligation_status_is_the_canonical_epistemic_status_type() {
    let batch = FIXTURE_A.extract(&fixture_input("known.rs", vec![SemanticDimension::Symbol]));
    let status: EpistemicStatus = batch
        .obligation_for(SemanticDimension::Symbol)
        .unwrap()
        .status;
    assert_eq!(status, EpistemicStatus::Observed);
}

// --- 11. provenance/evidence preserved ----------------------------------------------------------

#[test]
fn observed_records_remain_attributable_to_artifact_revision_and_extractor() {
    let batch = FIXTURE_A.extract(&fixture_input("known.rs", vec![SemanticDimension::Symbol]));
    assert_eq!(batch.evidence.len(), 1);
    let evidence = &batch.evidence[0];
    assert_eq!(evidence.path, "known.rs");
    assert_eq!(evidence.revision.as_ref().unwrap().value, "abc123");

    let SemanticObservation::Symbol(header) = &batch.observations[0] else {
        panic!("expected a Symbol observation");
    };
    assert_eq!(header.extractor, FIXTURE_A.identity());
    assert_eq!(header.provenance.source_path, "known.rs");
    assert_eq!(header.provenance.extractor, FIXTURE_A.identity().id);
    assert_eq!(header.evidence_refs.len(), 1);
    assert_eq!(header.evidence_refs[0].as_str(), evidence.id);
}

// --- 12. multiple extractor identities remain distinguishable -----------------------------------

#[test]
fn independent_extractors_stay_independently_attributable() {
    let input = fixture_input("known.rs", vec![SemanticDimension::Symbol]);
    let a = FIXTURE_A.extract(&input);
    let b = FIXTURE_B.extract(&input);

    assert_ne!(a.extractor, b.extractor);
    // Same subject observed by two independent extractors: neither overwrites the other: they
    // are two distinct observations with distinct extractor attribution, left for reconciliation.
    assert_eq!(a.observations.len(), 1);
    assert_eq!(b.observations.len(), 1);
    let SemanticObservation::Symbol(header_a) = &a.observations[0] else {
        panic!()
    };
    let SemanticObservation::Symbol(header_b) = &b.observations[0] else {
        panic!()
    };
    assert_eq!(header_a.subject, header_b.subject); // same claimed subject
    assert_ne!(header_a.extractor, header_b.extractor); // distinct attribution
}

// --- 13. no random/order-dependent IDs -----------------------------------------------------------

#[test]
fn requested_dimension_order_does_not_affect_output() {
    let forward = fixture_input(
        "known.rs",
        vec![
            SemanticDimension::Symbol,
            SemanticDimension::Type,
            SemanticDimension::Call,
        ],
    );
    let reversed = fixture_input(
        "known.rs",
        vec![
            SemanticDimension::Call,
            SemanticDimension::Type,
            SemanticDimension::Symbol,
        ],
    );
    let extractor = FIXTURE_A.identity();
    assert_eq!(
        forward.identity_key(&extractor),
        reversed.identity_key(&extractor)
    );

    let mut a = FIXTURE_A.extract(&forward).obligations;
    let mut b = FIXTURE_A.extract(&reversed).obligations;
    a.sort_by_key(|obligation| obligation.dimension.as_str());
    b.sort_by_key(|obligation| obligation.dimension.as_str());
    assert_eq!(a, b);
}
