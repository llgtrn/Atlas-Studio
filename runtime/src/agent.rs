//! Agent-Worn Atlas entry points (G122, ADR 0044): compose the census of a repository into a
//! `WorldModel` and serve the lens operations over it. The composition and every operation are
//! pure (`atlas_core::composition`); this module only runs the census and fixes digests.

use atlas_core::IntegrityDigest;
use atlas_core::composition::{WorldModel, compose, lens::MissionContext};
pub use atlas_core::composition::{closure, lens};
use std::collections::BTreeMap;
use std::{io, path::Path};

/// Census `root` and compose it.
pub fn world_model(root: impl AsRef<Path>) -> io::Result<WorldModel> {
    Ok(compose(&crate::systemize(root)?))
}

pub fn read_world_model(path: impl AsRef<Path>) -> io::Result<WorldModel> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// A function's identity across revisions: record ids carry the revision, so two revisions of an
/// unchanged function have different ids. `path#scope#name`, with `@line` only to separate two
/// functions that would otherwise share a key.
fn stable_keys(model: &WorldModel) -> BTreeMap<String, String> {
    let mut count: BTreeMap<String, usize> = BTreeMap::new();
    let key = |f: &atlas_core::composition::FunctionBehavior| {
        format!("{}#{}#{}", f.path, f.scope, f.name)
    };
    for f in &model.functions {
        *count.entry(key(f)).or_default() += 1;
    }
    model
        .functions
        .iter()
        .map(|f| {
            let k = key(f);
            let k = if count[&k] > 1 {
                format!("{k}@{}", f.line)
            } else {
                k
            };
            (f.id.clone(), k)
        })
        .collect()
}

fn normalize_value(value: &mut serde_json::Value, keys: &BTreeMap<String, String>) {
    match value {
        serde_json::Value::String(s) => {
            if let Some(k) = keys.get(s.as_str()) {
                *s = k.clone();
            } else if let Some(rest) = s.strip_prefix("semantic:") {
                // Any other record id: keep its dimension, drop its revision-bound hash.
                if let Some((dimension, _)) = rest.split_once(':') {
                    *s = format!("semantic:{dimension}");
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items.iter_mut() {
                normalize_value(item, keys);
            }
            if items.iter().all(|i| i.is_string()) {
                items.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values_mut() {
                normalize_value(item, keys);
            }
        }
        _ => {}
    }
}

/// `model` with every function id replaced by its stable key, every other record id reduced to
/// its dimension and every list of names sorted: two revisions of the same code normalize equal.
pub fn normalize(model: &WorldModel) -> WorldModel {
    let keys = stable_keys(model);
    let mut value = serde_json::to_value(model).expect("a WorldModel always serializes");
    value["revision"] = serde_json::Value::String(String::new());
    normalize_value(&mut value, &keys);
    let mut model: WorldModel =
        serde_json::from_value(value).expect("normalizing keeps the WorldModel shape");
    // Lists ordered by record id are re-ordered by the stable keys.
    model.functions.sort_by(|a, b| a.id.cmp(&b.id));
    model
        .relations
        .sort_by(|a, b| (a.kind, &a.from, &a.to).cmp(&(b.kind, &b.from, &b.to)));
    model
}

/// The impact closure of `changed` between two models and its check against their full-recompute
/// diff (G129), both over the normalized models.
pub fn impact_closure(
    before: &WorldModel,
    after: &WorldModel,
    changed: &[String],
) -> (closure::ImpactClosure, closure::ClosureOracle) {
    let (before, after) = (normalize(before), normalize(after));
    let result = closure::impact_closure(&before, &after, changed);
    let oracle = closure::oracle(&result, &before, &after);
    (result, oracle)
}

/// The paths a git range changes (`git diff --name-only <from> <to>` in `root`): the input of an
/// impact closure (G129).
pub fn changed_paths(root: impl AsRef<Path>, from: &str, to: &str) -> io::Result<Vec<String>> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", from, to])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect())
}

/// Digest of a MissionContext without the agent's mission text: two missions over the same target
/// of the same reality have the same structural digest, because the text never selects facts.
pub fn structural_digest(context: &MissionContext) -> String {
    let mut structural = context.clone();
    structural.mission.text.clear();
    let bytes = serde_json::to_vec(&structural).expect("a MissionContext always serializes");
    IntegrityDigest::of_bytes(&bytes).as_str().to_owned()
}

/// Digest of a whole world model (the composition's own determinism check).
pub fn model_digest(model: &WorldModel) -> String {
    let bytes = serde_json::to_vec(model).expect("a WorldModel always serializes");
    IntegrityDigest::of_bytes(&bytes).as_str().to_owned()
}

/// One Agent-Utility benchmark case (`.atlas/evidence/agent/benchmark.json`): a question an agent
/// asks while working, the Atlas operation that answers it, and the answer the case expects.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct BenchmarkCase {
    pub id: String,
    pub class: String,
    pub question: String,
    pub op: String,
    pub args: Vec<String>,
    pub expect: Vec<String>,
    /// What the agent would otherwise have to read or run by hand.
    pub manual_alternative: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct BenchmarkResult {
    pub id: String,
    pub class: String,
    pub correct: bool,
    pub answer: Vec<String>,
    /// Evidence items behind the answer (record ids, spans, calls); 0 for a gap answer.
    pub evidence: usize,
    /// For an UNKNOWN_HONESTY case: whether Atlas declined instead of inventing an answer.
    pub honest_unknown: Option<bool>,
}

fn find<'m, T>(items: &'m [T], key: impl Fn(&T) -> bool, what: &str) -> Result<&'m T, String> {
    items
        .iter()
        .find(|i| key(i))
        .ok_or(format!("no such {what}"))
}

/// Answer one case from the model: a list of strings and the evidence count behind it.
pub fn answer(model: &WorldModel, case: &BenchmarkCase) -> Result<(Vec<String>, usize), String> {
    let arg = |i: usize| {
        case.args
            .get(i)
            .map(String::as_str)
            .ok_or_else(|| format!("{}: missing argument {i}", case.id))
    };
    let index = lens::Index::new(model);
    Ok(match case.op.as_str() {
        "fn_subsystem" => {
            let scope = index.resolve(arg(0)?)?;
            (
                scope.subsystems.into_iter().collect(),
                scope.functions.len(),
            )
        }
        "hypothesis" => {
            let check = lens::hypothesis(model, arg(0)?)?;
            (vec![check.outcome], check.evidence.len())
        }
        "dependency_verdict" => {
            let (from, to) = (arg(0)?, arg(1)?);
            let edge = find(
                &model.architecture.dependencies,
                |d| d.from == from && d.to == to,
                "dependency edge",
            )?;
            (
                vec![edge.verdict.clone(), edge.status.as_str().to_owned()],
                edge.resolved_calls,
            )
        }
        "state_writers" => {
            let key = arg(0)?;
            let var = find(&model.state, |v| v.key == key, "state")?;
            let invariant = find(
                &model.invariants,
                |i| i.id == format!("INV-STATE:{key}"),
                "state invariant",
            )?;
            let mut answer: Vec<String> = var
                .writers
                .iter()
                .filter_map(|w| index.function(w).map(|f| f.name.clone()))
                .collect();
            answer.push(invariant.status.as_str().to_owned());
            (answer, var.writers.len())
        }
        "trace" => {
            let trace = lens::trace(model, arg(0)?, arg(1)?)?;
            (vec![trace.verdict], trace.steps.len())
        }
        "impact_candidates" => {
            let frontier = lens::impact(model, &[arg(0)?])?.frontier;
            let mut answer = vec![format!("candidates:{}", frontier.candidate_sites > 0)];
            answer.extend(
                frontier
                    .residual
                    .split(|c: char| c.is_whitespace() || c == '(' || c == ')')
                    .filter(|w| w.starts_with("NA-") || w.starts_with("GAP-"))
                    .map(str::to_owned),
            );
            (answer, frontier.candidate_sites + frontier.callers)
        }
        "understand_scope" => {
            let context = lens::understand(model, arg(0)?, &case.question)?;
            let components = context.scope.components;
            let mut answer = vec![format!(
                "components:{}",
                components.items.len() + components.omitted
            )];
            answer.extend(
                context
                    .entry_points
                    .items
                    .iter()
                    .map(|e| e.label.split('@').next().unwrap_or_default().to_owned()),
            );
            (answer, context.compression.raw_records)
        }
        "capability_realization" => {
            let name = arg(0)?;
            let capability = find(&model.capabilities, |c| c.name == name, "capability")?;
            let realized = &capability.realized_by;
            (
                vec![realized.status.as_str().to_owned()],
                realized.evidence.len(),
            )
        }
        "gap" => {
            let gap = lens::unanswerable(model, arg(0)?).ok_or("no such gap")?;
            (vec![gap.id, gap.debt], 0)
        }
        "component_purpose" => {
            let path = arg(0)?;
            let component = find(&model.components, |c| c.path == path, "component")?;
            let purpose = &component.purpose;
            (
                vec![purpose.status.as_str().to_owned()],
                purpose.evidence.len(),
            )
        }
        "subsystem_responsibility" => {
            let name = arg(0)?;
            let subsystem = find(&model.subsystems, |s| s.name == name, "subsystem")?;
            let r = &subsystem.responsibility;
            (
                vec![
                    r.value.clone().unwrap_or_default(),
                    r.status.as_str().to_owned(),
                ],
                r.evidence.len(),
            )
        }
        "why" => {
            let why = lens::why(model, arg(0)?, arg(1)?)?;
            let answer = why
                .invariants
                .iter()
                .map(|i| format!("{}|{}", i.id, i.status.as_str()))
                .collect();
            (answer, why.dependencies.len() + why.relations.items.len())
        }
        other => return Err(format!("unknown benchmark op `{other}`")),
    })
}

/// Run every case. A case is correct when each expectation holds: `x` must equal an answer item,
/// `contains:x` must be a substring of one.
pub fn run_benchmark(model: &WorldModel, cases: &[BenchmarkCase]) -> Vec<BenchmarkResult> {
    cases
        .iter()
        .map(|case| {
            let (answer, evidence) =
                answer(model, case).unwrap_or_else(|e| (vec![format!("ERROR: {e}")], 0));
            let correct = case
                .expect
                .iter()
                .all(|x| match x.strip_prefix("contains:") {
                    Some(part) => answer.iter().any(|a| a.contains(part)),
                    None => answer.contains(x),
                });
            BenchmarkResult {
                id: case.id.clone(),
                class: case.class.clone(),
                correct,
                honest_unknown: (case.class == "UNKNOWN_HONESTY").then_some(correct),
                answer,
                evidence,
            }
        })
        .collect()
}

pub fn read_benchmark(path: impl AsRef<Path>) -> io::Result<Vec<BenchmarkCase>> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::EpistemicStatus;
    use atlas_core::composition::{InvariantKind, RelationKind, lens};
    use std::path::PathBuf;
    use std::process::Command;

    const ADL: &str = r#"atlas 1

system Fixture

entity Runtime Core {
    kind = backend
    language = rust
    responsibility = storage
}

entity Runtime App {
    kind = backend
    language = rust
    responsibility = entry
}

capability Persist {
    input = Store
    output = File
}

Core ->provides-> Persist
App ->depends_on-> Core

materialize Core {
    path = "core"
    language = rust
    glob = "src/**/*.rs"
}

materialize App {
    path = "app"
    language = rust
    glob = "src/**/*.rs"
}
"#;

    const CORE_LIB: &str = "//! The shared core.\n/// Holds one count and persists it.\npub mod store;\n\
                            pub fn helper(x: u64) -> u64 { x + 1 }\n";
    const STORE: &str = r#"pub struct Store { count: u64 }
impl Store {
    pub fn new() -> Self { Store { count: 0 } }
    pub fn bump(&mut self) { self.count = crate::helper(self.count); }
    pub fn save(&self) { std::fs::write("out", self.count.to_string()).unwrap(); }
    pub fn get(&self) -> u64 { self.count }
}
"#;
    const MAIN: &str = r#"fn main() {
    let mut s = core::store::Store::new();
    s.bump();
    run(&s);
}
fn run(s: &core::store::Store) { s.save(); }
"#;

    /// A two-crate git repository: `app` depends on `core`.
    fn fixture(name: &str, adl: &str, core_lib: &str) -> PathBuf {
        fixture_with(
            name,
            adl,
            core_lib,
            STORE,
            BASE_LIB,
            BASE_MANIFEST,
            BASE_LOCK,
        )
    }

    const BASE_LIB: &str = "pub fn zero() -> u64 { 0 }\n";
    const BASE_MANIFEST: &str =
        "[package]\nname = \"base\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
    const BASE_LOCK: &str = "[[package]]\nname = \"base\"\nversion = \"0.1.0\"\n";

    fn fixture_with(
        name: &str,
        adl: &str,
        core_lib: &str,
        store: &str,
        base_lib: &str,
        base_manifest: &str,
        base_lock: &str,
    ) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("atlas-agent-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let lock = format!(
            "version = 4\n\n[[package]]\nname = \"app\"\nversion = \"0.1.0\"\n\
             dependencies = [\n \"core\",\n]\n\n{base_lock}\n\
             [[package]]\nname = \"core\"\nversion = \"0.1.0\"\ndependencies = [\n \"base\",\n]\n"
        );
        let files = [
            (
                "Cargo.toml",
                "[workspace]\nresolver = \"2\"\nmembers = [\"core\", \"app\", \"base\"]\n",
            ),
            (
                "core/Cargo.toml",
                "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
                 [dependencies]\nbase = { path = \"../base\" }\n",
            ),
            ("base/Cargo.toml", base_manifest),
            ("base/src/lib.rs", base_lib),
            (
                "app/Cargo.toml",
                "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
                 [dependencies]\ncore = { path = \"../core\" }\n",
            ),
            ("Cargo.lock", lock.as_str()),
            ("core/src/lib.rs", core_lib),
            ("core/src/store.rs", store),
            ("app/src/main.rs", MAIN),
            (".atlas/declared/system.adl", adl),
        ];
        for (path, text) in files {
            let path = dir.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        for args in [
            &["init", "--quiet"][..],
            &["add", "-A"],
            &[
                "-c",
                "user.name=a",
                "-c",
                "user.email=a@b",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        ] {
            assert!(
                Command::new("git")
                    .args(args)
                    .current_dir(&dir)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        dir
    }

    fn model_of(name: &str, adl: &str, core_lib: &str) -> WorldModel {
        let dir = fixture(name, adl, core_lib);
        let model = world_model(&dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        model
    }

    fn function<'m>(
        model: &'m WorldModel,
        name: &str,
    ) -> &'m atlas_core::composition::FunctionBehavior {
        model.functions.iter().find(|f| f.name == name).unwrap()
    }

    #[test]
    fn composition_joins_every_dimension_into_function_component_subsystem_and_architecture() {
        let model = model_of("compose", ADL, CORE_LIB);
        // Level 1: signature, resolved calls, unresolved calls, state, effects.
        let bump = function(&model, "bump");
        assert_eq!(bump.owner_type.as_deref(), Some("Store"));
        assert_eq!(bump.visibility.as_deref(), Some("pub"));
        assert_eq!(bump.calls, vec![function(&model, "helper").id.clone()]);
        let main = function(&model, "main");
        let new = function(&model, "new");
        let run = function(&model, "run");
        assert!(main.calls.contains(&new.id) && main.calls.contains(&run.id));
        assert_eq!(main.unresolved_calls, 1, "s.bump() needs a receiver type");
        assert_eq!(run.unresolved_calls, 1, "s.save() needs a receiver type");
        let save = function(&model, "save");
        let writes: Vec<_> = save
            .effects
            .iter()
            .filter(|e| e.kind == "FILESYSTEM_WRITE")
            .collect();
        assert_eq!(
            writes.len(),
            1,
            "the syntactic and resolved records of one site are one site"
        );
        assert_eq!(
            writes[0].status,
            EpistemicStatus::Derived,
            "the stronger engine's status"
        );
        let get = function(&model, "get");
        let count = model.state.iter().find(|v| v.key == "Store.count").unwrap();
        assert_eq!(count.writers, vec![bump.id.clone()]);
        assert!(count.readers.contains(&get.id));
        // Level 2: components and their dependencies.
        let store = model
            .components
            .iter()
            .find(|c| c.path == "core/src/store.rs")
            .unwrap();
        assert_eq!(store.module.as_deref(), Some("core::store"));
        assert_eq!(store.subsystem.as_deref(), Some("Core"));
        assert_eq!(store.depends_on.get("core/src/lib.rs"), Some(&1));
        // G128: purpose is the author's documentation -- the file's own `//!` text, else the
        // `///` text on the `mod` item declaring it -- and UNKNOWN without one, never a name.
        let purpose = |path: &str| {
            let c = model.components.iter().find(|c| c.path == path).unwrap();
            (
                c.purpose.status,
                c.purpose.value.clone(),
                c.purpose.evidence.len(),
            )
        };
        assert_eq!(
            purpose("core/src/lib.rs"),
            (
                EpistemicStatus::Declared,
                Some("The shared core.".into()),
                1
            )
        );
        assert_eq!(
            purpose("core/src/store.rs"),
            (
                EpistemicStatus::Declared,
                Some("Holds one count and persists it.".into()),
                1
            )
        );
        assert!(store.purpose.basis.contains("mod item"));
        assert_eq!(
            purpose("app/src/main.rs"),
            (EpistemicStatus::Unknown, None, 0),
            "purpose is never named from identifiers"
        );
        let gap = model
            .gaps
            .iter()
            .find(|g| g.id == "GAP-COMPONENT-PURPOSE")
            .unwrap();
        assert_eq!(
            gap.magnitude,
            model
                .components
                .iter()
                .filter(|c| c.purpose.status == EpistemicStatus::Unknown)
                .count(),
            "the gap counts the components still without a purpose"
        );
        // Level 3: declared subsystems with declared responsibility and observed interface.
        let core = model.subsystems.iter().find(|s| s.name == "Core").unwrap();
        assert_eq!(core.responsibility.value.as_deref(), Some("storage"));
        assert_eq!(core.responsibility.status, EpistemicStatus::Declared);
        assert_eq!(core.capabilities, vec!["Persist".to_owned()]);
        assert!(core.interface.contains(&new.id));
        assert!(core.state_owned.contains(&"Store.count".to_owned()));
        // Level 4: the capability is declared; its realization is not claimed.
        let persist = &model.capabilities[0];
        assert_eq!(persist.provided_by, vec!["Core".to_owned()]);
        assert_eq!(persist.realized_by.status, EpistemicStatus::Unknown);
        // Level 5: the declared dependency is built by Cargo and observed in resolved calls.
        let edge = model
            .architecture
            .dependencies
            .iter()
            .find(|d| d.from == "App" && d.to == "Core")
            .unwrap();
        assert_eq!((edge.verdict.as_str(), edge.cargo), ("CONFIRMED", true));
        assert_eq!(
            model.architecture.unplaced_members,
            vec!["base".to_owned()],
            "a member seen only as a provider is never placed by its name"
        );
        // Relations are typed; none claims causation.
        assert!(
            model
                .relations
                .iter()
                .any(|r| r.kind == RelationKind::StateFlow && r.from == bump.id && r.to == get.id)
        );
        assert!(
            model
                .relations
                .iter()
                .any(|r| r.kind == RelationKind::SuppliesData)
        );
        // Invariants: Cargo makes the reverse dependency impossible (OBSERVED); the state writer
        // set is only INFERRED while STATE coverage is not proven.
        let dep = model
            .invariants
            .iter()
            .find(|i| i.id == "INV-DEPENDENCY:Core->App")
            .unwrap();
        assert_eq!(dep.status, EpistemicStatus::Observed);
        assert!(
            !model
                .invariants
                .iter()
                .any(|i| i.id == "INV-DEPENDENCY:App->Core")
        );
        let state = model
            .invariants
            .iter()
            .find(|i| i.id == "INV-STATE:Store.count")
            .unwrap();
        assert_ne!(state.status, EpistemicStatus::Observed);
        if state.status == EpistemicStatus::Inferred {
            assert!(
                !state.residual.is_empty(),
                "an INFERRED universal names its residual"
            );
        }
        assert!(
            model
                .invariants
                .iter()
                .any(|i| i.kind == InvariantKind::Authority
                    && i.id == "INV-AUTHORITY:Core:FILESYSTEM_WRITE"
                    && i.evidence == vec![save.id.clone()])
        );
        // Accounting: every function carries the records composed into it.
        assert!(model.functions.iter().all(|f| f.records > 0));
    }

    /// Composition is a function of the records, not of their order: the same census with its
    /// records and obligations reversed composes into the identical model.
    #[test]
    fn composition_is_independent_of_record_order() {
        let dir = fixture("order", ADL, CORE_LIB);
        let mut report = crate::systemize(&dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let forward = compose(&report);
        report.census.typed_semantic_records.reverse();
        report.census.typed_obligations.reverse();
        assert_eq!(model_digest(&forward), model_digest(&compose(&report)));
    }

    #[test]
    fn mission_context_is_bounded_structural_and_independent_of_the_mission_text() {
        let model = model_of("mission", ADL, CORE_LIB);
        let a = lens::understand(&model, "subsystem:Core", "add a persisted counter").unwrap();
        let b = lens::understand(&model, "Core", "rename the store").unwrap();
        assert_eq!(a.mission.status, EpistemicStatus::Declared);
        assert_ne!(a.mission.text, b.mission.text);
        assert_eq!(
            structural_digest(&a),
            structural_digest(&b),
            "mission text never selects facts"
        );
        let new = function(&model, "new");
        assert!(
            a.entry_points.items.iter().any(|e| e.id == new.id),
            "reached from App"
        );
        assert!(a.impact_frontier.callers >= 1);
        assert!(a.impact_frontier.residual.starts_with("LOWER BOUND"));
        assert!(a.effects.iter().any(|e| e.category == "FILESYSTEM_WRITE"));
        assert!(a.resources.starts_with("UNKNOWN"));
        assert!(
            a.subsystems
                .iter()
                .any(|s| s.name == "App" && s.role == "NEIGHBOR")
        );
        assert!(a.compression.composed_objects > 0 && a.compression.raw_records > 0);
        assert!(!a.next_questions.is_empty());
    }

    #[test]
    fn impact_trace_and_hypotheses_never_turn_unresolved_into_absent() {
        let model = model_of("lens", ADL, CORE_LIB);
        let impact = lens::impact(&model, &["fn:helper"]).unwrap();
        let bump = function(&model, "bump");
        assert!(
            impact
                .frontier
                .nearest
                .items
                .iter()
                .any(|f| f.id == bump.id)
        );
        assert!(
            !impact
                .frontier
                .nearest
                .items
                .iter()
                .any(|f| f.label.starts_with("main")),
            "main reaches bump only through an unresolved call"
        );
        assert!(
            impact.state_readers.items.is_empty(),
            "helper writes no state"
        );
        let reached = lens::trace(&model, "fn:Store::bump", "fn:helper").unwrap();
        assert_eq!(
            (reached.verdict.as_str(), reached.steps.len()),
            ("PATH_OBSERVED", 1)
        );
        let open = lens::trace(&model, "fn:main", "fn:helper").unwrap();
        assert_eq!(open.verdict, "NO_PATH_OBSERVED");
        assert_eq!(
            open.status,
            EpistemicStatus::Unknown,
            "no path observed is not no path"
        );
        let check = |h: &str| lens::hypothesis(&model, h).unwrap().outcome;
        assert_eq!(check("never-invokes:Core:App"), "VALIDATED");
        assert_eq!(check("invokes:main:run"), "VALIDATED");
        assert_eq!(check("effect:save:FILESYSTEM_WRITE"), "VALIDATED");
        assert_eq!(check("writes-only:Store.count:new"), "FALSIFIED");
        assert_ne!(check("writes-only:Store.count:bump"), "FALSIFIED");
        assert_eq!(
            check("invokes:main:helper"),
            "FALSIFIED",
            "main's only unresolved site is spelled `.bump`, and its CALL coverage is OBSERVED"
        );
        assert_eq!(check("invokes:main:bump"), "STILL_HYPOTHESIZED");
        // G123: an unresolved `.bump()` makes main an INFERRED candidate caller of bump, never
        // a resolved one.
        let main = function(&model, "main");
        assert_eq!(main.unresolved_callee_names.get("bump"), Some(&1));
        let bump = lens::impact(&model, &["fn:Store::bump"]).unwrap().frontier;
        assert!(bump.candidate_callers.items.iter().any(|c| c.id == main.id));
        assert!(!bump.nearest.items.iter().any(|c| c.id == main.id));
        assert!(
            lens::understand(&model, "subsystem:Cor", "").is_err(),
            "no fuzzy selectors"
        );
        assert!(lens::understand(&model, "fn:hlper", "").is_err());
        assert!(
            lens::understand(&model, "path:core/src/stor", "").is_err(),
            "a prefix ends at a separator"
        );
        let scoped = lens::understand(&model, "path:core/src", "").unwrap();
        assert_eq!(
            scoped.scope.components.items,
            ["core/src/lib.rs", "core/src/store.rs"]
        );
    }

    #[test]
    fn an_undeclared_dependency_is_a_conflict_and_verify_reports_new_authority() {
        let undeclared = ADL.replace("App ->depends_on-> Core\n", "");
        let model = model_of("undeclared", &undeclared, CORE_LIB);
        let edge = model
            .architecture
            .dependencies
            .iter()
            .find(|d| d.from == "App" && d.to == "Core")
            .unwrap();
        assert_eq!(
            (edge.verdict.as_str(), edge.status),
            ("UNDECLARED", EpistemicStatus::Conflict),
            "a built dependency the architecture does not declare"
        );
        let before = model_of("before", ADL, CORE_LIB);
        let env_reading = "pub mod store;\npub fn helper(x: u64) -> u64 { let _ = std::env::var(\"K\"); x + 1 }\n";
        let after = model_of("after", ADL, env_reading);
        let delta = lens::verify(&before, &after);
        assert!(
            delta
                .new_authority
                .contains(&"Core: ENVIRONMENT_READ".to_owned()),
            "{delta:?}"
        );
        assert!(
            delta
                .new
                .iter()
                .any(|i| i.starts_with("INV-AUTHORITY:") && i.ends_with(":ENVIRONMENT_READ"))
        );
        assert_eq!(lens::verify(&before, &before).verdict, "HELD");
    }

    /// G129 (NA-IMPACT-CLOSURE): the closure of a change to `core/src/store.rs` holds every
    /// difference of the full recompute -- a caller of a removed function (R3), a callee that
    /// gains a caller (R2), and a call in an unchanged file that an added function now resolves
    /// (R4) -- and no more than it must: a caller of an unchanged function stays outside.
    #[test]
    fn the_impact_closure_holds_every_difference_of_the_full_recompute() {
        let core_lib = format!("{CORE_LIB}pub fn uses_extra() -> u64 {{ store::extra() }}\n");
        let changed_store = STORE
            .replace("    pub fn new() -> Self { Store { count: 0 } }\n", "")
            .replace(
                "std::fs::write(\"out\", self.count.to_string()).unwrap();",
                "std::fs::write(\"out\", crate::helper(self.count).to_string()).unwrap();",
            )
            + "pub fn extra() -> u64 { 1 }\n";
        assert_ne!(changed_store, STORE);
        let model = |name: &str, store: &str| {
            let dir = fixture_with(
                name,
                ADL,
                &core_lib,
                store,
                BASE_LIB,
                BASE_MANIFEST,
                BASE_LOCK,
            );
            let model = world_model(&dir).unwrap();
            std::fs::remove_dir_all(&dir).unwrap();
            model
        };
        // Record ids carry the revision: the closure works over the normalized models.
        let before = normalize(&model("closure-before", STORE));
        let after = normalize(&model("closure-after", &changed_store));
        // Normalizing is what makes two revisions comparable: the same code in another commit.
        let again = model("closure-again", STORE);
        assert_ne!(again.revision, before.revision);
        assert_eq!(normalize(&again), before);
        let changed = vec!["core/src/store.rs".to_owned()];
        let result = closure::impact_closure(&before, &after, &changed);
        let oracle = closure::oracle(&result, &before, &after);
        assert_eq!(oracle.verdict, "SOUND", "{oracle:#?}");
        assert!(!result.global);
        let id = |model: &WorldModel, name: &str| function(model, name).id.clone();
        for (name, rule) in [
            ("main", "R3_CALLER_OF_REMOVED"),
            ("helper", "R2_CALLEE"),
            ("uses_extra", "R4_NAME_CANDIDATE"),
        ] {
            assert!(
                result.affected_functions.contains(&id(&before, name)),
                "{name} by {rule}"
            );
            assert!(result.rules.contains_key(rule), "{rule}");
        }
        // Precision: `run` calls `save`, which keeps its identity, so `run` is only a frontier.
        assert!(!result.affected_functions.contains(&id(&before, "run")));
        assert!(result.transitive_dependents >= 1);
        // Every level of the full recompute changed, and every change is inside the closure.
        assert!(oracle.functions.changed >= 5 && oracle.functions.missed.is_empty());
        assert!(oracle.components.changed >= 2);
        // A manifest makes the closure global.
        let global = closure::impact_closure(&before, &after, &["core/Cargo.toml".to_owned()]);
        assert!(global.global);
        assert_eq!(global.affected_functions.len(), before.functions.len());
        assert_eq!(global.affected_invariants.len(), before.invariants.len());
        // A fixture's manifest nested inside a subsystem is not the workspace's.
        let nested = closure::impact_closure(
            &before,
            &after,
            &["core/tests/fixtures/x/Cargo.toml".to_owned()],
        );
        assert!(!nested.global);
        // An empty change affects nothing, and the oracle names what it then misses.
        let empty = closure::impact_closure(&before, &after, &[]);
        assert!(empty.affected_functions.is_empty());
        let missed = closure::oracle(&empty, &before, &after);
        assert_eq!(missed.verdict, "UNSOUND");
        assert_eq!(missed.functions.missed.len(), missed.functions.changed);
    }

    /// G130 (NA-CONSTRAINT-NONGROUND): invariants quantified over census truth are decided over
    /// the composed census with three values, identically by every entry point.
    #[test]
    fn census_quantified_invariants_are_decided_with_three_values() {
        let adl = format!(
            "{ADL}\ninvariant CoreNeverCallsApp {{\n    forall f: function in Core forbid call to App\n}}\n\
             invariant AppNeverCallsCore {{\n    forall f: function in App forbid call to Core\n}}\n\
             invariant CoreWritesNoFiles {{\n    forall f: function in Core forbid effect FILESYSTEM_WRITE\n}}\n\
             invariant AppSpawnsNothing {{\n    forall f: function in App forbid effect PROCESS_SPAWN\n}}\n\
             invariant NoSuchSubsystem {{\n    forall f: function in Ghost forbid call to Core\n}}\n\
             invariant BaseSpawnsNothing {{\n    forall f: function in Base forbid effect PROCESS_SPAWN\n}}\n\
             invariant CoreNeverCallsBase {{\n    forall f: function in Core forbid call to Base\n}}\n\
             invariant AppNeverCallsBase {{\n    forall f: function in App forbid call to Base\n}}\n\
             entity Runtime Base {{\n    kind = backend\n    language = rust\n}}\n\n\
             materialize Base {{\n    path = \"base\"\n    language = rust\n    glob = \"src/**/*.rs\"\n}}\n"
        );
        // Base also defines `bump`: App's unresolved `s.bump()` may reach it by name.
        let dir = fixture_with(
            "quantified",
            &adl,
            CORE_LIB,
            STORE,
            "pub fn zero() -> u64 { 0 }\npub fn bump() {}\n",
            BASE_MANIFEST,
            BASE_LOCK,
        );
        let report = crate::systemize(&dir).unwrap();
        let compiled = crate::check(&dir).unwrap();
        let analysis = crate::code_analyze(&dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        use atlas_core::constraint::ConstraintVerdict::{Satisfied, Unknown, Violated};
        let census_invariants = [
            ("CoreNeverCallsApp", Satisfied),
            ("CoreNeverCallsBase", Satisfied),
            ("AppNeverCallsCore", Violated),
            ("CoreWritesNoFiles", Violated),
            ("AppSpawnsNothing", Unknown),
            ("BaseSpawnsNothing", Unknown),
            ("NoSuchSubsystem", Unknown),
            ("AppNeverCallsBase", Unknown),
        ];
        let result = |name: &str| {
            report
                .adl
                .constraint_results
                .iter()
                .find(|r| r.name == name)
                .unwrap()
        };
        for (name, verdict) in census_invariants {
            let r = result(name);
            assert_eq!(r.verdict, verdict, "{name}: {r:?}");
            assert_eq!(r.passed, verdict == Satisfied);
            // The ADL compiler alone cannot decide it: UNKNOWN, never a pass.
            let alone = compiled
                .constraint_results
                .iter()
                .find(|r| r.name == name)
                .unwrap();
            assert_eq!(alone.verdict, Unknown, "{name}");
            assert!(alone.diagnostics.iter().any(|d| d.code == "ATLAS-E063"));
            // Every entry point decides it the same way.
            let analyzed = analysis["adl"]["constraint_results"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["name"] == name)
                .unwrap();
            assert_eq!(analyzed["verdict"], serde_json::to_value(verdict).unwrap());
        }
        // A violation names its counterexample; SATISFIED and UNKNOWN name what they rest on.
        assert!(
            result("AppNeverCallsCore")
                .diagnostics
                .iter()
                .any(|d| d.code == "ATLAS-E064"
                    && d.message.contains("main@app/src/main.rs")
                    && d.message.contains("new@core/src/store.rs"))
        );
        assert!(
            result("CoreWritesNoFiles").diagnostics[0]
                .message
                .contains("save@core/src/store.rs")
        );
        assert!(
            result("CoreNeverCallsApp").derivation[0].basis[0].contains("no Cargo dependency path")
        );
        assert!(result("CoreNeverCallsBase").derivation[0].basis[0].contains("CALL is OBSERVED"));
        assert!(result("BaseSpawnsNothing").derivation[0].basis[0].contains("EFFECT not OBSERVED"));
        assert!(
            result("AppNeverCallsBase").derivation[0]
                .basis
                .iter()
                .any(|b| b.contains("spelled with the name of a function of Base")),
            "{:?}",
            result("AppNeverCallsBase")
        );
        // Admission reads the decided verdicts.
        let blockers = &report.coding_admission.blockers;
        assert!(blockers.contains(&"ADL_CONSTRAINT_VIOLATED".to_owned()));
        assert!(blockers.contains(&"ADL_CONSTRAINT_UNKNOWN".to_owned()));
        // The world model carries them as invariants.
        let model = compose(&report);
        let status = |id: &str| model.invariants.iter().find(|i| i.id == id).unwrap().status;
        assert_eq!(
            status("INV-ADL:CoreNeverCallsApp"),
            EpistemicStatus::Derived
        );
        assert_eq!(
            status("INV-ADL:AppNeverCallsCore"),
            EpistemicStatus::Conflict
        );
        assert_eq!(
            status("INV-ADL:BaseSpawnsNothing"),
            EpistemicStatus::Unknown
        );
        let kind = |id: &str| model.invariants.iter().find(|i| i.id == id).unwrap().kind;
        assert_eq!(kind("INV-ADL:AppNeverCallsCore"), InvariantKind::Dependency);
        assert_eq!(kind("INV-ADL:CoreWritesNoFiles"), InvariantKind::Authority);
        // A change to App's code can change a verdict over App's calls: the impact closure holds it.
        let normalized = normalize(&model);
        let closure =
            closure::impact_closure(&normalized, &normalized, &["app/src/main.rs".to_owned()]);
        assert!(
            closure
                .affected_invariants
                .contains(&"INV-ADL:AppNeverCallsCore".to_owned())
        );
        // Counterfactual: were EFFECT OBSERVED on Base's source, the same census would decide it.
        let mut observed = model.clone();
        for c in observed
            .components
            .iter_mut()
            .filter(|c| c.path == "base/src/lib.rs")
        {
            c.coverage
                .insert("EFFECT".into(), EpistemicStatus::Observed);
        }
        use atlas_core::composition::quantified;
        use atlas_core::language::adl::CensusForbidden;
        let spawn = CensusForbidden::Effect {
            category: "PROCESS_SPAWN".into(),
        };
        assert_eq!(quantified::decide(&model, "Base", &spawn).verdict, Unknown);
        assert_eq!(
            quantified::decide(&observed, "Base", &spawn).verdict,
            Satisfied
        );
    }

    /// G128: a module file's own `//!` documentation outranks the `///` on its `mod` item.
    #[test]
    fn a_module_file_s_own_documentation_outranks_its_declaration() {
        let store = format!("//! Its own words.\n{STORE}");
        let dir = fixture_with(
            "ownwords",
            ADL,
            CORE_LIB,
            &store,
            BASE_LIB,
            BASE_MANIFEST,
            BASE_LOCK,
        );
        let model = world_model(&dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let store = model
            .components
            .iter()
            .find(|c| c.path == "core/src/store.rs")
            .unwrap();
        assert_eq!(store.purpose.value.as_deref(), Some("Its own words."));
        assert!(store.purpose.basis.contains("own documentation"));
    }

    /// A crate that calls into a crate it has no Cargo path to: the call is a CONFLICT and the
    /// dependency invariant it contradicts is CONFLICT too, never OBSERVED.
    #[test]
    fn a_call_without_a_dependency_path_contradicts_the_dependency_invariant() {
        let adl = format!(
            "{ADL}\nentity Runtime Base {{\n    kind = backend\n    language = rust\n}}\n\n\
             materialize Base {{\n    path = \"base\"\n    language = rust\n    glob = \"src/**/*.rs\"\n}}\n"
        );
        let dir = fixture_with(
            "nodep",
            &adl,
            CORE_LIB,
            STORE,
            "pub fn zero() -> u64 { app_util::one() }\npub mod app_util { pub fn one() -> u64 { crate::zero() } }\n\
             pub fn reach() -> u64 { core::helper(1) }\n",
            "[package]\nname = \"base\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
             [dependencies]\nserde = \"1\"\n",
            "[[package]]\nname = \"base\"\nversion = \"0.1.0\"\ndependencies = [\n \"serde\",\n]\n\n\
             [[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n\
             source = \"registry+https://github.com/rust-lang/crates.io-index\"\n\
             checksum = \"0000000000000000000000000000000000000000000000000000000000000000\"\n",
        );
        let mut report = crate::systemize(&dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let model = compose(&report);
        let reach = function(&model, "reach");
        assert_eq!(reach.subsystem.as_deref(), Some("Base"));
        assert!(
            reach.calls.is_empty(),
            "resolution never crosses a missing dependency"
        );
        let invariant = model
            .invariants
            .iter()
            .find(|i| i.id == "INV-DEPENDENCY:Base->Core")
            .unwrap();
        assert_eq!(invariant.status, EpistemicStatus::Observed);
        // Inject what a wrong resolution engine would claim: `reach` resolved into Core.
        let helper = function(&model, "helper").id.clone();
        let injected = report
            .census
            .typed_semantic_records
            .iter()
            .find_map(|r| match r {
                atlas_core::semantic::SemanticObservation::Call(h)
                    if h.subject.function.as_str() == reach.id =>
                {
                    let mut h = h.clone();
                    h.status = EpistemicStatus::Derived;
                    h.subject.callees =
                        vec![serde_json::from_value(serde_json::json!(helper)).unwrap()];
                    Some(atlas_core::semantic::SemanticObservation::Call(h))
                }
                _ => None,
            })
            .unwrap();
        report.census.typed_semantic_records.push(injected);
        let model = compose(&report);
        let edge = model
            .architecture
            .dependencies
            .iter()
            .find(|d| d.from == "Base" && d.to == "Core")
            .unwrap();
        assert_eq!(
            (edge.verdict.as_str(), edge.status),
            ("CALL_WITHOUT_DEPENDENCY", EpistemicStatus::Conflict)
        );
        let invariant = model
            .invariants
            .iter()
            .find(|i| i.id == "INV-DEPENDENCY:Base->Core")
            .unwrap();
        assert_eq!(
            invariant.status,
            EpistemicStatus::Conflict,
            "the counterexample call falsifies it"
        );
        assert!(!invariant.evidence.is_empty());
    }

    /// A world model written before callee names existed (G122) claims no narrowing: every
    /// unresolved site without a recorded name may reach anything.
    #[test]
    fn a_model_without_callee_names_never_narrows_impact() {
        let model = model_of("oldmodel", ADL, CORE_LIB);
        let mut json = serde_json::to_value(&model).unwrap();
        for f in json["functions"].as_array_mut().unwrap() {
            let f = f.as_object_mut().unwrap();
            f.remove("unresolved_callee_names");
            f.remove("unnamed_unresolved_calls");
        }
        let old: WorldModel = serde_json::from_value(json).unwrap();
        let unresolved: usize = old.functions.iter().map(|f| f.unresolved_calls).sum();
        let frontier = lens::impact(&old, &["fn:Store::bump"]).unwrap().frontier;
        assert_eq!(frontier.unnamed_sites, unresolved);
        assert_eq!(frontier.candidate_sites, 0);
        assert_eq!(
            lens::hypothesis(&old, "invokes:main:helper")
                .unwrap()
                .outcome,
            "STILL_HYPOTHESIZED"
        );
    }

    /// Atlas composed from its own census: every join lands, the declared architecture reconciles
    /// with Cargo and resolved calls, and composition is deterministic.
    #[test]
    fn atlas_self_world_model_is_complete_and_deterministic() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let report = crate::systemize(&root).unwrap();
        let model = compose(&report);
        assert_eq!(model_digest(&model), model_digest(&compose(&report)));
        assert_eq!(
            model.accounting.raw_records(),
            report.census.typed_semantic_records.len()
        );
        assert_eq!(
            model.accounting.obligations,
            report.census.typed_obligations.len()
        );
        assert!(
            model.functions.iter().all(|f| f.visibility.is_some()),
            "every signature joins"
        );
        let resolved_sites: usize = model
            .relations
            .iter()
            .filter(|r| r.kind == RelationKind::Invokes)
            .map(|r| r.weight)
            .sum();
        assert!(resolved_sites > 0);
        let names: Vec<&str> = model.subsystems.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Adapter", "AtlasCli", "Core", "Runtime", "WebUI"]);
        assert!(
            model.architecture.unassigned_components.is_empty()
                || model
                    .architecture
                    .unassigned_components
                    .iter()
                    .all(|p| !p.ends_with(".rs")),
            "every Rust source belongs to a declared subsystem"
        );
        for (from, to) in [
            ("Core", "Runtime"),
            ("Core", "Adapter"),
            ("Core", "AtlasCli"),
            ("Adapter", "Runtime"),
        ] {
            let id = format!("INV-DEPENDENCY:{from}->{to}");
            let invariant = model
                .invariants
                .iter()
                .find(|i| i.id == id)
                .unwrap_or_else(|| panic!("{id}"));
            assert_eq!(
                invariant.status,
                EpistemicStatus::Observed,
                "{id}: no counterexample call"
            );
        }
        let cli_core = model
            .architecture
            .dependencies
            .iter()
            .find(|d| d.from == "AtlasCli" && d.to == "Core")
            .unwrap();
        assert_eq!(
            cli_core.verdict, "TRANSITIVE_VIA_REEXPORT",
            "the CLI reaches Core through Runtime"
        );
        assert!(
            model
                .architecture
                .dependencies
                .iter()
                .all(|d| d.status != EpistemicStatus::Conflict)
        );
        assert!(
            model
                .invariants
                .iter()
                .all(|i| i.status != EpistemicStatus::Conflict)
        );
        // STATE is not OBSERVED on any Atlas file: no state writer set may be claimed universal.
        let state: Vec<_> = model
            .invariants
            .iter()
            .filter(|i| i.kind == InvariantKind::State)
            .collect();
        assert!(!state.is_empty());
        assert!(
            state
                .iter()
                .all(|i| i.status == EpistemicStatus::Inferred && !i.residual.is_empty())
        );
        // The self-census has unresolved calls: every authority invariant is INFERRED.
        assert!(
            model
                .invariants
                .iter()
                .filter(|i| i.kind == InvariantKind::Authority)
                .all(|i| i.status == EpistemicStatus::Inferred)
        );
        // Purpose is declared or UNKNOWN, never named from an identifier: every DECLARED purpose
        // is the summary of the documented SYMBOL record it cites (G128).
        let documented: std::collections::BTreeMap<&str, &str> = report
            .census
            .typed_semantic_records
            .iter()
            .filter_map(|r| match r {
                atlas_core::SemanticObservation::Symbol(h) => Some((
                    h.record_id.as_str(),
                    h.subject.documentation.as_ref()?.summary.as_str(),
                )),
                _ => None,
            })
            .collect();
        let mut declared = 0;
        for c in &model.components {
            match c.purpose.status {
                EpistemicStatus::Unknown => assert!(c.purpose.value.is_none()),
                EpistemicStatus::Declared => {
                    declared += 1;
                    assert_eq!(
                        c.purpose.value.as_deref(),
                        documented.get(c.purpose.evidence[0].as_str()).copied(),
                        "{}",
                        c.path
                    );
                }
                other => panic!("{}: purpose {other:?}", c.path),
            }
        }
        assert!(declared > 0, "the workspace documents its modules");
        assert!(model.subsystems.iter().all(|s| matches!(
            s.responsibility.status,
            EpistemicStatus::Declared | EpistemicStatus::Unknown
        )));
        // G123: every unresolved call site is named or counted as unnamed; the gap is the
        // unnamed remainder, and most unresolved sites are named.
        for f in &model.functions {
            assert_eq!(
                f.unresolved_calls,
                f.unnamed_unresolved_calls + f.unresolved_callee_names.values().sum::<usize>(),
                "{}",
                f.name
            );
        }
        let unnamed: usize = model
            .functions
            .iter()
            .map(|f| f.unnamed_unresolved_calls)
            .sum();
        let unresolved: usize = model.functions.iter().map(|f| f.unresolved_calls).sum();
        let gap = model
            .gaps
            .iter()
            .find(|g| g.id == "GAP-UNRESOLVED-CALLEE")
            .unwrap();
        assert_eq!(gap.magnitude, unnamed);
        assert!(
            unnamed * 10 < unresolved,
            "{unnamed} of {unresolved} unnamed"
        );
        // The Agent-Utility benchmark: every case answered correctly from the model, and every
        // positive answer carries evidence.
        let cases = read_benchmark(root.join(".atlas/evidence/agent/benchmark.json")).unwrap();
        assert!(cases.len() >= 12);
        for result in run_benchmark(&model, &cases) {
            assert!(result.correct, "{}: {:?}", result.id, result.answer);
            if result.class != "UNKNOWN_HONESTY" {
                assert!(
                    result.evidence > 0,
                    "{}: an answer without evidence",
                    result.id
                );
            }
        }
    }
}
