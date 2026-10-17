//! Codara's source-local runtime interfaces, within the existing runtime package.
//!
//! Artifact I/O, container packaging and reconstruction physically reside here. Historical modules
//! stay `atlasx`, `atlas` and `self_reconstruction` so public types keep their identities. These routes reach
//! the same definitions and grant no target admission or native adoption.

// The same container definitions remain available through the historical `runtime::atlas` API.
pub use crate::atlas as container;
pub use crate::atlasx as artifact;
pub use crate::self_reconstruction as reconstruction;
