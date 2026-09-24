//! Census certificate orchestration (ADR 0025): census a root twice (replay), then certify the
//! first pass against the Genome with both pass digests.

pub use atlas_core::certificate::{
    CensusCertificate, CertificateInputs, CertificateState, certify,
};
use atlas_core::{IntegrityDigest, recensus::CensusSnapshot};
use std::{fs, io, path::Path};

pub const GENOME_PATH: &str = ".atlas/genome/atlas.genome.toml";

/// The Genome's `schema` line and the BLAKE3 of its bytes.
pub fn genome_identity(root: &Path) -> io::Result<(String, IntegrityDigest)> {
    let bytes = fs::read(root.join(GENOME_PATH))?;
    let text = String::from_utf8_lossy(&bytes);
    let schema = text
        .lines()
        .find_map(|line| {
            let rest = line.trim().strip_prefix("schema = \"")?;
            Some(rest[..rest.find('"')?].to_owned())
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "genome has no schema line"))?;
    Ok((schema, IntegrityDigest::of_bytes(&bytes)))
}

/// Censuses `root` `passes` times (>= 2 for a replay fixed point) and certifies the first pass.
pub fn certificate(root: impl AsRef<Path>, passes: usize) -> io::Result<CensusCertificate> {
    let root = root.as_ref();
    let (genome_schema, genome_hash) = genome_identity(root)?;
    let first = crate::systemize(root)?;
    let mut digests = vec![CensusSnapshot::from_report(&first).census_digest];
    for _ in 1..passes {
        digests.push(crate::recensus::snapshot(root)?.census_digest);
    }
    Ok(certify(
        &first,
        &CertificateInputs {
            genome_schema: &genome_schema,
            genome_hash,
            pass_digests: &digests,
            atlas_root_hash: None,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn certify_with(report: &atlas_core::SystemizeReport, digests: &[&str]) -> CensusCertificate {
        let (schema, hash) = genome_identity(&root()).unwrap();
        let digests: Vec<String> = digests.iter().map(|d| (*d).to_owned()).collect();
        certify(
            report,
            &CertificateInputs {
                genome_schema: &schema,
                genome_hash: hash,
                pass_digests: &digests,
                atlas_root_hash: None,
            },
        )
    }

    /// The certificate of Atlas's own scope reports exactly what the census knows, including the
    /// gaps: the atlas-cli workspace member outside the inventory, artifacts without a source
    /// frontend, UNKNOWN coverage dimensions, single-engine extraction and no .atlas artifact.
    /// Mutated copies of the same real report prove each condition moves state and blockers.
    #[test]
    fn the_self_scope_certificate_is_honest_and_every_condition_is_load_bearing() {
        let report = crate::systemize(root()).unwrap();
        let pass = CensusSnapshot::from_report(&report).census_digest;
        let cert = certify_with(&report, &[&pass, &pass]);
        assert_eq!(cert.schema, "atlas.census-certificate.v2");
        assert!(cert.inventory.closed);
        assert!(cert.fixed_point.converged && cert.fixed_point.delta_at_exit == 0);
        assert!(cert.dependency.closed);
        // Found by this certificate (G59): obligation facts for artifacts no extractor covers
        // carry no extractor, and ADL-derived facts carry no revision.
        assert!(!cert.normalization.provenance_complete);
        let has = |prefix: &str| cert.blockers.iter().any(|b| b.starts_with(prefix));
        assert!(
            has("WORKSPACE_MEMBER_OUTSIDE_INVENTORY: atlas-cli (apps/cli)"),
            "{:#?}",
            cert.blockers
        );
        assert!(has("ARTIFACTS_WITHOUT_SOURCE_FRONTEND"));
        assert!(has("COVERAGE_UNKNOWN: CALL"));
        assert!(has("MULTI_ENGINE_RECONCILIATION_ABSENT"));
        assert!(has("ATLAS_ROOT_ABSENT"));
        assert_eq!(
            cert.dependency.source_backed_total,
            cert.dependency.source_censused_total + 1,
            "exactly one member (atlas-cli) is outside the census"
        );
        // Provenance is incomplete, so the scope cannot even be RECONCILED; never CLOSED.
        assert_eq!(cert.state, CertificateState::Censused);

        // Replay divergence and a single pass both fail the fixed point.
        let diverged = certify_with(&report, &[&pass, "blake3-256:other"]);
        assert!(!diverged.fixed_point.converged);
        assert!(
            diverged
                .blockers
                .iter()
                .any(|b| b.starts_with("REPLAY_NOT_CONVERGED"))
        );
        assert!(!certify_with(&report, &[&pass]).fixed_point.converged);

        // An unaccounted artifact makes it a DRAFT.
        let mut unaccounted = report.clone();
        unaccounted.census.artifacts_accounted_total -= 1;
        let draft = certify_with(&unaccounted, &[&pass, &pass]);
        assert_eq!(draft.state, CertificateState::Draft);
        assert!(
            draft
                .blockers
                .iter()
                .any(|b| b.starts_with("INVENTORY_NOT_ACCOUNTED"))
        );

        // Provenance accounting is exact: one more fact without a revision is counted.
        let missing = |c: &CensusCertificate| -> usize {
            c.blockers
                .iter()
                .find_map(|b| b.strip_prefix("PROVENANCE_INCOMPLETE: "))
                .and_then(|rest| rest.split(' ').next()?.parse().ok())
                .unwrap_or(0)
        };
        let mut orphan = report.clone();
        let index = orphan
            .census
            .facts
            .iter()
            .position(|f| f.provenance.source_revision.is_some())
            .unwrap();
        orphan.census.facts[index].provenance.source_revision = None;
        let orphaned = certify_with(&orphan, &[&pass, &pass]);
        assert_eq!(missing(&orphaned), missing(&cert) + 1);
        assert!(!orphaned.evidence.provenance_complete);

        assert!(has("UNKNOWN_FACTS"));
        assert!(has("UNSUPPORTED_FACTS"));

        // With provenance repaired the scope reaches RECONCILED -- and still not CLOSED, because
        // every other blocker remains (the state follows the blockers, never the other way).
        let mut repaired = report.clone();
        let revision = repaired.census.facts[index]
            .provenance
            .source_revision
            .clone();
        for fact in &mut repaired.census.facts {
            fact.provenance.source_revision =
                fact.provenance.source_revision.clone().or(revision.clone());
            if fact.provenance.extractor.is_empty() {
                fact.provenance.extractor = "test".into();
            }
        }
        let reconciled = certify_with(&repaired, &[&pass, &pass]);
        assert!(reconciled.normalization.provenance_complete);
        assert_eq!(
            reconciled.state,
            CertificateState::Reconciled,
            "{:#?}",
            reconciled.blockers
        );

        // A dirty input is not a pinned revision.
        let mut dirty = report.clone();
        dirty.snapshot.dirty = true;
        assert!(
            certify_with(&dirty, &[&pass, &pass])
                .blockers
                .iter()
                .any(|b| b.starts_with("REVISION_DIRTY"))
        );

        // The certificate identity is deterministic for pinned input.
        assert_eq!(
            cert.certificate_id,
            certify_with(&report, &[&pass, &pass]).certificate_id
        );
    }

    /// The serialized certificate has exactly the fields `census-certificate.schema.json`
    /// requires and allows (additionalProperties = false at every level).
    #[test]
    fn the_certificate_conforms_to_its_json_schema_field_sets() {
        let schema: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(root().join(".atlas/schemas/census-certificate.schema.json"))
                .unwrap(),
        )
        .unwrap();
        let report = crate::systemize(root()).unwrap();
        let pass = CensusSnapshot::from_report(&report).census_digest;
        let cert = serde_json::to_value(certify_with(&report, &[&pass, &pass])).unwrap();
        let check = |schema: &serde_json::Value, value: &serde_json::Value, at: &str| {
            let properties: std::collections::BTreeSet<&str> = schema["properties"]
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            let required: Vec<&str> = schema["required"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            let present: std::collections::BTreeSet<&str> = value
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            for r in &required {
                assert!(present.contains(r), "{at}: missing required `{r}`");
            }
            for p in &present {
                assert!(
                    properties.contains(p),
                    "{at}: `{p}` not allowed by the schema"
                );
            }
        };
        check(&schema, &cert, "certificate");
        for section in [
            "corpus",
            "genome",
            "inventory",
            "semantic",
            "normalization",
            "reconciliation",
            "fixed_point",
            "independent_passes",
            "evidence",
            "dependency",
        ] {
            check(&schema["properties"][section], &cert[section], section);
        }
        let states: Vec<&str> = schema["properties"]["state"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(states.contains(&cert["state"].as_str().unwrap()));
    }
}
