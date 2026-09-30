//! AtlasX: the deterministic executable projection of one SelectedDesign from one SEALED Atlas
//! root (`.atlas/contracts/ATLAS-TO-ATLASX.md`, `.atlas/contracts/ATLASX-FORMAT.md`).
//!
//! Five parts exist. The precondition gate (G179, construction node M10, ADR 0093) decides
//! whether a parent may be materialized at all; since G185 (ADR 0097) it re-runs the seal gate
//! over the inputs the seal record names, so an admitted parent is one the gate decided. The
//! object codec (G183, M12, ADR 0095) writes and reads one object class, FUNCTIONS, canonically
//! and addressed by its BLAKE3 digest. The materializer (G185, M11, ADR 0097) stages the
//! FUNCTIONS object of an admitted parent's selected functions, with lineage into the parent's
//! census records. G187 (M13, ADR 0099): the root manifest (`manifest`, class ROOT_MANIFEST
//! through the same codec framing) lists the staged objects and binds the parent, and the AtlasX
//! root identity is computed from it; the materializer stages it after its objects. The
//! validator (`validate`) judges one staged root against its parent in the contract's reader
//! order and says VALID or INVALID with typed defects. Every other class is unmaterialized, and
//! nothing is published or advertised: a VALID root is a validated staged root of the FUNCTIONS
//! class only, and every verdict lists what it does not verify.

pub mod codec;
#[cfg(test)]
pub(crate) mod fixture;
pub mod manifest;
pub mod materialize;
pub mod precondition;
pub mod validate;

pub use codec::{
    CODEC_NOT_VERIFIED, CODEC_SCHEMA_VERSION, CodecDefect, CodecReport, CodecVerdict,
    EncodedObject, FunctionSignatureRecord, ObjectHeader, decode, decode_named, encode_functions,
    encode_module, object_address,
};
pub use manifest::{
    AtlasxManifest, CLASS_ROOT_MANIFEST, MANIFEST_FILE, MANIFEST_SCHEMA_VERSION, ObjectEntry,
    encode_manifest, read_manifest, root_identity,
};
pub use materialize::{
    COMPILER_IR_CONTRACT_UNKNOWN, FUNCTIONS_DIRECTORY, MATERIALIZATION_SCHEMA_VERSION,
    MATERIALIZE_NOT_DONE, MATERIALIZER_IDENTITY, MATERIALIZER_VERSION, Materialization,
    MaterializationDefect, MaterializationVerdict, MaterializedFunction, StagedObject,
    TARGET_KIND_NONE, check_lineage, materialize, scope_id,
};
pub use precondition::{
    AdmittedParent, Declaration, Declared, NOT_VERIFIED, PRECONDITION_SCHEMA_VERSION, Precondition,
    PreconditionInputs, PreconditionRefusal, PreconditionVerdict, Reports, admit, precondition,
};
pub use validate::{
    RootEntry, RootFiles, VALIDATE_NOT_VERIFIED, VALIDATION_SCHEMA_VERSION, Validation,
    ValidationDefect, ValidationVerdict, validate,
};
