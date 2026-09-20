//! Production support semantics for building and operating Atlas itself.
//!
//! This crate is intentionally product-neutral. It models operational support lanes that help
//! Atlas create, verify and maintain its own system without importing product-specific intelligence
//! concepts.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpsSupportLane {
    pub id: String,
    pub purpose: String,
    pub evidence_required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpsProductionPlan {
    pub schema: String,
    pub system: String,
    pub role: String,
    pub forbidden_lanes: Vec<String>,
    pub lanes: Vec<OpsSupportLane>,
}

pub fn atlas_ops_production_plan() -> OpsProductionPlan {
    OpsProductionPlan {
        schema: "atlas.ops-production.plan.v1".into(),
        system: "AtlasSystemizer".into(),
        role: "support_atlas_system_creation_and_operation".into(),
        forbidden_lanes: vec!["organism".into()],
        lanes: vec![
            OpsSupportLane {
                id: "production-readiness".into(),
                purpose: "Track the evidence required before Atlas claims production readiness."
                    .into(),
                evidence_required: vec![
                    "systemize-report".into(),
                    "cargo-test-workspace".into(),
                    "donor-provenance-validation".into(),
                ],
            },
            OpsSupportLane {
                id: "donor-absorption".into(),
                purpose: "Keep OSS donor cloning, licensing, census and native replacement auditable."
                    .into(),
                evidence_required: vec![
                    "donor-manifest".into(),
                    "license-map".into(),
                    "native-technology-status".into(),
                ],
            },
            OpsSupportLane {
                id: "atlas-system-buildout".into(),
                purpose: "Coordinate support work that creates Atlas compiler, graph and verification capabilities."
                    .into(),
                evidence_required: vec![
                    "adl-check-report".into(),
                    "graph-report".into(),
                    "work-prepare-report".into(),
                ],
            },
        ],
    }
}

pub fn validates_atlas_support_shape(plan: &OpsProductionPlan) -> bool {
    !plan.lanes.iter().any(|lane| {
        plan.forbidden_lanes
            .iter()
            .any(|forbidden| forbidden == &lane.id)
    }) && plan.lanes.iter().all(|lane| {
        !lane.id.trim().is_empty()
            && !lane.purpose.trim().is_empty()
            && !lane.evidence_required.is_empty()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_ops_production_is_support_not_organism() {
        let plan = atlas_ops_production_plan();
        assert!(validates_atlas_support_shape(&plan));
        assert!(plan.forbidden_lanes.iter().any(|lane| lane == "organism"));
        assert!(!plan.lanes.iter().any(|lane| lane.id == "organism"));
    }
}
