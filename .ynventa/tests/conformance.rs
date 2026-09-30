//! The protocol itself: snapshot currency, self-conformance, tamper detection, deterministic
//! output, cross-repository graph merging, and agreement between rustc and the run-time reader.

mod common;

use common::*;
use ynventa::graph::NodeId;
use ynventa::protocol::{own_subsystem_dir, Snapshot};

#[test]
fn protocol_snapshot_is_current() {
    let recorded =
        Snapshot::load(&ynventa::default_root()).expect(".ynventa/protocol.snapshot exists");
    let current = Snapshot::current(own_subsystem_dir());
    assert_eq!(
        recorded, current,
        "stale snapshot: run `cargo run --manifest-path .ynventa/Cargo.toml -- protocol --write`"
    );
    assert_eq!(current.version, ynventa::YNVENTA_PROTOCOL_VERSION);
}

#[test]
fn dogfood_repository_conforms() {
    let a = ynventa::assess(&ynventa::default_root()).unwrap();
    let checks = ynventa::conformance::protocol_checks(&a, None);
    let failed: Vec<_> = checks.iter().filter(|c| !c.pass).collect();
    assert!(failed.is_empty(), "{failed:#?}");
    assert!(a.errors().next().is_none(), "{:#?}", a.findings);
    assert!(
        a.counts.v1_gate().iter().all(|(_, _, _, pass)| *pass),
        "{:#?}",
        a.counts.v1_gate()
    );
}

#[test]
fn compiled_and_runtime_declarations_agree() {
    use ynventa::declare::decl::*;
    const REPOSITORY: Repository = include!("../declared/repository.rs");
    const DONORS: &[Donor] = include!("../declared/donors.rs");
    const MIGRATION: Migration = include!("../declared/migration.rs");
    const TECHNOLOGIES: &[Technology] = include!("../declared/technologies.rs");
    let compiled = into_model(&REPOSITORY, DONORS, &MIGRATION, TECHNOLOGIES);
    assert_eq!(
        ynventa::declare::load(&ynventa::default_root()).unwrap(),
        compiled
    );
}

#[test]
fn a_modified_subsystem_copy_is_detected() {
    let r = extinct_baseline("tamper");
    assert_eq!(r.cli(&["migrate", "scaffold"]).0, 0);
    let before = ynventa::conformance::protocol_checks(&r.assess(), None);
    assert!(
        before
            .iter()
            .find(|c| c.id == "protocol.subsystem_integrity")
            .unwrap()
            .pass
    );
    let lib = r.read(".ynventa/src/extinction/mod.rs");
    r.write(
        ".ynventa/src/extinction/mod.rs",
        &lib.replace("pass: obs.is_empty()", "pass: true"),
    );
    let after = ynventa::conformance::protocol_checks(&r.assess(), None);
    for id in [
        "protocol.subsystem_integrity",
        "protocol.canonical_subsystem",
    ] {
        assert!(!after.iter().find(|c| c.id == id).unwrap().pass, "{id}");
    }
    let (code, _) = r.cli(&["conformance"]);
    assert_eq!(code, 1);
}

#[test]
fn outputs_are_deterministic() {
    let r = extinct_baseline("determinism");
    assert_eq!(r.cli(&["migrate", "scaffold"]).0, 0);
    for cmd in [
        &["conformance", "--json"][..],
        &["graph", "--json"],
        &["extinction", "--json"],
        &["metrics", "--json"],
        &["audit", "--json"],
    ] {
        let a = r.cli(cmd);
        let b = r.cli(cmd);
        assert_eq!(a, b, "{cmd:?}");
        assert!(
            ynventa::formats::json::parse(&a.1).is_ok(),
            "{cmd:?} emits JSON"
        );
    }
    assert_eq!(
        r.cli(&["conformance"]).0,
        0,
        "{}",
        r.cli(&["conformance"]).1
    );
}

#[test]
fn two_shards_link_into_one_chronica_without_rewriting_ids() {
    // Two physical shards whose nodes carry distinct Chronica keys.
    let a = extinct_baseline("link-a");
    let b = extinct_baseline("link-b");
    b.edit(|d| {
        d.repository.shard = "esellios".into();
        for n in d.repository.nodes.iter_mut() {
            n.key = format!("commerce.{}", n.key);
        }
        for dn in d.donors.iter_mut() {
            for c in dn.capabilities.iter_mut() {
                c.replacement = c.replacement.as_ref().map(|r| format!("commerce.{r}"));
            }
        }
    });
    b.prove();
    let ga = a.assess().graph;
    let gb = b.assess().graph;
    let caps = [
        ynventa::capsule::Capsule::compile(&a.assess()),
        ynventa::capsule::Capsule::compile(&b.assess()),
    ];
    let image = ynventa::linker::link(&caps);
    for (id, n) in ga.nodes.iter().chain(gb.nodes.iter()) {
        assert_eq!(
            image.graph.nodes[id].id, *id,
            "no identity is rewritten by linking"
        );
        assert_eq!(NodeId::of(&n.namespace, &n.semantic_key), *id);
    }
    // The same upstream donor is one node in the linked system.
    let donor = NodeId::of("oss", "github.com/georust/geo");
    assert!(ga.nodes.contains_key(&donor) && gb.nodes.contains_key(&donor));
    assert!(image.graph.nodes.len() < ga.nodes.len() + gb.nodes.len());
    assert!(
        !image.issues.iter().any(|i| i.code == "OWNERSHIP_COLLISION"),
        "{:#?}",
        image.issues
    );
    // Physical owner is metadata on the node.
    assert_eq!(
        image
            .graph
            .node_by_key("chronica", "commerce.geo")
            .unwrap()
            .repository,
        "esellios"
    );
    assert_eq!(
        image
            .graph
            .node_by_key("chronica", "geo")
            .unwrap()
            .repository,
        "mechatron"
    );
    // Deterministic, round-trippable system image.
    let bytes = image.encode();
    assert_eq!(ynventa::linker::SystemImage::decode(&bytes).unwrap(), image);
    assert_eq!(ynventa::linker::link(&caps).encode(), bytes);
    // Linking by root through the CLI.
    let out = a.path().join("target/sys.ynv").display().to_string();
    let (_, text) = a.cli(&[
        "link",
        &a.path().display().to_string(),
        &b.path().display().to_string(),
        "--out",
        &out,
    ]);
    assert!(text.contains("linked shards 2/7"), "{text}");
}

#[test]
fn one_semantic_identity_has_one_physical_owner() {
    let a = extinct_baseline("collide-a");
    let b = extinct_baseline("collide-b");
    b.edit(|d| d.repository.shard = "esellios".into());
    let image = ynventa::linker::link(&[
        ynventa::capsule::Capsule::compile(&a.assess()),
        ynventa::capsule::Capsule::compile(&b.assess()),
    ]);
    assert!(!image.pass());
    let collision = image
        .issues
        .iter()
        .find(|i| i.code == "OWNERSHIP_COLLISION" && i.subject == "geo")
        .unwrap();
    assert!(collision.detail.contains("mechatron") && collision.detail.contains("esellios"));
}

#[test]
fn ecosystem_metrics_sum_then_divide() {
    let a = extinct_baseline("agg-a");
    let b = extinct_baseline("agg-b");
    b.edit(|d| {
        d.repository.shard = "esellios".into();
        d.donors[0].claimed = ynventa::schema::DonorState::Registered;
    });
    b.remove(".ynventa/evidence");
    let (code, out) = a.cli(&["metrics", "--aggregate", &b.path().display().to_string()]);
    assert_eq!(code, 0);
    assert!(out.contains("donors_registered 2\n"), "{out}");
    assert!(out.contains("donors_extinct 1\n"), "{out}");
    assert!(out.contains("extinction_ratio 0.500000\n"), "{out}");
}

#[test]
fn identity_survives_physical_moves() {
    let r = extinct_baseline("moves");
    let id = r.assess().graph.node_by_key("chronica", "geo").unwrap().id;
    // Move the tree to a legacy location and back through a wave; identity never changes.
    std::fs::rename(r.path().join("substrate/geo"), r.path().join("geo")).unwrap();
    r.write(
        "Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"core\", \"geo\", \"tests\"]\n",
    );
    r.write(
        "geo/Cargo.toml",
        &r.read("geo/Cargo.toml").replace("../../core", "../core"),
    );
    r.write(
        "tests/Cargo.toml",
        &r.read("tests/Cargo.toml")
            .replace("../substrate/geo", "../geo"),
    );
    r.edit(|d| d.repository.nodes[1].path = "geo".into());
    let a = r.assess();
    assert_eq!(a.graph.node_by_key("chronica", "geo").unwrap().id, id);
    assert!(finding(&a, "LEGACY_PLACEMENT", "geo").is_some());
    let (_, plan) = r.cli(&["migrate", "plan", "--write"]);
    assert!(plan.contains("w01-geo PLANNED: geo"), "{plan}");
    let (code, out) = r.cli(&["migrate", "apply", "w01-geo"]);
    assert_eq!(code, 0, "{out}");
    let a = r.assess();
    assert_eq!(a.graph.node_by_key("chronica", "geo").unwrap().id, id);
    assert_eq!(
        a.graph.node_by_key("chronica", "geo").unwrap().path,
        "substrate/geo"
    );
    assert!(r.read("tests/Cargo.toml").contains("\"../substrate/geo\""));
    assert!(r
        .read("substrate/geo/Cargo.toml")
        .contains("\"../../core\""));
}
