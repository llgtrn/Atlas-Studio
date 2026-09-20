//! Target Design Graph and graph-diff planning between observed and intended technology.

pub const RESPONSIBILITY: &str = "DESIGN";

/// Atlas subsystem crates emit development analysis/evidence only.
/// They are never imported by Chronica runtime crates.
pub fn subsystem_boundary() -> &'static str {
    "DEVELOPMENT_ENGINEERING_TOOL"
}
