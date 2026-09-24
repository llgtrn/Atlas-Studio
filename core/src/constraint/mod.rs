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

/// Three-valued outcome of evaluating one constraint or invariant (ADR 0007).
///
/// `.atlas/contracts/ARCHITECTURAL-INTEGRITY.md`: "If Atlas cannot evaluate a required hard
/// invariant, the result is UNKNOWN/INCOMPLETE, not PASS." A boolean cannot carry that third
/// state -- it previously folded "could not be evaluated" into "failed", so a report could not say
/// whether a counterexample exists. `Unknown` never admits, and is never promoted to `Satisfied`.
#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConstraintVerdict {
    /// Evaluated over every relevant fact; no counterexample.
    Satisfied,
    /// At least one definite counterexample exists.
    Violated,
    /// Not decidable from the available facts, and no counterexample was found.
    #[default]
    Unknown,
}

impl ConstraintVerdict {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Satisfied => "SATISFIED",
            Self::Violated => "VIOLATED",
            Self::Unknown => "UNKNOWN",
        }
    }

    /// Only a fully evaluated, counterexample-free result admits.
    pub fn admits(self) -> bool {
        self == Self::Satisfied
    }

    /// Strong Kleene conjunction: a definite counterexample decides the conjunction even when
    /// another conjunct is undecidable; otherwise any undecidable conjunct leaves it undecided.
    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::Violated, _) | (_, Self::Violated) => Self::Violated,
            (Self::Unknown, _) | (_, Self::Unknown) => Self::Unknown,
            (Self::Satisfied, Self::Satisfied) => Self::Satisfied,
        }
    }

    pub fn all(verdicts: impl IntoIterator<Item = Self>) -> Self {
        verdicts.into_iter().fold(Self::Satisfied, Self::and)
    }
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

    for (field, roots) in [
        ("source_roots", &manifest.source_roots),
        ("backend_roots", &manifest.backend_roots),
        ("frontend_roots", &manifest.frontend_roots),
        ("test_roots", &manifest.test_roots),
    ] {
        for root in roots {
            if !declared_root_is_contained(root) {
                violations.push(format!(
                    "{field} entry `{root}` escapes the repository boundary"
                ));
            }
        }
    }
    violations
}

/// Whether a manifest-declared root, taken as a standalone string, could ever resolve outside
/// the repository root it is meant to be joined against. Checked purely lexically -- the declared
/// root may not exist on disk yet, so this must never `canonicalize` it. Mirrors
/// `adapter::source::declared_root_is_contained`, the sink-side enforcement of the same rule:
/// this copy exists so an escaping declared root is also a reported `policy_violations` entry
/// (and therefore a `REPO_GATE_NOT_READY` blocker), not only a silently-filtered inventory
/// artifact -- `core` has no dependency on `adapter` to share the one implementation directly.
/// `pub`: also used by `runtime::prepare_work` to filter `WorkRequest.allowed_paths` -- the
/// literal capability-scoping data handed to an external provider -- so an escaping manifest
/// entry can never appear in it, as defense in depth beyond `coding_admission.allowed` alone.
pub fn declared_root_is_contained(declared: &str) -> bool {
    use std::path::{Component, Path};
    let declared_path = Path::new(declared);
    if declared_path.is_absolute() {
        return false;
    }
    let mut depth: i64 = 0;
    for component in declared_path.components() {
        match component {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_with_roots(source_roots: Vec<&str>) -> RepoManifest {
        RepoManifest {
            schema: "atlas.repo.v2".into(),
            repo: "org/repo".into(),
            system_kind: "SYSTEM_INVENTION_FORGE".into(),
            backend_language: "rust".into(),
            frontend_language: "typescript".into(),
            coding_requires_docs_gate: true,
            graph_before_code_required: true,
            exact_base_sha_required: true,
            single_repository_target_required: true,
            knowledge_root: ".atlas".into(),
            temporary_root: ".atlas/temporary".into(),
            provenance_root: ".atlas/provenance".into(),
            license_root: ".atlas/licenses".into(),
            source_roots: source_roots.into_iter().map(String::from).collect(),
            backend_roots: Vec::new(),
            frontend_roots: Vec::new(),
            test_roots: Vec::new(),
        }
    }

    #[test]
    fn ordinary_relative_roots_are_contained() {
        assert!(declared_root_is_contained("core"));
        assert!(declared_root_is_contained("apps/studio/src"));
        assert!(declared_root_is_contained("./core"));
        assert!(declared_root_is_contained("core/../runtime"));
    }

    #[test]
    fn an_absolute_root_is_never_contained() {
        // `Path::join` returns an absolute joinee verbatim, discarding the intended root
        // entirely -- this is the exact escape a hostile or careless manifest could exploit.
        assert!(!declared_root_is_contained("/etc"));
    }

    #[test]
    fn a_net_upward_traversal_is_never_contained() {
        assert!(!declared_root_is_contained("../../etc"));
        assert!(!declared_root_is_contained("core/../../etc"));
    }

    #[test]
    fn validate_manifest_reports_an_escaping_source_root_as_a_violation() {
        let manifest = manifest_with_roots(vec!["../../etc"]);
        let violations = validate_manifest(&manifest);
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("source_roots")
                    && violation.contains("../../etc")),
            "expected an escape violation, got: {violations:?}"
        );
    }

    #[test]
    fn validate_manifest_accepts_ordinary_relative_roots() {
        let manifest = manifest_with_roots(vec!["core", "core/tests"]);
        assert!(validate_manifest(&manifest).is_empty());
    }
}

#[cfg(test)]
mod verdict_tests {
    use super::ConstraintVerdict::{self, *};

    const ALL: [ConstraintVerdict; 3] = [Satisfied, Violated, Unknown];

    #[test]
    fn conjunction_is_strong_kleene_over_the_full_truth_table() {
        // Oracle: Kleene's K3 with Satisfied=1, Unknown=1/2, Violated=0 and AND = min.
        let rank = |v: ConstraintVerdict| match v {
            Violated => 0,
            Unknown => 1,
            Satisfied => 2,
        };
        for a in ALL {
            for b in ALL {
                assert_eq!(rank(a.and(b)), rank(a).min(rank(b)), "{a:?} and {b:?}");
                assert_eq!(a.and(b), b.and(a));
                for c in ALL {
                    assert_eq!(a.and(b).and(c), a.and(b.and(c)));
                }
            }
        }
        assert_eq!(ConstraintVerdict::all([]), Satisfied, "empty conjunction");
    }

    #[test]
    fn only_satisfied_admits_and_the_default_is_never_a_pass() {
        assert!(Satisfied.admits());
        assert!(!Violated.admits());
        assert!(!Unknown.admits());
        assert_eq!(ConstraintVerdict::default(), Unknown);
    }
}
