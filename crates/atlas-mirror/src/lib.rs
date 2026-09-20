use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MirrorRequest {
    pub target_repo: String,
    pub target_sha: String,
    pub mirror_count: usize,
    pub owner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MirrorWorker {
    pub ordinal: usize,
    pub proposed_repo: String,
    pub mirror_of: String,
    pub exact_target_sha: String,
    pub topology: String,
    pub canonical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MirrorPlan {
    pub schema: String,
    pub target_repo: String,
    pub target_sha: String,
    pub workers: Vec<MirrorWorker>,
    pub reconvergence_required: bool,
}

fn repo_leaf(repo: &str) -> &str {
    repo.rsplit('/').next().unwrap_or(repo)
}

pub fn plan(request: MirrorRequest) -> Result<MirrorPlan, String> {
    if request.target_sha.len() != 40 {
        return Err("mirror plan requires exact 40-character target SHA".into());
    }
    if request.mirror_count == 0 {
        return Err("mirror_count must be greater than zero".into());
    }
    let leaf = repo_leaf(&request.target_repo).to_ascii_lowercase();
    let workers = (1..=request.mirror_count)
        .map(|ordinal| MirrorWorker {
            ordinal,
            proposed_repo: format!("{}/{}-atlas-mirror-{:02}", request.owner, leaf, ordinal),
            mirror_of: request.target_repo.clone(),
            exact_target_sha: request.target_sha.clone(),
            topology: "ONE_TO_ONE_TARGET_MIRROR".into(),
            canonical: false,
        })
        .collect();

    Ok(MirrorPlan {
        schema: "atlas.systemizer.mirror-plan.v1".into(),
        target_repo: request.target_repo,
        target_sha: request.target_sha,
        workers,
        reconvergence_required: true,
    })
}
