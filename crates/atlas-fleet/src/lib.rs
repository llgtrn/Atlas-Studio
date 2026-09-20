use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path, process::Command};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkRequest {
    pub id: String,
    pub repo: String,
    pub base_sha: String,
    pub scope: Vec<String>,
    pub repo_gate_ready: bool,
    pub docs_gate_ready: bool,
    pub docs_standard: String,
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
    pub selected_repo: String,
    pub merge_authority: bool,
    pub tasks: Vec<FleetTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectedRepo {
    pub repo: String,
    pub reference: String,
    pub head_sha: Option<String>,
    pub connected: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FleetConnectionReport {
    pub schema: String,
    pub manifest: String,
    pub repositories_total: usize,
    pub connected_total: usize,
    pub coding_mode: String,
    pub repositories: Vec<ConnectedRepo>,
}

fn exact_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

pub fn plan(mut requests: Vec<WorkRequest>) -> Result<FleetPlan, String> {
    if requests.is_empty() { return Err("coding plan requires at least one work request".into()); }
    let repos = requests.iter().map(|r| r.repo.as_str()).collect::<BTreeSet<_>>();
    if repos.len() != 1 {
        return Err("Atlas coding admission permits exactly one repository per coding session".into());
    }
    let selected_repo = (*repos.iter().next().unwrap()).to_owned();
    let mut ids = BTreeSet::new();
    let first_sha = requests[0].base_sha.clone();

    for request in &requests {
        if !ids.insert(request.id.clone()) { return Err(format!("duplicate work request id {}", request.id)); }
        if !request.repo_gate_ready { return Err(format!("{} blocked: repository gate is not ready", request.id)); }
        if !request.docs_gate_ready { return Err(format!("{} blocked: documentation gate is not ready", request.id)); }
        if request.docs_standard != "atlas.docs.v1" { return Err(format!("{} blocked: unsupported docs standard {}", request.id, request.docs_standard)); }
        if !exact_sha(&request.base_sha) { return Err(format!("{} requires exact lowercase 40-char base SHA", request.id)); }
        if request.base_sha != first_sha { return Err("one coding session must use one exact repository base SHA".into()); }
    }

    requests.sort_by(|a,b| a.id.cmp(&b.id));
    let mut previous: Option<String> = None;
    let tasks = requests.into_iter().map(|request| {
        let depends_on = previous.clone().into_iter().collect::<Vec<_>>();
        previous = Some(request.id.clone());
        FleetTask { id: request.id, repo: request.repo, base_sha: request.base_sha, scope: request.scope, depends_on }
    }).collect();

    Ok(FleetPlan {
        schema: "atlas.systemizer.fleet-plan.v2".into(),
        selected_repo,
        merge_authority: false,
        tasks,
    })
}

fn parse_repo_line(line: &str) -> Option<(String,String)> {
    let line = line.trim();
    if !line.starts_with("- {") || !line.contains("repo:") { return None; }
    let body = line.trim_start_matches("- {").trim_end_matches('}');
    let mut repo = None;
    let mut reference = Some("main".to_owned());
    for part in body.split(',') {
        let Some((key,value)) = part.split_once(':') else { continue };
        match key.trim() {
            "repo" => repo = Some(value.trim().to_owned()),
            "ref" => reference = Some(value.trim().to_owned()),
            _ => {}
        }
    }
    repo.map(|repo| (repo, reference.unwrap_or_else(|| "main".into())))
}

pub fn registered_repositories(manifest: impl AsRef<Path>) -> std::io::Result<Vec<String>> {
    let text = fs::read_to_string(manifest)?;
    let mut repos = text
        .lines()
        .filter_map(parse_repo_line)
        .map(|(repo, _)| repo)
        .collect::<Vec<_>>();
    repos.sort();
    repos.dedup();
    Ok(repos)
}

pub fn connect(manifest: impl AsRef<Path>) -> std::io::Result<FleetConnectionReport> {
    let manifest = manifest.as_ref();
    let text = fs::read_to_string(manifest)?;
    let definitions = text.lines().filter_map(parse_repo_line).collect::<Vec<_>>();
    let mut repositories = Vec::new();

    for (repo, reference) in definitions {
        let url = format!("https://github.com/{repo}.git");
        let refspec = format!("refs/heads/{reference}");
        let output = Command::new("git").args(["ls-remote","--heads",&url,&refspec]).output();
        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let sha = stdout.split_whitespace().next().filter(|v| exact_sha(v)).map(ToOwned::to_owned);
                repositories.push(ConnectedRepo { repo, reference, connected: sha.is_some(), head_sha: sha, error: None });
            }
            Ok(output) => repositories.push(ConnectedRepo {
                repo, reference, head_sha: None, connected: false,
                error: Some(String::from_utf8_lossy(&output.stderr).trim().to_owned()),
            }),
            Err(error) => repositories.push(ConnectedRepo { repo, reference, head_sha: None, connected: false, error: Some(error.to_string()) }),
        }
    }

    let connected_total = repositories.iter().filter(|row| row.connected).count();
    Ok(FleetConnectionReport {
        schema: "atlas.systemizer.fleet-connection.v1".into(),
        manifest: manifest.to_string_lossy().into_owned(),
        repositories_total: repositories.len(),
        connected_total,
        coding_mode: "ONE_REPOSITORY_AT_A_TIME".into(),
        repositories,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_cross_repo_coding_plan() {
        let sha = "a".repeat(40);
        let req = |id:&str, repo:&str| WorkRequest {
            id:id.into(), repo:repo.into(), base_sha:sha.clone(), scope:vec!["x".into()],
            repo_gate_ready:true, docs_gate_ready:true, docs_standard:"atlas.docs.v1".into()
        };
        assert!(plan(vec![req("a","org/A"), req("b","org/B")]).is_err());
    }

    #[test]
    fn refuses_code_when_docs_gate_fails() {
        let request = WorkRequest {
            id:"a".into(), repo:"org/A".into(), base_sha:"a".repeat(40), scope:vec!["x".into()],
            repo_gate_ready:true, docs_gate_ready:false, docs_standard:"atlas.docs.v1".into()
        };
        assert!(plan(vec![request]).is_err());
    }
}
