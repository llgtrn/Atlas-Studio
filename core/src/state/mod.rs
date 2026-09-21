//! Canonical state descriptors that are independent of filesystem or VCS mechanics.

use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepositorySnapshot {
    pub schema: String,
    pub root: String,
    pub head_sha: String,
    pub branch: Option<String>,
    pub dirty: bool,
    pub status_entries: Vec<String>,
}

impl RepositorySnapshot {
    pub fn revision(&self) -> RevisionRef {
        RevisionRef {
            kind: "git".into(),
            value: self.head_sha.clone(),
        }
    }
}
