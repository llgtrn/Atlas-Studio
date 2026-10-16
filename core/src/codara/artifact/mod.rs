//! Pure codecs for historical AtlasX FUNCTIONS objects and root manifests.
//!
//! This source module preserves the existing wire contracts, identities and typed refusals.
//! Decoding authenticates bytes; sealed-parent admission, staging, lineage and root validation
//! remain in `crate::atlasx`. This module grants no Chronica authority or semantic equivalence.

pub mod codec;
pub mod manifest;

pub use codec::{
    CODEC_NOT_VERIFIED, CODEC_SCHEMA_VERSION, CodecDefect, CodecReport, CodecVerdict,
    EncodedObject, FunctionSignatureRecord, ObjectHeader, decode, decode_named, encode_functions,
    encode_module, object_address,
};
pub use manifest::{
    AtlasxManifest, CLASS_ROOT_MANIFEST, MANIFEST_FILE, MANIFEST_SCHEMA_VERSION, ObjectEntry,
    encode_manifest, read_manifest, root_identity,
};
