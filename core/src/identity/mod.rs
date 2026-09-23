use serde::{Deserialize, Serialize};
use std::fmt;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntegrityDigest(pub String);

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
    fn escaping_is_a_no_op_for_ordinary_fields() {
        assert_eq!(escape_identity_field("core", '|'), "core");
        assert_eq!(
            escape_identity_field("adapter/Cargo.toml", ':'),
            "adapter/Cargo.toml"
        );
    }
}
