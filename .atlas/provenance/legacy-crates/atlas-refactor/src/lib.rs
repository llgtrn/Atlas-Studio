//! Refactor/migration plans generated from semantic and design graph differences.

pub const RESPONSIBILITY: &str = "REFACTOR";

/// Atlas subsystem crates emit development analysis/evidence only.
/// They are never imported by Chronica runtime crates.
pub fn subsystem_boundary() -> &'static str {
    "DEVELOPMENT_ENGINEERING_TOOL"
}
