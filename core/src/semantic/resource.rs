//! Resource acquisition and release identity (G157, ADR 0072, DEBT-RESOURCE).
//!
//! OWNERSHIP records moves and borrows; PERSISTENCE records durable identity; neither says when a
//! handle, a lock or a thread is acquired and when it is given back. RESOURCE does, in a bounded
//! first profile:
//!
//! - **Acquisition** is DERIVED from a call the path resolver resolves to a standard-library path
//!   the declared table below names ([`std_path_resource`]): opening or creating a file, connecting
//!   or binding a socket, locking a `Mutex` or `RwLock` whose receiver's std type is known, and
//!   spawning a thread. Name resolution fixes the callee, the std contract fixes the resource.
//! - **Release** is claimed only for an acquisition bound once by a `let` (through `?`,
//!   `.unwrap()` or `.expect(..)`) to a local that is never moved:
//!   - `SCOPE_END`: the local is dropped at the end of the block it lives in, which closes the
//!     file or socket and unlocks the guard. INFERRED: the move check is syntactic, not borrowck.
//!     A thread is never released at scope end -- dropping a `JoinHandle` detaches the thread.
//!   - `EXPLICIT_DROP`: `drop(local)` resolved to `std::mem::drop`. DERIVED.
//!   - `JOIN`: `local.join()` resolved to `std::thread::JoinHandle::join`, which consumes the
//!     handle and waits for the thread. DERIVED.
//!
//! Every other acquisition -- a temporary (`m.lock().unwrap().push(x)`), a local passed or
//! returned by value, a local named inside a macro that is not a formatting macro -- has no
//! release claimed: its release point is not known, never "none".

use super::SemanticRecordId;
use crate::identity::RepositoryId;
use crate::language::adl::SourceSpan;
use crate::temporal::RevisionRef;
use crate::vocabulary_enum;
use serde::{Deserialize, Serialize};

vocabulary_enum! {
    /// What is acquired. Part of `identity_key()`.
    pub enum ResourceKind {
        /// An open file handle (`std::fs::File`).
        File => "FILE",
        /// A connected or bound socket (`TcpStream`, `TcpListener`, `UdpSocket`, Unix sockets).
        Socket => "SOCKET",
        /// A lock guard (`MutexGuard`, `RwLockReadGuard`, `RwLockWriteGuard`).
        LockGuard => "LOCK_GUARD",
        /// A spawned thread, held through its `JoinHandle`.
        Thread => "THREAD",
    }
}

impl ResourceKind {
    /// Whether dropping the holder releases the resource: a file closes, a socket closes, a
    /// guard unlocks; a dropped `JoinHandle` detaches its thread, which keeps running.
    pub const fn released_by_drop(self) -> bool {
        !matches!(self, Self::Thread)
    }
}

vocabulary_enum! {
    /// Whether a record acquires or releases. Part of `identity_key()`.
    pub enum ResourceOperation {
        Acquire => "ACQUIRE",
        Release => "RELEASE",
    }
}

vocabulary_enum! {
    /// How a release happens. Part of `identity_key()` for a release.
    pub enum ResourceRelease {
        /// The holder is dropped at the end of its block (INFERRED).
        ScopeEnd => "SCOPE_END",
        /// `drop(holder)` resolved to `std::mem::drop` (DERIVED).
        ExplicitDrop => "EXPLICIT_DROP",
        /// `holder.join()` resolved to `std::thread::JoinHandle::join` (DERIVED).
        Join => "JOIN",
    }
}

/// The declared std-path resource table, sorted by path: the standard-library functions whose
/// call acquires a resource by their documented contract. A method (`Mutex::lock`) is its
/// `<std type>::<method>` path, derived only where the receiver's std type is known (G144); a
/// path absent here declares nothing (never "no resource").
const STD_PATH_RESOURCE: &[(&str, ResourceKind)] = &[
    ("std::fs::File::create", ResourceKind::File),
    ("std::fs::File::create_new", ResourceKind::File),
    ("std::fs::File::open", ResourceKind::File),
    ("std::net::TcpListener::bind", ResourceKind::Socket),
    ("std::net::TcpStream::connect", ResourceKind::Socket),
    ("std::net::UdpSocket::bind", ResourceKind::Socket),
    (
        "std::os::unix::net::UnixListener::bind",
        ResourceKind::Socket,
    ),
    (
        "std::os::unix::net::UnixStream::connect",
        ResourceKind::Socket,
    ),
    ("std::sync::Mutex::lock", ResourceKind::LockGuard),
    ("std::sync::RwLock::read", ResourceKind::LockGuard),
    ("std::sync::RwLock::write", ResourceKind::LockGuard),
    ("std::thread::spawn", ResourceKind::Thread),
];

/// The resource [`STD_PATH_RESOURCE`] declares `path` acquires, if any.
pub fn std_path_resource(path: &str) -> Option<ResourceKind> {
    STD_PATH_RESOURCE
        .binary_search_by(|(declared, _)| declared.cmp(&path))
        .ok()
        .map(|index| STD_PATH_RESOURCE[index].1)
}

/// Identity of one acquisition or release site within a function.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceIdentity {
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub function: SemanticRecordId,
    pub operation: ResourceOperation,
    pub kind: ResourceKind,
    /// The acquiring call, or the release point.
    pub span: SourceSpan,
    /// For a release: the acquisition it gives back (its call's span) and how.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acquired_at: Option<SourceSpan>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<ResourceRelease>,
    /// For a release: the local holding the resource.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub holder: String,
}

impl ResourceIdentity {
    /// Deterministic, order-independent encoding of this site's identity fields.
    pub fn identity_key(&self) -> String {
        let acquired = self.acquired_at.as_ref().map_or(String::new(), |span| {
            format!("{}:{}:{}", span.path, span.line, span.column)
        });
        format!(
            "{}|{}:{}|{}|{}|{}|{}:{}:{}|{}|{}|{}",
            self.repository.as_str(),
            self.revision.kind,
            self.revision.value,
            self.function.as_str(),
            self.operation.as_str(),
            self.kind.as_str(),
            self.span.path,
            self.span.line,
            self.span.column,
            acquired,
            self.release.map_or("", |r| r.as_str()),
            self.holder,
        )
    }

    /// A release names its acquisition and how it happens; an acquisition names neither.
    pub fn is_well_formed(&self) -> bool {
        match self.operation {
            ResourceOperation::Acquire => {
                self.acquired_at.is_none() && self.release.is_none() && self.holder.is_empty()
            }
            ResourceOperation::Release => {
                self.acquired_at.is_some()
                    && !self.holder.is_empty()
                    && match self.release {
                        Some(ResourceRelease::ScopeEnd | ResourceRelease::ExplicitDrop) => {
                            self.kind.released_by_drop()
                        }
                        Some(ResourceRelease::Join) => self.kind == ResourceKind::Thread,
                        None => false,
                    }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::SemanticDimension;

    fn span(line: usize) -> SourceSpan {
        SourceSpan {
            path: "src/lib.rs".into(),
            line,
            column: 5,
        }
    }

    fn site(operation: ResourceOperation, kind: ResourceKind) -> ResourceIdentity {
        ResourceIdentity {
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc".into(),
            },
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "f"),
            operation,
            kind,
            span: span(3),
            acquired_at: None,
            release: None,
            holder: String::new(),
        }
    }

    #[test]
    fn the_declared_table_is_sorted_and_names_only_std_paths() {
        assert!(STD_PATH_RESOURCE.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(
            STD_PATH_RESOURCE
                .iter()
                .all(|(p, _)| p.starts_with("std::"))
        );
        assert_eq!(
            std_path_resource("std::fs::File::open"),
            Some(ResourceKind::File)
        );
        assert_eq!(
            std_path_resource("std::sync::Mutex::lock"),
            Some(ResourceKind::LockGuard)
        );
        assert_eq!(
            std_path_resource("std::thread::spawn"),
            Some(ResourceKind::Thread)
        );
        // Absent is "not declared", never "no resource"; a spelling is never enough.
        assert_eq!(std_path_resource("std::fs::read_to_string"), None);
        assert_eq!(std_path_resource("File::open"), None);
        assert_eq!(std_path_resource("tokio::fs::File::open"), None);
    }

    #[test]
    fn a_release_names_its_acquisition_and_a_way_its_kind_allows() {
        let acquire = site(ResourceOperation::Acquire, ResourceKind::File);
        assert!(acquire.is_well_formed());
        let release = |kind, how| ResourceIdentity {
            acquired_at: Some(span(3)),
            release: Some(how),
            holder: "h".into(),
            span: span(9),
            ..site(ResourceOperation::Release, kind)
        };
        for kind in [
            ResourceKind::File,
            ResourceKind::Socket,
            ResourceKind::LockGuard,
        ] {
            assert!(release(kind, ResourceRelease::ScopeEnd).is_well_formed());
            assert!(release(kind, ResourceRelease::ExplicitDrop).is_well_formed());
            assert!(!release(kind, ResourceRelease::Join).is_well_formed());
        }
        // A dropped JoinHandle detaches its thread: only a join releases it.
        assert!(release(ResourceKind::Thread, ResourceRelease::Join).is_well_formed());
        assert!(!release(ResourceKind::Thread, ResourceRelease::ScopeEnd).is_well_formed());
        assert!(!release(ResourceKind::Thread, ResourceRelease::ExplicitDrop).is_well_formed());
        // An acquisition carries no release fields; a release carries all of them.
        let mut bad = acquire.clone();
        bad.release = Some(ResourceRelease::ScopeEnd);
        assert!(!bad.is_well_formed());
        let mut orphan = release(ResourceKind::File, ResourceRelease::ScopeEnd);
        orphan.acquired_at = None;
        assert!(!orphan.is_well_formed());
        let mut anonymous = release(ResourceKind::File, ResourceRelease::ScopeEnd);
        anonymous.holder.clear();
        assert!(!anonymous.is_well_formed());
    }

    #[test]
    fn identity_distinguishes_operation_release_and_acquisition() {
        let acquire = site(ResourceOperation::Acquire, ResourceKind::File);
        let release = ResourceIdentity {
            acquired_at: Some(span(3)),
            release: Some(ResourceRelease::ScopeEnd),
            holder: "h".into(),
            ..site(ResourceOperation::Release, ResourceKind::File)
        };
        let dropped = ResourceIdentity {
            release: Some(ResourceRelease::ExplicitDrop),
            ..release.clone()
        };
        let keys = [
            acquire.identity_key(),
            release.identity_key(),
            dropped.identity_key(),
        ];
        assert_ne!(keys[0], keys[1]);
        assert_ne!(keys[1], keys[2]);
        assert_eq!(acquire.identity_key(), acquire.clone().identity_key());
    }
}
