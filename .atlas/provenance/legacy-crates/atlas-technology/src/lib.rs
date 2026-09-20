//! Technology Genome and primitive/capability graph derived from donor/source evidence.

pub const RESPONSIBILITY: &str = "TECHNOLOGY";

/// Atlas subsystem crates emit development analysis/evidence only.
/// They are never imported by Chronica runtime crates.
pub fn subsystem_boundary() -> &'static str {
    "DEVELOPMENT_ENGINEERING_TOOL"
}
