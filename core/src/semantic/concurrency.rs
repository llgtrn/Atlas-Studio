//! Concurrency operation-site identity.
//!
//! R4.10 (`.atlas/contracts/SEMANTIC-FACTS.md#concurrencyfact`, `.atlas/roadmap/SELF-BUILDING-R4-R8.md`
//! R4.10): "threads/tasks; channels; locks/unlocks; atomics; synchronization; ordering
//! relationships; concurrent state interaction; dynamic/unresolved concurrency obligations."
//!
//! Scope this wave (see `adapter::semantic::rust::concurrency` for the extractor): only two of the
//! eight `ConcurrencyKind` variants are emitted, following the exact epistemic split R4.5's CALL,
//! R4.8's EFFECT and R4.9's OWNERSHIP already established for this extractor:
//!
//! - `Await` -- Rust's `.await` postfix is dedicated expression syntax (`syn::Expr::Await`), not a
//!   method call. It cannot be shadowed, aliased or overloaded the way an identifier or method name
//!   can, so every occurrence is a real, unambiguous concurrency-relevant suspension point. This is
//!   even stronger evidence than R4.9's `&`/`&mut` borrow-syntax precedent.
//! - `Spawn` -- detected purely by a call expression's callee spelling ending in the segment
//!   `spawn` (`thread::spawn`, `tokio::spawn`, `std::thread::spawn`, ...), exactly the same
//!   risk/precision class R4.8's `is_panic_like_macro` already accepts for macro names: a local
//!   identifier or re-exported function that happens to be named/aliased `spawn` would be a false
//!   positive (`fn spawn() { .. }`, `game::spawn(enemy)`), and this extractor has no `use`-import or
//!   type resolution to do better. This is why a spelling match is recorded as
//!   `EpistemicStatus::Inferred`, never `Observed` -- only a resolved/admitted concurrency API would
//!   justify `Observed`. Recording it as `Inferred` rather than dropping it preserves real evidence
//!   without overclaiming certainty.
//!
//! `Unlock`/`AtomicOp` are declared for a future wave and never emitted; `ChannelCreate` is derived
//! from resolved path calls (G117) and `Lock`/`ChannelSend`/`ChannelReceive` from method calls whose
//! receiver's std type is known (G144). None is ever claimed from a spelling: method names like `.lock()`/`.send()`/
//! `.recv()` collide constantly with unrelated user-defined methods of the same name (a `Config`
//! struct's own `.lock()`, a game's own `.send()`), and claiming them from a bare method name alone
//! -- without resolving the receiver's type -- would fabricate compiler-resolved semantics exactly
//! the way R4.8's EFFECT module explicitly declined to do for `FilesystemWrite`/`NetworkSend`/etc.
//!
//! G117 (P0, the native attack queue head of ADR 0038): the census recorded zero concurrency
//! although Atlas runs scoped worker threads. The path-resolution engine now derives concurrency
//! sites for path calls resolved to a declared standard-library path ([`std_path_concurrency`]):
//! `std::thread::scope` (`ThreadScope`), `std::thread::spawn` (`Spawn`) and the `mpsc` channel
//! constructors (`ChannelCreate`). These are DERIVED: name resolution fixes the callee, the std
//! contract fixes its meaning.

use super::SemanticRecordId;
use crate::identity::RepositoryId;
use crate::language::adl::SourceSpan;
use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

/// The concurrency operation this record represents. Part of `identity_key()`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConcurrencyKind {
    /// `expr.await` -- fully syntax-determined; cannot be shadowed or overloaded.
    Await,
    /// A call whose callee spelling ends in the segment `spawn` (e.g. `thread::spawn`,
    /// `tokio::spawn`). Spelling-based, not type/import resolved.
    Spawn,
    /// `Mutex::lock` on a receiver whose std type is known (G144, [`std_path_concurrency`]).
    Lock,
    /// Reserved; never emitted this wave.
    Unlock,
    /// `std::sync::mpsc::channel` / `sync_channel` resolved as a path call (G117).
    ChannelCreate,
    /// `Sender::send` on a receiver whose std type is known (G144).
    ChannelSend,
    /// `Receiver::recv` on a receiver whose std type is known (G144).
    ChannelReceive,
    /// Reserved; never emitted this wave.
    AtomicOp,
    /// A call that opens a structured thread scope (`std::thread::scope`, G117): every thread
    /// spawned inside the scope is joined before the call returns. Only derived from a call
    /// resolved to a declared std path ([`std_path_concurrency`]).
    ThreadScope,
}

impl ConcurrencyKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Await => "AWAIT",
            Self::Spawn => "SPAWN",
            Self::Lock => "LOCK",
            Self::Unlock => "UNLOCK",
            Self::ChannelCreate => "CHANNEL_CREATE",
            Self::ChannelSend => "CHANNEL_SEND",
            Self::ChannelReceive => "CHANNEL_RECEIVE",
            Self::AtomicOp => "ATOMIC_OP",
            Self::ThreadScope => "THREAD_SCOPE",
        }
    }
}

/// The declared std-path concurrency table (G117), sorted by path: the standard-library functions
/// whose call is a concurrency operation by their documented contract. A path is the canonical
/// spelling through imports (`std::thread::spawn`); a method (`Mutex::lock`, `Sender::send`,
/// `Receiver::recv`) is its `<std type>::<method>` path, derived only where the receiver's std
/// type is known (G144); a path absent here declares nothing (never "no concurrency").
const STD_PATH_CONCURRENCY: &[(&str, ConcurrencyKind)] = &[
    ("std::sync::Mutex::lock", ConcurrencyKind::Lock),
    (
        "std::sync::mpsc::Receiver::recv",
        ConcurrencyKind::ChannelReceive,
    ),
    (
        "std::sync::mpsc::Sender::send",
        ConcurrencyKind::ChannelSend,
    ),
    ("std::sync::mpsc::channel", ConcurrencyKind::ChannelCreate),
    (
        "std::sync::mpsc::sync_channel",
        ConcurrencyKind::ChannelCreate,
    ),
    ("std::thread::scope", ConcurrencyKind::ThreadScope),
    ("std::thread::spawn", ConcurrencyKind::Spawn),
];

/// The concurrency operation [`STD_PATH_CONCURRENCY`] declares for `path`, if any.
pub fn std_path_concurrency(path: &str) -> Option<ConcurrencyKind> {
    STD_PATH_CONCURRENCY
        .binary_search_by(|(declared, _)| declared.cmp(&path))
        .ok()
        .map(|index| STD_PATH_CONCURRENCY[index].1)
}

/// Identity of one concurrency-relevant operation site within a function.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConcurrencyIdentity {
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub function: SemanticRecordId,
    pub kind: ConcurrencyKind,
    pub span: SourceSpan,
}

impl ConcurrencyIdentity {
    /// Deterministic, order-independent encoding of this concurrency operation's identity fields.
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}:{}|{}|{}|{}:{}:{}",
            self.repository.as_str(),
            self.revision.kind,
            self.revision.value,
            self.function.as_str(),
            self.kind.as_str(),
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

    fn base() -> ConcurrencyIdentity {
        ConcurrencyIdentity {
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "fn-key"),
            kind: ConcurrencyKind::Await,
            span: SourceSpan {
                path: "core/src/lib.rs".into(),
                line: 10,
                column: 5,
            },
        }
    }

    #[test]
    fn identity_key_distinguishes_kind_at_same_span() {
        let other = ConcurrencyIdentity {
            kind: ConcurrencyKind::Spawn,
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn identity_key_distinguishes_owning_function() {
        let other = ConcurrencyIdentity {
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "other-fn-key"),
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn identity_key_distinguishes_span() {
        let other = ConcurrencyIdentity {
            span: SourceSpan {
                path: "core/src/lib.rs".into(),
                line: 99,
                column: 5,
            },
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn kind_as_str_matches_the_screaming_snake_vocabulary() {
        assert_eq!(ConcurrencyKind::Await.as_str(), "AWAIT");
        assert_eq!(ConcurrencyKind::Spawn.as_str(), "SPAWN");
        assert_eq!(ConcurrencyKind::Lock.as_str(), "LOCK");
        assert_eq!(ConcurrencyKind::Unlock.as_str(), "UNLOCK");
        assert_eq!(ConcurrencyKind::ChannelCreate.as_str(), "CHANNEL_CREATE");
        assert_eq!(ConcurrencyKind::ChannelSend.as_str(), "CHANNEL_SEND");
        assert_eq!(ConcurrencyKind::ChannelReceive.as_str(), "CHANNEL_RECEIVE");
        assert_eq!(ConcurrencyKind::AtomicOp.as_str(), "ATOMIC_OP");
        assert_eq!(ConcurrencyKind::ThreadScope.as_str(), "THREAD_SCOPE");
    }

    #[test]
    fn the_std_path_concurrency_table_is_sorted_unique_and_looked_up_exactly() {
        assert!(
            STD_PATH_CONCURRENCY.windows(2).all(|w| w[0].0 < w[1].0),
            "binary search needs a strictly sorted table"
        );
        assert_eq!(
            std_path_concurrency("std::thread::scope"),
            Some(ConcurrencyKind::ThreadScope)
        );
        assert_eq!(
            std_path_concurrency("std::thread::spawn"),
            Some(ConcurrencyKind::Spawn)
        );
        assert_eq!(
            std_path_concurrency("std::sync::mpsc::channel"),
            Some(ConcurrencyKind::ChannelCreate)
        );
        assert_eq!(
            std_path_concurrency("std::sync::mpsc::sync_channel"),
            Some(ConcurrencyKind::ChannelCreate)
        );
        for undeclared in [
            "std::thread",
            "std::thread::sleep",
            "std::thread::current",
            "std::thread::Builder::new",
            "std::sync::Mutex::new",
            "thread::spawn",
            "spawn",
            "",
        ] {
            assert_eq!(std_path_concurrency(undeclared), None, "{undeclared}");
        }
    }
}
