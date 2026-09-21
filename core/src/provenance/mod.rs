//! Provenance carried by semantic facts and graph records.

use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Provenance {
    pub source_path: String,
    pub source_revision: Option<RevisionRef>,
    pub extractor: String,
    pub content_hash: Option<String>,
    pub span: Option<String>,
}

pub fn provenance(path: impl Into<String>, extractor: impl Into<String>) -> Provenance {
    Provenance {
        source_path: path.into(),
        source_revision: None,
        extractor: extractor.into(),
        content_hash: None,
        span: None,
    }
}
