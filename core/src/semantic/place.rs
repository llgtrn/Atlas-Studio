//! `PlaceRef`: the shared cross-dimension semantic-location bridge.
//!
//! DATA_FLOW (`ValueIdentity`), STATE (`StateAccessIdentity`) and OWNERSHIP (`OwnershipIdentity`)
//! each already answer "what semantic location/value is this operation targeting?" with an
//! independently-shaped mechanism (see `core::semantic::persistence`'s module doc comment, which
//! first audited this landscape in detail for R4.11). Any later dimension that needs to reference
//! an existing dimension's own canonical location -- without inventing a fourth, fifth, ... answer
//! to the same question -- points at it through `PlaceRef` instead.
//!
//! First introduced by R4.11 (PERSISTENCE, `core::semantic::persistence::PersistenceIdentity`),
//! and reused as-is by R4.12 (CALL argument binding, `core::semantic::call::CallSiteIdentity`) --
//! moved to its own module at that point since it was no longer a persistence-specific concept.
//!
//! **CURRENT/BRIDGE/TARGET**: `PlaceRef` is deliberately not the final canonical place model. It
//! lets a dimension point at an existing dimension's own already-canonical `SemanticRecordId`
//! instead of inventing a new string-keyed target, without requiring DATA_FLOW/STATE/OWNERSHIP to
//! migrate their own stable identity shapes first. A single first-class `PlaceIdentity` (or
//! equivalent) shared natively by every dimension that needs one remains future TARGET work --
//! tracked in `.atlas/evidence/verification/r4.4-r4.10-second-hardening-pass-correction.json` --
//! and would likely absorb `PlaceRef::Resolved`'s role once it exists.

use super::{SemanticDimension, SemanticRecordId};
use serde::{Deserialize, Serialize};

/// A reference to the semantic location/value an operation acts on or refers to.
///
/// Deliberately excluded from every identity-bearing struct's own `identity_key()` (matching
/// `OwnershipIdentity`'s `resolution` and `ValueIdentity`'s `resolved_definition` precedent): a
/// fact ABOUT an already-identified operation, never part of what makes the operation itself a
/// distinct entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlaceRef {
    /// This operation's location is the same semantic location an existing dimension's record
    /// already names -- e.g. a STATE access's `record_id`, a DATA_FLOW value's `record_id`, or an
    /// OWNERSHIP operation's `record_id` this operation is provably co-located with. `dimension` is
    /// carried alongside `record_id` (rather than left to be re-derived) because each dimension's
    /// own engineering-graph node id uses a distinct prefix scheme (`state-access:`,
    /// `data-flow-value:`, `ownership-op:`, ...) that cannot be reconstructed from a bare
    /// `SemanticRecordId` alone. Never constructed from source spelling alone.
    Resolved {
        dimension: SemanticDimension,
        record_id: SemanticRecordId,
    },
    /// No existing canonical record to point at. Never backfilled with a spelling-derived name --
    /// see this module's doc comment's OWNERSHIP-bug precedent. Distinctness between two operations
    /// that both carry `Unresolved` must come from the containing identity's OWN identity-bearing
    /// fields (span/kind/etc.), never from anything inside this variant.
    Unresolved,
}
