//! AtlasX over files: the precondition (G179, construction node M10, ADR 0093) and the object
//! codec (G183, M12, ADR 0095). The files are read and written here; each decision is a pure
//! function of their bytes in `atlas_core::atlasx`.

use atlas_core::atlasx::PreconditionInputs;
use atlas_core::atlasx::codec;
pub use atlas_core::atlasx::codec::Defects as CodecDefects;
pub use atlas_core::atlasx::{
    AdmittedParent, CodecDefect, CodecReport, CodecVerdict, EncodedObject, Precondition,
    PreconditionRefusal, PreconditionVerdict,
};
use atlas_core::construction::ConstructionModule;
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

/// What a caller assembles the precondition's inputs from -- a census container, an integrity
/// envelope and a SelectedDesign with its authority event -- re-exported so a consumer of the
/// runtime (the CLI's end-to-end test) reaches Core only through Runtime.
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
    pub use atlas_core::verification::VerificationPolicy;
}

fn read(path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path).map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))
}

/// Whether the container at `atlas` is admitted as an AtlasX parent with the integrity envelope
/// at `envelope` and the SelectedDesign at `design`. A file that cannot be read is an error;
/// bytes that do not decode are a typed UNREADABLE refusal.
pub fn precondition(
    atlas: impl AsRef<Path>,
    envelope: impl AsRef<Path>,
    design: Option<&Path>,
) -> io::Result<Precondition> {
    let parent = read(atlas.as_ref())?;
    let envelope = read(envelope.as_ref())?;
    let design = design.map(read).transpose()?;
    Ok(atlas_core::atlasx::precondition(&PreconditionInputs {
        parent: &parent,
        envelope: &envelope,
        design: design.as_deref(),
    }))
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
