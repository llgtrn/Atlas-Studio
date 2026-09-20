use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const CLI_API: &str = "atlas.systemizer.cli.v1";
pub const BINARY: &str = "atlas-systemizer";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Contract {
    pub schema: String,
    pub binary: String,
    pub subsystem_kind: String,
    pub runtime_dependency_allowed: bool,
    pub commands: Vec<String>,
}
impl Default for Contract {
    fn default() -> Self {
        Self {
            schema: CLI_API.to_owned(),
            binary: BINARY.to_owned(),
            subsystem_kind: "DEVELOPMENT_ENGINEERING_TOOL".to_owned(),
            runtime_dependency_allowed: false,
            commands: vec!["contract".into(), "systemize".into(), "docs audit".into(), "code analyze".into()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileFact { pub path: String, pub language: String, pub bytes: u64 }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceReport {
    pub schema: String,
    pub root: String,
    pub files_total: usize,
    pub languages: BTreeMap<String, usize>,
    pub files: Vec<FileFact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocsReport {
    pub schema: String,
    pub root: String,
    pub documents_total: usize,
    pub canonical_frontmatter_total: usize,
    pub missing_frontmatter: Vec<String>,
    pub missing_required_fields: Vec<String>,
    pub invalid_type: Vec<String>,
    pub duplicate_ids: Vec<String>,
    pub superseded_without_successor: Vec<String>,
    pub broken_internal_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphSummary {
    pub schema: String,
    pub semantic_grade: String,
    pub nodes_total: usize,
    pub edges_total: usize,
    pub language_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoAuditSummary {
    pub schema: String,
    pub archetype: String,
    pub ready: bool,
    pub missing_required_roles: Vec<String>,
    pub missing_mapped_paths: Vec<String>,
    pub forbidden_roots_present: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemizeReport {
    pub schema: String,
    pub cli_api: String,
    pub root: String,
    pub repository: RepoAuditSummary,
    pub docs: DocsReport,
    pub source: SourceReport,
    pub graph: GraphSummary,
    pub invariants: Vec<String>,
}
