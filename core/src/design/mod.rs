//! SelectedDesign: a design chosen over a verified Atlas root, with its selection authority event
//! (G148, ADR 0064; `.atlas/contracts/SELECTED-DESIGN.md`).
//!
//! A design names the semantic roots it selects inside one verified census container (its parent
//! root) and the verification evidence for that exact candidate. Its identity is a digest of what
//! it selects -- never its display name, state, rationale or authority -- so a selection event
//! commits to one exact design. A design is `SELECTED` only through an authority event from a
//! declared principal whose kind the authority mode admits:
//! - `HUMAN_REQUIRED` and `HYBRID` need a `HUMAN` principal;
//! - `POLICY_AUTO` is refused while no bounded policy envelope is defined;
//! - a `PROVIDER` (an AI or tool proposing the design) never selects.
//!
//! The principal registry is declared by the repository owner; a provider never adds itself, and
//! an empty registry means nothing may be selected.

use crate::atlas::CensusAtlas;
use crate::identity::IntegrityDigest;
use crate::semantic::SemanticDimension;
use crate::verification::{ReportVerdict, VerificationReport};
use serde::{Deserialize, Serialize};

pub const DESIGN_SCHEMA_VERSION: &str = "atlas.selected-design.v1";
pub const REGISTRY_SCHEMA_VERSION: &str = "atlas.principal-registry.v1";
/// Where a repository declares the principals that may select designs.
pub const PRINCIPAL_REGISTRY_PATH: &str = ".atlas/declared/principals.json";

crate::vocabulary_enum! {
    /// A design's lifecycle (contract: never an epistemic status).
    pub enum DesignState {
        Candidate => "CANDIDATE",
        Validated => "VALIDATED",
        Selected => "SELECTED",
        Rejected => "REJECTED",
        Superseded => "SUPERSEDED",
    }
}

crate::vocabulary_enum! {
    /// Who may select at a coordinate.
    pub enum AuthorityMode {
        HumanRequired => "HUMAN_REQUIRED",
        PolicyAuto => "POLICY_AUTO",
        Hybrid => "HYBRID",
    }
}

crate::vocabulary_enum! {
    /// What a principal is.
    pub enum PrincipalKind {
        Human => "HUMAN",
        Policy => "POLICY",
        Provider => "PROVIDER",
    }
}

crate::vocabulary_enum! {
    /// How a design binds one of its choices.
    pub enum BindingKind {
        StaticSelected => "STATIC_SELECTED",
        DynamicPermitted => "DYNAMIC_PERMITTED",
        ExternalBoundary => "EXTERNAL_BOUNDARY",
        UnresolvedBlocker => "UNRESOLVED_BLOCKER",
    }
}

crate::vocabulary_enum! {
    /// Whether a design passed its check.
    pub enum DesignVerdict {
        Accepted => "ACCEPTED",
        Rejected => "REJECTED",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Principal {
    pub id: String,
    pub kind: PrincipalKind,
}

/// The declared principals; only they may author authority events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrincipalRegistry {
    pub schema: String,
    pub principals: Vec<Principal>,
    pub note: String,
}

impl PrincipalRegistry {
    pub fn empty() -> Self {
        Self {
            schema: REGISTRY_SCHEMA_VERSION.into(),
            principals: Vec::new(),
            note: "principals are declared by the repository owner; a provider never adds itself"
                .into(),
        }
    }
}

/// One selected semantic root: a typed record of the parent container.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticRoot {
    pub dimension: SemanticDimension,
    pub record_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DesignBinding {
    pub name: String,
    pub kind: BindingKind,
    pub target: String,
}

/// A principal's decision to select exactly one design.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthorityEvent {
    /// `event_identity` of the other fields.
    pub event_id: String,
    pub principal: Principal,
    pub mode: AuthorityMode,
    pub design_id: String,
    pub generation: String,
    pub statement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectedDesign {
    pub schema: String,
    /// `design_identity` of the identity-bearing fields.
    pub design_id: String,
    /// The root identity of the verified census container the roots are in.
    pub parent_root: String,
    /// That container's census digest: the candidate the verification evidence must name.
    pub candidate: String,
    pub scope: String,
    pub target_kind: String,
    pub variant: String,
    pub state: DesignState,
    pub roots: Vec<SemanticRoot>,
    pub bindings: Vec<DesignBinding>,
    /// The design ids considered at this coordinate, this one included.
    pub candidate_set: Vec<String>,
    /// The verification evidence the design rests on (report paths or ids).
    pub evidence_refs: Vec<String>,
    pub authority_mode: AuthorityMode,
    pub authority_event: Option<AuthorityEvent>,
    pub rationale: String,
    pub supersedes: Option<String>,
    /// G150: the `DesignComparison` a selection rests on (not part of the identity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparison: Option<String>,
}

/// A design's identity: BLAKE3 over its schema, parent root, candidate, coordinate, roots and
/// bindings (sorted). State, rationale, evidence and authority are facts about the design, not
/// part of what it selects.
pub fn design_identity(design: &SelectedDesign) -> String {
    let mut text = format!(
        "{}\nparent {}\ncandidate {}\nscope {}\ntarget {}\nvariant {}\n",
        design.schema,
        design.parent_root,
        design.candidate,
        design.scope,
        design.target_kind,
        design.variant
    );
    let mut roots = design.roots.clone();
    roots.sort();
    for root in &roots {
        text.push_str(&format!(
            "root {} {}\n",
            root.dimension.as_str(),
            root.record_id
        ));
    }
    let mut bindings = design.bindings.clone();
    bindings.sort();
    for binding in &bindings {
        text.push_str(&format!(
            "binding {} {} {}\n",
            binding.name,
            binding.kind.as_str(),
            binding.target
        ));
    }
    IntegrityDigest::of_bytes(text.as_bytes())
        .as_str()
        .to_owned()
}

/// An authority event's identity: BLAKE3 over its principal, mode, design, generation and
/// statement.
pub fn event_identity(event: &AuthorityEvent) -> String {
    let text = format!(
        "atlas.authority-event.v1\nprincipal {} {}\nmode {}\ndesign {}\ngeneration {}\nstatement {}\n",
        event.principal.kind.as_str(),
        event.principal.id,
        event.mode.as_str(),
        event.design_id,
        event.generation,
        event.statement
    );
    IntegrityDigest::of_bytes(text.as_bytes())
        .as_str()
        .to_owned()
}

/// Why a design is not accepted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DesignViolation {
    pub code: String,
    pub detail: String,
}

fn violation(code: &str, detail: impl Into<String>) -> DesignViolation {
    DesignViolation {
        code: code.into(),
        detail: detail.into(),
    }
}

/// The census digest of a container, as the verification evidence names candidates.
pub fn container_candidate(container: &CensusAtlas) -> String {
    IntegrityDigest::blake3_256(&container.manifest.census_digest)
        .as_str()
        .to_owned()
}

/// Every reason `design` is not acceptable in its state over the verified `container` (whose
/// root identity is `root_id`), the verification `report` for that container's candidate, and
/// the declared `registry`. Empty means accepted.
pub fn validate(
    design: &SelectedDesign,
    container: &CensusAtlas,
    root_id: &str,
    report: Option<&VerificationReport>,
    registry: &PrincipalRegistry,
) -> Vec<DesignViolation> {
    let mut out = Vec::new();
    if design.schema != DESIGN_SCHEMA_VERSION {
        out.push(violation("SCHEMA_UNKNOWN", &design.schema));
    }
    let identity = design_identity(design);
    if design.design_id != identity {
        out.push(violation(
            "DESIGN_ID_MISMATCH",
            format!("recorded {}, computed {identity}", design.design_id),
        ));
    }
    if design.parent_root != root_id {
        out.push(violation(
            "PARENT_ROOT_MISMATCH",
            format!(
                "design names {}, container is {root_id}",
                design.parent_root
            ),
        ));
    }
    let candidate = container_candidate(container);
    if design.candidate != candidate {
        out.push(violation(
            "CANDIDATE_MISMATCH",
            format!(
                "design names {}, container is {candidate}",
                design.candidate
            ),
        ));
    }
    if design.roots.is_empty() {
        out.push(violation("NO_ROOTS", "a design selects at least one root"));
    }
    for root in &design.roots {
        let resolves = container.typed_records.iter().any(|record| {
            record.record_id().as_str() == root.record_id && record.dimension() == root.dimension
        });
        if !resolves {
            out.push(violation(
                "ROOT_UNRESOLVED",
                format!("{} {}", root.dimension.as_str(), root.record_id),
            ));
        }
    }
    if !design.candidate_set.contains(&design.design_id) {
        out.push(violation(
            "CANDIDATE_SET_OMITS_DESIGN",
            "the candidate set names the design itself",
        ));
    }
    let needs_evidence =
        design.state == DesignState::Validated || design.state == DesignState::Selected;
    if needs_evidence {
        match report {
            _ if design.evidence_refs.is_empty() => {
                out.push(violation("EVIDENCE_MISSING", "no evidence reference"));
            }
            None => out.push(violation("EVIDENCE_MISSING", "no verification report")),
            Some(r) if r.candidate != candidate => out.push(violation(
                "EVIDENCE_OTHER_CANDIDATE",
                format!("the report verifies {}", r.candidate),
            )),
            Some(r) if r.verdict != ReportVerdict::Admissible => {
                out.push(violation("EVIDENCE_BLOCKED", r.blockers.join("; ")));
            }
            Some(_) => {}
        }
    }
    let selected = design.state == DesignState::Selected;
    if !selected && design.authority_event.is_some() {
        out.push(violation(
            "AUTHORITY_EVENT_UNEXPECTED",
            "only a SELECTED design carries a selection event",
        ));
    }
    if selected {
        for binding in &design.bindings {
            if binding.kind == BindingKind::UnresolvedBlocker {
                out.push(violation("UNRESOLVED_BLOCKER", &binding.name));
            }
        }
        if design.comparison.is_none() {
            out.push(violation(
                "COMPARISON_MISSING",
                "a SELECTED design cites the comparison of its candidates",
            ));
        }
        match &design.authority_event {
            None => out.push(violation(
                "AUTHORITY_EVENT_MISSING",
                "a SELECTED design needs a selection authority event",
            )),
            Some(event) => authority(design, event, registry, &mut out),
        }
    }
    out.sort();
    out
}

fn authority(
    design: &SelectedDesign,
    event: &AuthorityEvent,
    registry: &PrincipalRegistry,
    out: &mut Vec<DesignViolation>,
) {
    if event.design_id != design.design_id {
        out.push(violation(
            "AUTHORITY_EVENT_OTHER_DESIGN",
            format!("the event selects {}", event.design_id),
        ));
    }
    if event.mode != design.authority_mode {
        out.push(violation(
            "AUTHORITY_MODE_MISMATCH",
            format!(
                "event {}, design {}",
                event.mode.as_str(),
                design.authority_mode.as_str()
            ),
        ));
    }
    if event.event_id != event_identity(event) {
        out.push(violation("AUTHORITY_EVENT_ID_MISMATCH", &event.event_id));
    }
    if !registry.principals.contains(&event.principal) {
        out.push(violation(
            "PRINCIPAL_UNREGISTERED",
            format!("{} {}", event.principal.kind.as_str(), event.principal.id),
        ));
    }
    match (event.principal.kind, design.authority_mode) {
        (PrincipalKind::Provider, _) => out.push(violation(
            "PRINCIPAL_IS_PROVIDER",
            "a provider never selects",
        )),
        (_, AuthorityMode::PolicyAuto) => out.push(violation(
            "POLICY_ENVELOPE_UNDEFINED",
            "no bounded policy envelope is defined for POLICY_AUTO",
        )),
        (PrincipalKind::Human, _) => {}
        (kind, mode) => out.push(violation(
            "PRINCIPAL_KIND_INADMISSIBLE",
            format!("{} under {}", kind.as_str(), mode.as_str()),
        )),
    }
}

/// At most one non-superseded SELECTED design per coordinate (scope, target kind, variant).
pub fn coordinate_conflicts(designs: &[SelectedDesign]) -> Vec<DesignViolation> {
    let superseded: std::collections::BTreeSet<&str> = designs
        .iter()
        .filter_map(|d| d.supersedes.as_deref())
        .collect();
    let mut seen: std::collections::BTreeMap<(&str, &str, &str), &str> = Default::default();
    let mut out = Vec::new();
    for design in designs {
        if design.state != DesignState::Selected || superseded.contains(design.design_id.as_str()) {
            continue;
        }
        let coordinate = (
            design.scope.as_str(),
            design.target_kind.as_str(),
            design.variant.as_str(),
        );
        if let Some(other) = seen.insert(coordinate, design.design_id.as_str()) {
            out.push(violation(
                "COORDINATE_CONFLICT",
                format!("{other} and {} are both SELECTED", design.design_id),
            ));
        }
    }
    out
}

pub mod compare;

#[cfg(test)]
mod tests;
