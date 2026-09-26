//! SelectedDesign over a published census container (G148, ADR 0064).
//!
//! A design is proposed and checked against a container read and verified from its bytes (its
//! root identity is recomputed, never taken on trust), the verification report for that
//! container's candidate, and the repository's declared principal registry. Nothing here selects:
//! a `SELECTED` design needs an authority event from a principal the registry declares, and a
//! provider never is one.

use atlas_core::atlas::{CensusAtlas, read};
use atlas_core::design::{
    AuthorityMode, DESIGN_SCHEMA_VERSION, DesignBinding, DesignState, DesignVerdict,
    DesignViolation, PRINCIPAL_REGISTRY_PATH, PrincipalRegistry, SelectedDesign, SemanticRoot,
    container_candidate, design_identity, validate,
};
use atlas_core::verification::VerificationReport;
use std::{fs, io, path::Path};

pub use atlas_core::design::compare::{Criterion, DesignComparison, Measure};
pub use atlas_core::design::{DesignVerdict as Verdict, SelectedDesign as Design};

pub const CHECK_SCHEMA_VERSION: &str = "atlas.selected-design-check.v1";

/// A root named `DIMENSION:RECORD_ID` (e.g. `FUNCTION_IDENTITY:semantic:FUNCTION_IDENTITY:..`).
pub fn parse_root(spec: &str) -> Result<SemanticRoot, String> {
    let (dimension, record_id) = spec
        .split_once(':')
        .ok_or(format!("{spec}: expected DIMENSION:RECORD_ID"))?;
    let dimension = serde_json::from_value(serde_json::Value::String(dimension.into()))
        .map_err(|e| format!("{spec}: {e}"))?;
    Ok(SemanticRoot {
        dimension,
        record_id: record_id.to_owned(),
    })
}

pub fn read_design(path: impl AsRef<Path>) -> io::Result<SelectedDesign> {
    read_json(path)
}

/// A JSON file as `T`; malformed content is `InvalidData`.
pub(crate) fn read_json<T: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> io::Result<T> {
    serde_json::from_str(&fs::read_to_string(path)?).map_err(invalid)
}

pub(crate) fn invalid(why: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why.to_string())
}

/// A container's decoded content and its verified root identity.
pub fn read_container(path: impl AsRef<Path>) -> io::Result<(CensusAtlas, String)> {
    let bytes = fs::read(path)?;
    let (atlas, root) = read(&bytes).map_err(invalid)?;
    Ok((atlas, root.as_str().to_owned()))
}

/// A verification report, bare or as `atlas-systemizer verification self` writes it
/// (`{"evidence": [..], "report": {..}}`).
pub fn read_report(path: impl AsRef<Path>) -> io::Result<VerificationReport> {
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path)?).map_err(invalid)?;
    if let Some(report) = value.get_mut("report").map(serde_json::Value::take) {
        value = report;
    }
    serde_json::from_value(value).map_err(invalid)
}

/// The repository's declared principals; a repository that declares none selects nothing.
pub fn read_registry(root: impl AsRef<Path>) -> io::Result<PrincipalRegistry> {
    match fs::read_to_string(root.as_ref().join(PRINCIPAL_REGISTRY_PATH)) {
        Ok(text) => serde_json::from_str(&text).map_err(invalid),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(PrincipalRegistry::empty()),
        Err(e) => Err(e),
    }
}

/// What a caller asks a design to select.
pub struct Proposal {
    pub roots: Vec<SemanticRoot>,
    pub bindings: Vec<DesignBinding>,
    pub scope: String,
    pub target_kind: String,
    pub variant: String,
    pub rationale: String,
    pub evidence_ref: String,
}

/// A VALIDATED design over `container_path`, resting on `report`: its roots must resolve in the
/// container and the report must admit the container's candidate. Returns the design with every
/// violation (empty when it is valid).
pub fn propose(
    container_path: impl AsRef<Path>,
    report: &VerificationReport,
    proposal: Proposal,
) -> io::Result<(SelectedDesign, Vec<DesignViolation>)> {
    let (container, root) = read_container(container_path)?;
    let mut roots = proposal.roots;
    roots.sort();
    roots.dedup();
    let mut bindings = proposal.bindings;
    bindings.sort();
    let mut design = SelectedDesign {
        schema: DESIGN_SCHEMA_VERSION.into(),
        design_id: String::new(),
        parent_root: root.clone(),
        candidate: container_candidate(&container),
        scope: proposal.scope,
        target_kind: proposal.target_kind,
        variant: proposal.variant,
        state: DesignState::Validated,
        roots,
        bindings,
        candidate_set: Vec::new(),
        evidence_refs: vec![proposal.evidence_ref],
        authority_mode: AuthorityMode::HumanRequired,
        authority_event: None,
        rationale: proposal.rationale,
        supersedes: None,
        comparison: None,
    };
    design.design_id = design_identity(&design);
    design.candidate_set = vec![design.design_id.clone()];
    let violations = validate(
        &design,
        &container,
        &root,
        Some(report),
        &PrincipalRegistry::empty(),
    );
    Ok((design, violations))
}

/// G150 (ADR 0066): candidate designs at one coordinate over `container_path`, one per
/// proposal, each naming all of them as its candidate set. Returns each design with its
/// violations. Proposing candidates never selects.
pub fn propose_candidates(
    container_path: impl AsRef<Path>,
    report: &VerificationReport,
    proposals: Vec<Proposal>,
) -> io::Result<Vec<(SelectedDesign, Vec<DesignViolation>)>> {
    let mut designs = Vec::new();
    for proposal in proposals {
        designs.push(propose(container_path.as_ref(), report, proposal)?);
    }
    let mut ids: Vec<String> = designs.iter().map(|(d, _)| d.design_id.clone()).collect();
    ids.sort();
    ids.dedup();
    for (design, _) in &mut designs {
        design.candidate_set = ids.clone();
    }
    Ok(designs)
}

/// The comparison of `designs` over the verified container by `criteria`, or every reason it
/// cannot be made.
pub fn compare(
    container_path: impl AsRef<Path>,
    report: Option<&VerificationReport>,
    designs: &[SelectedDesign],
    criteria: &[Criterion],
) -> io::Result<Result<DesignComparison, Vec<DesignViolation>>> {
    let (container, root) = read_container(container_path)?;
    Ok(atlas_core::design::compare::compare_designs(
        designs, &container, &root, report, criteria,
    ))
}

pub fn read_comparison(path: impl AsRef<Path>) -> io::Result<DesignComparison> {
    read_json(path)
}

pub fn read_criteria(path: impl AsRef<Path>) -> io::Result<Vec<Criterion>> {
    read_json(path)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct DesignCheck {
    pub schema: String,
    pub design_id: String,
    pub design_state: DesignState,
    pub parent_root: String,
    pub verdict: DesignVerdict,
    pub violations: Vec<DesignViolation>,
}

/// Checks `design` in its state against the container, the report and the registry; a SELECTED
/// design also against the comparison it cites (G150).
pub fn check(
    design: &SelectedDesign,
    container_path: impl AsRef<Path>,
    report: Option<&VerificationReport>,
    registry: &PrincipalRegistry,
    comparison: Option<&DesignComparison>,
) -> io::Result<DesignCheck> {
    let (container, root) = read_container(container_path)?;
    let mut violations = validate(design, &container, &root, report, registry);
    if design.state == DesignState::Selected && design.comparison.is_some() {
        match comparison {
            Some(c) => {
                violations.extend(atlas_core::design::compare::validate_selection(design, c))
            }
            None => violations.push(DesignViolation {
                code: "COMPARISON_UNAVAILABLE".into(),
                detail: "the cited comparison was not supplied".into(),
            }),
        }
        violations.sort();
    }
    Ok(DesignCheck {
        schema: CHECK_SCHEMA_VERSION.into(),
        design_id: design.design_id.clone(),
        design_state: design.state,
        parent_root: root,
        verdict: if violations.is_empty() {
            DesignVerdict::Accepted
        } else {
            DesignVerdict::Rejected
        },
        violations,
    })
}

/// The FUNCTION_IDENTITY roots of a verified container whose function is named `name`: how a
/// pilot names roots by what they are, never by guessing revision-bound record ids.
pub fn function_roots(
    container_path: impl AsRef<Path>,
    name: &str,
) -> io::Result<Vec<SemanticRoot>> {
    function_roots_in(container_path, name, "")
}

/// `function_roots` limited to functions whose source path starts with `path_prefix` (G150: two
/// functions of one name are told apart by where they are, not by guessing).
pub fn function_roots_in(
    container_path: impl AsRef<Path>,
    name: &str,
    path_prefix: &str,
) -> io::Result<Vec<SemanticRoot>> {
    let (container, _) = read_container(container_path)?;
    let mut roots: Vec<SemanticRoot> = container
        .typed_records
        .iter()
        .filter_map(|record| match record {
            atlas_core::SemanticObservation::FunctionIdentity(header)
                if header.subject.symbol.name == name
                    && header.subject.span.path.starts_with(path_prefix) =>
            {
                Some(SemanticRoot {
                    dimension: atlas_core::SemanticDimension::FunctionIdentity,
                    record_id: header.record_id.as_str().to_owned(),
                })
            }
            _ => None,
        })
        .collect();
    roots.sort();
    roots.dedup();
    Ok(roots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::atlas::{CertificateRecord, RootManifest, UNSEALED, write};
    use atlas_core::design::{AuthorityEvent, Principal, PrincipalKind, event_identity};

    /// A published container of core's typed fixture, and a report admitting its candidate.
    fn published(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
        #[derive(serde::Deserialize)]
        struct Fixture {
            typed_records: Vec<atlas_core::SemanticObservation>,
            evidence: Vec<atlas_core::Evidence>,
            diagnostics: Vec<atlas_core::ExtractionDiagnostic>,
        }
        let fixture: Fixture =
            serde_json::from_str(include_str!("../../core/src/atlas/typed_fixture.json")).unwrap();
        let mut atlas = CensusAtlas {
            manifest: RootManifest {
                genome_schema: "atlas.genome.v1".into(),
                genome_hash: [7; 32],
                census_digest: [9; 32],
                revision: "git:abc123".into(),
                certificate_id: "census-certificate:x".into(),
                seal: UNSEALED.into(),
                tool: "test".into(),
                mode: "THIN".into(),
            },
            facts: Vec::new(),
            obligations: Vec::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            certificate: CertificateRecord {
                certificate_id: "census-certificate:x".into(),
                state: "RECONCILED".into(),
                blockers: Vec::new(),
            },
            typed_records: fixture.typed_records,
            evidence: fixture.evidence,
            diagnostics: fixture.diagnostics,
            seal: None,
        };
        atlas.canonicalize();
        fs::create_dir_all(dir).unwrap();
        let container = dir.join("fixture.atlas");
        fs::write(&container, write(&atlas).unwrap()).unwrap();
        // As `verification self` writes it: evidence beside the report.
        let report = dir.join("self-scope.json");
        let body = serde_json::json!({
            "evidence": [],
            "report": {
                "schema": "atlas.verification-report.v1",
                "policy": atlas_core::verification::VerificationPolicy::library_package(),
                "candidate": container_candidate(&atlas),
                "outcomes": [], "coverage": {}, "failures": [],
                "verdict": "ADMISSIBLE", "blockers": []
            }
        });
        fs::write(&report, body.to_string()).unwrap();
        (container, report)
    }

    #[test]
    fn a_design_is_proposed_and_checked_against_the_verified_container_and_nobody_selects_it() {
        let dir = std::env::temp_dir().join(format!("atlas-design-{}", std::process::id()));
        let (container, report_path) = published(&dir);
        let report = read_report(&report_path).unwrap();
        let (container_atlas, root) = read_container(&container).unwrap();
        let function = container_atlas
            .typed_records
            .iter()
            .find_map(|r| match r {
                atlas_core::SemanticObservation::FunctionIdentity(h) => {
                    Some(h.subject.symbol.name.clone())
                }
                _ => None,
            })
            .unwrap();
        let roots = function_roots(&container, &function).unwrap();
        assert!(!roots.is_empty());
        let proposal = |roots: Vec<SemanticRoot>| Proposal {
            roots,
            bindings: Vec::new(),
            scope: "fixture".into(),
            target_kind: "RUST_COMPONENT".into(),
            variant: "default".into(),
            rationale: "test".into(),
            evidence_ref: report_path.display().to_string(),
        };
        let (design, violations) = propose(&container, &report, proposal(roots)).unwrap();
        assert_eq!(violations, []);
        assert_eq!(
            design.parent_root, root,
            "the root is recomputed from the bytes"
        );
        let registry = read_registry(&dir).unwrap();
        assert_eq!(
            registry,
            PrincipalRegistry::empty(),
            "no registry declares nobody"
        );
        let checked = check(&design, &container, Some(&report), &registry, None).unwrap();
        assert_eq!(checked.verdict, DesignVerdict::Accepted);
        // An unknown root is refused at proposal.
        let absent =
            vec![parse_root("FUNCTION_IDENTITY:semantic:FUNCTION_IDENTITY:absent").unwrap()];
        let (_, violations) = propose(&container, &report, proposal(absent)).unwrap();
        assert_eq!(violations[0].code, "ROOT_UNRESOLVED");
        // A provider selecting the design is refused.
        let mut selected = design;
        selected.state = DesignState::Selected;
        let mut event = AuthorityEvent {
            event_id: String::new(),
            principal: Principal {
                id: "coding-agent".into(),
                kind: PrincipalKind::Provider,
            },
            mode: selected.authority_mode,
            design_id: selected.design_id.clone(),
            generation: "G148".into(),
            statement: "select".into(),
        };
        event.event_id = event_identity(&event);
        selected.authority_event = Some(event);
        let refused = check(&selected, &container, Some(&report), &registry, None).unwrap();
        assert_eq!(refused.verdict, DesignVerdict::Rejected);
        let codes: Vec<&str> = refused.violations.iter().map(|v| v.code.as_str()).collect();
        assert_eq!(
            codes,
            [
                "COMPARISON_MISSING",
                "PRINCIPAL_IS_PROVIDER",
                "PRINCIPAL_UNREGISTERED"
            ]
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn candidates_are_proposed_together_compared_and_a_selection_is_checked_against_the_comparison()
    {
        let dir = std::env::temp_dir().join(format!("atlas-candidates-{}", std::process::id()));
        let (container, report_path) = published(&dir);
        let report = read_report(&report_path).unwrap();
        let (atlas, _) = read_container(&container).unwrap();
        let functions: Vec<SemanticRoot> = atlas
            .typed_records
            .iter()
            .filter(|r| r.dimension() == atlas_core::SemanticDimension::FunctionIdentity)
            .map(|r| SemanticRoot {
                dimension: atlas_core::SemanticDimension::FunctionIdentity,
                record_id: r.record_id().as_str().to_owned(),
            })
            .collect();
        let proposal = |roots: Vec<SemanticRoot>| Proposal {
            roots,
            bindings: Vec::new(),
            scope: "fixture".into(),
            target_kind: "RUST_COMPONENT".into(),
            variant: "default".into(),
            rationale: "test".into(),
            evidence_ref: report_path.display().to_string(),
        };
        let designs = propose_candidates(
            &container,
            &report,
            vec![
                proposal(vec![functions[0].clone()]),
                proposal(vec![functions[0].clone(), functions[1].clone()]),
            ],
        )
        .unwrap();
        assert!(designs.iter().all(|(_, v)| v.is_empty()));
        let designs: Vec<SelectedDesign> = designs.into_iter().map(|(d, _)| d).collect();
        assert!(designs.iter().all(|d| d.candidate_set.len() == 2));
        let criteria = vec![Criterion {
            name: "roots".into(),
            measure: Measure::Roots,
            direction: atlas_core::visual::search::Direction::Minimize,
            meaning: "fewer selected functions".into(),
        }];
        let comparison = compare(&container, Some(&report), &designs, &criteria)
            .unwrap()
            .unwrap();
        assert_eq!(comparison.front, [designs[0].design_id.clone()]);
        // A provider selecting even the front design is refused; the comparison itself holds.
        let mut selected = designs[0].clone();
        selected.state = DesignState::Selected;
        selected.comparison = Some(comparison.comparison_id.clone());
        let mut event = AuthorityEvent {
            event_id: String::new(),
            principal: Principal {
                id: "coding-agent".into(),
                kind: PrincipalKind::Provider,
            },
            mode: selected.authority_mode,
            design_id: selected.design_id.clone(),
            generation: "G150".into(),
            statement: "select".into(),
        };
        event.event_id = event_identity(&event);
        selected.authority_event = Some(event);
        let registry = PrincipalRegistry::empty();
        let codes =
            |c: DesignCheck| -> Vec<String> { c.violations.into_iter().map(|v| v.code).collect() };
        let with = check(
            &selected,
            &container,
            Some(&report),
            &registry,
            Some(&comparison),
        );
        assert_eq!(
            codes(with.unwrap()),
            ["PRINCIPAL_IS_PROVIDER", "PRINCIPAL_UNREGISTERED"]
        );
        let without = check(&selected, &container, Some(&report), &registry, None);
        assert!(codes(without.unwrap()).contains(&"COMPARISON_UNAVAILABLE".to_owned()));
        // Through `check`, the cited comparison refuses a dominated candidate.
        let mut dominated = designs[1].clone();
        dominated.state = DesignState::Selected;
        dominated.comparison = Some(comparison.comparison_id.clone());
        dominated.authority_event = selected.authority_event.clone();
        let refused = check(
            &dominated,
            &container,
            Some(&report),
            &registry,
            Some(&comparison),
        );
        assert!(codes(refused.unwrap()).contains(&"DESIGN_DOMINATED".to_owned()));
        // One design alone is no comparison.
        let alone = compare(&container, Some(&report), &designs[..1], &criteria).unwrap();
        assert!(
            alone
                .unwrap_err()
                .iter()
                .any(|v| v.code == "TOO_FEW_CANDIDATES")
        );
        fs::remove_dir_all(&dir).ok();
    }
}
