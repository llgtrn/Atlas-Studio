use crate::{ArtifactId, IntegrityDigest};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod dependency;

pub use dependency::{
    DependencyActivation, DependencyClosureReport, DependencyClosureState, DependencyEcosystem,
    DependencyEdge, DependencyIdentity, DependencyReachability, DependencyRole,
    DependencySourceKind, DynamicDependencyObligation, ReachOrigin, ReachabilityApproximation,
    ReachedInstance,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArtifactKind {
    File,
    Symlink,
    PolicyBoundary,
    Special,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArtifactDisposition {
    Parsed,
    BinaryDescribed,
    Generated,
    IgnoredByExplicitPolicy,
    Unsupported,
    Unknown,
    ExternalizedWithEvidence,
}

impl ArtifactDisposition {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Parsed => "PARSED",
            Self::BinaryDescribed => "BINARY_DESCRIBED",
            Self::Generated => "GENERATED",
            Self::IgnoredByExplicitPolicy => "IGNORED_BY_EXPLICIT_POLICY",
            Self::Unsupported => "UNSUPPORTED",
            Self::Unknown => "UNKNOWN",
            Self::ExternalizedWithEvidence => "EXTERNALIZED_WITH_EVIDENCE",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactRecord {
    pub id: ArtifactId,
    pub path: String,
    pub kind: ArtifactKind,
    pub bytes: u64,
    pub disposition: ArtifactDisposition,
    pub language: Option<String>,
    pub reason: Option<String>,
    /// BLAKE3-256 of exactly the bytes read for this regular file (ADR 0005). `None` whenever the
    /// digest could not be taken over a stable, bounded read -- a non-file, an over-cap file, a
    /// file that changed while being read -- and a consumer comparing inventories must then
    /// treat the artifact as changed, never as unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_digest: Option<IntegrityDigest>,
    /// Why `content_digest` is absent for a regular file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_digest_withheld: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryReport {
    pub schema: String,
    pub root: String,
    pub artifacts_total: usize,
    pub dispositions: BTreeMap<String, usize>,
    pub artifacts: Vec<ArtifactRecord>,
}

impl InventoryReport {
    pub fn new(root: impl Into<String>, mut artifacts: Vec<ArtifactRecord>) -> Self {
        artifacts.sort_by(|a, b| a.path.cmp(&b.path));
        let mut dispositions = BTreeMap::new();
        for artifact in &artifacts {
            *dispositions
                .entry(artifact.disposition.as_str().to_owned())
                .or_insert(0) += 1;
        }
        Self {
            schema: "atlas.inventory-report.v2".into(),
            root: root.into(),
            artifacts_total: artifacts.len(),
            dispositions,
            artifacts,
        }
    }

    pub fn accounted_total(&self) -> usize {
        self.dispositions.values().sum()
    }

    pub fn is_closed(&self) -> bool {
        self.accounted_total() == self.artifacts_total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(path: &str, disposition: ArtifactDisposition) -> ArtifactRecord {
        ArtifactRecord {
            id: ArtifactId::new(format!("artifact:{path}")),
            path: path.into(),
            kind: ArtifactKind::File,
            bytes: 10,
            disposition,
            language: None,
            reason: None,
            content_digest: None,
            content_digest_withheld: None,
        }
    }

    #[test]
    fn a_freshly_constructed_report_is_always_closed() {
        let report = InventoryReport::new(
            "/repo",
            vec![
                artifact("a.rs", ArtifactDisposition::Parsed),
                artifact("b.bin", ArtifactDisposition::BinaryDescribed),
            ],
        );
        assert_eq!(report.accounted_total(), 2);
        assert!(report.is_closed());
    }

    // `INVENTORY_ACCOUNTING_NOT_CLOSED` (`runtime::systemize`) is this check's sole real-world
    // consumer, and gates real coding admission -- but every production constructor
    // (`InventoryReport::new`, the only one in real use) keeps `artifacts_total`/`dispositions`
    // mutually consistent by construction, so `is_closed()` is always `true` on any report this
    // codebase actually builds today. Before this test, nothing anywhere in the workspace directly
    // exercised `is_closed()` at all (confirmed by grep: `INVENTORY_ACCOUNTING_NOT_CLOSED` appears
    // exactly once, in the blocker that reads this check, never in a test) -- so a regression that
    // silently made `is_closed()` unconditionally `true` (e.g. an accidental `true` literal, or a
    // comparison against the wrong field) would have gone undetected indefinitely. `dispositions`
    // is `pub`, so a corrupted report -- e.g. one round-tripped through a stale cached JSON file
    // whose disposition counts no longer match its artifact list -- is a real, constructible value
    // of this type, not merely a hypothetical.
    #[test]
    fn a_report_whose_disposition_counts_disagree_with_its_artifacts_is_not_closed() {
        let mut report = InventoryReport::new(
            "/repo",
            vec![
                artifact("a.rs", ArtifactDisposition::Parsed),
                artifact("b.rs", ArtifactDisposition::Parsed),
            ],
        );
        report.dispositions.insert("PARSED".into(), 1);
        assert_eq!(
            report.accounted_total(),
            1,
            "sanity: the corrupted dispositions map now disagrees with artifacts_total"
        );
        assert!(
            !report.is_closed(),
            "a disposition count that disagrees with the real artifact list must never read as closed"
        );
    }

    #[test]
    fn a_report_whose_artifacts_total_disagrees_with_its_artifact_list_is_not_closed() {
        let mut report =
            InventoryReport::new("/repo", vec![artifact("a.rs", ArtifactDisposition::Parsed)]);
        report.artifacts_total = 5;
        assert!(
            !report.is_closed(),
            "an artifacts_total that disagrees with the real disposition tally must never read as closed"
        );
    }
}
