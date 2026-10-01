//! The protocol itself: snapshot currency, self-conformance, tamper detection, deterministic
//! output, cross-repository graph merging, and agreement between rustc and the run-time reader.

mod common;

use common::*;
use ynventa::graph::NodeId;
use ynventa::protocol::{own_subsystem_dir, Snapshot};
use ynventa::schema::{DonorState, FactKind};

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
    // Upgrading restores the canonical subsystem and removes files it no longer has.
    r.write(".ynventa/src/stale.rs", "// from an older snapshot\n");
    let (code, out) = r.cli(&["migrate", "scaffold"]);
    assert_eq!(code, 0, "{out}");
    assert!(!r.exists(".ynventa/src/stale.rs"));
    let again = ynventa::conformance::protocol_checks(&r.assess(), None);
    assert!(
        again
            .iter()
            .find(|c| c.id == "protocol.subsystem_integrity")
            .unwrap()
            .pass
    );
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

#[test]
fn reclaim_lowers_claims_above_evidence_and_keeps_them_as_facts() {
    let r = extinct_baseline("reclaim");
    // The replacement changes after its proofs ran: EXTINCT is now a claim above the evidence.
    r.write(
        "substrate/geo/src/lib.rs",
        "pub fn distance(a: (f64, f64), b: (f64, f64)) -> f64 { (a.0 - b.0).abs() + (a.1 - b.1).abs() }\n",
    );
    let a = r.assess();
    let effective = a.analysis.donors[0].effective;
    assert_ne!(effective, DonorState::Extinct);
    assert!(finding(&a, "CLAIM_EXCEEDS_EVIDENCE", "geo").is_some());

    let (code, out) = r.cli(&["migrate", "reclaim", "--dry-run"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("(dry run)"), "{out}");
    assert_eq!(r.declaration().donors[0].claimed, DonorState::Extinct);

    let (code, out) = r.cli(&["migrate", "reclaim"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(r.declaration().donors[0].claimed, effective);
    let a = r.assess();
    assert!(finding(&a, "CLAIM_EXCEEDS_EVIDENCE", "geo").is_none());
    // The retired claim is kept, with provenance, as knowledge.
    let k = ynventa::compact::facts::Knowledge::load(r.path());
    assert!(k.facts.iter().any(|f| f.kind == FactKind::LegacyClaim
        && f.subject == "geo"
        && f.value == "EXTINCT"
        && f.provenance.iter().any(|p| p.contains("reclaim"))));
    let (_, out) = r.cli(&["migrate", "reclaim"]);
    assert!(
        out.contains("every donor claim is within its evidence"),
        "{out}"
    );
}

#[test]
fn reclaim_withdraws_an_exception_the_evidence_does_not_allow() {
    let r = extinct_baseline("reclaim-exception");
    // "Studied, never adopted", while the donor package is in fact a dependency.
    r.write("core/Cargo.toml", "[package]\nname = \"core-kernel\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ngeo = \"0.33\"\n");
    r.edit(|d| {
        d.donors[0].exception = Some((
            ynventa::schema::ExceptionKind::Rejected,
            "never adopted".into(),
        ))
    });
    assert!(finding(&r.assess(), "ILLEGAL_EXCEPTION", "geo").is_some());
    let (code, out) = r.cli(&["migrate", "reclaim"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("exception withdrawn"), "{out}");
    assert!(r.declaration().donors[0].exception.is_none());
    let a = r.assess();
    assert!(finding(&a, "ILLEGAL_EXCEPTION", "geo").is_none());
    assert!(finding(&a, "CLAIM_EXCEEDS_EVIDENCE", "geo").is_none());
    let k = ynventa::compact::facts::Knowledge::load(r.path());
    assert!(k.facts.iter().any(|f| f.subject == "geo"
        && f.key == "exception"
        && f.value == "REJECTED: never adopted"));
}

#[test]
fn register_reads_the_licence_from_the_pinned_package_not_from_a_person() {
    let r = extinct_baseline("register");
    r.write("core/Cargo.toml", "[package]\nname = \"core-kernel\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nhashbrown = \"0.15\"\n");
    r.write("Cargo.lock", "version = 4\n\n[[package]]\nname = \"hashbrown\"\nversion = \"0.15.2\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n");
    r.edit(|d| {
        let mut dn = donor("cargo-hashbrown", "hashbrown");
        dn.claimed = DonorState::Discovered;
        dn.license = String::new();
        dn.origin = "https://crates.io/crates/hashbrown".into();
        dn.capabilities.clear();
        dn.cutover = None;
        d.donors.push(dn);
    });
    assert!(finding(&r.assess(), "DISCOVERED_BUT_ACTIVE", "cargo-hashbrown").is_some());
    let home = r.path().with_extension("cargo-home");
    let dir = home.join("registry/src/index.crates.io-0000/hashbrown-0.15.2");
    std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
    let args = |home: &std::path::Path| {
        vec![
            "migrate".to_string(),
            "register".into(),
            "--cargo-home".into(),
            home.display().to_string(),
        ]
    };
    // Without the package's own manifest nothing is registered, and the reason is given.
    let a: Vec<String> = args(&home);
    let (code, out) = r.cli(&a.iter().map(String::as_str).collect::<Vec<_>>());
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("hashbrown-0.15.2 is not in the local registry"),
        "{out}"
    );
    assert_eq!(
        r.declaration().donor("cargo-hashbrown").unwrap().claimed,
        DonorState::Discovered
    );
    // With it, the licence and its provenance come from the pinned version's manifest.
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"hashbrown\"\nversion = \"0.15.2\"\nlicense = \"MIT OR Apache-2.0\"\n",
    )
    .unwrap();
    let (code, out) = r.cli(&a.iter().map(String::as_str).collect::<Vec<_>>());
    assert_eq!(code, 0, "{out}");
    let dn = r.declaration().donor("cargo-hashbrown").unwrap().clone();
    assert_eq!(dn.claimed, DonorState::Registered);
    assert_eq!(dn.license, "MIT OR Apache-2.0");
    assert!(dn
        .provenance
        .iter()
        .any(|p| p == "registry:hashbrown-0.15.2/Cargo.toml"));
    assert!(finding(&r.assess(), "DISCOVERED_BUT_ACTIVE", "cargo-hashbrown").is_none());
}

#[test]
fn register_discovers_new_externals_and_reads_npm_licences_from_the_lockfile() {
    let r = extinct_baseline("register-npm");
    r.write(
        "package.json",
        "{\"name\": \"ui\", \"private\": true, \"devDependencies\": {\"vite\": \"^5.0.0\"}}\n",
    );
    r.write("package-lock.json", "{\"lockfileVersion\": 3, \"packages\": {\"\": {\"name\": \"ui\"}, \"node_modules/vite\": {\"version\": \"5.4.0\", \"license\": \"MIT\"}}}\n");
    let a = r.assess();
    assert!(
        a.findings.iter().any(|f| f.code == "UNREGISTERED_EXTERNAL"),
        "{:#?}",
        a.findings
    );
    let (code, out) = r.cli(&["migrate", "register"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("DISCOVERED npm-vite"), "{out}");
    let d = r.declaration();
    let dn = d
        .donor("npm-vite")
        .expect("new external entered the graph as a donor");
    assert_eq!(dn.claimed, DonorState::Registered);
    assert_eq!(dn.license, "MIT");
    assert!(dn
        .provenance
        .iter()
        .any(|p| p == "npm:package-lock.json#node_modules/vite"));
    let a = r.assess();
    assert!(
        !a.findings
            .iter()
            .any(|f| f.code == "UNREGISTERED_EXTERNAL" || f.code == "DISCOVERED_BUT_ACTIVE"),
        "{:#?}",
        a.findings
    );
}

#[test]
fn program_donors_are_the_projects_that_provide_them() {
    let r = extinct_baseline("programs");
    r.write(
        "core/src/lib.rs",
        "pub fn id() -> u64 { 1 }\npub fn probe() {\n    let _ = std::process::Command::new(\"date\").status();\n    let _ = std::process::Command::new(\"df\").status();\n    let _ = std::process::Command::new(\"kill\").status();\n}\n",
    );
    let (code, out) = r.cli(&["migrate", "register"]);
    assert_eq!(code, 0, "{out}");
    let d = r.declaration();
    // date and df are one donor, GNU coreutils, registered with its origin and licence.
    let cu = d.donor("native-coreutils").expect("{out}");
    assert_eq!(cu.claimed, DonorState::Registered);
    assert_eq!(cu.license, "GPL-3.0-or-later");
    assert_eq!(cu.origin, "https://www.gnu.org/software/coreutils");
    let progs: Vec<&str> = cu.packages.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(progs, vec!["date", "df"]);
    assert!(d.donor("native-date").is_none() && d.donor("native-df").is_none());
    // kill has no single providing project: it stays unregistered, with the reason.
    assert_eq!(
        d.donor("native-kill").unwrap().claimed,
        DonorState::Discovered
    );
    assert!(out.contains("no single providing project"), "{out}");
    // The old keys are kept as knowledge.
    let k = ynventa::compact::facts::Knowledge::load(r.path());
    assert!(k.facts.iter().any(|f| f.subject == "native-date"
        && f.key == "merged_into"
        && f.value == "native-coreutils"));
    // The census now finds date and df under the coreutils donor: only kill is left.
    let a = r.assess();
    let open: Vec<&str> = a
        .findings
        .iter()
        .filter(|f| f.code == "DISCOVERED_BUT_ACTIVE" || f.code == "UNREGISTERED_EXTERNAL")
        .map(|f| f.subject.as_str())
        .collect();
    assert_eq!(open, vec!["native-kill"], "{:#?}", a.findings);
}

#[test]
fn a_program_donor_already_named_for_its_project_is_registered_in_place() {
    let r = extinct_baseline("program-in-place");
    r.write(
        "core/src/lib.rs",
        "pub fn id() -> u64 { 1 }\npub fn fetch() { let _ = std::process::Command::new(\"curl\").status(); }\n",
    );
    let (code, out) = r.cli(&["migrate", "register"]);
    assert_eq!(code, 0, "{out}");
    let dn = r
        .declaration()
        .donor("native-curl")
        .cloned()
        .expect("{out}");
    assert_eq!(dn.claimed, DonorState::Registered, "{out}");
    assert_eq!(dn.license, "curl");
    let a = r.assess();
    assert!(
        !a.findings.iter().any(|f| f.code == "DISCOVERED_BUT_ACTIVE"),
        "{:#?}",
        a.findings
    );
}
