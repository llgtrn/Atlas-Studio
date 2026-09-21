//! Capability-scoped work contracts.
//!
//! These types describe requested work and admission results. They do not grant execution
//! authority and they do not perform effects.

use crate::{
    constraint::CodingAdmission,
    evidence::Evidence,
    schema::{GraphSummary, RepoAudit},
    state::RepositorySnapshot,
    temporal::RevisionRef,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkRequest {
    pub schema: String,
    pub repository: String,
    pub base_revision: RevisionRef,
    pub goal: String,
    pub scope: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub required_verification: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkPrepareReport {
    pub schema: String,
    pub request: WorkRequest,
    pub repository: RepoAudit,
    pub snapshot: RepositorySnapshot,
    pub graph: GraphSummary,
    pub coding_admission: CodingAdmission,
    pub allowed: bool,
    pub blockers: Vec<String>,
    pub evidence: Vec<Evidence>,
}
