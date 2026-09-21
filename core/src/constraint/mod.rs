//! Admission constraints and repository invariants.

use crate::schema::RepoManifest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodingAdmission {
    pub schema: String,
    pub allowed: bool,
    pub docs_standard: String,
    pub blockers: Vec<String>,
}

pub fn validate_manifest(manifest: &RepoManifest) -> Vec<String> {
    let mut violations = Vec::new();
    let expected = [
        ("schema", &manifest.schema, "atlas.repo.v2"),
        (
            "system_kind",
            &manifest.system_kind,
            "SYSTEM_INVENTION_FORGE",
        ),
        ("backend_language", &manifest.backend_language, "rust"),
        (
            "frontend_language",
            &manifest.frontend_language,
            "typescript",
        ),
        ("knowledge_root", &manifest.knowledge_root, ".atlas"),
        (
            "temporary_root",
            &manifest.temporary_root,
            ".atlas/temporary",
        ),
        (
            "provenance_root",
            &manifest.provenance_root,
            ".atlas/provenance",
        ),
        ("license_root", &manifest.license_root, ".atlas/licenses"),
    ];
    for (field, actual, required) in expected {
        if actual != required {
            violations.push(format!("{field} must be {required}"));
        }
    }

    let required_true = [
        (
            "coding_requires_docs_gate",
            manifest.coding_requires_docs_gate,
        ),
        (
            "graph_before_code_required",
            manifest.graph_before_code_required,
        ),
        ("exact_base_sha_required", manifest.exact_base_sha_required),
        (
            "single_repository_target_required",
            manifest.single_repository_target_required,
        ),
    ];
    for (field, actual) in required_true {
        if !actual {
            violations.push(format!("{field} must be true"));
        }
    }
    violations
}
