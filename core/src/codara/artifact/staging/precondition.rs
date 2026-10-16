//! The AtlasX precondition gate (G179, construction node M10, ADR 0093; `contracts/
//! ATLAS-TO-ATLASX.md`, "Precondition gate").
//!
//! A parent is admitted for AtlasX only when all of these hold:
//! - it reads back as a SEALED census container (the M9 reader verifies the wire, every section
//!   hash and the root identity);
//! - its seal record binds it (`seal::check_record`, re-run here rather than trusted to the
//!   reader);
//! - G185 (ADR 0097): the integrity envelope is the repository's pinned one (a `Declaration`,
//!   never supplied with the parent), its identity verifies (M5), and it is the one the record
//!   names;
//! - the SelectedDesign supplied is the one the record names: its identity verifies, it is
//!   SELECTED, and it selects this container's census under the sealed scope;
//! - G185: the verification and integrity reports are consistent in themselves (their coverage,
//!   counts, blockers and verdicts re-derived from their outcomes and evaluations), and the
//!   integrity report with the pinned envelope;
//! - G185: the seal gate, re-run here over the declared seal policy, the container's own
//!   certificate, both reports, the design and the declared principal registry, decides ELIGIBLE
//!   for the container as it was before the seal, and the record it decides is exactly the record
//!   the container carries.
//!
//! The last check closes the G179 self-certification: a seal record's identity is a digest, so
//! anyone can stamp a record that binds a container. Such a record is refused unless the gate,
//! given inputs whose identities the record names, decides that same record -- which needs a
//! design selected under a declared principal's signature (ADR 0085). Atlas holds no key and
//! signs nothing. The gate runs last, and only when every other check passed; its authority check
//! runs before the container is re-encoded.
//!
//! Anything else is REFUSED, and every reason found is reported, typed and in a deterministic
//! order. Undecodable input is UNREADABLE, never a panic. The gate is a pure function of the bytes
//! and declarations it is given: the caller reads the files, and nothing here touches I/O.
//!
//! ADMITTED does not claim the whole contract precondition. The steps this gate does not verify
//! yet are listed in every verdict (`NOT_VERIFIED`). In particular, the reports' obligation
//! outcomes and invariant evaluations are taken as authored: their evidence is not in the
//! container, and nothing here re-runs it.

use crate::atlas::{self, CensusAtlas, SEALED, UNSEALED, seal_binding};
use crate::design::{
    DesignState, PrincipalRegistry, SelectedDesign, authority_violations, design_identity,
};
use crate::identity::IntegrityDigest;
use crate::integrity::{IntegrityEnvelope, IntegrityReport, envelope_identity};
use crate::seal::{
    CertificateBinding, GateInputs, ScopeVerdict, SealPolicy, SealRecord, check_record, gate,
};
use crate::verification::VerificationReport;
use serde::{Deserialize, Serialize};

pub const PRECONDITION_SCHEMA_VERSION: &str = "atlas.atlasx-precondition.v1";

/// Contract precondition steps this gate does not verify yet, by the contract's step number.
pub const NOT_VERIFIED: &[&str] = &[
    "3: Genome compatibility (the reader checks only the section schema identities it knows)",
    "5: the verification report's obligation outcomes and the integrity report's invariant evaluations, as authored: each report must be internally consistent (its summary re-derived from them) and name this candidate, but the evidence behind them is not in the container and is not re-run",
    "6: the obligation state the scope requires (the certificate is re-judged by the re-run seal gate)",
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
        /// The parent, design or report bytes do not decode.
        Unreadable => "UNREADABLE",
        /// The parent is not a SEALED census container.
        NotSealed => "NOT_SEALED",
        /// The seal record does not bind the parent (`seal::check_record`).
        SealRecordUnbound => "SEAL_RECORD_UNBOUND",
        /// The pinned envelope's identity does not verify.
        EnvelopeUnverified => "ENVELOPE_UNVERIFIED",
        /// The seal record names another envelope than the one the repository pins.
        EnvelopeMismatch => "ENVELOPE_MISMATCH",
        /// No SelectedDesign was supplied.
        DesignAbsent => "DESIGN_ABSENT",
        /// The design's identity does not verify.
        DesignUnverified => "DESIGN_UNVERIFIED",
        /// The design is not the one the seal record names, or selects another census or scope.
        DesignMismatch => "DESIGN_MISMATCH",
        /// The design is not SELECTED.
        DesignNotSelected => "DESIGN_NOT_SELECTED",
        /// G185: the integrity report contradicts itself or the pinned envelope
        /// (`integrity::check_report`).
        IntegrityReportInconsistent => "INTEGRITY_REPORT_INCONSISTENT",
        /// G185: the verification report contradicts itself (`verification::check_report`).
        VerificationReportInconsistent => "VERIFICATION_REPORT_INCONSISTENT",
        /// G185: the seal gate, re-run over the supplied inputs, does not decide ELIGIBLE.
        SealGateNotEligible => "SEAL_GATE_NOT_ELIGIBLE",
        /// G185: the re-run gate decides ELIGIBLE, but not the record the container carries.
        SealRecordNotDecided => "SEAL_RECORD_NOT_DECIDED",
    }
}

/// The identities an admitted parent binds: what a materialization must name as its inputs,
/// and (G185) under which declaration and by whose selection it was admitted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdmittedParent {
    /// The verified root identity of the SEALED container.
    pub parent_root: String,
    pub census_digest: String,
    pub revision: String,
    pub scope: String,
    pub seal_id: String,
    /// The pinned envelope the seal names.
    pub envelope_id: String,
    pub design_id: String,
    /// G185: the declared seal policy the gate decided under.
    pub policy_id: String,
    /// G185: the declared principal whose signed event selected the design.
    pub principal: String,
    /// G185: the public key (lowercase hex) that event is signed under.
    pub principal_key: String,
    /// G185: BLAKE3 of the declared principal registry the admission was judged against.
    pub registry_digest: String,
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

/// G185: the repository's declarations an admission is judged under -- never the parent's and
/// never an operator's override: the declared seal policy, the pinned integrity envelope and the
/// declared principal registry.
pub struct Declaration<'a> {
    pub policy: &'a SealPolicy,
    pub envelope: &'a IntegrityEnvelope,
    pub registry: &'a PrincipalRegistry,
}

/// G185: the repository's declarations, owned: what a `Declaration` borrows from.
pub struct Declared {
    pub policy: SealPolicy,
    pub envelope: IntegrityEnvelope,
    pub registry: PrincipalRegistry,
}

impl Declared {
    pub fn declaration(&self) -> Declaration<'_> {
        Declaration {
            policy: &self.policy,
            envelope: &self.envelope,
            registry: &self.registry,
        }
    }
}

/// The precondition's inputs: the parent container, the SelectedDesign (JSON), if one is
/// supplied, and the verification and integrity reports (JSON) as bytes, and the repository's
/// declaration.
pub struct PreconditionInputs<'a> {
    pub parent: &'a [u8],
    pub design: Option<&'a [u8]>,
    /// The verification report (JSON).
    pub verification: &'a [u8],
    /// The integrity report (JSON).
    pub integrity: &'a [u8],
    pub declaration: &'a Declaration<'a>,
}

/// G185: the reports the re-run seal gate joins, decoded.
pub struct Reports<'a> {
    pub verification: &'a VerificationReport,
    pub integrity: &'a IntegrityReport,
}

type Reasons = Vec<(PreconditionRefusal, String)>;

/// An admitted parent, decoded: the container and the design the materializer (M11) reads.
pub(crate) struct Admission {
    pub atlas: CensusAtlas,
    pub design: SelectedDesign,
}

fn decode_json<T: serde::de::DeserializeOwned>(
    name: &str,
    bytes: &[u8],
    reasons: &mut Reasons,
) -> Option<T> {
    match serde_json::from_slice::<T>(bytes) {
        Ok(value) => Some(value),
        Err(e) => {
            reasons.push((PreconditionRefusal::Unreadable, format!("{name}: {e}")));
            None
        }
    }
}

/// M10: whether the parent in `inputs` is admitted for AtlasX, with every reason when it is not.
pub fn precondition(inputs: &PreconditionInputs) -> Precondition {
    evaluate(inputs).0
}

/// M10 over bytes, with the decoded parent and design when they are admitted.
pub(crate) fn evaluate(inputs: &PreconditionInputs) -> (Precondition, Option<Admission>) {
    use PreconditionRefusal as R;
    let mut reasons = Reasons::new();
    let parent = match atlas::read(inputs.parent) {
        Ok((atlas, root)) => Some((atlas, root.as_str().to_owned())),
        Err(e) => {
            reasons.push((R::Unreadable, format!("parent: {e}")));
            None
        }
    };
    let design = match inputs.design {
        None => {
            reasons.push((R::DesignAbsent, "no SelectedDesign was supplied".into()));
            None
        }
        Some(bytes) => decode_json::<SelectedDesign>("design", bytes, &mut reasons),
    };
    let verification =
        decode_json::<VerificationReport>("verification", inputs.verification, &mut reasons);
    let integrity = decode_json::<IntegrityReport>("integrity", inputs.integrity, &mut reasons);
    let reports = match (&verification, &integrity) {
        (Some(verification), Some(integrity)) => Some(Reports {
            verification,
            integrity,
        }),
        _ => None,
    };
    let admitted = check(
        parent.as_ref().map(|(atlas, root)| (atlas, root.as_str())),
        design.as_ref(),
        reports.as_ref(),
        inputs.declaration,
        &mut reasons,
    );
    let verdict = decide(reasons, admitted);
    let admission = match (&verdict.verdict, parent, design) {
        (PreconditionVerdict::Admitted, Some((atlas, _)), Some(design)) => {
            Some(Admission { atlas, design })
        }
        _ => None,
    };
    (verdict, admission)
}

/// M10 over inputs already decoded: `parent` as the M9 reader returned it, with its verified
/// root identity `parent_root`, the reports the re-run seal gate joins, and the repository's
/// declaration.
pub fn admit(
    parent: &CensusAtlas,
    parent_root: &str,
    design: Option<&SelectedDesign>,
    reports: &Reports,
    declaration: &Declaration,
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
        design,
        Some(reports),
        declaration,
        &mut reasons,
    );
    decide(reasons, admitted)
}

/// BLAKE3 over a registry's canonical serde form.
fn registry_digest(registry: &PrincipalRegistry) -> String {
    let text = serde_json::to_vec(registry).unwrap_or_default();
    IntegrityDigest::of_bytes(&text).as_str().to_owned()
}

/// Every check the inputs that decoded allow; the bound identities when all of them pass.
fn check(
    parent: Option<(&CensusAtlas, &str)>,
    design: Option<&SelectedDesign>,
    reports: Option<&Reports>,
    declaration: &Declaration,
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
    // The envelope: the repository's pinned one, whose identity verifies, is the one the record
    // names.
    let envelope = declaration.envelope;
    if envelope_identity(envelope) != envelope.envelope_id {
        reasons.push((
            R::EnvelopeUnverified,
            format!(
                "the identity of the pinned envelope {} does not verify",
                envelope.envelope_id
            ),
        ));
    }
    if let Some(record) = record
        && envelope.envelope_id != record.integrity_envelope
    {
        reasons.push((
            R::EnvelopeMismatch,
            format!(
                "the seal names envelope {}, the repository pins {}",
                record.integrity_envelope, envelope.envelope_id
            ),
        ));
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
    // G185: the reports are consistent in themselves, and the integrity report with the pinned
    // envelope; their verdicts are then the ones their own contents imply.
    if let Some(reports) = reports {
        for problem in crate::integrity::check_report(reports.integrity, envelope) {
            reasons.push((R::IntegrityReportInconsistent, problem));
        }
        for problem in crate::verification::check_report(reports.verification) {
            reasons.push((R::VerificationReportInconsistent, problem));
        }
    }
    // G185: last, and only over inputs every other check admits, the seal gate itself.
    if let (true, Some((atlas, _)), Some(record), Some(design), Some(reports)) =
        (reasons.is_empty(), parent, record, design, reports)
    {
        gate_decides(atlas, record, design, reports, declaration, reasons);
    }
    match (parent, record, design) {
        (Some((atlas, root)), Some(record), Some(design)) => {
            let event = design.authority_event.as_ref();
            Some(AdmittedParent {
                parent_root: root.to_owned(),
                census_digest: seal_binding(&atlas.manifest).census_digest,
                revision: atlas.manifest.revision.clone(),
                scope: record.scope.clone(),
                seal_id: record.seal_id.clone(),
                envelope_id: envelope.envelope_id.clone(),
                design_id: design.design_id.clone(),
                policy_id: declaration.policy.policy_id.clone(),
                principal: event.map(|e| e.principal.id.clone()).unwrap_or_default(),
                principal_key: event
                    .and_then(|e| e.signature.as_ref())
                    .map(|s| s.public_key.clone())
                    .unwrap_or_default(),
                registry_digest: registry_digest(declaration.registry),
            })
        }
        _ => None,
    }
}

/// G185: whether the seal gate, re-run over the declared policy and registry, `reports`, the
/// container's own certificate and `design`, decides ELIGIBLE for `atlas` as it stood before the seal, and decides exactly
/// `record`. The gate saw the unsealed container: sealing changes the root manifest, so the root
/// the design selected is recomputed from the same content with the seal removed.
fn gate_decides(
    atlas: &CensusAtlas,
    record: &SealRecord,
    design: &SelectedDesign,
    reports: &Reports,
    declaration: &Declaration,
    reasons: &mut Reasons,
) {
    use PreconditionRefusal as R;
    // The gate's own authority check first: a design no declared principal selected is refused
    // before the container is re-encoded.
    // The same reasons, in the same words, as the gate's.
    if design.authority_event.is_none() {
        reasons.push((
            R::SealGateNotEligible,
            "DESIGN_WITHOUT_AUTHORITY: no authority event".into(),
        ));
        return;
    }
    let refused = authority_violations(design, declaration.registry);
    if !refused.is_empty() {
        let codes: Vec<&str> = refused.iter().map(|v| v.code.as_str()).collect();
        reasons.push((
            R::SealGateNotEligible,
            format!("DESIGN_AUTHORITY_REFUSED: {}", codes.join(", ")),
        ));
        return;
    }
    let mut unsealed = atlas.clone();
    unsealed.manifest.seal = UNSEALED.into();
    unsealed.seal = None;
    let root = match atlas::write(&unsealed).and_then(|bytes| atlas::read(&bytes)) {
        Ok((_, root)) => root.as_str().to_owned(),
        Err(e) => {
            reasons.push((
                R::SealGateNotEligible,
                format!("the container does not encode unsealed: {e}"),
            ));
            return;
        }
    };
    let mut container = seal_binding(&atlas.manifest);
    container.root_id = root;
    let certificate = CertificateBinding {
        certificate_id: atlas.certificate.certificate_id.clone(),
        state: atlas.certificate.state.clone(),
        blockers: atlas.certificate.blockers.clone(),
    };
    let decision = gate(&GateInputs {
        policy: declaration.policy,
        container: &container,
        certificate: &certificate,
        verification: reports.verification,
        integrity: reports.integrity,
        envelope: declaration.envelope,
        design: Some(design),
        registry: declaration.registry,
    });
    if decision.verdict != ScopeVerdict::Eligible {
        for (reason, detail) in decision.reasons {
            reasons.push((R::SealGateNotEligible, format!("{reason}: {detail}")));
        }
        return;
    }
    if decision.record.as_ref() != Some(record) {
        reasons.push((
            R::SealRecordNotDecided,
            format!(
                "the gate decides seal {}, not the container's {}",
                decision.record.map(|r| r.seal_id).unwrap_or_default(),
                record.seal_id
            ),
        ));
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
