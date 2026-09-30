//! The executable conformance suite. Pointed at any Ynventa repository it answers, with one
//! output schema and no repository-specific interpretation: does this repository carry the
//! canonical subsystem unmodified, speak protocol v1, and hold state the protocol accepts?

use crate::compact::facts::{fold, Fact};
use crate::declare;
use crate::extinction::{gates, CapabilityVerdict, DonorFacts};
use crate::formats::json::Json;
use crate::graph::{Graph, NodeId};
use crate::metrics::DEFINITIONS;
use crate::protocol::{
    own_subsystem_dir, schema_identity, subsystem_digest, Snapshot, COMMANDS,
    YNVENTA_PROTOCOL_VERSION,
};
use crate::schema::{FactKind, Scope};
use crate::{Assessment, Severity};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Check {
    pub id: &'static str,
    pub pass: bool,
    pub detail: String,
}

impl Check {
    fn new(id: &'static str, pass: bool, detail: impl Into<String>) -> Check {
        Check {
            id,
            pass,
            detail: detail.into(),
        }
    }
}

fn no_findings(a: &Assessment, id: &'static str, codes: &[&str]) -> Check {
    let hits: Vec<String> = a
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error && codes.contains(&f.code.as_str()))
        .map(|f| format!("{} {}", f.code, f.subject))
        .collect();
    let n = hits.len();
    Check::new(
        id,
        hits.is_empty(),
        if hits.is_empty() {
            "none".to_string()
        } else {
            format!(
                "{n}: {}",
                hits.into_iter().take(5).collect::<Vec<_>>().join("; ")
            )
        },
    )
}

/// The protocol checks. `ynventa_protocol_conformance` = passed / total of exactly these.
pub fn protocol_checks(a: &Assessment, reference: Option<&Snapshot>) -> Vec<Check> {
    let mut v = Vec::new();
    let snap = Snapshot::load(&a.root);
    let canonical = reference
        .cloned()
        .unwrap_or_else(|| Snapshot::current(own_subsystem_dir()));
    match &snap {
        None => {
            for id in [
                "protocol.version",
                "protocol.schema_identity",
                "protocol.subsystem_integrity",
                "protocol.canonical_subsystem",
                "protocol.commands",
            ] {
                v.push(Check::new(id, false, "no .ynventa/protocol.snapshot"));
            }
        }
        Some(s) => {
            v.push(Check::new(
                "protocol.version",
                s.protocol == "ynventa" && s.version == YNVENTA_PROTOCOL_VERSION,
                format!("{} {}", s.protocol, s.version),
            ));
            v.push(Check::new(
                "protocol.schema_identity",
                s.schema == schema_identity(),
                format!("repository {} verifier {}", s.schema, schema_identity()),
            ));
            let actual = subsystem_digest(&a.root.join(".ynventa"));
            v.push(Check::new(
                "protocol.subsystem_integrity",
                actual == s.subsystem,
                format!("recorded {} actual {actual}", s.subsystem),
            ));
            v.push(Check::new(
                "protocol.canonical_subsystem",
                actual == canonical.subsystem && s.schema == canonical.schema,
                format!("repository {actual} canonical {}", canonical.subsystem),
            ));
            let cmds = COMMANDS
                .iter()
                .map(|(c, _)| *c)
                .collect::<Vec<_>>()
                .join(" ");
            v.push(Check::new(
                "protocol.commands",
                s.commands == cmds,
                s.commands.clone(),
            ));
        }
    }

    let d = &a.declaration;
    let fixed = declare::parse_repository(&declare::render_repository(&d.repository)).ok()
        == Some(d.repository.clone())
        && declare::parse_donors(&declare::render_donors(&d.donors)).ok() == Some(d.donors.clone())
        && declare::parse_migration(&declare::render_migration(&d.migration)).ok()
            == Some(d.migration.clone());
    v.push(Check::new(
        "declarations.typed_roundtrip",
        fixed,
        "declarations render and re-read identically",
    ));

    v.push(no_findings(
        a,
        "graph.schema",
        &[
            "DUPLICATE_NODE_KEY",
            "RESERVED_NODE_KEY",
            "DANGLING_EDGE",
            "ILLEGAL_EDGE",
            "DEPENDENCY_CYCLE",
            "UNKNOWN_LINEAGE_DONOR",
            "IDENTITY_KIND_CONFLICT",
        ],
    ));
    let shard = &d.repository.shard;
    let registered = crate::protocol::shard(shard).is_some();
    v.push(Check::new(
        "system.shard_registered",
        d.repository.system == crate::schema::SYSTEM && registered,
        format!(
            "system `{}`, shard `{shard}`; the canonical shards are {}",
            d.repository.system,
            crate::protocol::SHARDS
                .iter()
                .map(|s| s.id)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ));
    let ids_ok = a.graph.nodes.values().all(|n| {
        NodeId::of(&n.namespace, &n.semantic_key) == n.id
            && matches!(n.namespace.as_str(), "chronica" | "oss" | "external")
    });
    v.push(Check::new(
        "graph.identity",
        ids_ok,
        "every id = H(chronica | oss | external, semantic_key); no repository in any identity",
    ));
    let bytes = a.graph.encode();
    let rt = Graph::decode(&bytes)
        .map(|g| g.encode() == bytes && g == a.graph)
        .unwrap_or(false);
    v.push(Check::new(
        "graph.roundtrip",
        rt,
        format!(
            "{} nodes, {} edges, {} bytes",
            a.graph.nodes.len(),
            a.graph.edges.len(),
            bytes.len()
        ),
    ));

    v.push(no_findings(
        a,
        "lifecycle.claims_within_evidence",
        &["CLAIM_EXCEEDS_EVIDENCE"],
    ));
    v.push(no_findings(
        a,
        "lifecycle.exceptions_legal",
        &["ILLEGAL_EXCEPTION"],
    ));
    v.push(no_findings(
        a,
        "lifecycle.no_vanished_donor",
        &["DONOR_VANISHED"],
    ));
    v.push(no_findings(
        a,
        "donors.every_external_registered",
        &["UNREGISTERED_EXTERNAL", "DISCOVERED_BUT_ACTIVE"],
    ));
    v.push(no_findings(a, "state.integrity", &["CORRUPT_STATE"]));
    v.push(no_findings(
        a,
        "migration.shims_expire",
        &[
            "SHIM_WITHOUT_VALID_EXPIRY",
            "EXPIRED_SHIM_PRESENT",
            "COMPAT_WITHOUT_EXPIRY",
        ],
    ));
    v.push(no_findings(
        a,
        "migration.waves_consistent",
        &["WAVE_UNKNOWN_NODE", "NODE_IN_TWO_WAVES", "WAVE_NOT_APPLIED"],
    ));

    let values = a.counts.values();
    let names_ok = values.len() == DEFINITIONS.len()
        && values
            .iter()
            .zip(DEFINITIONS)
            .all(|((n, _), d)| n == d.name);
    let ratios_ok = values
        .iter()
        .filter(|(_, v)| v.contains('.'))
        .all(|(_, v)| v.as_str() <= "1.000000" && v.as_str() >= "0.000000");
    let c = &a.counts;
    v.push(Check::new(
        "metrics.schema",
        names_ok
            && ratios_ok
            && c.capabilities_proven <= c.capabilities_native
            && c.capabilities_native <= c.capabilities_total,
        format!("{} metrics in schema order", values.len()),
    ));

    let manifest = std::fs::read_to_string(a.root.join(".ynventa/Cargo.toml")).unwrap_or_default();
    let zero = crate::formats::toml::parse(&manifest)
        .map(|m| {
            ["dependencies", "build-dependencies", "dev-dependencies"]
                .iter()
                .all(|s| {
                    m.get(s)
                        .and_then(|t| t.table())
                        .is_none_or(|t| t.is_empty())
                })
        })
        .unwrap_or(false);
    let self_obs = a
        .census
        .observations
        .iter()
        .filter(|o| o.file.starts_with(".ynventa/"))
        .count();
    v.push(Check::new(
        "subsystem.zero_dependency",
        zero && self_obs == 0,
        format!("manifest dependency-free: {zero}; foreign references from .ynventa: {self_obs}"),
    ));

    v.push(no_findings(
        a,
        "generated.untracked",
        &["GENERATED_TRACKED"],
    ));

    // Determinism: a second, independent assessment of the same universe agrees exactly.
    let again = crate::assess_core(&a.root, a.files.clone(), a.declaration.clone());
    let det = again.counts.values() == crate::count_public(a).values()
        && again.graph.encode() == bytes
        && again.findings == a.findings;
    v.push(Check::new(
        "output.deterministic",
        det,
        "metrics, graph bytes and findings are identical on re-assessment",
    ));

    v.extend(probes());
    v
}

/// In-memory probes of the verifier's semantics, identical everywhere.
pub fn probes() -> Vec<Check> {
    let mut v = Vec::new();
    // Identity survives a move.
    let before = NodeId::of("probe", "storage");
    let mut n = crate::graph::GNode::new(
        "probe",
        "storage",
        crate::schema::NodeKind::Substrate,
        "storage",
    );
    n.path = "storage".into();
    let moved = {
        let mut m = n.clone();
        m.path = "substrate/storage".into();
        m
    };
    v.push(Check::new(
        "probe.identity_survives_moves",
        n.id == before && moved.id == before,
        before.to_string(),
    ));

    // A donor with one surviving build edge is not extinct, whatever else holds.
    let cap = CapabilityVerdict {
        key: "c".into(),
        required: true,
        specified: true,
        replacement_exists: true,
        replacement_canonical: true,
        native: true,
        native_detail: String::new(),
        parity: vec![("t::p".into(), crate::evidence::Verdict::Pass)],
        regression: vec![("t::r".into(), crate::evidence::Verdict::Pass)],
    };
    let clean = gates(&DonorFacts::default(), std::slice::from_ref(&cap), true)
        .iter()
        .all(|g| g.pass);
    let mut dirty = DonorFacts::default();
    dirty.push(crate::census::Observation {
        file: "x/Cargo.toml".into(),
        ecosystem: crate::schema::Ecosystem::Cargo,
        name: "d".into(),
        ident: "d".into(),
        scope: Scope::Build,
        via: crate::census::Via::Manifest,
    });
    let still = gates(&dirty, &[cap], true).iter().all(|g| g.pass);
    v.push(Check::new(
        "probe.no_false_extinction",
        clean && !still,
        "one BUILD edge blocks EXTINCT",
    ));

    // Compaction is order-independent and supersedes.
    let f1 = Fact::new(FactKind::Definition, "", "t", "old", "p1", 1);
    let f2 = Fact::new(FactKind::Definition, "", "t", "new", "p2", 2);
    let x = fold(vec![f1.clone(), f2.clone()]);
    let y = fold(vec![f2, f1]);
    v.push(Check::new(
        "probe.compaction",
        x == y && x.len() == 1 && x[0].value == "new" && x[0].superseded == ["old"],
        "fold is order-independent; newer supersedes",
    ));

    // Path rewriting keeps workspaces resolvable.
    let moves = vec![("storage".to_string(), "substrate/storage".to_string())];
    let out = crate::migration::pathmap::rewrite_paths(
        "core = { path = \"../core\" }",
        "storage",
        "substrate/storage",
        &moves,
    );
    v.push(Check::new(
        "probe.migration_pathmap",
        out.contains("\"../../core\""),
        out,
    ));
    v
}

pub struct Suite {
    pub checks: Vec<Check>,
}

pub fn run(a: &Assessment, reference: Option<&Snapshot>) -> Suite {
    Suite {
        checks: protocol_checks(a, reference),
    }
}

pub fn render_json(a: &Assessment, suite: &Suite) -> Json {
    let checks: Vec<Json> = suite
        .checks
        .iter()
        .map(|c| {
            Json::obj()
                .with("id", c.id)
                .with("pass", c.pass)
                .with("detail", &c.detail)
        })
        .collect();
    let gate: Vec<Json> = a
        .counts
        .v1_gate()
        .into_iter()
        .map(|(k, want, got, pass)| {
            Json::obj()
                .with("metric", k)
                .with("required", want)
                .with("actual", got)
                .with("pass", pass)
        })
        .collect();
    Json::obj()
        .with("schema", schema_identity())
        .with("protocol_version", YNVENTA_PROTOCOL_VERSION as u64)
        .with("system", &a.declaration.repository.system)
        .with("shard", &a.declaration.repository.shard)
        .with("origin", &a.declaration.repository.origin)
        .with("checks", Json::Array(checks))
        .with("metrics", a.counts.to_json())
        .with("v1_gate", Json::Array(gate))
        .with(
            "findings",
            Json::Array(a.findings.iter().map(|f| f.to_json()).collect()),
        )
}

/// Loads a reference snapshot from a path (a snapshot file or a repository root).
pub fn load_reference(p: &Path) -> Option<Snapshot> {
    if p.is_dir() {
        Snapshot::load(p)
    } else {
        Snapshot::parse(&std::fs::read_to_string(p).ok()?)
    }
}
