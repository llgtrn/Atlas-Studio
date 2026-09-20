use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CiShard {
    pub worker_repo: String,
    pub exact_base_sha: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DistributedCiPlan {
    pub schema: String,
    pub canonical_target: String,
    pub shards: Vec<CiShard>,
    pub aggregate_required: bool,
}

pub fn plan(canonical_target: String, shards: Vec<CiShard>) -> Result<DistributedCiPlan, String> {
    if shards.iter().any(|s| s.exact_base_sha.len() != 40) {
        return Err("all CI shards require exact base SHA".into());
    }
    Ok(DistributedCiPlan {
        schema: "atlas.systemizer.distributed-ci-plan.v1".into(),
        canonical_target,
        shards,
        aggregate_required: true,
    })
}
