//! Cross-repository engineering/network planning.
//!
//! Atlas may produce a plan, branch/worktree instructions and evidence requirements.
//! It never owns merge authority.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const RESPONSIBILITY: &str = "FLEET";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkRequest {
    pub id: String,
    pub repo: String,
    pub base_sha: String,
    pub scope: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FleetTask {
    pub id: String,
    pub repo: String,
    pub base_sha: String,
    pub scope: Vec<String>,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FleetPlan {
    pub schema: String,
    pub merge_authority: bool,
    pub tasks: Vec<FleetTask>,
    pub parallel_waves: Vec<Vec<String>>,
}

fn exact_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

pub fn plan(mut requests: Vec<WorkRequest>) -> Result<FleetPlan, String> {
    let mut ids = BTreeSet::new();
    let mut repo_sha = BTreeMap::<String, String>::new();

    for request in &requests {
        if !ids.insert(request.id.clone()) {
            return Err(format!("duplicate work request id {}", request.id));
        }
        if !exact_sha(&request.base_sha) {
            return Err(format!("{} requires exact lowercase 40-char base SHA", request.id));
        }
        if let Some(existing) = repo_sha.insert(request.repo.clone(), request.base_sha.clone()) {
            if existing != request.base_sha {
                return Err(format!("repo {} has multiple base SHAs in one plan", request.repo));
            }
        }
    }

    requests.sort_by(|a,b| a.repo.cmp(&b.repo).then(a.id.cmp(&b.id)));
    let mut previous_by_repo = BTreeMap::<String, String>::new();
    let mut tasks = Vec::new();
    for request in requests {
        let depends_on = previous_by_repo
            .get(&request.repo)
            .cloned()
            .into_iter()
            .collect::<Vec<_>>();
        previous_by_repo.insert(request.repo.clone(), request.id.clone());
        tasks.push(FleetTask {
            id: request.id,
            repo: request.repo,
            base_sha: request.base_sha,
            scope: request.scope,
            depends_on,
        });
    }

    let mut waves = Vec::<Vec<String>>::new();
    let mut depth_by_repo = BTreeMap::<String, usize>::new();
    for task in &tasks {
        let depth = *depth_by_repo.get(&task.repo).unwrap_or(&0);
        if waves.len() <= depth { waves.push(Vec::new()); }
        waves[depth].push(task.id.clone());
        depth_by_repo.insert(task.repo.clone(), depth + 1);
    }
    for wave in &mut waves { wave.sort(); }

    Ok(FleetPlan {
        schema: "atlas.systemizer.fleet-plan.v1".into(),
        merge_authority: false,
        tasks,
        parallel_waves: waves,
    })
}

pub fn subsystem_boundary() -> &'static str {
    "DEVELOPMENT_ENGINEERING_TOOL"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallelizes_across_repos_and_serializes_within_repo() {
        let sha = "a".repeat(40);
        let plan = plan(vec![
            WorkRequest { id:"a1".into(), repo:"org/A".into(), base_sha:sha.clone(), scope:vec!["x".into()] },
            WorkRequest { id:"a2".into(), repo:"org/A".into(), base_sha:sha.clone(), scope:vec!["y".into()] },
            WorkRequest { id:"b1".into(), repo:"org/B".into(), base_sha:sha, scope:vec!["z".into()] },
        ]).unwrap();
        assert_eq!(plan.parallel_waves[0], vec!["a1".to_string(), "b1".to_string()]);
        assert_eq!(plan.parallel_waves[1], vec!["a2".to_string()]);
        assert_eq!(plan.tasks.iter().find(|t| t.id=="a2").unwrap().depends_on, vec!["a1".to_string()]);
        assert!(!plan.merge_authority);
    }
}
