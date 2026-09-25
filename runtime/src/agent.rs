//! Agent-Worn Atlas entry points (G122, ADR 0044): compose the census of a repository into a
//! `WorldModel` and serve the lens operations over it. The composition and every operation are
//! pure (`atlas_core::composition`); this module only runs the census and fixes digests.

use atlas_core::IntegrityDigest;
pub use atlas_core::composition::lens;
use atlas_core::composition::{WorldModel, compose, lens::MissionContext};
use std::{io, path::Path};

/// Census `root` and compose it.
pub fn world_model(root: impl AsRef<Path>) -> io::Result<WorldModel> {
    Ok(compose(&crate::systemize(root)?))
}

pub fn read_world_model(path: impl AsRef<Path>) -> io::Result<WorldModel> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
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

    const CORE_LIB: &str = "pub mod store;\npub fn helper(x: u64) -> u64 { x + 1 }\n";
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
        fixture_with(name, adl, core_lib, BASE_LIB, BASE_MANIFEST, BASE_LOCK)
    }

    const BASE_LIB: &str = "pub fn zero() -> u64 { 0 }\n";
    const BASE_MANIFEST: &str =
        "[package]\nname = \"base\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
    const BASE_LOCK: &str = "[[package]]\nname = \"base\"\nversion = \"0.1.0\"\n";

    fn fixture_with(
        name: &str,
        adl: &str,
        core_lib: &str,
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
            ("core/src/store.rs", STORE),
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
        let count = model
            .state
            .iter()
            .find(|v| v.key == "impl:Store.count")
            .unwrap();
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
        assert_eq!(
            store.purpose.status,
            EpistemicStatus::Unknown,
            "purpose is never named from identifiers"
        );
        // Level 3: declared subsystems with declared responsibility and observed interface.
        let core = model.subsystems.iter().find(|s| s.name == "Core").unwrap();
        assert_eq!(core.responsibility.value.as_deref(), Some("storage"));
        assert_eq!(core.responsibility.status, EpistemicStatus::Declared);
        assert_eq!(core.capabilities, vec!["Persist".to_owned()]);
        assert!(core.interface.contains(&new.id));
        assert!(core.state_owned.contains(&"impl:Store.count".to_owned()));
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
            .find(|i| i.id == "INV-STATE:impl:Store.count")
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
        assert_eq!(check("writes-only:impl:Store.count:new"), "FALSIFIED");
        assert_ne!(check("writes-only:impl:Store.count:bump"), "FALSIFIED");
        assert_eq!(check("invokes:main:helper"), "STILL_HYPOTHESIZED");
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
        // Purpose is declared or UNKNOWN, never named from an identifier.
        assert!(
            model
                .components
                .iter()
                .all(|c| c.purpose.status == EpistemicStatus::Unknown)
        );
        assert!(model.subsystems.iter().all(|s| matches!(
            s.responsibility.status,
            EpistemicStatus::Declared | EpistemicStatus::Unknown
        )));
        let unresolved: usize = model.functions.iter().map(|f| f.unresolved_calls).sum();
        let gap = model
            .gaps
            .iter()
            .find(|g| g.id == "GAP-UNRESOLVED-CALLEE")
            .unwrap();
        assert_eq!(gap.magnitude, unresolved);
    }
}
