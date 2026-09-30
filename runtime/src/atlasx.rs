//! AtlasX over files: the precondition (G179, construction node M10, ADR 0093; the re-run seal
//! gate of G185, ADR 0097), the object codec (G183, M12, ADR 0095) and the materializer's first
//! slice (G185, M11, ADR 0097). The files are read and written here; each decision is a pure
//! function of their bytes in `atlas_core::atlasx`.

use atlas_core::atlasx::PreconditionInputs;
use atlas_core::atlasx::codec;
pub use atlas_core::atlasx::codec::Defects as CodecDefects;
pub use atlas_core::atlasx::{
    AdmittedParent, CodecDefect, CodecReport, CodecVerdict, EncodedObject, Materialization,
    MaterializationDefect, MaterializationVerdict, Precondition, PreconditionRefusal,
    PreconditionVerdict,
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
/// in. Each object is created
/// new at its canonical path (`functions/<address>.atlasx`) and read back, and must decode under
/// its name. A refused materialization writes nothing, and a staging failure removes what it
/// wrote. Nothing is published: no manifest or root identity is written.
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

/// Creates each object of `materialization` new under `out_dir` and reads it back; every file
/// and directory created is pushed to `written`.
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
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(at(&path))?;
        written.push(path.clone());
        file.write_all(&object.bytes)
            .and_then(|()| file.sync_all())
            .map_err(at(&path))?;
        // Read back under its name: DECODED there means the file is the canonical object whose
        // digest is its address, which is the object written.
        let report = decode_object(&path)?;
        if report.verdict != CodecVerdict::Decoded {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{}: the staged object does not read back", path.display()),
            ));
        }
    }
    Ok(())
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
            "/../core/src/atlasx/codec/fixture_module.json"
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
