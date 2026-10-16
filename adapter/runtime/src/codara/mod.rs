//! Codara's source-local runtime interfaces, within the existing runtime package.
//!
//! Artifact I/O and container packaging physically reside here. Their historical logical modules
//! stay `atlasx` and `atlas` so public types and callers keep their identities. These routes reach
//! the same definitions and grant no target admission or native adoption.

// The same container definitions remain available through the historical `runtime::atlas` API.
pub use crate::atlas as container;
pub use crate::atlasx as artifact;
