//! Durable-state/recovery operation-site identity, and the `PlaceRef` semantic-location bridge.
//!
//! R4.11 (`.atlas/contracts/SEMANTIC-FACTS.md#persistencefact`, `.atlas/roadmap/
//! SELF-BUILDING-R4-R8.md` R4.11): persistent state; failure; recovery action; restored state;
//! durable writes; transaction boundaries; log/checkpoint/recovery behavior where applicable;
//! durability ordering; recovery/failure paths; external persistence boundaries; explicit
//! unknowns.
//!
//! # The identity-debt problem this module exists to avoid
//!
//! DATA_FLOW (`ValueIdentity`), STATE (`StateAccessIdentity`) and OWNERSHIP (`OwnershipIdentity`)
//! each already answer "what semantic location/value is this operation targeting?" with an
//! independently-shaped mechanism:
//!
//! - DATA_FLOW: a resolved `Use`/`Store` carries `resolved_definition: Option<SemanticRecordId>`
//!   -- a real typed cross-reference to the `Definition` record it resolves to.
//! - STATE: `StateAccessIdentity.name` (scoped by `scope`) names a `self.<field>` place; the
//!   engineering graph converges same-named accesses onto one content-derived `StateEntity` node.
//! - OWNERSHIP: `OwnershipIdentity.name` plus `resolution: OwnershipResolution` -- `Resolved` for a
//!   real bare-identifier place, `Unresolved` for a temporary expression's mere textual spelling
//!   (added after a real bug: two independent temporaries with identical spelling had collapsed
//!   onto one graph node -- see `.atlas/evidence/verification/
//!   r4.4-r4.10-second-hardening-pass-correction.json`).
//!
//! A fourth, independently-invented `PersistenceTarget { name: String }` would be a FIFTH
//! incompatible answer to the same question, repeating exactly the OWNERSHIP bug's root cause
//! (source spelling standing in for canonical object identity) in new code. `super::place::PlaceRef`
//! is the deliberately small bridge that lets PERSISTENCE point at an EXISTING dimension's own
//! already-canonical identity where evidence allows, or explicitly say "no existing canonical
//! record, and no fabricated one either" where it doesn't -- without requiring the larger,
//! genuinely cross-cutting migration a single shared `PlaceIdentity` type (replacing all three
//! existing shapes) would need. See the CURRENT/BRIDGE/TARGET note on `PlaceRef` itself
//! (`core::semantic::place`) -- moved out of this module once R4.12 gave it a second real
//! consumer (`core::semantic::call::CallSiteIdentity`).
//!
//! # Scope this wave
//!
//! This extractor has no dedicated Rust syntax for persistence (unlike R4.10's `.await`), no
//! resolved-API adapter, and no compiler/type resolution. Every `PersistenceKind` this wave
//! materializes is therefore a textual callee-spelling candidate ONLY -- exactly R4.8's
//! `is_panic_like_macro`/R4.10's `is_spawn_call` risk class -- and is ALWAYS recorded as
//! `EpistemicStatus::Inferred` with `PersistenceResolution::Unresolved` and `PlaceRef::Unresolved`,
//! never `Observed`. `game.commit()`, `ui.flush()`, `cache.sync()` and `builder.snapshot()` are
//! exactly as "persistence-shaped" by spelling as a real `wal.commit()` or `file.sync_all()` --
//! this extractor cannot tell them apart without resolving the receiver's type, so it never claims
//! to. `DurableRead`/`DurableWrite`/`JournalAppend`/`TransactionBegin`/`TransactionAbort`/`Recover`/
//! `Restore` are declared for a future wave (would need resolved-API evidence this extractor does
//! not have) but never emitted.

use super::SemanticRecordId;
use super::place::PlaceRef;
use crate::identity::RepositoryId;
use crate::language::adl::SourceSpan;
use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

/// Durable-state/recovery operation categories. An extensible, named vocabulary (matching
/// `EffectCategory`/`ConcurrencyKind`'s precedent) -- never a free-form string -- so a category
/// this extractor cannot yet materialize is still explicit, typed, and declared rather than
/// silently absent from the enum.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PersistenceKind {
    /// A callee spelling ending in `commit` -- a durability/transaction-commit-shaped candidate.
    Commit,
    /// A callee spelling ending in `flush` -- a buffered-write-shaped candidate.
    Flush,
    /// A callee spelling ending in `sync` or `sync_all` -- an `fsync`-shaped candidate.
    Sync,
    /// A callee spelling ending in `checkpoint` -- a checkpoint-shaped candidate.
    Checkpoint,
    /// A callee spelling ending in `snapshot` -- a snapshot-shaped candidate.
    Snapshot,
    /// Reserved; never emitted this wave (would require resolved-API evidence).
    JournalAppend,
    /// A durable read of file content: derived only from a call resolved to a std path the
    /// declared table ([`std_path_persistence`]) names (G125).
    DurableRead,
    /// A durable write of file content: derived as [`Self::DurableRead`] is (G125).
    DurableWrite,
    /// Reserved; never emitted this wave.
    TransactionBegin,
    /// Reserved; never emitted this wave.
    TransactionAbort,
    /// Reserved; never emitted this wave.
    Recover,
    /// Reserved; never emitted this wave.
    Restore,
}

impl PersistenceKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Commit => "COMMIT",
            Self::Flush => "FLUSH",
            Self::Sync => "SYNC",
            Self::Checkpoint => "CHECKPOINT",
            Self::Snapshot => "SNAPSHOT",
            Self::JournalAppend => "JOURNAL_APPEND",
            Self::DurableRead => "DURABLE_READ",
            Self::DurableWrite => "DURABLE_WRITE",
            Self::TransactionBegin => "TRANSACTION_BEGIN",
            Self::TransactionAbort => "TRANSACTION_ABORT",
            Self::Recover => "RECOVER",
            Self::Restore => "RESTORE",
        }
    }
}

/// Whether this persistence operation's evidence is a bare textual spelling candidate or is
/// backed by stronger (resolved-API/compiler/admitted-adapter) evidence. Mirrors R4.8's
/// `StateResolution`/R4.9's `OwnershipResolution` precedent exactly.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PersistenceResolution {
    /// Backed by evidence stronger than spelling alone (a future resolved/admitted API adapter).
    Resolved,
    /// A bare textual spelling candidate only -- this extractor's only mode this wave.
    Unresolved,
}

/// The declared std-path persistence table (G125, NA-PERSISTENCE-RESOLVED), sorted by path: the
/// standard-library functions whose call reads or writes durable file content, or forces it to
/// stable storage, by their documented contract. A path is the canonical spelling through imports
/// (`std::fs::write`); a method is its `<std type>::<method>` path, derived only where the
/// receiver's std type is known (`file.sync_all()`, G144; `writer.flush()` is a trait method and
/// is not covered); durable-state mutations without a fitting kind (`rename`, `remove_file`,
/// `create_dir`) are not declared; a path absent here declares nothing (never "not persistent").
const STD_PATH_PERSISTENCE: &[(&str, PersistenceKind)] = &[
    ("std::fs::File::sync_all", PersistenceKind::Sync),
    ("std::fs::File::sync_data", PersistenceKind::Sync),
    ("std::fs::copy", PersistenceKind::DurableWrite),
    ("std::fs::read", PersistenceKind::DurableRead),
    ("std::fs::read_to_string", PersistenceKind::DurableRead),
    ("std::fs::write", PersistenceKind::DurableWrite),
];

/// The persistence operation [`STD_PATH_PERSISTENCE`] declares for `path`, if any.
pub fn std_path_persistence(path: &str) -> Option<PersistenceKind> {
    STD_PATH_PERSISTENCE
        .binary_search_by(|(declared, _)| declared.cmp(&path))
        .ok()
        .map(|index| STD_PATH_PERSISTENCE[index].1)
}

impl PersistenceResolution {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Resolved => "RESOLVED",
            Self::Unresolved => "UNRESOLVED",
        }
    }
}

/// Identity of one durable-state/recovery operation site: `kind` performed by the function at
/// `span`, referencing `place` (a `super::place::PlaceRef`) where evidence allows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistenceIdentity {
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub function: SemanticRecordId,
    pub kind: PersistenceKind,
    pub span: SourceSpan,
    pub place: PlaceRef,
    pub resolution: PersistenceResolution,
}

impl PersistenceIdentity {
    /// Deterministic, order-independent encoding of this persistence operation's identity fields.
    /// `place`/`resolution` are deliberately excluded -- facts about an already-identified
    /// operation, not part of what makes the operation itself distinct (see the module doc
    /// comment and `PlaceRef`'s own doc comment).
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}:{}|{}|{}:{}:{}|{}",
            self.repository.as_str(),
            self.revision.kind,
            self.revision.value,
            self.function.as_str(),
            self.span.path,
            self.span.line,
            self.span.column,
            self.kind.as_str(),
        )
    }
}

#[cfg(test)]
mod std_path_table_tests {
    use super::*;

    #[test]
    fn the_declared_persistence_table_is_sorted_and_exact() {
        assert!(STD_PATH_PERSISTENCE.windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!(
            std_path_persistence("std::fs::write"),
            Some(PersistenceKind::DurableWrite)
        );
        assert_eq!(
            std_path_persistence("std::fs::read_to_string"),
            Some(PersistenceKind::DurableRead)
        );
        assert_eq!(
            std_path_persistence("std::fs::File::sync_all"),
            Some(PersistenceKind::Sync)
        );
        assert_eq!(
            std_path_persistence("std::fs::rename"),
            None,
            "no fitting kind is declared"
        );
        assert_eq!(
            std_path_persistence("std::fs::writ"),
            None,
            "exact paths only"
        );
        assert_eq!(std_path_persistence("crate::fs::write"), None);
    }
}

#[cfg(test)]
mod tests {
    use super::super::SemanticDimension;
    use super::*;

    fn base() -> PersistenceIdentity {
        PersistenceIdentity {
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "fn-key"),
            kind: PersistenceKind::Commit,
            span: SourceSpan {
                path: "src/lib.rs".into(),
                line: 10,
                column: 5,
            },
            place: PlaceRef::Unresolved,
            resolution: PersistenceResolution::Unresolved,
        }
    }

    #[test]
    fn identity_key_distinguishes_owning_function() {
        let other = PersistenceIdentity {
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "other-fn-key"),
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn identity_key_distinguishes_span() {
        let other = PersistenceIdentity {
            span: SourceSpan {
                line: 99,
                ..base().span
            },
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn identity_key_distinguishes_kind() {
        let other = PersistenceIdentity {
            kind: PersistenceKind::Flush,
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn identity_key_is_unaffected_by_place_or_resolution() {
        let other = PersistenceIdentity {
            place: PlaceRef::Resolved {
                dimension: SemanticDimension::State,
                record_id: SemanticRecordId::new(SemanticDimension::State, "some-key"),
            },
            resolution: PersistenceResolution::Resolved,
            ..base()
        };
        assert_eq!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn two_unresolved_operations_at_different_sites_have_distinct_identity_despite_identical_place()
    {
        // Both carry PlaceRef::Unresolved (no site-specific content inside the variant at all) --
        // distinctness must come from PersistenceIdentity's own span, not from PlaceRef.
        let a = base();
        let b = PersistenceIdentity {
            span: SourceSpan {
                line: 11,
                ..base().span
            },
            ..base()
        };
        assert_ne!(a.identity_key(), b.identity_key());
        assert_eq!(a.place, PlaceRef::Unresolved);
        assert_eq!(b.place, PlaceRef::Unresolved);
    }

    #[test]
    fn kind_as_str_matches_the_screaming_snake_vocabulary() {
        assert_eq!(PersistenceKind::Commit.as_str(), "COMMIT");
        assert_eq!(PersistenceKind::Flush.as_str(), "FLUSH");
        assert_eq!(PersistenceKind::Sync.as_str(), "SYNC");
        assert_eq!(PersistenceKind::Checkpoint.as_str(), "CHECKPOINT");
        assert_eq!(PersistenceKind::Snapshot.as_str(), "SNAPSHOT");
        assert_eq!(PersistenceKind::JournalAppend.as_str(), "JOURNAL_APPEND");
        assert_eq!(PersistenceKind::DurableRead.as_str(), "DURABLE_READ");
        assert_eq!(PersistenceKind::DurableWrite.as_str(), "DURABLE_WRITE");
        assert_eq!(
            PersistenceKind::TransactionBegin.as_str(),
            "TRANSACTION_BEGIN"
        );
        assert_eq!(
            PersistenceKind::TransactionAbort.as_str(),
            "TRANSACTION_ABORT"
        );
        assert_eq!(PersistenceKind::Recover.as_str(), "RECOVER");
        assert_eq!(PersistenceKind::Restore.as_str(), "RESTORE");
    }

    #[test]
    fn resolution_as_str_matches_the_screaming_snake_vocabulary() {
        assert_eq!(PersistenceResolution::Resolved.as_str(), "RESOLVED");
        assert_eq!(PersistenceResolution::Unresolved.as_str(), "UNRESOLVED");
    }
}
