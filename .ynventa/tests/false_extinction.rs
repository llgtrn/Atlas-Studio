//! No false green. Each test starts from a donor that is genuinely EXTINCT and introduces one
//! way a repository could still carry the donor while claiming extinction; the verifier must
//! refuse EXTINCT, name the failing gate, and report the false claim.

mod common;

use common::*;
use ynventa::declare::*;
use ynventa::schema::*;

fn not_extinct(r: &Repo, failing: Gate) -> ynventa::Assessment {
    let a = r.assess();
    let d = a.donor("geo").unwrap();
    assert_ne!(
        d.effective,
        DonorState::Extinct,
        "must not be extinct: {:#?}",
        d.gates
    );
    assert!(
        !gate(&a, "geo", failing),
        "gate {failing} should fail: {:#?}",
        d.gates
    );
    assert!(
        finding(&a, "CLAIM_EXCEEDS_EVIDENCE", "geo").is_some(),
        "the EXTINCT claim must be reported as false"
    );
    assert_ne!(a.metric("extinction_ratio"), "1.000000");
    a
}

#[test]
fn baseline_is_genuinely_extinct() {
    let r = extinct_baseline("baseline");
    let a = r.assess();
    let d = a.donor("geo").unwrap();
    assert_eq!(d.effective, DonorState::Extinct, "{:#?}", d.gates);
    assert!(d.gates.iter().all(|g| g.pass));
    assert!(a.errors().next().is_none(), "{:#?}", a.findings);
    assert_eq!(a.metric("extinction_ratio"), "1.000000");
    assert_eq!(a.metric("proof_completion_ratio"), "1.000000");
}

#[test]
fn donor_source_deleted_but_crate_dependency_remains() {
    let r = extinct_baseline("dep-remains");
    r.write("core/Cargo.toml", "[package]\nname = \"core-kernel\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ngeo = \"0.33\"\n");
    not_extinct(&r, Gate::RuntimeEdges);
}

#[test]
fn build_dependency_remains() {
    let r = extinct_baseline("build-dep");
    r.write("core/Cargo.toml", "[package]\nname = \"core-kernel\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[build-dependencies]\ngeo = \"0.33\"\n");
    let a = not_extinct(&r, Gate::BuildEdges);
    assert!(gate(&a, "geo", Gate::RuntimeEdges));
}

#[test]
fn wrapper_calls_donor() {
    let r = extinct_baseline("wrapper");
    r.write(
        "substrate/geo/Cargo.toml",
        "[package]\nname = \"geo-native\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ngeo = \"0.33\"\n",
    );
    r.write(
        "substrate/geo/src/lib.rs",
        "pub fn distance(a: (f64, f64), b: (f64, f64)) -> f64 { use geo::EuclideanDistance; geo::Point::from(a).euclidean_distance(&geo::Point::from(b)) }\n#[cfg(test)]\nmod tests { #[test] fn regression_distance() {} }\n",
    );
    r.prove();
    let a = not_extinct(&r, Gate::CapabilityCoverage);
    let cap = &a.donor("geo").unwrap().capabilities[0];
    assert!(
        !cap.native && cap.native_detail.contains("WRAPPER"),
        "{}",
        cap.native_detail
    );
    assert!(!gate(&a, "geo", Gate::SourceImports));
    assert_eq!(a.analysis.node_status["geo"], NativeStatus::Wrapper);
    assert!(
        a.donor("geo").unwrap().effective < DonorState::ParityProven,
        "a wrapper never reaches PARITY_PROVEN"
    );
}

#[test]
fn wrapper_through_an_internal_dependency() {
    let r = extinct_baseline("transitive");
    r.write("Cargo.toml", "[workspace]\nresolver = \"2\"\nmembers = [\"core\", \"substrate/geo\", \"substrate/shim\", \"tests\"]\n");
    r.write("substrate/shim/Cargo.toml", "[package]\nname = \"shim\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ngeo = \"0.33\"\n");
    r.write("substrate/shim/src/lib.rs", "pub use geo::Point;\n");
    r.write(
        "substrate/geo/Cargo.toml",
        "[package]\nname = \"geo-native\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nshim = { path = \"../shim\" }\n",
    );
    r.edit(|d| {
        d.repository.nodes.push(node(
            "shim",
            NodeKind::Substrate,
            "substrate/shim",
            "substrate/shim",
        ))
    });
    r.prove();
    let a = not_extinct(&r, Gate::CapabilityCoverage);
    let detail = &a.donor("geo").unwrap().capabilities[0].native_detail;
    assert!(detail.contains("geo -> shim"), "{detail}");
    assert_eq!(
        a.analysis.node_status["geo"],
        NativeStatus::Native,
        "geo itself imports nothing"
    );
}

#[test]
fn all_code_replaced_but_proof_missing() {
    let r = extinct_baseline("no-proof");
    r.remove(".ynventa/evidence");
    let a = not_extinct(&r, Gate::ParityProofs);
    let cap = &a.donor("geo").unwrap().capabilities[0];
    assert!(cap.native, "the code is native; only the proof is missing");
    assert_eq!(cap.parity[0].1, ynventa::evidence::Verdict::Unrecorded);
    assert_eq!(a.donor("geo").unwrap().effective, DonorState::NativeShadow);
}

#[test]
fn declared_proof_that_does_not_exist() {
    let r = extinct_baseline("absent-proof");
    r.edit(|d| {
        d.donors[0].capabilities[0].proofs[0].locator = "tests/tests/parity.rs::no_such_test".into()
    });
    let a = not_extinct(&r, Gate::ParityProofs);
    assert_eq!(
        a.donor("geo").unwrap().capabilities[0].parity[0].1,
        ynventa::evidence::Verdict::Absent
    );
}

#[test]
fn proof_is_stale_after_the_replacement_changes() {
    let r = extinct_baseline("stale");
    r.write("substrate/geo/src/extra.rs", "pub fn changed() {}\n");
    let a = not_extinct(&r, Gate::ParityProofs);
    assert_eq!(
        a.donor("geo").unwrap().capabilities[0].parity[0].1,
        ynventa::evidence::Verdict::Stale
    );
    r.prove();
    assert_eq!(
        r.assess().donor("geo").unwrap().effective,
        DonorState::Extinct
    );
}

#[test]
fn failing_run_of_the_same_bytes_wins() {
    struct Fail;
    impl ynventa::evidence::Runner for Fail {
        fn run(
            &mut self,
            _: &std::path::Path,
            _: &ynventa::repository::files::Files,
            _: &Proof,
        ) -> (bool, String) {
            (false, "failed".into())
        }
    }
    let r = extinct_baseline("fail");
    let d = r.declaration();
    let files = ynventa::repository::files::Files::scan(r.path()).unwrap();
    ynventa::evidence::prove(r.path(), &files, &d, None, &mut Fail);
    let a = not_extinct(&r, Gate::ParityProofs);
    assert_eq!(
        a.donor("geo").unwrap().capabilities[0].parity[0].1,
        ynventa::evidence::Verdict::Fail
    );
}

#[test]
fn ffi_into_the_donor() {
    let r = extinct_baseline("ffi");
    r.edit(|d| {
        d.donors[0].packages.push(Package {
            ecosystem: Ecosystem::Native,
            name: "geos".into(),
        })
    });
    r.write("core/src/lib.rs", "#[link(name = \"geos\")]\nextern \"C\" { fn GEOSversion() -> *const u8; }\npub fn id() -> u64 { 1 }\n");
    not_extinct(&r, Gate::LinkedEdges);
}

#[test]
fn donor_executed_as_a_process() {
    let r = extinct_baseline("process");
    r.edit(|d| {
        d.donors[0].packages.push(Package {
            ecosystem: Ecosystem::Native,
            name: "proj".into(),
        })
    });
    r.write("core/src/lib.rs", "pub fn id() -> u64 { std::process::Command::new(\"proj\").status().map(|_| 1).unwrap_or(0) }\n");
    not_extinct(&r, Gate::RuntimeEdges);
}

#[test]
fn live_oracle_in_tests_is_not_extinct() {
    let r = extinct_baseline("oracle");
    r.write(
        "tests/Cargo.toml",
        "[package]\nname = \"verification\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dev-dependencies]\ngeo-native = { path = \"../substrate/geo\" }\ngeo = \"0.33\"\n",
    );
    let a = not_extinct(&r, Gate::TestEdges);
    assert!(gate(&a, "geo", Gate::RuntimeEdges) && gate(&a, "geo", Gate::BuildEdges));
    assert_eq!(
        a.analysis.node_status["tests"],
        NativeStatus::Native,
        "test-only use makes no node a wrapper"
    );
}

#[test]
fn donor_source_hidden_in_research_or_fixtures() {
    for hidden in [
        "research/geo-study/src/algorithm.rs",
        "tests/fixtures/geo/src/lib.rs",
        "vendor/geo/src/lib.rs",
        ".atlas/temporary/donors/geo/src/lib.rs",
    ] {
        let r = extinct_baseline("hidden");
        let dir = hidden.rsplitn(3, '/').nth(2).unwrap().to_string();
        r.write(hidden, "// donor source\npub fn f() {}\n");
        r.edit(|d| d.donors[0].source_paths = vec![dir.clone()]);
        let a = not_extinct(&r, Gate::ResidentSource);
        assert!(
            a.donor("geo")
                .unwrap()
                .facts
                .resident
                .iter()
                .any(|f| f == hidden),
            "{hidden}"
        );
    }
}

#[test]
fn deletion_is_not_extinction() {
    // The directory is gone and the claim says EXTINCT, but nothing was ever decomposed,
    // replaced or proven.
    let r = extinct_baseline("deleted");
    r.edit(|d| {
        d.donors[0].capabilities.clear();
        d.donors[0].cutover = None;
    });
    let a = not_extinct(&r, Gate::CapabilityCoverage);
    assert_eq!(a.donor("geo").unwrap().effective, DonorState::Registered);
}

#[test]
fn unregistering_a_difficult_donor_cannot_raise_the_ratio() {
    let r = extinct_baseline("vanish");
    r.edit(|d| {
        let mut hard = donor("hard", "hard");
        hard.capabilities.clear();
        hard.claimed = DonorState::Registered;
        d.donors.push(hard);
    });
    let before = r.assess();
    assert_eq!(before.metric("extinction_ratio"), "0.500000");
    assert_eq!(r.cli(&["census", "--record"]).0, 0);
    r.edit(|d| d.donors.retain(|x| x.key != "hard"));
    let after = r.assess();
    assert!(finding(&after, "DONOR_VANISHED", "hard").is_some());
    assert_eq!(after.metric("donors_registered"), "2");
    assert_eq!(
        after.metric("extinction_ratio"),
        "0.500000",
        "the denominator never shrinks"
    );
}

#[test]
fn rejecting_or_superseding_an_active_donor_is_illegal() {
    let r = extinct_baseline("reject");
    r.write("core/Cargo.toml", "[package]\nname = \"core-kernel\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ngeo = \"0.33\"\n");
    r.edit(|d| d.donors[0].exception = Some((ExceptionKind::Rejected, "never needed".into())));
    let a = r.assess();
    assert!(finding(&a, "ILLEGAL_EXCEPTION", "geo").is_some());
    assert_eq!(a.metric("donors_rejected"), "0");
    assert_eq!(a.metric("donors_active"), "1");

    r.edit(|d| {
        let mut succ = donor("successor", "other");
        succ.capabilities.clear();
        succ.claimed = DonorState::Registered;
        d.donors.push(succ);
        d.donors[0].exception = Some((ExceptionKind::Superseded, "successor".into()));
    });
    let a = r.assess();
    assert!(finding(&a, "ILLEGAL_EXCEPTION", "geo")
        .unwrap()
        .detail
        .contains("packages"));
    assert_eq!(a.metric("donors_superseded"), "0");
}

#[test]
fn donor_fallback_shim_blocks_extinction() {
    let r = extinct_baseline("fallback");
    r.edit(|d| {
        d.migration.shims.push(Shim {
            key: "geo-fallback".into(),
            kind: ShimKind::DonorFallback,
            path: String::new(),
            serves: "geo".into(),
            expires: (ExpiryKind::DonorExtinct, "geo".into()),
        })
    });
    not_extinct(&r, Gate::RollbackIndependent);
}

#[test]
fn replacement_must_be_a_canonical_native_role() {
    let r = extinct_baseline("compat-replacement");
    r.edit(|d| {
        d.repository.nodes[1].kind = NodeKind::Compat;
        d.repository.nodes[1].canonical_path = "compat/geo".into();
    });
    let a = not_extinct(&r, Gate::CanonicalReplacement);
    assert!(finding(&a, "COMPAT_WITHOUT_EXPIRY", "geo").is_some());
}

#[test]
fn unregistered_external_joins_the_denominator() {
    let r = extinct_baseline("unregistered");
    r.write("core/Cargo.toml", "[package]\nname = \"core-kernel\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n");
    let a = r.assess();
    assert!(finding(&a, "UNREGISTERED_EXTERNAL", "CARGO:serde").is_some());
    assert_eq!(a.metric("extinction_ratio"), "0.500000");
    assert_eq!(a.metric("runtime_external_edges"), "1");
    assert_eq!(a.analysis.node_status["core"], NativeStatus::Dependent);
}

#[test]
fn platform_libraries_are_not_foreign() {
    let r = extinct_baseline("platform");
    r.write(
        "core/src/lib.rs",
        "#[cfg(windows)]\n#[link(name = \"kernel32\")]\nextern \"system\" { fn GetLastError() -> u32; }\npub fn id() -> u64 { 1 }\n",
    );
    let a = r.assess();
    assert_eq!(a.metric("linked_external_edges"), "0");
    assert!(a.analysis.unregistered.is_empty());
    assert_eq!(a.donor("geo").unwrap().effective, DonorState::Extinct);
}
