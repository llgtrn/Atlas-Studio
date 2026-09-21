//! Revision and temporal reference primitives.
//!
//! This module owns revision identity only. Wall-clock policy and derived timelines belong in
//! runtime layers until they have durable semantics.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevisionRef {
    pub kind: String,
    pub value: String,
}
