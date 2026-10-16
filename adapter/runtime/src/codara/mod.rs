//! Codara's source-local runtime interfaces, within the existing runtime package.
//!
//! The artifact implementation physically resides here. Its historical logical module stays
//! `atlasx` so existing public types and callers keep their identities; this route reaches the
//! same implementation and grants no target admission or native adoption.

pub use crate::atlasx as artifact;
