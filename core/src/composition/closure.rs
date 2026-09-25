//! Affected-impact closure of a change (G129, NA-IMPACT-CLOSURE; `contracts/ARCHITECTURAL-INTEGRITY.md`
//! "Impact closure and continuous revalidation").
//!
//! A change is a set of changed paths. The closure maps it to the semantic identities whose composed
//! facts can differ and, through the relations that carry impact, to the invariants that can
//! change. It reads the model before the change in full and, of the model after it, only what the
//! changed paths themselves contain -- what an incremental re-extraction of those paths would
//! have. The oracle composes both models in full and checks the closure against the full-recompute
//! diff: SOUND when every difference lies inside the closure; the misses otherwise.
//!
//! Propagation (a function's composed behavior is local to its own records and its resolved call
//! edges, so function impact is one hop; invariants aggregate over subsystems):
//! - R1 seed: every function defined in a changed path, before or after;
//! - R2 callees: a seed function's callees gain or lose a caller;
//! - R3 callers of removed functions: their resolved call no longer resolves;
//! - R4 name candidates of added functions: a call spelled with an added function's name, resolved
//!   or not, may now resolve to it (name-based resolution; G123's callee spelling).
//!
//! A workspace manifest, the root lockfile or the system's ADL among the changed paths makes the
//! closure global: they change resolution and the declared architecture everywhere. Known blind spot: a
//! changed `pub use` re-export or `mod` item can re-route path resolution in unchanged files by a
//! name the model does not carry; the oracle is the check.

use super::{InvariantKind, WorldModel};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const IMPACT_CLOSURE_SCHEMA: &str = "atlas.impact-closure.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImpactClosure {
    pub schema: String,
    pub changed_paths: Vec<String>,
    /// A manifest, lockfile or ADL declaration changed: everything is affected.
    pub global: bool,
    /// Changed paths added or deleted (the file set, which declared materializations compare).
    pub file_set_changed: Vec<String>,
    pub seed_functions: Vec<String>,
    pub affected_functions: Vec<String>,
    pub affected_components: Vec<String>,
    pub affected_subsystems: Vec<String>,
    pub affected_state: Vec<String>,
    pub affected_invariants: Vec<String>,
    /// How many affected functions each rule contributed (a function counts under its first rule).
    pub rules: BTreeMap<String, usize>,
    /// Resolved transitive callers of the seed in the model before: the semantic frontier an agent
    /// should re-read, not a set whose composed facts change.
    pub transitive_dependents: usize,
    pub blind_spots: Vec<String>,
}

/// One closure level checked against the full-recompute diff.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LevelCheck {
    pub changed: usize,
    pub affected: usize,
    /// Differences outside the closure: each one is a soundness failure.
    pub missed: Vec<String>,
}

crate::vocabulary_enum! {
    /// Whether a closure held every difference of the full recompute (G134: typed).
    pub enum ClosureVerdict {
        Sound => "SOUND",
        Unsound => "UNSOUND",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClosureOracle {
    /// `SOUND` when no level missed a difference, else `UNSOUND`.
    pub verdict: ClosureVerdict,
    pub functions: LevelCheck,
    pub components: LevelCheck,
    pub subsystems: LevelCheck,
    pub state: LevelCheck,
    pub relations: LevelCheck,
    pub invariants: LevelCheck,
}

/// A path whose change re-shapes resolution or the declared architecture everywhere: the root
/// lockfile, the root manifest, a subsystem root's manifest, a manifest outside every subsystem
/// (it may be an undeclared member), or the system's own ADL. A fixture's manifest or ADL nested
/// inside a subsystem is not the workspace's.
fn is_global_path(path: &str, subsystem_roots: &BTreeSet<&str>) -> bool {
    if path.starts_with(".atlas/declared/") && path.ends_with(".adl") {
        return true;
    }
    let (dir, name) = path.rsplit_once('/').unwrap_or(("", path));
    match name {
        "Cargo.lock" => dir.is_empty(),
        "Cargo.toml" => {
            dir.is_empty()
                || subsystem_roots.contains(dir)
                || !subsystem_roots
                    .iter()
                    .any(|root| dir.starts_with(&format!("{root}/")))
        }
        _ => false,
    }
}

/// The closure of `changed_paths`, from the model before the change and the changed paths' part of
/// the model after it. Both models must be normalized (`runtime::agent::normalize`): record ids
/// carry the revision, so functions are matched across revisions by their stable key.
pub fn impact_closure(
    before: &WorldModel,
    after: &WorldModel,
    changed_paths: &[String],
) -> ImpactClosure {
    let changed: BTreeSet<&str> = changed_paths.iter().map(String::as_str).collect();
    let roots: BTreeSet<&str> = before.subsystems.iter().map(|s| s.root.as_str()).collect();
    let global = changed.iter().any(|p| is_global_path(p, &roots));
    let in_changed = |path: &str| changed.contains(path);
    // What an incremental re-extraction of the changed paths would have: nothing else of `after`.
    let after_seed: Vec<&super::FunctionBehavior> = after
        .functions
        .iter()
        .filter(|f| in_changed(&f.path))
        .collect();
    let before_seed: Vec<&super::FunctionBehavior> = before
        .functions
        .iter()
        .filter(|f| in_changed(&f.path))
        .collect();
    let before_components: BTreeSet<&str> =
        before.components.iter().map(|c| c.path.as_str()).collect();
    let after_components: BTreeSet<&str> = after
        .components
        .iter()
        .filter(|c| in_changed(&c.path))
        .map(|c| c.path.as_str())
        .collect();
    let file_set_changed: Vec<String> = changed
        .iter()
        .filter(|p| before_components.contains(*p) != after_components.contains(*p))
        .map(|p| (*p).to_owned())
        .collect();

    let mut rules: BTreeMap<String, usize> = BTreeMap::new();
    let mut affected: BTreeSet<String> = BTreeSet::new();
    let mut add = |id: &str, rule: &str, affected: &mut BTreeSet<String>| {
        if affected.insert(id.to_owned()) {
            *rules.entry(rule.to_owned()).or_default() += 1;
        }
    };
    // R1.
    for f in before_seed.iter().chain(&after_seed) {
        add(&f.id, "R1_SEED", &mut affected);
    }
    let seed: BTreeSet<String> = affected.clone();
    let before_ids: BTreeSet<&str> = before_seed.iter().map(|f| f.id.as_str()).collect();
    let after_ids: BTreeSet<&str> = after_seed.iter().map(|f| f.id.as_str()).collect();
    // R2.
    for f in before_seed.iter().chain(&after_seed) {
        for callee in &f.calls {
            add(callee, "R2_CALLEE", &mut affected);
        }
    }
    // R3.
    for f in &before_seed {
        if !after_ids.contains(f.id.as_str()) {
            for caller in &f.callers {
                add(caller, "R3_CALLER_OF_REMOVED", &mut affected);
            }
        }
    }
    // R4.
    let by_id: BTreeMap<&str, &super::FunctionBehavior> = before
        .functions
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let added_names: BTreeSet<&str> = after_seed
        .iter()
        .filter(|f| !before_ids.contains(f.id.as_str()))
        .map(|f| f.name.as_str())
        .collect();
    if !added_names.is_empty() {
        for f in &before.functions {
            let spelled = f
                .unresolved_callee_names
                .keys()
                .any(|n| added_names.contains(n.as_str()));
            let resolved = f.calls.iter().any(|c| {
                by_id
                    .get(c.as_str())
                    .is_some_and(|callee| added_names.contains(callee.name.as_str()))
            });
            if spelled || resolved {
                add(&f.id, "R4_NAME_CANDIDATE", &mut affected);
            }
        }
    }
    if global {
        for f in &before.functions {
            add(&f.id, "GLOBAL", &mut affected);
        }
    }
    // Functions whose own outgoing calls may change: invariants over calls read these.
    let call_changed: BTreeSet<&str> = affected
        .iter()
        .filter(|id| {
            seed.contains(*id) || {
                let f = by_id.get(id.as_str());
                f.is_some_and(|f| {
                    f.unresolved_callee_names
                        .keys()
                        .any(|n| added_names.contains(n.as_str()))
                        || f.calls.iter().any(|c| {
                            !after_ids.contains(c.as_str()) && before_ids.contains(c.as_str())
                                || by_id.get(c.as_str()).is_some_and(|callee| {
                                    added_names.contains(callee.name.as_str())
                                })
                        })
                })
            }
        })
        .map(String::as_str)
        .collect();

    let path_of: BTreeMap<&str, &str> = before
        .functions
        .iter()
        .chain(after_seed.iter().copied())
        .map(|f| (f.id.as_str(), f.path.as_str()))
        .collect();
    let mut components: BTreeSet<String> = changed.iter().map(|p| (*p).to_owned()).collect();
    components.extend(
        affected
            .iter()
            .filter_map(|id| path_of.get(id.as_str()).map(|p| (*p).to_owned())),
    );
    let subsystem_of_component: BTreeMap<&str, &str> = before
        .components
        .iter()
        .chain(after.components.iter().filter(|c| in_changed(&c.path)))
        .filter_map(|c| Some((c.path.as_str(), c.subsystem.as_deref()?)))
        .collect();
    let subsystems_of = |paths: &mut dyn Iterator<Item = &str>| -> BTreeSet<String> {
        paths
            .filter_map(|p| subsystem_of_component.get(p).map(|s| (*s).to_owned()))
            .collect()
    };
    let affected_subsystems: BTreeSet<String> = if global {
        before.subsystems.iter().map(|s| s.name.clone()).collect()
    } else {
        subsystems_of(&mut components.iter().map(String::as_str))
    };
    let changed_subsystems = subsystems_of(&mut changed.iter().copied());
    let caller_subsystems = subsystems_of(
        &mut call_changed
            .iter()
            .filter_map(|id| path_of.get(id).copied()),
    );

    // State: the keys the seed reads or writes, before or after.
    let mut state: BTreeSet<String> = before_seed
        .iter()
        .chain(&after_seed)
        .flat_map(|f| f.state.iter().map(|s| s.state.clone()))
        .collect();
    if global {
        state.extend(before.state.iter().map(|s| s.key.clone()));
    }

    // Authority categories the seed gains or loses anywhere re-shape every subsystem's invariants.
    let categories = |fs: &[&super::FunctionBehavior]| -> BTreeSet<String> {
        fs.iter()
            .flat_map(|f| f.effects.iter().map(|e| e.kind.clone()))
            .collect()
    };
    let category_shift: BTreeSet<String> = categories(&before_seed)
        .symmetric_difference(&categories(&after_seed))
        .cloned()
        .collect();

    // An invariant the change introduces (a newly written field's INV-STATE) exists only after
    // it; the same seed-derived rule decides it (G136, found on the datafrog donor).
    let before_invariants: BTreeSet<&str> =
        before.invariants.iter().map(|i| i.id.as_str()).collect();
    let introduced = after
        .invariants
        .iter()
        .filter(|i| !before_invariants.contains(i.id.as_str()));
    let mut invariants: BTreeSet<String> = BTreeSet::new();
    for invariant in before.invariants.iter().chain(introduced) {
        let scoped = |set: &BTreeSet<String>| invariant.scope.iter().any(|s| set.contains(s));
        let hit = global
            // A declared invariant compares the declared architecture with the file set; a
            // census-quantified one (G130) also reads the calls, effects and coverage of the
            // entities it names.
            || (invariant.id.starts_with("INV-ADL:")
                && (!file_set_changed.is_empty()
                    || scoped(&changed_subsystems)
                    || scoped(&caller_subsystems)))
            || match invariant.kind {
                InvariantKind::Dependency if invariant.id.starts_with("INV-DEPENDENCY:") => {
                    invariant
                        .scope
                        .first()
                        .is_some_and(|from| caller_subsystems.contains(from))
                }
                InvariantKind::Safety => scoped(&changed_subsystems),
                InvariantKind::Authority => {
                    scoped(&changed_subsystems)
                        || scoped(&caller_subsystems)
                        || category_shift
                            .iter()
                            .any(|c| invariant.id.ends_with(&format!(":{c}")))
                }
                InvariantKind::State => {
                    invariant
                        .id
                        .strip_prefix("INV-STATE:")
                        .is_some_and(|key| state.contains(key))
                        || scoped(&changed_subsystems)
                }
                _ => false,
            };
        if hit {
            invariants.insert(invariant.id.clone());
        }
    }

    // The semantic frontier: resolved transitive callers of the seed.
    let mut frontier: BTreeSet<&str> = BTreeSet::new();
    let mut queue: Vec<&str> = seed.iter().map(String::as_str).collect();
    while let Some(id) = queue.pop() {
        if let Some(f) = by_id.get(id) {
            for caller in &f.callers {
                if !seed.contains(caller) && frontier.insert(caller) {
                    queue.push(caller);
                }
            }
        }
    }

    ImpactClosure {
        schema: IMPACT_CLOSURE_SCHEMA.into(),
        changed_paths: changed.iter().map(|p| (*p).to_owned()).collect(),
        global,
        file_set_changed,
        seed_functions: seed.into_iter().collect(),
        affected_functions: affected.into_iter().collect(),
        affected_components: components.into_iter().collect(),
        affected_subsystems: affected_subsystems.into_iter().collect(),
        affected_state: state.into_iter().collect(),
        affected_invariants: invariants.into_iter().collect(),
        rules,
        transitive_dependents: frontier.len(),
        blind_spots: vec![
            "a changed `pub use` re-export or `mod` item re-routing path resolution in unchanged \
             files by a name the model does not carry"
                .into(),
        ],
    }
}

fn refs(v: &[String]) -> BTreeSet<&str> {
    v.iter().map(String::as_str).collect()
}

/// Ids whose value differs between the two lists, including ids present in only one.
fn differing<T: PartialEq>(before: &[T], after: &[T], id: impl Fn(&T) -> String) -> Vec<String> {
    let a: BTreeMap<String, &T> = before.iter().map(|x| (id(x), x)).collect();
    let b: BTreeMap<String, &T> = after.iter().map(|x| (id(x), x)).collect();
    a.keys()
        .chain(b.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|k| a.get(*k) != b.get(*k))
        .cloned()
        .collect()
}

fn check(changed: Vec<String>, affected: &BTreeSet<&str>) -> LevelCheck {
    LevelCheck {
        missed: changed
            .iter()
            .filter(|c| !affected.contains(c.as_str()))
            .cloned()
            .collect(),
        changed: changed.len(),
        affected: affected.len(),
    }
}

/// Check `closure` against the full-recompute diff of `before` and `after` (both normalized).
pub fn oracle(closure: &ImpactClosure, before: &WorldModel, after: &WorldModel) -> ClosureOracle {
    let functions_ref = refs(&closure.affected_functions);
    let relation_id = |r: &super::Relation| format!("{}|{}|{}", r.kind.as_str(), r.from, r.to);
    let changed_relations = differing(&before.relations, &after.relations, relation_id);
    // A relation lies inside the closure when it touches an affected function.
    let relations: BTreeSet<&str> = changed_relations
        .iter()
        .filter(|r| {
            let mut parts = r.split('|').skip(1);
            parts.any(|end| functions_ref.contains(end))
        })
        .map(String::as_str)
        .collect();
    let result = ClosureOracle {
        verdict: ClosureVerdict::Sound,
        functions: check(
            differing(&before.functions, &after.functions, |f| f.id.clone()),
            &functions_ref,
        ),
        components: check(
            differing(&before.components, &after.components, |c| c.path.clone()),
            &refs(&closure.affected_components),
        ),
        subsystems: check(
            differing(&before.subsystems, &after.subsystems, |s| s.name.clone()),
            &refs(&closure.affected_subsystems),
        ),
        state: check(
            differing(&before.state, &after.state, |s| s.key.clone()),
            &refs(&closure.affected_state),
        ),
        relations: LevelCheck {
            missed: changed_relations
                .iter()
                .filter(|r| !relations.contains(r.as_str()))
                .cloned()
                .collect(),
            changed: changed_relations.len(),
            affected: relations.len(),
        },
        invariants: check(
            differing(&before.invariants, &after.invariants, |i| i.id.clone()),
            &refs(&closure.affected_invariants),
        ),
    };
    let sound = [
        &result.functions,
        &result.components,
        &result.subsystems,
        &result.state,
        &result.relations,
        &result.invariants,
    ]
    .iter()
    .all(|l| l.missed.is_empty());
    ClosureOracle {
        verdict: if sound {
            ClosureVerdict::Sound
        } else {
            ClosureVerdict::Unsound
        },
        ..result
    }
}
