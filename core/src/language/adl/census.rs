//! Census -> ADL bridge (G63, P2 `ADL_ATLAS_ATLASX_END_TO_END`).
//!
//! Two directions, one observed architecture:
//! - `reconcile_dependencies` checks authored `depends_on` declarations against the dependency
//!   census. Agreement and disagreement both become typed `ConstraintResult`s; neither epistemic
//!   path erases the other (`.atlas/contracts/ADL-TO-ATLAS.md`, "Status rules").
//! - `derive_census_adl` turns census truth the authored ADL does not yet declare into ADL text
//!   (`CENSUS_ADL_PATH`), so census findings become durable declared semantics that every later
//!   census re-checks.
//!
//! An entity corresponds to a workspace member only through an observed materialization path
//! (`materialize X { path = "<member dir>" }`), never through name equality.

use super::{
    ConstraintCheckDerivation, ConstraintCheckKind, ConstraintResult, DeclaredGraph,
    DeclaredObservedDelta, adl_diag,
};
use crate::{
    SourceReport,
    census::dependency::{DependencyClosureReport, DependencyRole, DependencySourceKind},
    constraint::ConstraintVerdict,
};
use std::collections::{BTreeMap, BTreeSet};

/// Where the census-derived declarations live. Derivation reads the authored ADL *without* this
/// file, so regenerating it is a fixed point.
pub const CENSUS_ADL_PATH: &str = ".atlas/declared/census.adl";
/// The derivation's identity, cited in the generated file's header.
pub const CENSUS_DERIVATION_ID: &str = "atlas.adl.census-derivation.v1";
/// The only relation the dependency census can check.
const DEPENDS_ON: &str = "depends_on";

/// A Cargo workspace member whose own manifest the dependency census read.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObservedMember {
    pub package: String,
    /// Manifest directory relative to the census root (`""` for a root package).
    pub dir: String,
    pub manifest: String,
}

/// A runtime, build or proc-macro dependency of one workspace member on another. Dev-only
/// dependencies are test scaffolding, not architecture, and are excluded.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObservedMemberDependency {
    pub consumer: String,
    pub provider: String,
    pub role: DependencyRole,
    pub evidence_path: String,
}

/// The member-level architecture the dependency census observed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObservedArchitecture {
    pub members: Vec<ObservedMember>,
    pub dependencies: Vec<ObservedMemberDependency>,
    /// Workspace members seen only as a provider: no manifest of theirs was read, so their
    /// directory is not evidenced and no entity can be matched to them.
    pub unplaced_members: Vec<String>,
}

impl ObservedArchitecture {
    pub fn from_closure(closure: &DependencyClosureReport) -> Self {
        let mut members = BTreeMap::new();
        for edge in &closure.edges {
            if edge.role.is_some() && edge.evidence_path.ends_with("Cargo.toml") {
                let dir = edge
                    .evidence_path
                    .rsplit_once('/')
                    .map_or("", |(dir, _)| dir);
                members
                    .entry(edge.consumer.clone())
                    .or_insert_with(|| ObservedMember {
                        package: edge.consumer.clone(),
                        dir: dir.to_owned(),
                        manifest: edge.evidence_path.clone(),
                    });
            }
        }
        let mut dependencies = BTreeMap::new();
        let mut unplaced = BTreeSet::new();
        for edge in &closure.edges {
            if edge.provider.source_kind != DependencySourceKind::WorkspaceMember {
                continue;
            }
            if !members.contains_key(&edge.provider.name) {
                unplaced.insert(edge.provider.name.clone());
            }
            let Some(role) = edge.role.filter(|role| *role != DependencyRole::Dev) else {
                continue;
            };
            if members.contains_key(&edge.consumer) {
                dependencies
                    .entry((edge.consumer.clone(), edge.provider.name.clone()))
                    .or_insert_with(|| ObservedMemberDependency {
                        consumer: edge.consumer.clone(),
                        provider: edge.provider.name.clone(),
                        role,
                        evidence_path: edge.evidence_path.clone(),
                    });
            }
        }
        Self {
            members: members.into_values().collect(),
            dependencies: dependencies.into_values().collect(),
            unplaced_members: unplaced.into_iter().collect(),
        }
    }

    fn member_dir(&self, package: &str) -> Option<&str> {
        self.members
            .iter()
            .find(|member| member.package == package)
            .map(|member| member.dir.as_str())
    }
}

/// How authored `depends_on` declarations relate to the observed architecture. Entity pairs are
/// authored names; member pairs are Cargo package names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DependencyReconciliation {
    /// Declared and observed.
    pub agreed: Vec<(String, String)>,
    /// Declared between two entities materialized at workspace members that have no observed
    /// dependency.
    pub declared_not_observed: Vec<(String, String)>,
    /// Observed but not declared between any entities materialized at the two members (or one of
    /// the members has no entity at all).
    pub observed_not_declared: Vec<ObservedMemberDependency>,
    /// Workspace members no authored entity is materialized at.
    pub undeclared_members: Vec<ObservedMember>,
    /// Declared `depends_on` whose endpoints are not both materialized at census-read workspace
    /// members: no census frontend can check them. Recorded, never passed or failed.
    pub not_censusable: Vec<(String, String)>,
}

/// Entities materialized at each observed member's directory.
fn entities_by_member(
    declared: &DeclaredGraph,
    observed: &ObservedArchitecture,
) -> BTreeMap<String, BTreeSet<String>> {
    observed
        .members
        .iter()
        .map(|member| {
            let entities = declared
                .materializations
                .iter()
                .filter(|m| m.path.trim_end_matches('/') == member.dir)
                .map(|m| m.target.clone())
                .collect();
            (member.package.clone(), entities)
        })
        .collect()
}

pub fn reconcile_dependencies(
    declared: &DeclaredGraph,
    observed: &ObservedArchitecture,
) -> DependencyReconciliation {
    let by_member = entities_by_member(declared, observed);
    let member_of = |entity: &str| -> BTreeSet<&str> {
        by_member
            .iter()
            .filter(|(_, entities)| entities.contains(entity))
            .map(|(member, _)| member.as_str())
            .collect()
    };
    let observed_pairs: BTreeSet<(&str, &str)> = observed
        .dependencies
        .iter()
        .map(|d| (d.consumer.as_str(), d.provider.as_str()))
        .collect();
    let mut result = DependencyReconciliation::default();
    let mut declared_member_pairs = BTreeSet::new();
    let declared_edges: BTreeSet<(&str, &str)> = declared
        .edges
        .iter()
        .filter(|edge| edge.relation == DEPENDS_ON)
        .map(|edge| (edge.from.as_str(), edge.to.as_str()))
        .collect();
    for (from, to) in declared_edges {
        let (from_members, to_members) = (member_of(from), member_of(to));
        let pair = (from.to_owned(), to.to_owned());
        if from_members.is_empty() || to_members.is_empty() {
            result.not_censusable.push(pair);
            continue;
        }
        let mut seen = false;
        for consumer in &from_members {
            for provider in &to_members {
                declared_member_pairs.insert((*consumer, *provider));
                seen |= observed_pairs.contains(&(*consumer, *provider));
            }
        }
        if seen {
            result.agreed.push(pair);
        } else {
            result.declared_not_observed.push(pair);
        }
    }
    for dependency in &observed.dependencies {
        let pair = (dependency.consumer.as_str(), dependency.provider.as_str());
        if !declared_member_pairs.contains(&pair) {
            result.observed_not_declared.push(dependency.clone());
        }
    }
    result.undeclared_members = observed
        .members
        .iter()
        .filter(|member| {
            by_member
                .get(&member.package)
                .is_none_or(BTreeSet::is_empty)
        })
        .cloned()
        .collect();
    result
}

fn dependency_result(
    name: String,
    verdict: ConstraintVerdict,
    code: Option<(&str, String)>,
    nodes: Vec<String>,
) -> ConstraintResult {
    ConstraintResult {
        name,
        passed: verdict == ConstraintVerdict::Satisfied,
        verdict,
        diagnostics: code
            .map(|(code, message)| vec![adl_diag(code, message, ".atlas/declared", 1, 1)])
            .unwrap_or_default(),
        derivation: vec![ConstraintCheckDerivation {
            rule: ConstraintCheckKind::ObservedDependency,
            supporting_node_names: nodes,
            materialization_target: None,
            basis: Vec::new(),
        }],
    }
}

impl DependencyReconciliation {
    /// One typed result per checkable relation: agreement is SATISFIED, each disagreement is
    /// VIOLATED (`ATLAS-E060` declared-not-observed, `ATLAS-E061` observed-not-declared,
    /// `ATLAS-E062` a workspace member with no declared entity).
    pub fn constraint_results(&self) -> Vec<ConstraintResult> {
        let mut results = Vec::new();
        for (from, to) in &self.agreed {
            results.push(dependency_result(
                format!("DeclaredDependency:{from}->{to}"),
                ConstraintVerdict::Satisfied,
                None,
                vec![from.clone(), to.clone()],
            ));
        }
        for (from, to) in &self.declared_not_observed {
            results.push(dependency_result(
                format!("DeclaredDependency:{from}->{to}"),
                ConstraintVerdict::Violated,
                Some((
                    "ATLAS-E060",
                    format!(
                        "declared `{from} ->depends_on-> {to}` but the dependency census observes \
                         no runtime/build dependency between their workspace members"
                    ),
                )),
                vec![from.clone(), to.clone()],
            ));
        }
        for dependency in &self.observed_not_declared {
            results.push(dependency_result(
                format!(
                    "ObservedDependency:{}->{}",
                    dependency.consumer, dependency.provider
                ),
                ConstraintVerdict::Violated,
                Some((
                    "ATLAS-E061",
                    format!(
                        "the dependency census observes `{}` -> `{}` ({}) but no declared \
                         `depends_on` relates their entities",
                        dependency.consumer, dependency.provider, dependency.evidence_path
                    ),
                )),
                Vec::new(),
            ));
        }
        for member in &self.undeclared_members {
            results.push(dependency_result(
                format!("ObservedMember:{}", member.package),
                ConstraintVerdict::Violated,
                Some((
                    "ATLAS-E062",
                    format!(
                        "workspace member `{}` ({}) has no declared entity materialized at `{}`",
                        member.package, member.manifest, member.dir
                    ),
                )),
                Vec::new(),
            ));
        }
        results
    }

    /// Declared relations no census frontend can check, as explicit deltas.
    pub fn deltas(&self) -> Vec<DeclaredObservedDelta> {
        self.not_censusable
            .iter()
            .map(|(from, to)| DeclaredObservedDelta {
                code: "DECLARED_DEPENDENCY_NOT_CENSUSABLE".into(),
                message: format!(
                    "declared `{from} ->depends_on-> {to}`: an endpoint is not materialized at a \
                     census-read workspace member, so no dependency census can check it"
                ),
                subject: format!("{from}->{to}"),
            })
            .collect()
    }
}

/// `atlas-cli` -> `AtlasCli`: an ADL handle for a package, suffixed until it is unused.
fn entity_name(package: &str, taken: &BTreeSet<String>) -> String {
    let mut name: String = package
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_ascii_uppercase().to_string() + chars.as_str()
            })
        })
        .collect();
    if name.is_empty() {
        name = "Member".into();
    }
    while taken.contains(&name) {
        name.push_str("Package");
    }
    name
}

/// The ADL text declaring every piece of observed architecture `authored` does not declare:
/// an entity, materialization and materialization constraint per undeclared workspace member, and
/// a `depends_on` per undeclared observed dependency. Each declaration cites its census evidence.
/// `authored` must not include `CENSUS_ADL_PATH` itself (the derivation is then a fixed point).
pub fn derive_census_adl(
    authored: &DeclaredGraph,
    observed: &ObservedArchitecture,
    source: &SourceReport,
) -> String {
    let reconciliation = reconcile_dependencies(authored, observed);
    let mut taken: BTreeSet<String> = authored.nodes.iter().map(|n| n.name.clone()).collect();
    let by_member = entities_by_member(authored, observed);
    let mut entity_of: BTreeMap<&str, String> = by_member
        .iter()
        .filter_map(|(member, entities)| Some((member.as_str(), entities.iter().next()?.clone())))
        .collect();

    let mut text = format!(
        "atlas 1\n\n# Census-derived architecture ({CENSUS_DERIVATION_ID}). Generated -- do not \
         edit.\n# Regenerate: atlas-systemizer adl derive --root . --out {CENSUS_ADL_PATH}\n# \
         Declares only what the dependency census observes and the authored ADL does not; every \
         later\n# census re-checks it (ADL-TO-ATLAS.md).\n"
    );
    for member in &reconciliation.undeclared_members {
        let name = entity_name(&member.package, &taken);
        taken.insert(name.clone());
        let prefix = format!("{}/", member.dir);
        let rust = source.files.iter().any(|f| {
            f.language == "rust" && (member.dir.is_empty() || f.path.starts_with(&prefix))
        });
        text.push_str(&format!(
            "\n# census: workspace member `{package}` ({manifest})\nentity Package {name} {{\n    \
             origin = census\n    package = \"{package}\"\n}}\n\nmaterialize {name} {{\n    path = \
             \"{dir}\"\n{language}}}\n\nconstraint {name}IsMaterialized {{\n    require \
             materialized {name}\n}}\n",
            package = member.package,
            manifest = member.manifest,
            dir = member.dir,
            language = if rust { "    language = rust\n" } else { "" },
        ));
        entity_of.insert(member.package.as_str(), name);
    }
    let mut relations = Vec::new();
    for dependency in &reconciliation.observed_not_declared {
        let (Some(from), Some(to)) = (
            entity_of.get(dependency.consumer.as_str()),
            entity_of.get(dependency.provider.as_str()),
        ) else {
            continue;
        };
        relations.push(format!(
            "# census: `{}` -> `{}` ({}, {})\n{from} ->{DEPENDS_ON}-> {to}\n",
            dependency.consumer,
            dependency.provider,
            dependency.role.as_str(),
            dependency.evidence_path
        ));
    }
    if !relations.is_empty() {
        text.push('\n');
        text.push_str(&relations.join(""));
    }
    for package in &observed.unplaced_members {
        if observed.member_dir(package).is_none() {
            text.push_str(&format!(
                "\n# census: workspace member `{package}` is only seen as a provider; its manifest \
                 was not read, so it is not declared.\n"
            ));
        }
    }
    text
}

/// G137 (ADR 0055): the effect envelope of every composed subsystem with functions whose
/// authored ADL declares none -- `forall f: function in <S> observed effect within <C>, ...`
/// listing each category its functions definitely (OBSERVED or DERIVED) perform, or `none`.
/// `model` must be composed from the authored ADL plus the census membership declarations, never
/// from the committed census file, so regeneration stays a fixed point. Every later census
/// decides each envelope: a definite effect site outside it is a named counterexample.
pub fn derive_effect_envelopes(
    authored: &DeclaredGraph,
    model: &crate::composition::WorldModel,
) -> String {
    use super::{CensusForbidden, ConstraintCheck};
    use crate::EpistemicStatus;
    let declared: BTreeSet<&str> = authored
        .constraints
        .iter()
        .chain(&authored.invariants)
        .flat_map(|c| &c.checks)
        .filter_map(|check| match check {
            ConstraintCheck::CensusForbid {
                subsystem,
                forbidden: CensusForbidden::ObservedEffectOutside { .. },
            } => Some(subsystem.as_str()),
            _ => None,
        })
        .collect();
    let mut taken: BTreeSet<String> = authored
        .nodes
        .iter()
        .map(|n| n.name.clone())
        .chain(
            authored
                .constraints
                .iter()
                .chain(&authored.invariants)
                .map(|c| c.name.clone()),
        )
        .collect();
    let mut text = String::new();
    for subsystem in &model.subsystems {
        let name = subsystem.name.as_str();
        if declared.contains(name) {
            continue;
        }
        let mut sites: BTreeSet<&str> = BTreeSet::new();
        let mut functions = 0;
        for f in model
            .functions
            .iter()
            .filter(|f| f.subsystem.as_deref() == Some(name))
        {
            functions += 1;
            for site in &f.effects {
                if site.status == EpistemicStatus::Observed
                    || site.status == EpistemicStatus::Derived
                {
                    sites.insert(site.kind.as_str());
                }
            }
        }
        if functions == 0 {
            continue;
        }
        let mut invariant = format!("{name}EffectEnvelope");
        while taken.contains(&invariant) {
            invariant.push_str("Census");
        }
        taken.insert(invariant.clone());
        // Categories only, never counts: the file changes when an envelope does, not whenever a
        // function or a site is added.
        let within = if sites.is_empty() {
            "none".to_owned()
        } else {
            sites.into_iter().collect::<Vec<_>>().join(", ")
        };
        text.push_str(&format!(
            "\n# census: effect envelope of `{name}` -- every category its functions definitely \
             perform (OBSERVED\n# or DERIVED sites)\ninvariant {invariant} {{\n    forall f: \
             function in {name} observed effect within {within}\n}}\n"
        ));
    }
    text
}

#[cfg(test)]
mod tests;
