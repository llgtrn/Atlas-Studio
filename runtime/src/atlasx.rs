//! The AtlasX precondition over files (G179, construction node M10, ADR 0093). The files are
//! read here; the decision is `atlas_core::atlasx::precondition`, a pure function of their bytes.

use atlas_core::atlasx::PreconditionInputs;
pub use atlas_core::atlasx::{
    AdmittedParent, Precondition, PreconditionRefusal, PreconditionVerdict,
};
use std::{fs, io, path::Path};

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
