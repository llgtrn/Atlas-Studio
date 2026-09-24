//! Self-recensus orchestration (ADR 0024): census Atlas, project the census into its
//! revision-independent semantic state, and prove a generation against its pre-change census.

pub use atlas_core::recensus::{
    CensusSnapshot, RecensusIntent, SelfRecensusReport, Verdict, prove,
};
use std::{fs, io, path::Path};

/// Runs the full census (`systemize`) of `root` and projects it.
pub fn snapshot(root: impl AsRef<Path>) -> io::Result<CensusSnapshot> {
    let report = crate::systemize(root)?;
    Ok(CensusSnapshot::from_report(&report))
}

pub fn read_snapshot(path: impl AsRef<Path>) -> io::Result<CensusSnapshot> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)?;
    let snapshot: CensusSnapshot = serde_json::from_str(&text).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: {e}", path.display()),
        )
    })?;
    if !snapshot.verify_digest() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{}: census digest does not match the snapshot content",
                path.display()
            ),
        ));
    }
    Ok(snapshot)
}

/// Censuses the candidate twice (the second run is the deterministic replay) and proves it
/// against `before`. Returns the report and the candidate snapshot (the next generation's base).
pub fn prove_candidate(
    root: impl AsRef<Path>,
    generation: &str,
    before: &CensusSnapshot,
    intent: &RecensusIntent,
) -> io::Result<(SelfRecensusReport, CensusSnapshot)> {
    let root = root.as_ref();
    let after = snapshot(root)?;
    let replay = snapshot(root)?;
    Ok((prove(generation, before, &after, &replay, intent), after))
}
