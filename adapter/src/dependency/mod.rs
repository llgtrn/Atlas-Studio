//! Dependency-census extraction mechanics (`.atlas/contracts/DEPENDENCY-CENSUS.md`).
//!
//! Typed carrier types live in `atlas_core::census::dependency`; this module holds the filesystem/
//! parsing mechanics that populate them, matching this crate's own boundary ("adapter output
//! becomes typed Atlas facts; adapter mechanics never define the canonical model").

pub mod cargo;
#[cfg(test)]
mod cargo_oracle;

pub use cargo::census_cargo_workspace;
