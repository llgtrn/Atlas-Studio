//! The Agent-Worn Atlas lens (G122, ADR 0044): the operations an AI agent calls to understand work
//! through Atlas instead of reading source first.
//!
//! Every operation reads the composed `WorldModel`; none searches text. A target is an exact
//! structural selector (`subsystem:`, `path:`, `fn:`, `state:`, `capability:`); an unknown selector
//! is an error, never a fuzzy guess. The agent's mission text is recorded as its DECLARED intent and
//! never used to select facts, so the structural part of a `MissionContext` is a function of the
//! pinned reality and the Atlas version alone. Agent hypotheses are checked by `hypothesis` and
//! never merged into the model.

use super::{
    FunctionBehavior, Invariant, InvariantKind, RelationKind, StateVariable, UnderstandingGap,
    WorldModel, status_rank,
};
use crate::EpistemicStatus;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const MISSION_CONTEXT_SCHEMA: &str = "atlas.mission-context.v1";

/// How many entries a bounded list shows; the rest are counted in `omitted`.
pub const LIST_LIMIT: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bounded<T> {
    pub items: Vec<T>,
    pub omitted: usize,
}

fn bounded<T>(mut items: Vec<T>) -> Bounded<T> {
    let omitted = items.len().saturating_sub(LIST_LIMIT);
    items.truncate(LIST_LIMIT);
    Bounded { items, omitted }
}

/// A resolved target: the functions, components and subsystems a selector denotes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Scope {
    pub selector: String,
    pub kind: String,
    pub functions: BTreeSet<String>,
    pub components: BTreeSet<String>,
    pub subsystems: BTreeSet<String>,
}

/// Indexes over a model, built once per operation.
pub struct Index<'m> {
    pub model: &'m WorldModel,
    functions: BTreeMap<&'m str, &'m FunctionBehavior>,
}

impl<'m> Index<'m> {
    pub fn new(model: &'m WorldModel) -> Self {
        Self {
            model,
            functions: model.functions.iter().map(|f| (f.id.as_str(), f)).collect(),
        }
    }

    pub fn function(&self, id: &str) -> Option<&'m FunctionBehavior> {
        self.functions.get(id).copied()
    }

    /// `name@path:line`, the way a function is shown to the agent.
    pub fn label(&self, id: &str) -> String {
        match self.function(id) {
            Some(f) => {
                let owner = f
                    .owner_type
                    .as_deref()
                    .map_or(String::new(), |o| format!("{o}::"));
                format!("{owner}{}@{}:{}", f.name, f.path, f.line)
            }
            None => id.to_owned(),
        }
    }

    /// Resolve an exact structural selector.
    pub fn resolve(&self, selector: &str) -> Result<Scope, String> {
        let model = self.model;
        let (kind, value) = match selector.split_once(':') {
            Some((k, v))
                if ["subsystem", "path", "fn", "state", "capability"].contains(&k)
                    && !selector.starts_with("semantic:") =>
            {
                (k, v)
            }
            _ => {
                if model.subsystems.iter().any(|s| s.name == selector) {
                    ("subsystem", selector)
                } else if selector.starts_with("semantic:FUNCTION_IDENTITY:") {
                    ("fn", selector)
                } else {
                    ("path", selector)
                }
            }
        };
        let mut functions: BTreeSet<String> = BTreeSet::new();
        let mut components: BTreeSet<String> = BTreeSet::new();
        match kind {
            "subsystem" => {
                let s = model
                    .subsystems
                    .iter()
                    .find(|s| s.name == value)
                    .ok_or_else(|| {
                        format!("no subsystem `{value}`; known: {}", self.subsystem_names())
                    })?;
                components.extend(s.components.iter().cloned());
                functions.extend(
                    model
                        .functions
                        .iter()
                        .filter(|f| f.subsystem.as_deref() == Some(value))
                        .map(|f| f.id.clone()),
                );
            }
            "path" => {
                let prefix = value.trim_end_matches('/');
                components.extend(
                    model
                        .components
                        .iter()
                        .filter(|c| c.path == prefix || c.path.starts_with(&format!("{prefix}/")))
                        .map(|c| c.path.clone()),
                );
                if components.is_empty() {
                    return Err(format!(
                        "no censused component at `{value}` (a file path or a directory prefix)"
                    ));
                }
                functions.extend(
                    model
                        .functions
                        .iter()
                        .filter(|f| components.contains(&f.path))
                        .map(|f| f.id.clone()),
                );
            }
            "fn" => {
                if value.starts_with("semantic:FUNCTION_IDENTITY:") {
                    if self.function(value).is_none() {
                        return Err(format!("no function record `{value}`"));
                    }
                    functions.insert(value.to_owned());
                } else {
                    let (owner, name) = match value.rsplit_once("::") {
                        Some((o, n)) => (Some(o), n),
                        None => (None, value),
                    };
                    functions.extend(
                        model
                            .functions
                            .iter()
                            .filter(|f| {
                                f.name == name
                                    && owner.is_none_or(|o| {
                                        f.owner_type.as_deref().is_some_and(|t| {
                                            // `Owner` names `Owner<'a, T>`; the full spelling
                                            // matches too.
                                            t == o || t.split('<').next() == Some(o)
                                        })
                                    })
                            })
                            .map(|f| f.id.clone()),
                    );
                    if functions.is_empty() {
                        return Err(format!("no function named `{value}`"));
                    }
                }
                components.extend(
                    functions
                        .iter()
                        .filter_map(|id| self.function(id))
                        .map(|f| f.path.clone()),
                );
            }
            "state" => {
                let var = model.state.iter().find(|v| v.key == value).ok_or_else(|| {
                    format!("no state `{value}` (keys are `<self type>.<field>`)")
                })?;
                functions.extend(var.writers.iter().chain(&var.readers).cloned());
                components.extend(
                    functions
                        .iter()
                        .filter_map(|id| self.function(id))
                        .map(|f| f.path.clone()),
                );
            }
            "capability" => {
                let capability = model
                    .capabilities
                    .iter()
                    .find(|c| c.name == value)
                    .ok_or_else(|| format!("no declared capability `{value}`"))?;
                for provider in &capability.provided_by {
                    if let Some(s) = model.subsystems.iter().find(|s| &s.name == provider) {
                        components.extend(s.components.iter().cloned());
                    }
                }
                functions.extend(
                    model
                        .functions
                        .iter()
                        .filter(|f| components.contains(&f.path))
                        .map(|f| f.id.clone()),
                );
            }
            _ => unreachable!(),
        }
        let subsystems: BTreeSet<String> = model
            .components
            .iter()
            .filter(|c| components.contains(&c.path))
            .filter_map(|c| c.subsystem.clone())
            .collect();
        Ok(Scope {
            selector: format!("{kind}:{value}"),
            kind: kind.to_owned(),
            functions,
            components,
            subsystems,
        })
    }

    fn subsystem_names(&self) -> String {
        self.model
            .subsystems
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Transitive callers of `seed` (excluding `seed`), with their distance.
    pub fn callers_closure(&self, seed: &BTreeSet<String>) -> BTreeMap<String, usize> {
        let mut depth: BTreeMap<String, usize> = BTreeMap::new();
        let mut queue: VecDeque<(String, usize)> = seed.iter().map(|s| (s.clone(), 0)).collect();
        while let Some((id, d)) = queue.pop_front() {
            let Some(f) = self.function(&id) else {
                continue;
            };
            for caller in &f.callers {
                if seed.contains(caller) || depth.contains_key(caller) {
                    continue;
                }
                depth.insert(caller.clone(), d + 1);
                queue.push_back((caller.clone(), d + 1));
            }
        }
        depth
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MissionIntent {
    pub text: String,
    /// Always DECLARED: this is what the agent says it wants, not an Atlas observation.
    pub status: EpistemicStatus,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubsystemView {
    pub name: String,
    pub role: String,
    pub responsibility: Option<String>,
    pub responsibility_status: EpistemicStatus,
    pub capabilities: Vec<String>,
    pub functions_in_scope: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionRef {
    pub label: String,
    pub id: String,
    pub signature: String,
    pub weight: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Flow {
    pub direction: String,
    pub kind: String,
    pub counterpart: String,
    pub weight: usize,
    pub status: EpistemicStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateView {
    pub key: String,
    pub writers_in_scope: usize,
    pub writers_elsewhere: usize,
    pub readers_in_scope: usize,
    pub readers_elsewhere: usize,
    pub invariant: Option<String>,
    pub invariant_status: Option<EpistemicStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EffectView {
    pub category: String,
    pub sites: usize,
    pub status: EpistemicStatus,
    pub functions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvariantView {
    pub id: String,
    pub kind: InvariantKind,
    pub statement: String,
    pub status: EpistemicStatus,
    pub residual: Vec<String>,
}

impl From<&Invariant> for InvariantView {
    fn from(i: &Invariant) -> Self {
        Self {
            id: i.id.clone(),
            kind: i.kind,
            statement: i.statement.clone(),
            status: i.status,
            residual: i.residual.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Unknowns {
    /// Per dimension: [files with an OBSERVED-or-better obligation, UNKNOWN, UNSUPPORTED].
    pub coverage: BTreeMap<String, [usize; 3]>,
    pub unresolved_call_sites: usize,
    pub unresolved_values: usize,
    pub components_without_purpose: usize,
    pub gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImpactFrontier {
    /// Resolved callers outside the scope: a LOWER BOUND while unresolved call sites exist.
    pub callers: usize,
    pub by_subsystem: BTreeMap<String, usize>,
    pub nearest: Bounded<FunctionRef>,
    pub tests_reached_inferred: usize,
    pub status: EpistemicStatus,
    /// Functions with an unresolved call site spelled with the name of a function in the seed:
    /// INFERRED candidates (spelling, not resolution), one hop.
    pub candidate_callers: Bounded<FunctionRef>,
    pub candidate_sites: usize,
    /// Unresolved call sites anywhere whose callee is not a name: they may reach anything.
    pub unnamed_sites: usize,
    pub residual: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Compression {
    pub raw_records: usize,
    pub raw_obligations: usize,
    pub composed_objects: usize,
    /// Raw records and obligations per composed object presented (rounded).
    pub ratio: usize,
}

/// The bounded semantic package the agent reasons over for one mission.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MissionContext {
    pub schema: String,
    pub revision: String,
    pub mission: MissionIntent,
    pub target: String,
    pub target_kind: String,
    pub scope: ScopeSummary,
    pub subsystems: Vec<SubsystemView>,
    pub entry_points: Bounded<FunctionRef>,
    pub public_surface: Bounded<FunctionRef>,
    pub critical_calls: Bounded<Flow>,
    pub data_paths: Bounded<Flow>,
    pub state: Bounded<StateView>,
    pub effects: Vec<EffectView>,
    pub resources: String,
    pub constraints: Bounded<InvariantView>,
    pub invariants: Bounded<InvariantView>,
    pub unknowns: Unknowns,
    pub impact_frontier: ImpactFrontier,
    pub next_questions: Vec<String>,
    pub compression: Compression,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScopeSummary {
    pub functions: usize,
    pub components: Bounded<String>,
    pub subsystems: Vec<String>,
}

fn signature(f: &FunctionBehavior) -> String {
    let inputs: Vec<String> = f
        .inputs
        .iter()
        .map(|p| format!("{}: {}", p.name, p.type_spelling))
        .collect();
    format!(
        "{}{}fn {}({}){}",
        f.visibility
            .as_deref()
            .map_or(String::new(), |v| if v == "inherited" {
                String::new()
            } else {
                format!("{v} ")
            }),
        if f.is_async { "async " } else { "" },
        f.name,
        inputs.join(", "),
        f.output
            .as_deref()
            .map_or(String::new(), |o| format!(" -> {o}"))
    )
}

fn function_ref(index: &Index, id: &str, weight: usize) -> FunctionRef {
    FunctionRef {
        label: index.label(id),
        id: id.to_owned(),
        signature: index.function(id).map(signature).unwrap_or_default(),
        weight,
    }
}

fn component_of(index: &Index, id: &str) -> String {
    index
        .function(id)
        .map_or_else(|| id.to_owned(), |f| f.path.clone())
}

/// Coverage and unresolved counts of a scope.
pub fn unknowns(index: &Index, scope: &Scope) -> Unknowns {
    let mut u = Unknowns::default();
    for c in index
        .model
        .components
        .iter()
        .filter(|c| scope.components.contains(&c.path))
    {
        for (dimension, status) in &c.coverage {
            let entry = u.coverage.entry(dimension.clone()).or_insert([0; 3]);
            match status {
                EpistemicStatus::Unknown => entry[1] += 1,
                EpistemicStatus::Unsupported => entry[2] += 1,
                _ => entry[0] += 1,
            }
        }
        u.components_without_purpose += usize::from(c.purpose.status == EpistemicStatus::Unknown);
    }
    for id in &scope.functions {
        if let Some(f) = index.function(id) {
            u.unresolved_call_sites += f.unresolved_calls;
            u.unresolved_values += f.data_flow.unresolved;
        }
    }
    u.gaps = index.model.gaps.iter().map(|g| g.id.clone()).collect();
    u
}

fn invariants_for<'m>(index: &Index<'m>, scope: &Scope) -> Vec<&'m Invariant> {
    let mut out: Vec<&Invariant> = index
        .model
        .invariants
        .iter()
        .filter(|i| {
            let evidence_hit = i.evidence.iter().any(|e| scope.functions.contains(e));
            let scope_hit = match i.kind {
                InvariantKind::State => evidence_hit,
                _ => i.scope.iter().any(|s| scope.subsystems.contains(s)),
            };
            scope_hit || evidence_hit
        })
        .collect();
    out.sort_by(|a, b| (status_rank(a.status), &a.id).cmp(&(status_rank(b.status), &b.id)));
    out
}

/// Unresolved call sites of `f` without a recorded callee name: they may reach any function.
/// Derived as unresolved minus named, never read from a counter, so a model written before
/// callee names were recorded (G123) claims no narrowing it cannot support.
pub fn unattributed_calls(f: &FunctionBehavior) -> usize {
    f.unresolved_calls
        .saturating_sub(f.unresolved_callee_names.values().sum::<usize>())
}

/// Impact frontier of a function set: resolved transitive callers outside it.
pub fn frontier(index: &Index, seed: &BTreeSet<String>) -> ImpactFrontier {
    let closure = index.callers_closure(seed);
    let mut by_subsystem: BTreeMap<String, usize> = BTreeMap::new();
    let mut tests = 0;
    let mut nearest: Vec<(usize, String)> = Vec::new();
    for (id, depth) in &closure {
        if let Some(f) = index.function(id) {
            *by_subsystem
                .entry(f.subsystem.clone().unwrap_or_else(|| "-".into()))
                .or_default() += 1;
            tests += usize::from(f.test_scope);
        }
        nearest.push((*depth, id.clone()));
    }
    nearest.sort();
    let names: BTreeSet<&str> = seed
        .iter()
        .filter_map(|id| index.function(id))
        .map(|f| f.name.as_str())
        .collect();
    let mut candidates: Vec<(usize, String)> = Vec::new();
    for f in &index.model.functions {
        if seed.contains(&f.id) {
            continue;
        }
        let sites: usize = names
            .iter()
            .filter_map(|n| f.unresolved_callee_names.get(*n))
            .sum();
        if sites > 0 {
            candidates.push((sites, f.id.clone()));
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let candidate_sites: usize = candidates.iter().map(|c| c.0).sum();
    let unnamed: usize = index.model.functions.iter().map(unattributed_calls).sum();
    ImpactFrontier {
        callers: closure.len(),
        by_subsystem,
        nearest: bounded(
            nearest
                .into_iter()
                .map(|(d, id)| function_ref(index, &id, d))
                .collect(),
        ),
        tests_reached_inferred: tests,
        status: EpistemicStatus::Derived,
        candidate_callers: bounded(
            candidates
                .into_iter()
                .map(|(sites, id)| function_ref(index, &id, sites))
                .collect(),
        ),
        candidate_sites,
        unnamed_sites: unnamed,
        residual: format!(
            "LOWER BOUND: resolved callers only; {candidate_sites} unresolved call sites spelled \
             with a scope function's name are INFERRED candidates; {unnamed} unresolved call sites \
             with a non-name callee may reach anything (GAP-UNRESOLVED-CALLEE); calls inside \
             closure bodies are not censused at all (NA-CLOSURE-REGIONS)"
        ),
    }
}

/// `understand(target, mission)`: the bounded MissionContext for one mission.
pub fn understand(
    model: &WorldModel,
    target: &str,
    mission: &str,
) -> Result<MissionContext, String> {
    let index = Index::new(model);
    let scope = index.resolve(target)?;
    let in_scope = |id: &str| scope.functions.contains(id);
    let mut raw_records = 0;
    for id in &scope.functions {
        raw_records += index.function(id).map_or(0, |f| f.records);
    }
    let raw_obligations: usize = model
        .components
        .iter()
        .filter(|c| scope.components.contains(&c.path))
        .map(|c| c.coverage.len())
        .sum();

    // Entry points: in-scope functions reached from outside the scope.
    let mut entry: Vec<(usize, String)> = Vec::new();
    let mut surface: Vec<String> = Vec::new();
    for id in &scope.functions {
        let Some(f) = index.function(id) else {
            continue;
        };
        let outside = f.callers.iter().filter(|c| !in_scope(c)).count();
        if outside > 0 {
            entry.push((outside, id.clone()));
        }
        if f.visibility.as_deref() == Some("pub") && !f.test_scope {
            surface.push(id.clone());
        }
    }
    entry.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

    // Calls and data leaving or entering the scope, aggregated by counterpart component.
    let mut calls: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut data: BTreeMap<(String, String), usize> = BTreeMap::new();
    for r in &model.relations {
        let (from_in, to_in) = (in_scope(&r.from), in_scope(&r.to));
        if from_in == to_in {
            continue;
        }
        let (direction, other) = if from_in {
            ("OUT", component_of(&index, &r.to))
        } else {
            ("IN", component_of(&index, &r.from))
        };
        let bucket = match r.kind {
            RelationKind::Invokes => &mut calls,
            RelationKind::SuppliesData | RelationKind::StateFlow => &mut data,
        };
        let label = if r.kind == RelationKind::StateFlow {
            format!("{direction}:STATE_FLOW")
        } else {
            direction.to_owned()
        };
        *bucket.entry((label, other)).or_default() += r.weight;
    }
    let flows = |map: BTreeMap<(String, String), usize>, kind: &str| {
        let mut v: Vec<Flow> = map
            .into_iter()
            .map(|((direction, counterpart), weight)| {
                let (direction, kind) = match direction.split_once(':') {
                    Some((d, k)) => (d.to_owned(), k.to_owned()),
                    None => (direction, kind.to_owned()),
                };
                Flow {
                    direction,
                    kind,
                    counterpart,
                    weight,
                    status: EpistemicStatus::Derived,
                }
            })
            .collect();
        v.sort_by(|a, b| {
            b.weight
                .cmp(&a.weight)
                .then(a.counterpart.cmp(&b.counterpart))
        });
        bounded(v)
    };

    // State touched by the scope.
    let mut state: Vec<StateView> = model
        .state
        .iter()
        .filter(|v| v.writers.iter().chain(&v.readers).any(|f| in_scope(f)))
        .map(|v| state_view(model, v, &in_scope))
        .collect();
    state.sort_by(|a, b| (b.writers_in_scope, &a.key).cmp(&(a.writers_in_scope, &b.key)));

    let effects = effects(&index, &scope);
    let invariants = invariants_for(&index, &scope);
    let (constraints, invariants): (Vec<&Invariant>, Vec<&Invariant>) =
        invariants.into_iter().partition(|i| {
            matches!(
                i.kind,
                InvariantKind::Dependency | InvariantKind::Construction
            )
        });
    let unknowns = unknowns(&index, &scope);
    let impact_frontier = frontier(&index, &scope.functions);

    let subsystems: Vec<SubsystemView> = model
        .subsystems
        .iter()
        .filter(|s| {
            scope.subsystems.contains(&s.name)
                || calls
                    .keys()
                    .chain(data.keys())
                    .any(|(_, c)| s.components.contains(c))
        })
        .map(|s| SubsystemView {
            name: s.name.clone(),
            role: if scope.subsystems.contains(&s.name) {
                "TARGET"
            } else {
                "NEIGHBOR"
            }
            .into(),
            responsibility: s.responsibility.value.clone(),
            responsibility_status: s.responsibility.status,
            capabilities: s.capabilities.clone(),
            functions_in_scope: scope
                .functions
                .iter()
                .filter(|id| {
                    index.function(id).and_then(|f| f.subsystem.as_deref()) == Some(&s.name)
                })
                .count(),
        })
        .collect();

    let mut next_questions = Vec::new();
    if unknowns.unresolved_call_sites > 0 {
        next_questions.push(format!(
            "which callees do the {} unresolved call sites in scope reach? (receiver types: \
             NA-CALL-TYPE-RESIDUAL; callee spelling: GAP-UNRESOLVED-CALLEE)",
            unknowns.unresolved_call_sites
        ));
    }
    for (dimension, [_, unknown, unsupported]) in &unknowns.coverage {
        if unknown + unsupported > 0 {
            next_questions.push(format!(
                "is {dimension} complete on the {} files of scope whose coverage is not OBSERVED?",
                unknown + unsupported
            ));
        }
    }
    if unknowns.components_without_purpose > 0 {
        next_questions.push(format!(
            "what are the {} undocumented components in scope for? (no module documentation: GAP-COMPONENT-PURPOSE)",
            unknowns.components_without_purpose
        ));
    }
    for v in state
        .iter()
        .filter(|v| v.invariant_status == Some(EpistemicStatus::Inferred))
        .take(3)
    {
        next_questions.push(format!(
            "does anything besides the observed writers mutate {}? (STATE coverage not OBSERVED)",
            v.key
        ));
    }

    let entry_points = bounded(
        entry
            .into_iter()
            .map(|(w, id)| function_ref(&index, &id, w))
            .collect(),
    );
    let public_surface = bounded(
        surface
            .into_iter()
            .map(|id| function_ref(&index, &id, 0))
            .collect(),
    );
    let critical_calls = flows(calls, "INVOKES");
    let data_paths = flows(data, "SUPPLIES_DATA");
    let state = bounded(state);
    let constraints = bounded(constraints.into_iter().map(InvariantView::from).collect());
    let invariants = bounded(invariants.into_iter().map(InvariantView::from).collect());
    let composed_objects = subsystems.len()
        + entry_points.items.len()
        + public_surface.items.len()
        + critical_calls.items.len()
        + data_paths.items.len()
        + state.items.len()
        + effects.len()
        + constraints.items.len()
        + invariants.items.len()
        + impact_frontier.nearest.items.len()
        + next_questions.len();
    let mut components: Vec<String> = scope.components.iter().cloned().collect();
    components.sort();
    Ok(MissionContext {
        schema: MISSION_CONTEXT_SCHEMA.into(),
        revision: model.revision.clone(),
        mission: MissionIntent {
            text: mission.to_owned(),
            status: EpistemicStatus::Declared,
            role: "agent intent: recorded, never used to select facts".into(),
        },
        target: scope.selector.clone(),
        target_kind: scope.kind.clone(),
        scope: ScopeSummary {
            functions: scope.functions.len(),
            components: bounded(components),
            subsystems: scope.subsystems.iter().cloned().collect(),
        },
        subsystems,
        entry_points,
        public_surface,
        critical_calls,
        data_paths,
        state,
        effects,
        resources: "UNKNOWN: no RESOURCE dimension (GAP-RESOURCE, DEBT-RESOURCE)".into(),
        constraints,
        invariants,
        unknowns,
        impact_frontier,
        next_questions,
        compression: Compression {
            raw_records,
            raw_obligations,
            composed_objects,
            ratio: (raw_records + raw_obligations + composed_objects / 2)
                .checked_div(composed_objects)
                .unwrap_or(0),
        },
    })
}

fn state_view(model: &WorldModel, v: &StateVariable, in_scope: &dyn Fn(&str) -> bool) -> StateView {
    let invariant = model
        .invariants
        .iter()
        .find(|i| i.kind == InvariantKind::State && i.id == format!("INV-STATE:{}", v.key));
    StateView {
        key: v.key.clone(),
        writers_in_scope: v.writers.iter().filter(|f| in_scope(f)).count(),
        writers_elsewhere: v.writers.iter().filter(|f| !in_scope(f)).count(),
        readers_in_scope: v.readers.iter().filter(|f| in_scope(f)).count(),
        readers_elsewhere: v.readers.iter().filter(|f| !in_scope(f)).count(),
        invariant: invariant.map(|i| i.id.clone()),
        invariant_status: invariant.map(|i| i.status),
    }
}

/// `effects(scope)`: effect categories exercised in scope, strongest status first.
pub fn effects(index: &Index, scope: &Scope) -> Vec<EffectView> {
    let mut by: BTreeMap<String, (usize, EpistemicStatus, BTreeSet<String>)> = BTreeMap::new();
    for id in &scope.functions {
        let Some(f) = index.function(id) else {
            continue;
        };
        for e in &f.effects {
            let entry = by
                .entry(e.kind.clone())
                .or_insert((0, e.status, BTreeSet::new()));
            entry.0 += 1;
            if status_rank(e.status) < status_rank(entry.1) {
                entry.1 = e.status;
            }
            entry.2.insert(index.label(id));
        }
    }
    let mut v: Vec<EffectView> = by
        .into_iter()
        .map(|(category, (sites, status, functions))| EffectView {
            category,
            sites,
            status,
            functions: functions.into_iter().take(LIST_LIMIT).collect(),
        })
        .collect();
    v.sort_by(|a, b| b.sites.cmp(&a.sites).then(a.category.cmp(&b.category)));
    v
}

/// `explain(entity)`: the composed object(s) a selector denotes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Explanation {
    Functions(Vec<FunctionExplanation>),
    Component(Box<super::ComponentBehavior>),
    Subsystem(Box<super::SubsystemModel>),
    State {
        variable: StateVariable,
        writers: Vec<String>,
        readers: Vec<String>,
        invariant: Option<Invariant>,
    },
    Capability(super::CapabilityModel),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionExplanation {
    pub label: String,
    pub signature: String,
    pub behavior: FunctionBehavior,
    pub calls: Vec<String>,
    pub callers: Vec<String>,
}

pub fn explain(model: &WorldModel, target: &str) -> Result<Explanation, String> {
    let index = Index::new(model);
    let scope = index.resolve(target)?;
    let value = target.split_once(':').map_or(target, |(_, v)| v);
    Ok(match scope.kind.as_str() {
        "subsystem" => Explanation::Subsystem(Box::new(
            model
                .subsystems
                .iter()
                .find(|s| s.name == value)
                .cloned()
                .unwrap(),
        )),
        "path" if scope.components.len() == 1 => {
            let path = scope.components.iter().next().unwrap();
            Explanation::Component(Box::new(
                model
                    .components
                    .iter()
                    .find(|c| &c.path == path)
                    .cloned()
                    .unwrap(),
            ))
        }
        "state" => {
            let variable = model
                .state
                .iter()
                .find(|v| v.key == value)
                .cloned()
                .unwrap();
            let invariant = model
                .invariants
                .iter()
                .find(|i| i.id == format!("INV-STATE:{}", variable.key))
                .cloned();
            Explanation::State {
                writers: variable.writers.iter().map(|w| index.label(w)).collect(),
                readers: variable.readers.iter().map(|r| index.label(r)).collect(),
                variable,
                invariant,
            }
        }
        "capability" => Explanation::Capability(
            model
                .capabilities
                .iter()
                .find(|c| c.name == value)
                .cloned()
                .unwrap(),
        ),
        _ => Explanation::Functions(
            scope
                .functions
                .iter()
                .filter_map(|id| index.function(id))
                .take(LIST_LIMIT)
                .map(|f| FunctionExplanation {
                    label: index.label(&f.id),
                    signature: signature(f),
                    calls: f.calls.iter().map(|c| index.label(c)).collect(),
                    callers: f.callers.iter().map(|c| index.label(c)).collect(),
                    behavior: f.clone(),
                })
                .collect(),
        ),
    })
}

/// `impact(change)`: what a change to the targets can reach.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Impact {
    pub changed: Vec<String>,
    pub changed_functions: usize,
    pub frontier: ImpactFrontier,
    pub components: Bounded<(String, usize)>,
    pub state_readers: Bounded<(String, String)>,
    pub invariants_at_risk: Bounded<InvariantView>,
}

pub fn impact(model: &WorldModel, targets: &[&str]) -> Result<Impact, String> {
    let index = Index::new(model);
    let mut seed: BTreeSet<String> = BTreeSet::new();
    let mut subsystems: BTreeSet<String> = BTreeSet::new();
    for target in targets {
        let scope = index.resolve(target)?;
        seed.extend(scope.functions);
        subsystems.extend(scope.subsystems);
    }
    let frontier = frontier(&index, &seed);
    let closure = index.callers_closure(&seed);
    let mut components: BTreeMap<String, usize> = BTreeMap::new();
    for id in closure.keys() {
        *components.entry(component_of(&index, id)).or_default() += 1;
    }
    let mut components: Vec<(String, usize)> = components.into_iter().collect();
    components.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut readers: Vec<(String, String)> = Vec::new();
    for v in &model.state {
        if v.writers.iter().any(|w| seed.contains(w)) {
            for r in v.readers.iter().filter(|r| !seed.contains(*r)) {
                readers.push((v.key.clone(), index.label(r)));
            }
        }
    }
    let scope = Scope {
        selector: targets.join(","),
        kind: "change".into(),
        functions: seed.clone(),
        components: BTreeSet::new(),
        subsystems,
    };
    let at_risk: Vec<InvariantView> = invariants_for(&index, &scope)
        .into_iter()
        .filter(|i| {
            i.evidence.iter().any(|e| seed.contains(e))
                || matches!(i.kind, InvariantKind::Dependency)
        })
        .map(InvariantView::from)
        .collect();
    Ok(Impact {
        changed: targets.iter().map(|t| (*t).to_owned()).collect(),
        changed_functions: seed.len(),
        frontier,
        components: bounded(components),
        state_readers: bounded(readers),
        invariants_at_risk: bounded(at_risk),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TraceStep {
    pub from: String,
    pub relation: String,
    pub to: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Trace {
    pub verdict: String,
    pub status: EpistemicStatus,
    pub steps: Vec<TraceStep>,
    pub note: String,
}

/// `trace(origin, destination)`: the shortest typed path through INVOKES, SUPPLIES_DATA and
/// STATE_FLOW. No path found is UNKNOWN, never a proof of absence.
pub fn trace(model: &WorldModel, origin: &str, destination: &str) -> Result<Trace, String> {
    let index = Index::new(model);
    let from = index.resolve(origin)?;
    let to = index.resolve(destination)?;
    let mut adjacency: BTreeMap<&str, Vec<&super::Relation>> = BTreeMap::new();
    for r in &model.relations {
        adjacency.entry(r.from.as_str()).or_default().push(r);
    }
    let mut previous: BTreeMap<String, &super::Relation> = BTreeMap::new();
    let mut queue: VecDeque<String> = from.functions.iter().cloned().collect();
    let mut seen: BTreeSet<String> = from.functions.clone();
    let mut reached = None;
    if let Some(hit) = from.functions.iter().find(|f| to.functions.contains(*f)) {
        reached = Some(hit.clone());
    }
    while reached.is_none() {
        let Some(node) = queue.pop_front() else { break };
        for r in adjacency.get(node.as_str()).into_iter().flatten() {
            if seen.insert(r.to.clone()) {
                previous.insert(r.to.clone(), r);
                if to.functions.contains(&r.to) {
                    reached = Some(r.to.clone());
                    break;
                }
                queue.push_back(r.to.clone());
            }
        }
    }
    let Some(end) = reached else {
        let unresolved: usize = from
            .functions
            .iter()
            .filter_map(|f| index.function(f))
            .map(|f| f.unresolved_calls)
            .sum();
        return Ok(Trace {
            verdict: "NO_PATH_OBSERVED".into(),
            status: EpistemicStatus::Unknown,
            steps: Vec::new(),
            note: format!(
                "no typed path among resolved relations; the origin alone has {unresolved} \
                 unresolved call sites, so absence is not established"
            ),
        });
    };
    let mut steps = Vec::new();
    let mut cursor = end;
    while let Some(r) = previous.get(&cursor) {
        steps.push(TraceStep {
            from: index.label(&r.from),
            relation: r.kind.as_str().into(),
            to: index.label(&r.to),
            evidence: r.evidence.first().cloned().unwrap_or_default(),
        });
        cursor = r.from.clone();
    }
    steps.reverse();
    Ok(Trace {
        verdict: "PATH_OBSERVED".into(),
        status: EpistemicStatus::Derived,
        steps,
        note: "each step is a resolved relation; no step implies causation (GAP-CAUSALITY)".into(),
    })
}

/// `why(from, to)`: every direct reason two targets are related.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Why {
    pub relations: Bounded<TraceStep>,
    pub dependencies: Vec<super::DependencyEdge>,
    pub invariants: Vec<InvariantView>,
}

pub fn why(model: &WorldModel, a: &str, b: &str) -> Result<Why, String> {
    let index = Index::new(model);
    let (sa, sb) = (index.resolve(a)?, index.resolve(b)?);
    let relations: Vec<TraceStep> = model
        .relations
        .iter()
        .filter(|r| {
            (sa.functions.contains(&r.from) && sb.functions.contains(&r.to))
                || (sb.functions.contains(&r.from) && sa.functions.contains(&r.to))
        })
        .map(|r| TraceStep {
            from: index.label(&r.from),
            relation: r.kind.as_str().into(),
            to: index.label(&r.to),
            evidence: r.evidence.first().cloned().unwrap_or_default(),
        })
        .collect();
    let dependencies = model
        .architecture
        .dependencies
        .iter()
        .filter(|d| {
            (sa.subsystems.contains(&d.from) && sb.subsystems.contains(&d.to))
                || (sb.subsystems.contains(&d.from) && sa.subsystems.contains(&d.to))
        })
        .cloned()
        .collect();
    let invariants = model
        .invariants
        .iter()
        .filter(|i| {
            i.kind == InvariantKind::Dependency
                && i.scope.iter().any(|s| sa.subsystems.contains(s))
                && i.scope.iter().any(|s| sb.subsystems.contains(s))
        })
        .map(InvariantView::from)
        .collect();
    Ok(Why {
        relations: bounded(relations),
        dependencies,
        invariants,
    })
}

/// `state_model(scope)`: every piece of state the scope touches, with its writers and readers.
pub fn state_model(model: &WorldModel, target: &str) -> Result<Vec<StateView>, String> {
    let index = Index::new(model);
    let scope = index.resolve(target)?;
    let in_scope = |id: &str| scope.functions.contains(id);
    Ok(model
        .state
        .iter()
        .filter(|v| v.writers.iter().chain(&v.readers).any(|f| in_scope(f)))
        .map(|v| state_view(model, v, &in_scope))
        .collect())
}

/// `invariants(scope)`.
pub fn invariants(model: &WorldModel, target: &str) -> Result<Vec<InvariantView>, String> {
    let index = Index::new(model);
    let scope = index.resolve(target)?;
    Ok(invariants_for(&index, &scope)
        .into_iter()
        .map(InvariantView::from)
        .collect())
}

/// `dependencies(scope)`: the subsystem dependency edges touching the scope.
pub fn dependencies(
    model: &WorldModel,
    target: &str,
) -> Result<Vec<super::DependencyEdge>, String> {
    let index = Index::new(model);
    let scope = index.resolve(target)?;
    Ok(model
        .architecture
        .dependencies
        .iter()
        .filter(|d| scope.subsystems.contains(&d.from) || scope.subsystems.contains(&d.to))
        .cloned()
        .collect())
}

/// `capabilities(scope)`: declared capabilities of the scope's subsystems.
pub fn capabilities(
    model: &WorldModel,
    target: &str,
) -> Result<Vec<super::CapabilityModel>, String> {
    let index = Index::new(model);
    let scope = index.resolve(target)?;
    Ok(model
        .capabilities
        .iter()
        .filter(|c| {
            c.provided_by
                .iter()
                .chain(&c.consumed_by)
                .any(|s| scope.subsystems.contains(s))
        })
        .cloned()
        .collect())
}

/// Operations Atlas cannot answer yet return the gap that owns them, never a guess.
pub fn unanswerable(model: &WorldModel, gap: &str) -> Option<UnderstandingGap> {
    model.gaps.iter().find(|g| g.id == gap).cloned()
}

/// `compare(a, b)`: two targets' behavior side by side.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Comparison {
    pub a: String,
    pub b: String,
    pub functions: [usize; 2],
    pub effects_only_a: Vec<String>,
    pub effects_only_b: Vec<String>,
    pub effects_shared: Vec<String>,
    pub shared_callees: Bounded<String>,
    pub shared_state: Vec<String>,
    pub unresolved_calls: [usize; 2],
}

pub fn compare(model: &WorldModel, a: &str, b: &str) -> Result<Comparison, String> {
    let index = Index::new(model);
    let (sa, sb) = (index.resolve(a)?, index.resolve(b)?);
    let categories = |s: &Scope| -> BTreeSet<String> {
        effects(&index, s).into_iter().map(|e| e.category).collect()
    };
    let callees = |s: &Scope| -> BTreeSet<String> {
        s.functions
            .iter()
            .filter_map(|id| index.function(id))
            .flat_map(|f| f.calls.iter().cloned())
            .collect()
    };
    let touched = |s: &Scope| -> BTreeSet<String> {
        s.functions
            .iter()
            .filter_map(|id| index.function(id))
            .flat_map(|f| f.state.iter().map(|a| a.state.clone()))
            .collect()
    };
    let unresolved = |s: &Scope| -> usize {
        s.functions
            .iter()
            .filter_map(|id| index.function(id))
            .map(|f| f.unresolved_calls)
            .sum()
    };
    let (ea, eb) = (categories(&sa), categories(&sb));
    Ok(Comparison {
        a: a.to_owned(),
        b: b.to_owned(),
        functions: [sa.functions.len(), sb.functions.len()],
        effects_only_a: ea.difference(&eb).cloned().collect(),
        effects_only_b: eb.difference(&ea).cloned().collect(),
        effects_shared: ea.intersection(&eb).cloned().collect(),
        shared_callees: bounded(
            callees(&sa)
                .intersection(&callees(&sb))
                .map(|id| index.label(id))
                .collect(),
        ),
        shared_state: touched(&sa).intersection(&touched(&sb)).cloned().collect(),
        unresolved_calls: [unresolved(&sa), unresolved(&sb)],
    })
}

/// `plan(goal)`: a DERIVED checklist the agent's design must satisfy. The design itself is the
/// agent's; Atlas supplies what must be preserved, re-verified and resolved.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanChecklist {
    pub status: EpistemicStatus,
    pub note: String,
    pub preserve: Vec<String>,
    pub reverify_components: Vec<String>,
    pub resolve_first: Vec<String>,
    pub verification: Vec<String>,
}

pub fn plan(model: &WorldModel, target: &str) -> Result<PlanChecklist, String> {
    let context = understand(model, target, "")?;
    let imp = impact(model, &[target])?;
    Ok(PlanChecklist {
        status: EpistemicStatus::Derived,
        note: "structural obligations only: the design is the agent's HYPOTHESIS until verified"
            .into(),
        preserve: context
            .constraints
            .items
            .iter()
            .chain(&context.invariants.items)
            .map(|i| format!("{} [{}]", i.id, i.status.as_str()))
            .collect(),
        reverify_components: imp
            .components
            .items
            .iter()
            .map(|(c, _)| c.clone())
            .collect(),
        resolve_first: context.next_questions.clone(),
        verification: vec![
            "cargo fmt --all --check".into(),
            "cargo clippy -q --workspace --all-targets -- -D warnings".into(),
            "cargo test -q --workspace".into(),
            "atlas-systemizer agent verify --before <pre-change world model>".into(),
            "atlas-systemizer recensus prove".into(),
        ],
    })
}

/// `verify(candidate)`: invariant regressions between the model before a change and after it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvariantDelta {
    pub verdict: String,
    pub broken: Vec<String>,
    pub dropped: Vec<String>,
    pub weakened: Vec<String>,
    pub strengthened: Vec<String>,
    pub new: Vec<String>,
    pub new_authority: Vec<String>,
    pub dependency_changes: Vec<String>,
}

pub fn verify(before: &WorldModel, after: &WorldModel) -> InvariantDelta {
    let b: BTreeMap<&str, &Invariant> = before
        .invariants
        .iter()
        .map(|i| (i.id.as_str(), i))
        .collect();
    let a: BTreeMap<&str, &Invariant> = after
        .invariants
        .iter()
        .map(|i| (i.id.as_str(), i))
        .collect();
    let mut delta = InvariantDelta {
        verdict: String::new(),
        broken: Vec::new(),
        dropped: Vec::new(),
        weakened: Vec::new(),
        strengthened: Vec::new(),
        new: Vec::new(),
        new_authority: Vec::new(),
        dependency_changes: Vec::new(),
    };
    for (id, old) in &b {
        match a.get(id) {
            None => delta.dropped.push((*id).to_owned()),
            Some(new)
                if new.status == EpistemicStatus::Conflict
                    && old.status != EpistemicStatus::Conflict =>
            {
                delta.broken.push((*id).to_owned());
            }
            Some(new) if status_rank(new.status) > status_rank(old.status) => delta.weakened.push(
                format!("{id}: {} -> {}", old.status.as_str(), new.status.as_str()),
            ),
            Some(new) if status_rank(new.status) < status_rank(old.status) => {
                delta.strengthened.push(format!(
                    "{id}: {} -> {}",
                    old.status.as_str(),
                    new.status.as_str()
                ))
            }
            Some(_) => {}
        }
    }
    for id in a.keys().filter(|id| !b.contains_key(*id)) {
        if a[id].status == EpistemicStatus::Conflict {
            delta.broken.push((*id).to_owned());
        } else {
            delta.new.push((*id).to_owned());
        }
    }
    for s in &after.subsystems {
        let old = before.subsystems.iter().find(|o| o.name == s.name);
        for category in s.effects.keys() {
            if super::AUTHORITY_CATEGORIES.contains(&category.as_str())
                && old.is_none_or(|o| !o.effects.contains_key(category))
            {
                delta.new_authority.push(format!("{}: {category}", s.name));
            }
        }
    }
    let edges = |m: &WorldModel| -> BTreeMap<(String, String), String> {
        m.architecture
            .dependencies
            .iter()
            .map(|d| ((d.from.clone(), d.to.clone()), d.verdict.clone()))
            .collect()
    };
    let (eb, ea) = (edges(before), edges(after));
    for (k, v) in &ea {
        if eb.get(k) != Some(v) {
            delta.dependency_changes.push(format!(
                "{}->{}: {} -> {v}",
                k.0,
                k.1,
                eb.get(k).map_or("absent", String::as_str)
            ));
        }
    }
    for k in eb.keys().filter(|k| !ea.contains_key(*k)) {
        delta
            .dependency_changes
            .push(format!("{}->{}: removed", k.0, k.1));
    }
    delta.verdict = if delta.broken.is_empty() && delta.dropped.is_empty() {
        "HELD".into()
    } else {
        "REGRESSED".into()
    };
    delta
}

/// An agent hypothesis, checked against the model and kept apart from it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HypothesisCheck {
    pub hypothesis: String,
    pub status: EpistemicStatus,
    pub outcome: String,
    pub evidence: Vec<String>,
    pub note: String,
}

/// Structured hypotheses: `writes-only:<state>:<fn>[,<fn>...]`, `never-invokes:<A>:<B>` (subsystems),
/// `invokes:<fn>:<fn>`, `effect:<fn>:<CATEGORY>`. Outcomes: VALIDATED, FALSIFIED,
/// STILL_HYPOTHESIZED. The status of the hypothesis itself stays HYPOTHESIS; the outcome is Atlas's.
pub fn hypothesis(model: &WorldModel, text: &str) -> Result<HypothesisCheck, String> {
    let index = Index::new(model);
    let mut parts = text.splitn(2, ':');
    let kind = parts.next().unwrap_or_default();
    let rest = parts.next().ok_or("expected `<kind>:<arguments>`")?;
    let check = |outcome: &str, evidence: Vec<String>, note: String| HypothesisCheck {
        hypothesis: text.to_owned(),
        status: EpistemicStatus::Hypothesis,
        outcome: outcome.into(),
        evidence,
        note,
    };
    match kind {
        "writes-only" => {
            let (state, names) = rest
                .rsplit_once(':')
                .ok_or("writes-only:<state>:<fn,...>")?;
            let var = model
                .state
                .iter()
                .find(|v| v.key == state)
                .ok_or_else(|| format!("no state `{state}`"))?;
            let claimed: BTreeSet<&str> = names.split(',').collect();
            let observed: BTreeSet<&str> = var
                .writers
                .iter()
                .filter_map(|w| index.function(w).map(|f| f.name.as_str()))
                .collect();
            let extra: Vec<String> = observed
                .difference(&claimed)
                .map(|s| (*s).to_owned())
                .collect();
            if !extra.is_empty() {
                return Ok(check(
                    "FALSIFIED",
                    extra,
                    "observed writers outside the claim".into(),
                ));
            }
            let invariant = model
                .invariants
                .iter()
                .find(|i| i.id == format!("INV-STATE:{state}"));
            match invariant {
                Some(i) if i.status == EpistemicStatus::Derived => Ok(check(
                    "VALIDATED",
                    var.writers.clone(),
                    "every observed writer is claimed and STATE is OBSERVED in scope".into(),
                )),
                Some(i) => Ok(check(
                    "STILL_HYPOTHESIZED",
                    var.writers.clone(),
                    format!(
                        "consistent with every observed writer; residual: {}",
                        i.residual.join("; ")
                    ),
                )),
                None => Ok(check(
                    "STILL_HYPOTHESIZED",
                    Vec::new(),
                    "no writer observed".into(),
                )),
            }
        }
        "never-invokes" => {
            let (a, b) = rest.split_once(':').ok_or("never-invokes:<A>:<B>")?;
            let invariant = model
                .invariants
                .iter()
                .find(|i| i.id == format!("INV-DEPENDENCY:{a}->{b}"));
            let calls = model
                .architecture
                .dependencies
                .iter()
                .find(|d| d.from == a && d.to == b)
                .map_or(0, |d| d.resolved_calls);
            if calls > 0 {
                return Ok(check(
                    "FALSIFIED",
                    vec![format!("{calls} resolved calls {a} -> {b}")],
                    String::new(),
                ));
            }
            match invariant {
                Some(i) if i.status == EpistemicStatus::Observed => {
                    Ok(check("VALIDATED", i.evidence.clone(), i.statement.clone()))
                }
                _ => Ok(check(
                    "STILL_HYPOTHESIZED",
                    Vec::new(),
                    "no resolved call observed, but a dependency path exists or is unknown".into(),
                )),
            }
        }
        "invokes" => {
            let (a, b) = rest.split_once(':').ok_or("invokes:<fn>:<fn>")?;
            let (sa, sb) = (
                index.resolve(&format!("fn:{a}"))?,
                index.resolve(&format!("fn:{b}"))?,
            );
            let hits: Vec<String> = model
                .relations
                .iter()
                .filter(|r| {
                    r.kind == RelationKind::Invokes
                        && sa.functions.contains(&r.from)
                        && sb.functions.contains(&r.to)
                })
                .flat_map(|r| r.evidence.iter().cloned())
                .collect();
            if !hits.is_empty() {
                return Ok(check("VALIDATED", hits, String::new()));
            }
            // Only an unresolved site spelled with the callee's name, or one with a non-name
            // callee, can hide the call (G123).
            let callee_names: BTreeSet<&str> = sb
                .functions
                .iter()
                .filter_map(|f| index.function(f))
                .map(|f| f.name.as_str())
                .collect();
            let unresolved: usize = sa
                .functions
                .iter()
                .filter_map(|f| index.function(f))
                .map(|f| {
                    unattributed_calls(f)
                        + callee_names
                            .iter()
                            .filter_map(|n| f.unresolved_callee_names.get(*n))
                            .sum::<usize>()
                })
                .sum();
            let coverage_observed = sa.components.iter().all(|p| {
                model
                    .components
                    .iter()
                    .find(|c| &c.path == p)
                    .is_some_and(|c| {
                        c.coverage.get("CALL").is_some_and(|s| {
                            status_rank(*s) <= status_rank(EpistemicStatus::Derived)
                        })
                    })
            });
            if unresolved == 0 && coverage_observed {
                Ok(check(
                    "FALSIFIED",
                    Vec::new(),
                    "every call site of the caller is resolved and none reaches the callee".into(),
                ))
            } else {
                Ok(check(
                    "STILL_HYPOTHESIZED",
                    Vec::new(),
                    format!(
                        "{unresolved} unresolved call sites in the caller are spelled with the callee's name or have a non-name callee"
                    ),
                ))
            }
        }
        "effect" => {
            let (f, category) = rest.rsplit_once(':').ok_or("effect:<fn>:<CATEGORY>")?;
            let scope = index.resolve(&format!("fn:{f}"))?;
            let hits: Vec<String> = scope
                .functions
                .iter()
                .filter_map(|id| index.function(id))
                .flat_map(|fb| {
                    fb.effects
                        .iter()
                        .filter(|e| e.kind == category)
                        .flat_map(|e| e.records.clone())
                })
                .collect();
            if !hits.is_empty() {
                return Ok(check("VALIDATED", hits, String::new()));
            }
            Ok(check(
                "STILL_HYPOTHESIZED",
                Vec::new(),
                "no such effect observed; EFFECT coverage and unresolved calls leave absence open"
                    .into(),
            ))
        }
        _ => Err(format!("unknown hypothesis kind `{kind}`")),
    }
}
