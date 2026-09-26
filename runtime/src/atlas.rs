//! Census -> validated semantic state -> packaged `.atlas` (G64, ADR 0027).
//!
//! `census_atlas` packages one census of a root as an unsealed census container
//! (`atlas_core::atlas`); `pack` publishes it transactionally after verifying the written bytes
//! decode back to exactly the packaged state; `verify` re-reads a container and, given a root,
//! checks that it still describes that root's census; `certificate_with_atlas` issues the
//! CensusCertificate with the container's verified root identity.

use crate::certificate::{CensusCertificate, CertificateInputs, certify, genome_identity};
use atlas_core::{
    IntegrityDigest, SystemizeReport,
    atlas::{
        CensusAtlas, CensusFact, CensusObligation, CertificateRecord, DeclaredEdgeRecord,
        DeclaredNodeRecord, RootManifest, UNSEALED, read, write,
    },
    recensus::CensusSnapshot,
};
use std::{
    fs::{self, File},
    io::{self, Write},
    path::Path,
};

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn digest_bytes(digest: &IntegrityDigest) -> io::Result<[u8; 32]> {
    let hex = digest
        .as_str()
        .strip_prefix(IntegrityDigest::BLAKE3_256_PREFIX)
        .ok_or_else(|| invalid(format!("not a BLAKE3-256 digest: {}", digest.as_str())))?;
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16)
            .map_err(|_| invalid(format!("bad digest hex: {hex}")))?;
    }
    Ok(out)
}

fn revision(report: &SystemizeReport) -> String {
    let dirty = if report.snapshot.dirty { "+dirty" } else { "" };
    format!("git:{}{dirty}", report.snapshot.head_sha)
}

/// The container for `report`: every census fact, typed obligation and declared ADL node/edge,
/// (G147) every typed semantic record, evidence record and extraction diagnostic, plus
/// `certificate` as issued for this container (its own root hash cannot be inside it; state
/// and blockers do not depend on the hash value).
pub fn atlas_of(
    report: &SystemizeReport,
    genome_schema: &str,
    genome_hash: &IntegrityDigest,
    certificate: &CensusCertificate,
) -> io::Result<CensusAtlas> {
    let census_digest = IntegrityDigest::parse(&CensusSnapshot::from_report(report).census_digest)
        .map_err(invalid)?;
    let census = &report.census;
    let mut atlas = CensusAtlas {
        manifest: RootManifest {
            genome_schema: genome_schema.to_owned(),
            genome_hash: digest_bytes(genome_hash)?,
            census_digest: digest_bytes(&census_digest)?,
            revision: revision(report),
            certificate_id: certificate.certificate_id.clone(),
            seal: UNSEALED.into(),
            tool: format!("atlas-systemizer {}", env!("CARGO_PKG_VERSION")),
            mode: "THIN".into(),
        },
        facts: census
            .facts
            .iter()
            .map(|f| CensusFact {
                id: f.id.clone(),
                kind: format!("{:?}", f.kind),
                status: f.status.as_str().to_owned(),
                subject: f.subject.clone(),
                predicate: f.predicate.clone(),
                object: f.object.clone(),
                source_path: f.provenance.source_path.clone(),
                revision: f
                    .provenance
                    .source_revision
                    .as_ref()
                    .map(|r| format!("{}:{}", r.kind, r.value)),
                extractor: f.provenance.extractor.clone(),
                span: f.provenance.span.clone(),
            })
            .collect(),
        obligations: census
            .typed_obligations
            .iter()
            .map(|o| CensusObligation {
                artifact: o.artifact.as_str().to_owned(),
                dimension: o.dimension.as_str().to_owned(),
                extractor: o.extractor.id.clone(),
                extractor_version: o.extractor.version.clone(),
                status: o.status.as_str().to_owned(),
            })
            .collect(),
        nodes: report
            .adl
            .ir
            .declared
            .nodes
            .iter()
            .map(|n| DeclaredNodeRecord {
                name: n.name.clone(),
                kind: n.node_kind.clone(),
                origin: n.origin.clone(),
            })
            .collect(),
        edges: report
            .adl
            .ir
            .declared
            .edges
            .iter()
            .map(|e| DeclaredEdgeRecord {
                from: e.from.clone(),
                relation: e.relation.clone(),
                to: e.to.clone(),
            })
            .collect(),
        certificate: CertificateRecord {
            certificate_id: certificate.certificate_id.clone(),
            state: format!("{:?}", certificate.state).to_ascii_uppercase(),
            blockers: certificate.blockers.clone(),
        },
        // G147 (NA-ATLAS-TYPED-SECTIONS): every typed record, evidence record and diagnostic.
        typed_records: census.typed_semantic_records.clone(),
        evidence: census.evidence.clone(),
        diagnostics: census.diagnostics.clone(),
    };
    atlas.canonicalize();
    // Canonicalization deduplicates; a census whose records collapse is not packaged silently.
    for (what, packaged, censused) in [
        ("facts", atlas.facts.len(), census.facts.len()),
        (
            "obligations",
            atlas.obligations.len(),
            census.typed_obligations.len(),
        ),
        (
            "typed records",
            atlas.typed_records.len(),
            census.typed_semantic_records.len(),
        ),
        (
            "evidence records",
            atlas.evidence.len(),
            census.evidence.len(),
        ),
        (
            "diagnostics",
            atlas.diagnostics.len(),
            census.diagnostics.len(),
        ),
    ] {
        if packaged != censused {
            return Err(invalid(format!(
                "{censused} census {what} collapse to {packaged} distinct records"
            )));
        }
    }
    Ok(atlas)
}

/// Censuses `root` twice (replay), certifies the first pass as it will stand once packaged, and
/// builds its container.
pub fn census_atlas(root: impl AsRef<Path>) -> io::Result<CensusAtlas> {
    let root = root.as_ref();
    let (genome_schema, genome_hash) = genome_identity(root)?;
    let report = crate::systemize(root)?;
    let digests = vec![
        CensusSnapshot::from_report(&report).census_digest,
        crate::recensus::snapshot(root)?.census_digest,
    ];
    let certificate = certify(
        &report,
        &CertificateInputs {
            genome_schema: &genome_schema,
            genome_hash: genome_hash.clone(),
            pass_digests: &digests,
            // The container is this census's packaged root; its hash is not known inside it.
            atlas_root_hash: Some(String::new()),
            atlas_root_sealed: false,
        },
    );
    atlas_of(&report, &genome_schema, &genome_hash, &certificate)
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PackOutcome {
    pub path: String,
    pub root_id: String,
    pub bytes: usize,
    pub census_digest: String,
    pub facts: usize,
    pub obligations: usize,
    pub declared_nodes: usize,
    pub declared_edges: usize,
    pub certificate_state: String,
    /// G147: the typed semantic records, evidence records and diagnostics packaged.
    pub typed_records: usize,
    pub evidence: usize,
    pub diagnostics: usize,
}

fn outcome(
    path: &Path,
    bytes: usize,
    atlas: &CensusAtlas,
    root_id: &IntegrityDigest,
) -> PackOutcome {
    PackOutcome {
        path: path.display().to_string(),
        root_id: root_id.as_str().to_owned(),
        bytes,
        census_digest: IntegrityDigest::blake3_256(&atlas.manifest.census_digest)
            .as_str()
            .to_owned(),
        facts: atlas.facts.len(),
        obligations: atlas.obligations.len(),
        declared_nodes: atlas.nodes.len(),
        declared_edges: atlas.edges.len(),
        certificate_state: atlas.certificate.state.clone(),
        typed_records: atlas.typed_records.len(),
        evidence: atlas.evidence.len(),
        diagnostics: atlas.diagnostics.len(),
    }
}

/// Encodes `atlas`, proves the bytes decode back to it, then publishes them at `out`
/// transactionally: temporary file, fsync, atomic rename (contract: a partially written file is
/// never advertised as a valid root).
pub fn publish(atlas: &CensusAtlas, out: &Path) -> io::Result<PackOutcome> {
    let bytes = write(atlas).map_err(|e| invalid(e.to_string()))?;
    let (decoded, root_id) = read(&bytes).map_err(|e| invalid(e.to_string()))?;
    if &decoded != atlas {
        return Err(invalid(
            "written container does not decode to the packaged state",
        ));
    }
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let temporary = out.with_extension("atlas.partial");
    {
        let mut file = File::create(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }
    fs::rename(&temporary, out)?;
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        File::open(parent)?.sync_all()?;
    }
    Ok(outcome(out, bytes.len(), atlas, &root_id))
}

pub fn pack(root: impl AsRef<Path>, out: impl AsRef<Path>) -> io::Result<PackOutcome> {
    publish(&census_atlas(root)?, out.as_ref())
}

/// Reads and verifies a container file; with `root`, also requires that it packages the current
/// census of that root (same census digest, Genome and certificate identity).
pub fn verify(path: impl AsRef<Path>, root: Option<&Path>) -> io::Result<PackOutcome> {
    let path = path.as_ref();
    let bytes = fs::read(path)?;
    let (atlas, root_id) = read(&bytes).map_err(|e| invalid(e.to_string()))?;
    if let Some(root) = root {
        let current = census_atlas(root)?;
        if current.manifest != atlas.manifest {
            return Err(invalid(format!(
                "{} does not package the current census of {}",
                path.display(),
                root.display()
            )));
        }
        if current != atlas {
            return Err(invalid(format!(
                "{} carries the current census identity but different content",
                path.display()
            )));
        }
    }
    Ok(outcome(path, bytes.len(), &atlas, &root_id))
}

/// The CensusCertificate of `root` with the verified root identity of the container at `atlas`,
/// which must package exactly this census.
pub fn certificate_with_atlas(
    root: impl AsRef<Path>,
    atlas: impl AsRef<Path>,
) -> io::Result<CensusCertificate> {
    let root = root.as_ref();
    let verified = verify(atlas.as_ref(), Some(root))?;
    let (genome_schema, genome_hash) = genome_identity(root)?;
    let report = crate::systemize(root)?;
    let digests = vec![
        CensusSnapshot::from_report(&report).census_digest,
        crate::recensus::snapshot(root)?.census_digest,
    ];
    Ok(certify(
        &report,
        &CertificateInputs {
            genome_schema: &genome_schema,
            genome_hash,
            pass_digests: &digests,
            atlas_root_hash: Some(verified.root_id),
            atlas_root_sealed: false,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    /// The self-scope census packages, publishes and verifies: every fact and obligation is in
    /// the container, the bytes are deterministic, the certificate names the unsealed root, and a
    /// container of a different census is refused.
    #[test]
    fn the_self_census_packages_into_a_verified_unsealed_container() {
        let root = root();
        let (genome_schema, genome_hash) = genome_identity(&root).unwrap();
        let report = crate::systemize(&root).unwrap();
        let digest = CensusSnapshot::from_report(&report).census_digest;
        let certificate = certify(
            &report,
            &CertificateInputs {
                genome_schema: &genome_schema,
                genome_hash: genome_hash.clone(),
                pass_digests: &[digest.clone(), digest],
                atlas_root_hash: Some(String::new()),
                atlas_root_sealed: false,
            },
        );
        let atlas = atlas_of(&report, &genome_schema, &genome_hash, &certificate).unwrap();
        // A census whose facts would collapse under canonicalization is refused, not shrunk.
        let mut duplicated = report.clone();
        let first = duplicated.census.facts[0].clone();
        duplicated.census.facts.push(first);
        assert!(atlas_of(&duplicated, &genome_schema, &genome_hash, &certificate).is_err());
        assert_eq!(atlas.facts.len(), report.census.facts.len());
        assert_eq!(
            atlas.obligations.len(),
            report.census.typed_obligations.len()
        );
        assert!(atlas.facts.iter().any(|f| f.status == "UNKNOWN"));
        assert!(atlas.facts.iter().any(|f| f.status == "UNSUPPORTED"));
        assert!(
            atlas
                .nodes
                .iter()
                .any(|n| n.name == "AtlasCli" && n.kind == "Package")
        );
        assert!(
            atlas
                .certificate
                .blockers
                .iter()
                .any(|b| b.starts_with("ATLAS_ROOT_UNSEALED"))
        );
        assert!(
            !atlas
                .certificate
                .blockers
                .iter()
                .any(|b| b.starts_with("ATLAS_ROOT_ABSENT"))
        );

        let dir = std::env::temp_dir().join(format!("atlas-pack-{}", std::process::id()));
        let out = dir.join("self.atlas");
        let packed = publish(&atlas, &out).unwrap();
        assert!(!out.with_extension("atlas.partial").exists());
        assert_eq!(verify(&out, None).unwrap(), packed);
        assert_eq!(
            fs::read(&out).unwrap(),
            atlas_core::atlas::write(&atlas).unwrap()
        );
        let again = publish(&atlas, &dir.join("again.atlas")).unwrap();
        assert_eq!(again.root_id, packed.root_id, "deterministic root identity");
        // G147 (NA-ATLAS-TYPED-SECTIONS): every typed record of the census -- all twelve
        // families -- every evidence record and every diagnostic come back from the container
        // (`publish` proved the decoded container equal to `atlas`).
        let decoded = &atlas;
        let mut censused = report.census.typed_semantic_records.clone();
        censused.sort_by_cached_key(atlas_core::atlas::typed_key);
        assert_eq!(decoded.typed_records, censused);
        let families: std::collections::BTreeSet<_> = decoded
            .typed_records
            .iter()
            .map(|r| r.dimension())
            .collect();
        assert_eq!(families.len(), 12, "{families:?}");
        assert_eq!(decoded.evidence.len(), report.census.evidence.len());
        assert_eq!(decoded.diagnostics.len(), report.census.diagnostics.len());
        let mut duplicated_record = report.clone();
        let record = duplicated_record.census.typed_semantic_records[0].clone();
        duplicated_record.census.typed_semantic_records.push(record);
        assert!(
            atlas_of(
                &duplicated_record,
                &genome_schema,
                &genome_hash,
                &certificate
            )
            .is_err(),
            "typed records that collapse are refused, not shrunk"
        );

        // The certificate issued with this root is not SEALED and says why.
        let with_root = certify(
            &report,
            &CertificateInputs {
                genome_schema: &genome_schema,
                genome_hash,
                pass_digests: &[packed.census_digest.clone(), packed.census_digest.clone()],
                atlas_root_hash: Some(packed.root_id.clone()),
                atlas_root_sealed: false,
            },
        );
        assert_eq!(
            with_root
                .atlas
                .as_ref()
                .and_then(|a| a.root_hash.as_deref()),
            Some(packed.root_id.as_str())
        );
        assert_ne!(
            with_root.state,
            crate::certificate::CertificateState::Sealed
        );
        assert_eq!(with_root.certificate_id, atlas.certificate.certificate_id);

        // A corrupted file on disk is rejected.
        let mut bytes = fs::read(&out).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        fs::write(dir.join("corrupt.atlas"), &bytes).unwrap();
        assert!(verify(dir.join("corrupt.atlas"), None).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}
