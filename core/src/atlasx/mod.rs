//! AtlasX: the deterministic executable projection of one SelectedDesign from one SEALED Atlas
//! root (`.atlas/contracts/ATLAS-TO-ATLASX.md`, `.atlas/contracts/ATLASX-FORMAT.md`).
//!
//! Only the precondition gate exists (G179, construction node M10, ADR 0093): whether a parent
//! may be materialized at all. The materializer, codec and validator (M11, M12, M13) do not.

pub mod precondition;

pub use precondition::{
    AdmittedParent, NOT_VERIFIED, PRECONDITION_SCHEMA_VERSION, Precondition, PreconditionInputs,
    PreconditionRefusal, PreconditionVerdict, admit, precondition,
};
