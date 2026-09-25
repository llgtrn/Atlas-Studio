//! Self-recensus (ADR 0024): the proof of NEXTGEN.
//!
//! `Atlas_N -> census -> change -> Atlas_N+1 -> recensus -> semantic diff -> verdict`.
//!
//! A [`CensusSnapshot`] is a revision-independent projection of one full census: every artifact's
//! content identity and extracted semantics (fact / typed-record / obligation / diagnostic counts
//! by kind and epistemic status, plus a digest of the semantic projection), dependency closure,
//! engineering-graph totals, coverage, ADL state and gates. Record identities embed the revision
//! by design, so the projection keeps only revision-free fields: an unchanged artifact at a new
//! revision has an identical projection. [`prove`] compares two snapshots against declared
//! intentions and decides `PROVEN` or `NOT_PROVEN` -- never by Git diff, always by census state.

pub mod entity;

pub use entity::{Correspondence, EntityChange, EntityState};

use crate::{DocsReport, SystemizeReport, identity::IntegrityDigest};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// v2 (G63): facts from ADL sources are attributed to the ADL surface (`AdlState::sources`)
/// instead of counting as unattributed semantics.
/// v3 (G66): every function carries a revision-stable descriptor (`entities`), and the proof
/// classifies entity correspondence between revisions.
pub const SNAPSHOT_SCHEMA: &str = "atlas.census-snapshot.v3";
/// Facts whose provenance lies under this root are ADL semantics, not inventoried artifacts.
const ADL_ROOT: &str = ".atlas/declared";
pub const RECENSUS_SCHEMA: &str = "atlas.self-recensus-report.v1";

/// One artifact's census state, free of revision-dependent identifiers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactState {
    pub path: String,
    pub disposition: String,
    pub language: Option<String>,
    pub content_digest: Option<String>,
    /// `KIND|STATUS` -> count over the compatibility fact projection.
    pub facts: BTreeMap<String, usize>,
    /// `DIMENSION|STATUS` -> count over typed semantic records.
    pub records: BTreeMap<String, usize>,
    /// `DIMENSION|STATUS` -> count over typed obligations.
    pub obligations: BTreeMap<String, usize>,
    /// Facts whose status is UNKNOWN or UNSUPPORTED (the explicit unknowns of this artifact).
    pub unknowns: usize,
    /// BLAKE3 over the sorted revision-free projection of this artifact's facts and records.
    pub semantic_digest: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyState {
    pub state: String,
    /// `consumer -> provider@version (source) [role]`, sorted.
    pub edges: Vec<String>,
    pub dangling: Vec<String>,
    pub unsupported: Vec<String>,
    pub dynamic_obligations: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdlState {
    /// `Kind:name`, sorted.
    pub nodes: Vec<String>,
    /// `from -relation-> to`, sorted.
    pub edges: Vec<String>,
    pub diagnostics: usize,
    /// constraint name -> verdict.
    pub constraints: BTreeMap<String, String>,
    /// ADL source path (or `.atlas/declared` for compiler-level results) -> BLAKE3 over the
    /// projections of every fact attributed to it (v2). Any change to a source's semantics --
    /// bindings, materializations and spans included -- changes its digest.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sources: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CensusSnapshot {
    pub schema: String,
    /// Recorded for the chain of custody; excluded from `census_digest`.
    pub revision: String,
    pub dirty: bool,
    /// BLAKE3 over the canonical, revision-free serialization of everything below.
    pub census_digest: String,
    pub artifacts: Vec<ArtifactState>,
    pub coverage: BTreeMap<String, String>,
    /// Named totals: facts, typed records, obligations, evidence, diagnostics by code, graph
    /// nodes/edges/bindings, normalization, docs.
    pub totals: BTreeMap<String, usize>,
    pub dependency: DependencyState,
    pub adl: AdlState,
    pub admission_allowed: bool,
    pub admission_blockers: Vec<String>,
    pub docs_gate_ready: bool,
    pub typed_semantics_closed: bool,
    /// Every function/method with its revision-stable descriptor (v3), sorted. Absent in v1/v2
    /// snapshots, which keep their recorded digests.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<EntityState>,
}

/// The v3 entity layer of `report`: one `EntityState` per `FunctionSignature` record.
fn entities_of(report: &SystemizeReport) -> Vec<EntityState> {
    let manifests: BTreeSet<String> = report
        .inventory
        .artifacts
        .iter()
        .filter_map(|a| {
            if a.path == "Cargo.toml" {
                Some(String::new())
            } else {
                a.path.strip_suffix("/Cargo.toml").map(str::to_owned)
            }
        })
        .collect();
    let mut entities: Vec<EntityState> = report
        .census
        .typed_semantic_records
        .iter()
        .filter_map(|record| match record {
            crate::semantic::SemanticObservation::FunctionSignature(header) => {
                Some(&header.subject)
            }
            _ => None,
        })
        .map(|sig| {
            let function = &sig.function;
            let path = &function.span.path;
            let package = entity::package_dir(path, &manifests).unwrap_or("");
            EntityState {
                descriptor: entity::descriptor(
                    package,
                    path,
                    &function.scope.segments,
                    &function.symbol.name,
                ),
                path: path.clone(),
                signature: entity::signature_fingerprint(sig),
                body: sig.body_fingerprint.clone().unwrap_or_else(|| "-".into()),
                visibility: sig.visibility.clone(),
            }
        })
        .collect();
    entities.sort();
    entities
}

fn key(a: &str, b: &str) -> String {
    format!("{a}|{b}")
}

fn bump(map: &mut BTreeMap<String, usize>, key: String) {
    *map.entry(key).or_default() += 1;
}

impl CensusSnapshot {
    /// Projects a full census report into its revision-independent semantic state.
    pub fn from_report(report: &SystemizeReport) -> Self {
        let census = &report.census;
        let mut artifacts: BTreeMap<String, ArtifactState> = report
            .inventory
            .artifacts
            .iter()
            .map(|a| {
                (
                    a.path.clone(),
                    ArtifactState {
                        path: a.path.clone(),
                        disposition: format!("{:?}", a.disposition),
                        language: a.language.clone(),
                        content_digest: a.content_digest.as_ref().map(|d| d.as_str().to_owned()),
                        facts: BTreeMap::new(),
                        records: BTreeMap::new(),
                        obligations: BTreeMap::new(),
                        unknowns: 0,
                        semantic_digest: String::new(),
                    },
                )
            })
            .collect();
        let id_to_path: BTreeMap<&str, &str> = report
            .inventory
            .artifacts
            .iter()
            .map(|a| (a.id.as_str(), a.path.as_str()))
            .collect();
        let mut projections: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut unattributed = 0usize;
        let mut adl_semantics = 0usize;
        let mut adl_projections: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for fact in &census.facts {
            let status = fact.status.as_str();
            let kind = format!("{:?}", fact.kind);
            let path = &fact.provenance.source_path;
            if !artifacts.contains_key(path)
                && (path == ADL_ROOT || path.starts_with(&format!("{ADL_ROOT}/")))
            {
                adl_semantics += 1;
                adl_projections
                    .entry(path.clone())
                    .or_default()
                    .push(format!(
                        "F|{kind}|{status}|{}|{}|{}|{}|{}",
                        fact.subject,
                        fact.predicate,
                        fact.object,
                        fact.provenance.span.as_deref().unwrap_or(""),
                        fact.provenance.extractor
                    ));
                continue;
            }
            match artifacts.get_mut(path) {
                Some(artifact) => {
                    bump(&mut artifact.facts, key(&kind, status));
                    if matches!(status, "UNKNOWN" | "UNSUPPORTED") {
                        artifact.unknowns += 1;
                    }
                    projections.entry(path.clone()).or_default().push(format!(
                        "F|{kind}|{status}|{}|{}|{}|{}",
                        fact.predicate,
                        fact.object,
                        fact.provenance.span.as_deref().unwrap_or(""),
                        fact.provenance.extractor
                    ));
                }
                None => unattributed += 1,
            }
        }
        for record in &census.typed_semantic_records {
            let provenance = record.provenance();
            let dimension = record.dimension().as_str();
            let status = record.status().as_str();
            match artifacts.get_mut(&provenance.source_path) {
                Some(artifact) => {
                    bump(&mut artifact.records, key(dimension, status));
                    projections
                        .entry(provenance.source_path.clone())
                        .or_default()
                        .push(format!(
                            "R|{dimension}|{status}|{}|{}|{}",
                            record.scope().segments.join("::"),
                            provenance.span.as_deref().unwrap_or(""),
                            provenance.extractor
                        ));
                }
                None => unattributed += 1,
            }
        }
        for obligation in &census.typed_obligations {
            let path = id_to_path.get(obligation.artifact.as_str()).copied();
            match path.and_then(|p| artifacts.get_mut(p)) {
                Some(artifact) => bump(
                    &mut artifact.obligations,
                    key(obligation.dimension.as_str(), obligation.status.as_str()),
                ),
                None => unattributed += 1,
            }
        }
        for (path, mut lines) in projections {
            lines.sort();
            if let Some(artifact) = artifacts.get_mut(&path) {
                artifact.semantic_digest = IntegrityDigest::of_bytes(lines.join("\n").as_bytes())
                    .as_str()
                    .to_owned();
            }
        }
        for artifact in artifacts.values_mut() {
            if artifact.semantic_digest.is_empty() {
                artifact.semantic_digest = IntegrityDigest::of_bytes(b"").as_str().to_owned();
            }
        }

        let mut totals = BTreeMap::new();
        totals.insert("artifacts".into(), report.inventory.artifacts_total);
        totals.insert("facts".into(), census.facts_total);
        totals.insert("typed_records".into(), census.typed_semantic_records.len());
        totals.insert("obligations".into(), census.typed_obligations.len());
        totals.insert("evidence".into(), census.evidence.len());
        totals.insert("unattributed_semantics".into(), unattributed);
        totals.insert("adl_semantics".into(), adl_semantics);
        let entities = entities_of(report);
        let mut descriptors = BTreeMap::new();
        for e in &entities {
            *descriptors.entry(e.descriptor.as_str()).or_insert(0usize) += 1;
        }
        totals.insert("entities".into(), entities.len());
        totals.insert(
            "entity_duplicate_descriptors".into(),
            descriptors.values().filter(|n| **n > 1).count(),
        );
        for diagnostic in &census.diagnostics {
            bump(&mut totals, format!("diagnostic:{:?}", diagnostic.code));
        }
        // G120: per-dimension, per-engine record and obligation counts, so a generation's claimed
        // semantic delta is derived from its pre/post snapshots, never hand-maintained.
        for record in &census.typed_semantic_records {
            bump(
                &mut totals,
                format!(
                    "records:{}|{}",
                    record.dimension().as_str(),
                    record.provenance().extractor
                ),
            );
        }
        for obligation in &census.typed_obligations {
            bump(
                &mut totals,
                format!(
                    "obligations:{}|{}|{}",
                    obligation.dimension.as_str(),
                    obligation.extractor.id,
                    obligation.status.as_str()
                ),
            );
        }
        totals.insert("graph_nodes".into(), report.graph.nodes_total);
        totals.insert("graph_edges".into(), report.graph.edges_total);
        totals.insert("graph_bindings".into(), report.graph.bindings_total);
        totals.insert(
            "normalized_facts".into(),
            report.normalization.normalized_facts_total,
        );
        totals.insert(
            "normalization_conflicts".into(),
            report.normalization.conflict_candidates.len(),
        );
        totals.insert("docs".into(), report.docs.documents_total);
        totals.insert("docs_hard_violations".into(), docs_violations(&report.docs));
        let unknown_facts = census
            .facts
            .iter()
            .filter(|f| matches!(f.status.as_str(), "UNKNOWN"))
            .count();
        let unsupported_facts = census
            .facts
            .iter()
            .filter(|f| matches!(f.status.as_str(), "UNSUPPORTED"))
            .count();
        totals.insert("unknown_facts".into(), unknown_facts);
        totals.insert("unsupported_facts".into(), unsupported_facts);
        // G122 (ADR 0044): the composed world model is recensused too, so a change in composed
        // understanding (a new CONFLICT invariant, a dependency verdict, a lost relation) is a
        // census change that must be intended.
        let model = crate::composition::compose(report);
        for (kind, n) in &model.accounting.composed_objects {
            totals.insert(format!("composed:{kind}"), *n);
        }
        for relation in &model.relations {
            bump(
                &mut totals,
                format!("composed:relation:{}", relation.kind.as_str()),
            );
        }
        for invariant in &model.invariants {
            bump(
                &mut totals,
                format!(
                    "composed:invariant:{}|{}",
                    format!("{:?}", invariant.kind).to_uppercase(),
                    invariant.status.as_str()
                ),
            );
        }
        for edge in &model.architecture.dependencies {
            bump(&mut totals, format!("composed:dependency:{}", edge.verdict));
        }
        // G128 (mission M2): how many components have a DECLARED purpose, how many UNKNOWN.
        for component in &model.components {
            bump(
                &mut totals,
                format!("composed:purpose:{}", component.purpose.status.as_str()),
            );
        }
        // G132 (mission M3): the same for functions.
        for function in &model.functions {
            bump(
                &mut totals,
                format!(
                    "composed:function_purpose:{}",
                    function.purpose.status.as_str()
                ),
            );
        }

        let closure = &report.dependency_closure;
        let mut edges: Vec<String> = closure
            .edges
            .iter()
            .map(|e| {
                format!(
                    "{} -> {}@{} ({:?}) [{:?}]",
                    e.consumer, e.provider.name, e.provider.version, e.provider.source_kind, e.role
                )
            })
            .collect();
        edges.sort();
        edges.dedup();
        let mut dangling = closure.dangling_references.clone();
        dangling.sort();
        let mut unsupported: Vec<String> = closure
            .unsupported_constructs
            .iter()
            .map(|u| format!("{u:?}"))
            .collect();
        unsupported.sort();
        let dependency = DependencyState {
            state: format!("{:?}", closure.state),
            edges,
            dangling,
            unsupported,
            dynamic_obligations: closure.dynamic_obligations.len(),
        };

        let declared = &report.adl.ir.declared;
        let mut nodes: Vec<String> = declared
            .nodes
            .iter()
            .map(|n| format!("{}:{}", n.node_kind, n.name))
            .collect();
        nodes.sort();
        let mut adl_edges: Vec<String> = declared
            .edges
            .iter()
            .map(|e| format!("{} -{}-> {}", e.from, e.relation, e.to))
            .collect();
        adl_edges.sort();
        let adl = AdlState {
            nodes,
            edges: adl_edges,
            diagnostics: report.adl.diagnostics.len(),
            constraints: report
                .adl
                .constraint_results
                .iter()
                .map(|c| (c.name.clone(), c.verdict.as_str().to_owned()))
                .collect(),
            sources: adl_projections
                .into_iter()
                .map(|(path, mut lines)| {
                    lines.sort();
                    let digest = IntegrityDigest::of_bytes(lines.join("\n").as_bytes());
                    (path, digest.as_str().to_owned())
                })
                .collect(),
        };
        let mut admission_blockers: Vec<String> = report
            .coding_admission
            .blockers
            .iter()
            .map(|b| format!("{b:?}"))
            .collect();
        admission_blockers.sort();

        let mut snapshot = Self {
            schema: SNAPSHOT_SCHEMA.into(),
            revision: report.snapshot.head_sha.clone(),
            dirty: report.snapshot.dirty,
            census_digest: String::new(),
            artifacts: artifacts.into_values().collect(),
            coverage: census
                .coverage
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().to_owned()))
                .collect(),
            totals,
            dependency,
            adl,
            admission_allowed: report.coding_admission.allowed,
            admission_blockers,
            docs_gate_ready: report.docs.gate_ready,
            typed_semantics_closed: census.typed_semantics_closed(),
            entities,
        };
        snapshot.census_digest = snapshot.compute_digest().as_str().to_owned();
        snapshot
    }

    /// Canonical revision-free serialization (one line per fact of the state, sorted maps).
    pub fn canonical_text(&self) -> String {
        let mut out = vec![format!("schema {}", self.schema)];
        for a in &self.artifacts {
            out.push(format!(
                "artifact {} {} {} {} unknowns={} semantic={}",
                a.path,
                a.disposition,
                a.language.as_deref().unwrap_or("-"),
                a.content_digest.as_deref().unwrap_or("-"),
                a.unknowns,
                a.semantic_digest
            ));
            for (label, map) in [
                ("fact", &a.facts),
                ("record", &a.records),
                ("obligation", &a.obligations),
            ] {
                for (k, v) in map {
                    out.push(format!("  {label} {k} {v}"));
                }
            }
        }
        for (k, v) in &self.coverage {
            out.push(format!("coverage {k} {v}"));
        }
        for (k, v) in &self.totals {
            out.push(format!("total {k} {v}"));
        }
        out.push(format!("dependency state {}", self.dependency.state));
        for e in &self.dependency.edges {
            out.push(format!("dependency edge {e}"));
        }
        for d in &self.dependency.dangling {
            out.push(format!("dependency dangling {d}"));
        }
        for u in &self.dependency.unsupported {
            out.push(format!("dependency unsupported {u}"));
        }
        out.push(format!(
            "dependency dynamic_obligations {}",
            self.dependency.dynamic_obligations
        ));
        for n in &self.adl.nodes {
            out.push(format!("adl node {n}"));
        }
        for e in &self.adl.edges {
            out.push(format!("adl edge {e}"));
        }
        out.push(format!("adl diagnostics {}", self.adl.diagnostics));
        for (k, v) in &self.adl.constraints {
            out.push(format!("adl constraint {k} {v}"));
        }
        // Emitted only when present, so v1 snapshots keep their recorded digests.
        for (path, digest) in &self.adl.sources {
            out.push(format!("adl source {path} {digest}"));
        }
        out.push(format!("admission {}", self.admission_allowed));
        for b in &self.admission_blockers {
            out.push(format!("admission blocker {b}"));
        }
        out.push(format!("docs_gate {}", self.docs_gate_ready));
        out.push(format!(
            "typed_semantics_closed {}",
            self.typed_semantics_closed
        ));
        // Emitted only when present, so v1/v2 snapshots keep their recorded digests.
        for e in &self.entities {
            out.push(format!(
                "entity {} {} sig={} body={} vis={}",
                e.descriptor, e.path, e.signature, e.body, e.visibility
            ));
        }
        out.join("\n")
    }

    pub fn compute_digest(&self) -> IntegrityDigest {
        IntegrityDigest::of_bytes(self.canonical_text().as_bytes())
    }

    /// Whether the recorded digest matches the content (a tampered snapshot is refused).
    pub fn verify_digest(&self) -> bool {
        self.compute_digest().as_str() == self.census_digest
    }

    fn artifact(&self, path: &str) -> Option<&ArtifactState> {
        self.artifacts
            .binary_search_by(|a| a.path.as_str().cmp(path))
            .ok()
            .map(|i| &self.artifacts[i])
    }
}

fn docs_violations(docs: &DocsReport) -> usize {
    docs.hard_violations_total
}

/// What a generation declares it intends to change. Everything the recensus observes must be
/// intended or explicitly accepted with a reason; everything intended must be observed.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecensusIntent {
    pub objective: String,
    /// Artifact paths (exact, or a directory prefix ending in `/`) expected to be created,
    /// modified or deleted, or to change semantically.
    #[serde(default)]
    pub changed_paths: Vec<String>,
    /// Named totals expected to increase (`+name`) or decrease (`-name`).
    #[serde(default)]
    pub total_changes: Vec<String>,
    /// Dependency edges expected to appear (`+edge`) or disappear (`-edge`).
    #[serde(default)]
    pub dependency_changes: Vec<String>,
    /// ADL nodes/edges/constraints expected to appear or disappear (`+item` / `-item`).
    #[serde(default)]
    pub adl_changes: Vec<String>,
    /// Coverage dimensions expected to change, as `DIMENSION=STATUS`.
    #[serde(default)]
    pub coverage_changes: Vec<String>,
    /// Paths whose unknown count may increase, with the reason (`path: reason`).
    #[serde(default)]
    pub explained_unknowns: Vec<String>,
    /// Observed changes accepted although not the objective (`item: reason`).
    #[serde(default)]
    pub accepted_unexpected: Vec<String>,
    /// Entity correspondences expected between revisions (v3): an exact `EntityChange::item`
    /// (`CHANGED core language/adl/census/f().`), or `<KIND|*> <descriptor prefix>*` covering
    /// every such change whose descriptors all start with the prefix.
    #[serde(default)]
    pub entity_changes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryDelta {
    pub created: Vec<String>,
    pub modified: Vec<String>,
    pub deleted: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticDelta {
    /// Artifacts present on both sides whose semantic projection changed.
    pub changed: Vec<String>,
    /// Summed fact/record/obligation count changes by `layer:KIND|STATUS`.
    pub counts: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetDelta {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnknownDelta {
    /// Artifacts whose explicit unknown count increased: `path: before -> after`.
    pub increased: Vec<String>,
    pub decreased: Vec<String>,
    pub total_before: usize,
    pub total_after: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    Proven,
    GenerationNotProven,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelfRecensusReport {
    pub schema: String,
    pub generation: String,
    pub base_revision: String,
    pub candidate_revision: String,
    pub candidate_dirty: bool,
    pub before_census_digest: String,
    pub after_census_digest: String,
    /// The candidate censused twice; both digests must agree.
    pub replay_digest: String,
    pub inventory_delta: InventoryDelta,
    pub semantic_delta: SemanticDelta,
    pub dependency_delta: SetDelta,
    pub dependency_state: (String, String),
    pub graph_delta: BTreeMap<String, i64>,
    pub obligation_delta: BTreeMap<String, i64>,
    pub adl_delta: SetDelta,
    /// Function-level correspondence (v3 on both sides); SAME entities are omitted.
    #[serde(default)]
    pub entity_delta: Vec<EntityChange>,
    pub coverage_delta: Vec<String>,
    pub unknown_delta: UnknownDelta,
    pub intent: RecensusIntent,
    pub intended_changes: Vec<String>,
    pub intended_but_unobserved: Vec<String>,
    pub unexpected_changes: Vec<String>,
    pub regressions: Vec<String>,
    pub verdict: Verdict,
}

fn coverage_rank(status: &str) -> u8 {
    match status {
        "OBSERVED" => 4,
        "DERIVED" | "DECLARED" | "INFERRED" => 3,
        "CONFLICT" => 2,
        "UNKNOWN" | "UNSUPPORTED" | "IGNORED" => 1,
        _ => 0,
    }
}

/// Whether one `entity_changes` declaration covers an observed entity item: exactly, or as
/// `<KIND|*> <descriptor prefix>*` when every descriptor in the item starts with the prefix.
fn entity_declaration_covers(declaration: &str, item: &str) -> bool {
    if declaration == item {
        return true;
    }
    let Some(pattern) = declaration.strip_suffix('*') else {
        return false;
    };
    let (Some((kind, prefix)), Some((item_kind, rest))) =
        (pattern.split_once(' '), item.split_once(' '))
    else {
        return false;
    };
    (kind == "*" || kind == item_kind)
        && rest
            .split(" -> ")
            .flat_map(|side| side.split(" | "))
            .all(|descriptor| descriptor.starts_with(prefix))
}

fn path_intended(intent: &RecensusIntent, path: &str) -> bool {
    intent
        .changed_paths
        .iter()
        .any(|p| p == path || (p.ends_with('/') && path.starts_with(p.as_str())))
}

fn accepted(intent: &RecensusIntent, item: &str) -> bool {
    intent.accepted_unexpected.iter().any(|a| {
        a.split_once(": ").is_some_and(|(k, reason)| {
            !reason.trim().is_empty() && (k == item || (k.ends_with('/') && item.starts_with(k)))
        })
    })
}

fn set_delta(before: &[String], after: &[String]) -> SetDelta {
    let (b, a): (BTreeSet<&String>, BTreeSet<&String>) =
        (before.iter().collect(), after.iter().collect());
    SetDelta {
        added: a.difference(&b).map(|s| (*s).clone()).collect(),
        removed: b.difference(&a).map(|s| (*s).clone()).collect(),
    }
}

fn map_delta(
    before: &BTreeMap<String, usize>,
    after: &BTreeMap<String, usize>,
) -> BTreeMap<String, i64> {
    let keys: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    keys.into_iter()
        .filter_map(|k| {
            let d = *after.get(k).unwrap_or(&0) as i64 - *before.get(k).unwrap_or(&0) as i64;
            (d != 0).then(|| (k.clone(), d))
        })
        .collect()
}

/// Decides a generation from its pre- and post-change self-census (`after` and `replay` are two
/// independent censuses of the candidate). Every observed change must be intended or accepted,
/// every intention observed, and no forbidden regression may remain.
pub fn prove(
    generation: &str,
    before: &CensusSnapshot,
    after: &CensusSnapshot,
    replay: &CensusSnapshot,
    intent: &RecensusIntent,
) -> SelfRecensusReport {
    let mut regressions = Vec::new();
    for (label, snapshot) in [("before", before), ("after", after), ("replay", replay)] {
        if !snapshot.verify_digest() {
            regressions.push(format!(
                "{label} snapshot digest does not match its content"
            ));
        }
    }
    if after.census_digest != replay.census_digest {
        regressions.push(format!(
            "deterministic replay failed: {} != {}",
            after.census_digest, replay.census_digest
        ));
    }

    // Inventory and semantics.
    let before_paths: BTreeSet<&str> = before.artifacts.iter().map(|a| a.path.as_str()).collect();
    let after_paths: BTreeSet<&str> = after.artifacts.iter().map(|a| a.path.as_str()).collect();
    let mut inventory = InventoryDelta::default();
    let mut semantic = SemanticDelta::default();
    for path in after_paths.difference(&before_paths) {
        inventory.created.push((*path).to_owned());
    }
    for path in before_paths.difference(&after_paths) {
        inventory.deleted.push((*path).to_owned());
    }
    let mut unknown = UnknownDelta {
        total_before: before.artifacts.iter().map(|a| a.unknowns).sum(),
        total_after: after.artifacts.iter().map(|a| a.unknowns).sum(),
        ..UnknownDelta::default()
    };
    for path in after_paths.intersection(&before_paths) {
        let (b, a) = (
            before.artifact(path).unwrap(),
            after.artifact(path).unwrap(),
        );
        if b.content_digest != a.content_digest || b.disposition != a.disposition {
            inventory.modified.push((*path).to_owned());
        }
        if b.semantic_digest != a.semantic_digest || b.facts != a.facts || b.records != a.records {
            semantic.changed.push((*path).to_owned());
        }
        if a.unknowns > b.unknowns {
            unknown
                .increased
                .push(format!("{path}: {} -> {}", b.unknowns, a.unknowns));
        } else if a.unknowns < b.unknowns {
            unknown
                .decreased
                .push(format!("{path}: {} -> {}", b.unknowns, a.unknowns));
        }
    }
    for path in &inventory.created {
        if let Some(a) = after.artifact(path)
            && a.unknowns > 0
        {
            unknown
                .increased
                .push(format!("{path}: 0 -> {}", a.unknowns));
        }
    }
    let layer_counts =
        |s: &CensusSnapshot, layer: &str, pick: fn(&ArtifactState) -> &BTreeMap<String, usize>| {
            let mut m = BTreeMap::new();
            for a in &s.artifacts {
                for (k, v) in pick(a) {
                    *m.entry(format!("{layer}:{k}")).or_insert(0) += v;
                }
            }
            m
        };
    for (layer, pick) in [
        (
            "fact",
            (|a: &ArtifactState| &a.facts) as fn(&ArtifactState) -> &BTreeMap<String, usize>,
        ),
        ("record", |a: &ArtifactState| &a.records),
    ] {
        semantic.counts.extend(map_delta(
            &layer_counts(before, layer, pick),
            &layer_counts(after, layer, pick),
        ));
    }
    let obligation_delta = map_delta(
        &layer_counts(before, "obligation", |a| &a.obligations),
        &layer_counts(after, "obligation", |a| &a.obligations),
    );
    let totals_delta = map_delta(&before.totals, &after.totals);
    let graph_delta: BTreeMap<String, i64> = totals_delta
        .iter()
        .filter(|(k, _)| k.starts_with("graph_"))
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    let dependency_delta = set_delta(&before.dependency.edges, &after.dependency.edges);
    let adl_items = |s: &CensusSnapshot| -> Vec<String> {
        s.adl
            .nodes
            .iter()
            .map(|n| format!("node {n}"))
            .chain(s.adl.edges.iter().map(|e| format!("edge {e}")))
            .chain(
                s.adl
                    .constraints
                    .iter()
                    .map(|(k, v)| format!("constraint {k} {v}")),
            )
            .collect()
    };
    let adl_delta = set_delta(&adl_items(before), &adl_items(after));
    // Entity correspondence needs the v3 layer on both sides; a schema change is itself an
    // observed change that must be accepted with a reason.
    let entity_delta = if before.schema == SNAPSHOT_SCHEMA && after.schema == SNAPSHOT_SCHEMA {
        entity::correspond(&before.entities, &after.entities)
    } else {
        Vec::new()
    };
    let dims: BTreeSet<&String> = before
        .coverage
        .keys()
        .chain(after.coverage.keys())
        .collect();
    let mut coverage_delta = Vec::new();
    for dim in dims {
        let (b, a) = (
            before.coverage.get(dim).map_or("ABSENT", String::as_str),
            after.coverage.get(dim).map_or("ABSENT", String::as_str),
        );
        if a != b {
            coverage_delta.push(format!("{dim}: {b} -> {a}"));
            let intended = intent
                .coverage_changes
                .iter()
                .any(|c| c == &format!("{dim}={a}"));
            if coverage_rank(a) < coverage_rank(b) && !intended {
                regressions.push(format!("coverage regression {dim}: {b} -> {a}"));
            }
        }
    }

    // Forbidden regressions of the census itself.
    if before.dependency.state == "Closed" && after.dependency.state != "Closed" {
        regressions.push(format!(
            "dependency closure left CLOSED: {}",
            after.dependency.state
        ));
    }
    let dangling = set_delta(&before.dependency.dangling, &after.dependency.dangling);
    for d in &dangling.added {
        regressions.push(format!("new dangling dependency reference {d}"));
    }
    if before.typed_semantics_closed && !after.typed_semantics_closed {
        regressions.push("typed semantic closure broken".into());
    }
    if before.admission_allowed && !after.admission_allowed {
        regressions.push(format!(
            "coding admission blocked: {:?}",
            after.admission_blockers
        ));
    }
    if before.docs_gate_ready && !after.docs_gate_ready {
        regressions.push("docs gate no longer ready".into());
    }
    if after
        .totals
        .get("unattributed_semantics")
        .copied()
        .unwrap_or(0)
        > before
            .totals
            .get("unattributed_semantics")
            .copied()
            .unwrap_or(0)
    {
        regressions.push("semantics not attributable to an inventoried artifact increased".into());
    }
    for (constraint, verdict) in &after.adl.constraints {
        if verdict == "VIOLATED"
            && before.adl.constraints.get(constraint).map(String::as_str) != Some("VIOLATED")
        {
            regressions.push(format!("ADL constraint {constraint} now VIOLATED"));
        }
    }
    for line in &unknown.increased {
        let path = line.split(": ").next().unwrap_or_default();
        let explained = intent.explained_unknowns.iter().any(|e| {
            e.split_once(": ").is_some_and(|(p, reason)| {
                !reason.trim().is_empty()
                    && (p == path || (p.ends_with('/') && path.starts_with(p)))
            })
        });
        if !explained {
            regressions.push(format!("unexplained new unknowns {line}"));
        }
    }

    // Intended vs observed.
    let mut observed_changes: Vec<String> = Vec::new();
    let changed_paths: BTreeSet<&String> = inventory
        .created
        .iter()
        .chain(&inventory.modified)
        .chain(&inventory.deleted)
        .chain(&semantic.changed)
        .collect();
    for path in &changed_paths {
        observed_changes.push(format!("path {path}"));
    }
    for (k, v) in &totals_delta {
        observed_changes.push(format!("total {}{k}", if *v > 0 { "+" } else { "-" }));
    }
    for e in &dependency_delta.added {
        observed_changes.push(format!("dependency +{e}"));
    }
    for e in &dependency_delta.removed {
        observed_changes.push(format!("dependency -{e}"));
    }
    for e in &adl_delta.added {
        observed_changes.push(format!("adl +{e}"));
    }
    for e in &adl_delta.removed {
        observed_changes.push(format!("adl -{e}"));
    }
    let adl_source_paths: BTreeSet<&String> = before
        .adl
        .sources
        .keys()
        .chain(after.adl.sources.keys())
        .collect();
    for path in adl_source_paths {
        if before.adl.sources.get(path) != after.adl.sources.get(path) {
            observed_changes.push(format!("adl source {path}"));
        }
    }
    for change in &entity_delta {
        observed_changes.push(format!("entity {}", change.item()));
    }
    if before.schema != after.schema {
        observed_changes.push(format!("schema {} -> {}", before.schema, after.schema));
    }
    for c in &coverage_delta {
        observed_changes.push(format!("coverage {c}"));
    }
    let mut intended_changes = Vec::new();
    let mut unexpected_changes = Vec::new();
    for change in &observed_changes {
        let (kind, item) = change.split_once(' ').unwrap_or((change, ""));
        let intended = match kind {
            "path" => path_intended(intent, item),
            "total" => intent.total_changes.iter().any(|t| t == item),
            "dependency" => intent.dependency_changes.iter().any(|d| d == item),
            "adl" => intent.adl_changes.iter().any(|d| d == item),
            "entity" => intent
                .entity_changes
                .iter()
                .any(|d| entity_declaration_covers(d, item)),
            "coverage" => {
                let (dim, transition) = item.split_once(": ").unwrap_or((item, ""));
                let to = transition.rsplit(" -> ").next().unwrap_or("");
                intent
                    .coverage_changes
                    .iter()
                    .any(|c| c == &format!("{dim}={to}"))
            }
            _ => false,
        };
        if intended {
            intended_changes.push(change.clone());
        } else if accepted(intent, change) || accepted(intent, item) {
            intended_changes.push(format!("{change} (accepted)"));
        } else {
            unexpected_changes.push(change.clone());
        }
    }
    let mut intended_but_unobserved = Vec::new();
    for path in &intent.changed_paths {
        let seen = changed_paths
            .iter()
            .any(|p| *p == path || (path.ends_with('/') && p.starts_with(path.as_str())));
        if !seen {
            intended_but_unobserved.push(format!("path {path}"));
        }
    }
    for t in &intent.total_changes {
        if !observed_changes.iter().any(|c| c == &format!("total {t}")) {
            intended_but_unobserved.push(format!("total {t}"));
        }
    }
    for d in &intent.dependency_changes {
        if !observed_changes
            .iter()
            .any(|c| c == &format!("dependency {d}"))
        {
            intended_but_unobserved.push(format!("dependency {d}"));
        }
    }
    for d in &intent.adl_changes {
        if !observed_changes.iter().any(|c| c == &format!("adl {d}")) {
            intended_but_unobserved.push(format!("adl {d}"));
        }
    }
    for d in &intent.entity_changes {
        let seen = observed_changes.iter().any(|c| {
            c.strip_prefix("entity ")
                .is_some_and(|item| entity_declaration_covers(d, item))
        });
        if !seen {
            intended_but_unobserved.push(format!("entity {d}"));
        }
    }
    for c in &intent.coverage_changes {
        let (dim, to) = c.split_once('=').unwrap_or((c, ""));
        if after.coverage.get(dim).map(String::as_str) != Some(to) {
            intended_but_unobserved.push(format!("coverage {c}"));
        }
    }
    let proven = regressions.is_empty()
        && unexpected_changes.is_empty()
        && intended_but_unobserved.is_empty()
        && !intent.objective.trim().is_empty();
    if intent.objective.trim().is_empty() {
        regressions
            .push("no declared objective: a generation must state its bounded change".into());
    }
    SelfRecensusReport {
        schema: RECENSUS_SCHEMA.into(),
        generation: generation.into(),
        base_revision: before.revision.clone(),
        candidate_revision: after.revision.clone(),
        candidate_dirty: after.dirty,
        before_census_digest: before.census_digest.clone(),
        after_census_digest: after.census_digest.clone(),
        replay_digest: replay.census_digest.clone(),
        inventory_delta: inventory,
        semantic_delta: semantic,
        dependency_delta,
        dependency_state: (
            before.dependency.state.clone(),
            after.dependency.state.clone(),
        ),
        graph_delta,
        obligation_delta,
        adl_delta,
        entity_delta,
        coverage_delta,
        unknown_delta: unknown,
        intent: intent.clone(),
        intended_changes,
        intended_but_unobserved,
        unexpected_changes,
        regressions,
        verdict: if proven {
            Verdict::Proven
        } else {
            Verdict::GenerationNotProven
        },
    }
}

#[cfg(test)]
mod tests;
