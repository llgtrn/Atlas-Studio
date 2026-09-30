//! Chronica as one system: capsules, the linker, the technology graph and agent context.

mod common;

use common::*;
use ynventa::capsule::Capsule;
use ynventa::declare::*;
use ynventa::linker::{link, SystemImage};
use ynventa::schema::*;

/// A second shard (Esellios) with its own keys, a node that needs `identity.digest`.
fn commerce_shard(name: &str) -> Repo {
    let r = Repo::new(name);
    r.write(
        "Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"domain/order\"]\n",
    );
    r.write("README.md", "# commerce\n");
    r.write(
        "domain/order/Cargo.toml",
        "[package]\nname = \"order\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    r.write("domain/order/src/lib.rs", "pub fn place_order(cart: Cart, buyer: BuyerId) -> Result<OrderId, OrderError> { todo!() }\n");
    let mut order = node(
        "commerce.order",
        NodeKind::Domain,
        "domain/order",
        "domain/order",
    );
    order.requires = vec!["identity.digest".into()];
    order.provides = vec!["commerce.order".into()];
    r.store(&Declaration {
        repository: Repository {
            system: "chronica".into(),
            shard: "esellios".into(),
            name: name.into(),
            origin: "llgtrn/Esellios".into(),
            nodes: vec![
                order,
                node(
                    "ynventa.esellios",
                    NodeKind::Ynventa,
                    ".ynventa",
                    ".ynventa",
                ),
            ],
            edges: vec![],
        },
        donors: vec![],
        migration: Migration::default(),
        technologies: vec![],
    });
    r
}

/// The Mechatron baseline, declaring a canonical native technology implementing `identity.digest`.
fn technology_shard(name: &str) -> Repo {
    let r = extinct_baseline(name);
    r.write("core/src/digest.rs", "pub fn digest(bytes: &[u8]) -> Digest { todo!() }\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn vectors() {}\n}\n");
    r.edit(|d| {
        d.repository.nodes[0].provides = vec!["identity.digest".into()];
        d.technologies = vec![Technology {
            key: "hash.digest".into(),
            name: "native digest".into(),
            kind: TechnologyKind::Algorithm,
            claimed: TechnologyLifecycle::Canonical,
            purpose: "content identity".into(),
            implements: vec!["identity.digest".into()],
            node: "core".into(),
            sources: vec!["core/src/digest.rs".into()],
            invariants: vec!["published vectors".into()],
            proofs: vec![proof(ProofKind::Regression, "core/src/digest.rs::vectors")],
            lineage: vec!["geo".into()],
            relations: vec![],
            claims: vec![Improvement {
                dimension: Dimension::Latency,
                baseline: "donor".into(),
                workload: "1 MiB".into(),
                statement: "faster".into(),
                evidence: vec![],
            }],
        }];
    });
    r.prove();
    r
}

fn link_repos(repos: &[&Repo]) -> SystemImage {
    let caps: Vec<Capsule> = repos
        .iter()
        .map(|r| Capsule::compile(&r.assess()))
        .collect();
    link(&caps)
}

fn has(image: &SystemImage, code: &str) -> bool {
    image.issues.iter().any(|i| i.code == code)
}

#[test]
fn required_capabilities_must_resolve_across_shards() {
    let commerce = commerce_shard("req-commerce");
    let image = link_repos(&[&commerce]);
    let unresolved = image
        .issues
        .iter()
        .find(|i| i.code == "UNPROVIDED_CAPABILITY")
        .unwrap();
    assert_eq!(unresolved.subject, "identity.digest");
    assert!(!image.pass());
    let machine = technology_shard("req-machine");
    let image = link_repos(&[&commerce, &machine]);
    assert!(!has(&image, "UNPROVIDED_CAPABILITY"), "{:#?}", image.issues);
    let cap = image
        .capabilities
        .iter()
        .find(|c| c.key == "identity.digest")
        .unwrap();
    assert_eq!(
        cap.providers,
        vec![("mechatron".to_string(), "core".to_string())]
    );
    assert_eq!(
        cap.requirers,
        vec![("esellios".to_string(), "commerce.order".to_string())]
    );
    assert_eq!(cap.technologies, vec!["hash.digest".to_string()]);
}

#[test]
fn technology_lifecycle_is_computed_and_claims_are_checked() {
    let r = technology_shard("tech-life");
    let a = r.assess();
    let t = a
        .technologies
        .iter()
        .find(|t| t.key == "hash.digest")
        .unwrap();
    assert_eq!(
        t.effective,
        TechnologyLifecycle::Canonical,
        "{}",
        t.stopped_by
    );
    assert_eq!(a.metric("technologies_canonical"), "1");
    assert!(
        finding(&a, "IMPROVEMENT_UNPROVEN", "hash.digest").is_some(),
        "\"faster\" without evidence stays unproven"
    );
    // Proofs bind to the technology's canonical sources: unrelated edits keep them fresh...
    r.write("core/src/lib.rs", "pub fn id() -> u64 { 2 }\n");
    assert_eq!(
        r.assess().technologies[0].effective,
        TechnologyLifecycle::Canonical
    );
    // ...an edit of the sources makes them stale, and the CANONICAL claim becomes a false claim.
    r.write("core/src/digest.rs", "pub fn digest(bytes: &[u8]) -> Digest { unimplemented!() }\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn vectors() {}\n}\n");
    let a = r.assess();
    assert_eq!(a.technologies[0].effective, TechnologyLifecycle::Native);
    assert!(finding(&a, "TECHNOLOGY_CLAIM_EXCEEDS_EVIDENCE", "hash.digest").is_some());
    // A node that wraps a donor does not taint a technology whose sources stand alone...
    r.write("core/Cargo.toml", "[package]\nname = \"core-kernel\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ngeo = \"0.33\"\n");
    assert_eq!(
        r.assess().technologies[0].effective,
        TechnologyLifecycle::Native
    );
    // ...but sources that import the donor, or reach into the wrapping node, are not native.
    for body in [
        "pub fn digest(bytes: &[u8]) -> geo::Point { todo!() }\n",
        "pub fn digest(bytes: &[u8]) -> crate::Point { todo!() }\n",
    ] {
        r.write("core/src/digest.rs", body);
        let a = r.assess();
        assert_eq!(
            a.technologies[0].effective,
            TechnologyLifecycle::Experimental,
            "{body}"
        );
        assert!(a.technologies[0].stopped_by.contains("not self-contained"));
    }
}

#[test]
fn technology_is_reused_by_materialization_not_by_service() {
    let machine = technology_shard("reuse-machine");
    let commerce = commerce_shard("reuse-commerce");
    let (code, out) = commerce.cli(&[
        "technology",
        "materialize",
        "hash.digest",
        "--from",
        &machine.path().display().to_string(),
        "--node",
        "commerce.order",
        "--into",
        "domain/order/src/tech",
    ]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        commerce.read("domain/order/src/tech/digest.rs"),
        machine.read("core/src/digest.rs")
    );
    assert_eq!(
        commerce
            .declaration()
            .node("commerce.order")
            .unwrap()
            .reuses,
        vec!["hash.digest".to_string()]
    );
    // Adoption is a system fact: decided by linking.
    let image = link_repos(&[&machine, &commerce]);
    let t = image
        .technologies
        .iter()
        .find(|t| t.key == "hash.digest")
        .unwrap();
    assert_eq!(t.effective, TechnologyLifecycle::Adopted);
    assert_eq!(t.birthplace, "mechatron");
    assert_eq!(t.consumers, vec!["esellios".to_string()]);
    assert_eq!(image.counts().technologies_adopted, 1);
    // No runtime coupling was created: no cross-shard code dependency.
    assert!(!has(&image, "CROSS_SHARD_CODE_DEPENDENCY"));
    // A local edit of materialized source is a silent fork.
    commerce.write(
        "domain/order/src/tech/digest.rs",
        "pub fn digest(bytes: &[u8]) -> Digest { unimplemented!() }\n",
    );
    assert!(finding(&commerce.assess(), "MATERIALIZED_FORK", "hash.digest").is_some());
}

#[test]
fn duplicated_technology_without_relation_fails_linking() {
    let machine = technology_shard("dup-machine");
    let commerce = commerce_shard("dup-commerce");
    commerce.write(
        "domain/order/src/digest.rs",
        "pub fn digest(bytes: &[u8]) -> Digest { todo!() }\n",
    );
    commerce.edit(|d| {
        d.repository.nodes[0]
            .provides
            .push("identity.digest".into());
        d.technologies = vec![Technology {
            key: "hash.order-digest".into(),
            name: "another digest".into(),
            kind: TechnologyKind::Algorithm,
            claimed: TechnologyLifecycle::Experimental,
            purpose: String::new(),
            implements: vec!["identity.digest".into()],
            node: "commerce.order".into(),
            sources: vec!["domain/order/src/digest.rs".into()],
            invariants: vec![],
            proofs: vec![],
            lineage: vec![],
            relations: vec![],
            claims: vec![],
        }];
    });
    let image = link_repos(&[&machine, &commerce]);
    assert!(has(&image, "DUPLICATE_TECHNOLOGY"), "{:#?}", image.issues);
    assert!(has(&image, "DUPLICATE_CAPABILITY"));
    // The same operation shape in both shards is reported from YIR.
    assert!(
        image
            .duplicate_symbols
            .iter()
            .any(|(_, sig, m)| sig.starts_with("digest(") && m.len() == 2),
        "{:#?}",
        image.duplicate_symbols
    );
    // Declaring the relation makes the specialization legitimate.
    commerce.edit(|d| {
        d.technologies[0].relations = vec![Relation {
            kind: EdgeKind::Specializes,
            target: "hash.digest".into(),
        }];
        d.repository.edges = vec![Edge {
            from: "commerce.order".into(),
            to: "core".into(),
            kind: EdgeKind::Specializes,
            scope: Scope::Semantic,
        }];
    });
    let image = link_repos(&[&machine, &commerce]);
    assert!(
        !has(&image, "DUPLICATE_TECHNOLOGY") && !has(&image, "DUPLICATE_CAPABILITY"),
        "{:#?}",
        image.issues
    );
    // A third shard's alternative for the same canonical technology joins its declared family:
    // no pairwise relation to every sibling is needed...
    let third = commerce_shard("dup-third");
    third.write(
        "domain/order/src/digest.rs",
        "pub fn digest_alt(bytes: &[u8]) -> Digest { todo!() }\n",
    );
    third.edit(|d| {
        d.repository.shard = "fi-game".into();
        d.repository.origin = "llgtrn/Fi-game".into();
        d.repository.nodes[0].key = "finance.digest".into();
        d.repository.nodes[0].provides = vec![];
        d.repository.nodes[0].requires = vec![];
        d.repository.nodes[1].key = "ynventa.fi-game".into();
        d.technologies = vec![Technology {
            key: "hash.alt-digest".into(),
            name: "alternative digest".into(),
            kind: TechnologyKind::Algorithm,
            claimed: TechnologyLifecycle::Experimental,
            purpose: String::new(),
            implements: vec!["identity.digest".into()],
            node: "finance.digest".into(),
            sources: vec!["domain/order/src/digest.rs".into()],
            invariants: vec![],
            proofs: vec![],
            lineage: vec![],
            relations: vec![Relation {
                kind: EdgeKind::AlternativeFor,
                target: "hash.digest".into(),
            }],
            claims: vec![],
        }];
    });
    let image = link_repos(&[&machine, &commerce, &third]);
    assert!(!has(&image, "DUPLICATE_TECHNOLOGY"), "{:#?}", image.issues);
    // ...but without its relation it is an independent duplicate of both.
    third.edit(|d| d.technologies[0].relations.clear());
    let image = link_repos(&[&machine, &commerce, &third]);
    let dups = image
        .issues
        .iter()
        .filter(|i| i.code == "DUPLICATE_TECHNOLOGY")
        .count();
    assert_eq!(dups, 2, "{:#?}", image.issues);
}

#[test]
fn shards_couple_only_through_capabilities_and_technologies() {
    let machine = technology_shard("couple-machine");
    let commerce = commerce_shard("couple-commerce");
    commerce.edit(|d| {
        d.repository.edges = vec![Edge {
            from: "commerce.order".into(),
            to: "core".into(),
            kind: EdgeKind::DependsOn,
            scope: Scope::Build,
        }]
    });
    let image = link_repos(&[&machine, &commerce]);
    let e = image
        .issues
        .iter()
        .find(|i| i.code == "CROSS_SHARD_CODE_DEPENDENCY")
        .unwrap();
    assert!(e.detail.contains("esellios") && e.detail.contains("mechatron"));
    // A reference to a node no shard declares is a link error.
    commerce.edit(|d| {
        d.repository.edges = vec![Edge {
            from: "commerce.order".into(),
            to: "payment.unknown".into(),
            kind: EdgeKind::DependsOn,
            scope: Scope::Architectural,
        }]
    });
    let image = link_repos(&[&machine, &commerce]);
    assert!(has(&image, "UNRESOLVED_REFERENCE"), "{:#?}", image.issues);
}

#[test]
fn capsules_are_deterministic_and_round_trip() {
    let r = technology_shard("capsule");
    let a = Capsule::compile(&r.assess());
    let b = Capsule::compile(&r.assess());
    assert_eq!(a.encode(), b.encode());
    assert_eq!(Capsule::decode(&a.encode()).unwrap(), a);
    assert_eq!(a.provides(), vec!["identity.digest".to_string()]);
    assert!(a.owns().contains(&"technology/hash.digest".to_string()));
    assert!(a
        .symbols
        .iter()
        .any(|s| s.name == "digest" && s.output == "Digest"));
    let (code, out) = r.cli(&["capsule"]);
    assert_eq!(code, 0, "{out}");
    assert!(r.exists("target/ynventa/mechatron.ynv"));
}

#[test]
fn agent_context_knows_what_exists_elsewhere() {
    let machine = technology_shard("ctx-machine");
    let commerce = commerce_shard("ctx-commerce");
    let sys = commerce
        .path()
        .join("target/chronica.system.ynv")
        .display()
        .to_string();
    let (_, out) = commerce.cli(&[
        "link",
        &machine.path().display().to_string(),
        &commerce.path().display().to_string(),
        "--out",
        &sys,
    ]);
    assert!(out.contains("system link"), "{out}");
    let (code, ctx) = commerce.cli(&["context", "--system", &sys]);
    assert_eq!(code, 0);
    assert!(ctx.contains("CURRENT PHYSICAL SHARD\n  esellios"), "{ctx}");
    assert!(
        ctx.contains("identity.digest <- core in mechatron"),
        "{ctx}"
    );
    assert!(ctx.contains("hash.digest"), "{ctx}");
    assert!(ctx.contains("FORBIDDEN DUPLICATION"), "{ctx}");
    let (_, search) = commerce.cli(&["technology", "search", "digest", "--system", &sys]);
    assert!(
        search.contains("hash.digest") && search.contains("born in mechatron"),
        "{search}"
    );
    let (_, show) = commerce.cli(&["show", "identity.digest", "--system", &sys]);
    assert!(
        show.contains("<- PROVIDES")
            && show.contains("<- REQUIRES")
            && show.contains("<- IMPLEMENTS"),
        "{show}"
    );
    let (_, back) = commerce.cli(&["backlinks", "ynv://chronica/core", "--system", &sys]);
    assert!(back.contains("physical owner mechatron"), "{back}");
}
