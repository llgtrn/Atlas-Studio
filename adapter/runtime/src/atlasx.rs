//! AtlasX over files: the precondition (G179, construction node M10, ADR 0093; the re-run seal
//! gate of G185, ADR 0097), the object codec (G183, M12, ADR 0095), the materializer's first
//! slice (G185, M11, ADR 0097) with its root manifest, and the validator (G187, M13, ADR 0099).
//! The files are read and written here; each decision is a pure function of their bytes in
//! `atlas_core::atlasx`.

use atlas_core::atlasx::PreconditionInputs;
use atlas_core::atlasx::codec;
pub use atlas_core::atlasx::codec::Defects as CodecDefects;
pub use atlas_core::atlasx::{
    AdmittedParent, CodecDefect, CodecReport, CodecVerdict, EncodedObject, MANIFEST_FILE,
    Materialization, MaterializationDefect, MaterializationVerdict, Precondition,
    PreconditionRefusal, PreconditionVerdict, RootEntry, RootFiles, Validation, ValidationDefect,
    ValidationVerdict,
};
use atlas_core::construction::ConstructionModule;
use atlas_core::integrity::PINNED_ENVELOPE_PATH;
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

/// What a caller assembles the precondition's inputs from -- a census container, an integrity
/// envelope, a SelectedDesign with its authority event, and (G185) a seal record -- re-exported
/// so a consumer of the runtime (the CLI's end-to-end test) reaches Core only through Runtime.
pub mod inputs {
    pub use atlas_core::atlas::{
        CensusAtlas, CertificateRecord, RootManifest, SEALED, UNSEALED, read, write,
    };
    pub use atlas_core::design::{
        AuthorityEvent, AuthorityMode, DesignState, EventSignature, Principal, PrincipalKey,
        PrincipalKind, PrincipalRegistry, SelectedDesign, container_candidate, design_identity,
        event_identity, signing_message,
    };
    pub use atlas_core::integrity::{
        ENVELOPE_SCHEMA_VERSION, EnvelopeInvariant, EnvelopeStatus, ImpactClosureRef,
        IntegrityEnvelope, IntegrityReport, IntegrityVerdict, InvariantClass,
        REPORT_SCHEMA_VERSION, Strength, ViolationAction, envelope_identity,
    };
    pub use atlas_core::seal::{SealRecord, seal_identity};
    pub use atlas_core::verification::VerificationPolicy;
}

fn read(path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path).map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))
}

pub use atlas_core::atlasx::Declared;

/// G185: the declarations of the repository at `root`, read from disk: the declared seal policy,
/// the pinned integrity envelope and the declared principals. There is no override: the parent
/// and the operator supply neither.
pub fn read_declared(root: impl AsRef<Path>) -> io::Result<Declared> {
    let root = root.as_ref();
    Ok(Declared {
        policy: crate::seal::declared_policy(root)?,
        envelope: crate::integrity::read_envelope(root.join(PINNED_ENVELOPE_PATH))?,
        registry: crate::design::read_registry(root)?,
    })
}

/// What an AtlasX parent is judged by: the container, design, verification report and integrity
/// report files, under the repository's declarations.
pub struct ParentFiles<'a> {
    pub atlas: &'a Path,
    pub design: Option<&'a Path>,
    pub verification: &'a Path,
    pub integrity: &'a Path,
    pub declared: &'a Declared,
}

/// `decide` over the bytes of `files`. A file that cannot be read is an error.
fn over_bytes<T>(
    files: &ParentFiles,
    decide: impl FnOnce(&PreconditionInputs) -> T,
) -> io::Result<T> {
    let parent = read(files.atlas)?;
    let design = files.design.map(read).transpose()?;
    // A verification file as `verification self` writes it wraps the report with its evidence
    // (`{"evidence": .., "report": ..}`); the seal gate reads the report (`design::read_report`).
    let verification = read(files.verification)?;
    let verification = match serde_json::from_slice::<serde_json::Value>(&verification) {
        Ok(mut wrapped) if wrapped.get("report").is_some() => {
            serde_json::to_vec(&wrapped["report"].take()).unwrap_or(verification)
        }
        _ => verification,
    };
    let integrity = read(files.integrity)?;
    Ok(decide(&PreconditionInputs {
        parent: &parent,
        design: design.as_deref(),
        verification: &verification,
        integrity: &integrity,
        declaration: &files.declared.declaration(),
    }))
}

/// Whether the container in `files` is admitted as an AtlasX parent. A file that cannot be read
/// is an error; bytes that do not decode are a typed UNREADABLE refusal.
pub fn precondition(files: &ParentFiles) -> io::Result<Precondition> {
    over_bytes(files, atlas_core::atlasx::precondition)
}

/// M11 (FUNCTIONS only): materializes the parent in `files` into the staging directory
/// `out_dir`, which must be empty, or absent with its parent present, so no stale object mixes
/// in. Each object is created new at its canonical path (`functions/<address>.atlasx`) and read
/// back, and must decode under its name; then (G187) `manifest.atlasx` is created last and read
/// back, and must decode with the root identity recomputed from what was read. A refused
/// materialization writes nothing, and a staging failure removes what it wrote. Publication
/// steps 1 to 6 are done in staging; nothing is atomically published or advertised (7, 8).
pub fn materialize(files: &ParentFiles, out_dir: impl AsRef<Path>) -> io::Result<Materialization> {
    let materialization = over_bytes(files, atlas_core::atlasx::materialize)?;
    if materialization.verdict != MaterializationVerdict::Staged {
        return Ok(materialization);
    }
    stage(&materialization, out_dir.as_ref())?;
    Ok(materialization)
}

/// Stages the objects of `materialization` under `out_dir`, which must be empty, or absent with
/// its parent present; on any failure, removes every file and directory it created, `out_dir`
/// included when it created it.
fn stage(materialization: &Materialization, out_dir: &Path) -> io::Result<()> {
    let existed = match fs::read_dir(out_dir) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("{}: the staging directory is not empty", out_dir.display()),
                ));
            }
            true
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => false,
        Err(e) => return Err(at(out_dir)(e)),
    };
    let mut written: Vec<PathBuf> = Vec::new();
    let staged = (|| {
        if !existed {
            fs::create_dir(out_dir).map_err(at(out_dir))?;
            written.push(out_dir.to_path_buf());
        }
        stage_objects(materialization, out_dir, &mut written)
    })();
    if staged.is_err() {
        // Nothing half-staged stays behind: the files, then the directories, newest first.
        for path in written.iter().rev() {
            let _ = fs::remove_file(path).or_else(|_| fs::remove_dir(path));
        }
    }
    staged
}

/// An I/O error that names `path`.
fn at(path: &Path) -> impl Fn(io::Error) -> io::Error + use<> {
    let path = path.to_path_buf();
    move |e| io::Error::new(e.kind(), format!("{}: {e}", path.display()))
}

/// Creates each object of `materialization` new under `out_dir` and reads it back, then its
/// manifest; every file and directory created is pushed to `written`.
fn stage_objects(
    materialization: &Materialization,
    out_dir: &Path,
    written: &mut Vec<PathBuf>,
) -> io::Result<()> {
    let mut made: Vec<PathBuf> = Vec::new();
    for object in &materialization.objects {
        let path = out_dir.join(&object.path);
        // Each object directory is created here, never found: one that already exists (a
        // symbolic link included) is an error.
        if let Some(dir) = path.parent()
            && dir != out_dir
            && !made.iter().any(|m| m == dir)
        {
            fs::create_dir(dir).map_err(at(dir))?;
            made.push(dir.to_path_buf());
            written.push(dir.to_path_buf());
        }
        create_synced(&path, &object.bytes, written)?;
        // Read back under its name: DECODED there means the file is the canonical object whose
        // digest is its address, which is the object written.
        let report = decode_object(&path)?;
        if report.verdict != CodecVerdict::Decoded {
            return Err(does_not_read_back(&path));
        }
    }
    // G187, publication step 6: the manifest last, once every object it lists is staged and
    // verified; read back, it must decode and carry the root identity its fields give, which is
    // the one the materialization computed.
    if let Some(manifest) = &materialization.manifest {
        let path = out_dir.join(&manifest.path);
        create_synced(&path, &manifest.bytes, written)?;
        let bytes = read_at_most(&path, OBJECT_READ_LIMIT)?;
        let verified = atlas_core::atlasx::read_manifest(&bytes)
            .ok()
            .and_then(|(_, read)| {
                atlas_core::atlasx::root_identity(&read)
                    .ok()
                    .zip(Some(read))
            });
        match (verified, &materialization.root_id) {
            (Some((recomputed, read)), Some(root_id))
                if recomputed == *root_id && read.root_id == *root_id => {}
            _ => return Err(does_not_read_back(&path)),
        }
    }
    Ok(())
}

fn does_not_read_back(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{}: the staged object does not read back", path.display()),
    )
}

/// Creates the file `path` new with `bytes`, synced, and pushes it to `written`.
fn create_synced(path: &Path, bytes: &[u8], written: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(at(path))?;
    written.push(path.to_path_buf());
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(at(path))
}

/// G187: the most entries `read_root` lists, and the most bytes it reads in all (four objects
/// of the largest size); past either it stops, and the validator refuses ROOT_LIMIT.
pub const ROOT_ENTRY_LIMIT: usize = 1024;
pub const ROOT_BYTE_LIMIT: u64 = 4 * OBJECT_READ_LIMIT;

/// G187: the files of the root directory `dir`, as the validator judges them: each entry at the
/// top level or one directory down. A top-level directory is listed as a directory and read; a
/// regular file is read, at most `OBJECT_READ_LIMIT` of its bytes; a link, a special file or a
/// directory two levels down is listed as not a file and never followed; a name that is not
/// UTF-8 is listed as such. An entry's type and identity are checked when it is listed (without
/// following it) and again when it is opened: the file opened must be a regular file with the
/// listed device and inode, or it is listed as not a file. A writer changing the directory while
/// it is read is outside the model: a FIFO swapped in between the two checks can still block the
/// open (residual RES-G187-CONCURRENT-WRITER).
pub fn read_root(dir: impl AsRef<Path>) -> io::Result<RootFiles> {
    read_root_within(dir.as_ref(), ROOT_ENTRY_LIMIT, ROOT_BYTE_LIMIT)
}

/// `read_root` under the limits `entry_limit` and `byte_limit`; every entry, directories
/// included, counts toward `entry_limit` before it is used.
fn read_root_within(dir: &Path, entry_limit: usize, byte_limit: u64) -> io::Result<RootFiles> {
    let mut root = RootFiles::default();
    let (mut listed, mut total) = (0usize, 0u64);
    let mut pending = vec![(dir.to_path_buf(), String::new(), 0usize)];
    while let Some((path, prefix, depth)) = pending.pop() {
        for entry in fs::read_dir(&path).map_err(at(&path))? {
            let entry = entry.map_err(at(&path))?;
            listed += 1;
            if listed > entry_limit {
                root.over_limit = Some(format!("more than {entry_limit} entries"));
                return Ok(root);
            }
            let own = entry.file_name();
            let Some(own) = own.to_str() else {
                let name = format!("{prefix}{}", own.to_string_lossy());
                root.entries.insert(name, RootEntry::NameNotUtf8);
                continue;
            };
            let name = format!("{prefix}{own}");
            let listing = fs::symlink_metadata(entry.path()).map_err(at(&entry.path()))?;
            let kind = listing.file_type();
            if kind.is_dir() && depth == 0 {
                root.entries.insert(name.clone(), RootEntry::Directory);
                pending.push((entry.path(), format!("{name}/"), depth + 1));
                continue;
            }
            let bytes = if kind.is_file() {
                read_listed(&entry.path(), &listing, OBJECT_READ_LIMIT)?
            } else {
                None
            };
            let Some(bytes) = bytes else {
                root.entries.insert(name, RootEntry::NotAFile);
                continue;
            };
            total += bytes.len() as u64;
            if total > byte_limit {
                root.over_limit = Some(format!("more than {byte_limit} bytes"));
                return Ok(root);
            }
            root.entries.insert(name, RootEntry::File(bytes));
        }
    }
    Ok(root)
}

/// At most `limit` bytes of the file at `path`, when the file opened is the one listed as
/// `listing`: a regular file with the listed device and inode. `None` when it is not -- the entry
/// was replaced after it was listed, by a link to another file, say.
fn read_listed(path: &Path, listing: &fs::Metadata, limit: u64) -> io::Result<Option<Vec<u8>>> {
    let file = fs::File::open(path).map_err(at(path))?;
    let opened = file.metadata().map_err(at(path))?;
    if !opened.is_file() || !same_file(listing, &opened) {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    file.take(limit).read_to_end(&mut bytes).map_err(at(path))?;
    Ok(Some(bytes))
}

/// Whether `a` and `b` describe one file: the same device and inode.
#[cfg(unix)]
fn same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    (a.dev(), a.ino()) == (b.dev(), b.ino())
}

/// Without device and inode numbers, only the type is checked at open.
#[cfg(not(unix))]
fn same_file(_: &fs::Metadata, _: &fs::Metadata) -> bool {
    true
}

/// M13 (FUNCTIONS only): whether the root directory `root_dir` is a valid AtlasX root of the
/// parent in `files`. The root is read by `read_root`; a directory that cannot be listed or a
/// file that cannot be read is an error.
pub fn validate(files: &ParentFiles, root_dir: impl AsRef<Path>) -> io::Result<Validation> {
    let root = read_root(root_dir)?;
    over_bytes(files, |inputs| atlas_core::atlasx::validate(&root, inputs))
}

/// A module written: the object and its path; or why the codec refuses to write it.
pub type Encoded = Result<(EncodedObject, PathBuf), CodecDefects>;

/// M12: the FUNCTIONS object of the construction module at `module` (JSON), written into the
/// directory `out_dir` under its address (`<address>.atlasx`), with the path written. A file that
/// cannot be read or parsed is an error; a module the codec cannot write canonically is its typed
/// defects.
pub fn encode_module(module: impl AsRef<Path>, out_dir: impl AsRef<Path>) -> io::Result<Encoded> {
    let module = module.as_ref();
    let parsed: ConstructionModule = serde_json::from_slice(&read(module)?).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: {e}", module.display()),
        )
    })?;
    let encoded = match atlas_core::atlasx::encode_module(&parsed) {
        Ok(encoded) => encoded,
        Err(defects) => return Ok(Err(defects)),
    };
    let out_dir = out_dir.as_ref();
    let path = out_dir.join(format!("{}.atlasx", encoded.address));
    fs::create_dir_all(out_dir)
        .and_then(|()| fs::write(&path, &encoded.bytes))
        .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))?;
    Ok(Ok((encoded, path)))
}

/// The most bytes `decode_object` reads: a header, the largest payload, and one byte more, so an
/// over-long file is still refused LENGTH_MISMATCH without being read whole.
pub const OBJECT_READ_LIMIT: u64 = codec::HEADER_LEN as u64 + codec::MAX_DECODED_LENGTH + 1;

/// At most `limit` bytes of the file at `path`.
fn read_at_most(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|file| file.take(limit).read_to_end(&mut bytes))
        .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))?;
    Ok(bytes)
}

/// M12: the verdict on the object file at `path`, which must be named by its address. At most
/// `OBJECT_READ_LIMIT` bytes are read.
pub fn decode_object(path: impl AsRef<Path>) -> io::Result<CodecReport> {
    let path = path.as_ref();
    let bytes = read_at_most(path, OBJECT_READ_LIMIT)?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(atlas_core::atlasx::decode_named(&name, &bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("atlas-runtime-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// G185 review N3: a staged object that does not read back is refused, and staging removes
    /// everything it wrote; a staging directory that holds anything is refused before writing.
    #[test]
    fn a_failed_staging_leaves_nothing_behind() {
        let dir = scratch("stage-fail");
        let mut m: Materialization = serde_json::from_value(serde_json::json!({
            "schema": "test", "verdict": "STAGED", "defects": [],
            "precondition": {
                "schema": "test", "verdict": "ADMITTED", "reasons": [], "admitted": null,
                "not_verified": []
            },
            "objects": [], "functions": [], "not_materialized": [], "not_done": []
        }))
        .unwrap();
        let object: atlas_core::atlasx::StagedObject = serde_json::from_value(serde_json::json!({
            "path": "functions/00.atlasx", "address": "00",
            "digest": format!("blake3-256:{}", "0".repeat(64)), "object_class": 4, "decoded_length": 0, "records": 0
        }))
        .unwrap();
        m.objects.push(atlas_core::atlasx::StagedObject {
            bytes: b"not an object".to_vec(),
            ..object
        });
        let out = dir.join("staging");
        // Absent: created, then removed with everything in it.
        let err = stage(&m, &out).unwrap_err();
        assert!(err.to_string().contains("does not read back"), "{err}");
        assert!(!out.exists());
        // Present and empty: kept, emptied.
        fs::create_dir(&out).unwrap();
        let err = stage(&m, &out).unwrap_err();
        assert!(err.to_string().contains("does not read back"), "{err}");
        assert_eq!(fs::read_dir(&out).unwrap().count(), 0);
        // An object directory that already exists as a link is not staged into.
        #[cfg(unix)]
        {
            let elsewhere = dir.join("elsewhere");
            fs::create_dir(&elsewhere).unwrap();
            let fresh = dir.join("fresh");
            fs::create_dir(&fresh).unwrap();
            std::os::unix::fs::symlink(&elsewhere, fresh.join("functions")).unwrap();
            // `stage` refuses the non-empty directory first; beneath it, the object directory is
            // created, never followed.
            assert!(
                stage(&m, &fresh)
                    .unwrap_err()
                    .to_string()
                    .contains("not empty")
            );
            let mut written = Vec::new();
            let err = stage_objects(&m, &fresh, &mut written).unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::AlreadyExists, "{err}");
            assert!(written.is_empty());
            assert_eq!(fs::read_dir(&elsewhere).unwrap().count(), 0);
        }
        fs::write(out.join("stale"), b"x").unwrap();
        let err = stage(&m, &out).unwrap_err();
        assert!(err.to_string().contains("not empty"), "{err}");
        fs::remove_dir_all(&dir).ok();
    }

    /// G187: a staged manifest that does not read back, or does not carry the root identity the
    /// materialization computed, is refused after its objects were written, and staging removes
    /// everything, objects included.
    #[test]
    fn a_manifest_that_does_not_read_back_leaves_nothing_behind() {
        use atlas_core::atlasx::{AtlasxManifest, ObjectEntry, StagedObject};
        use atlas_core::identity::IntegrityDigest;
        let dir = scratch("stage-manifest");
        let module = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../core/src/atlasx/codec/fixture_module.json"
        );
        let (encoded, _) = encode_module(module, &dir).unwrap().unwrap();
        let object = StagedObject {
            path: format!("functions/{}.atlasx", encoded.address),
            address: encoded.address.clone(),
            digest: encoded.digest.clone(),
            object_class: encoded.object_class,
            decoded_length: encoded.decoded_length,
            records: encoded.records,
            bytes: encoded.bytes.clone(),
        };
        let digest = |b: u8| IntegrityDigest::blake3_256(&[b; 32]);
        let mut manifest = AtlasxManifest {
            root_id: digest(0),
            parent_root: digest(1),
            genome_hash: digest(2),
            design_id: "design:x".into(),
            scope_id: "x".into(),
            target_kind: "NONE".into(),
            materializer: "m".into(),
            materializer_version: "1".into(),
            materialization_schema: "s".into(),
            compiler_ir_contract: "UNKNOWN".into(),
            objects: vec![ObjectEntry {
                relative_path: object.path.clone(),
                object_class: 4,
                object_schema_version: 1,
                decoded_content_hash: object.digest.clone(),
                decoded_length: object.decoded_length,
                required: true,
                logical_record_count: 1,
            }],
            census_digest: digest(3),
            revision: "r".into(),
            seal_id: "seal".into(),
        };
        manifest.root_id = atlas_core::atlasx::root_identity(&manifest).unwrap();
        let bytes = atlas_core::atlasx::encode_manifest(&manifest).unwrap();
        let staged_manifest = StagedObject {
            path: MANIFEST_FILE.into(),
            bytes: bytes.clone(),
            ..object.clone()
        };
        let mut m: Materialization = serde_json::from_value(serde_json::json!({
            "schema": "test", "verdict": "STAGED", "defects": [],
            "precondition": {
                "schema": "test", "verdict": "ADMITTED", "reasons": [], "admitted": null,
                "not_verified": []
            },
            "objects": [], "functions": [], "not_materialized": [], "not_done": []
        }))
        .unwrap();
        m.objects.push(object);
        m.manifest = Some(staged_manifest.clone());
        m.root_id = Some(manifest.root_id.clone());
        // Staged, objects first and the manifest last.
        let out = dir.join("good");
        stage(&m, &out).unwrap();
        let root = read_root(&out).unwrap();
        // The object, its class directory and the manifest.
        assert_eq!(root.entries.len(), 3);
        assert_eq!(root.entries[MANIFEST_FILE], RootEntry::File(bytes.clone()));
        // Bytes that are not a manifest, a root id the manifest does not carry, and no root id.
        let mut garbage = m.clone();
        garbage.manifest.as_mut().unwrap().bytes = b"not a manifest".to_vec();
        let mut other = m.clone();
        other.root_id = Some(digest(9));
        let mut none = m.clone();
        none.root_id = None;
        // A manifest whose root id is not the one its fields give, though the materialization
        // names it.
        let mut forged = manifest.clone();
        forged.root_id = digest(9);
        let mut unrooted = m.clone();
        unrooted.manifest.as_mut().unwrap().bytes =
            atlas_core::atlasx::encode_manifest(&forged).unwrap();
        unrooted.root_id = Some(digest(9));
        // A manifest carrying another root id than the one it and the materialization give.
        let mut carried = m.clone();
        carried.manifest.as_mut().unwrap().bytes =
            atlas_core::atlasx::encode_manifest(&forged).unwrap();
        for (name, bad) in [
            ("garbage", garbage),
            ("other", other),
            ("none", none),
            ("unrooted", unrooted),
            ("carried", carried),
        ] {
            let out = dir.join(name);
            let err = stage(&bad, &out).unwrap_err();
            assert!(
                err.to_string()
                    .contains("manifest.atlasx: the staged object does not read back"),
                "{name}: {err}"
            );
            assert!(!out.exists(), "{name}");
        }
        fs::remove_dir_all(&dir).ok();
    }

    /// G187: a root is read one directory deep, every entry counted toward the limit, directories
    /// included; types and identity are checked when an entry is listed and again when it is
    /// opened, so a link is never followed and a file replaced after listing is not read; a
    /// name that is not UTF-8 is listed as such. A concurrent writer is outside the model.
    #[test]
    fn a_root_is_read_bounded_with_types_and_identity_checked_at_listing_and_open() {
        let dir = scratch("read-root");
        let root = dir.join("root");
        fs::create_dir_all(root.join("functions/deeper")).unwrap();
        fs::create_dir_all(root.join("empty")).unwrap();
        fs::write(root.join(MANIFEST_FILE), b"m").unwrap();
        fs::write(root.join("functions/a.atlasx"), b"aa").unwrap();
        fs::write(root.join("functions/deeper/b.atlasx"), b"b").unwrap();
        let read = read_root(&root).unwrap();
        let mut expected = std::collections::BTreeMap::new();
        expected.insert(MANIFEST_FILE.to_owned(), RootEntry::File(b"m".to_vec()));
        expected.insert("functions".to_owned(), RootEntry::Directory);
        expected.insert("empty".to_owned(), RootEntry::Directory);
        expected.insert(
            "functions/a.atlasx".to_owned(),
            RootEntry::File(b"aa".to_vec()),
        );
        expected.insert("functions/deeper".to_owned(), RootEntry::NotAFile);
        assert_eq!(read.entries, expected);
        assert_eq!(read.over_limit, None);
        // Limits: entries, then bytes.
        let limited = read_root_within(&root, 2, 100).unwrap();
        assert_eq!(limited.over_limit.as_deref(), Some("more than 2 entries"));
        let limited = read_root_within(&root, 100, 2).unwrap();
        assert_eq!(limited.over_limit.as_deref(), Some("more than 2 bytes"));
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let outside = dir.join("outside");
            fs::create_dir(&outside).unwrap();
            fs::write(outside.join("secret"), b"s").unwrap();
            std::os::unix::fs::symlink(&outside, root.join("linked")).unwrap();
            std::os::unix::fs::symlink(outside.join("secret"), root.join("functions/l.atlasx"))
                .unwrap();
            let odd = std::ffi::OsStr::from_bytes(b"odd\xff");
            fs::write(root.join(odd), b"x").unwrap();
            fs::create_dir(root.join("functions").join(odd)).unwrap();
            let read = read_root(&root).unwrap();
            assert_eq!(read.entries["linked"], RootEntry::NotAFile);
            assert_eq!(read.entries["functions/l.atlasx"], RootEntry::NotAFile);
            assert!(!read.entries.keys().any(|k| k.contains("secret")));
            assert_eq!(read.entries["odd\u{fffd}"], RootEntry::NameNotUtf8);
            assert_eq!(
                read.entries["functions/odd\u{fffd}"],
                RootEntry::NameNotUtf8
            );
            // A file replaced after it was listed -- here by a link to another file -- is not
            // read: the file opened is not the one listed.
            let listed = root.join("functions/a.atlasx");
            let listing = fs::symlink_metadata(&listed).unwrap();
            assert_eq!(
                read_listed(&listed, &listing, 100).unwrap(),
                Some(b"aa".to_vec())
            );
            fs::remove_file(&listed).unwrap();
            std::os::unix::fs::symlink(outside.join("secret"), &listed).unwrap();
            assert_eq!(read_listed(&listed, &listing, 100).unwrap(), None);
            // Replaced by another regular file (created first, so it has its own inode, then
            // renamed over the entry): not read either.
            fs::write(dir.join("other"), b"zz").unwrap();
            fs::remove_file(&listed).unwrap();
            fs::rename(dir.join("other"), &listed).unwrap();
            assert_eq!(read_listed(&listed, &listing, 100).unwrap(), None);
            // A directory where a file was listed.
            fs::remove_file(&listed).unwrap();
            fs::create_dir(&listed).unwrap();
            assert_eq!(read_listed(&listed, &listing, 100).unwrap(), None);
        }
        assert_eq!(ROOT_BYTE_LIMIT, 4 * OBJECT_READ_LIMIT);
        // Directories count: more than the limit of empty directories is ROOT_LIMIT.
        let many = dir.join("many");
        for i in 0..=ROOT_ENTRY_LIMIT {
            fs::create_dir_all(many.join(format!("d{i}"))).unwrap();
        }
        let read = read_root(&many).unwrap();
        assert_eq!(
            read.over_limit,
            Some(format!("more than {ROOT_ENTRY_LIMIT} entries"))
        );
        let err = read_root(dir.join("absent")).unwrap_err();
        assert!(err.to_string().contains("absent"), "{err}");
        fs::remove_dir_all(&dir).ok();
    }

    /// G183: an object file is read only up to a header, the largest payload and one byte more;
    /// a file past that is refused LENGTH_MISMATCH from its first `OBJECT_READ_LIMIT` bytes.
    #[test]
    fn an_object_file_is_read_only_up_to_the_limit() {
        assert_eq!(
            OBJECT_READ_LIMIT,
            codec::HEADER_LEN as u64 + codec::MAX_DECODED_LENGTH + 1
        );
        let dir = scratch("codec-read");
        let small = dir.join("small");
        fs::write(&small, b"0123456789").unwrap();
        assert_eq!(read_at_most(&small, 4).unwrap(), b"0123");
        assert_eq!(read_at_most(&small, 100).unwrap(), b"0123456789");
        let err = read_at_most(&dir.join("absent"), 4).unwrap_err();
        assert!(err.to_string().contains("absent"), "{err}");

        let module = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../core/src/atlasx/codec/fixture_module.json"
        );
        let (encoded, path) = encode_module(module, &dir).unwrap().unwrap();
        assert_eq!(decode_object(&path).unwrap().verdict, CodecVerdict::Decoded);
        // The same object, followed by zeros to one byte past the limit (a sparse file).
        let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.set_len(OBJECT_READ_LIMIT + 1).unwrap();
        drop(file);
        let report = decode_object(&path).unwrap();
        assert_eq!(report.verdict, CodecVerdict::Refused);
        assert_eq!(report.defects.len(), 1);
        assert_eq!(report.defects[0].0, CodecDefect::LengthMismatch);
        let trailing = OBJECT_READ_LIMIT - encoded.bytes.len() as u64;
        assert!(
            report.defects[0]
                .1
                .starts_with(&format!("{trailing} bytes")),
            "{:?}",
            report.defects
        );
        fs::remove_dir_all(&dir).ok();
    }
}
