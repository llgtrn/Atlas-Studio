use crate::ArtifactId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod dependency;

pub use dependency::{
    DependencyActivation, DependencyClosureReport, DependencyClosureState, DependencyEcosystem,
    DependencyEdge, DependencyIdentity, DependencyRole, DependencySourceKind,
    DynamicDependencyObligation,
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
            schema: "atlas.inventory-report.v1".into(),
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
