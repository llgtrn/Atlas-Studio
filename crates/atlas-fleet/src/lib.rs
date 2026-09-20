//! Cross-repository engineering/network audit and convergence planning.

pub const RESPONSIBILITY: &str = "FLEET";

/// Atlas subsystem crates emit development analysis/evidence only.
/// They are never imported by Chronica runtime crates.
pub fn subsystem_boundary() -> &'static str {
    "DEVELOPMENT_ENGINEERING_TOOL"
}
