//! Architectural integrity entry points (G138, ADR 0056): derive the envelope a repository's ADL
//! declares, and evaluate a pinned envelope against the repository's decided census. The envelope
//! and the evaluation are pure (`atlas_core::integrity`); this module runs the census.

pub use atlas_core::integrity::{
    IntegrityEnvelope, IntegrityReport, IntegrityVerdict, PINNED_ENVELOPE_PATH, check_report,
};
use atlas_core::integrity::{derive_envelope, evaluate};
use std::{io, path::Path};

fn subject(report: &crate::SystemizeReport) -> String {
    report
        .adl
        .ir
        .system
        .clone()
        .map(|s| format!("system:{s}"))
        .unwrap_or_else(|| "system:undeclared".into())
}

/// The envelope `root`'s ADL declares now (authored and census-derived ADL alike).
pub fn envelope(root: impl AsRef<Path>) -> io::Result<IntegrityEnvelope> {
    let report = crate::systemize(root)?;
    Ok(derive_envelope(&subject(&report), &report.adl.ir.declared))
}

pub fn read_envelope(path: impl AsRef<Path>) -> io::Result<IntegrityEnvelope> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: {e}", path.display()),
        )
    })
}

/// Evaluate `pinned` against `root`: one census decides every constraint and names the observed
/// architecture root (its census digest); the envelope the ADL declares now shows whether a pinned
/// invariant was dropped or changed.
pub fn report(root: impl AsRef<Path>, pinned: &IntegrityEnvelope) -> io::Result<IntegrityReport> {
    let report = crate::systemize(root)?;
    let snapshot = atlas_core::recensus::CensusSnapshot::from_report(&report);
    let current = derive_envelope(&subject(&report), &report.adl.ir.declared);
    Ok(evaluate(
        pinned,
        &current,
        &report.adl.constraint_results,
        &format!("revision:{}", report.snapshot.head_sha),
        snapshot.census_digest.as_str(),
    ))
}
