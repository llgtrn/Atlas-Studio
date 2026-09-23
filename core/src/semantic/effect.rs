//! Effect site identity and category.
//!
//! R4.8 (`.atlas/contracts/SEMANTIC-FACTS.md#effectfact`, `.atlas/roadmap/SELF-BUILDING-R4-R8.md`
//! R4.8): "externally observable effects; filesystem/network/process/FFI/build/runtime interactions
//! where applicable; failure effects; explicit unknown/dynamic behavior."
//!
//! Scope this wave: only panic-like macro spellings are materialized as EffectCategory::Panic
//! candidates, and those records are INFERRED rather than OBSERVED because macro/name resolution
//! is not available and Rust macro bindings may be shadowed. Filesystem/network/process/FFI/
//! persistence/event/auth/allocation/free/external-IO effects remain unmaterialized. The Rust
//! extractor therefore keeps the overall EFFECT obligation UNKNOWN until the declared profile can
//! prove closure over these cases.

use super::SemanticRecordId;
use crate::identity::RepositoryId;
use crate::language::adl::SourceSpan;
use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

/// Minimum observable effect categories; see `.atlas/contracts/SEMANTIC-FACTS.md#effectfact`.
/// Additional categories extend this enum; effect kind never collapses into a free-form string.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EffectCategory {
    FilesystemRead,
    FilesystemWrite,
    NetworkSend,
    NetworkReceive,
    ProcessSpawn,
    FfiCall,
    Persist,
    EmitEvent,
    AuthCheck,
    Alloc,
    Free,
    Panic,
    ExternalIo,
}

impl EffectCategory {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::FilesystemRead => "FILESYSTEM_READ",
            Self::FilesystemWrite => "FILESYSTEM_WRITE",
            Self::NetworkSend => "NETWORK_SEND",
            Self::NetworkReceive => "NETWORK_RECEIVE",
            Self::ProcessSpawn => "PROCESS_SPAWN",
            Self::FfiCall => "FFI_CALL",
            Self::Persist => "PERSIST",
            Self::EmitEvent => "EMIT_EVENT",
            Self::AuthCheck => "AUTH_CHECK",
            Self::Alloc => "ALLOC",
            Self::Free => "FREE",
            Self::Panic => "PANIC",
            Self::ExternalIo => "EXTERNAL_IO",
        }
    }
}

/// Identity of one effect-producing site within a function.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EffectIdentity {
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub function: SemanticRecordId,
    pub category: EffectCategory,
    pub span: SourceSpan,
}

impl EffectIdentity {
    /// Deterministic, order-independent encoding of this effect site's identity fields.
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}:{}|{}|{}|{}:{}:{}",
            self.repository.as_str(),
            self.revision.kind,
            self.revision.value,
            self.function.as_str(),
            self.category.as_str(),
            self.span.path,
            self.span.line,
            self.span.column,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::SemanticDimension;
    use super::*;

    fn base() -> EffectIdentity {
        EffectIdentity {
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "fn-key"),
            category: EffectCategory::FilesystemRead,
            span: SourceSpan {
                path: "core/src/lib.rs".into(),
                line: 10,
                column: 5,
            },
        }
    }

    #[test]
    fn identity_key_distinguishes_category_at_same_span() {
        let other = EffectIdentity {
            category: EffectCategory::FilesystemWrite,
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }
}
