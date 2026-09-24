//! Inventory change detection: the difference between two complete `InventoryReport` snapshots of
//! the same root (ADR 0006).
//!
//! The change taxonomy and decision rule are absorbed from rust-analyzer's `crates/vfs`
//! (`rust-lang/rust-analyzer`, `crates/vfs/src/lib.rs`: `Change::{Create, Modify, Delete}`, and
//! `set_file_contents`'s state/contents decision with its equal-hash short circuit), adapted to two
//! snapshots instead of an in-memory event feed:
//!
//! - a path only in the current snapshot is `Created`, a path only in the previous one `Deleted`
//!   (vfs: an unseen path defaults to `Deleted`, so its first contents are a `Create`);
//! - a path in both is `Modified` unless both sides carry the same kind and the same content
//!   digest (vfs: equal hash means no change).
//!
//! Deliberately different from vfs: identity is the artifact path (`ArtifactId =
//! stable_id(path)`), not a process-local interned `FileId`; content identity is BLAKE3-256, not a
//! 64-bit `FxHasher` whose collision would silently drop a modification; there is no within-cycle
//! merge table (a snapshot diff has no intermediate events, so create-then-delete is simply
//! absent and A->B->A is unchanged); and a missing digest on either side is `Modified`, never
//! unchanged (`ArtifactRecord.content_digest`: `None` means "treat as changed").

use super::{ArtifactKind, ArtifactRecord, InventoryReport};
use crate::ArtifactId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArtifactChangeKind {
    Created,
    Modified,
    Deleted,
}

/// Why a path was reported as changed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArtifactChangeCause {
    /// Only the current snapshot has this path.
    Appeared,
    /// Only the previous snapshot has this path.
    Disappeared,
    /// Both sides carry a content digest and they differ.
    ContentDigestDiffers,
    /// At least one side carries no content digest (withheld, or an artifact kind with no content
    /// identity), so the content cannot be vouched for as unchanged.
    ContentIdentityUnavailable,
    /// The artifact kind differs (e.g. a regular file replaced by a symlink).
    KindChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactChange {
    pub path: String,
    pub artifact: ArtifactId,
    pub change: ArtifactChangeKind,
    pub cause: ArtifactChangeCause,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryDelta {
    pub schema: String,
    pub root: String,
    /// Every changed path, sorted by path.
    pub changes: Vec<ArtifactChange>,
    pub created: usize,
    pub modified: usize,
    pub deleted: usize,
    pub unchanged: usize,
    /// Paths whose content is unchanged but whose recorded classification (`bytes`,
    /// `disposition`, `language`, `reason`) differs -- e.g. a newly registered source frontend.
    /// Not a content change; a consumer caching a derivation must still key on the classifier.
    pub classification_drift: Vec<String>,
}

impl InventoryDelta {
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

/// Why two reports cannot be diffed. Never papered over: a wrong baseline must not yield a
/// plausible-looking delta.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "refusal")]
pub enum InventoryDiffRefusal {
    RootDiffers { previous: String, current: String },
    SchemaDiffers { previous: String, current: String },
    DuplicatePath { snapshot: String, path: String },
}

impl std::fmt::Display for InventoryDiffRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RootDiffers { previous, current } => write!(
                f,
                "inventory roots differ (previous `{previous}`, current `{current}`)"
            ),
            Self::SchemaDiffers { previous, current } => write!(
                f,
                "inventory schemas differ (previous `{previous}`, current `{current}`): re-baseline"
            ),
            Self::DuplicatePath { snapshot, path } => {
                write!(f, "{snapshot} inventory lists `{path}` more than once")
            }
        }
    }
}

fn index<'a>(
    report: &'a InventoryReport,
    snapshot: &str,
) -> Result<BTreeMap<&'a str, &'a ArtifactRecord>, InventoryDiffRefusal> {
    let mut by_path = BTreeMap::new();
    for artifact in &report.artifacts {
        if by_path.insert(artifact.path.as_str(), artifact).is_some() {
            return Err(InventoryDiffRefusal::DuplicatePath {
                snapshot: snapshot.to_owned(),
                path: artifact.path.clone(),
            });
        }
    }
    Ok(by_path)
}

/// The change, if any, between one path's previous and current records.
fn compare(previous: &ArtifactRecord, current: &ArtifactRecord) -> Option<ArtifactChangeCause> {
    if previous.kind != current.kind {
        return Some(ArtifactChangeCause::KindChanged);
    }
    if previous.kind == ArtifactKind::PolicyBoundary {
        // An explicitly ignored directory (`target`, `.git`, ...): its contents are outside the
        // census by policy on both sides, so its presence is the whole observation.
        return None;
    }
    match (&previous.content_digest, &current.content_digest) {
        (Some(before), Some(after)) if before == after => None,
        (Some(_), Some(_)) => Some(ArtifactChangeCause::ContentDigestDiffers),
        _ => Some(ArtifactChangeCause::ContentIdentityUnavailable),
    }
}

fn classification_differs(previous: &ArtifactRecord, current: &ArtifactRecord) -> bool {
    previous.bytes != current.bytes
        || previous.disposition != current.disposition
        || previous.language != current.language
        || previous.reason != current.reason
}

pub fn diff_inventories(
    previous: &InventoryReport,
    current: &InventoryReport,
) -> Result<InventoryDelta, InventoryDiffRefusal> {
    if previous.root != current.root {
        return Err(InventoryDiffRefusal::RootDiffers {
            previous: previous.root.clone(),
            current: current.root.clone(),
        });
    }
    if previous.schema != current.schema {
        return Err(InventoryDiffRefusal::SchemaDiffers {
            previous: previous.schema.clone(),
            current: current.schema.clone(),
        });
    }
    let before = index(previous, "previous")?;
    let after = index(current, "current")?;

    let mut changes = Vec::new();
    let mut unchanged = 0;
    let mut classification_drift = Vec::new();
    for (path, record) in &after {
        let (change, cause) = match before.get(path) {
            None => (ArtifactChangeKind::Created, ArtifactChangeCause::Appeared),
            Some(old) => match compare(old, record) {
                Some(cause) => (ArtifactChangeKind::Modified, cause),
                None => {
                    unchanged += 1;
                    if classification_differs(old, record) {
                        classification_drift.push((*path).to_owned());
                    }
                    continue;
                }
            },
        };
        changes.push(ArtifactChange {
            path: (*path).to_owned(),
            artifact: record.id.clone(),
            change,
            cause,
        });
    }
    for (path, record) in &before {
        if !after.contains_key(path) {
            changes.push(ArtifactChange {
                path: (*path).to_owned(),
                artifact: record.id.clone(),
                change: ArtifactChangeKind::Deleted,
                cause: ArtifactChangeCause::Disappeared,
            });
        }
    }
    changes.sort_by(|a, b| a.path.cmp(&b.path));

    let count = |kind| changes.iter().filter(|c| c.change == kind).count();
    Ok(InventoryDelta {
        schema: "atlas.inventory-delta.v1".into(),
        root: current.root.clone(),
        created: count(ArtifactChangeKind::Created),
        modified: count(ArtifactChangeKind::Modified),
        deleted: count(ArtifactChangeKind::Deleted),
        unchanged,
        classification_drift,
        changes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArtifactDisposition, IntegrityDigest};

    fn file(path: &str, content: &[u8]) -> ArtifactRecord {
        ArtifactRecord {
            id: ArtifactId::new(format!("artifact:{path}")),
            path: path.into(),
            kind: ArtifactKind::File,
            bytes: content.len() as u64,
            disposition: ArtifactDisposition::Parsed,
            language: Some("rust".into()),
            reason: None,
            content_digest: Some(IntegrityDigest::of_bytes(content)),
            content_digest_withheld: None,
        }
    }

    fn report(artifacts: Vec<ArtifactRecord>) -> InventoryReport {
        InventoryReport::new("/repo", artifacts)
    }

    fn change(
        delta: &InventoryDelta,
        path: &str,
    ) -> Option<(ArtifactChangeKind, ArtifactChangeCause)> {
        delta
            .changes
            .iter()
            .find(|c| c.path == path)
            .map(|c| (c.change, c.cause))
    }

    #[test]
    fn identical_snapshots_have_an_empty_delta() {
        let a = report(vec![file("a.rs", b"a"), file("b.rs", b"b")]);
        let delta = diff_inventories(&a, &a.clone()).unwrap();
        assert!(delta.is_empty());
        assert_eq!(delta.unchanged, 2);
    }

    #[test]
    fn created_modified_deleted_and_unchanged_are_each_classified() {
        let before = report(vec![
            file("keep.rs", b"k"),
            file("edit.rs", b"1"),
            file("gone.rs", b"g"),
        ]);
        let after = report(vec![
            file("keep.rs", b"k"),
            file("edit.rs", b"2"),
            file("new.rs", b"n"),
        ]);
        let delta = diff_inventories(&before, &after).unwrap();
        use ArtifactChangeCause::*;
        use ArtifactChangeKind::*;
        assert_eq!(change(&delta, "new.rs"), Some((Created, Appeared)));
        assert_eq!(
            change(&delta, "edit.rs"),
            Some((Modified, ContentDigestDiffers))
        );
        assert_eq!(change(&delta, "gone.rs"), Some((Deleted, Disappeared)));
        assert_eq!(change(&delta, "keep.rs"), None);
        assert_eq!(
            (
                delta.created,
                delta.modified,
                delta.deleted,
                delta.unchanged
            ),
            (1, 1, 1, 1)
        );
        let paths: Vec<_> = delta.changes.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(paths, ["edit.rs", "gone.rs", "new.rs"], "sorted by path");
    }

    #[test]
    fn a_missing_digest_on_either_side_is_never_unchanged() {
        let mut withheld = file("x.bin", b"same");
        withheld.content_digest = None;
        withheld.content_digest_withheld = Some("content-digest-withheld: size-limit".into());
        let digested = file("x.bin", b"same");
        for (before, after) in [
            (withheld.clone(), digested.clone()),
            (digested.clone(), withheld.clone()),
            (withheld.clone(), withheld.clone()),
        ] {
            let delta = diff_inventories(&report(vec![before]), &report(vec![after])).unwrap();
            assert_eq!(
                change(&delta, "x.bin"),
                Some((
                    ArtifactChangeKind::Modified,
                    ArtifactChangeCause::ContentIdentityUnavailable
                ))
            );
        }
    }

    #[test]
    fn a_kind_change_is_a_modification_of_the_same_path() {
        let before = file("link", b"target");
        let mut after = before.clone();
        after.kind = ArtifactKind::Symlink;
        after.content_digest = None;
        let delta = diff_inventories(&report(vec![before]), &report(vec![after])).unwrap();
        assert_eq!(
            change(&delta, "link"),
            Some((
                ArtifactChangeKind::Modified,
                ArtifactChangeCause::KindChanged
            ))
        );
    }

    #[test]
    fn symlinks_and_special_files_carry_no_content_identity_so_they_always_count_as_modified() {
        let mut link = file("l", b"");
        link.kind = ArtifactKind::Symlink;
        link.content_digest = None;
        let delta = diff_inventories(&report(vec![link.clone()]), &report(vec![link])).unwrap();
        assert_eq!(
            change(&delta, "l"),
            Some((
                ArtifactChangeKind::Modified,
                ArtifactChangeCause::ContentIdentityUnavailable
            ))
        );
    }

    #[test]
    fn a_policy_boundary_present_on_both_sides_is_unchanged() {
        let mut boundary = file("target", b"");
        boundary.kind = ArtifactKind::PolicyBoundary;
        boundary.content_digest = None;
        let delta =
            diff_inventories(&report(vec![boundary.clone()]), &report(vec![boundary])).unwrap();
        assert!(delta.is_empty());
    }

    #[test]
    fn a_move_is_a_delete_plus_a_create_never_a_rename() {
        let delta = diff_inventories(
            &report(vec![file("old.rs", b"body")]),
            &report(vec![file("new.rs", b"body")]),
        )
        .unwrap();
        assert_eq!((delta.created, delta.deleted, delta.modified), (1, 1, 0));
    }

    #[test]
    fn case_only_path_differences_are_distinct_identities() {
        let delta = diff_inventories(
            &report(vec![file("A.rs", b"x")]),
            &report(vec![file("a.rs", b"x")]),
        )
        .unwrap();
        assert_eq!((delta.created, delta.deleted), (1, 1));
    }

    #[test]
    fn unchanged_content_with_new_classification_is_drift_not_a_change() {
        let before = file("a.txt", b"text");
        let mut after = before.clone();
        after.disposition = ArtifactDisposition::Unknown;
        after.language = None;
        let delta = diff_inventories(&report(vec![before]), &report(vec![after])).unwrap();
        assert!(delta.is_empty());
        assert_eq!(delta.classification_drift, ["a.txt"]);
    }

    #[test]
    fn mismatched_roots_schemas_or_duplicate_paths_are_refused() {
        let a = report(vec![file("a.rs", b"a")]);
        let mut other_root = a.clone();
        other_root.root = "/elsewhere".into();
        assert!(matches!(
            diff_inventories(&a, &other_root),
            Err(InventoryDiffRefusal::RootDiffers { .. })
        ));
        let mut old_schema = a.clone();
        old_schema.schema = "atlas.inventory-report.v1".into();
        assert!(matches!(
            diff_inventories(&old_schema, &a),
            Err(InventoryDiffRefusal::SchemaDiffers { .. })
        ));
        let duplicated = report(vec![file("a.rs", b"a"), file("a.rs", b"b")]);
        assert!(matches!(
            diff_inventories(&a, &duplicated),
            Err(InventoryDiffRefusal::DuplicatePath { .. })
        ));
        assert!(matches!(
            diff_inventories(&duplicated, &a),
            Err(InventoryDiffRefusal::DuplicatePath { .. })
        ));
    }

    /// Deterministic xorshift64*, so any failing case is reproducible.
    struct Stream(u64);
    impl Stream {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
    }

    #[test]
    fn replaying_the_delta_onto_the_previous_snapshot_reproduces_the_current_one() {
        // The fast-path invariant: previous + delta must equal a full re-walk (the current
        // snapshot) path for path, for random edit scripts; and the reverse diff must mirror it.
        let mut stream = Stream(0xA5A5_5A5A_1234_5678);
        for round in 0..200 {
            let mut state: BTreeMap<String, Vec<u8>> = BTreeMap::new();
            for i in 0..(stream.next() % 12) {
                state.insert(format!("f{i}.rs"), vec![(stream.next() % 4) as u8]);
            }
            let before = state.clone();
            for _ in 0..(stream.next() % 8) {
                let path = format!("f{}.rs", stream.next() % 14);
                match stream.next() % 3 {
                    0 => {
                        state.remove(&path);
                    }
                    _ => {
                        state.insert(path, vec![(stream.next() % 4) as u8]);
                    }
                }
            }
            let snapshot = |files: &BTreeMap<String, Vec<u8>>| {
                report(files.iter().map(|(p, c)| file(p, c)).collect())
            };
            let (previous, current) = (snapshot(&before), snapshot(&state));
            let delta = diff_inventories(&previous, &current).unwrap();

            let mut replayed = before.clone();
            for c in &delta.changes {
                match c.change {
                    ArtifactChangeKind::Deleted => {
                        replayed.remove(&c.path);
                    }
                    _ => {
                        replayed.insert(c.path.clone(), state[&c.path].clone());
                    }
                }
            }
            assert_eq!(replayed, state, "round {round}");
            let truly_changed = before
                .keys()
                .chain(state.keys())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .filter(|p| before.get(*p) != state.get(*p))
                .count();
            assert_eq!(
                delta.changes.len(),
                truly_changed,
                "round {round}: no spurious change"
            );

            let reverse = diff_inventories(&current, &previous).unwrap();
            assert_eq!(
                (reverse.created, reverse.deleted),
                (delta.deleted, delta.created)
            );
            assert_eq!(reverse.modified, delta.modified);
        }
    }
}
