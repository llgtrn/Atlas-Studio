//! The AtlasX precondition gate (G179, construction node M10, ADR 0093; `contracts/
//! ATLAS-TO-ATLASX.md`, "Precondition gate").
//!
//! A parent is admitted for AtlasX only when all of these hold:
//! - it reads back as a SEALED census container (the M9 reader verifies the wire, every section
//!   hash and the root identity);
//! - its seal record binds it (`seal::check_record`, re-run here rather than trusted to the
//!   reader);
//! - the integrity envelope supplied is the one the record names, and its identity verifies (M5);
//! - the SelectedDesign supplied is the one the record names: its identity verifies, it is
//!   SELECTED, and it selects this container's census under the sealed scope.
//!
//! Anything else is REFUSED, and every reason found is reported, typed and in a deterministic
//! order. Undecodable input is UNREADABLE, never a panic. The gate is a pure function of the bytes
//! it is given: the caller reads the files, and nothing here touches I/O.
//!
//! ADMITTED does not claim the whole contract precondition. The steps this gate does not verify
//! yet are listed in every verdict (`NOT_VERIFIED`). In particular, a SEALED container proves only
//! that its record binds it: the record's identity is a digest, not a signature, so the container
//! does not prove the seal gate decided it.

use crate::atlas::{self, CensusAtlas, SEALED, seal_binding};
use crate::design::{DesignState, SelectedDesign, design_identity};
use crate::integrity::{IntegrityEnvelope, envelope_identity};
use crate::seal::{SealRecord, check_record};
use serde::{Deserialize, Serialize};

pub const PRECONDITION_SCHEMA_VERSION: &str = "atlas.atlasx-precondition.v1";

/// Contract precondition steps this gate does not verify yet, by the contract's step number.
pub const NOT_VERIFIED: &[&str] = &[
    "3: Genome compatibility (the reader checks only the section schema identities it knows)",
    "5: the seal policy, verification report and integrity report the record names (carried as identities, not re-read)",
    "6: the certificate and obligation state the scope requires",
    "7: the design's selection authority (checked by the seal gate; the record carries no signature)",
    "8: the design's roots reachable from the parent root",
    "9: materialization-critical obligations closed",
    "12: the architectural impact of bindings, profiles and closure expansion",
];

crate::vocabulary_enum! {
    /// Whether a parent is admitted for AtlasX materialization.
    pub enum PreconditionVerdict {
        Admitted => "ADMITTED",
        Refused => "REFUSED",
    }
}

crate::vocabulary_enum! {
    /// Why a parent is refused.
    pub enum PreconditionRefusal {
        /// The parent, envelope or design bytes do not decode or do not verify.
        Unreadable => "UNREADABLE",
        /// The parent is not a SEALED census container.
        NotSealed => "NOT_SEALED",
        /// The seal record does not bind the parent (`seal::check_record`).
        SealRecordUnbound => "SEAL_RECORD_UNBOUND",
        /// The envelope's identity does not verify.
        EnvelopeUnverified => "ENVELOPE_UNVERIFIED",
        /// The envelope is not the one the seal record names.
        EnvelopeMismatch => "ENVELOPE_MISMATCH",
        /// No SelectedDesign was supplied.
        DesignAbsent => "DESIGN_ABSENT",
        /// The design's identity does not verify.
        DesignUnverified => "DESIGN_UNVERIFIED",
        /// The design is not the one the seal record names, or selects another census or scope.
        DesignMismatch => "DESIGN_MISMATCH",
        /// The design is not SELECTED.
        DesignNotSelected => "DESIGN_NOT_SELECTED",
    }
}

/// The identities an admitted parent binds: what a materialization must name as its inputs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdmittedParent {
    /// The verified root identity of the SEALED container.
    pub parent_root: String,
    pub census_digest: String,
    pub revision: String,
    pub scope: String,
    pub seal_id: String,
    pub envelope_id: String,
    pub design_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Precondition {
    pub schema: String,
    pub verdict: PreconditionVerdict,
    /// Every reason the parent is refused, typed, with its detail; sorted.
    pub reasons: Vec<(PreconditionRefusal, String)>,
    /// The bound identities, only when ADMITTED.
    pub admitted: Option<AdmittedParent>,
    /// The contract precondition steps this gate does not verify (`NOT_VERIFIED`).
    pub not_verified: Vec<String>,
}

/// The precondition's inputs, as bytes: the parent container, the integrity envelope (JSON) and
/// the SelectedDesign (JSON), if one is supplied.
pub struct PreconditionInputs<'a> {
    pub parent: &'a [u8],
    pub envelope: &'a [u8],
    pub design: Option<&'a [u8]>,
}

type Reasons = Vec<(PreconditionRefusal, String)>;

/// M10: whether the parent in `inputs` is admitted for AtlasX, with every reason when it is not.
pub fn precondition(inputs: &PreconditionInputs) -> Precondition {
    use PreconditionRefusal as R;
    let mut reasons = Reasons::new();
    let parent = match atlas::read(inputs.parent) {
        Ok((atlas, root)) => Some((atlas, root.as_str().to_owned())),
        Err(e) => {
            reasons.push((R::Unreadable, format!("parent: {e}")));
            None
        }
    };
    let envelope = match serde_json::from_slice::<IntegrityEnvelope>(inputs.envelope) {
        Ok(envelope) => Some(envelope),
        Err(e) => {
            reasons.push((R::Unreadable, format!("envelope: {e}")));
            None
        }
    };
    let design = match inputs.design {
        None => {
            reasons.push((R::DesignAbsent, "no SelectedDesign was supplied".into()));
            None
        }
        Some(bytes) => match serde_json::from_slice::<SelectedDesign>(bytes) {
            Ok(design) => Some(design),
            Err(e) => {
                reasons.push((R::Unreadable, format!("design: {e}")));
                None
            }
        },
    };
    let admitted = check(
        parent.as_ref().map(|(atlas, root)| (atlas, root.as_str())),
        envelope.as_ref(),
        design.as_ref(),
        &mut reasons,
    );
    decide(reasons, admitted)
}

/// M10 over inputs already decoded: `parent` as the M9 reader returned it, with its verified
/// root identity `parent_root`.
pub fn admit(
    parent: &CensusAtlas,
    parent_root: &str,
    envelope: &IntegrityEnvelope,
    design: Option<&SelectedDesign>,
) -> Precondition {
    let mut reasons = Reasons::new();
    if design.is_none() {
        reasons.push((
            PreconditionRefusal::DesignAbsent,
            "no SelectedDesign was supplied".into(),
        ));
    }
    let admitted = check(
        Some((parent, parent_root)),
        Some(envelope),
        design,
        &mut reasons,
    );
    decide(reasons, admitted)
}

/// Every check the inputs that decoded allow; the bound identities when all of them pass.
fn check(
    parent: Option<(&CensusAtlas, &str)>,
    envelope: Option<&IntegrityEnvelope>,
    design: Option<&SelectedDesign>,
    reasons: &mut Reasons,
) -> Option<AdmittedParent> {
    use PreconditionRefusal as R;
    // The parent: SEALED, and its record binds it.
    let mut record: Option<&SealRecord> = None;
    let mut census_digest = None;
    if let Some((atlas, _)) = parent {
        let binding = seal_binding(&atlas.manifest);
        match (atlas.manifest.seal.as_str(), &atlas.seal) {
            (SEALED, Some(sealed)) => record = Some(sealed),
            (status, _) => reasons.push((R::NotSealed, format!("seal status {status}"))),
        }
        if let Some(record) = record {
            for problem in check_record(record, &binding, &atlas.certificate.blockers) {
                reasons.push((R::SealRecordUnbound, problem));
            }
        }
        census_digest = Some(binding.census_digest);
    }
    // The envelope: its identity verifies, and it is the one the record names.
    if let Some(envelope) = envelope {
        if envelope_identity(envelope) != envelope.envelope_id {
            reasons.push((
                R::EnvelopeUnverified,
                format!("the identity of {} does not verify", envelope.envelope_id),
            ));
        }
        if let Some(record) = record
            && envelope.envelope_id != record.integrity_envelope
        {
            reasons.push((
                R::EnvelopeMismatch,
                format!(
                    "the seal names envelope {}, not {}",
                    record.integrity_envelope, envelope.envelope_id
                ),
            ));
        }
    }
    // The design: its identity verifies, it is SELECTED, and it is the one the record names,
    // for this container's census and the sealed scope.
    if let Some(design) = design {
        if design_identity(design) != design.design_id {
            reasons.push((
                R::DesignUnverified,
                format!("the identity of {} does not verify", design.design_id),
            ));
        }
        if design.state != DesignState::Selected {
            reasons.push((
                R::DesignNotSelected,
                format!("design state {}", design.state.as_str()),
            ));
        }
        if let (Some(record), Some(census_digest)) = (record, &census_digest)
            && (design.design_id != record.design_id
                || design.candidate != *census_digest
                || design.scope != record.scope)
        {
            reasons.push((
                R::DesignMismatch,
                format!(
                    "the seal names design {} for {} in scope {}, not {} for {} in scope {}",
                    record.design_id,
                    census_digest,
                    record.scope,
                    design.design_id,
                    design.candidate,
                    design.scope
                ),
            ));
        }
    }
    match (parent, record, envelope, design) {
        (Some((atlas, root)), Some(record), Some(envelope), Some(design)) => Some(AdmittedParent {
            parent_root: root.to_owned(),
            census_digest: seal_binding(&atlas.manifest).census_digest,
            revision: atlas.manifest.revision.clone(),
            scope: record.scope.clone(),
            seal_id: record.seal_id.clone(),
            envelope_id: envelope.envelope_id.clone(),
            design_id: design.design_id.clone(),
        }),
        _ => None,
    }
}

fn decide(mut reasons: Reasons, admitted: Option<AdmittedParent>) -> Precondition {
    reasons.sort();
    reasons.dedup();
    let admitted = admitted.filter(|_| reasons.is_empty());
    Precondition {
        schema: PRECONDITION_SCHEMA_VERSION.into(),
        verdict: if admitted.is_some() {
            PreconditionVerdict::Admitted
        } else {
            PreconditionVerdict::Refused
        },
        reasons,
        admitted,
        not_verified: NOT_VERIFIED.iter().map(|s| (*s).to_owned()).collect(),
    }
}

#[cfg(test)]
mod tests;
