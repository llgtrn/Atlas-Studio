use super::*;
use crate::census::extraction::{
    CensusExtractionAccounting, extract_semantics, requested_dimensions,
};
use atlas_core::{ArtifactId, ArtifactKind, ArtifactRecord, RepositoryId, RevisionRef};
use std::time::{SystemTime, UNIX_EPOCH};

fn map(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn an_unrenamed_registry_dependency_is_named_after_its_library() {
    // G177 (review): Cargo passes an unrenamed dependency as `--extern <library name>`. `alpha`'s
    // library is `beta` and `gamma`'s is `alpha`, so `use alpha::T` names `gamma`: neither key
    // may carry its package. `plain` (library name = key), a renamed `ren`, and `absent` (its
    // sources not extracted, so every answer about it is UNKNOWN) keep theirs.
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let registry = std::env::temp_dir().join(format!("atlas-lib-names-{nonce}"));
    let index = registry.join("index.crates.io-0000000000000000");
    for (package, manifest) in [
        (
            "alpha",
            "[package]\nname = \"alpha\"\nversion = \"1.0.0\"\n[lib]\nname = \"beta\"\n",
        ),
        (
            "gamma",
            "[package]\nname = \"gamma\"\nversion = \"1.0.0\"\n[lib]\nname = \"alpha\"\n",
        ),
        (
            "plain",
            "[package]\nname = \"plain\"\nversion = \"1.0.0\"\n",
        ),
        (
            "renamed",
            "[package]\nname = \"renamed\"\nversion = \"1.0.0\"\n[lib]\nname = \"other\"\n",
        ),
    ] {
        let dir = index.join(format!("{package}-1.0.0"));
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("Cargo.toml"), manifest).unwrap();
        fs::write(dir.join("src/lib.rs"), "pub trait T {}\n").unwrap();
        fs::write(dir.join(".cargo-ok"), "").unwrap();
    }
    let registry_package = |name: &str| {
        format!(
            "\n[[package]]\nname = \"{name}\"\nversion = \"1.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n"
        )
    };
    let mut lock = String::from(
        "version = 4\n\n[[package]]\nname = \"app\"\nversion = \"0.1.0\"\ndependencies = [\"absent\", \"alpha\", \"gamma\", \"plain\", \"renamed\"]\n",
    );
    for name in ["absent", "alpha", "gamma", "plain", "renamed"] {
        lock.push_str(&registry_package(name));
    }
    let locked = adapter::read_locked_trait_methods(&lock, &registry);
    let manifests = map(&[(
        "app/Cargo.toml",
        "[package]\nname = \"app\"\n\n[dependencies]\nalpha = \"1\"\ngamma = \"1\"\nplain = \"1\"\nren = { package = \"renamed\", version = \"1\" }\nabsent = \"1\"\n",
    )]);
    let sources = map(&[("app/src/lib.rs", "")]);
    let crates = crate_targets_locked(&manifests, &sources, &locked, None);
    let foreign: Vec<(String, String)> = crates[0]
        .foreign
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    assert_eq!(
        foreign,
        map(&[("absent", "absent"), ("plain", "plain"), ("ren", "renamed")])
            .into_iter()
            .collect::<Vec<_>>()
    );
    fs::remove_dir_all(&registry).unwrap();
}

#[test]
fn every_declared_dependency_names_the_extern_prelude_whatever_its_source() {
    // G178 (review 2): a git, path or unlocked registry dependency is in the target's extern
    // prelude too, contents unknown: its key (`-` as `_`), an unrenamed workspace library's own
    // name, and a binary's own library by its `[lib] name`. Dev dependencies count for the
    // library and binaries (their test builds), build dependencies for the build script alone.
    let manifests = map(&[
        (
            "core/Cargo.toml",
            "[package]\nname = \"core\"\n\n[lib]\nname = \"kernel\"\n",
        ),
        (
            "app/Cargo.toml",
            "[package]\nname = \"atlas-app\"\n\n[lib]\nname = \"app_lib\"\n\n[dependencies]\nlocal = { package = \"core\", path = \"../core\" }\nzip = { git = \"https://example.invalid/zip\" }\next-lib = { path = \"../../elsewhere\" }\nquote = \"1\"\n\n[dev-dependencies]\nproptest = \"1\"\n\n[build-dependencies]\ncc = \"1\"\n",
        ),
        (
            "tool/Cargo.toml",
            "[package]\nname = \"tool\"\n\n[dependencies]\ncore = { path = \"../core\" }\n",
        ),
    ]);
    let sources = map(&[
        ("core/src/lib.rs", ""),
        ("app/src/lib.rs", ""),
        ("app/src/main.rs", ""),
        ("app/build.rs", ""),
        ("tool/src/main.rs", ""),
    ]);
    let described: Vec<(String, Vec<String>)> = crate_targets(&manifests, &sources)
        .into_iter()
        .map(|c| (c.root, c.declared.into_iter().collect()))
        .collect();
    let names = |n: &[&str]| n.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>();
    let library = ["ext_lib", "local", "proptest", "quote", "zip"];
    assert_eq!(
        described,
        vec![
            ("app/src/lib.rs".to_owned(), names(&library)),
            ("core/src/lib.rs".to_owned(), Vec::new()),
            (
                "app/src/main.rs".to_owned(),
                names(&["app_lib", "ext_lib", "local", "proptest", "quote", "zip"])
            ),
            ("app/build.rs".to_owned(), names(&["cc"])),
            ("tool/src/main.rs".to_owned(), names(&["kernel"])),
        ]
    );
}

#[test]
fn cargo_passes_an_unrenamed_workspace_library_by_its_crate_name() {
    // G178 (review 3), checked with Cargo: package `core` with `[lib] name = "kernel"`, declared
    // by `tool` as `core = { path = "../core" }`, is `--extern kernel`: `kernel::mem::swap` is
    // the workspace fn and `core::mem::swap` stays libcore. Package `app`'s binary sees its own
    // library as `applib` (`app::` is E0433). A `package` field -- even one naming the key --
    // renames: Cargo passes the key. An inherited entry takes the root's rename; when the root
    // is not in view, only a key that is the library's own name is sure. An unrenamed key naming
    // another package (`other = { path = "../core" }`) is a Cargo error: nothing binds.
    let manifests = map(&[
        (
            "Cargo.toml",
            "[workspace]\nmembers = [\"core\", \"tool\", \"app\", \"ren\", \"ws\"]\n\n[workspace.dependencies]\ncore = { path = \"core\" }\nsame = { package = \"core\", path = \"core\" }\n",
        ),
        (
            "core/Cargo.toml",
            "[package]\nname = \"core\"\n\n[lib]\nname = \"kernel\"\n",
        ),
        (
            "tool/Cargo.toml",
            "[package]\nname = \"tool\"\n\n[dependencies]\ncore = { path = \"../core\" }\n",
        ),
        (
            "app/Cargo.toml",
            "[package]\nname = \"app\"\n\n[lib]\nname = \"applib\"\n",
        ),
        (
            "ren/Cargo.toml",
            "[package]\nname = \"ren\"\n\n[dependencies]\ncore = { package = \"core\", path = \"../core\" }\n",
        ),
        (
            "ws/Cargo.toml",
            "[package]\nname = \"ws\"\n\n[dependencies]\ncore.workspace = true\nsame.workspace = true\n",
        ),
        (
            "bad/Cargo.toml",
            "[package]\nname = \"bad\"\n\n[dependencies]\nother = { path = \"../core\" }\n",
        ),
    ]);
    let tool = "fn main() {\n    let (mut a, mut b) = (1u8, 2u8);\n    core::mem::swap(&mut a, &mut b);\n    kernel::mem::swap(&mut a, &mut b);\n}\n";
    let sources = map(&[
        (
            "core/src/lib.rs",
            "pub mod mem {\n    pub fn swap(_a: &mut u8, _b: &mut u8) {}\n}\n",
        ),
        ("tool/src/main.rs", tool),
        ("app/src/lib.rs", "pub fn hello() {}\n"),
        (
            "app/src/main.rs",
            "fn main() {\n    app::hello();\n    applib::hello();\n}\n",
        ),
        ("ren/src/main.rs", ""),
        ("ws/src/main.rs", ""),
        ("bad/src/main.rs", ""),
    ]);
    let crates = crate_targets(&manifests, &sources);
    let described: Vec<(String, Vec<String>, Vec<String>)> = (crates.iter())
        .map(|c| {
            let externs = c
                .externs
                .iter()
                .map(|(name, &lib)| format!("{name}={}", crates[lib].root));
            (
                c.root.clone(),
                externs.collect(),
                c.declared.iter().cloned().collect(),
            )
        })
        .collect();
    let v = |n: &[&str]| n.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>();
    assert_eq!(
        described,
        vec![
            ("app/src/lib.rs".to_owned(), v(&[]), v(&[])),
            ("core/src/lib.rs".to_owned(), v(&[]), v(&[])),
            (
                "app/src/main.rs".to_owned(),
                v(&["applib=app/src/lib.rs"]),
                v(&["applib"])
            ),
            (
                "bad/src/main.rs".to_owned(),
                v(&[]),
                v(&["kernel", "other"])
            ),
            (
                "ren/src/main.rs".to_owned(),
                v(&["core=core/src/lib.rs"]),
                v(&["core"])
            ),
            (
                "tool/src/main.rs".to_owned(),
                v(&["kernel=core/src/lib.rs"]),
                v(&["kernel"])
            ),
            (
                "ws/src/main.rs".to_owned(),
                v(&["kernel=core/src/lib.rs", "same=core/src/lib.rs"]),
                v(&["kernel", "same"])
            ),
        ]
    );
    // Without the root manifest in view, `core.workspace = true` may be renamed or not: neither
    // name binds the library, and both are declared.
    let mut far = manifests.clone();
    far.remove("Cargo.toml");
    far.insert(
        "ws/Cargo.toml".into(),
        "[package]\nname = \"ws\"\n\n[dependencies]\ncore.workspace = true\n".into(),
    );
    let ws = crate_targets(&far, &sources)
        .into_iter()
        .find(|c| c.root == "ws/src/main.rs")
        .unwrap();
    assert!(ws.externs.is_empty(), "{:?}", ws.externs);
    assert_eq!(
        ws.declared,
        BTreeSet::from(["core".into(), "kernel".into()])
    );
    let calls = adapter::resolve_path_calls(&crates, &sources);
    let outcome = |path: &str| -> Vec<(String, String)> {
        (calls.iter())
            .filter(|call| call.path == path)
            .map(|call| {
                let outcome = match &call.outcome {
                    adapter::PathCallOutcome::Resolved(t) => format!("{}:{}", t.path, t.name),
                    adapter::PathCallOutcome::External(path) => format!("external:{path}"),
                    other => format!("{other:?}").chars().take(12).collect(),
                };
                (call.callee.clone(), outcome)
            })
            .collect()
    };
    let pairs = |p: &[(&str, &str)]| -> Vec<(String, String)> {
        p.iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    };
    assert_eq!(
        outcome("tool/src/main.rs"),
        pairs(&[
            ("core::mem::swap", "external:core::mem::swap"),
            ("kernel::mem::swap", "core/src/lib.rs:swap"),
        ])
    );
    let app = outcome("app/src/main.rs");
    assert_eq!(
        app[1],
        (
            "applib::hello".to_owned(),
            "app/src/lib.rs:hello".to_owned()
        )
    );
    assert!(!app[0].1.starts_with("app/"), "{app:?}");
}

#[test]
fn a_dependency_table_in_any_toml_spelling_is_declared() {
    // G178 (review 3): `use std::iter::zip;` beside a `zip` dependency the manifest reading
    // dropped (a header comment, spaces in the brackets, a quoted key) claimed
    // `std::iter::zip::open`; each spelling declares it, so the call is withheld.
    let lib = "use std::iter::zip;\npub fn f() {\n    zip::open();\n    bar_baz::g();\n}\n";
    for dependencies in [
        "[dependencies] # runtime\nzip = { git = \"https://example.invalid/zip\" }\n\"bar-baz\" = \"1\"\n",
        "[ dependencies ]\n\"zip\" = \"1\"\n'bar-baz' = \"1\"\n",
        "[dependencies.zip] # dotted\ngit = \"https://example.invalid/zip\"\n[target.'cfg(unix)'.dependencies.\"bar-baz\"]\nversion = \"1\"\n",
    ] {
        let manifests = map(&[(
            "app/Cargo.toml",
            &format!("[package]\nname = \"app\"\n\n{dependencies}"),
        )]);
        let sources = map(&[("app/src/lib.rs", lib)]);
        let crates = crate_targets(&manifests, &sources);
        let declared: Vec<&str> = crates[0].declared.iter().map(String::as_str).collect();
        assert_eq!(declared, ["bar_baz", "zip"], "{dependencies}");
        let calls = adapter::resolve_path_calls(&crates, &sources);
        assert_eq!(calls.len(), 2);
        for call in &calls {
            assert!(
                !matches!(&call.outcome, adapter::PathCallOutcome::External(path) if path.starts_with("std")),
                "{dependencies}: {call:?}"
            );
        }
    }
}

#[test]
fn a_multi_line_path_binds_its_directory_and_an_empty_path_binds_nothing() {
    // G178 (review 4), H1: `'''../zip'''` read as `""` joined to the package's own directory, so
    // `zip` bound the package's own library and `zip::open()` claimed its own `open`. Cargo
    // binds `../zip`; an empty path (which never names another package) binds nothing, its name
    // only declared.
    let lib = "pub fn open() {}\npub fn f() {\n    zip::open();\n    own::open();\n}\n";
    let manifests = map(&[
        (
            "app/Cargo.toml",
            "[package]\nname = \"app\"\n[dependencies]\nzip = { path = '''../zip''', package = \"zip\" }\nown = { path = \"\", package = \"app\" }\n",
        ),
        ("zip/Cargo.toml", "[package]\nname = \"zip\"\n"),
    ]);
    let sources = map(&[
        ("app/src/lib.rs", lib),
        ("zip/src/lib.rs", "pub fn open() {}\n"),
    ]);
    let crates = crate_targets(&manifests, &sources);
    let app = &crates[0];
    assert_eq!(app.root, "app/src/lib.rs");
    let externs: Vec<(&str, &str)> = (app.externs.iter())
        .map(|(name, &lib)| (name.as_str(), crates[lib].root.as_str()))
        .collect();
    assert_eq!(externs, [("zip", "zip/src/lib.rs")]);
    assert_eq!(app.declared, BTreeSet::from(["own".into(), "zip".into()]));
    let calls = adapter::resolve_path_calls(&crates, &sources);
    let outcomes: Vec<(&str, String)> = (calls.iter())
        .filter(|call| call.path == "app/src/lib.rs")
        .map(|call| {
            let outcome = match &call.outcome {
                adapter::PathCallOutcome::Resolved(t) => format!("{}:{}", t.path, t.name),
                other => format!("{other:?}").chars().take(8).collect(),
            };
            (call.callee.as_str(), outcome)
        })
        .collect();
    assert_eq!(outcomes[0], ("zip::open", "zip/src/lib.rs:open".to_owned()));
    assert_eq!(outcomes[1].0, "own::open");
    assert!(!outcomes[1].1.starts_with("app/"), "{outcomes:?}");
}

#[test]
fn entries_sharing_a_key_or_a_name_that_bind_differently_bind_neither() {
    // G178 (review 4), H2: two entries of one key (a table read where there is none, or Cargo's
    // "different source paths depending on the build target" error), or two keys passing one
    // extern name, that bind different libraries: neither binds, both names stay declared.
    // Entries that agree still bind.
    let manifests = map(&[
        (
            "app/Cargo.toml",
            "[package]\nname = \"app\"\n[dependencies]\nzip = { path = \"../zip\" }\nsame = { path = \"../same\" }\np1 = { path = \"../p1\" }\np2 = { path = \"../p2\" }\n[target.'cfg(unix)'.dependencies]\nzip = { path = \"../evil\", package = \"zip\" }\n[dev-dependencies]\nsame = { path = \"../same\" }\n",
        ),
        ("zip/Cargo.toml", "[package]\nname = \"zip\"\n"),
        ("evil/Cargo.toml", "[package]\nname = \"zip\"\n"),
        ("same/Cargo.toml", "[package]\nname = \"same\"\n"),
        (
            "p1/Cargo.toml",
            "[package]\nname = \"p1\"\n[lib]\nname = \"shared\"\n",
        ),
        (
            "p2/Cargo.toml",
            "[package]\nname = \"p2\"\n[lib]\nname = \"shared\"\n",
        ),
    ]);
    let sources = map(&[
        ("app/src/lib.rs", ""),
        ("zip/src/lib.rs", ""),
        ("evil/src/lib.rs", ""),
        ("same/src/lib.rs", ""),
        ("p1/src/lib.rs", ""),
        ("p2/src/lib.rs", ""),
    ]);
    let crates = crate_targets(&manifests, &sources);
    let app = crates.iter().find(|c| c.root == "app/src/lib.rs").unwrap();
    let externs: Vec<(&str, &str)> = (app.externs.iter())
        .map(|(name, &lib)| (name.as_str(), crates[lib].root.as_str()))
        .collect();
    assert_eq!(externs, [("same", "same/src/lib.rs")]);
    assert_eq!(
        app.declared,
        BTreeSet::from(["same".into(), "shared".into(), "zip".into()])
    );
}

#[test]
fn registry_dependencies_map_to_the_packages_the_lockfile_confirms() {
    // G173: each target's registry extern names, to the package its manifest declares (a rename,
    // an entry inherited from a root manifest not in view), kept only when the lockfile records
    // that package as a direct dependency of the target's package; a build script sees its own.
    let manifests = map(&[
        ("core/Cargo.toml", "[package]\nname = \"core\"\n"),
        (
            "app/Cargo.toml",
            "[package]\nname = \"atlas-app\"\n\n[dependencies]\nlocal = { package = \"core\", path = \"../core\" }\nser = { package = \"serde\", version = \"1\" }\njson = { workspace = true }\nquote = \"1\"\n\n[build-dependencies]\ncc = \"1\"\n",
        ),
    ]);
    let sources = map(&[
        ("core/src/lib.rs", ""),
        ("app/src/lib.rs", ""),
        ("app/src/main.rs", ""),
        ("app/build.rs", ""),
    ]);
    let lock = "version = 4\n\n[[package]]\nname = \"atlas-app\"\nversion = \"0.1.0\"\ndependencies = [\"cc\", \"core\", \"serde\", \"serde_json\"]\n\n[[package]]\nname = \"core\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n\n[[package]]\nname = \"serde_json\"\nversion = \"1.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n\n[[package]]\nname = \"cc\"\nversion = \"1.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n";
    let locked = adapter::read_locked_trait_methods(lock, Path::new("/nonexistent-registry"));
    let root = adapter::workspace_dependencies(
        "[workspace]\nmembers = [\"core\", \"app\"]\n\n[workspace.dependencies]\njson = { package = \"serde_json\", version = \"1\" }\n",
    );
    let described = |crates: &[CrateInput]| -> Vec<(String, Vec<(String, String)>)> {
        crates
            .iter()
            .map(|c| {
                let foreign = c.foreign.iter().map(|(k, v)| (k.clone(), v.clone()));
                (c.root.clone(), foreign.collect())
            })
            .collect()
    };
    let pairs = |p: &[(&str, &str)]| -> Vec<(String, String)> {
        p.iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    };
    let crates = crate_targets_locked(&manifests, &sources, &locked, root.as_ref());
    let runtime = pairs(&[("json", "serde_json"), ("ser", "serde")]);
    assert_eq!(
        described(&crates),
        vec![
            ("app/src/lib.rs".to_owned(), runtime.clone()),
            ("core/src/lib.rs".to_owned(), Vec::new()),
            ("app/src/main.rs".to_owned(), runtime),
            ("app/build.rs".to_owned(), pairs(&[("cc", "cc")])),
        ]
    );
    // `quote` is declared but not locked, the path dependency is a workspace crate, and without
    // the root manifest the inherited `json` entry names no package.
    let crates = crate_targets_locked(&manifests, &sources, &locked, None);
    assert_eq!(described(&crates)[0].1, pairs(&[("ser", "serde")]));
    // Without a lockfile nothing is confirmed.
    let crates = crate_targets(&manifests, &sources);
    assert!(crates.iter().all(|c| c.foreign.is_empty()));
}

#[test]
fn crate_targets_follow_manifests_renames_roles_and_own_library() {
    let manifests = map(&[
        ("Cargo.toml", "[workspace]\nmembers = [\"core\", \"app\"]\n"),
        ("core/Cargo.toml", "[package]\nname = \"core\"\n"),
        (
            "app/Cargo.toml",
            "[package]\nname = \"atlas-app\"\n\n[dependencies]\natlas_core = { package = \"core\", path = \"../core\" }\nserde = \"1\"\n\n[build-dependencies]\nbuild_core = { package = \"core\", path = \"../core\" }\n",
        ),
    ]);
    let sources = map(&[
        ("core/src/lib.rs", ""),
        ("app/src/lib.rs", ""),
        ("app/src/main.rs", ""),
        ("app/build.rs", ""),
    ]);
    let crates = crate_targets(&manifests, &sources);
    let described: Vec<(String, Vec<(String, String)>)> = crates
        .iter()
        .map(|c| {
            (
                c.root.clone(),
                c.externs
                    .iter()
                    .map(|(name, index)| (name.clone(), crates[*index].root.clone()))
                    .collect(),
            )
        })
        .collect();
    let s = |v: &str| v.to_owned();
    assert_eq!(
        described,
        [
            (
                s("app/src/lib.rs"),
                vec![(s("atlas_core"), s("core/src/lib.rs"))]
            ),
            (s("core/src/lib.rs"), vec![]),
            (
                s("app/src/main.rs"),
                vec![
                    (s("atlas_app"), s("app/src/lib.rs")),
                    (s("atlas_core"), s("core/src/lib.rs")),
                ]
            ),
            (
                s("app/build.rs"),
                vec![(s("build_core"), s("core/src/lib.rs"))]
            ),
        ]
    );
}

/// G168 (replay R10, rust-analyzer): its workspace keeps `lib/lsp-server` (0.10.0) as a member
/// while every crate takes `lsp-server = { version = "0.7.9" }` from the registry; Atlas bound the
/// member. A dependency binds the package directory its declaration resolves to, through its own
/// path, an inherited `[workspace.dependencies]` entry (renames included) or a `[patch]`, never a
/// same-named member of the workspace otherwise.
#[test]
fn crate_targets_bind_the_package_a_declaration_names_not_a_member_of_the_same_name() {
    let manifests = map(&[
        (
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/*\", \"lib/*\"]\n\n[workspace.dependencies]\nide = { path = \"./crates/ide\", version = \"0.0.0\" }\nlsp-server = { version = \"0.7.9\" }\nrenamed = { package = \"text-size\", path = \"lib/text-size\" }\n\n[patch.crates-io]\nla-arena = { path = \"lib/la-arena\" }\n",
        ),
        ("crates/ide/Cargo.toml", "[package]\nname = \"ide\"\n"),
        (
            "lib/lsp-server/Cargo.toml",
            "[package]\nname = \"lsp-server\"\n",
        ),
        (
            "lib/text-size/Cargo.toml",
            "[package]\nname = \"text-size\"\n",
        ),
        (
            "lib/la-arena/Cargo.toml",
            "[package]\nname = \"la-arena\"\n",
        ),
        (
            "lib/line-index/Cargo.toml",
            "[package]\nname = \"line-index\"\n",
        ),
        (
            "crates/app/Cargo.toml",
            "[package]\nname = \"app\"\n\n[dependencies]\nide.workspace = true\nlsp-server.workspace = true\nrenamed = { workspace = true }\nla-arena = \"0.3\"\nline-index = \"0.1\"\nlocal = { package = \"line-index\", path = \"../../lib/line-index\" }\n",
        ),
    ]);
    let sources = map(&[
        ("crates/ide/src/lib.rs", ""),
        ("lib/lsp-server/src/lib.rs", ""),
        ("lib/text-size/src/lib.rs", ""),
        ("lib/la-arena/src/lib.rs", ""),
        ("lib/line-index/src/lib.rs", ""),
        ("crates/app/src/lib.rs", ""),
    ]);
    let crates = crate_targets(&manifests, &sources);
    let app = crates
        .iter()
        .find(|c| c.root == "crates/app/src/lib.rs")
        .unwrap();
    let externs: Vec<(&str, &str)> = app
        .externs
        .iter()
        .map(|(name, index)| (name.as_str(), crates[*index].root.as_str()))
        .collect();
    assert_eq!(
        externs,
        [
            ("ide", "crates/ide/src/lib.rs"),
            ("la_arena", "lib/la-arena/src/lib.rs"),
            ("local", "lib/line-index/src/lib.rs"),
            ("renamed", "lib/text-size/src/lib.rs"),
        ]
    );
    // G178 (review 4): with no workspace root manifest in view, an inherited entry binds
    // nothing, even when a library of its key's name is in view: the unseen root may declare
    // `b = { package = "c", path = ".." }`, and Cargo then passes `c`'s library as `b`. The name
    // stays declared, so `b::` is withheld, never bound to the wrong crate.
    let orphan = map(&[
        (
            "a/Cargo.toml",
            "[package]\nname = \"a\"\n\n[dependencies]\nb.workspace = true\n",
        ),
        ("b/Cargo.toml", "[package]\nname = \"b\"\n"),
    ]);
    let crates = crate_targets(&orphan, &map(&[("a/src/lib.rs", ""), ("b/src/lib.rs", "")]));
    let a = crates.iter().find(|c| c.root == "a/src/lib.rs").unwrap();
    assert!(a.externs.is_empty(), "{:?}", a.externs);
    assert_eq!(a.declared, BTreeSet::from(["b".to_owned()]));
}

/// G164 (replay R8, crubit): a manifest may keep its targets outside its own directory
/// (`cargo/x/Cargo.toml` with `[lib] path = "../../x.rs"`); the crate root is the normalized
/// inventory path, its dependencies bind through it, and a target climbing above the repository
/// root is no crate at all.
#[test]
fn crate_targets_normalize_climbing_target_paths_and_refuse_escapes() {
    let manifests = map(&[
        (
            "cargo/tool/Cargo.toml",
            "[package]\nname = \"tool\"\n\n[[bin]]\nname = \"tool\"\npath = \"../../tool/./main.rs\"\n\n[dependencies]\ncmdline = { path = \"../cmdline\", package = \"tool_cmdline\" }\n",
        ),
        (
            "cargo/cmdline/Cargo.toml",
            "[package]\nname = \"tool_cmdline\"\n\n[lib]\npath = \"../../tool/cmdline.rs\"\n",
        ),
        (
            "cargo/escape/Cargo.toml",
            "[package]\nname = \"escape\"\n\n[lib]\npath = \"../../../outside.rs\"\n",
        ),
    ]);
    let sources = map(&[
        ("tool/main.rs", ""),
        ("tool/cmdline.rs", ""),
        ("outside.rs", ""),
    ]);
    let crates = crate_targets(&manifests, &sources);
    let described: Vec<(String, Vec<(String, String)>)> = crates
        .iter()
        .map(|c| {
            (
                c.root.clone(),
                c.externs
                    .iter()
                    .map(|(name, index)| (name.clone(), crates[*index].root.clone()))
                    .collect(),
            )
        })
        .collect();
    let s = |v: &str| v.to_owned();
    assert_eq!(
        described,
        [
            (s("tool/cmdline.rs"), vec![]),
            (
                s("tool/main.rs"),
                vec![(s("cmdline"), s("tool/cmdline.rs"))]
            ),
        ]
    );
    assert_eq!(target_path("a/b", "../../c.rs").as_deref(), Some("c.rs"));
    assert_eq!(target_path("a", "./b/../c.rs").as_deref(), Some("a/c.rs"));
    assert_eq!(target_path("a", "../../c.rs"), None);
    assert_eq!(target_path("", "src/lib.rs").as_deref(), Some("src/lib.rs"));
}

fn artifact(path: &str, language: &str) -> ArtifactRecord {
    ArtifactRecord {
        id: ArtifactId::new(format!("artifact:{path}")),
        path: path.to_owned(),
        kind: ArtifactKind::File,
        bytes: 0,
        disposition: ArtifactDisposition::Parsed,
        language: Some(language.into()),
        reason: None,
        content_digest: None,
        content_digest_withheld: None,
    }
}

fn workspace() -> (std::path::PathBuf, InventoryReport) {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "atlas-g75-resolution-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(dir.join("core/src")).unwrap();
    fs::write(dir.join("core/Cargo.toml"), "[package]\nname = \"core\"\n").unwrap();
    fs::write(
        dir.join("core/src/lib.rs"),
        "pub mod a;\npub mod io;\npub fn f() {\n    a::g();\n    helper();\n    x.method();\n}\nfn helper() {}\n",
    )
    .unwrap();
    fs::write(
        dir.join("core/src/io.rs"),
        "use std::fs;\nuse std::fs::File;\npub fn save() {\n    fs::write(\"a\", \"b\");\n    File::open(\"a\");\n    fs::copy(\"a\", \"b\");\n    std::env::var(\"X\"); std::time::Instant::now();\n    std::mem::size_of::<u128>();\n}\npub struct Status;\npub fn typed<T>(a: Status, b: crate::io::Status, c: Vec<Status>, d: File, e: T) {}\npub struct Other;\npub fn plain(o: Other) {}\npub fn shadow<Other>(o: Other) {}\n",
    )
    .unwrap();
    fs::write(dir.join("core/src/a.rs"), "pub fn g() {}\n").unwrap();
    let inventory = InventoryReport::new(
        dir.to_string_lossy().into_owned(),
        vec![
            artifact("core/Cargo.toml", "toml"),
            artifact("core/src/lib.rs", "rust"),
            artifact("core/src/a.rs", "rust"),
            artifact("core/src/io.rs", "rust"),
        ],
    );
    (dir, inventory)
}

fn revision() -> RevisionRef {
    RevisionRef {
        kind: "git".into(),
        value: "abc123".into(),
    }
}

#[test]
fn resolutions_observe_the_syntactic_claims_and_name_their_callee_identity() {
    let (dir, inventory) = workspace();
    let batches = extract_semantics(&inventory, RepositoryId::new("atlas-studio"), revision());
    let resolution = resolve_rust_path_calls(&inventory, &batches);
    assert_eq!(resolution.len(), 3, "one batch per reached Rust artifact");

    let calls = |batch: &ExtractionBatch| -> Vec<atlas_core::SemanticRecordHeader<atlas_core::CallSiteIdentity>> {
        batch
            .observations
            .iter()
            .filter_map(|o| match o {
                SemanticObservation::Call(h) => Some(h.clone()),
                _ => None,
            })
            .collect()
    };
    let syntactic_lib = batches
        .iter()
        .find(|b| b.artifact.as_str() == "artifact:core/src/lib.rs")
        .unwrap();
    let lib = resolution
        .iter()
        .find(|b| b.artifact.as_str() == "artifact:core/src/lib.rs")
        .unwrap();
    let resolved = calls(lib);
    assert_eq!(
        resolved.len(),
        2,
        "a::g() and helper(); never the method call"
    );
    let functions: BTreeMap<String, SemanticRecordId> = batches
        .iter()
        .flat_map(|b| &b.observations)
        .filter_map(|o| match o {
            SemanticObservation::FunctionIdentity(h) => {
                Some((h.subject.symbol.name.clone(), h.record_id.clone()))
            }
            _ => None,
        })
        .collect();
    for call in &resolved {
        // The same claim the syntactic extractor made.
        let claim = calls(syntactic_lib)
            .into_iter()
            .find(|c| c.record_id == call.record_id)
            .expect("a resolution observes an existing CALL claim");
        assert_eq!(claim.subject.span, call.subject.span);
        assert_eq!(claim.subject.dispatch, CallDispatchKind::Unresolved);
        assert_eq!(call.subject.dispatch, CallDispatchKind::StaticResolved);
        assert_eq!(call.extractor.id, RUST_PATH_RESOLUTION_ID);
        assert_eq!(call.status, EpistemicStatus::Derived);
    }
    let callees: Vec<&SemanticRecordId> = resolved.iter().map(|c| &c.subject.callees[0]).collect();
    assert_eq!(callees, [&functions["g"], &functions["helper"]]);

    // The engine accounts for CALL and EFFECT only, and closure holds per engine.
    let dimensions: Vec<SemanticDimension> = lib.obligations.iter().map(|o| o.dimension).collect();
    assert_eq!(dimensions, RUST_PATH_RESOLUTION_DIMENSIONS);
    assert!(
        lib.obligations
            .iter()
            .all(|o| o.status == EpistemicStatus::Unknown)
    );
    let mut accounting = CensusExtractionAccounting::new();
    for batch in batches.iter().chain(&resolution) {
        accounting.record_batch(batch);
    }
    assert!(accounting.is_closed_per_extractor(requested_dimensions));
    assert!(
        !accounting.is_closed(&crate::census::extraction::ALL_SEMANTIC_DIMENSIONS),
        "the resolution engine is not asked for every dimension"
    );

    // Two engines' observations of one claim are not a normalization conflict.
    let all: Vec<ExtractionBatch> = batches.iter().chain(&resolution).cloned().collect();
    let source = adapter::source_report_from_inventory(&inventory);
    let adl = atlas_core::compile_adl(&[], &source);
    let census = crate::census::build_census(&inventory, &source, &adl, &all, &revision());
    let normalized = crate::normalize::normalize(&census);
    assert!(normalized.conflict_candidates.is_empty());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_resolution_without_a_syntactic_claim_is_a_diagnosed_disagreement() {
    let (dir, inventory) = workspace();
    let mut batches = extract_semantics(&inventory, RepositoryId::new("atlas-studio"), revision());
    // Drop the syntactic claims for `a::g()` (lib.rs line 4) and for the effectful `fs::write`
    // (io.rs line 4), and keep the others.
    let mut dropped = None;
    for batch in &mut batches {
        batch.observations.retain(|o| match o {
            SemanticObservation::Call(h)
                if h.subject.span.path == "core/src/lib.rs" && h.subject.span.line == 4 =>
            {
                dropped = Some(h.record_id.clone());
                false
            }
            SemanticObservation::Call(h)
                if h.subject.span.path == "core/src/io.rs" && h.subject.span.line == 4 =>
            {
                false
            }
            _ => true,
        });
    }
    let dropped = dropped.expect("the a::g() claim existed");
    let resolution = resolve_rust_path_calls(&inventory, &batches);
    let lib = resolution
        .iter()
        .find(|b| b.artifact.as_str() == "artifact:core/src/lib.rs")
        .unwrap();
    let observed: Vec<&SemanticRecordId> = lib.observations.iter().map(|o| o.record_id()).collect();
    assert_eq!(
        observed.len(),
        1,
        "only helper() still has a claim to observe"
    );
    assert!(!observed.contains(&&dropped), "never a fabricated claim");
    assert!(
        lib.diagnostics
            .iter()
            .any(|d| d.message.starts_with("1 resolved path call(s)")),
        "{:#?}",
        lib.diagnostics
    );
    assert_eq!(lib.obligations[0].diagnostics.len(), 2);
    // An effect needs the caller its claim names: none is derived for the unclaimed call.
    let io = resolution
        .iter()
        .find(|b| b.artifact.as_str() == "artifact:core/src/io.rs")
        .unwrap();
    let effect_lines: Vec<usize> = io
        .observations
        .iter()
        .filter_map(|o| match o {
            SemanticObservation::Effect(h) => Some(h.subject.span.line),
            _ => None,
        })
        .collect();
    assert_eq!(effect_lines, [5, 6, 6, 7, 7]);
    assert!(
        io.diagnostics
            .iter()
            .any(|d| d.message.starts_with("1 resolved path call(s)"))
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn resolved_standard_library_paths_are_effect_sites_of_the_caller() {
    let (dir, inventory) = workspace();
    let batches = extract_semantics(&inventory, RepositoryId::new("atlas-studio"), revision());
    let resolution = resolve_rust_path_calls(&inventory, &batches);
    let io = resolution
        .iter()
        .find(|b| b.artifact.as_str() == "artifact:core/src/io.rs")
        .unwrap();
    let effects: Vec<(usize, &str)> = io
        .observations
        .iter()
        .filter_map(|o| match o {
            SemanticObservation::Effect(h) => {
                Some((h.subject.span.line, h.subject.category.as_str()))
            }
            _ => None,
        })
        .collect();
    // fs::write, File::open, fs::copy (read and write), std::env::var and Instant::now (G91);
    // never std::mem::size_of (not declared).
    assert_eq!(
        effects,
        [
            (4, "FILESYSTEM_WRITE"),
            (5, "FILESYSTEM_READ"),
            (6, "FILESYSTEM_READ"),
            (6, "FILESYSTEM_WRITE"),
            (7, "ENVIRONMENT_READ"),
            (7, "CLOCK_READ"),
        ]
    );
    let save = batches
        .iter()
        .flat_map(|b| &b.observations)
        .find_map(|o| match o {
            SemanticObservation::FunctionIdentity(h) if h.subject.symbol.name == "save" => {
                Some(h.record_id.clone())
            }
            _ => None,
        })
        .unwrap();
    for observation in &io.observations {
        if let SemanticObservation::Effect(h) = observation {
            assert_eq!(h.subject.function, save, "the caller owns the effect");
            assert_eq!(h.status, EpistemicStatus::Derived);
            assert_eq!(h.extractor.id, RUST_PATH_RESOLUTION_ID);
        }
    }
    let effect = io
        .obligations
        .iter()
        .find(|o| o.dimension == SemanticDimension::Effect)
        .unwrap();
    assert_eq!(effect.observation_ids.len(), 6);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_type_spelling_resolved_everywhere_in_its_artifact_is_a_canonical_type_claim() {
    let (dir, inventory) = workspace();
    let batches = extract_semantics(&inventory, RepositoryId::new("atlas-studio"), revision());
    let resolution = resolve_rust_path_calls(&inventory, &batches);
    let io = resolution
        .iter()
        .find(|b| b.artifact.as_str() == "artifact:core/src/io.rs")
        .unwrap();
    let claims: BTreeMap<String, String> = io
        .observations
        .iter()
        .filter_map(|o| match o {
            SemanticObservation::Type(h) => {
                Some((h.subject.name.clone(), h.subject.canonical.clone().unwrap()))
            }
            _ => None,
        })
        .collect();
    // Two spellings of one type share its canonical identity; a generic parameter has none.
    assert_eq!(
        claims.get("Status").map(String::as_str),
        Some("core io/Status#")
    );
    assert_eq!(
        claims.get("crate::io::Status").map(String::as_str),
        Some("core io/Status#")
    );
    assert_eq!(
        claims.get("Vec<Status>").map(String::as_str),
        Some("std::vec::Vec<core io/Status#>")
    );
    assert_eq!(
        claims.get("File").map(String::as_str),
        Some("std::fs::File")
    );
    assert!(!claims.contains_key("T"));
    // `Other` means the struct in one function and a generic parameter in another: no claim.
    assert!(!claims.contains_key("Other"));
    // Every claim names a spelling the syntactic extractor recorded in this artifact.
    let recorded: std::collections::BTreeSet<String> = batches
        .iter()
        .flat_map(|b| &b.observations)
        .filter_map(|o| match o {
            SemanticObservation::Type(h) if h.subject.path == "core/src/io.rs" => {
                Some(h.subject.name.clone())
            }
            _ => None,
        })
        .collect();
    assert!(claims.keys().all(|spelling| recorded.contains(spelling)));
    for observation in &io.observations {
        if let SemanticObservation::Type(h) = observation {
            assert_eq!(h.status, EpistemicStatus::Derived);
            assert!(
                h.subject.path.is_empty(),
                "a canonical type is not file-scoped"
            );
        }
    }
    let types = io
        .obligations
        .iter()
        .find(|o| o.dimension == SemanticDimension::Type)
        .unwrap();
    assert_eq!(types.observation_ids.len(), claims.len());
    fs::remove_dir_all(&dir).unwrap();
}

/// G117 (NA-CONCURRENCY-RESOLVED): a path call resolved to a std path the declared concurrency
/// table names is a DERIVED concurrency site of its caller; an undeclared std path, a workspace
/// function merely named `spawn`, and a method call inside a closure are not.
#[test]
fn resolved_standard_library_paths_are_concurrency_sites_of_the_caller() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "atlas-g117-concurrency-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(dir.join("core/src")).unwrap();
    fs::write(dir.join("core/Cargo.toml"), "[package]\nname = \"core\"\n").unwrap();
    fs::write(dir.join("core/src/lib.rs"), "pub mod work;\n").unwrap();
    fs::write(
        dir.join("core/src/work.rs"),
        "use std::sync::mpsc;\nuse std::thread;\npub fn run() {\n    thread::scope(|s| {\n        s.spawn(|| {});\n    });\n    std::thread::spawn(|| {});\n    let (_tx, _rx) = mpsc::channel::<u8>();\n    thread::sleep(std::time::Duration::from_millis(0));\n}\nmod pool {\n    pub fn spawn() {}\n}\npub fn fake() {\n    pool::spawn();\n}\n",
    )
    .unwrap();
    let inventory = InventoryReport::new(
        dir.to_string_lossy().into_owned(),
        vec![
            artifact("core/Cargo.toml", "toml"),
            artifact("core/src/lib.rs", "rust"),
            artifact("core/src/work.rs", "rust"),
        ],
    );
    let batches = extract_semantics(&inventory, RepositoryId::new("atlas-studio"), revision());
    let resolution = resolve_rust_path_calls(&inventory, &batches);
    let work = resolution
        .iter()
        .find(|b| b.artifact.as_str() == "artifact:core/src/work.rs")
        .unwrap();
    let sites: Vec<(usize, &str)> = work
        .observations
        .iter()
        .filter_map(|o| match o {
            SemanticObservation::Concurrency(h) => {
                Some((h.subject.span.line, h.subject.kind.as_str()))
            }
            _ => None,
        })
        .collect();
    // thread::scope, std::thread::spawn, mpsc::channel; never thread::sleep (undeclared),
    // pool::spawn (a workspace function) or s.spawn (a method call inside a closure).
    assert_eq!(
        sites,
        [(4, "THREAD_SCOPE"), (7, "SPAWN"), (8, "CHANNEL_CREATE")]
    );
    let run = batches
        .iter()
        .flat_map(|b| &b.observations)
        .find_map(|o| match o {
            SemanticObservation::FunctionIdentity(h) if h.subject.symbol.name == "run" => {
                Some(h.record_id.clone())
            }
            _ => None,
        })
        .unwrap();
    for observation in &work.observations {
        if let SemanticObservation::Concurrency(h) = observation {
            assert_eq!(h.subject.function, run, "the caller owns the site");
            assert_eq!(h.status, EpistemicStatus::Derived);
            assert_eq!(h.extractor.id, RUST_PATH_RESOLUTION_ID);
        }
    }
    let obligation = work
        .obligations
        .iter()
        .find(|o| o.dimension == SemanticDimension::Concurrency)
        .expect("the engine is asked for CONCURRENCY");
    assert_eq!(obligation.status, EpistemicStatus::Unknown);
    assert_eq!(obligation.observation_ids.len(), 3);
    fs::remove_dir_all(&dir).unwrap();
}

/// G125 (NA-PERSISTENCE-RESOLVED): a path call resolved to a std path the declared persistence
/// table names is a DERIVED persistence site of its caller -- the resolved API is the evidence, the
/// place is not claimed -- and nothing else is: an opened file, a workspace function spelled like
/// std, or a method call.
#[test]
fn resolved_standard_library_paths_are_persistence_sites_of_the_caller() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "atlas-g125-persistence-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(dir.join("core/src")).unwrap();
    fs::write(dir.join("core/Cargo.toml"), "[package]\nname = \"core\"\n").unwrap();
    fs::write(dir.join("core/src/lib.rs"), "pub mod store;\n").unwrap();
    fs::write(
        dir.join("core/src/store.rs"),
        "use std::fs;\npub fn keep() {\n    fs::write(\"a\", \"b\");\n    let _ = std::fs::read_to_string(\"a\");\n    let f = fs::File::open(\"a\");\n    local::write();\n    f.sync_all();\n}\nmod local {\n    pub fn write() {}\n}\n",
    )
    .unwrap();
    let inventory = InventoryReport::new(
        dir.to_string_lossy().into_owned(),
        vec![
            artifact("core/Cargo.toml", "toml"),
            artifact("core/src/lib.rs", "rust"),
            artifact("core/src/store.rs", "rust"),
        ],
    );
    let batches = extract_semantics(&inventory, RepositoryId::new("atlas-studio"), revision());
    let resolution = resolve_rust_path_calls(&inventory, &batches);
    let store = resolution
        .iter()
        .find(|b| b.artifact.as_str() == "artifact:core/src/store.rs")
        .unwrap();
    let sites: Vec<(usize, &str)> = store
        .observations
        .iter()
        .filter_map(|o| match o {
            SemanticObservation::Persistence(h) => {
                assert_eq!(h.status, EpistemicStatus::Derived);
                assert_eq!(h.extractor.id, RUST_PATH_RESOLUTION_ID);
                assert_eq!(
                    h.subject.resolution,
                    atlas_core::PersistenceResolution::Resolved
                );
                assert_eq!(h.subject.place, atlas_core::PlaceRef::Unresolved);
                Some((h.subject.span.line, h.subject.kind.as_str()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(sites, [(3, "DURABLE_WRITE"), (4, "DURABLE_READ")]);
    let obligation = store
        .obligations
        .iter()
        .find(|o| o.dimension == SemanticDimension::Persistence)
        .expect("the engine accounts for PERSISTENCE");
    assert_eq!(
        obligation.status,
        EpistemicStatus::Unknown,
        "method calls stay outside"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn resolved_acquisitions_and_let_bound_releases_are_resource_records() {
    // G157: an acquisition is DERIVED at the resolved call; a holder never moved is released at
    // its block's end (INFERRED, syntactic move check), a resolved `drop` statement releases it
    // there (DERIVED); a moved holder and a temporary guard have no release claimed; every
    // release names the acquisition it gives back.
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "atlas-g157-resource-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(dir.join("core/src")).unwrap();
    fs::write(dir.join("core/Cargo.toml"), "[package]\nname = \"core\"\n").unwrap();
    fs::write(dir.join("core/src/lib.rs"), "pub mod io;\n").unwrap();
    fs::write(
        dir.join("core/src/io.rs"),
        "use std::fs::File;\nuse std::sync::Mutex;\nfn keep(_f: File) {}\npub fn read(p: &str, m: &Mutex<u8>) -> std::io::Result<()> {\n    let file = File::open(p)?;\n    file.metadata()?;\n    let guard = m.lock().unwrap();\n    drop(guard);\n    let kept = File::open(p)?;\n    keep(kept);\n    let _v = *m.lock().unwrap();\n    Ok(())\n}\n",
    )
    .unwrap();
    let inventory = InventoryReport::new(
        dir.to_string_lossy().into_owned(),
        vec![
            artifact("core/Cargo.toml", "toml"),
            artifact("core/src/lib.rs", "rust"),
            artifact("core/src/io.rs", "rust"),
        ],
    );
    let batches = extract_semantics(&inventory, RepositoryId::new("atlas-studio"), revision());
    let resolution = resolve_rust_path_calls(&inventory, &batches);
    let io = resolution
        .iter()
        .find(|b| b.artifact.as_str() == "artifact:core/src/io.rs")
        .unwrap();
    let sites: Vec<(usize, String, EpistemicStatus, Option<usize>)> = io
        .observations
        .iter()
        .filter_map(|o| match o {
            SemanticObservation::Resource(h) => {
                assert_eq!(h.extractor.id, RUST_PATH_RESOLUTION_ID);
                assert!(h.subject.is_well_formed());
                let kind = match h.subject.release {
                    Some(release) => format!(
                        "{} {} {} {}",
                        h.subject.operation.as_str(),
                        h.subject.kind.as_str(),
                        release.as_str(),
                        h.subject.holder
                    ),
                    None => format!(
                        "{} {}",
                        h.subject.operation.as_str(),
                        h.subject.kind.as_str()
                    ),
                };
                Some((
                    h.subject.span.line,
                    kind,
                    h.status,
                    h.subject.acquired_at.as_ref().map(|a| a.line),
                ))
            }
            _ => None,
        })
        .collect();
    let derived = EpistemicStatus::Derived;
    assert_eq!(
        sites,
        [
            (5, "ACQUIRE FILE".to_owned(), derived, None),
            (7, "ACQUIRE LOCK_GUARD".to_owned(), derived, None),
            (9, "ACQUIRE FILE".to_owned(), derived, None),
            (11, "ACQUIRE LOCK_GUARD".to_owned(), derived, None),
            (
                13,
                "RELEASE FILE SCOPE_END file".to_owned(),
                EpistemicStatus::Inferred,
                Some(5)
            ),
            (
                8,
                "RELEASE LOCK_GUARD EXPLICIT_DROP guard".to_owned(),
                derived,
                Some(7)
            ),
        ]
    );
    let obligation = io
        .obligations
        .iter()
        .find(|o| o.dimension == SemanticDimension::Resource)
        .expect("the engine accounts for RESOURCE");
    assert_eq!(
        obligation.status,
        EpistemicStatus::Unknown,
        "temporaries and moved holders stay outside"
    );
    fs::remove_dir_all(&dir).unwrap();
}
