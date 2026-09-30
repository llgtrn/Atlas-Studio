//! The AtlasX materializer, first slice (G185, construction node M11, ADR 0097; `contracts/
//! ATLAS-TO-ATLASX.md`, "Materialization stages" and "Lineage"; `contracts/
//! ATLASX-BINARY-WIRE-FORMAT.md`, "Publication transaction").
//!
//! It materializes one object class, FUNCTIONS, the only class the object codec (M12) has, from
//! a parent the precondition gate (M10) admits, and nothing else:
//!
//! 1. M0 (in part: Genome compatibility is not verified) and M1: the precondition decides on the
//!    same bytes, and the materializer reads the parent and the SelectedDesign only when they are
//!    ADMITTED. A refused parent is refused PARENT_NOT_ADMITTED with each of the precondition's
//!    reasons, and no object exists.
//! 2. M2, bounded: every root the design selects must resolve in the parent as a record of the
//!    dimension it names. Each FUNCTION_IDENTITY root selects one function; its FUNCTION_SIGNATURE
//!    record is the one of the same function identity. No signature, two different signatures, or
//!    a declaration kind FUNCTIONS has no dispatch code for is a typed failure, never a guess.
//!    Roots of other dimensions are listed, sorted, in `not_materialized`, never dropped
//!    silently. A repeated root selects once; two distinct functions with one GLOBAL_ID are
//!    GLOBAL_ID_COLLISION.
//! 3. M7/M8: the FUNCTION_SIGNATURE records are written through the codec into one canonical
//!    object, addressed by its digest, for the staging path `functions/<address>.atlasx`.
//! 4. M11, for this object: the bytes are read back through the codec, and every record's lineage
//!    must resolve among the parent's census records and include a root the design selected.
//!
//! 5. G187 (M13, ADR 0099), publication steps 4 to 6 in part: the ROOT_MANIFEST of the staged
//!    objects is built (`manifest_for`), naming the parent binding the precondition admitted, its
//!    AtlasX root identity computed, and the manifest read back through its codec with the root
//!    identity recomputed. The runtime writes it last, after the objects.
//!
//! Each record's lineage is the census records it was lifted from: the function's
//! FUNCTION_IDENTITY and FUNCTION_SIGNATURE records. Its GLOBAL_ID is the construction IR's
//! provisional function id (`fn:<path>::<owner>::<name>`). The parent root, census digest,
//! revision, seal and design are bound canonically by the manifest (G187); the declaration the
//! parent was admitted under, by the embedded precondition verdict. Publication steps 7 and 8
//! are not done (`MATERIALIZE_NOT_DONE`): what this produces is a staged root, never a published
//! or advertised one, and a staged root is VALID only when `validate` says so.

use super::codec::{self, CodecVerdict, FunctionSignatureRecord};
use super::manifest::{self, AtlasxManifest, CLASS_ROOT_MANIFEST, MANIFEST_FILE, ObjectEntry};
use super::precondition::{
    Admission, AdmittedParent, Precondition, PreconditionInputs, PreconditionVerdict, evaluate,
};
use crate::SemanticObservation;
use crate::atlas::CensusAtlas;
use crate::construction::{Dispatch, IrParam};
use crate::design::SelectedDesign;
use crate::identity::IntegrityDigest;
use crate::semantic::{FunctionDeclarationKind, SemanticDimension};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// v2 (G187): the result carries the staged manifest and the AtlasX root identity. It is also
/// the manifest's tag 9, so the version takes part in the root identity.
pub const MATERIALIZATION_SCHEMA_VERSION: &str = "atlas.atlasx-materialization.v2";
/// The canonical directory of FUNCTIONS objects (`ATLASX-BINARY-WIRE-FORMAT.md`).
pub const FUNCTIONS_DIRECTORY: &str = "functions";
/// G187: the materializer's identity and version, as the manifest names them (tags 7 and 8).
/// The version moves with any change to what is written for the same inputs; the fixture's
/// pinned root id shows such a change.
pub const MATERIALIZER_IDENTITY: &str = "atlas_core::atlasx::materialize";
pub const MATERIALIZER_VERSION: &str = "1";
/// G187: the target kind (tag 6). No target or profile is frozen (M6 is not done): none is named.
pub const TARGET_KIND_NONE: &str = "NONE";
/// G187: the compiler IR contract (tag 10). None is defined: its compatibility is UNKNOWN.
pub const COMPILER_IR_CONTRACT_UNKNOWN: &str = "UNKNOWN";

/// What this slice of the materializer does not do; listed in every result.
pub const MATERIALIZE_NOT_DONE: &[&str] = &[
    "publication steps 7-8: the objects and manifest.atlasx stay in staging; nothing is atomically published or advertised",
    "every class but FUNCTIONS: a selected root of another dimension is listed in not_materialized",
    "selection closure beyond the design's FUNCTION_IDENTITY roots (calls, types, state, effects and the rest of the contract's closure)",
    "function bodies: FUNCTION_SIGNATURE carries none",
    "a stable GLOBAL_ID form: the construction IR's provisional function id is carried",
    "the epistemic status of the census records materialized",
    "stages M3-M6: obligations, bindings, template expansion, profiles",
    "M11's deterministic reproduction check (a second, independent materialization compared byte for byte)",
    "validation of the staged root: `validate` (M13) judges it",
    "the manifest's profiles, external bindings, semantic barriers, dynamic obligations and compatibility requirements: none is materialized, the target kind is NONE and the compiler IR contract UNKNOWN",
];

crate::vocabulary_enum! {
    /// Whether a materialization produced staged objects.
    pub enum MaterializationVerdict {
        Staged => "STAGED",
        Refused => "REFUSED",
    }
}

crate::vocabulary_enum! {
    /// Why a materialization is refused.
    pub enum MaterializationDefect {
        /// The precondition gate does not admit the parent.
        ParentNotAdmitted => "PARENT_NOT_ADMITTED",
        /// A design root does not resolve in the parent as a record of the dimension it names.
        RootUnresolved => "ROOT_UNRESOLVED",
        /// The design selects no function.
        NothingToMaterialize => "NOTHING_TO_MATERIALIZE",
        /// A selected function has no FUNCTION_SIGNATURE record in the parent.
        SignatureAbsent => "SIGNATURE_ABSENT",
        /// A selected function has two different FUNCTION_SIGNATURE records.
        SignatureConflict => "SIGNATURE_CONFLICT",
        /// A selected function's declaration kind has no FUNCTIONS dispatch code.
        DispatchUnsupported => "DISPATCH_UNSUPPORTED",
        /// Two distinct selected functions have the same (provisional) GLOBAL_ID.
        GlobalIdCollision => "GLOBAL_ID_COLLISION",
        /// The codec refuses to write the records, or to read back what it wrote.
        EncodingRefused => "ENCODING_REFUSED",
        /// A record's lineage names a record the parent does not carry.
        LineageOutsideParent => "LINEAGE_OUTSIDE_PARENT",
        /// A record's lineage includes no root the design selected.
        LineageNotSelected => "LINEAGE_NOT_SELECTED",
    }
}

pub type MaterializationDefects = Vec<(MaterializationDefect, String)>;

/// One object for staging, at `path` under the staging directory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StagedObject {
    /// `functions/<address>.atlasx`.
    pub path: String,
    pub address: String,
    pub digest: IntegrityDigest,
    pub object_class: u16,
    pub decoded_length: u64,
    pub records: usize,
    #[serde(skip)]
    pub bytes: Vec<u8>,
}

/// The explicit mapping of one materialized function to the census records it came from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MaterializedFunction {
    pub function_id: String,
    pub lineage: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Materialization {
    pub schema: String,
    pub verdict: MaterializationVerdict,
    /// Every defect, typed, with its detail; sorted.
    pub defects: MaterializationDefects,
    /// The precondition's verdict on the same inputs: the parent root, seal and design bound.
    pub precondition: Precondition,
    /// Only when STAGED.
    pub objects: Vec<StagedObject>,
    /// G187, only when STAGED: `manifest.atlasx` over `objects`, and the AtlasX root identity it
    /// carries. A staged root, never a published one.
    pub manifest: Option<StagedObject>,
    pub root_id: Option<IntegrityDigest>,
    pub functions: Vec<MaterializedFunction>,
    /// Selected roots of dimensions no materialized class carries, as `DIMENSION:record`.
    pub not_materialized: Vec<String>,
    pub not_done: Vec<String>,
}

fn settle(mut defects: MaterializationDefects) -> MaterializationDefects {
    defects.sort();
    defects.dedup();
    defects
}

/// M11 (FUNCTIONS only): the staged objects of the parent in `inputs`, when the precondition
/// admits it and every selected function materializes with lineage into the parent.
pub fn materialize(inputs: &PreconditionInputs) -> Materialization {
    let (precondition, admission) = evaluate(inputs);
    materialize_over(precondition, admission.as_ref())
}

/// `materialize` over a precondition already decided, with its decoded parent when admitted.
/// G187: the validator reproduces a root through this, over the admission it already holds.
pub(crate) fn materialize_over(
    precondition: Precondition,
    admission: Option<&Admission>,
) -> Materialization {
    use MaterializationDefect as D;
    let mut result = Materialization {
        schema: MATERIALIZATION_SCHEMA_VERSION.into(),
        verdict: MaterializationVerdict::Refused,
        defects: MaterializationDefects::new(),
        precondition,
        objects: Vec::new(),
        manifest: None,
        root_id: None,
        functions: Vec::new(),
        not_materialized: Vec::new(),
        not_done: MATERIALIZE_NOT_DONE
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
    };
    let admission = match (&result.precondition.verdict, admission) {
        (PreconditionVerdict::Admitted, Some(admission)) => admission,
        _ => {
            let mut defects: MaterializationDefects = result
                .precondition
                .reasons
                .iter()
                .map(|(reason, detail)| (D::ParentNotAdmitted, format!("{reason}: {detail}")))
                .collect();
            if defects.is_empty() {
                defects.push((D::ParentNotAdmitted, "the parent is not admitted".into()));
            }
            result.defects = settle(defects);
            return result;
        }
    };
    let (records, not_materialized, mut defects) = select(&admission.atlas, &admission.design);
    result.not_materialized = not_materialized;
    let mut object = None;
    if defects.is_empty() {
        match stage(&admission.atlas, &admission.design, &records) {
            Ok(staged) => object = Some(staged),
            Err(found) => defects = found,
        }
    }
    let mut root = None;
    if let (Some(admitted), Some((staged, _))) = (&result.precondition.admitted, &object) {
        match stage_manifest(admission, admitted, std::slice::from_ref(staged)) {
            Ok(staged) => root = Some(staged),
            Err(found) => defects.extend(encoding(found)),
        }
    }
    result.defects = settle(defects);
    if let (true, Some((staged, decoded)), Some((manifest, root_id))) =
        (result.defects.is_empty(), object, root)
    {
        result.verdict = MaterializationVerdict::Staged;
        result.manifest = Some(manifest);
        result.root_id = Some(root_id);
        result.functions = decoded
            .into_iter()
            .map(|r| MaterializedFunction {
                function_id: r.function_id,
                lineage: r.lineage,
            })
            .collect();
        result.objects = vec![staged];
    }
    result
}

/// The FUNCTION_SIGNATURE records the design's roots select in `parent`, the roots no class here
/// carries, and every defect of the selection.
fn select(
    parent: &CensusAtlas,
    design: &SelectedDesign,
) -> (
    Vec<FunctionSignatureRecord>,
    Vec<String>,
    MaterializationDefects,
) {
    use MaterializationDefect as D;
    let by_id: BTreeMap<&str, &SemanticObservation> = parent
        .typed_records
        .iter()
        .map(|r| (r.record_id().as_str(), r))
        .collect();
    // Every FUNCTION_SIGNATURE record by the identity key of the function it signs, built once.
    let mut signatures_of: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for record in &parent.typed_records {
        if let SemanticObservation::FunctionSignature(s) = record {
            signatures_of
                .entry(s.subject.function.identity_key())
                .or_default()
                .push(s.as_ref());
        }
    }
    let mut records = Vec::new();
    let mut not_materialized = Vec::new();
    let mut defects = MaterializationDefects::new();
    let mut seen = BTreeSet::new();
    for root in &design.roots {
        // A root the design repeats selects once.
        if !seen.insert((root.dimension, root.record_id.as_str())) {
            continue;
        }
        let named = format!("{}:{}", root.dimension.as_str(), root.record_id);
        let record = match by_id.get(root.record_id.as_str()) {
            Some(record) if record.dimension() == root.dimension => *record,
            Some(record) => {
                defects.push((
                    D::RootUnresolved,
                    format!("{named} is a {} record", record.dimension().as_str()),
                ));
                continue;
            }
            None => {
                defects.push((D::RootUnresolved, format!("{named} is not in the parent")));
                continue;
            }
        };
        let SemanticObservation::FunctionIdentity(identity) = record else {
            not_materialized.push(named);
            continue;
        };
        let signatures = signatures_of
            .get(&identity.subject.identity_key())
            .map(Vec::as_slice)
            .unwrap_or_default();
        let Some(signature) = signatures.first() else {
            defects.push((D::SignatureAbsent, named));
            continue;
        };
        if signatures.iter().any(|s| s.subject != signature.subject) {
            let ids: Vec<&str> = signatures.iter().map(|s| s.record_id.as_str()).collect();
            defects.push((D::SignatureConflict, format!("{named}: {}", ids.join(", "))));
            continue;
        }
        let f = &signature.subject.function;
        let dispatch = match f.declaration_kind {
            FunctionDeclarationKind::FreeFunction => Dispatch::FreeFunction,
            FunctionDeclarationKind::InherentMethod => Dispatch::InherentMethod,
            FunctionDeclarationKind::AssociatedFunction => Dispatch::AssociatedFunction,
            other => {
                defects.push((
                    D::DispatchUnsupported,
                    format!("{named}: {}", other.as_str()),
                ));
                continue;
            }
        };
        let owner = f.owner.target.as_ref().map(|t| t.name.clone());
        let function_id = match &owner {
            Some(owner) => format!("fn:{}::{owner}::{}", f.span.path, f.symbol.name),
            None => format!("fn:{}::{}", f.span.path, f.symbol.name),
        };
        let mut record = FunctionSignatureRecord {
            function_id,
            name: f.symbol.name.clone(),
            owner,
            dispatch,
            visibility: signature.subject.visibility.clone(),
            documentation: f.symbol.documentation.as_ref().map(|d| d.summary.clone()),
            params: signature
                .subject
                .parameters
                .iter()
                .map(|p| IrParam {
                    name: p.name.clone(),
                    type_spelling: p.type_identity.name.clone(),
                })
                .collect(),
            result: signature
                .subject
                .return_type
                .as_ref()
                .map(|t| t.name.clone()),
            body_fingerprint: signature.subject.body_fingerprint.clone(),
            lineage: vec![
                identity.record_id.as_str().to_owned(),
                signature.record_id.as_str().to_owned(),
            ],
        };
        record.canonicalize();
        records.push(record);
    }
    // Distinct functions the provisional GLOBAL_ID form cannot tell apart: none is chosen.
    let mut by_id: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for record in &records {
        by_id
            .entry(record.function_id.clone())
            .or_default()
            .push(record.lineage.join(" "));
    }
    for (function_id, lineages) in &by_id {
        if lineages.len() > 1 {
            defects.push((
                D::GlobalIdCollision,
                format!("{function_id}: {}", lineages.join("; ")),
            ));
        }
    }
    records.retain(|r| by_id.get(&r.function_id).is_none_or(|l| l.len() == 1));
    not_materialized.sort();
    if defects.is_empty() && records.is_empty() {
        defects.push((
            D::NothingToMaterialize,
            "the design selects no FUNCTION_IDENTITY root".into(),
        ));
    }
    (records, not_materialized, defects)
}

/// A codec refusal, each defect ENCODING_REFUSED with the codec's own.
fn encoding(defects: codec::Defects) -> MaterializationDefects {
    defects
        .into_iter()
        .map(|(defect, detail)| {
            (
                MaterializationDefect::EncodingRefused,
                format!("{defect}: {detail}"),
            )
        })
        .collect()
}

/// G187: the materialized scope (tag 5): the sealed scope, narrowed to the one class and the
/// selection this slice materializes (RES-G185-CLOSURE-ROOTS-ONLY).
pub fn scope_id(sealed_scope: &str) -> String {
    format!("{sealed_scope}/FUNCTIONS/design-function-roots")
}

/// G187: the manifest this materializer writes for `objects` of the parent `admitted` (whose
/// container is `admission`), with its root identity set. The validator builds the same one to
/// judge a manifest's parent binding and fields.
pub(crate) fn manifest_for(
    admission: &Admission,
    admitted: &AdmittedParent,
    objects: Vec<ObjectEntry>,
) -> Result<AtlasxManifest, codec::Defects> {
    let digest = |value: &str| {
        IntegrityDigest::parse(value)
            .or_else(|e| codec::defect(codec::CodecDefect::MalformedValue, e))
    };
    let mut manifest = AtlasxManifest {
        root_id: IntegrityDigest::blake3_256(&[0; 32]),
        parent_root: digest(&admitted.parent_root)?,
        genome_hash: IntegrityDigest::blake3_256(&admission.atlas.manifest.genome_hash),
        design_id: admitted.design_id.clone(),
        scope_id: scope_id(&admitted.scope),
        target_kind: TARGET_KIND_NONE.into(),
        materializer: MATERIALIZER_IDENTITY.into(),
        materializer_version: MATERIALIZER_VERSION.into(),
        materialization_schema: MATERIALIZATION_SCHEMA_VERSION.into(),
        compiler_ir_contract: COMPILER_IR_CONTRACT_UNKNOWN.into(),
        objects,
        census_digest: digest(&admitted.census_digest)?,
        revision: admitted.revision.clone(),
        seal_id: admitted.seal_id.clone(),
    };
    manifest.root_id = manifest::root_identity(&manifest)?;
    Ok(manifest)
}

/// G187, publication steps 4 and 5: the manifest over the staged `objects`, with its root
/// identity. Step 6's verification is the runtime's: it reads `manifest.atlasx` back from staging
/// and recomputes the root identity from what it read.
fn stage_manifest(
    admission: &Admission,
    admitted: &AdmittedParent,
    objects: &[StagedObject],
) -> Result<(StagedObject, IntegrityDigest), codec::Defects> {
    let entries = objects
        .iter()
        .map(|o| ObjectEntry {
            relative_path: o.path.clone(),
            object_class: o.object_class,
            object_schema_version: codec::FUNCTIONS_SCHEMA_VERSION,
            decoded_content_hash: o.digest.clone(),
            decoded_length: o.decoded_length,
            required: true,
            logical_record_count: o.records as u64,
        })
        .collect();
    let built = manifest_for(admission, admitted, entries)?;
    let bytes = manifest::encode_manifest(&built)?;
    let header = codec::ObjectHeader::parse(&bytes).expect("the writer writes a header");
    let staged = StagedObject {
        path: MANIFEST_FILE.into(),
        address: header.address(),
        digest: header.digest(),
        object_class: CLASS_ROOT_MANIFEST,
        decoded_length: header.decoded_length,
        records: 1,
        bytes,
    };
    Ok((staged, built.root_id))
}

/// The FUNCTIONS object of `records`, read back through the codec, with its lineage checked
/// against `parent` and `design`.
fn stage(
    parent: &CensusAtlas,
    design: &SelectedDesign,
    records: &[FunctionSignatureRecord],
) -> Result<(StagedObject, Vec<FunctionSignatureRecord>), MaterializationDefects> {
    let bytes = codec::encode_functions(records).map_err(encoding)?;
    let report = codec::decode(&bytes);
    if report.verdict != CodecVerdict::Decoded {
        return Err(encoding(report.defects));
    }
    let (Some(address), Some(digest), Some(object_class), Some(decoded_length), Some(decoded)) = (
        report.address,
        report.digest,
        report.object_class,
        report.decoded_length,
        report.records,
    ) else {
        return Err(encoding(report.defects));
    };
    let defects = check_lineage(parent, design, &decoded);
    if !defects.is_empty() {
        return Err(defects);
    }
    let staged = StagedObject {
        path: format!("{FUNCTIONS_DIRECTORY}/{address}.atlasx"),
        address,
        digest,
        object_class,
        decoded_length,
        records: decoded.len(),
        bytes,
    };
    Ok((staged, decoded))
}

/// Every way the lineage of `records` fails to map back to `parent` under `design`: an entry
/// that is not a census record of the parent, or a record whose lineage includes no
/// FUNCTION_IDENTITY root the design selected.
pub fn check_lineage(
    parent: &CensusAtlas,
    design: &SelectedDesign,
    records: &[FunctionSignatureRecord],
) -> MaterializationDefects {
    use MaterializationDefect as D;
    let parent_ids: BTreeSet<&str> = parent
        .typed_records
        .iter()
        .map(|r| r.record_id().as_str())
        .collect();
    let selected: BTreeSet<&str> = design
        .roots
        .iter()
        .filter(|r| r.dimension == SemanticDimension::FunctionIdentity)
        .map(|r| r.record_id.as_str())
        .collect();
    let mut defects = MaterializationDefects::new();
    for record in records {
        for entry in &record.lineage {
            if !parent_ids.contains(entry.as_str()) {
                defects.push((
                    D::LineageOutsideParent,
                    format!("{}: {entry}", record.function_id),
                ));
            }
        }
        if !record
            .lineage
            .iter()
            .any(|entry| selected.contains(entry.as_str()))
        {
            defects.push((D::LineageNotSelected, record.function_id.clone()));
        }
    }
    settle(defects)
}

#[cfg(test)]
mod tests;
