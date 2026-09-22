//! Semantic extractor boundary (R4.2).
//!
//! Canonical path: `Inventory -> SourceFrontend -> SemanticExtractor -> ExtractionBatch -> Census
//! -> Normalize -> Reconcile -> Graph/ATLAS projection`
//! (`.atlas/decisions/0001-one-normalized-semantic-path.md`). This module owns the boundary and
//! its accounting model only; it performs no deep Rust (or any other language) semantic analysis
//! — that is R4.3+. It has no dependency on `atlas_core::graph`: an extractor cannot mutate the
//! engineering graph directly.

pub mod batch;
pub mod extractor;
pub mod registry;

pub use batch::{ExtractionBatch, ObligationResult};
pub use extractor::{DiagnosticCode, ExtractionDiagnostic, ExtractionInput, SemanticExtractor};
pub use registry::{StaticUnsupportedExtractor, extractors_for_language, semantic_extractors};
