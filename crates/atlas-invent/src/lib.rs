use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventionRequest {
    pub target_repo: String,
    pub exact_base_sha: String,
    pub docs_ready: bool,
    pub graph_before_code: bool,
    pub zero_runtime_dependency_target: bool,
    pub objective: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventionPlan {
    pub schema: String,
    pub admitted: bool,
    pub target_repo: String,
    pub blockers: Vec<String>,
    pub stages: Vec<String>,
}

pub fn plan(request: InventionRequest) -> InventionPlan {
    let mut blockers = Vec::new();
    if !request.docs_ready { blockers.push("DOCS_GATE_NOT_READY".into()); }
    if !request.graph_before_code { blockers.push("GRAPH_BEFORE_CODE_REQUIRED".into()); }
    if !request.zero_runtime_dependency_target { blockers.push("ZERO_RUNTIME_DEPENDENCY_TARGET_REQUIRED".into()); }
    if request.exact_base_sha.len() != 40 { blockers.push("EXACT_BASE_SHA_REQUIRED".into()); }

    InventionPlan {
        schema: "atlas.systemizer.invention-plan.v1".into(),
        admitted: blockers.is_empty(),
        target_repo: request.target_repo,
        blockers,
        stages: vec![
            "NORTH_STAR".into(),
            "SYSTEM_BLUEPRINT".into(),
            "DOCS_PLAN".into(),
            "OSS_DISCOVERY".into(),
            "TECHNOLOGY_GRAPH".into(),
            "TARGET_DESIGN_GRAPH".into(),
            "BOUNDED_IMPLEMENTATION".into(),
            "DIFFERENTIAL_PROOF".into(),
            "ZERO_DEPENDENCY_ADMISSION".into(),
        ],
    }
}
