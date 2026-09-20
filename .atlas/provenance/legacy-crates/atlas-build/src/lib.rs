//! Build/affected-CI dependency graph and bounded engineering scheduling.

pub const RESPONSIBILITY: &str = "BUILD";

/// Atlas subsystem crates emit development analysis/evidence only.
/// They are never imported by Chronica runtime crates.
pub fn subsystem_boundary() -> &'static str {
    "DEVELOPMENT_ENGINEERING_TOOL"
}
