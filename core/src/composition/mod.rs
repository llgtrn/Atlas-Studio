//! Semantic composition (G122, ADR 0044): the census's typed records composed into larger semantic
//! objects -- function behavior, component behavior, subsystem model, capability model and
//! architecture model -- with typed relations, typed invariants and explicit understanding gaps.
//!
//! Every composed claim carries an `EpistemicStatus` and the record ids (or declared-fact spans) it
//! rests on. Composition never names a purpose, a cause or an owner the records do not support:
//! purpose comes only from a declaration (ADL `responsibility`), causality is never claimed, and a
//! universal claim ("written only by") whose dimension is not OBSERVED on every artifact in scope is
//! INFERRED with its residual named. What composition cannot represent is recorded as an
//! `UnderstandingGap` routed to the essential debt that owns it.
//!
//! Pure: it reads a `SystemizeReport` and returns a `WorldModel`; the agent-facing operations over
//! the model live in `lens`. See `.atlas/contracts/AGENT-WORN-ATLAS.md`.

pub mod closure;
pub mod lens;
pub mod quantified;

use crate::language::adl::ConstraintCheckKind;
use crate::semantic::call::callee_name;
use crate::semantic::{
    ControlFlowEdgeKind, PlaceRef, SemanticDimension, SemanticObservation, SemanticRecordId,
    StateAccessKind,
};
use crate::{ConstraintVerdict, EpistemicStatus, SystemizeReport};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const WORLD_MODEL_SCHEMA: &str = "atlas.world-model.v1";

/// Effect categories that exercise authority over the world outside the process.
pub const AUTHORITY_CATEGORIES: [&str; 8] = [
    "FILESYSTEM_READ",
    "FILESYSTEM_WRITE",
    "PROCESS_SPAWN",
    "ENVIRONMENT_READ",
    "CLOCK_READ",
    "NETWORK_SEND",
    "NETWORK_RECEIVE",
    "FFI_CALL",
];

/// Strength order of epistemic statuses: lower is stronger evidence.
pub const fn status_rank(status: EpistemicStatus) -> u8 {
    match status {
        EpistemicStatus::Observed => 0,
        EpistemicStatus::Declared => 1,
        EpistemicStatus::Derived => 2,
        EpistemicStatus::Inferred => 3,
        EpistemicStatus::Hypothesis => 4,
        EpistemicStatus::Simulated => 5,
        EpistemicStatus::Conflict => 6,
        EpistemicStatus::Unknown => 7,
        EpistemicStatus::Unsupported => 8,
        EpistemicStatus::Ignored => 9,
    }
}

fn stronger(a: EpistemicStatus, b: EpistemicStatus) -> EpistemicStatus {
    if status_rank(b) < status_rank(a) {
        b
    } else {
        a
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldModel {
    pub schema: String,
    pub revision: String,
    pub system: Option<String>,
    pub functions: Vec<FunctionBehavior>,
    pub components: Vec<ComponentBehavior>,
    pub subsystems: Vec<SubsystemModel>,
    pub capabilities: Vec<CapabilityModel>,
    pub state: Vec<StateVariable>,
    pub architecture: ArchitectureModel,
    pub relations: Vec<Relation>,
    pub invariants: Vec<Invariant>,
    pub gaps: Vec<UnderstandingGap>,
    pub accounting: CompositionAccounting,
}

/// A value with its epistemic status and the evidence it rests on.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claim {
    pub value: Option<String>,
    pub status: EpistemicStatus,
    pub evidence: Vec<String>,
    /// Why the status is not stronger (empty when nothing is missing).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub basis: String,
}

impl Claim {
    /// The purpose of a function with no `///` documentation (G132); also the purpose a model
    /// written before G132 reads back with.
    fn undocumented_function() -> Self {
        Self::unknown(
            "no documentation (///) on the function; purpose is never named from identifiers",
        )
    }

    fn unknown(basis: &str) -> Self {
        Self {
            value: None,
            status: EpistemicStatus::Unknown,
            evidence: Vec::new(),
            basis: basis.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Port {
    pub name: String,
    pub type_spelling: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Site {
    pub kind: String,
    pub line: usize,
    pub status: EpistemicStatus,
    pub records: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateAccess {
    /// `StateVariable.key` of the accessed state.
    pub state: String,
    pub kind: String,
    pub line: usize,
    pub status: EpistemicStatus,
    pub record: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataFlowSummary {
    pub definitions: usize,
    pub uses: usize,
    pub stores: usize,
    pub parameter_flows: usize,
    pub return_flows: usize,
    pub unresolved: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ControlSummary {
    pub blocks: usize,
    pub decision_blocks: usize,
    pub panic_exits: usize,
    pub return_exits: usize,
}

/// Level 1 of the hierarchy: what one function does, composed from every dimension's records.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionBehavior {
    /// The FUNCTION_IDENTITY record id.
    pub id: String,
    pub name: String,
    pub path: String,
    pub line: usize,
    /// In-file scope (`impl:Store`, `tests`, ...).
    pub scope: String,
    pub kind: String,
    pub owner_type: Option<String>,
    pub subsystem: Option<String>,
    pub visibility: Option<String>,
    pub is_unsafe: bool,
    pub is_async: bool,
    pub inputs: Vec<Port>,
    pub output: Option<String>,
    /// INFERRED: an in-file scope segment `tests` or a `tests.rs` file (test attributes are not
    /// censused).
    pub test_scope: bool,
    /// Resolved callees (FUNCTION_IDENTITY ids), DERIVED by the resolution engine.
    pub calls: Vec<String>,
    /// Call sites no engine resolved: their callee is UNKNOWN.
    pub unresolved_calls: usize,
    /// The unresolved call sites by the name their callee is spelled with (OBSERVED syntax): a
    /// site spelled `.bump()` may reach any function named `bump` (G123). Defaulted so a model
    /// written by an earlier Atlas stays readable (`verify` compares across versions).
    #[serde(default)]
    pub unresolved_callee_names: BTreeMap<String, usize>,
    /// Unresolved call sites whose callee is not a path or method name (closures, function
    /// pointers): they may reach any function.
    #[serde(default)]
    pub unnamed_unresolved_calls: usize,
    /// Functions whose resolved calls reach this one.
    pub callers: Vec<String>,
    pub effects: Vec<Site>,
    pub state: Vec<StateAccess>,
    pub ownership: BTreeMap<String, usize>,
    pub concurrency: Vec<Site>,
    pub persistence: Vec<Site>,
    pub data_flow: DataFlowSummary,
    pub control: ControlSummary,
    /// Raw typed records composed into this behavior.
    pub records: usize,
    /// G132 (mission M3): the function's own documentation (`///`) as DECLARED purpose, citing
    /// its FUNCTION_IDENTITY record; UNKNOWN without one.
    #[serde(default = "Claim::undocumented_function")]
    pub purpose: Claim,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EffectTally {
    pub sites: usize,
    pub status: Option<EpistemicStatus>,
    pub functions: Vec<String>,
}

/// Level 2: one source artifact (a module file).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComponentBehavior {
    pub path: String,
    /// DERIVED from the Rust module-file convention (`#[path]` overrides are not censused).
    pub module: Option<String>,
    pub subsystem: Option<String>,
    pub language: Option<String>,
    pub functions: Vec<String>,
    pub public_interface: Vec<String>,
    pub test_functions: usize,
    pub effects: BTreeMap<String, EffectTally>,
    pub state_written: Vec<String>,
    pub state_read: Vec<String>,
    /// Components this one invokes through resolved calls, with the call count.
    pub depends_on: BTreeMap<String, usize>,
    pub used_by: BTreeMap<String, usize>,
    pub unresolved_calls: usize,
    /// Per dimension, the best engine's obligation status on this artifact.
    pub coverage: BTreeMap<String, EpistemicStatus>,
    pub purpose: Claim,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverageTally {
    pub observed: usize,
    pub unknown: usize,
    pub unsupported: usize,
}

/// Level 3: a declared subsystem (an ADL entity with a materialization path).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubsystemModel {
    pub name: String,
    pub root: String,
    pub declared_at: String,
    pub node_kind: String,
    pub responsibility: Claim,
    pub attributes: BTreeMap<String, String>,
    pub capabilities: Vec<String>,
    pub components: Vec<String>,
    pub functions: usize,
    pub public_functions: usize,
    pub unsafe_functions: usize,
    /// Functions of this subsystem that other subsystems reach by resolved calls.
    pub interface: Vec<String>,
    pub effects: BTreeMap<String, EffectTally>,
    /// State written by functions of this subsystem and by no function elsewhere (as observed).
    pub state_owned: Vec<String>,
    pub unresolved_calls: usize,
    pub coverage: BTreeMap<String, CoverageTally>,
}

/// Level 4: a declared capability and what is known about its realization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityModel {
    pub name: String,
    pub declared_at: String,
    pub input: Option<String>,
    pub output: Option<String>,
    pub provided_by: Vec<String>,
    pub consumed_by: Vec<String>,
    /// Which functions realize the capability. UNKNOWN until a declaration or a derivation maps
    /// the capability to code.
    pub realized_by: Claim,
}

/// A piece of state: a field of an owner type, with who writes and reads it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateVariable {
    /// `<self type>.<field>`, e.g. `Store.count` (G123: keyed by the impl's self type).
    pub key: String,
    pub owner: String,
    pub field: String,
    pub writers: Vec<String>,
    pub readers: Vec<String>,
    pub subsystems: Vec<String>,
    pub records: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
    pub declared: Option<String>,
    pub cargo: bool,
    pub resolved_calls: usize,
    pub verdict: String,
    pub status: EpistemicStatus,
}

/// Level 5: the system as declared and as observed, reconciled.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArchitectureModel {
    pub subsystems: Vec<String>,
    pub dependencies: Vec<DependencyEdge>,
    /// Cargo members seen only as providers: their directory is not evidenced, so their Cargo
    /// edges cannot be attributed to a subsystem.
    pub unplaced_members: Vec<String>,
    pub unassigned_components: Vec<String>,
}

/// A typed relation. Never a generic edge: `kind` says exactly what was observed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationKind {
    /// `from` invokes `to` through a resolved call site.
    Invokes,
    /// A resolved argument or return value carries data from `from` to `to` at a resolved call.
    SuppliesData,
    /// `from` writes state that `to` reads (no ordering between them is claimed).
    StateFlow,
}

impl RelationKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Invokes => "INVOKES",
            Self::SuppliesData => "SUPPLIES_DATA",
            Self::StateFlow => "STATE_FLOW",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Relation {
    pub kind: RelationKind,
    pub from: String,
    pub to: String,
    pub status: EpistemicStatus,
    pub weight: usize,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InvariantKind {
    State,
    Ordering,
    Dependency,
    Authority,
    Resource,
    Safety,
    Construction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Invariant {
    pub id: String,
    pub kind: InvariantKind,
    pub statement: String,
    /// Subsystem names (or `*`) the invariant ranges over.
    pub scope: Vec<String>,
    pub status: EpistemicStatus,
    pub evidence: Vec<String>,
    /// What keeps a universal claim from being stronger.
    pub residual: Vec<String>,
}

/// Something the agent needs that Atlas cannot represent yet, routed to the debt that owns it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnderstandingGap {
    pub id: String,
    pub question_class: String,
    pub missing: String,
    pub magnitude: usize,
    pub debt: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompositionAccounting {
    pub records: BTreeMap<String, usize>,
    pub obligations: usize,
    pub declared_facts: usize,
    pub composed_objects: BTreeMap<String, usize>,
}

impl CompositionAccounting {
    pub fn raw_records(&self) -> usize {
        self.records.values().sum()
    }
}

/// The self type a `self.field` access belongs to, from the owning impl scope: `impl:Store`,
/// `impl:Visit<'ast> for Opaque<'_>` and `impl:Opaque<'_>` name `Store`, `Opaque`, `Opaque` (G123:
/// one field is one piece of state whichever impl block touches it). Same-named types of different
/// modules share a key: state identity is by spelling (see AGENT-WORN-ATLAS.md).
pub fn self_type_of(scope: &str) -> String {
    let last = scope.rsplit("::").next().unwrap_or(scope);
    let impl_target = last.strip_prefix("impl:").unwrap_or(last);
    let self_type = match impl_target.rsplit_once(" for ") {
        Some((_, target)) => target,
        None => impl_target,
    };
    let mut depth = 0usize;
    let mut base = String::new();
    for c in self_type.chars() {
        match c {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            c if depth == 0 => base.push(c),
            _ => {}
        }
    }
    let base = base.trim().trim_start_matches('&').trim();
    base.rsplit("::").next().unwrap_or(base).to_owned()
}

/// The module path of a Rust source file under a crate root (`core/src/census/delta.rs` under
/// `core` -> `core::census::delta`).
fn module_of(path: &str, root: &str, crate_name: &str) -> Option<String> {
    let rest = path.strip_prefix(root)?.strip_prefix("/src/")?;
    let rest = rest.strip_suffix(".rs")?;
    let mut segments: Vec<&str> = rest.split('/').collect();
    if matches!(segments.last(), Some(&"mod" | &"lib" | &"main")) {
        segments.pop();
    }
    let mut module = crate_name.replace('-', "_");
    for segment in segments {
        module.push_str("::");
        module.push_str(segment);
    }
    Some(module)
}

struct Materialized<'a> {
    name: &'a str,
    root: &'a str,
}

fn subsystem_of<'a>(materialized: &[Materialized<'a>], path: &str) -> Option<&'a str> {
    materialized
        .iter()
        .filter(|m| path == m.root || path.starts_with(&format!("{}/", m.root)))
        .max_by_key(|m| m.root.len())
        .map(|m| m.name)
}

fn push_unique(list: &mut Vec<String>, value: &str) {
    if !list.iter().any(|v| v == value) {
        list.push(value.to_owned());
    }
}

/// Compose a census report into a `WorldModel`. Deterministic: the same report gives the same
/// model, field for field.
pub fn compose(report: &SystemizeReport) -> WorldModel {
    compose_input(CompositionInput {
        census: &report.census,
        inventory: &report.inventory,
        adl: &report.adl,
        dependency_closure: &report.dependency_closure,
        revision: &report.snapshot.head_sha,
    })
}

/// What composition reads: the census, the inventory, the compiled ADL, the dependency closure and
/// the revision
/// (G130: `systemize` composes from these before its report exists, to decide census-quantified
/// invariants).
pub struct CompositionInput<'a> {
    pub census: &'a crate::schema::CensusReport,
    pub inventory: &'a crate::census::InventoryReport,
    pub adl: &'a crate::language::adl::AdlCompileReport,
    pub dependency_closure: &'a crate::census::DependencyClosureReport,
    pub revision: &'a str,
}

/// `compose` over its inputs.
pub fn compose_input(input: CompositionInput<'_>) -> WorldModel {
    let census = &input.census;
    let declared = &input.adl.ir.declared;
    let mut accounting = CompositionAccounting::default();

    // Subsystems are declared: an ADL entity with a materialization path.
    let nodes: BTreeMap<&str, &crate::language::adl::DeclaredNode> = declared
        .nodes
        .iter()
        .map(|n| (n.name.as_str(), n))
        .collect();
    let materialized: Vec<Materialized> = declared
        .materializations
        .iter()
        .map(|m| Materialized {
            name: m.target.as_str(),
            root: m.path.trim_end_matches('/'),
        })
        .collect();
    accounting.declared_facts = declared.nodes.len()
        + declared.edges.len()
        + declared.bindings.len()
        + declared.materializations.len()
        + input.adl.constraint_results.len();

    // Level 1: functions.
    let mut functions: BTreeMap<String, FunctionBehavior> = BTreeMap::new();
    for record in &census.typed_semantic_records {
        *accounting
            .records
            .entry(record.dimension().as_str().to_owned())
            .or_default() += 1;
        if let SemanticObservation::FunctionIdentity(header) = record {
            let f = &header.subject;
            let path = f.span.path.clone();
            let scope = f.scope.join();
            let test_scope = f.scope.segments.iter().any(|s| s == "tests")
                || path.ends_with("/tests.rs")
                || path.contains("/tests/");
            functions
                .entry(header.record_id.as_str().to_owned())
                .or_insert_with(|| FunctionBehavior {
                    id: header.record_id.as_str().to_owned(),
                    name: f.symbol.name.clone(),
                    subsystem: subsystem_of(&materialized, &path).map(str::to_owned),
                    path,
                    line: f.span.line,
                    scope,
                    kind: f.declaration_kind.as_str().to_owned(),
                    owner_type: f.owner.target.as_ref().map(|t| t.name.clone()),
                    visibility: None,
                    is_unsafe: false,
                    is_async: false,
                    inputs: Vec::new(),
                    output: None,
                    test_scope,
                    calls: Vec::new(),
                    unresolved_calls: 0,
                    unresolved_callee_names: BTreeMap::new(),
                    unnamed_unresolved_calls: 0,
                    callers: Vec::new(),
                    effects: Vec::new(),
                    state: Vec::new(),
                    ownership: BTreeMap::new(),
                    concurrency: Vec::new(),
                    persistence: Vec::new(),
                    data_flow: DataFlowSummary::default(),
                    control: ControlSummary::default(),
                    records: 0,
                    purpose: match &f.symbol.documentation {
                        Some(documentation) => Claim {
                            value: Some(documentation.summary.clone()),
                            status: EpistemicStatus::Declared,
                            evidence: vec![header.record_id.as_str().to_owned()],
                            basis: "the function's own documentation (///): its author's \
                                    statement, not checked against behavior"
                                .into(),
                        },
                        None => Claim::undocumented_function(),
                    },
                })
                .records += 1;
        }
    }

    // Calls: one call site may carry a syntactic record and a resolution record under the same
    // record id; the site is resolved when any engine names its callees.
    #[derive(Default)]
    struct CallSite {
        function: String,
        spelling: Option<String>,
        callees: BTreeSet<String>,
        supplies: bool,
        returns: bool,
        records: usize,
    }
    let mut call_sites: BTreeMap<String, CallSite> = BTreeMap::new();
    let mut effect_sites: BTreeMap<(String, String, usize), Site> = BTreeMap::new();
    let mut state_vars: BTreeMap<String, StateVariable> = BTreeMap::new();

    for record in &census.typed_semantic_records {
        match record {
            SemanticObservation::FunctionSignature(header) => {
                let s = &header.subject;
                let id = SemanticRecordId::new(
                    SemanticDimension::FunctionIdentity,
                    &s.function.identity_key(),
                );
                if let Some(f) = functions.get_mut(id.as_str()) {
                    f.visibility = Some(s.visibility.clone());
                    f.is_unsafe = s.is_unsafe;
                    f.is_async = s.is_async;
                    f.inputs = s
                        .parameters
                        .iter()
                        .map(|p| Port {
                            name: p.name.clone(),
                            type_spelling: p.type_identity.name.clone(),
                        })
                        .collect();
                    f.output = s.return_type.as_ref().map(|t| t.name.clone());
                    f.records += 1;
                }
            }
            SemanticObservation::Call(header) => {
                let c = &header.subject;
                let site = call_sites
                    .entry(header.record_id.as_str().to_owned())
                    .or_default();
                site.function = c.function.as_str().to_owned();
                site.records += 1;
                if site.spelling.is_none() {
                    site.spelling = c.callee_spelling.clone();
                }
                if !c.callees.is_empty() {
                    site.callees
                        .extend(c.callees.iter().map(|id| id.as_str().to_owned()));
                    site.supplies |= c
                        .arguments
                        .iter()
                        .any(|a| matches!(a, PlaceRef::Resolved { .. }));
                    site.returns |= matches!(c.result, PlaceRef::Resolved { .. });
                }
            }
            SemanticObservation::Effect(header) => {
                let e = &header.subject;
                let key = (
                    e.function.as_str().to_owned(),
                    e.category.as_str().to_owned(),
                    e.span.line,
                );
                let site = effect_sites.entry(key).or_insert_with(|| Site {
                    kind: e.category.as_str().to_owned(),
                    line: e.span.line,
                    status: header.status,
                    records: Vec::new(),
                });
                site.status = stronger(site.status, header.status);
                site.records.push(header.record_id.as_str().to_owned());
            }
            SemanticObservation::State(header) => {
                let s = &header.subject;
                let owner = self_type_of(&s.scope.join());
                let key = format!("{owner}.{}", s.name);
                let var = state_vars
                    .entry(key.clone())
                    .or_insert_with(|| StateVariable {
                        key: key.clone(),
                        owner,
                        field: s.name.clone(),
                        writers: Vec::new(),
                        readers: Vec::new(),
                        subsystems: Vec::new(),
                        records: 0,
                    });
                var.records += 1;
                let function = s.function.as_str();
                if matches!(s.kind, StateAccessKind::Read) {
                    push_unique(&mut var.readers, function);
                } else {
                    push_unique(&mut var.writers, function);
                }
                if let Some(f) = functions.get_mut(function) {
                    if let Some(subsystem) = &f.subsystem {
                        push_unique(&mut var.subsystems, subsystem);
                    }
                    f.state.push(StateAccess {
                        state: key,
                        kind: s.kind.as_str().to_owned(),
                        line: s.span.line,
                        status: header.status,
                        record: header.record_id.as_str().to_owned(),
                    });
                    f.records += 1;
                }
            }
            SemanticObservation::Ownership(header) => {
                if let Some(f) = functions.get_mut(header.subject.function.as_str()) {
                    *f.ownership
                        .entry(header.subject.kind.as_str().to_owned())
                        .or_default() += 1;
                    f.records += 1;
                }
            }
            SemanticObservation::Concurrency(header) => {
                if let Some(f) = functions.get_mut(header.subject.function.as_str()) {
                    f.concurrency.push(Site {
                        kind: header.subject.kind.as_str().to_owned(),
                        line: header.subject.span.line,
                        status: header.status,
                        records: vec![header.record_id.as_str().to_owned()],
                    });
                    f.records += 1;
                }
            }
            SemanticObservation::Persistence(header) => {
                if let Some(f) = functions.get_mut(header.subject.function.as_str()) {
                    f.persistence.push(Site {
                        kind: header.subject.kind.as_str().to_owned(),
                        line: header.subject.span.line,
                        status: header.status,
                        records: vec![header.record_id.as_str().to_owned()],
                    });
                    f.records += 1;
                }
            }
            SemanticObservation::DataFlow(header) => {
                let v = &header.subject;
                if let Some(f) = functions.get_mut(v.function.as_str()) {
                    let d = &mut f.data_flow;
                    match v.role {
                        crate::semantic::ValueRole::Definition => d.definitions += 1,
                        crate::semantic::ValueRole::Use => d.uses += 1,
                        crate::semantic::ValueRole::Store => d.stores += 1,
                    }
                    d.parameter_flows += usize::from(v.is_parameter);
                    d.return_flows += usize::from(v.is_return_flow);
                    d.unresolved += usize::from(matches!(
                        v.resolution,
                        crate::semantic::DataFlowResolution::Unresolved
                    ));
                    f.records += 1;
                }
            }
            SemanticObservation::ControlFlow(header) => {
                let b = &header.subject;
                if let Some(f) = functions.get_mut(b.function.as_str()) {
                    let c = &mut f.control;
                    c.blocks += 1;
                    c.decision_blocks += usize::from(b.successors.len() > 1);
                    c.panic_exits += b
                        .successors
                        .iter()
                        .filter(|e| matches!(e.kind, ControlFlowEdgeKind::Panic))
                        .count();
                    c.return_exits += b
                        .successors
                        .iter()
                        .filter(|e| matches!(e.kind, ControlFlowEdgeKind::Return))
                        .count();
                    f.records += 1;
                }
            }
            _ => {}
        }
    }
    for ((function, _, _), mut site) in effect_sites {
        site.records.sort();
        if let Some(f) = functions.get_mut(&function) {
            f.records += site.records.len();
            f.effects.push(site);
        }
    }

    // Relations from call sites.
    let mut relations: BTreeMap<(RelationKind, String, String), Relation> = BTreeMap::new();
    let mut relate = |kind: RelationKind, from: &str, to: &str, evidence: &str| {
        let relation = relations
            .entry((kind, from.to_owned(), to.to_owned()))
            .or_insert_with(|| Relation {
                kind,
                from: from.to_owned(),
                to: to.to_owned(),
                status: EpistemicStatus::Derived,
                weight: 0,
                evidence: Vec::new(),
            });
        relation.weight += 1;
        relation.evidence.push(evidence.to_owned());
    };
    for (id, site) in &call_sites {
        if site.callees.is_empty() {
            if let Some(f) = functions.get_mut(&site.function) {
                f.unresolved_calls += 1;
                match site.spelling.as_deref().and_then(callee_name) {
                    Some(name) => {
                        *f.unresolved_callee_names
                            .entry(name.to_owned())
                            .or_default() += 1
                    }
                    None => f.unnamed_unresolved_calls += 1,
                }
                f.records += site.records;
            }
            continue;
        }
        if let Some(f) = functions.get_mut(&site.function) {
            f.records += site.records;
        }
        for callee in &site.callees {
            relate(RelationKind::Invokes, &site.function, callee, id);
            if site.supplies {
                relate(RelationKind::SuppliesData, &site.function, callee, id);
            }
            if site.returns {
                relate(RelationKind::SuppliesData, callee, &site.function, id);
            }
        }
    }
    for var in state_vars.values() {
        for writer in &var.writers {
            for reader in &var.readers {
                if writer != reader {
                    relate(RelationKind::StateFlow, writer, reader, &var.key);
                }
            }
        }
    }
    let relations: Vec<Relation> = relations.into_values().collect();
    for relation in &relations {
        if relation.kind != RelationKind::Invokes {
            continue;
        }
        if let Some(f) = functions.get_mut(&relation.from) {
            push_unique(&mut f.calls, &relation.to);
        }
        if let Some(f) = functions.get_mut(&relation.to) {
            push_unique(&mut f.callers, &relation.from);
        }
    }
    for f in functions.values_mut() {
        f.calls.sort();
        f.callers.sort();
        f.effects
            .sort_by(|a, b| (a.line, &a.kind).cmp(&(b.line, &b.kind)));
        f.state
            .sort_by(|a, b| (a.line, &a.state, &a.kind).cmp(&(b.line, &b.state, &b.kind)));
        f.concurrency
            .sort_by(|a, b| (a.line, &a.kind).cmp(&(b.line, &b.kind)));
        f.persistence
            .sort_by(|a, b| (a.line, &a.kind).cmp(&(b.line, &b.kind)));
    }
    for var in state_vars.values_mut() {
        var.writers.sort();
        var.readers.sort();
        var.subsystems.sort();
    }

    // Level 2: components.
    let artifact_paths: BTreeMap<&str, &crate::census::ArtifactRecord> = input
        .inventory
        .artifacts
        .iter()
        .map(|a| (a.id.as_str(), a))
        .collect();
    let mut components: BTreeMap<String, ComponentBehavior> = BTreeMap::new();
    // Workspace members whose manifest was read have an evidenced directory; a member seen only
    // as a provider stays unplaced (`ObservedArchitecture`), never placed by its name.
    let observed = crate::ObservedArchitecture::from_closure(input.dependency_closure);
    let crate_names: BTreeMap<String, String> = observed
        .members
        .iter()
        .map(|m| (m.dir.clone(), m.package.clone()))
        .collect();
    let new_component = |path: &str, language: Option<String>| {
        let subsystem = subsystem_of(&materialized, path).map(str::to_owned);
        let module = materialized
            .iter()
            .filter(|m| path.starts_with(&format!("{}/", m.root)))
            .max_by_key(|m| m.root.len())
            .and_then(|m| {
                let crate_name = crate_names.get(m.root)?;
                module_of(path, m.root, crate_name)
            });
        ComponentBehavior {
            path: path.to_owned(),
            module,
            subsystem,
            language,
            functions: Vec::new(),
            public_interface: Vec::new(),
            test_functions: 0,
            effects: BTreeMap::new(),
            state_written: Vec::new(),
            state_read: Vec::new(),
            depends_on: BTreeMap::new(),
            used_by: BTreeMap::new(),
            unresolved_calls: 0,
            coverage: BTreeMap::new(),
            purpose: Claim::unknown(
                "no module documentation (//! in the file, /// on its mod item); purpose is never \
                 named from identifiers",
            ),
        }
    };
    for obligation in &census.typed_obligations {
        accounting.obligations += 1;
        let Some(artifact) = artifact_paths.get(obligation.artifact.as_str()) else {
            continue;
        };
        let component = components
            .entry(artifact.path.clone())
            .or_insert_with(|| new_component(&artifact.path, artifact.language.clone()));
        let dimension = obligation.dimension.as_str().to_owned();
        let status = component
            .coverage
            .get(&dimension)
            .map_or(obligation.status, |s| stronger(*s, obligation.status));
        component.coverage.insert(dimension, status);
    }
    for f in functions.values() {
        let component = components
            .entry(f.path.clone())
            .or_insert_with(|| new_component(&f.path, Some("rust".into())));
        component.functions.push(f.id.clone());
        if f.visibility.as_deref() == Some("pub") {
            component.public_interface.push(f.id.clone());
        }
        component.test_functions += usize::from(f.test_scope);
        component.unresolved_calls += f.unresolved_calls;
        for effect in &f.effects {
            let tally = component.effects.entry(effect.kind.clone()).or_default();
            tally.sites += 1;
            tally.status = Some(
                tally
                    .status
                    .map_or(effect.status, |s| stronger(s, effect.status)),
            );
            push_unique(&mut tally.functions, &f.id);
        }
        for access in &f.state {
            if access.kind == "READ" {
                push_unique(&mut component.state_read, &access.state);
            } else {
                push_unique(&mut component.state_written, &access.state);
            }
        }
    }
    let function_path: BTreeMap<&str, &str> = functions
        .values()
        .map(|f| (f.id.as_str(), f.path.as_str()))
        .collect();
    let mut component_edges: BTreeMap<(String, String), usize> = BTreeMap::new();
    for relation in &relations {
        if relation.kind != RelationKind::Invokes {
            continue;
        }
        if let (Some(from), Some(to)) = (
            function_path.get(relation.from.as_str()),
            function_path.get(relation.to.as_str()),
        ) && from != to
        {
            *component_edges
                .entry(((*from).to_owned(), (*to).to_owned()))
                .or_default() += relation.weight;
        }
    }
    for ((from, to), calls) in &component_edges {
        if let Some(c) = components.get_mut(from) {
            c.depends_on.insert(to.clone(), *calls);
        }
        if let Some(c) = components.get_mut(to) {
            c.used_by.insert(from.clone(), *calls);
        }
    }
    for c in components.values_mut() {
        c.state_read.sort();
        c.state_written.sort();
    }

    // G128 (mission M2): a component's purpose is the documentation its author wrote -- the
    // file's own `//!` text (a `self` definition at its root scope) or, failing that, the `///`
    // text on the `mod` item that declares it. DECLARED: the author's statement, never checked
    // against the behavior; never named from identifiers.
    let module_component: BTreeMap<String, String> = components
        .values()
        .filter_map(|c| Some((c.module.clone()?, c.path.clone())))
        .collect();
    let mut declared_by_parent: Vec<(String, String, String)> = Vec::new();
    for record in &census.typed_semantic_records {
        let SemanticObservation::Symbol(header) = record else {
            continue;
        };
        let symbol = &header.subject;
        let Some(documentation) = &symbol.documentation else {
            continue;
        };
        let evidence = header.record_id.as_str().to_owned();
        if symbol.name == "self" && symbol.scope.segments.is_empty() {
            if let Some(c) = components.get_mut(&symbol.path) {
                c.purpose = Claim {
                    value: Some(documentation.summary.clone()),
                    status: EpistemicStatus::Declared,
                    evidence: vec![evidence],
                    basis: "the module's own documentation (//!): its author's statement, not \
                            checked against behavior"
                        .into(),
                };
            }
            continue;
        }
        let Some(parent) = components.get(&symbol.path).and_then(|c| c.module.clone()) else {
            continue;
        };
        let mut module = parent;
        for segment in symbol.scope.segments.iter().chain([&symbol.name]) {
            module.push_str("::");
            module.push_str(segment);
        }
        if let Some(path) = module_component.get(&module) {
            declared_by_parent.push((path.clone(), documentation.summary.clone(), evidence));
        }
    }
    for (path, summary, evidence) in declared_by_parent {
        let c = components.get_mut(&path).expect("mapped from a component");
        if c.purpose.status == EpistemicStatus::Unknown {
            c.purpose = Claim {
                value: Some(summary),
                status: EpistemicStatus::Declared,
                evidence: vec![evidence],
                basis: "the documentation (///) on the mod item declaring this module file: its \
                        author's statement, not checked against behavior"
                    .into(),
            };
        }
    }

    // Level 3: subsystems.
    let function_subsystem: BTreeMap<&str, Option<&str>> = functions
        .values()
        .map(|f| (f.id.as_str(), f.subsystem.as_deref()))
        .collect();
    let mut subsystem_calls: BTreeMap<(String, String), (usize, Vec<String>)> = BTreeMap::new();
    let mut interface: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for relation in &relations {
        if relation.kind != RelationKind::Invokes {
            continue;
        }
        let from = function_subsystem
            .get(relation.from.as_str())
            .copied()
            .flatten();
        let to = function_subsystem
            .get(relation.to.as_str())
            .copied()
            .flatten();
        if let (Some(from), Some(to)) = (from, to)
            && from != to
        {
            let entry = subsystem_calls
                .entry((from.to_owned(), to.to_owned()))
                .or_default();
            entry.0 += relation.weight;
            entry.1.extend(relation.evidence.iter().cloned());
            interface
                .entry(to.to_owned())
                .or_default()
                .insert(relation.to.clone());
        }
    }
    let provides: Vec<(&str, &str)> = declared
        .edges
        .iter()
        .filter(|e| e.relation == "provides")
        .map(|e| (e.from.as_str(), e.to.as_str()))
        .collect();
    let mut subsystems: Vec<SubsystemModel> = Vec::new();
    for m in &materialized {
        let node = nodes.get(m.name);
        let attributes = node.map(|n| n.attributes.clone()).unwrap_or_default();
        let declared_at = node
            .map(|n| format!("{}:{}", n.span.path, n.span.line))
            .unwrap_or_default();
        let responsibility = match attributes.get("responsibility") {
            Some(value) => Claim {
                value: Some(value.clone()),
                status: EpistemicStatus::Declared,
                evidence: vec![declared_at.clone()],
                basis: String::new(),
            },
            None => Claim::unknown("no declared responsibility"),
        };
        let members: Vec<&FunctionBehavior> = functions
            .values()
            .filter(|f| f.subsystem.as_deref() == Some(m.name))
            .collect();
        let mut effects: BTreeMap<String, EffectTally> = BTreeMap::new();
        for f in &members {
            for effect in &f.effects {
                let tally = effects.entry(effect.kind.clone()).or_default();
                tally.sites += 1;
                tally.status = Some(
                    tally
                        .status
                        .map_or(effect.status, |s| stronger(s, effect.status)),
                );
                push_unique(&mut tally.functions, &f.id);
            }
        }
        let component_paths: Vec<String> = components
            .values()
            .filter(|c| c.subsystem.as_deref() == Some(m.name))
            .map(|c| c.path.clone())
            .collect();
        let mut coverage: BTreeMap<String, CoverageTally> = BTreeMap::new();
        for path in &component_paths {
            for (dimension, status) in &components[path].coverage {
                let tally = coverage.entry(dimension.clone()).or_default();
                match status {
                    EpistemicStatus::Unknown => tally.unknown += 1,
                    EpistemicStatus::Unsupported => tally.unsupported += 1,
                    _ => tally.observed += 1,
                }
            }
        }
        let state_owned: Vec<String> = state_vars
            .values()
            .filter(|v| {
                !v.writers.is_empty()
                    && v.writers.iter().all(|w| {
                        function_subsystem.get(w.as_str()).copied().flatten() == Some(m.name)
                    })
            })
            .map(|v| v.key.clone())
            .collect();
        subsystems.push(SubsystemModel {
            name: m.name.to_owned(),
            root: m.root.to_owned(),
            declared_at,
            node_kind: node.map(|n| n.node_kind.clone()).unwrap_or_default(),
            responsibility,
            attributes,
            capabilities: provides
                .iter()
                .filter(|(from, _)| *from == m.name)
                .map(|(_, to)| (*to).to_owned())
                .collect(),
            functions: members.len(),
            public_functions: members
                .iter()
                .filter(|f| f.visibility.as_deref() == Some("pub"))
                .count(),
            unsafe_functions: members.iter().filter(|f| f.is_unsafe).count(),
            interface: interface
                .remove(m.name)
                .map(|s| s.into_iter().collect())
                .unwrap_or_default(),
            effects,
            state_owned,
            unresolved_calls: members.iter().map(|f| f.unresolved_calls).sum(),
            coverage,
            components: component_paths,
        });
    }
    subsystems.sort_by(|a, b| a.name.cmp(&b.name));

    // Level 4: capabilities.
    let capabilities: Vec<CapabilityModel> = declared
        .nodes
        .iter()
        .filter(|n| n.node_kind == "Capability")
        .map(|n| CapabilityModel {
            name: n.name.clone(),
            declared_at: format!("{}:{}", n.span.path, n.span.line),
            input: n.attributes.get("input").cloned(),
            output: n.attributes.get("output").cloned(),
            provided_by: provides
                .iter()
                .filter(|(_, to)| *to == n.name)
                .map(|(from, _)| (*from).to_owned())
                .collect(),
            consumed_by: declared
                .bindings
                .iter()
                .filter(|b| b.capability == n.name)
                .map(|b| b.consumer.clone())
                .collect(),
            realized_by: Claim::unknown(
                "no declaration or derivation maps a declared capability to the functions that \
                 realize it",
            ),
        })
        .collect();

    // Level 5: architecture -- declared dependencies reconciled with Cargo and resolved calls.
    let dir_of_package: BTreeMap<&str, &str> = crate_names
        .iter()
        .map(|(dir, name)| (name.as_str(), dir.as_str()))
        .collect();
    let subsystem_of_dir = |dir: &str| {
        materialized
            .iter()
            .find(|m| m.root == dir)
            .map(|m| m.name.to_owned())
    };
    let mut cargo_edges: BTreeSet<(String, String)> = BTreeSet::new();
    let mut cargo_evidence: BTreeMap<String, String> = BTreeMap::new();
    for edge in &input.dependency_closure.edges {
        let Some(consumer_dir) = dir_of_package.get(edge.consumer.as_str()) else {
            continue;
        };
        let Some(provider_dir) = dir_of_package.get(edge.provider.name.as_str()) else {
            continue;
        };
        if let (Some(from), Some(to)) = (
            subsystem_of_dir(consumer_dir),
            subsystem_of_dir(provider_dir),
        ) {
            cargo_evidence.insert(from.clone(), edge.evidence_path.clone());
            cargo_edges.insert((from, to));
        }
    }
    let cargo_members: BTreeSet<String> = crate_names
        .keys()
        .filter_map(|dir| subsystem_of_dir(dir))
        .collect();
    // Cargo reachability: a call may legally reach any crate on a dependency path (re-exports).
    let reach = |from: &str| {
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut frontier = vec![from.to_owned()];
        while let Some(next) = frontier.pop() {
            for (a, b) in &cargo_edges {
                if *a == next && seen.insert(b.clone()) {
                    frontier.push(b.clone());
                }
            }
        }
        seen
    };
    let declared_deps: BTreeMap<(String, String), String> = declared
        .edges
        .iter()
        .filter(|e| e.relation == "depends_on")
        .map(|e| {
            (
                (e.from.clone(), e.to.clone()),
                format!("{}:{}", e.span.path, e.span.line),
            )
        })
        .collect();
    let mut pairs: BTreeSet<(String, String)> = declared_deps.keys().cloned().collect();
    pairs.extend(cargo_edges.iter().cloned());
    pairs.extend(subsystem_calls.keys().cloned());
    let dependencies: Vec<DependencyEdge> = pairs
        .into_iter()
        .map(|(from, to)| {
            let declared = declared_deps.get(&(from.clone(), to.clone())).cloned();
            let cargo = cargo_edges.contains(&(from.clone(), to.clone()));
            let resolved_calls = subsystem_calls
                .get(&(from.clone(), to.clone()))
                .map_or(0, |c| c.0);
            let buildable = cargo_members.contains(&from) && cargo_members.contains(&to);
            let transitive = reach(&from).contains(&to);
            let (verdict, status) = match (declared.is_some(), cargo, resolved_calls > 0) {
                (true, true, true) => ("CONFIRMED", EpistemicStatus::Observed),
                (true, true, false) => (
                    "DECLARED_AND_BUILT_NO_RESOLVED_CALL",
                    EpistemicStatus::Observed,
                ),
                (true, false, _) if !buildable => {
                    ("DECLARED_NOT_CENSUSABLE", EpistemicStatus::Unknown)
                }
                (true, false, _) => ("DECLARED_NOT_BUILT", EpistemicStatus::Conflict),
                (false, true, _) => ("UNDECLARED", EpistemicStatus::Conflict),
                // Calls reaching a crate the consumer only depends on transitively: through a
                // re-export of a declared, built dependency.
                (false, false, true) if transitive => {
                    ("TRANSITIVE_VIA_REEXPORT", EpistemicStatus::Derived)
                }
                (false, false, _) => ("CALL_WITHOUT_DEPENDENCY", EpistemicStatus::Conflict),
            };
            DependencyEdge {
                from,
                to,
                declared,
                cargo,
                resolved_calls,
                verdict: verdict.into(),
                status,
            }
        })
        .collect();
    let architecture = ArchitectureModel {
        subsystems: subsystems.iter().map(|s| s.name.clone()).collect(),
        dependencies,
        unplaced_members: observed.unplaced_members.clone(),
        unassigned_components: components
            .values()
            .filter(|c| c.subsystem.is_none())
            .map(|c| c.path.clone())
            .collect(),
    };

    // Invariants.
    let mut invariants: Vec<Invariant> = Vec::new();
    // Dependency: no Cargo path means no call path, whatever resolution misses.
    for from in &cargo_members {
        let reachable = reach(from);
        for to in &cargo_members {
            if from == to || reachable.contains(to) {
                continue;
            }
            let counterexamples = subsystem_calls
                .get(&(from.clone(), to.clone()))
                .map(|c| c.1.clone())
                .unwrap_or_default();
            let (status, evidence) = if counterexamples.is_empty() {
                (
                    EpistemicStatus::Observed,
                    vec![
                        cargo_evidence
                            .get(from)
                            .cloned()
                            .unwrap_or_else(|| format!("{from}: no Cargo dependencies")),
                    ],
                )
            } else {
                (EpistemicStatus::Conflict, counterexamples)
            };
            invariants.push(Invariant {
                id: format!("INV-DEPENDENCY:{from}->{to}"),
                kind: InvariantKind::Dependency,
                statement: format!(
                    "{from} cannot invoke {to}: no Cargo dependency path from {from} to {to}"
                ),
                scope: vec![from.clone(), to.clone()],
                status,
                evidence,
                residual: Vec::new(),
            });
        }
    }
    // Declared constraints, as checked by the ADL compiler.
    for result in &input.adl.constraint_results {
        let kind = match result.derivation.first() {
            Some(d) if d.rule == ConstraintCheckKind::ObservedDependency => {
                InvariantKind::Dependency
            }
            // G130: a census-quantified `forbid call to` names two entities; `forbid effect` one.
            Some(d) if d.rule == ConstraintCheckKind::CensusQuantified => {
                if d.supporting_node_names.len() == 2 {
                    InvariantKind::Dependency
                } else {
                    InvariantKind::Authority
                }
            }
            _ => InvariantKind::Construction,
        };
        let status = match result.verdict {
            ConstraintVerdict::Satisfied => EpistemicStatus::Derived,
            ConstraintVerdict::Violated => EpistemicStatus::Conflict,
            _ => EpistemicStatus::Unknown,
        };
        let declared_at = declared
            .constraints
            .iter()
            .chain(&declared.invariants)
            .find(|c| c.name == result.name)
            .map(|c| format!("{}:{}", c.span.path, c.span.line));
        let mut scope: Vec<String> = result
            .derivation
            .iter()
            .flat_map(|d| {
                d.supporting_node_names
                    .iter()
                    .cloned()
                    .chain(d.materialization_target.clone())
            })
            .filter(|n| materialized.iter().any(|m| m.name == n))
            .collect();
        scope.sort();
        scope.dedup();
        invariants.push(Invariant {
            id: format!("INV-ADL:{}", result.name),
            kind,
            statement: format!("declared constraint {} holds", result.name),
            scope,
            status,
            evidence: declared_at.into_iter().collect(),
            residual: Vec::new(),
        });
    }
    let unobserved_files = |subsystem: &str, dimension: &str| -> usize {
        subsystems
            .iter()
            .find(|s| s.name == subsystem)
            .and_then(|s| s.coverage.get(dimension))
            .map_or(0, |t| t.unknown + t.unsupported)
    };
    // Safety: declared `unsafe fn`, over FUNCTION_SIGNATURE coverage.
    for s in &subsystems {
        if s.functions == 0 || s.unsafe_functions > 0 {
            continue;
        }
        let residual_files = unobserved_files(&s.name, "FUNCTION_SIGNATURE");
        invariants.push(Invariant {
            id: format!("INV-SAFETY:{}", s.name),
            kind: InvariantKind::Safety,
            statement: format!(
                "no function of {} is declared `unsafe fn` (unsafe blocks inside safe functions are \
                 not censused)",
                s.name
            ),
            scope: vec![s.name.clone()],
            status: if residual_files == 0 {
                EpistemicStatus::Observed
            } else {
                EpistemicStatus::Inferred
            },
            evidence: vec![format!(
                "{} FUNCTION_SIGNATURE records of {}",
                s.functions, s.name
            )],
            residual: if residual_files == 0 {
                Vec::new()
            } else {
                vec![format!("FUNCTION_SIGNATURE not OBSERVED on {residual_files} files")]
            },
        });
    }
    // State: who writes each piece of state, universal only where STATE is OBSERVED.
    for var in state_vars.values() {
        if var.writers.is_empty() {
            continue;
        }
        let residual: Vec<String> = var
            .subsystems
            .iter()
            .filter_map(|s| {
                let n = unobserved_files(s, "STATE");
                (n > 0).then(|| format!("STATE not OBSERVED on {n} files of {s}"))
            })
            .collect();
        invariants.push(Invariant {
            id: format!("INV-STATE:{}", var.key),
            kind: InvariantKind::State,
            statement: format!(
                "{} is written only by {} function(s)",
                var.key,
                var.writers.len()
            ),
            scope: var.subsystems.clone(),
            status: if residual.is_empty() {
                EpistemicStatus::Derived
            } else {
                EpistemicStatus::Inferred
            },
            evidence: var.writers.clone(),
            residual,
        });
    }
    // Authority: where each world-touching effect category originates.
    let workspace_categories: BTreeSet<&str> = functions
        .values()
        .flat_map(|f| f.effects.iter().map(|e| e.kind.as_str()))
        .filter(|k| AUTHORITY_CATEGORIES.contains(k))
        .collect();
    for s in &subsystems {
        if s.functions == 0 {
            continue;
        }
        for category in &workspace_categories {
            let origins = s
                .effects
                .get(*category)
                .map(|t| t.functions.clone())
                .unwrap_or_default();
            let mut residual = Vec::new();
            let n = unobserved_files(&s.name, "EFFECT");
            if n > 0 {
                residual.push(format!("EFFECT not OBSERVED on {n} files of {}", s.name));
            }
            if s.unresolved_calls > 0 {
                residual.push(format!(
                    "{} unresolved call sites in {} may reach an effect",
                    s.unresolved_calls, s.name
                ));
            }
            invariants.push(Invariant {
                id: format!("INV-AUTHORITY:{}:{category}", s.name),
                kind: InvariantKind::Authority,
                statement: format!(
                    "{category} authority in {} originates only in {} function(s)",
                    s.name,
                    origins.len()
                ),
                scope: vec![s.name.clone()],
                status: if residual.is_empty() {
                    EpistemicStatus::Derived
                } else {
                    EpistemicStatus::Inferred
                },
                evidence: origins,
                residual,
            });
        }
    }
    invariants.sort_by(|a, b| a.id.cmp(&b.id));

    // Understanding gaps: what an agent will ask that this model cannot answer.
    let unnamed_total: usize = functions.values().map(|f| f.unnamed_unresolved_calls).sum();
    let tests_inferred = functions.values().filter(|f| f.test_scope).count();
    let symbol_definitions = accounting.records.get("SYMBOL").copied().unwrap_or(0);
    let gaps = vec![
        UnderstandingGap {
            id: "GAP-COMPONENT-PURPOSE".into(),
            question_class: "what is this component for?".into(),
            missing: "components with no module documentation (//! in the file, /// on its mod \
                      item): their purpose is UNKNOWN (G128)"
                .into(),
            magnitude: components
                .values()
                .filter(|c| c.purpose.status == EpistemicStatus::Unknown)
                .count(),
            debt: "DEBT-SEMANTIC-COMPOSITION".into(),
        },
        UnderstandingGap {
            id: "GAP-ITEM-DOCUMENTATION".into(),
            question_class: "what is this function for?".into(),
            missing: "functions with no documentation (///): their purpose is UNKNOWN (G132); \
                      type documentation is carried on SYMBOL records but no type-level \
                      behavior composes it"
                .into(),
            magnitude: functions
                .values()
                .filter(|f| f.purpose.status == EpistemicStatus::Unknown)
                .count(),
            debt: "DEBT-SEMANTIC-COMPOSITION".into(),
        },
        UnderstandingGap {
            id: "GAP-UNRESOLVED-CALLEE".into(),
            question_class: "what else reaches this function?".into(),
            missing: "an unresolved call site whose callee is not a path or method name (a \
                      closure, a function pointer) cannot be narrowed to candidate callees; named \
                      ones narrow by spelling (G123)"
                .into(),
            magnitude: unnamed_total,
            debt: "DEBT-CALL".into(),
        },
        UnderstandingGap {
            id: "GAP-CAUSALITY".into(),
            question_class: "what causes / precedes this?".into(),
            missing: "no cross-function ordering or causation is represented; INVOKES, \
                      SUPPLIES_DATA and STATE_FLOW never imply cause"
                .into(),
            magnitude: relations.len(),
            debt: "DEBT-CAUSALITY".into(),
        },
        UnderstandingGap {
            id: "GAP-RESOURCE".into(),
            question_class: "what resource lifetime crosses this boundary?".into(),
            missing: "no RESOURCE dimension: acquisition/release and lifetimes are not censused"
                .into(),
            magnitude: functions.len(),
            debt: "DEBT-RESOURCE".into(),
        },
        UnderstandingGap {
            id: "GAP-CAPABILITY-REALIZATION".into(),
            question_class: "what implementation realizes this capability?".into(),
            missing: "declared capabilities are not mapped to realizing functions".into(),
            magnitude: capabilities.len(),
            debt: "DEBT-SEMANTIC-COMPOSITION".into(),
        },
        UnderstandingGap {
            id: "GAP-TYPE-USE-SITES".into(),
            question_class: "where is this type constructed or used?".into(),
            missing:
                "SYMBOL records definitions and declarations only (no REFERENCE role), so the \
                      construction and use sites of a type are not censused (G123 mission)"
                    .into(),
            magnitude: symbol_definitions,
            debt: "DEBT-SYMBOL".into(),
        },
        UnderstandingGap {
            id: "GAP-TEST-IDENTITY".into(),
            question_class: "which tests exercise this?".into(),
            missing: "test attributes are not censused; test scope is INFERRED from a `tests` \
                      module or file"
                .into(),
            magnitude: tests_inferred,
            debt: "DEBT-SEMANTIC-COMPOSITION".into(),
        },
    ];

    let functions: Vec<FunctionBehavior> = functions.into_values().collect();
    let components: Vec<ComponentBehavior> = components.into_values().collect();
    let state: Vec<StateVariable> = state_vars.into_values().collect();
    for (name, n) in [
        ("function_behavior", functions.len()),
        ("component_behavior", components.len()),
        ("subsystem_model", subsystems.len()),
        ("capability_model", capabilities.len()),
        ("state_variable", state.len()),
        ("relation", relations.len()),
        ("invariant", invariants.len()),
        ("understanding_gap", gaps.len()),
    ] {
        accounting.composed_objects.insert(name.into(), n);
    }
    WorldModel {
        schema: WORLD_MODEL_SCHEMA.into(),
        revision: input.revision.to_owned(),
        system: input.adl.ir.system.clone(),
        functions,
        components,
        subsystems,
        capabilities,
        state,
        architecture,
        relations,
        invariants,
        gaps,
        accounting,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_paths_follow_the_rust_module_file_convention() {
        assert_eq!(
            module_of("core/src/census/delta.rs", "core", "core").as_deref(),
            Some("core::census::delta")
        );
        assert_eq!(
            module_of("core/src/lib.rs", "core", "core").as_deref(),
            Some("core")
        );
        assert_eq!(
            module_of("apps/cli/src/main.rs", "apps/cli", "atlas-cli").as_deref(),
            Some("atlas_cli")
        );
        assert_eq!(
            module_of("core/src/semantic/mod.rs", "core", "core").as_deref(),
            Some("core::semantic")
        );
        assert_eq!(module_of("core/Cargo.toml", "core", "core"), None);
    }

    #[test]
    fn a_path_belongs_to_the_longest_materialized_root() {
        let m = [
            Materialized {
                name: "Apps",
                root: "apps",
            },
            Materialized {
                name: "Cli",
                root: "apps/cli",
            },
        ];
        assert_eq!(subsystem_of(&m, "apps/cli/src/main.rs"), Some("Cli"));
        assert_eq!(subsystem_of(&m, "apps/studio/x.ts"), Some("Apps"));
        assert_eq!(
            subsystem_of(&m, "applied/x.rs"),
            None,
            "a prefix must end at a separator"
        );
    }

    #[test]
    fn state_belongs_to_the_impl_self_type() {
        assert_eq!(self_type_of("impl:Store"), "Store");
        assert_eq!(self_type_of("impl:Opaque<'_>"), "Opaque");
        assert_eq!(self_type_of("impl:Visit<'ast> for Opaque<'_>"), "Opaque");
        assert_eq!(
            self_type_of("impl:syn::visit::Visit<'ast> for IndependentCallSites"),
            "IndependentCallSites"
        );
        assert_eq!(self_type_of("impl:CfgBuilder<'ctx, 'a>"), "CfgBuilder");
        assert_eq!(self_type_of("outer::impl:crate::a::Wrapper<T>"), "Wrapper");
    }

    #[test]
    fn stronger_evidence_wins_and_ties_keep_the_first() {
        use EpistemicStatus::*;
        assert_eq!(stronger(Inferred, Derived), Derived);
        assert_eq!(stronger(Derived, Inferred), Derived);
        assert_eq!(stronger(Unknown, Unsupported), Unknown);
        assert_eq!(stronger(Observed, Declared), Observed);
    }
}
