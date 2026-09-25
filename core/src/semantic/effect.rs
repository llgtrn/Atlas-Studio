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
//!
//! G77: a call a name-resolution engine resolves to a standard-library path is an effect site when
//! [`std_path_effects`] declares that path's effects -- the declared std-path effect table: the
//! filesystem entry points of `std::fs` and the platform `symlink` functions, and (G91) the
//! process-environment and clock reads of `std::env` and `std::time`.

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
    /// A read of the process environment: variables, arguments, working and well-known
    /// directories (G91). An ambient input, not I/O: the same code reads different values in
    /// different processes (IRIS's `EnvGet` effect tag).
    EnvironmentRead,
    /// A read of a clock (G91): an ambient input whose value differs on every call (IRIS's
    /// `ClockNs`/`Timestamp` effect tags).
    ClockRead,
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
            Self::EnvironmentRead => "ENVIRONMENT_READ",
            Self::ClockRead => "CLOCK_READ",
        }
    }
}

/// The declared std-path effect table (G77), sorted by path: the standard-library functions whose
/// call is a filesystem effect, or (G91) a read of the process environment or of a clock, by
/// their documented contract. A path is the canonical spelling through imports
/// (`std::fs::File::open`); re-exports and method calls are not covered, and a path absent here
/// declares nothing (never "no effect") -- environment writes (`set_var`, `set_current_dir`) and
/// `current_exe` (a filesystem lookup on most platforms) among them.
const STD_PATH_EFFECTS: &[(&str, &[EffectCategory])] = {
    use EffectCategory::{
        ClockRead as C, EnvironmentRead as E, FilesystemRead as R, FilesystemWrite as W,
    };
    &[
        ("std::env::args", &[E]),
        ("std::env::args_os", &[E]),
        ("std::env::current_dir", &[E]),
        ("std::env::home_dir", &[E]),
        ("std::env::temp_dir", &[E]),
        ("std::env::var", &[E]),
        ("std::env::var_os", &[E]),
        ("std::env::vars", &[E]),
        ("std::env::vars_os", &[E]),
        ("std::fs::File::create", &[W]),
        ("std::fs::File::create_new", &[W]),
        ("std::fs::File::open", &[R]),
        ("std::fs::canonicalize", &[R]),
        ("std::fs::copy", &[R, W]),
        ("std::fs::create_dir", &[W]),
        ("std::fs::create_dir_all", &[W]),
        ("std::fs::exists", &[R]),
        ("std::fs::hard_link", &[W]),
        ("std::fs::metadata", &[R]),
        ("std::fs::read", &[R]),
        ("std::fs::read_dir", &[R]),
        ("std::fs::read_link", &[R]),
        ("std::fs::read_to_string", &[R]),
        ("std::fs::remove_dir", &[W]),
        ("std::fs::remove_dir_all", &[W]),
        ("std::fs::remove_file", &[W]),
        ("std::fs::rename", &[W]),
        ("std::fs::set_permissions", &[W]),
        ("std::fs::soft_link", &[W]),
        ("std::fs::symlink_metadata", &[R]),
        ("std::fs::write", &[W]),
        ("std::os::unix::fs::symlink", &[W]),
        ("std::os::windows::fs::symlink_dir", &[W]),
        ("std::os::windows::fs::symlink_file", &[W]),
        ("std::time::Instant::now", &[C]),
        ("std::time::SystemTime::now", &[C]),
    ]
};

/// The effects [`STD_PATH_EFFECTS`] declares for `path`; empty when the table says nothing.
pub fn std_path_effects(path: &str) -> &'static [EffectCategory] {
    STD_PATH_EFFECTS
        .binary_search_by(|(declared, _)| declared.cmp(&path))
        .map_or(&[], |index| STD_PATH_EFFECTS[index].1)
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

    #[test]
    fn the_std_path_effect_table_is_sorted_unique_and_looked_up_exactly() {
        assert!(
            STD_PATH_EFFECTS.windows(2).all(|w| w[0].0 < w[1].0),
            "binary search needs a strictly sorted table"
        );
        assert_eq!(
            std_path_effects("std::fs::write"),
            [EffectCategory::FilesystemWrite]
        );
        assert_eq!(
            std_path_effects("std::fs::copy"),
            [
                EffectCategory::FilesystemRead,
                EffectCategory::FilesystemWrite
            ]
        );
        assert_eq!(
            std_path_effects("std::fs::File::open"),
            [EffectCategory::FilesystemRead]
        );
        assert_eq!(
            std_path_effects("std::env::var"),
            [EffectCategory::EnvironmentRead]
        );
        assert_eq!(
            std_path_effects("std::env::temp_dir"),
            [EffectCategory::EnvironmentRead]
        );
        assert_eq!(
            std_path_effects("std::time::Instant::now"),
            [EffectCategory::ClockRead]
        );
        assert_eq!(
            std_path_effects("std::time::SystemTime::now"),
            [EffectCategory::ClockRead]
        );
        for undeclared in [
            "std::fs",
            "std::fs::writ",
            "std::env::set_var",
            "std::env::current_exe",
            "std::time::Instant",
            "std::time::Instant::elapsed",
            "fs::write",
            "",
        ] {
            assert!(std_path_effects(undeclared).is_empty(), "{undeclared}");
        }
    }

    #[test]
    fn the_ambient_input_categories_carry_their_contract_names() {
        assert_eq!(EffectCategory::EnvironmentRead.as_str(), "ENVIRONMENT_READ");
        assert_eq!(EffectCategory::ClockRead.as_str(), "CLOCK_READ");
    }

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
