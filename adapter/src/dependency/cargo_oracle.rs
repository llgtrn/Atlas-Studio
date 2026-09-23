//! Independent verification oracle for `cargo.rs`'s dependency-census extractor -- **test-only**,
//! never reachable from production code (`.atlas/contracts/RECURSIVE-SELF-CENSUS.md`'s
//! stable-validator law: a candidate must never be sole authority proving itself).
//!
//! # Why this exists
//!
//! An earlier evidence record for this extractor claimed independent validation by running the
//! pre-existing `systemize` CLI against this repository and inspecting its output. That claim was
//! false: `systemize` calls `adapter::census_cargo_workspace` internally -- the exact candidate
//! parser under test, reached through a different entry point. Candidate implementation -> CLI
//! wrapper -> candidate implementation result is not independent evidence, it is the same code
//! disagreeing with itself only if it has a bug that also breaks its own self-consistency, which
//! proves nothing about correctness against ground truth.
//!
//! This module is a **second, separately-sourced oracle**: `cargo metadata --format-version=1
//! --offline`, Cargo's own dependency resolver, run as a subprocess and diffed against this
//! crate's static parser output. It shares no code with `cargo.rs`. Cargo's resolver is
//! authoritative for what Cargo itself actually resolved -- exactly the fact `cargo.rs` exists to
//! reconstruct statically.
//!
//! # Canonical census extractor != independent verification oracle
//!
//! `cargo.rs::census_cargo_workspace` is the canonical, production census path: pure static
//! parsing, no process execution, matching the "ingestion is not execution" security boundary
//! (`.atlas/contracts/DEPENDENCY-CENSUS.md`). This module executes `cargo metadata` as a
//! subprocess and therefore must **never** be called from production code, `systemize`, or
//! anything gating coding admission -- it exists strictly as `#[cfg(test)]` verification evidence,
//! run only in this crate's own test suite, never shipped in the `adapter` library itself.

use super::cargo::census_cargo_workspace;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    process::Command,
};

/// One `(consumer name, provider name, provider version)` resolved edge, comparable across both
/// the candidate extractor and the oracle regardless of how each represents source/kind/evidence
/// metadata (those are candidate-only concerns `cargo metadata` doesn't need to agree on).
type EdgeTriple = (String, String, String);

/// Runs `cargo metadata` fully offline against `root`'s already-resolved `Cargo.lock` and local
/// registry cache -- no network access, and no mutation of the lockfile or the source tree.
fn run_cargo_metadata(root: &Path) -> serde_json::Value {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1", "--offline"])
        .current_dir(root)
        .output()
        .expect("cargo metadata must be runnable offline against an already-resolved workspace");
    assert!(
        output.status.success(),
        "cargo metadata failed (exit {:?}): {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata must emit valid JSON")
}

/// Every resolved dependency edge `cargo metadata` itself reports, as `(consumer, provider,
/// provider_version)` triples -- Cargo's own resolver, not this crate's parser.
fn oracle_edges(metadata: &serde_json::Value) -> BTreeSet<EdgeTriple> {
    let packages_by_id: BTreeMap<&str, (&str, &str)> = metadata["packages"]
        .as_array()
        .expect("metadata.packages must be an array")
        .iter()
        .map(|package| {
            (
                package["id"].as_str().expect("package.id must be a string"),
                (
                    package["name"]
                        .as_str()
                        .expect("package.name must be a string"),
                    package["version"]
                        .as_str()
                        .expect("package.version must be a string"),
                ),
            )
        })
        .collect();

    let mut edges = BTreeSet::new();
    for node in metadata["resolve"]["nodes"]
        .as_array()
        .expect("metadata.resolve.nodes must be an array")
    {
        let consumer_id = node["id"].as_str().expect("node.id must be a string");
        let (consumer_name, _) = packages_by_id[consumer_id];
        for dep in node["deps"].as_array().expect("node.deps must be an array") {
            let provider_id = dep["pkg"].as_str().expect("dep.pkg must be a string");
            let (provider_name, provider_version) = packages_by_id[provider_id];
            edges.insert((
                consumer_name.to_owned(),
                provider_name.to_owned(),
                provider_version.to_owned(),
            ));
        }
    }
    edges
}

/// The same triples from the candidate static parser's own `DependencyClosureReport`.
fn candidate_edges(root: &Path) -> BTreeSet<EdgeTriple> {
    census_cargo_workspace(root)
        .expect("reading this repository's own Cargo.lock/Cargo.toml must not fail")
        .expect("this repository's own root has a Cargo.lock")
        .edges
        .into_iter()
        .map(|edge| (edge.consumer, edge.provider.name, edge.provider.version))
        .collect()
    // Deliberately drops `kind`/`source_kind`/`evidence_path`/`checksum`: `cargo metadata` does
    // not report an independent value for evidence-path or checksum, and its dep_kinds/source
    // shapes differ enough from this crate's own vocabulary that comparing them 1:1 would compare
    // two different representations rather than the same fact -- resolved identity and edge
    // topology (who requires which exact provider instance) is the shared ground truth both sides
    // can be checked against without smuggling in an assumption about either one's own shape.
}

#[test]
fn candidate_parser_agrees_with_cargo_metadata_on_this_repositorys_real_resolved_edges() {
    // This repository's own real workspace root -- the same corpus `cargo.rs`'s own
    // `real_self_census_of_this_repositorys_own_workspace_finds_no_dangling_references` test
    // reads, but verified here against a second, independently-sourced oracle instead of only
    // this crate's own synthetic fixtures or its own tests.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("adapter/ has a parent directory");

    let metadata = run_cargo_metadata(root);
    let oracle = oracle_edges(&metadata);
    let candidate = candidate_edges(root);

    assert_eq!(
        candidate, oracle,
        "candidate static parser and cargo metadata (an independent oracle) disagree on this \
         repository's own real resolved dependency edges"
    );
    assert!(
        !oracle.is_empty(),
        "the oracle itself must observe real edges, or this comparison proves nothing"
    );
}

#[test]
fn candidate_parser_agrees_with_cargo_metadata_on_resolved_package_identity() {
    // Beyond edge topology: every package the candidate resolved must be the exact same
    // (name, version) pair the independent oracle resolved too -- no phantom or mismatched
    // instances on either side.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("adapter/ has a parent directory");

    let metadata = run_cargo_metadata(root);
    let oracle_instances: BTreeSet<(String, String)> = metadata["packages"]
        .as_array()
        .expect("metadata.packages must be an array")
        .iter()
        .map(|package| {
            (
                package["name"].as_str().unwrap().to_owned(),
                package["version"].as_str().unwrap().to_owned(),
            )
        })
        .collect();

    let candidate_report = census_cargo_workspace(root).unwrap().unwrap();
    let candidate_instances: BTreeSet<(String, String)> = candidate_report
        .edges
        .iter()
        .map(|edge| (edge.provider.name.clone(), edge.provider.version.clone()))
        .collect();

    // The candidate's instance set is every package reachable as *someone's* dependency; the
    // oracle's is every package in the workspace's resolved graph (also includes the workspace
    // roots themselves, which never appear as a `provider` on the candidate side since nothing
    // in this workspace depends on itself). So this checks candidate ⊆ oracle, not equality --
    // asserting equality here would be a false claim neither the candidate nor oracle nor this
    // module's authors are contractually stating.
    let missing_from_oracle: Vec<_> = candidate_instances.difference(&oracle_instances).collect();
    assert!(
        missing_from_oracle.is_empty(),
        "candidate resolved a package identity the independent oracle never resolved at all: {missing_from_oracle:?}"
    );
}
