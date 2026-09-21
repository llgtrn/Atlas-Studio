//! Runtime census stage.
//!
//! Census converts admitted inventory and declared ADL into typed semantic facts while preserving
//! provenance and explicit unsupported/unknown states. It does not grant truth to model output.

use atlas_core::{
    AdlCompileReport, ArtifactDisposition, CensusReport, EpistemicStatus, InventoryReport,
    Provenance, RevisionRef, SemanticFact, SemanticFactKind, SourceReport, stable_id,
};
use std::{collections::BTreeMap, path::Path};

fn fact_id(seed: &str) -> String {
    stable_id("semantic-fact", seed)
}

fn source_provenance(path: &str, revision: Option<&RevisionRef>) -> Provenance {
    let extractor = adapter::resolve_source_frontend(Path::new(path))
        .map(|matched| matched.frontend_id.to_owned())
        .unwrap_or_else(|| "atlas.inventory.v1".to_owned());
    Provenance {
        source_path: path.to_owned(),
        source_revision: revision.cloned(),
        extractor,
        content_hash: None,
        span: None,
    }
}

fn adl_provenance(
    path: &str,
    line: usize,
    column: usize,
    revision: Option<&RevisionRef>,
) -> Provenance {
    Provenance {
        source_path: path.to_owned(),
        source_revision: revision.cloned(),
        extractor: "atlas.adl.compiler.v1".into(),
        content_hash: None,
        span: Some(format!("{line}:{column}")),
    }
}

fn disposition_status(disposition: &ArtifactDisposition) -> EpistemicStatus {
    match disposition {
        ArtifactDisposition::Parsed
        | ArtifactDisposition::BinaryDescribed
        | ArtifactDisposition::Generated
        | ArtifactDisposition::ExternalizedWithEvidence => EpistemicStatus::Observed,
        ArtifactDisposition::IgnoredByExplicitPolicy => EpistemicStatus::Ignored,
        ArtifactDisposition::Unsupported => EpistemicStatus::Unsupported,
        ArtifactDisposition::Unknown => EpistemicStatus::Unknown,
    }
}

pub fn build_census(
    inventory: &InventoryReport,
    source: &SourceReport,
    adl: &AdlCompileReport,
    revision: Option<&RevisionRef>,
) -> CensusReport {
    let mut facts = Vec::new();

    for artifact in &inventory.artifacts {
        let subject = artifact.id.as_str().to_owned();
        facts.push(SemanticFact {
            id: fact_id(&format!("artifact:{}:disposition", artifact.path)),
            kind: SemanticFactKind::SourceArtifact,
            status: disposition_status(&artifact.disposition),
            subject: subject.clone(),
            predicate: "disposition".into(),
            object: artifact.disposition.as_str().into(),
            provenance: source_provenance(&artifact.path, revision),
        });

        if let Some(language) = &artifact.language {
            facts.push(SemanticFact {
                id: fact_id(&format!("artifact:{}:language:{language}", artifact.path)),
                kind: SemanticFactKind::SourceArtifact,
                status: EpistemicStatus::Observed,
                subject,
                predicate: "language".into(),
                object: language.clone(),
                provenance: source_provenance(&artifact.path, revision),
            });
        }
    }

    for node in &adl.ir.declared.nodes {
        facts.push(SemanticFact {
            id: fact_id(&format!("declared-node:{}:{}", node.id, node.node_kind)),
            kind: SemanticFactKind::DeclaredNode,
            status: EpistemicStatus::Declared,
            subject: node.name.clone(),
            predicate: "node_kind".into(),
            object: node.node_kind.clone(),
            provenance: adl_provenance(&node.span.path, node.span.line, node.span.column, revision),
        });
    }

    for edge in &adl.ir.declared.edges {
        facts.push(SemanticFact {
            id: fact_id(&format!(
                "declared-edge:{}:{}:{}",
                edge.from, edge.relation, edge.to
            )),
            kind: SemanticFactKind::DeclaredEdge,
            status: EpistemicStatus::Declared,
            subject: edge.from.clone(),
            predicate: edge.relation.clone(),
            object: edge.to.clone(),
            provenance: adl_provenance(&edge.span.path, edge.span.line, edge.span.column, revision),
        });
    }

    for binding in &adl.ir.declared.bindings {
        facts.push(SemanticFact {
            id: fact_id(&format!(
                "binding:{}:{}:{}",
                binding.consumer, binding.capability, binding.provider
            )),
            kind: SemanticFactKind::Binding,
            status: EpistemicStatus::Declared,
            subject: binding.consumer.clone(),
            predicate: "binds_to".into(),
            object: binding.provider.clone(),
            provenance: adl_provenance(
                &binding.span.path,
                binding.span.line,
                binding.span.column,
                revision,
            ),
        });
        facts.push(SemanticFact {
            id: fact_id(&format!("binding:{}:capability", binding.name)),
            kind: SemanticFactKind::Binding,
            status: EpistemicStatus::Declared,
            subject: binding.name.clone(),
            predicate: "capability".into(),
            object: binding.capability.clone(),
            provenance: adl_provenance(
                &binding.span.path,
                binding.span.line,
                binding.span.column,
                revision,
            ),
        });
    }

    for constraint in &adl.ir.declared.constraints {
        facts.push(SemanticFact {
            id: fact_id(&format!("constraint:{}", constraint.name)),
            kind: SemanticFactKind::Constraint,
            status: EpistemicStatus::Declared,
            subject: constraint.name.clone(),
            predicate: "declared".into(),
            object: "constraint".into(),
            provenance: adl_provenance(
                &constraint.span.path,
                constraint.span.line,
                constraint.span.column,
                revision,
            ),
        });
    }

    for invariant in &adl.ir.declared.invariants {
        facts.push(SemanticFact {
            id: fact_id(&format!("invariant:{}", invariant.name)),
            kind: SemanticFactKind::Invariant,
            status: EpistemicStatus::Declared,
            subject: invariant.name.clone(),
            predicate: "declared".into(),
            object: "invariant".into(),
            provenance: adl_provenance(
                &invariant.span.path,
                invariant.span.line,
                invariant.span.column,
                revision,
            ),
        });
    }

    for result in &adl.constraint_results {
        facts.push(SemanticFact {
            id: fact_id(&format!("constraint-result:{}:{}", result.name, result.passed)),
            kind: SemanticFactKind::ConstraintResult,
            status: EpistemicStatus::Derived,
            subject: result.name.clone(),
            predicate: "passed".into(),
            object: result.passed.to_string(),
            provenance: Provenance {
                source_path: ".atlas/declared".into(),
                source_revision: revision.cloned(),
                extractor: "atlas.adl.constraint-evaluator.v1".into(),
                content_hash: None,
                span: None,
            },
        });
    }

    for delta in &adl.deltas {
        facts.push(SemanticFact {
            id: fact_id(&format!("diagnostic:{}:{}", delta.code, delta.subject)),
            kind: SemanticFactKind::Diagnostic,
            status: EpistemicStatus::Derived,
            subject: delta.subject.clone(),
            predicate: delta.code.clone(),
            object: delta.message.clone(),
            provenance: Provenance {
                source_path: ".atlas/declared".into(),
                source_revision: revision.cloned(),
                extractor: "atlas.adl.delta.v1".into(),
                content_hash: None,
                span: None,
            },
        });
    }

    for transform in &adl.ir.declared.transforms {
        facts.push(SemanticFact {
            id: fact_id(&format!("transform:{}", transform.name)),
            kind: SemanticFactKind::Transform,
            status: EpistemicStatus::Declared,
            subject: transform.name.clone(),
            predicate: "declared".into(),
            object: "transform".into(),
            provenance: adl_provenance(
                &transform.span.path,
                transform.span.line,
                transform.span.column,
                revision,
            ),
        });
    }

    for materialization in &adl.ir.declared.materializations {
        facts.push(SemanticFact {
            id: fact_id(&format!(
                "materialization:{}:{}",
                materialization.target, materialization.path
            )),
            kind: SemanticFactKind::Materialization,
            status: EpistemicStatus::Declared,
            subject: materialization.target.clone(),
            predicate: "materializes_at".into(),
            object: materialization.path.clone(),
            provenance: adl_provenance(
                &materialization.span.path,
                materialization.span.line,
                materialization.span.column,
                revision,
            ),
        });
    }

    let parsed_artifacts = source.files_total;
    let mut coverage = BTreeMap::from([
        ("SOURCE_ARTIFACT".into(), "OBSERVED".into()),
        ("DECLARED_ADL".into(), "DECLARED".into()),
        ("SYMBOL".into(), "UNSUPPORTED".into()),
        ("TYPE".into(), "UNSUPPORTED".into()),
        ("CALL".into(), "UNSUPPORTED".into()),
        ("CONTROL_FLOW".into(), "UNSUPPORTED".into()),
        ("DATA_FLOW".into(), "UNSUPPORTED".into()),
        ("BUILD".into(), "UNSUPPORTED".into()),
        ("STATE".into(), "UNSUPPORTED".into()),
        ("EFFECT".into(), "UNSUPPORTED".into()),
    ]);
    if parsed_artifacts == 0 {
        coverage.insert("SOURCE_ARTIFACT".into(), "UNKNOWN".into());
    }

    facts.sort_by(|a, b| {
        a.provenance
            .source_path
            .cmp(&b.provenance.source_path)
            .then_with(|| a.id.cmp(&b.id))
    });

    CensusReport {
        schema: "atlas.census-report.v1".into(),
        artifacts_total: inventory.artifacts_total,
        artifacts_accounted_total: inventory.artifacts.len(),
        facts_total: facts.len(),
        coverage,
        facts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::{
        ArtifactId, ArtifactKind, ArtifactRecord, FileFact, InventoryReport, SourceReport,
        compile_adl,
    };

    #[test]
    fn census_accounts_for_every_inventory_artifact() {
        let inventory = InventoryReport::new(
            "/repo",
            vec![
                ArtifactRecord {
                    id: ArtifactId::new("artifact:rs"),
                    path: "src/lib.rs".into(),
                    kind: ArtifactKind::File,
                    bytes: 10,
                    disposition: ArtifactDisposition::Parsed,
                    language: Some("rust".into()),
                    reason: None,
                },
                ArtifactRecord {
                    id: ArtifactId::new("artifact:unknown"),
                    path: "src/data.xyz".into(),
                    kind: ArtifactKind::File,
                    bytes: 3,
                    disposition: ArtifactDisposition::Unknown,
                    language: None,
                    reason: Some("no-registered-source-frontend".into()),
                },
            ],
        );
        let source = SourceReport {
            schema: "test".into(),
            root: "/repo".into(),
            files_total: 1,
            languages: BTreeMap::from([("rust".into(), 1)]),
            files: vec![FileFact {
                path: "src/lib.rs".into(),
                language: "rust".into(),
                bytes: 10,
            }],
        };
        let adl = compile_adl(&[], &source);
        let report = build_census(&inventory, &source, &adl, None);

        assert!(report.is_closed());
        assert_eq!(report.artifacts_accounted_total, 2);
        assert_eq!(report.coverage.get("SYMBOL").map(String::as_str), Some("UNSUPPORTED"));
        assert!(report.facts.iter().any(|fact| fact.status == EpistemicStatus::Unknown));
    }
}
