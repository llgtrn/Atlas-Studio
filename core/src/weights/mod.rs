//! Weight artifacts: the physical tensor schema of a neural-network weight container (G163,
//! owner directive, ADR 0078, `contracts/UNIVERSAL-ENGINEERING-WORLD.md` "Weight artifacts").
//!
//! A weight container is one more artifact class of the universal artifact model. This module is
//! its *physical* layer only: what a container stores -- tensor names, element types, shapes and
//! the byte ranges of their payloads -- never what a tensor means. A physical name such as
//! `model.layers.12.self_attn.q_proj.weight` is evidence for a semantic role, not the role: no
//! type here carries one, and a census reports every role `UNKNOWN` until an architecture mapping
//! proves it (stage 5, not implemented).
//!
//! Payload bytes are never copied into Atlas records. A tensor's payload is referenced by the
//! container's digest, its byte range and its own digest (the `.atlas` THIN mode: content stays
//! external, referenced by digest), so a census reads only the header plus bounded chunks, and
//! a construction streams each range, verifying its digest as it copies.

use crate::EpistemicStatus;
use crate::identity::IntegrityDigest;
use crate::vocabulary_enum;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const WEIGHT_CENSUS_SCHEMA: &str = "atlas.weight-census.v1";

vocabulary_enum! {
    /// Container formats Atlas censuses natively.
    pub enum WeightFormat {
        SafeTensors => "SAFETENSORS",
    }
}

vocabulary_enum! {
    /// Element types of a stored tensor, by their storage encoding.
    /// G166 (replay R9): the format's full element-type table, sub-byte types included.
    pub enum TensorDtype {
        Bool => "BOOL",
        F4 => "F4",
        F6E2M3 => "F6_E2M3",
        F6E3M2 => "F6_E3M2",
        U8 => "U8",
        I8 => "I8",
        F8E5M2 => "F8_E5M2",
        F8E4M3 => "F8_E4M3",
        F8E8M0 => "F8_E8M0",
        F8E4M3Fnuz => "F8_E4M3FNUZ",
        F8E5M2Fnuz => "F8_E5M2FNUZ",
        I16 => "I16",
        U16 => "U16",
        F16 => "F16",
        BF16 => "BF16",
        I32 => "I32",
        U32 => "U32",
        F32 => "F32",
        C64 => "C64",
        F64 => "F64",
        I64 => "I64",
        U64 => "U64",
    }
}

impl TensorDtype {
    /// Bits per element: sub-byte types pack elements across byte boundaries, so a tensor's
    /// size is its element count times this, in whole bytes. Block-quantized encodings (GGUF)
    /// are a later stage.
    pub const fn element_bits(self) -> u64 {
        match self {
            Self::F4 => 4,
            Self::F6E2M3 | Self::F6E3M2 => 6,
            Self::Bool
            | Self::U8
            | Self::I8
            | Self::F8E5M2
            | Self::F8E4M3
            | Self::F8E8M0
            | Self::F8E4M3Fnuz
            | Self::F8E5M2Fnuz => 8,
            Self::I16 | Self::U16 | Self::F16 | Self::BF16 => 16,
            Self::I32 | Self::U32 | Self::F32 => 32,
            Self::C64 | Self::F64 | Self::I64 | Self::U64 => 64,
        }
    }
}

/// Why a tensor's dtype and shape name no byte size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeError {
    /// The element or bit count overflows.
    Overflow,
    /// Sub-byte elements that do not fill whole bytes (G166: the format refuses them too).
    Misaligned { bits: u64 },
}

impl std::fmt::Display for SizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Overflow => f.write_str("element count overflows"),
            Self::Misaligned { bits } => write!(f, "{bits} bits do not fill whole bytes"),
        }
    }
}

vocabulary_enum! {
    /// What a transformation preserves (`contracts/UNIVERSAL-ENGINEERING-WORLD.md`). Exact
    /// reconstruction may be claimed across a LOSSLESS transform, and across a
    /// REVERSIBLE_WITH_RETAINED one only while the retained information is present; never across
    /// a LOSSY one.
    pub enum PreservationClass {
        Lossless => "LOSSLESS",
        ReversibleWithRetained => "REVERSIBLE_WITH_RETAINED",
        Lossy => "LOSSY",
    }
}

impl PreservationClass {
    /// Whether exact reconstruction of the source may be claimed across the transform.
    pub const fn exact_reconstruction_claimable(self, retained_present: bool) -> bool {
        match self {
            Self::Lossless => true,
            Self::ReversibleWithRetained => retained_present,
            Self::Lossy => false,
        }
    }
}

/// A byte range of a container's payload region, `start..end`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct PayloadRange {
    pub start: u64,
    pub end: u64,
}

/// One stored tensor as its container records it: storage facts only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhysicalTensor {
    /// The container's own name for the tensor -- evidence, never a semantic identity.
    pub name: String,
    pub dtype: TensorDtype,
    pub shape: Vec<u64>,
    /// Where the payload lies in the container's payload region.
    pub payload: PayloadRange,
    /// BLAKE3 of the payload bytes.
    pub digest: String,
}

/// The census of one weight container.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WeightCensus {
    pub schema: String,
    pub format: WeightFormat,
    /// BLAKE3 of the whole container: the artifact the payload ranges are read from.
    pub artifact_digest: String,
    pub artifact_bytes: u64,
    /// The payload region's offset in the container (the header precedes it).
    pub payload_offset: u64,
    pub payload_bytes: u64,
    /// Free-form string metadata the container carries, verbatim.
    pub metadata: BTreeMap<String, String>,
    /// In payload order.
    pub tensors: Vec<PhysicalTensor>,
    /// Semantic roles: `UNKNOWN` until an architecture mapping proves them.
    pub roles: EpistemicStatus,
    pub roles_basis: String,
}

/// Every reason `census` is not a well-formed physical census: element counts that overflow,
/// payload ranges that disagree with dtype x shape, overlap, holes, or ranges outside the payload,
/// duplicate names, a malformed digest.
pub fn validate(census: &WeightCensus) -> Vec<String> {
    let mut problems = Vec::new();
    if census.schema != WEIGHT_CENSUS_SCHEMA {
        problems.push(format!("unknown weight census schema `{}`", census.schema));
    }
    let mut names = std::collections::BTreeSet::new();
    let mut cursor = 0u64;
    for t in &census.tensors {
        if !names.insert(t.name.as_str()) {
            problems.push(format!("tensor `{}` appears twice", t.name));
        }
        match expected_bytes(t.dtype, &t.shape) {
            Err(error) => problems.push(format!("tensor `{}`: {error}", t.name)),
            Ok(bytes) => {
                if t.payload.end.checked_sub(t.payload.start) != Some(bytes) {
                    problems.push(format!(
                        "tensor `{}`: range {}..{} holds not the {bytes} bytes its dtype and shape need",
                        t.name, t.payload.start, t.payload.end
                    ));
                }
            }
        }
        if t.payload.start != cursor {
            problems.push(format!(
                "tensor `{}` starts at {}, not at {cursor}: payload ranges overlap or leave a hole",
                t.name, t.payload.start
            ));
        }
        cursor = cursor.max(t.payload.end);
        if IntegrityDigest::parse(&t.digest).is_err() {
            problems.push(format!("tensor `{}`: malformed digest", t.name));
        }
    }
    if cursor != census.payload_bytes {
        problems.push(format!(
            "tensors cover {cursor} of the {} payload bytes",
            census.payload_bytes
        ));
    }
    if census.payload_offset.checked_add(census.payload_bytes) != Some(census.artifact_bytes) {
        problems.push("header and payload do not tile the artifact".into());
    }
    if census.roles != EpistemicStatus::Unknown {
        problems.push("a physical census claims semantic roles it cannot prove".into());
    }
    problems
}

/// Bytes a tensor of `dtype` and `shape` occupies: its element count times the dtype's bits, in
/// whole bytes.
pub fn expected_bytes(dtype: TensorDtype, shape: &[u64]) -> Result<u64, SizeError> {
    let bits = shape
        .iter()
        .try_fold(1u64, |n, d| n.checked_mul(*d))
        .and_then(|count| count.checked_mul(dtype.element_bits()))
        .ok_or(SizeError::Overflow)?;
    if !bits.is_multiple_of(8) {
        return Err(SizeError::Misaligned { bits });
    }
    Ok(bits / 8)
}

/// A census's identity: BLAKE3 over its canonical serde form.
pub fn census_identity(census: &WeightCensus) -> String {
    let text = serde_json::to_string(census).expect("a WeightCensus always serializes");
    IntegrityDigest::of_bytes(text.as_bytes())
        .as_str()
        .to_owned()
}

/// What two censuses of the same content must agree on: everything but the container's own
/// digest, size and header placement, which a lossless repack may change.
pub fn same_content(a: &WeightCensus, b: &WeightCensus) -> bool {
    a.format == b.format
        && a.metadata == b.metadata
        && a.tensors == b.tensors
        && a.payload_bytes == b.payload_bytes
}

#[cfg(test)]
mod tests;
