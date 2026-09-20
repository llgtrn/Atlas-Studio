use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OssCandidate {
    pub repo: String,
    pub exact_sha: String,
    pub technology_roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OssAllocation {
    pub schema: String,
    pub donor_repo: String,
    pub exact_sha: String,
    pub primary_target: String,
    pub consumers: Vec<String>,
    pub runtime_dependency_allowed: bool,
}

pub fn allocate(candidate: OssCandidate, primary_target: String, consumers: Vec<String>) -> Result<OssAllocation, String> {
    if candidate.exact_sha.len() != 40 {
        return Err("OSS allocation requires exact SHA".into());
    }
    let unique = consumers.iter().collect::<BTreeSet<_>>();
    if unique.len() != consumers.len() {
        return Err("duplicate OSS consumer".into());
    }
    Ok(OssAllocation {
        schema: "atlas.systemizer.oss-allocation.v1".into(),
        donor_repo: candidate.repo,
        exact_sha: candidate.exact_sha,
        primary_target,
        consumers,
        runtime_dependency_allowed: false,
    })
}
