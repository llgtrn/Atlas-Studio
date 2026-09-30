//! M13 (FUNCTIONS only) falsified on the G161 fixture: the materialized root validates end to end
//! with its root identity pinned; each tampering -- a flipped object or manifest byte, a missing,
//! unlisted or misnamed object, a traversal path, lineage outside the parent, a manifest naming
//! another parent, seal or design, a manifest edited without its root id recomputed -- is INVALID
//! with its typed defect; a root validated against a parent the precondition refuses is INVALID;
//! arbitrary bytes are refused, never a panic.

use super::*;
use crate::atlas::CensusAtlas;
use crate::atlas::write;
use crate::atlasx::codec::{HEADER_LEN, encode_functions};
use crate::atlasx::fixture::{self, Fixture, IS_LOCAL_IDENTITY, fixture};
use crate::atlasx::manifest::{AtlasxManifest, encode_manifest, read_manifest, root_identity};
use crate::atlasx::materialize::{Materialization, MaterializationVerdict, materialize};
use crate::atlasx::precondition::PreconditionVerdict;
use crate::design::{SelectedDesign, SemanticRoot};
use crate::semantic::SemanticDimension;

use ValidationDefect as V;

/// The AtlasX root identity of the G161 fixture's materialized root. Pinned: a change of the
/// manifest schema, the root identity rule, the parent binding or any object shows here.
const FIXTURE_ROOT_ID: &str =
    "blake3-256:7d9e6cf960320931703ad3d9b767af1f188ea866ac2185096c19c349ed463393";

fn json<T: serde::Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

/// `run` over the parent `parent` and `design`, with the fixture's reports and declarations.
fn with_parent<T>(
    f: &Fixture,
    parent: &CensusAtlas,
    design: &SelectedDesign,
    run: impl FnOnce(&PreconditionInputs) -> T,
) -> T {
    let (parent, design) = (write(parent).unwrap(), json(design));
    let (verification, integrity) = (json(&f.verification), json(&f.integrity));
    let declaration = f.declared.declaration();
    run(&PreconditionInputs {
        parent: &parent,
        design: Some(&design),
        verification: &verification,
        integrity: &integrity,
        declaration: &declaration,
    })
}

/// The files a staged materialization leaves: its objects, then its manifest.
fn files_of(m: &Materialization) -> RootFiles {
    assert_eq!(m.verdict, MaterializationVerdict::Staged, "{:?}", m.defects);
    let mut root = RootFiles::default();
    for object in m.objects.iter().chain(m.manifest.as_ref()) {
        root.entries
            .insert(object.path.clone(), RootEntry::File(object.bytes.clone()));
        // The class directory each object is in, as the runtime lists it.
        if let Some((directory, _)) = object.path.split_once('/') {
            root.entries.insert(directory.into(), RootEntry::Directory);
        }
    }
    root
}

/// The G161 fixture's staged root.
fn staged(f: &Fixture) -> (Materialization, RootFiles) {
    let m = with_parent(f, &f.sealed, &f.design, materialize);
    let root = files_of(&m);
    (m, root)
}

/// `root` validated against the G161 parent.
fn check(f: &Fixture, root: &RootFiles) -> Validation {
    with_parent(f, &f.sealed, &f.design, |inputs| validate(root, inputs))
}

fn kinds(v: &Validation) -> Vec<ValidationDefect> {
    v.defects.iter().map(|(d, _)| *d).collect()
}

fn manifest_of(root: &RootFiles) -> AtlasxManifest {
    let Some(RootEntry::File(bytes)) = root.entries.get(MANIFEST_FILE) else {
        panic!("no manifest");
    };
    read_manifest(bytes).unwrap().1
}

/// `root` with its manifest replaced by `manifest`, the root id recomputed when `reroot`.
fn with_manifest(root: &RootFiles, mut manifest: AtlasxManifest, reroot: bool) -> RootFiles {
    if reroot {
        manifest.root_id = root_identity(&manifest).unwrap();
    }
    let mut root = root.clone();
    root.entries.insert(
        MANIFEST_FILE.into(),
        RootEntry::File(encode_manifest(&manifest).unwrap()),
    );
    root
}

fn object_path(root: &RootFiles) -> String {
    manifest_of(root).objects[0].relative_path.clone()
}

fn hex_of(digest: &IntegrityDigest) -> &str {
    &digest.as_str()[IntegrityDigest::BLAKE3_256_PREFIX.len()..]
}

#[test]
fn the_materialized_g161_root_is_valid_with_its_root_identity_pinned() {
    let f = fixture();
    let (m, root) = staged(&f);
    let v = check(&f, &root);
    assert_eq!(v.verdict, ValidationVerdict::Valid, "{:?}", v.defects);
    assert!(v.defects.is_empty());
    assert_eq!(v.schema, VALIDATION_SCHEMA_VERSION);
    assert_eq!(v.not_verified, VALIDATE_NOT_VERIFIED);
    assert_eq!(
        v.root_id.as_ref().map(|r| r.as_str()),
        Some(FIXTURE_ROOT_ID)
    );
    assert_eq!(v.root_id, m.root_id);
    assert_eq!(v.objects, [m.objects[0].path.clone()]);
    let precondition = v.precondition.as_ref().unwrap();
    assert_eq!(precondition.verdict, PreconditionVerdict::Admitted);
    // The manifest binds the parent the precondition admitted, canonically.
    let manifest = manifest_of(&root);
    let admitted = precondition.admitted.as_ref().unwrap();
    assert_eq!(manifest.root_id, *m.root_id.as_ref().unwrap());
    assert_eq!(manifest.parent_root.as_str(), admitted.parent_root);
    assert_eq!(manifest.census_digest.as_str(), admitted.census_digest);
    assert_eq!(manifest.revision, admitted.revision);
    assert_eq!(manifest.seal_id, admitted.seal_id);
    assert_eq!(manifest.design_id, f.design.design_id);
    assert_eq!(manifest.genome_hash, IntegrityDigest::blake3_256(&[7; 32]));
    assert_eq!(manifest.scope_id, "fixture/FUNCTIONS/design-function-roots");
    assert_eq!(manifest.target_kind, "NONE");
    assert_eq!(manifest.compiler_ir_contract, "UNKNOWN");
    let object = &m.objects[0];
    assert_eq!(
        manifest.objects,
        [ObjectEntry {
            relative_path: object.path.clone(),
            object_class: codec::CLASS_FUNCTIONS,
            object_schema_version: codec::FUNCTIONS_SCHEMA_VERSION,
            decoded_content_hash: object.digest.clone(),
            decoded_length: object.decoded_length,
            required: true,
            logical_record_count: 1,
        }]
    );
    let file = m.manifest.as_ref().unwrap();
    assert_eq!(file.path, MANIFEST_FILE);
    assert_eq!(file.bytes.len(), 834);
    assert_eq!(file.object_class, manifest::CLASS_ROOT_MANIFEST);
    // Deterministic: materialized again, the same root.
    let (again, _) = staged(&f);
    assert_eq!(again.root_id, m.root_id);
    assert_eq!(again.manifest, m.manifest);
    let text = serde_json::to_value(&v).unwrap();
    assert_eq!(text["verdict"], "VALID");
}

#[test]
fn a_flipped_byte_in_an_object_or_the_manifest_is_invalid() {
    let f = fixture();
    let (_, root) = staged(&f);
    let path = object_path(&root);
    let RootEntry::File(object) = root.entries[&path].clone() else {
        panic!()
    };
    // A payload byte of the object: its digest no longer authenticates it.
    let mut tampered = root.clone();
    let mut flipped = object.clone();
    flipped[HEADER_LEN + 10] ^= 0x01;
    tampered
        .entries
        .insert(path.clone(), RootEntry::File(flipped));
    let v = check(&f, &tampered);
    assert_eq!(v.verdict, ValidationVerdict::Invalid);
    assert_eq!(kinds(&v), [V::ObjectRefused]);
    assert!(
        v.defects[0].1.contains("DIGEST_MISMATCH"),
        "{:?}",
        v.defects
    );
    assert_eq!(v.root_id, None);
    // Every byte of the object, flipped, is refused.
    for at in 0..object.len() {
        let mut flipped = object.clone();
        flipped[at] ^= 0x80;
        let mut tampered = root.clone();
        tampered
            .entries
            .insert(path.clone(), RootEntry::File(flipped));
        assert_eq!(
            kinds(&check_fast(&f, &tampered)),
            [V::ObjectRefused],
            "{at}"
        );
    }
    // A payload byte of the manifest.
    let RootEntry::File(bytes) = root.entries[MANIFEST_FILE].clone() else {
        panic!()
    };
    let mut flipped = bytes.clone();
    flipped[HEADER_LEN + 20] ^= 0x01;
    let mut tampered = root.clone();
    tampered
        .entries
        .insert(MANIFEST_FILE.into(), RootEntry::File(flipped));
    let v = check(&f, &tampered);
    assert_eq!(kinds(&v), [V::ManifestRefused]);
    assert!(
        v.defects[0].1.starts_with("DIGEST_MISMATCH: "),
        "{:?}",
        v.defects
    );
    // A manifest that does not read stops validation: the parent is not even judged.
    assert_eq!(v.precondition, None);
    for at in 0..bytes.len() {
        let mut flipped = bytes.clone();
        flipped[at] ^= 0x80;
        let mut tampered = root.clone();
        tampered
            .entries
            .insert(MANIFEST_FILE.into(), RootEntry::File(flipped));
        assert_eq!(
            kinds(&check_fast(&f, &tampered)),
            [V::ManifestRefused],
            "{at}"
        );
    }
}

/// `validate` against a parent that does not decode, its PARENT_NOT_ADMITTED dropped: only for
/// defects found without the parent (the manifest and the objects), so the gate is not re-run
/// for each of hundreds of cases.
fn check_fast(f: &Fixture, root: &RootFiles) -> Validation {
    let declaration = f.declared.declaration();
    let mut v = validate(
        root,
        &PreconditionInputs {
            parent: b"not a container",
            design: None,
            verification: b"{}",
            integrity: b"{}",
            declaration: &declaration,
        },
    );
    v.defects.retain(|(d, _)| *d != V::ParentNotAdmitted);
    v
}

#[test]
fn a_missing_unlisted_or_misnamed_object_is_invalid() {
    let f = fixture();
    let (_, root) = staged(&f);
    let path = object_path(&root);
    // Missing.
    let mut missing = root.clone();
    missing.entries.remove(&path);
    let v = check(&f, &missing);
    assert_eq!(kinds(&v), [V::ObjectMissing]);
    assert_eq!(v.defects[0].1, path);
    // Not a regular file where the object belongs.
    let mut link = root.clone();
    link.entries.insert(path.clone(), RootEntry::NotAFile);
    assert_eq!(kinds(&check(&f, &link)), [V::ObjectMissing]);
    // Listed as not required, and absent: nothing missing -- but the materializer writes every
    // object required, so the root is not the one it reproduces for the parent; present, the same.
    let mut optional = manifest_of(&root);
    optional.objects[0].required = false;
    let present = with_manifest(&root, optional, true);
    let mut absent = present.clone();
    absent.entries.remove(&path);
    for root in [&absent, &present] {
        let v = check(&f, root);
        assert_eq!(kinds(&v), [V::RootNotReproduced]);
    }
    let v = check(&f, &absent);
    assert!(
        v.defects[0].1.ends_with("absent from the root"),
        "{:?}",
        v.defects
    );
    let v = check(&f, &present);
    assert!(
        v.defects[0]
            .1
            .starts_with("manifest.atlasx: not the manifest the parent materializes"),
        "{:?}",
        v.defects
    );
    // An extra file the manifest does not list, anywhere in the root: refused.
    for extra in ["functions/extra.atlasx", "notes.md", "types/x.atlasx"] {
        let mut unlisted = root.clone();
        unlisted
            .entries
            .insert(extra.into(), RootEntry::File(b"debug".to_vec()));
        let v = check(&f, &unlisted);
        assert_eq!(kinds(&v), [V::UnlistedFile], "{extra}");
        assert_eq!(v.defects[0].1, extra);
    }
    // A directory holding no listed object -- an empty `types/`, a `.git/` -- and a name that is
    // not UTF-8: refused, so a staged root holds exactly its canonical files.
    for (extra, entry, detail) in [
        (
            "types",
            RootEntry::Directory,
            "types/: a directory holding no listed object",
        ),
        (
            ".git",
            RootEntry::Directory,
            ".git/: a directory holding no listed object",
        ),
        // A prefix of the class directory's name is another directory.
        (
            "func",
            RootEntry::Directory,
            "func/: a directory holding no listed object",
        ),
        (
            "odd\u{fffd}",
            RootEntry::NameNotUtf8,
            "odd\u{fffd}: a name that is not UTF-8",
        ),
        (
            "functions/odd\u{fffd}",
            RootEntry::NameNotUtf8,
            "functions/odd\u{fffd}: a name that is not UTF-8",
        ),
    ] {
        let mut unlisted = root.clone();
        unlisted.entries.insert(extra.into(), entry);
        let v = check(&f, &unlisted);
        assert_eq!(v.defects, [(V::UnlistedFile, detail.to_owned())], "{extra}");
    }
    // The class directory holding the listed object is part of the root.
    assert_eq!(root.entries["functions"], RootEntry::Directory);
    // No manifest at all, or a manifest that is not a file.
    let mut bare = root.clone();
    bare.entries.remove(MANIFEST_FILE);
    let v = check(&f, &bare);
    assert_eq!(kinds(&v), [V::ManifestAbsent]);
    assert_eq!(v.precondition, None);
    bare.entries
        .insert(MANIFEST_FILE.into(), RootEntry::NotAFile);
    assert_eq!(kinds(&check(&f, &bare)), [V::ManifestAbsent]);
    // The runtime stopped reading the root.
    let mut limited = root.clone();
    limited.over_limit = Some("more than 1024 entries".into());
    let v = check(&f, &limited);
    assert_eq!(
        v.defects,
        [(V::RootLimit, "more than 1024 entries".to_owned())]
    );
    // An object whose file name is not its address: moved to another name the manifest lists
    // consistently (path and hash agree), the codec refuses it under that name.
    let RootEntry::File(object) = root.entries[&path].clone() else {
        panic!()
    };
    let other = IntegrityDigest::blake3_256(&[0xab; 32]);
    let renamed_path = format!("functions/{}.atlasx", hex_of(&other));
    let mut manifest = manifest_of(&root);
    manifest.objects[0].relative_path = renamed_path.clone();
    manifest.objects[0].decoded_content_hash = other;
    let mut renamed = with_manifest(&root, manifest, true);
    renamed.entries.remove(&path);
    renamed
        .entries
        .insert(renamed_path, RootEntry::File(object.clone()));
    let v = check(&f, &renamed);
    assert_eq!(kinds(&v), [V::ObjectRefused]);
    assert!(
        v.defects[0].1.contains("ADDRESS_MISMATCH"),
        "{:?}",
        v.defects
    );
    // An entry whose path names another hash than the entry's own: not canonical, and the object
    // at its canonical name is then unlisted.
    let mut manifest = manifest_of(&root);
    manifest.objects[0].relative_path = format!(
        "functions/{}.atlasx",
        hex_of(&IntegrityDigest::blake3_256(&[1; 32]))
    );
    let v = check(&f, &with_manifest(&root, manifest, true));
    assert_eq!(kinds(&v), [V::EntryPathInvalid, V::UnlistedFile]);
    assert!(
        v.defects[0].1.contains("is not `functions/"),
        "{:?}",
        v.defects
    );
}

#[test]
fn an_entry_path_outside_the_canonical_mapping_is_invalid() {
    let f = fixture();
    let (_, root) = staged(&f);
    let path = object_path(&root);
    for bad in [
        format!("../{path}"),
        format!("/{path}"),
        format!("functions/../{path}"),
        format!("./{path}"),
        format!("functions//{}", &path["functions/".len()..]),
        path.replace('/', "\\"),
        format!("types/{}", &path["functions/".len()..]),
        format!("{path}.bak"),
    ] {
        let mut manifest = manifest_of(&root);
        manifest.objects[0].relative_path = bad.clone();
        let v = check(&f, &with_manifest(&root, manifest, true));
        // The file at the canonical path is then unlisted, and so is `functions/` when no
        // listed path is under it; the bad path is never opened.
        let dir_unlisted = !bad.starts_with("functions/");
        let mut expected = vec![V::EntryPathInvalid, V::UnlistedFile];
        if dir_unlisted {
            expected.push(V::UnlistedFile);
        }
        assert_eq!(kinds(&v), expected, "{bad}");
        assert!(v.defects[0].1.contains(&bad), "{:?}", v.defects);
        assert!(
            v.defects.contains(&(V::UnlistedFile, path.clone())),
            "{bad}"
        );
        assert_eq!(
            v.defects.contains(&(
                V::UnlistedFile,
                "functions/: a directory holding no listed object".into()
            )),
            dir_unlisted,
            "{bad}"
        );
    }
    // The traversal detail names the rule it breaks.
    let mut manifest = manifest_of(&root);
    manifest.objects[0].relative_path = format!("../{path}");
    let v = check(&f, &with_manifest(&root, manifest, true));
    assert!(
        v.defects[0]
            .1
            .ends_with("absolute, empty or `.`/`..` segment")
    );
    // A class this validator does not read, at the canonical path of FUNCTIONS: not validated.
    let mut manifest = manifest_of(&root);
    manifest.objects[0].object_class = 3;
    let v = check(&f, &with_manifest(&root, manifest, true));
    assert_eq!(kinds(&v), [V::EntryClassUnsupported]);
    let mut manifest = manifest_of(&root);
    manifest.objects[0].object_class = manifest::CLASS_ROOT_MANIFEST;
    let v = check(&f, &with_manifest(&root, manifest, true));
    assert_eq!(kinds(&v), [V::EntryClassUnsupported]);
}

#[test]
fn an_entry_that_does_not_describe_its_object_is_invalid() {
    let f = fixture();
    let (_, root) = staged(&f);
    for (edit, what) in [
        (
            (|e: &mut ObjectEntry| e.decoded_length += 1) as fn(&mut ObjectEntry),
            "decoded_length",
        ),
        (|e| e.logical_record_count = 2, "logical_record_count"),
        (|e| e.object_schema_version = 2, "object_schema_version"),
    ] {
        let mut manifest = manifest_of(&root);
        edit(&mut manifest.objects[0]);
        let v = check(&f, &with_manifest(&root, manifest, true));
        assert_eq!(kinds(&v), [V::EntryMismatch], "{what}");
        assert!(v.defects[0].1.contains(what), "{:?}", v.defects);
    }
}

/// `root` with its one object replaced by the FUNCTIONS object of `records`, listed by a
/// manifest re-rooted over it.
fn with_records(root: &RootFiles, records: &[FunctionSignatureRecord]) -> RootFiles {
    with_object(root, encode_functions(records).unwrap(), records.len())
}

/// `root` with its one object replaced by `bytes` (holding `count` records), listed at its
/// address with its hash and length by a manifest re-rooted over it.
fn with_object(root: &RootFiles, bytes: Vec<u8>, count: usize) -> RootFiles {
    let header = codec::ObjectHeader::parse(&bytes).unwrap();
    let mut manifest = manifest_of(root);
    let old = manifest.objects[0].relative_path.clone();
    let entry = &mut manifest.objects[0];
    entry.relative_path = format!("functions/{}.atlasx", header.address());
    entry.decoded_content_hash = header.digest();
    entry.decoded_length = header.decoded_length;
    entry.logical_record_count = count as u64;
    let path = entry.relative_path.clone();
    let mut root = with_manifest(root, manifest, true);
    root.entries.remove(&old);
    root.entries.insert(path, RootEntry::File(bytes));
    root
}

fn records_of(root: &RootFiles) -> Vec<FunctionSignatureRecord> {
    let RootEntry::File(bytes) = &root.entries[&object_path(root)] else {
        panic!()
    };
    codec::decode(bytes).records.unwrap()
}

#[test]
fn lineage_outside_the_parent_and_duplicate_identities_are_invalid() {
    let f = fixture();
    let (_, root) = staged(&f);
    let records = records_of(&root);
    // Re-materialized as is: the same root.
    assert_eq!(
        check(&f, &with_records(&root, &records)).verdict,
        ValidationVerdict::Valid
    );
    // A lineage entry outside the parent, the object and manifest otherwise canonical.
    let mut foreign = records.clone();
    foreign[0].lineage[1] = "semantic:FUNCTION_SIGNATURE:0000000000000000".into();
    let v = check(&f, &with_records(&root, &foreign));
    assert_eq!(kinds(&v), [V::LineageOutsideParent]);
    assert!(
        v.defects[0]
            .1
            .ends_with("semantic:FUNCTION_SIGNATURE:0000000000000000")
    );
    // Lineage without the selected root.
    let mut unselected = records.clone();
    unselected[0].lineage.retain(|l| l != IS_LOCAL_IDENTITY);
    assert_eq!(
        kinds(&check(&f, &with_records(&root, &unselected))),
        [V::LineageNotSelected]
    );
    // Two objects carrying one function identity.
    let mut twin = records.clone();
    twin[0].documentation = Some("another".into());
    let bytes = encode_functions(&twin).unwrap();
    let report = codec::decode(&bytes);
    let mut manifest = manifest_of(&root);
    let mut entry = manifest.objects[0].clone();
    entry.relative_path = format!("functions/{}.atlasx", report.address.unwrap());
    entry.decoded_content_hash = report.digest.unwrap();
    entry.decoded_length = report.decoded_length.unwrap();
    manifest.objects.push(entry.clone());
    let mut doubled = with_manifest(&root, manifest, true);
    doubled
        .entries
        .insert(entry.relative_path, RootEntry::File(bytes));
    let v = check(&f, &doubled);
    assert_eq!(kinds(&v), [V::DuplicateIdentity]);
    assert_eq!(v.defects[0].1, records[0].function_id);
}

#[test]
fn a_manifest_naming_another_parent_or_edited_without_its_root_id_is_invalid() {
    let f = fixture();
    let (_, root) = staged(&f);
    let other = IntegrityDigest::blake3_256(&[0xcd; 32]);
    // Each parent binding field changed alone, the root id recomputed.
    for (edit, what) in [
        (
            (|m: &mut AtlasxManifest| m.seal_id = "seal:another".into()) as fn(&mut AtlasxManifest),
            "seal_id",
        ),
        (|m| m.design_id = "design:another".into(), "design_id"),
        (
            |m| m.parent_root = IntegrityDigest::blake3_256(&[0xcd; 32]),
            "parent_root",
        ),
        (
            |m| m.census_digest = IntegrityDigest::blake3_256(&[0xcd; 32]),
            "census_digest",
        ),
        (
            |m| m.genome_hash = IntegrityDigest::blake3_256(&[0xcd; 32]),
            "genome_hash",
        ),
        (|m| m.revision = "git:another".into(), "revision"),
        (|m| m.scope_id = "another".into(), "scope_id"),
    ] {
        let mut manifest = manifest_of(&root);
        edit(&mut manifest);
        let v = check(&f, &with_manifest(&root, manifest, true));
        assert_eq!(kinds(&v), [V::ParentMismatch], "{what}");
        assert!(
            v.defects[0].1.starts_with(&format!("{what}: ")),
            "{:?}",
            v.defects
        );
    }
    // Another materializer, schema, target kind or compiler contract.
    for (edit, what) in [
        (
            (|m: &mut AtlasxManifest| m.materializer = "another".into()) as fn(&mut AtlasxManifest),
            "materializer",
        ),
        (
            |m| m.materializer_version = "2".into(),
            "materializer_version",
        ),
        (
            |m| m.materialization_schema = "v2".into(),
            "materialization_schema",
        ),
        (|m| m.target_kind = "LIBRARY_PACKAGE".into(), "target_kind"),
        (
            |m| m.compiler_ir_contract = "v1".into(),
            "compiler_ir_contract",
        ),
    ] {
        let mut manifest = manifest_of(&root);
        edit(&mut manifest);
        let v = check(&f, &with_manifest(&root, manifest, true));
        assert_eq!(kinds(&v), [V::SchemaIncompatible], "{what}");
        assert!(
            v.defects[0].1.starts_with(&format!("{what}: ")),
            "{:?}",
            v.defects
        );
    }
    // Edited and re-hashed (the codec accepts it), its root id not recomputed.
    let mut manifest = manifest_of(&root);
    manifest.root_id = other;
    let v = check(&f, &with_manifest(&root, manifest, false));
    assert_eq!(kinds(&v), [V::RootIdMismatch]);
    let mut manifest = manifest_of(&root);
    manifest.seal_id = "seal:another".into();
    let v = check(&f, &with_manifest(&root, manifest, false));
    assert_eq!(kinds(&v), [V::RootIdMismatch, V::ParentMismatch]);
}

#[test]
fn a_root_of_a_parent_not_admitted_or_of_another_parent_is_invalid() {
    let f = fixture();
    let (_, root) = staged(&f);
    // The same root against the unsealed parent: the precondition refuses it.
    let v = with_parent(&f, &f.unsealed, &f.design, |inputs| validate(&root, inputs));
    assert_eq!(kinds(&v), [V::ParentNotAdmitted]);
    assert_eq!(
        v.defects[0].1,
        "NOT_SEALED: seal status UNSEALED_CENSUS_CONTAINER"
    );
    assert_eq!(
        v.precondition.as_ref().unwrap().verdict,
        PreconditionVerdict::Refused
    );
    // The G179 self-certified seal: refused by the re-run gate.
    let mut forged = f.sealed.clone();
    let record = forged.seal.as_mut().unwrap();
    record.policy_id = "blake3-256:any-policy".into();
    record.seal_id = crate::seal::seal_identity(record);
    let v = with_parent(&f, &forged, &f.design, |inputs| validate(&root, inputs));
    assert_eq!(kinds(&v), [V::ParentNotAdmitted]);
    assert!(v.defects[0].1.starts_with("SEAL_RECORD_NOT_DECIDED: "));
    // Lineage is not judged without an admitted parent, but everything else still is.
    let mut missing = root.clone();
    missing.entries.remove(&object_path(&root));
    let v = with_parent(&f, &f.unsealed, &f.design, |inputs| {
        validate(&missing, inputs)
    });
    assert_eq!(kinds(&v), [V::ParentNotAdmitted, V::ObjectMissing]);
    // A root materialized from another admitted parent -- the same container sealed for a design
    // selecting only the function -- is valid against its own parent, not the G161 one.
    let design = fixture::design_over(
        &f.unsealed,
        vec![SemanticRoot {
            dimension: SemanticDimension::FunctionIdentity,
            record_id: IS_LOCAL_IDENTITY.into(),
        }],
    );
    let sealed = fixture::seal(&f.unsealed, &design, &f).expect("ELIGIBLE");
    let own = files_of(&with_parent(&f, &sealed, &design, materialize));
    let v = with_parent(&f, &sealed, &design, |inputs| validate(&own, inputs));
    assert_eq!(v.verdict, ValidationVerdict::Valid, "{:?}", v.defects);
    assert_ne!(
        v.root_id.as_ref().map(|r| r.as_str()),
        Some(FIXTURE_ROOT_ID)
    );
    let v = check(&f, &own);
    // Sealed for another design, it is another container: its root, seal and design differ.
    assert_eq!(kinds(&v), [V::ParentMismatch; 3]);
    assert!(v.defects[0].1.starts_with("design_id: "));
    assert!(v.defects[1].1.starts_with("parent_root: "));
    assert!(v.defects[2].1.starts_with("seal_id: "));
}

/// G187 review (was RES-G187-VALID-NOT-REPRODUCED): a record edited beyond its lineage, in any
/// field class, re-encoded canonically and re-rooted, passes every structural and lineage check
/// -- and is INVALID, because the root is not the one the materializer reproduces for the
/// admitted parent. Non-canonical edits are refused earlier by the codec, and another
/// materializer version by the manifest's binding.
#[test]
fn a_record_edited_beyond_its_lineage_is_not_reproduced() {
    use crate::construction::{Dispatch, IrParam};
    let f = fixture();
    let (_, root) = staged(&f);
    let records = records_of(&root);
    type Edit = fn(&mut FunctionSignatureRecord);
    let cases: [(&str, Edit); 10] = [
        ("name", |r| r.name = "is_remote".into()),
        ("owner", |r| r.owner = None),
        ("dispatch", |r| r.dispatch = Dispatch::AssociatedFunction),
        ("visibility", |r| r.visibility = "pub(crate)".into()),
        ("documentation", |r| r.documentation = Some("edited".into())),
        ("params", |r| {
            r.params.push(IrParam {
                name: "x".into(),
                type_spelling: "u8".into(),
            })
        }),
        ("result", |r| r.result = Some("u8".into())),
        ("body_fingerprint", |r| {
            r.body_fingerprint = Some(IntegrityDigest::blake3_256(&[3; 32]).into())
        }),
        // Another census record of the parent, beside the selected root: the lineage still
        // resolves, but is not the lineage the parent gives.
        ("lineage", |r| r.lineage.push(fixture::MODE_SYMBOL.into())),
        ("record", |r| r.function_id.push('x')),
    ];
    for (field, edit) in cases {
        let mut edited = records.clone();
        edit(&mut edited[0]);
        let v = check(&f, &with_records(&root, &edited));
        assert_eq!(
            kinds(&v),
            [V::RootNotReproduced],
            "{field}: {:?}",
            v.defects
        );
        assert_eq!(v.root_id, None);
        let detail = &v.defects[0].1;
        if field == "record" {
            assert!(
                detail.ends_with("not a function the parent materializes"),
                "{detail}"
            );
        } else {
            assert!(
                detail.contains(&format!("`{field}` is not what")),
                "{detail}"
            );
        }
    }
    // The lineage order: a set, sorted -- the reversed order, framed and re-hashed, is not the
    // canonical form, and the codec refuses the object.
    let RootEntry::File(bytes) = root.entries[&object_path(&root)].clone() else {
        panic!()
    };
    let reordered = with_object(&root, reverse_lineage(&bytes), 1);
    let v = check(&f, &reordered);
    assert_eq!(kinds(&v), [V::ObjectRefused]);
    assert!(v.defects[0].1.contains("NON_CANONICAL"), "{:?}", v.defects);
    // Another materializer version, re-rooted: refused by the binding before reproduction.
    let mut manifest = manifest_of(&root);
    manifest.materializer_version = "2".into();
    let v = check(&f, &with_manifest(&root, manifest, true));
    assert_eq!(kinds(&v), [V::SchemaIncompatible]);
    // Unedited, re-encoded and re-rooted: the same bytes, VALID.
    assert_eq!(
        check(&f, &with_records(&root, &records)).verdict,
        ValidationVerdict::Valid
    );
    assert!(
        !VALIDATE_NOT_VERIFIED
            .iter()
            .any(|s| s.contains("only lineage is resolved"))
    );
}

/// The FUNCTIONS object `bytes` (one record, lineage last) with its lineage records reversed,
/// re-hashed.
fn reverse_lineage(bytes: &[u8]) -> Vec<u8> {
    use crate::atlas::{u32_at, u64_at};
    let payload = &bytes[HEADER_LEN..];
    let record = &payload[codec::RECORD_HEADER_LEN..];
    // The fields, in order; the lineage (tag 10) is the last.
    let mut at = 0;
    let mut last = 0;
    while at < record.len() {
        last = at;
        at += codec::FIELD_HEADER_LEN + u32_at(record, at + 4) as usize;
    }
    let content = &record[last + codec::FIELD_HEADER_LEN..];
    let mut refs = Vec::new();
    let mut at = 0;
    while at < content.len() {
        let end = at + codec::RECORD_HEADER_LEN + u64_at(content, at + 8) as usize;
        refs.push(content[at..end].to_vec());
        at = end;
    }
    assert_eq!(refs.len(), 2);
    refs.reverse();
    let mut out_payload =
        payload[..codec::RECORD_HEADER_LEN + last + codec::FIELD_HEADER_LEN].to_vec();
    out_payload.extend(refs.concat());
    codec::object(codec::ObjectHeader::functions(&out_payload), &out_payload)
}

/// xorshift64*: deterministic pseudo-random bytes, no dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

#[test]
fn arbitrary_roots_are_invalid_never_a_panic() {
    let f = fixture();
    let (_, root) = staged(&f);
    let RootEntry::File(manifest) = root.entries[MANIFEST_FILE].clone() else {
        panic!()
    };
    let path = object_path(&root);
    let mut rng = Rng(0x1319_8a2e_0370_7344);
    // One garbage parent: the gate is never reached, so each round is cheap.
    let declaration = f.declared.declaration();
    let parent = PreconditionInputs {
        parent: b"not a container",
        design: None,
        verification: b"{}",
        integrity: b"{}",
        declaration: &declaration,
    };
    for round in 0..600 {
        let mut garbage = root.clone();
        let target = if round % 2 == 0 {
            MANIFEST_FILE
        } else {
            path.as_str()
        };
        let RootEntry::File(mut bytes) = garbage.entries[target].clone() else {
            panic!()
        };
        if round % 3 == 0 {
            bytes = (0..rng.below(400)).map(|_| rng.next() as u8).collect();
        } else {
            for _ in 0..1 + rng.below(4) {
                let at = rng.below(bytes.len().max(1));
                match rng.below(3) {
                    0 if !bytes.is_empty() => bytes[at] = rng.next() as u8,
                    1 if !bytes.is_empty() => {
                        bytes.remove(at);
                    }
                    _ => bytes.insert(at.min(bytes.len()), rng.next() as u8),
                }
            }
        }
        garbage
            .entries
            .insert(target.into(), RootEntry::File(bytes));
        if round % 5 == 0 {
            let name: String = (0..rng.below(12))
                .map(|_| char::from(b'.' + (rng.next() % 64) as u8))
                .collect();
            garbage.entries.insert(name, RootEntry::NotAFile);
        }
        let v = validate(&garbage, &parent);
        assert_eq!(v.verdict, ValidationVerdict::Invalid, "{round}");
        assert!(!v.defects.is_empty());
        assert_eq!(v.root_id, None);
    }
    // The manifest bytes themselves are intact here: only the parent is refused.
    let v = validate(&root, &parent);
    assert!(
        kinds(&v).iter().all(|d| *d == V::ParentNotAdmitted),
        "{:?}",
        v.defects
    );
    assert!(!manifest.is_empty());
}
