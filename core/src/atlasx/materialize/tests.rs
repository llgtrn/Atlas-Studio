//! M11 (FUNCTIONS only) falsified on the G161 fixture carrying real census records of
//! `MaterializationMode::is_local`: an admitted parent stages one FUNCTIONS object whose record
//! reads back with lineage into the parent; a parent the precondition refuses -- unsealed, or
//! sealed by a record the gate did not decide -- is refused before any object exists; a selection
//! the parent cannot answer and lineage that does not map back are refused with typed defects.

use super::*;
use crate::atlas::write;
use crate::atlasx::fixture::{
    self, Fixture, IS_LOCAL_IDENTITY, IS_LOCAL_SIGNATURE, MODE_SYMBOL, fixture,
};
use crate::atlasx::precondition::PreconditionRefusal;
use crate::construction::ConstructionModule;
use crate::design::{SelectedDesign, SemanticRoot};
use crate::seal::seal_identity;

use MaterializationDefect as D;

fn json<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

/// The materializer over `parent` and `design`, with the fixture's envelope and seal gate inputs.
fn run(f: &Fixture, parent: &CensusAtlas, design: Option<&SelectedDesign>) -> Materialization {
    let design = design.map(json);
    materialize(&PreconditionInputs {
        parent: &write(parent).unwrap(),
        design: design.as_deref(),
        verification: &json(&f.verification),
        integrity: &json(&f.integrity),
        declaration: &f.declared.declaration(),
    })
}

/// `unsealed` sealed by the real gate for a design selecting `roots`, then materialized.
fn sealed_run(f: &Fixture, unsealed: &CensusAtlas, roots: Vec<SemanticRoot>) -> Materialization {
    let design = fixture::design_over(unsealed, roots);
    let sealed = fixture::seal(unsealed, &design, f).expect("the gate decides ELIGIBLE");
    let m = run(f, &sealed, Some(&design));
    assert_eq!(
        m.precondition.verdict,
        PreconditionVerdict::Admitted,
        "{:?}",
        m.precondition
    );
    m
}

fn kinds(m: &Materialization) -> Vec<MaterializationDefect> {
    m.defects.iter().map(|(d, _)| *d).collect()
}

fn root(dimension: SemanticDimension, record_id: &str) -> SemanticRoot {
    SemanticRoot {
        dimension,
        record_id: record_id.into(),
    }
}

#[test]
fn an_admitted_parent_stages_its_selected_functions_with_lineage_into_the_parent() {
    let f = fixture();
    let m = run(&f, &f.sealed, Some(&f.design));
    assert_eq!(m.verdict, MaterializationVerdict::Staged, "{:?}", m.defects);
    assert!(m.defects.is_empty());
    assert_eq!(m.schema, MATERIALIZATION_SCHEMA_VERSION);
    assert_eq!(m.not_done, MATERIALIZE_NOT_DONE);
    let admitted = m.precondition.admitted.as_ref().expect("admitted");
    assert_eq!(admitted.design_id, f.design.design_id);
    assert_eq!(admitted.seal_id, f.sealed.seal.as_ref().unwrap().seal_id);
    // The type definition the design also selects has no class here: listed, not dropped.
    assert_eq!(m.not_materialized, [format!("SYMBOL:{MODE_SYMBOL}")]);
    // One FUNCTIONS object, addressed by its digest, under the canonical directory.
    assert_eq!(m.objects.len(), 1);
    let object = &m.objects[0];
    assert_eq!(object.path, format!("functions/{}.atlasx", object.address));
    // Pinned: a change of the record, its lineage or the codec shows here.
    assert_eq!(
        object.address,
        "6e6af317bfa2df1957f19149fab9bdea2cbd1c582eb6ed4d6cbfd56545199e39"
    );
    assert_eq!(object.bytes.len(), 465);
    assert_eq!(object.object_class, codec::CLASS_FUNCTIONS);
    assert_eq!(
        object.digest.as_str(),
        format!("blake3-256:{}", object.address)
    );
    assert_eq!(
        object.decoded_length as usize + codec::HEADER_LEN,
        object.bytes.len()
    );
    // Read back by its file name through the codec: one record, lineage into the parent.
    let name = object.path.rsplit('/').next().unwrap();
    let report = codec::decode_named(name, &object.bytes);
    assert_eq!(
        report.verdict,
        CodecVerdict::Decoded,
        "{:?}",
        report.defects
    );
    let records = report.records.unwrap();
    assert_eq!(object.records, 1);
    let record = &records[0];
    assert_eq!(
        record.function_id,
        "fn:core/src/donor/mod.rs::MaterializationMode::is_local"
    );
    assert_eq!(record.lineage, [IS_LOCAL_IDENTITY, IS_LOCAL_SIGNATURE]);
    let parent_ids: BTreeSet<&str> = f
        .sealed
        .typed_records
        .iter()
        .map(|r| r.record_id().as_str())
        .collect();
    assert!(
        record
            .lineage
            .iter()
            .all(|l| parent_ids.contains(l.as_str()))
    );
    assert_eq!(check_lineage(&f.sealed, &f.design, &records), vec![]);
    assert_eq!(
        m.functions,
        [MaterializedFunction {
            function_id: record.function_id.clone(),
            lineage: record.lineage.clone(),
        }]
    );
    // The signature is the one the construction IR lifted for SR1-3 (G181) from an earlier
    // census of the same source: every field but the lineage, whose records are that census's.
    let module: ConstructionModule =
        serde_json::from_str(include_str!("../codec/fixture_module.json")).unwrap();
    let mut lifted = FunctionSignatureRecord::of(&module.functions[0]);
    lifted.lineage = record.lineage.clone();
    assert_eq!(*record, lifted);
    // Deterministic: the same inputs stage the same bytes.
    assert_eq!(run(&f, &f.sealed, Some(&f.design)), m);
    assert_eq!(
        run(&f, &f.sealed, Some(&f.design)).objects[0].bytes,
        object.bytes
    );
    // Its verdict is typed vocabulary on the wire.
    let text = serde_json::to_value(&m).unwrap();
    assert_eq!(text["verdict"], "STAGED");
    // G187: the manifest over the object, and the root identity it carries (pinned and
    // validated in `validate::tests`).
    let manifest = m.manifest.as_ref().expect("a manifest");
    let (_, read) = crate::atlasx::manifest::read_manifest(&manifest.bytes).unwrap();
    assert_eq!(Some(&read.root_id), m.root_id.as_ref());
    assert_eq!(read.objects.len(), 1);
    assert_eq!(read.objects[0].relative_path, object.path);
}

#[test]
fn a_parent_the_precondition_refuses_is_refused_before_any_object_exists() {
    let f = fixture();
    // Unsealed.
    let m = run(&f, &f.unsealed, Some(&f.design));
    assert_eq!(m.verdict, MaterializationVerdict::Refused);
    assert_eq!(kinds(&m), [D::ParentNotAdmitted]);
    assert_eq!(
        m.defects[0].1,
        "NOT_SEALED: seal status UNSEALED_CENSUS_CONTAINER"
    );
    assert!(m.objects.is_empty() && m.functions.is_empty());
    assert!(m.manifest.is_none() && m.root_id.is_none());
    assert_eq!(m.precondition.verdict, PreconditionVerdict::Refused);
    // No design.
    let m = run(&f, &f.sealed, None);
    assert_eq!(kinds(&m), [D::ParentNotAdmitted]);
    assert!(m.defects[0].1.starts_with("DESIGN_ABSENT: "));
    assert!(m.objects.is_empty());
    // The G179 self-certification: a record re-stamped to name another policy binds the
    // container and reads back SEALED, but the gate did not decide it.
    let mut forged = f.sealed.clone();
    let record = forged.seal.as_mut().unwrap();
    record.policy_id = "blake3-256:any-policy".into();
    record.seal_id = seal_identity(record);
    let m = run(&f, &forged, Some(&f.design));
    assert_eq!(kinds(&m), [D::ParentNotAdmitted]);
    assert_eq!(
        m.precondition.reasons[0].0,
        PreconditionRefusal::SealRecordNotDecided
    );
    assert!(m.defects[0].1.starts_with("SEAL_RECORD_NOT_DECIDED: "));
    assert!(m.objects.is_empty());
    // Bytes that are not a container at all.
    let m = materialize(&PreconditionInputs {
        parent: b"not a container",
        design: Some(&json(&f.design)),
        verification: &json(&f.verification),
        integrity: &json(&f.integrity),
        declaration: &f.declared.declaration(),
    });
    assert_eq!(kinds(&m), [D::ParentNotAdmitted]);
    assert!(m.defects[0].1.starts_with("UNREADABLE: parent: "));
    assert_eq!(serde_json::to_value(&m).unwrap()["verdict"], "REFUSED");
}

#[test]
fn lineage_that_does_not_map_back_to_the_parent_is_refused() {
    let f = fixture();
    let m = run(&f, &f.sealed, Some(&f.design));
    let records = codec::decode(&m.objects[0].bytes).records.unwrap();
    // A lineage entry re-pointed at a record the parent does not carry.
    let mut foreign = records.clone();
    foreign[0].lineage[1] = "semantic:FUNCTION_SIGNATURE:0000000000000000".into();
    foreign[0].canonicalize();
    assert_eq!(
        check_lineage(&f.sealed, &f.design, &foreign),
        [(
            D::LineageOutsideParent,
            "fn:core/src/donor/mod.rs::MaterializationMode::is_local: \
             semantic:FUNCTION_SIGNATURE:0000000000000000"
                .into()
        )]
    );
    // The same object against a parent that does not carry its records.
    let mut other = f.sealed.clone();
    other.typed_records.clear();
    let defects = check_lineage(&other, &f.design, &records);
    let kinds: Vec<_> = defects.iter().map(|(d, _)| *d).collect();
    assert_eq!(kinds, [D::LineageOutsideParent; 2]);
    // Against a design that did not select the function.
    let mut design = f.design.clone();
    design.roots = vec![root(SemanticDimension::Symbol, MODE_SYMBOL)];
    assert_eq!(
        check_lineage(&f.sealed, &design, &records),
        [(
            D::LineageNotSelected,
            "fn:core/src/donor/mod.rs::MaterializationMode::is_local".into()
        )]
    );
    // A record whose lineage is only its signature: in the parent, but not a selected root.
    let mut unrooted = records;
    unrooted[0].lineage = vec![IS_LOCAL_SIGNATURE.into()];
    assert_eq!(
        check_lineage(&f.sealed, &f.design, &unrooted),
        [(
            D::LineageNotSelected,
            "fn:core/src/donor/mod.rs::MaterializationMode::is_local".into()
        )]
    );
}

#[test]
fn a_selection_the_parent_cannot_answer_is_refused_with_its_defect() {
    let f = fixture();
    // A root that is not in the parent, and one that names a record under another dimension.
    let m = sealed_run(
        &f,
        &f.unsealed,
        vec![
            root(
                SemanticDimension::FunctionIdentity,
                "semantic:FUNCTION_IDENTITY:absent",
            ),
            root(SemanticDimension::FunctionIdentity, MODE_SYMBOL),
        ],
    );
    assert_eq!(m.verdict, MaterializationVerdict::Refused);
    assert_eq!(
        m.defects,
        [
            (
                D::RootUnresolved,
                "FUNCTION_IDENTITY:semantic:FUNCTION_IDENTITY:absent is not in the parent".into()
            ),
            (
                D::RootUnresolved,
                format!("FUNCTION_IDENTITY:{MODE_SYMBOL} is a SYMBOL record")
            ),
        ]
    );
    assert!(m.objects.is_empty());
    // No function selected; the roots no class carries are listed, sorted.
    let m = sealed_run(
        &f,
        &f.unsealed,
        vec![
            root(SemanticDimension::Symbol, MODE_SYMBOL),
            root(SemanticDimension::FunctionSignature, IS_LOCAL_SIGNATURE),
        ],
    );
    assert_eq!(kinds(&m), [D::NothingToMaterialize]);
    assert_eq!(
        m.not_materialized,
        [
            format!("FUNCTION_SIGNATURE:{IS_LOCAL_SIGNATURE}"),
            format!("SYMBOL:{MODE_SYMBOL}")
        ]
    );
    // A selected function whose signature the parent does not carry: never invented.
    let mut unsigned = f.unsealed.clone();
    unsigned
        .typed_records
        .retain(|r| r.record_id().as_str() != IS_LOCAL_SIGNATURE);
    let m = sealed_run(&f, &unsigned, fixture::roots());
    assert_eq!(
        m.defects,
        [(
            D::SignatureAbsent,
            format!("FUNCTION_IDENTITY:{IS_LOCAL_IDENTITY}")
        )]
    );
    // Two different signatures of one function: no winner is chosen.
    let mut conflicting = f.unsealed.clone();
    let second = conflicting
        .typed_records
        .iter()
        .find_map(|r| match r {
            SemanticObservation::FunctionSignature(s) => {
                let mut s = s.clone();
                s.record_id = crate::semantic::SemanticRecordId::new(
                    SemanticDimension::FunctionSignature,
                    "a second engine",
                );
                s.subject.visibility = "pub(crate)".into();
                Some(SemanticObservation::FunctionSignature(s))
            }
            _ => None,
        })
        .unwrap();
    conflicting.typed_records.push(second);
    conflicting.canonicalize();
    let m = sealed_run(&f, &conflicting, fixture::roots());
    assert_eq!(kinds(&m), [D::SignatureConflict]);
    assert!(
        m.defects[0].1.contains(IS_LOCAL_SIGNATURE),
        "{:?}",
        m.defects
    );
    // A declaration kind FUNCTIONS has no dispatch code for.
    let mut trait_method = f.unsealed.clone();
    for record in &mut trait_method.typed_records {
        match record {
            SemanticObservation::FunctionIdentity(h) => {
                h.subject.declaration_kind = FunctionDeclarationKind::TraitImplementationMethod;
            }
            SemanticObservation::FunctionSignature(h) => {
                h.subject.function.declaration_kind =
                    FunctionDeclarationKind::TraitImplementationMethod;
            }
            _ => {}
        }
    }
    let m = sealed_run(&f, &trait_method, fixture::roots());
    assert_eq!(
        m.defects,
        [(
            D::DispatchUnsupported,
            format!("FUNCTION_IDENTITY:{IS_LOCAL_IDENTITY}: TRAIT_IMPLEMENTATION_METHOD")
        )]
    );
}

#[test]
fn a_record_the_codec_refuses_is_refused_encoding_refused() {
    let f = fixture();
    // A body fingerprint that is not a 32-byte digest: the codec will not write it.
    let mut malformed = f.unsealed.clone();
    for record in &mut malformed.typed_records {
        if let SemanticObservation::FunctionSignature(h) = record {
            h.subject.body_fingerprint = Some("blake3-256:not-hex".into());
        }
    }
    let m = sealed_run(&f, &malformed, fixture::roots());
    assert_eq!(kinds(&m), [D::EncodingRefused], "{:?}", m.defects);
    assert!(
        m.defects[0].1.starts_with("MALFORMED_VALUE: "),
        "{:?}",
        m.defects
    );
    assert!(m.objects.is_empty());
}

/// The read-back check runs on what the codec wrote, not only on what the selection produced:
/// records whose lineage leaves the parent, or includes no selected root, are refused at staging.
#[test]
fn staging_refuses_records_whose_lineage_does_not_map_back() {
    let f = fixture();
    let m = run(&f, &f.sealed, Some(&f.design));
    let records = codec::decode(&m.objects[0].bytes).records.unwrap();
    let mut foreign = records.clone();
    foreign[0].lineage = vec!["semantic:FUNCTION_IDENTITY:0000000000000000".into()];
    let defects = stage(&f.sealed, &f.design, &foreign).expect_err("refused");
    let kinds: Vec<_> = defects.iter().map(|(d, _)| *d).collect();
    assert_eq!(kinds, [D::LineageOutsideParent, D::LineageNotSelected]);
    let (staged, decoded) = stage(&f.sealed, &f.design, &records).expect("staged");
    assert_eq!(decoded, records);
    assert_eq!(staged.bytes, m.objects[0].bytes);
}

/// G185 review N1: a root the design repeats selects once; two distinct functions the provisional
/// GLOBAL_ID cannot tell apart are a typed selection defect, not a codec refusal.
#[test]
fn repeated_roots_select_once_and_colliding_global_ids_are_refused() {
    let f = fixture();
    let mut roots = fixture::roots();
    roots.push(root(SemanticDimension::FunctionIdentity, IS_LOCAL_IDENTITY));
    let m = sealed_run(&f, &f.unsealed, roots);
    assert_eq!(m.verdict, MaterializationVerdict::Staged, "{:?}", m.defects);
    assert_eq!(m.objects[0].records, 1);
    assert_eq!(
        m.objects[0].bytes,
        run(&f, &f.sealed, Some(&f.design)).objects[0].bytes
    );
    // A second `is_local` of the same owner in the same file, one line down.
    let mut twin = f.unsealed.clone();
    let mut copies = Vec::new();
    for record in &twin.typed_records {
        match record {
            SemanticObservation::FunctionIdentity(h) => {
                let mut h = h.clone();
                h.subject.span.line += 1;
                h.record_id = crate::semantic::SemanticRecordId::new(
                    SemanticDimension::FunctionIdentity,
                    &h.subject.identity_key(),
                );
                copies.push(SemanticObservation::FunctionIdentity(h));
            }
            SemanticObservation::FunctionSignature(h) => {
                let mut h = h.clone();
                h.subject.function.span.line += 1;
                h.record_id = crate::semantic::SemanticRecordId::new(
                    SemanticDimension::FunctionSignature,
                    &h.subject.function.identity_key(),
                );
                copies.push(SemanticObservation::FunctionSignature(h));
            }
            _ => {}
        }
    }
    let twin_identity = copies[0].record_id().as_str().to_owned();
    twin.typed_records.extend(copies);
    twin.canonicalize();
    let m = sealed_run(
        &f,
        &twin,
        vec![
            root(SemanticDimension::FunctionIdentity, IS_LOCAL_IDENTITY),
            root(SemanticDimension::FunctionIdentity, &twin_identity),
        ],
    );
    assert_eq!(kinds(&m), [D::GlobalIdCollision], "{:?}", m.defects);
    assert!(
        m.defects[0]
            .1
            .starts_with("fn:core/src/donor/mod.rs::MaterializationMode::is_local: ")
    );
    assert!(m.objects.is_empty());
}
