//! Semantic extractor boundary (R4.2) plus its first real extractor (R4.3, `rust`).
//!
//! Canonical path: `Inventory -> SourceFrontend -> SemanticExtractor -> ExtractionBatch -> Census
//! -> Normalize -> Reconcile -> Graph/ATLAS projection`
//! (`.atlas/decisions/0001-one-normalized-semantic-path.md`). `batch`/`extractor`/`registry` own
//! the boundary and its accounting model, language-agnostically; `rust` is the first deep
//! semantic analyzer built behind that boundary (SYMBOL/TYPE/FUNCTION_IDENTITY/FUNCTION_SIGNATURE
//! only -- R4.4+ territory such as CALL/CONTROL_FLOW/compiler-backed resolution remains
//! unimplemented). Nothing under this module imports the engineering-graph module: an extractor
//! cannot mutate the engineering graph directly.

pub mod batch;
pub mod extractor;
pub mod registry;
pub mod rust;

#[cfg(test)]
mod boundary_tests;

pub use batch::{ExtractionBatch, ObligationResult};
pub use extractor::{DiagnosticCode, ExtractionDiagnostic, ExtractionInput, SemanticExtractor};
pub use registry::{StaticUnsupportedExtractor, extractors_for_language, semantic_extractors};
pub use rust::RustSemanticExtractor;
pub use rust::resolve::{
    CrateInput, FnTarget, PathCallOutcome, PathCallResolution, WorkspaceResolution,
    join as join_path, resolve_path_calls, resolve_workspace,
};
