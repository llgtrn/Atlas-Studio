//! Derived source facts and cross-reference relations; never canonical Chronica truth.

pub const RESPONSIBILITY: &str = "FACT";

/// Atlas subsystem crates emit development analysis/evidence only.
/// They are never imported by Chronica runtime crates.
pub fn subsystem_boundary() -> &'static str {
    "DEVELOPMENT_ENGINEERING_TOOL"
}
