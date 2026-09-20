//! Development proof orchestration and evidence requirements; no execution authority.

pub const RESPONSIBILITY: &str = "PROOF";

/// Atlas subsystem crates emit development analysis/evidence only.
/// They are never imported by Chronica runtime crates.
pub fn subsystem_boundary() -> &'static str {
    "DEVELOPMENT_ENGINEERING_TOOL"
}
