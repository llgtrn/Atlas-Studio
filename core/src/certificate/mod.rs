//! CensusCertificate v2 (ADR 0025, `contracts/CENSUS-CERTIFICATE.md`,
//! `schemas/census-certificate.schema.json`): the machine-verifiable closure record of one census
//! of one scope. It records whether accounting, semantic coverage, dependency closure,
//! normalization, reconciliation and replay conditions actually hold -- it never manufactures
//! completeness. Every unmet condition is a typed blocker; the state follows from the blockers.

use crate::{EpistemicStatus, SystemizeReport, identity::IntegrityDigest};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const CERTIFICATE_SCHEMA: &str = "atlas.census-certificate.v2";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CertificateState {
    Draft,
    Censused,
    Reconciled,
    Closed,
    Sealed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub corpus_id: String,
    pub root_hash: String,
    pub revisions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Genome {
    pub schema: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    pub artifacts_total: usize,
    pub artifacts_accounted_total: usize,
    pub closed: bool,
    pub dispositions: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub contexts_total: usize,
    pub nodes_total: usize,
    pub edges_total: usize,
    /// Workspace members: source available to Atlas.
    pub source_backed_total: usize,
    /// Source-backed members whose manifest directory is inside the census inventory.
    pub source_censused_total: usize,
    /// Registry packages accounted by name/version/checksum whose source Atlas does not census.
    pub terminal_boundary_total: usize,
    pub unresolved_total: usize,
    pub fixed_point_converged: bool,
    pub closed: bool,
    pub root_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Semantic {
    pub facts_total: usize,
    pub obligations_total: usize,
    pub obligations_accounted_total: usize,
    pub coverage: BTreeMap<String, String>,
    pub unknown_total: usize,
    pub unsupported_total: usize,
    pub ignored_total: usize,
    pub conflict_total: usize,
    pub dynamic_edges_total: usize,
    pub binding_gaps_total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Normalization {
    pub input_records_total: usize,
    pub normalized_records_total: usize,
    pub exact_duplicates_merged: usize,
    pub equivalence_classes_total: usize,
    pub conflict_candidates_total: usize,
    pub provenance_complete: bool,
    pub root_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Reconciliation {
    pub complete: bool,
    pub unresolved_conflicts_total: usize,
}

/// Census passes over the same pinned input; converged when every pass yields the same census
/// digest (deterministic replay).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FixedPoint {
    pub iterations: usize,
    pub converged: bool,
    pub delta_at_exit: usize,
}

/// Independent extraction engines per dimension (not replays of one engine).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IndependentPasses {
    pub passes_total: usize,
    pub agreement: String,
    pub conflicts_total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRoot {
    pub root_hash: String,
    pub provenance_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AtlasRoot {
    pub root_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CensusCertificate {
    pub schema: String,
    pub certificate_id: String,
    pub state: CertificateState,
    pub corpus: Corpus,
    pub genome: Genome,
    pub inventory: Inventory,
    pub dependency: Dependency,
    pub semantic: Semantic,
    pub normalization: Normalization,
    pub reconciliation: Reconciliation,
    pub fixed_point: FixedPoint,
    pub independent_passes: IndependentPasses,
    pub evidence: EvidenceRoot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atlas: Option<AtlasRoot>,
    pub blockers: Vec<String>,
}

/// Everything the certificate is computed from besides the report itself.
#[derive(Debug, Clone)]
pub struct CertificateInputs<'a> {
    pub genome_schema: &'a str,
    /// BLAKE3 of the Genome file bytes.
    pub genome_hash: IntegrityDigest,
    /// Census digests of every pass over the same input (at least one).
    pub pass_digests: &'a [String],
    /// Root hash of a packaged `.atlas` artifact for this census, when one exists.
    pub atlas_root_hash: Option<String>,
}

fn digest_lines(mut lines: Vec<String>) -> String {
    lines.sort();
    IntegrityDigest::of_bytes(lines.join("\n").as_bytes())
        .as_str()
        .to_owned()
}

/// Directory of a manifest path (`apps/cli/Cargo.toml` -> `apps/cli`; `Cargo.toml` -> ``).
fn manifest_dir(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

pub fn certify(report: &SystemizeReport, inputs: &CertificateInputs<'_>) -> CensusCertificate {
    let mut blockers: BTreeSet<String> = BTreeSet::new();
    let census = &report.census;
    let inventory_report = &report.inventory;

    // Corpus identity: inventoried paths and content, pinned to the revision.
    let corpus_root = digest_lines(
        inventory_report
            .artifacts
            .iter()
            .map(|a| {
                format!(
                    "{} {:?} {}",
                    a.path,
                    a.disposition,
                    a.content_digest.as_ref().map_or("-", |d| d.as_str())
                )
            })
            .collect(),
    );
    let repository = report
        .repository
        .manifest
        .as_ref()
        .map(|m| m.repo.clone())
        .unwrap_or_else(|| report.root.clone());
    let mut revisions = BTreeMap::new();
    revisions.insert(repository.clone(), report.snapshot.head_sha.clone());
    if report.snapshot.dirty {
        blockers.insert("REVISION_DIRTY: the census input is not a pinned commit".into());
    }

    // Inventory accounting.
    let accounted = census.artifacts_accounted_total;
    let inventory = Inventory {
        artifacts_total: inventory_report.artifacts_total,
        artifacts_accounted_total: accounted,
        closed: accounted == inventory_report.artifacts_total,
        dispositions: inventory_report.dispositions.clone(),
    };
    if !inventory.closed {
        blockers.insert(format!(
            "INVENTORY_NOT_ACCOUNTED: {accounted} of {}",
            inventory.artifacts_total
        ));
    }
    if let Some(unknown) = inventory.dispositions.get("UNKNOWN").filter(|n| **n > 0) {
        blockers.insert(format!("ARTIFACTS_WITHOUT_SOURCE_FRONTEND: {unknown}"));
    }

    // Dependency accounting, reconciled against the inventory: a workspace member whose manifest
    // directory contains no inventoried artifact is source Atlas has but does not census.
    let closure = &report.dependency_closure;
    let inventoried: Vec<&str> = inventory_report
        .artifacts
        .iter()
        .map(|a| a.path.as_str())
        .collect();
    let mut members: BTreeMap<String, String> = BTreeMap::new();
    for edge in &closure.edges {
        if members.contains_key(&edge.consumer) {
            continue;
        }
        if closure.edges.iter().any(|e| {
            e.provider.name == edge.consumer
                && format!("{:?}", e.provider.source_kind) == "WorkspaceMember"
        }) || edge.evidence_path.ends_with("Cargo.toml") && !edge.evidence_path.is_empty()
        {
            members.insert(
                edge.consumer.clone(),
                manifest_dir(&edge.evidence_path).to_owned(),
            );
        }
    }
    let mut censused_members = 0;
    for (member, dir) in &members {
        let covered = inventoried
            .iter()
            .any(|p| dir.is_empty() || p.starts_with(&format!("{dir}/")));
        if covered {
            censused_members += 1;
        } else {
            blockers.insert(format!(
                "WORKSPACE_MEMBER_OUTSIDE_INVENTORY: {member} ({dir})"
            ));
        }
    }
    let instances: BTreeSet<String> = closure
        .edges
        .iter()
        .map(|e| format!("{}@{}", e.provider.name, e.provider.version))
        .chain(members.keys().cloned())
        .collect();
    let registry: BTreeSet<String> = closure
        .edges
        .iter()
        .filter(|e| format!("{:?}", e.provider.source_kind) == "Registry")
        .map(|e| format!("{}@{}", e.provider.name, e.provider.version))
        .collect();
    let closed = format!("{:?}", closure.state) == "Closed";
    let dependency = Dependency {
        contexts_total: 1,
        nodes_total: instances.len(),
        edges_total: closure.edges_total,
        source_backed_total: members.len(),
        source_censused_total: censused_members,
        terminal_boundary_total: registry.len(),
        unresolved_total: closure.dangling_references.len(),
        fixed_point_converged: closed,
        closed,
        root_hash: Some(digest_lines(
            closure
                .edges
                .iter()
                .map(|e| {
                    format!(
                        "{} -> {}@{} {:?} {:?}",
                        e.consumer,
                        e.provider.name,
                        e.provider.version,
                        e.provider.source_kind,
                        e.role
                    )
                })
                .collect(),
        )),
    };
    if !closed {
        blockers.insert(format!(
            "DEPENDENCY_CLOSURE_NOT_CLOSED: {:?}",
            closure.state
        ));
    }
    if dependency.unresolved_total > 0 {
        blockers.insert(format!(
            "DEPENDENCY_UNRESOLVED: {}",
            dependency.unresolved_total
        ));
    }

    // Semantic accounting.
    let count_status = |status: &str| {
        census
            .facts
            .iter()
            .filter(|f| f.status.as_str() == status)
            .count()
    };
    let semantic = Semantic {
        facts_total: census.facts_total,
        obligations_total: census.typed_obligations.len(),
        obligations_accounted_total: if census.typed_semantics_closed() {
            census.typed_obligations.len()
        } else {
            census
                .typed_closure
                .typed_obligations_total
                .min(census.typed_obligations.len())
        },
        coverage: census
            .coverage
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().to_owned()))
            .collect(),
        unknown_total: count_status("UNKNOWN"),
        unsupported_total: count_status("UNSUPPORTED"),
        ignored_total: count_status("IGNORED"),
        conflict_total: count_status("CONFLICT"),
        dynamic_edges_total: closure.dynamic_obligations.len(),
        binding_gaps_total: report.adl.deltas.len(),
    };
    if semantic.obligations_accounted_total != semantic.obligations_total {
        blockers.insert("SEMANTIC_OBLIGATIONS_NOT_ACCOUNTED".into());
    }
    for (dimension, status) in &semantic.coverage {
        if matches!(status.as_str(), "UNKNOWN" | "UNSUPPORTED" | "CONFLICT") {
            blockers.insert(format!("COVERAGE_{status}: {dimension}"));
        }
    }
    // No scope policy permits unknowns in the Atlas self-scope (Genome:
    // critical_scope_unknown_zero_when_policy_requires); they stay explicit and blocking.
    if semantic.unknown_total > 0 {
        blockers.insert(format!("UNKNOWN_FACTS: {}", semantic.unknown_total));
    }
    if semantic.unsupported_total > 0 {
        blockers.insert(format!("UNSUPPORTED_FACTS: {}", semantic.unsupported_total));
    }
    if semantic.dynamic_edges_total > 0 {
        blockers.insert(format!(
            "DYNAMIC_DEPENDENCY_OBLIGATIONS: {}",
            semantic.dynamic_edges_total
        ));
    }

    // Normalization and provenance.
    let normalization_report = &report.normalization;
    let missing_revision = census
        .facts
        .iter()
        .filter(|f| f.provenance.source_revision.is_none())
        .count();
    let missing_extractor = census
        .facts
        .iter()
        .filter(|f| f.provenance.extractor.is_empty())
        .count();
    let missing_path = census
        .facts
        .iter()
        .filter(|f| f.provenance.source_path.is_empty())
        .count();
    let provenance_complete = missing_revision == 0 && missing_extractor == 0 && missing_path == 0;
    let normalization = Normalization {
        input_records_total: normalization_report.input_facts_total,
        normalized_records_total: normalization_report.normalized_facts_total,
        exact_duplicates_merged: normalization_report.exact_duplicates_merged,
        equivalence_classes_total: normalization_report.normalized_facts_total,
        conflict_candidates_total: normalization_report.conflict_candidates.len(),
        provenance_complete,
        root_hash: Some(digest_lines(
            normalization_report
                .facts
                .iter()
                .map(|f| {
                    format!(
                        "{:?}|{}|{}|{}|{}|{}",
                        f.kind,
                        f.status.as_str(),
                        f.predicate,
                        f.object,
                        f.provenance.source_path,
                        f.provenance.span.as_deref().unwrap_or("")
                    )
                })
                .collect(),
        )),
    };
    if !provenance_complete {
        blockers.insert(format!(
            "PROVENANCE_INCOMPLETE: {missing_revision} facts without revision, {missing_extractor} without extractor, {missing_path} without source path"
        ));
    }
    if normalization.conflict_candidates_total > 0 {
        blockers.insert(format!(
            "NORMALIZATION_CONFLICTS: {}",
            normalization.conflict_candidates_total
        ));
    }
    let reconciliation = Reconciliation {
        complete: normalization.conflict_candidates_total == 0 && semantic.conflict_total == 0,
        unresolved_conflicts_total: normalization.conflict_candidates_total
            + semantic.conflict_total,
    };

    // Replay fixed point.
    let distinct: BTreeSet<&String> = inputs.pass_digests.iter().collect();
    let fixed_point = FixedPoint {
        iterations: inputs.pass_digests.len(),
        converged: inputs.pass_digests.len() >= 2 && distinct.len() == 1,
        delta_at_exit: distinct.len().saturating_sub(1),
    };
    if !fixed_point.converged {
        blockers.insert(format!(
            "REPLAY_NOT_CONVERGED: {} passes, {} distinct digests",
            inputs.pass_digests.len(),
            distinct.len()
        ));
    }

    // Independent engines: one extractor per dimension today.
    // Independence is per (artifact, dimension): distinct extractors that evaluated the same
    // obligation. Different extractors on different artifacts are not independent passes, and an
    // UNSUPPORTED obligation was not evaluated at all -- whoever asserted it is not an engine.
    let mut engines: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for obligation in &census.typed_obligations {
        if obligation.status == EpistemicStatus::Unsupported {
            continue;
        }
        engines
            .entry((
                obligation.artifact.as_str().to_owned(),
                obligation.dimension.as_str().to_owned(),
            ))
            .or_default()
            .insert(obligation.extractor.id.clone());
    }
    let max_engines = engines.values().map(BTreeSet::len).max().unwrap_or(0);
    let independent_passes = IndependentPasses {
        passes_total: max_engines,
        agreement: if max_engines >= 2 {
            "PARTIAL".into()
        } else {
            "NOT_APPLICABLE".into()
        },
        conflicts_total: 0,
    };
    if max_engines < 2 {
        blockers.insert(
            "MULTI_ENGINE_RECONCILIATION_ABSENT: one extractor per dimension (Genome: multi_engine_reconciliation_required)"
                .into(),
        );
    }

    let evidence = EvidenceRoot {
        root_hash: digest_lines(
            census
                .evidence
                .iter()
                .map(|e| e.id.as_str().to_owned())
                .collect(),
        ),
        provenance_complete,
    };
    let atlas = inputs.atlas_root_hash.clone().map(|root_hash| AtlasRoot {
        root_hash: Some(root_hash),
    });
    if atlas.is_none() {
        blockers.insert("ATLAS_ROOT_ABSENT: no packaged .atlas artifact for this census".into());
    }

    let blockers: Vec<String> = blockers.into_iter().collect();
    let only_atlas_root = blockers.iter().all(|b| b.starts_with("ATLAS_ROOT_ABSENT"));
    let state = if !inventory.closed {
        CertificateState::Draft
    } else if !(reconciliation.complete && provenance_complete && fixed_point.converged) {
        CertificateState::Censused
    } else if !(only_atlas_root) {
        CertificateState::Reconciled
    } else if atlas.is_none() {
        CertificateState::Closed
    } else {
        CertificateState::Sealed
    };
    let certificate_id = format!(
        "census-certificate:{}",
        &IntegrityDigest::of_bytes(
            format!(
                "{corpus_root}|{:?}|{}|{}",
                revisions,
                inputs.genome_hash.as_str(),
                CERTIFICATE_SCHEMA
            )
            .as_bytes()
        )
        .as_str()["blake3-256:".len()..]
    );
    CensusCertificate {
        schema: CERTIFICATE_SCHEMA.into(),
        certificate_id,
        state,
        corpus: Corpus {
            corpus_id: repository,
            root_hash: corpus_root,
            revisions,
        },
        genome: Genome {
            schema: inputs.genome_schema.into(),
            hash: inputs.genome_hash.as_str().to_owned(),
        },
        inventory,
        dependency,
        semantic,
        normalization,
        reconciliation,
        fixed_point,
        independent_passes,
        evidence,
        atlas,
        blockers,
    }
}
