//! Historical AtlasX object codecs and source staging.
//!
//! This source module preserves the existing wire contracts, identities and typed refusals.
//! Decoding authenticates bytes. `staging` evaluates sealed-parent prerequisites, materializes
//! selected FUNCTIONS objects and validates their staged root. These source integrity checks
//! grant no Chronica authority or semantic equivalence.

pub mod census_container;
pub mod codec;
pub mod manifest;
pub mod staging;

pub use codec::{
    CODEC_NOT_VERIFIED, CODEC_SCHEMA_VERSION, CodecDefect, CodecReport, CodecVerdict,
    EncodedObject, FunctionSignatureRecord, ObjectHeader, decode, decode_named, encode_functions,
    encode_module, object_address,
};
pub use manifest::{
    AtlasxManifest, CLASS_ROOT_MANIFEST, MANIFEST_FILE, MANIFEST_SCHEMA_VERSION, ObjectEntry,
    encode_manifest, read_manifest, root_identity,
};
