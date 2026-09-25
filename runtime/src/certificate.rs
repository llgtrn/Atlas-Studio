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
            atlas_root_sealed: false,
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
                atlas_root_sealed: false,
            },
        )
    }

    /// The certificate of Atlas's own scope reports exactly what the census knows, including the
    /// gaps that keep it RECONCILED rather than CLOSED: UNKNOWN coverage dimensions, single-engine
    /// extraction and no .atlas artifact.
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
        // Closed in G62 (found by this certificate in G59): census-level and ADL-derived facts
        // carry the census revision, and unsupported-language obligations name the dispatch that
        // asserted them.
        assert!(cert.normalization.provenance_complete);
        let revision = report.snapshot.revision();
        assert!(
            report
                .census
                .facts
                .iter()
                .all(|f| f.provenance.source_revision.as_ref() == Some(&revision)),
            "every fact carries the census's own revision, not merely some revision"
        );
        let has = |prefix: &str| cert.blockers.iter().any(|b| b.starts_with(prefix));
        // Closed in G60: every workspace member is inside the census scope.
        assert!(
            !has("WORKSPACE_MEMBER_OUTSIDE_INVENTORY"),
            "{:#?}",
            cert.blockers
        );
        // Detection still works: drop the CLI's artifacts from the inventory and it reappears.
        let mut without_cli = report.clone();
        without_cli
            .inventory
            .artifacts
            .retain(|a| !a.path.starts_with("apps/cli/"));
        let reopened = certify_with(&without_cli, &[&pass, &pass]);
        assert!(
            reopened
                .blockers
                .iter()
                .any(|b| b == "WORKSPACE_MEMBER_OUTSIDE_INVENTORY: atlas-cli (apps/cli)"),
            "{:#?}",
            reopened.blockers
        );
        // Closed in G61: html/css fixtures have registered frontends and the BLAKE3 vector files
        // are classified by their observed `include!` call.
        assert!(
            !has("ARTIFACTS_WITHOUT_SOURCE_FRONTEND"),
            "{:#?}",
            cert.blockers
        );
        for fragment in [
            "core/src/identity/blake3_vectors.in",
            "core/src/identity/blake3_xof_vectors.in",
            "core/src/identity/blake3_high_counter.in",
        ] {
            let artifact = report
                .inventory
                .artifacts
                .iter()
                .find(|a| a.path == fragment)
                .unwrap();
            assert_eq!(artifact.language.as_deref(), Some("rust-include-fragment"));
            // Census provenance names the frontend that classified it, recovered from the
            // recorded language because the path alone resolves none.
            let disposition = report
                .census
                .facts
                .iter()
                .find(|f| f.provenance.source_path == fragment && f.object == "PARSED")
                .unwrap();
            assert_eq!(
                disposition.provenance.extractor,
                "atlas.source.rust-include-fragment.bootstrap.v1"
            );
        }
        // Detection still works: an artifact with no frontend reopens it, counted exactly.
        let mut unrecognized = report.clone();
        let css = unrecognized
            .inventory
            .artifacts
            .iter_mut()
            .find(|a| a.path.ends_with("hermetic-probe.css"))
            .unwrap();
        css.disposition = atlas_core::ArtifactDisposition::Unknown;
        css.language = None;
        *unrecognized
            .inventory
            .dispositions
            .get_mut("PARSED")
            .unwrap() -= 1;
        unrecognized
            .inventory
            .dispositions
            .insert("UNKNOWN".into(), 1);
        assert!(
            certify_with(&unrecognized, &[&pass, &pass])
                .blockers
                .iter()
                .any(|b| b == "ARTIFACTS_WITHOUT_SOURCE_FRONTEND: 1"),
        );
        assert!(has("COVERAGE_UNKNOWN: CALL"));
        assert!(has("MULTI_ENGINE_RECONCILIATION_ABSENT"));
        assert!(has("ATLAS_ROOT_ABSENT"));
        assert_eq!(
            cert.dependency.source_backed_total, cert.dependency.source_censused_total,
            "every source-backed workspace member is censused (G60)"
        );
        assert_eq!(
            reopened.dependency.source_censused_total + 1,
            reopened.dependency.source_backed_total
        );
        // Inventory, provenance, reconciliation and replay hold, so the scope is RECONCILED (G62);
        // never CLOSED while any other blocker remains (the state follows the blockers).
        assert!(!has("PROVENANCE_INCOMPLETE"), "{:#?}", cert.blockers);
        assert_eq!(
            cert.state,
            CertificateState::Reconciled,
            "{:#?}",
            cert.blockers
        );

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

        // Provenance accounting is exact and load-bearing: one fact without a revision, or one
        // without an extractor, is counted and drops the scope back to CENSUSED.
        let mut orphan = report.clone();
        orphan.census.facts[0].provenance.source_revision = None;
        let orphaned = certify_with(&orphan, &[&pass, &pass]);
        assert!(
            orphaned
                .blockers
                .iter()
                .any(|b| b.starts_with("PROVENANCE_INCOMPLETE: 1 facts without revision, 0 ")),
            "{:#?}",
            orphaned.blockers
        );
        assert!(!orphaned.evidence.provenance_complete);
        assert_eq!(orphaned.state, CertificateState::Censused);
        let mut anonymous = report.clone();
        let last = anonymous.census.facts.len() - 1;
        anonymous.census.facts[last].provenance.extractor.clear();
        let anonymized = certify_with(&anonymous, &[&pass, &pass]);
        assert!(
            anonymized
                .blockers
                .iter()
                .any(|b| b.ends_with(" 1 without extractor, 0 without source path")),
            "{:#?}",
            anonymized.blockers
        );
        assert_eq!(anonymized.state, CertificateState::Censused);

        assert!(has("UNKNOWN_FACTS"));
        assert!(has("UNSUPPORTED_FACTS"));

        // An engine is an extractor that evaluated the obligation, and reconciliation is per
        // dimension (G75): CALL has two engines on this repository (the syntactic extractor and
        // path resolution), every other evaluated dimension one, and the blocker names them.
        let multi = |cert: &CensusCertificate| {
            cert.blockers
                .iter()
                .find(|b| b.starts_with("MULTI_ENGINE_RECONCILIATION_ABSENT"))
                .cloned()
        };
        assert_eq!(cert.independent_passes.passes_total, 2);
        let named = multi(&cert).unwrap();
        assert!(!named.contains("CALL"), "{named}");
        assert!(
            named.contains("SYMBOL") && named.contains("DATA_FLOW"),
            "{named}"
        );
        // A second identity that only asserts UNSUPPORTED is not an engine; one that evaluated
        // the obligation reconciles that dimension, and only that one.
        let evaluated = report
            .census
            .typed_obligations
            .iter()
            .find(|o| {
                o.status != atlas_core::EpistemicStatus::Unsupported
                    && o.dimension != atlas_core::SemanticDimension::Call
            })
            .unwrap()
            .clone();
        let dimension = evaluated.dimension.as_str();
        let mut asserted_only = report.clone();
        let mut second = evaluated.clone();
        second.extractor.id = "atlas.test.second-engine".into();
        second.status = atlas_core::EpistemicStatus::Unsupported;
        asserted_only.census.typed_obligations.push(second.clone());
        assert!(
            multi(&certify_with(&asserted_only, &[&pass, &pass]))
                .unwrap()
                .contains(dimension)
        );
        let mut two_engines = report.clone();
        second.status = evaluated.status;
        two_engines.census.typed_obligations.push(second);
        let doubled = certify_with(&two_engines, &[&pass, &pass]);
        let after = multi(&doubled).unwrap();
        assert!(!after.contains(dimension), "{after}");
        // Two engines on every evaluated (artifact, dimension): no blocker at all.
        let mut everywhere = report.clone();
        let seconds: Vec<_> = everywhere
            .census
            .typed_obligations
            .iter()
            .filter(|o| o.status != atlas_core::EpistemicStatus::Unsupported)
            .map(|o| {
                let mut o = o.clone();
                o.extractor.id = "atlas.test.second-engine".into();
                o
            })
            .collect();
        everywhere.census.typed_obligations.extend(seconds);
        assert_eq!(multi(&certify_with(&everywhere, &[&pass, &pass])), None);

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
