//! AtlasX: the deterministic executable projection of one SelectedDesign from one SEALED Atlas
//! root (`.atlas/contracts/ATLAS-TO-ATLASX.md`, `.atlas/contracts/ATLASX-FORMAT.md`).
//!
//! Two parts exist. The precondition gate (G179, construction node M10, ADR 0093) decides whether
//! a parent may be materialized at all. The object codec (G183, M12, ADR 0095) writes and reads
//! one object class, FUNCTIONS, canonically and addressed by its BLAKE3 digest. The materializer
//! (M11) and the validator (M13) do not exist: no manifest, root identity or cross-object closure
//! is produced or checked.

pub mod codec;
pub mod precondition;

pub use codec::{
    CODEC_NOT_VERIFIED, CODEC_SCHEMA_VERSION, CodecDefect, CodecReport, CodecVerdict,
    EncodedObject, FunctionSignatureRecord, ObjectHeader, decode, decode_named, encode_functions,
    encode_module, object_address,
};
pub use precondition::{
    AdmittedParent, NOT_VERIFIED, PRECONDITION_SCHEMA_VERSION, Precondition, PreconditionInputs,
    PreconditionRefusal, PreconditionVerdict, admit, precondition,
};
