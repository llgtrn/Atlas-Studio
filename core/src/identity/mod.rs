use serde::{Deserialize, Serialize};
use std::fmt;

pub mod blake3;

pub fn stable_id(prefix: &str, identity: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in identity.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{prefix}:{hash:016x}")
}

/// Escapes `\` and `separator` in `value` so joining several fields with `separator` as a
/// delimiter can never be confused with a different split of the same joined characters (e.g.
/// joining `["a|b", "c"]` and `["a", "b|c"]` with `|` would otherwise both produce `a|b|c`, and
/// two genuinely different composite identities would hash to the same `stable_id`).
///
/// Only needed for a field the caller cannot prove is free of `separator`. A field drawn from a
/// fixed, code-controlled set (an enum's own `.as_str()`, a literal) never needs this: the set of
/// possible values is closed and known not to contain `separator`. A field extracted through a
/// real lexer that forbids `separator` in a valid token (e.g. a Rust identifier via `syn` -- `|`
/// cannot appear in one) doesn't either. It IS needed for a field read by a permissive parser that
/// never validates its content against any charset (Cargo package `name`/`version` from a static
/// lockfile parser; an ADL relation's `from`/`relation`/`to`, extracted from raw source text by
/// simple substring splitting, not through a restrictive lexer) -- exactly the "malformed/hostile
/// input" class this codebase's own falsification testing is meant to cover.
pub fn escape_identity_field(value: &str, separator: char) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch == '\\' || ch == separator {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

typed_id!(NodeId);
typed_id!(EdgeId);
typed_id!(RepositoryId);
typed_id!(RevisionId);
typed_id!(SymbolId);
typed_id!(CapabilityId);
typed_id!(TechnologyId);
typed_id!(EvidenceId);
typed_id!(ArtifactId);
// Identity of one exact raw `SemanticObservation` -- distinct from `SemanticRecordId` (the
// semantic CLAIM identity, shared by every extractor that reports the same subject/scope/
// revision). See `SemanticObservation::raw_observation_id` and
// `.atlas/contracts/SEMANTIC-EXTRACTION.md#multi-engine-extraction`.
typed_id!(RawObservationId);
// Identity of one `SemanticObligationRecord` -- the (repository, revision, artifact, extractor,
// dimension) coordinate an `ObligationResult` was reported against. See
// `.atlas/contracts/CENSUS-COMPLETENESS.md`.
typed_id!(SemanticObligationId);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentFingerprint(pub String);

/// A collision-resistant content digest. The algorithm is spelled into the value itself
/// (`"blake3-256:<64 hex>"`), matching `.atlas/contracts/ATLAS-BINARY-WIRE-FORMAT.md`'s rule that
/// the exact algorithm is explicit on the artifact (wire id `1 = BLAKE3_256`).
///
/// Validated on construction and on deserialization: the only accepted spelling is the prefix
/// followed by exactly 64 lowercase hex digits, so one digest has exactly one textual form and
/// maps one-to-one onto the wire `digest_algorithm` id.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(try_from = "String", into = "String")]
pub struct IntegrityDigest(String);

impl IntegrityDigest {
    pub const BLAKE3_256_PREFIX: &'static str = "blake3-256:";
    /// `ATLAS-BINARY-WIRE-FORMAT.md` digest algorithm id for BLAKE3-256.
    pub const BLAKE3_256_WIRE_ID: u8 = 1;

    pub fn parse(value: &str) -> Result<Self, String> {
        let Some(hex) = value.strip_prefix(Self::BLAKE3_256_PREFIX) else {
            return Err(format!(
                "integrity digest `{value}` does not start with `blake3-256:`"
            ));
        };
        if hex.len() != 64
            || !hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(format!(
                "integrity digest `{value}` must carry exactly 64 lowercase hex digits"
            ));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn wire_algorithm_id(&self) -> u8 {
        Self::BLAKE3_256_WIRE_ID
    }

    pub fn blake3_256(digest: &[u8; 32]) -> Self {
        let mut value = String::with_capacity(Self::BLAKE3_256_PREFIX.len() + 64);
        value.push_str(Self::BLAKE3_256_PREFIX);
        for byte in digest {
            value.push_str(&format!("{byte:02x}"));
        }
        Self(value)
    }

    /// The BLAKE3-256 digest of `bytes`, computed by Atlas's own `identity::blake3`.
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self::blake3_256(&blake3::hash(bytes))
    }
}

impl TryFrom<String> for IntegrityDigest {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<IntegrityDigest> for String {
    fn from(digest: IntegrityDigest) -> Self {
        digest.0
    }
}

impl fmt::Display for IntegrityDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping_prevents_a_two_field_join_from_colliding_across_different_splits() {
        let separator = '|';
        let a = format!(
            "{}{separator}{}",
            escape_identity_field("foo|1.0.0", separator),
            escape_identity_field("2.0.0", separator),
        );
        let b = format!(
            "{}{separator}{}",
            escape_identity_field("foo", separator),
            escape_identity_field("1.0.0|2.0.0", separator),
        );
        assert_ne!(
            a, b,
            "two genuinely different (name, version) pairs must never join to the same string"
        );
    }

    #[test]
    fn escaping_prevents_a_collision_via_an_embedded_backslash() {
        let separator = ':';
        // A field ending in a literal backslash, if left unescaped, would swallow the following
        // delimiter when a naive unescaper tries to walk the string looking for `\<separator>`.
        let a = format!(
            "{}{separator}{}",
            escape_identity_field("foo\\", separator),
            escape_identity_field("bar", separator),
        );
        let b = format!(
            "{}{separator}{}",
            escape_identity_field("foo", separator),
            escape_identity_field("\\bar", separator),
        );
        assert_ne!(a, b);
    }

    #[test]
    fn integrity_digest_round_trips_and_names_its_wire_algorithm() {
        let digest = IntegrityDigest::of_bytes(b"");
        assert_eq!(
            digest.as_str(),
            "blake3-256:af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
        assert_eq!(IntegrityDigest::parse(digest.as_str()), Ok(digest.clone()));
        assert_eq!(digest.wire_algorithm_id(), 1);
        assert_ne!(
            IntegrityDigest::of_bytes(b"a"),
            IntegrityDigest::of_bytes(b"b")
        );
    }

    #[test]
    fn integrity_digest_rejects_every_non_canonical_spelling() {
        let hex = "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262";
        for bad in [
            String::new(),
            hex.to_owned(),
            format!("BLAKE3-256:{hex}"),
            format!("sha256:{hex}"),
            format!("blake3-256:{}", hex.to_uppercase()),
            format!("blake3-256:{}", &hex[..63]),
            format!("blake3-256:{hex}0"),
            format!("blake3-256:{}g", &hex[..63]),
            format!("blake3-256: {}", &hex[..63]),
        ] {
            assert!(IntegrityDigest::parse(&bad).is_err(), "accepted `{bad}`");
        }
    }

    #[test]
    fn escaping_is_a_no_op_for_ordinary_fields() {
        assert_eq!(escape_identity_field("core", '|'), "core");
        assert_eq!(
            escape_identity_field("adapter/Cargo.toml", ':'),
            "adapter/Cargo.toml"
        );
    }
}
