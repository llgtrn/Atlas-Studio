//! AtlasX: the deterministic executable projection of one SelectedDesign from one SEALED Atlas
//! root (`.atlas/contracts/ATLAS-TO-ATLASX.md`, `.atlas/contracts/ATLASX-FORMAT.md`).
//!
//! Three parts exist. The precondition gate (G179, construction node M10, ADR 0093) decides
//! whether a parent may be materialized at all; since G185 (ADR 0097) it re-runs the seal gate
//! over the inputs the seal record names, so an admitted parent is one the gate decided. The
//! object codec (G183, M12, ADR 0095) writes and reads one object class, FUNCTIONS, canonically
//! and addressed by its BLAKE3 digest. The materializer (G185, M11, ADR 0097) stages the
//! FUNCTIONS object of an admitted parent's selected functions, with lineage into the parent's
//! census records, and nothing more: no manifest, root identity or publication, and every other
//! class unmaterialized. The validator (M13) does not exist: no staged root is checked.

pub mod codec;
#[cfg(test)]
pub(crate) mod fixture;
pub mod materialize;
pub mod precondition;

pub use codec::{
    CODEC_NOT_VERIFIED, CODEC_SCHEMA_VERSION, CodecDefect, CodecReport, CodecVerdict,
    EncodedObject, FunctionSignatureRecord, ObjectHeader, decode, decode_named, encode_functions,
    encode_module, object_address,
};
pub use materialize::{
    FUNCTIONS_DIRECTORY, MATERIALIZATION_SCHEMA_VERSION, MATERIALIZE_NOT_DONE, Materialization,
    MaterializationDefect, MaterializationVerdict, MaterializedFunction, StagedObject,
    check_lineage, materialize,
};
pub use precondition::{
    AdmittedParent, Declaration, Declared, NOT_VERIFIED, PRECONDITION_SCHEMA_VERSION, Precondition,
    PreconditionInputs, PreconditionRefusal, PreconditionVerdict, Reports, admit, precondition,
};
