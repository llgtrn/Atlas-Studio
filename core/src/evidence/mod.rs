//! Evidence references produced by observation and verification.

use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Evidence {
    pub id: String,
    pub kind: String,
    pub path: String,
    pub summary: String,
    pub revision: Option<RevisionRef>,
}
