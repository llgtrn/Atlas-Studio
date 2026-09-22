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
