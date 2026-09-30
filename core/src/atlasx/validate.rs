//! The AtlasX validator, first slice (G187, construction node M13, ADR 0099; `contracts/
//! ATLASX-BINARY-WIRE-FORMAT.md`, "Reader verification order"; `contracts/ATLAS-TO-ATLASX.md`,
//! "Validation").
//!
//! It judges one root -- the files of a staging directory, as the runtime read them with bounded
//! reads -- against its parent, in the contract's reader order:
//!
//! 1. manifest header, magic and version; bounds, codec and hash; framing and schema
//!    (`manifest::read_manifest`): `MANIFEST_ABSENT`, `MANIFEST_REFUSED`. A manifest that does
//!    not read stops validation: nothing else it names can be trusted.
//! 2. the AtlasX root identity, recomputed (`ROOT_ID_MISMATCH`).
//! 3. parent compatibility. Validation requires the parent's inputs: the precondition is re-run
//!    over them (the G185 admission, with its re-run seal gate), and a parent it does not admit
//!    is `PARENT_NOT_ADMITTED` with each of its reasons. The manifest's parent binding (parent
//!    root, Genome hash, census digest, revision, seal, design, materialized scope) must be the
//!    admitted parent's (`PARENT_MISMATCH`); its materializer, version, schema, target kind and
//!    compiler contract must be the ones this materializer writes (`SCHEMA_INCOMPATIBLE`).
//! 4. object-entry paths: exactly the canonical `<class directory>/<decoded hash hex>.atlasx`
//!    (`ENTRY_PATH_INVALID`: traversal, absolute, or not canonical), of a class this validator
//!    reads (`ENTRY_CLASS_UNSUPPORTED`: FUNCTIONS only). Paths are never opened: they are looked
//!    up among the files the runtime listed.
//! 5. presence: every required entry's file exists as a regular file (`OBJECT_MISSING`), and no
//!    entry of the root is outside the manifest (`UNLISTED_FILE`): a file, a directory holding no
//!    listed object (an empty `types/`, a `.git/`), a name that is not UTF-8. The contract lets
//!    unlisted debug files coexist; this validator refuses them (ADR 0099), so a staged root holds
//!    exactly its canonical files and their class directories.
//! 6. each object's header, bounds, decoded hash, record and field framing, schema and canonical
//!    form, under its address name (`codec::decode_named`): `OBJECT_REFUSED`; and the entry must
//!    describe it: schema version, decoded length, record count (`ENTRY_MISMATCH`).
//! 7. identity uniqueness across objects (`DUPLICATE_IDENTITY`), and lineage closure against the
//!    admitted parent (`materialize::check_lineage`): `LINEAGE_OUTSIDE_PARENT`,
//!    `LINEAGE_NOT_SELECTED`.
//!
//! 8. reproduction, last, and only over a root every earlier step accepts (G187 review): the
//!    admitted parent is materialized again in memory (`materialize_over`, reusing the admission)
//!    and the root must be exactly that root -- every record, every object's bytes and the
//!    manifest's bytes (`ROOT_NOT_REPRODUCED`, naming the first differing record field, record,
//!    object or the manifest). A record edited beyond its lineage and re-rooted is refused here.
//!
//! FUNCTIONS has no cross-object reference beyond lineage: owners and parameter and result type
//! spellings are carried as text, not references. What VALID does not claim is listed in every
//! verdict (`VALIDATE_NOT_VERIFIED`). The validator is a pure function of the bytes it is given.

use super::codec::{self, CodecVerdict, FunctionSignatureRecord};
use super::manifest::{self, MANIFEST_FILE, ObjectEntry};
use super::materialize::{
    FUNCTIONS_DIRECTORY, Materialization, MaterializationDefect, MaterializationVerdict,
    check_lineage, manifest_for, materialize_over,
};
use super::precondition::{Precondition, PreconditionInputs, evaluate};
use crate::identity::IntegrityDigest;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const VALIDATION_SCHEMA_VERSION: &str = "atlas.atlasx-validation.v1";

/// What a VALID root does not prove; listed in every verdict.
pub const VALIDATE_NOT_VERIFIED: &[&str] = &[
    "the precondition steps the embedded precondition verdict lists (Genome compatibility among them: the manifest's genome_hash is the parent's, carried, not judged)",
    "dynamic/external boundary validity, semantic barrier presence and profiles/bindings: the manifest carries none (tags 11-13, 15, 16 are undeclared and refused)",
    "compiler-contract compatibility: the compiler IR contract is UNKNOWN and the target kind NONE",
    "cross-object references beyond lineage: owner names and parameter and result type spellings are text, not resolved",
    "that the materializer's own mapping from census records to FUNCTION_SIGNATURE fields is right: VALID means the root is byte for byte the one materializer version 1 writes for the admitted parent (reproduced in memory), not that the mapping was checked independently",
    "that the root is the design's complete closure: the materialized scope is the design's FUNCTION_IDENTITY roots (RES-G185-CLOSURE-ROOTS-ONLY)",
    "function bodies: FUNCTION_SIGNATURE carries none",
    "every object class but FUNCTIONS: an entry of another class is refused, never validated",
    "publication: a staged root is validated, not published or advertised",
];

crate::vocabulary_enum! {
    /// Whether an AtlasX root is valid (M13).
    pub enum ValidationVerdict {
        Valid => "VALID",
        Invalid => "INVALID",
    }
}

crate::vocabulary_enum! {
    /// Why an AtlasX root is invalid, in the reader's verification order.
    pub enum ValidationDefect {
        /// The root holds more entries or bytes than the runtime reads.
        RootLimit => "ROOT_LIMIT",
        /// No `manifest.atlasx` regular file.
        ManifestAbsent => "MANIFEST_ABSENT",
        /// The manifest's header, bounds, hash, framing, schema or canonical form is refused.
        ManifestRefused => "MANIFEST_REFUSED",
        /// The root id the manifest carries is not the one its fields give.
        RootIdMismatch => "ROOT_ID_MISMATCH",
        /// The precondition does not admit the parent the root is validated against.
        ParentNotAdmitted => "PARENT_NOT_ADMITTED",
        /// The manifest's parent binding is not the admitted parent's.
        ParentMismatch => "PARENT_MISMATCH",
        /// The manifest names another materializer, schema, target kind or compiler contract.
        SchemaIncompatible => "SCHEMA_INCOMPATIBLE",
        /// An entry's path is not the canonical path of its class and hash.
        EntryPathInvalid => "ENTRY_PATH_INVALID",
        /// An entry of a class this validator does not read.
        EntryClassUnsupported => "ENTRY_CLASS_UNSUPPORTED",
        /// A required entry's file is absent or not a regular file.
        ObjectMissing => "OBJECT_MISSING",
        /// A file of the root the manifest does not list.
        UnlistedFile => "UNLISTED_FILE",
        /// The object codec refuses a listed object.
        ObjectRefused => "OBJECT_REFUSED",
        /// An entry does not describe its object.
        EntryMismatch => "ENTRY_MISMATCH",
        /// Two records of the root with one identity.
        DuplicateIdentity => "DUPLICATE_IDENTITY",
        /// A record's lineage names a record the parent does not carry.
        LineageOutsideParent => "LINEAGE_OUTSIDE_PARENT",
        /// A record's lineage includes no root the design selected.
        LineageNotSelected => "LINEAGE_NOT_SELECTED",
        /// G187 (review): the root is not exactly the one the materializer writes, in memory, for
        /// the admitted parent: a record field, a record, an object or the manifest differs.
        RootNotReproduced => "ROOT_NOT_REPRODUCED",
    }
}

pub type ValidationDefects = Vec<(ValidationDefect, String)>;

/// One entry of a root directory, as the runtime read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootEntry {
    /// A regular file, with at most the runtime's read limit of its bytes.
    File(Vec<u8>),
    /// A link, a directory where a file belongs, or anything else that is not a regular file.
    NotAFile,
    /// A top-level directory, read; it must hold a listed object.
    Directory,
    /// An entry whose name is not UTF-8 (listed under its lossy spelling): no manifest lists it.
    NameNotUtf8,
}

/// The entries of one root directory by relative path (`/`-separated): its files, its top-level
/// directories, and whatever else it holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RootFiles {
    pub entries: BTreeMap<String, RootEntry>,
    /// Why the runtime stopped reading the root, when it did.
    pub over_limit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Validation {
    pub schema: String,
    pub verdict: ValidationVerdict,
    /// Every defect found, typed, with its detail; sorted.
    pub defects: ValidationDefects,
    /// Only when VALID: the AtlasX root identity.
    pub root_id: Option<IntegrityDigest>,
    /// The precondition's verdict on the parent inputs; absent when the manifest does not read.
    pub precondition: Option<Precondition>,
    /// The objects validated, by path.
    pub objects: Vec<String>,
    pub not_verified: Vec<String>,
}

/// The canonical directory of a class this validator reads.
fn directory_of(class: u16) -> Option<&'static str> {
    (class == codec::CLASS_FUNCTIONS).then_some(FUNCTIONS_DIRECTORY)
}

/// Why `path` is not the canonical path of `entry`, if it is not.
fn path_defect(entry: &ObjectEntry) -> Option<(ValidationDefect, String)> {
    use ValidationDefect as V;
    let path = &entry.relative_path;
    let traversal = path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..");
    if traversal {
        return Some((
            V::EntryPathInvalid,
            format!("`{path}`: absolute, empty or `.`/`..` segment"),
        ));
    }
    let Some(directory) = directory_of(entry.object_class) else {
        return Some((
            V::EntryClassUnsupported,
            format!("`{path}`: class {}", entry.object_class),
        ));
    };
    let hex = &entry.decoded_content_hash.as_str()[IntegrityDigest::BLAKE3_256_PREFIX.len()..];
    let canonical = format!("{directory}/{hex}.atlasx");
    (*path != canonical).then(|| {
        (
            V::EntryPathInvalid,
            format!("`{path}` is not `{canonical}`"),
        )
    })
}

fn finish(
    defects: ValidationDefects,
    root_id: Option<IntegrityDigest>,
    precondition: Option<Precondition>,
    objects: Vec<String>,
) -> Validation {
    let mut defects = defects;
    defects.sort();
    defects.dedup();
    let valid = defects.is_empty();
    Validation {
        schema: VALIDATION_SCHEMA_VERSION.into(),
        verdict: if valid {
            ValidationVerdict::Valid
        } else {
            ValidationVerdict::Invalid
        },
        defects,
        root_id: root_id.filter(|_| valid),
        precondition,
        objects,
        not_verified: VALIDATE_NOT_VERIFIED
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
    }
}

/// M13 (FUNCTIONS only): whether `root` is a valid AtlasX root of the parent in `parent`.
pub fn validate(root: &RootFiles, parent: &PreconditionInputs) -> Validation {
    use ValidationDefect as V;
    let mut defects = ValidationDefects::new();
    if let Some(limit) = &root.over_limit {
        defects.push((V::RootLimit, limit.clone()));
        return finish(defects, None, None, Vec::new());
    }
    // 1. The manifest: header, bounds, hash, framing, schema, canonical form.
    let Some(RootEntry::File(bytes)) = root.entries.get(MANIFEST_FILE) else {
        defects.push((V::ManifestAbsent, format!("no {MANIFEST_FILE} file")));
        return finish(defects, None, None, Vec::new());
    };
    let refused = |found: codec::Defects| -> ValidationDefects {
        found
            .into_iter()
            .map(|(d, detail)| (V::ManifestRefused, format!("{d}: {detail}")))
            .collect()
    };
    let read = match manifest::read_manifest(bytes) {
        Ok((_, read)) => read,
        Err(found) => return finish(refused(found), None, None, Vec::new()),
    };
    // 2. The root identity, recomputed.
    let recomputed = manifest::root_identity(&read).ok();
    if recomputed.as_ref() != Some(&read.root_id) {
        defects.push((
            V::RootIdMismatch,
            format!(
                "the manifest carries {}, its fields give {}",
                read.root_id,
                recomputed.map_or_else(|| "none".into(), String::from)
            ),
        ));
    }
    // 3. The parent: admitted, and the one the manifest binds.
    let (precondition, admission) = evaluate(parent);
    // `evaluate` gives the admitted identities and the decoded parent only when it admits.
    let admission = match (&precondition.admitted, admission) {
        (Some(admitted), Some(admission)) => {
            match manifest_for(&admission, admitted, read.objects.clone()) {
                Ok(expected) => defects.extend(binding_defects(&expected, &read)),
                Err(found) => defects.extend(refused(found)),
            }
            Some(admission)
        }
        _ => {
            for (reason, detail) in &precondition.reasons {
                defects.push((V::ParentNotAdmitted, format!("{reason}: {detail}")));
            }
            if precondition.reasons.is_empty() {
                defects.push((V::ParentNotAdmitted, "the parent is not admitted".into()));
            }
            None
        }
    };
    // 4-6. Entry paths, presence, every object through the codec.
    let listed: BTreeSet<&str> = read
        .objects
        .iter()
        .map(|e| e.relative_path.as_str())
        .collect();
    // Whether the directory `dir` holds a listed object.
    let holds_listed = |dir: &str| {
        listed.iter().any(|l| {
            l.strip_prefix(dir)
                .is_some_and(|rest| rest.starts_with('/'))
        })
    };
    for (path, entry) in &root.entries {
        let unlisted = match entry {
            RootEntry::NameNotUtf8 => Some(format!("{path}: a name that is not UTF-8")),
            RootEntry::Directory if holds_listed(path) => None,
            RootEntry::Directory => Some(format!("{path}/: a directory holding no listed object")),
            _ if path == MANIFEST_FILE || listed.contains(path.as_str()) => None,
            _ => Some(path.clone()),
        };
        defects.extend(unlisted.map(|detail| (V::UnlistedFile, detail)));
    }
    let mut records: Vec<FunctionSignatureRecord> = Vec::new();
    let mut objects = Vec::new();
    for entry in &read.objects {
        if let Some(found) = path_defect(entry) {
            defects.push(found);
            continue;
        }
        let path = &entry.relative_path;
        let bytes = match root.entries.get(path) {
            Some(RootEntry::File(bytes)) => bytes,
            Some(_) => {
                defects.push((V::ObjectMissing, format!("{path}: not a regular file")));
                continue;
            }
            None if entry.required => {
                defects.push((V::ObjectMissing, path.clone()));
                continue;
            }
            None => continue,
        };
        let name = path.rsplit('/').next().unwrap_or_default();
        let report = codec::decode_named(name, bytes);
        let (CodecVerdict::Decoded, Some(decoded)) = (report.verdict, report.records) else {
            for (d, detail) in report.defects {
                defects.push((V::ObjectRefused, format!("{path}: {d}: {detail}")));
            }
            continue;
        };
        for (what, listed, found) in [
            (
                "object_schema_version",
                u64::from(entry.object_schema_version),
                report.object_schema_version.map(u64::from),
            ),
            (
                "decoded_length",
                entry.decoded_length,
                report.decoded_length,
            ),
            (
                "logical_record_count",
                entry.logical_record_count,
                Some(decoded.len() as u64),
            ),
        ] {
            if Some(listed) != found {
                defects.push((
                    V::EntryMismatch,
                    format!("{path}: {what} listed {listed}, found {found:?}"),
                ));
            }
        }
        objects.push(path.clone());
        records.extend(decoded);
    }
    // 7. Identity uniqueness across objects, then lineage closure against the parent.
    let mut seen = BTreeSet::new();
    for record in &records {
        if !seen.insert(record.function_id.as_str()) {
            defects.push((V::DuplicateIdentity, record.function_id.clone()));
        }
    }
    if let Some(admission) = &admission {
        for (defect, detail) in check_lineage(&admission.atlas, &admission.design, &records) {
            let kind = match defect {
                MaterializationDefect::LineageOutsideParent => V::LineageOutsideParent,
                _ => V::LineageNotSelected,
            };
            defects.push((kind, detail));
        }
    }
    // 8. Reproduction, last and only over a root every earlier step accepts: the admitted parent
    // is materialized again, in memory, and the root must be exactly that root.
    if let (true, Some(admission)) = (defects.is_empty(), &admission) {
        let expected = materialize_over(precondition.clone(), Some(admission));
        defects.extend(reproduction_defect(root, &records, &expected));
    }
    finish(defects, Some(read.root_id), Some(precondition), objects)
}

/// The first field in which `found` is not `want`, by the record's field classes.
fn differing_field(
    found: &FunctionSignatureRecord,
    want: &FunctionSignatureRecord,
) -> &'static str {
    [
        ("name", found.name != want.name),
        ("owner", found.owner != want.owner),
        ("dispatch", found.dispatch != want.dispatch),
        ("visibility", found.visibility != want.visibility),
        ("documentation", found.documentation != want.documentation),
        ("params", found.params != want.params),
        ("result", found.result != want.result),
        (
            "body_fingerprint",
            found.body_fingerprint != want.body_fingerprint,
        ),
        ("lineage", found.lineage != want.lineage),
    ]
    .into_iter()
    .find(|(_, differs)| *differs)
    .map_or("record", |(name, _)| name)
}

/// G187 (review): the first way `root`, whose decoded records are `records`, is not the root the
/// materializer writes for the admitted parent (`expected`): a record (naming its first differing
/// field), a record missing, an object's bytes, then the manifest's bytes.
fn reproduction_defect(
    root: &RootFiles,
    records: &[FunctionSignatureRecord],
    expected: &Materialization,
) -> Option<(ValidationDefect, String)> {
    let not = |detail: String| Some((ValidationDefect::RootNotReproduced, detail));
    let (MaterializationVerdict::Staged, Some(manifest), Some(root_id)) =
        (expected.verdict, &expected.manifest, &expected.root_id)
    else {
        return not(format!(
            "the admitted parent does not materialize: {:?}",
            expected.defects
        ));
    };
    let want: Vec<FunctionSignatureRecord> = expected
        .objects
        .iter()
        .flat_map(|o| codec::decode(&o.bytes).records.unwrap_or_default())
        .collect();
    for record in records {
        let id = &record.function_id;
        match want.iter().find(|w| w.function_id == *id) {
            None => return not(format!("{id}: not a function the parent materializes")),
            Some(w) if w != record => {
                return not(format!(
                    "{id}: `{}` is not what the parent's census records give",
                    differing_field(record, w)
                ));
            }
            Some(_) => {}
        }
    }
    if let Some(w) = want
        .iter()
        .find(|w| !records.iter().any(|r| r.function_id == w.function_id))
    {
        return not(format!(
            "{}: materialized from the parent, absent from the root",
            w.function_id
        ));
    }
    let holds = |path: &str, bytes: &[u8]| matches!(root.entries.get(path), Some(RootEntry::File(found)) if found.as_slice() == bytes);
    for object in &expected.objects {
        if !holds(&object.path, &object.bytes) {
            return not(format!(
                "{}: not the object the parent materializes",
                object.path
            ));
        }
    }
    if !holds(MANIFEST_FILE, &manifest.bytes) {
        return not(format!(
            "{MANIFEST_FILE}: not the manifest the parent materializes (root {root_id})"
        ));
    }
    None
}

/// Every field of `read` that is not what this materializer writes for the admitted parent
/// (`expected`): the parent binding, then the materializer's own fields.
fn binding_defects(
    expected: &manifest::AtlasxManifest,
    read: &manifest::AtlasxManifest,
) -> ValidationDefects {
    use ValidationDefect as V;
    let pairs: [(ValidationDefect, &str, &str, &str); 12] = [
        (
            V::ParentMismatch,
            "parent_root",
            expected.parent_root.as_str(),
            read.parent_root.as_str(),
        ),
        (
            V::ParentMismatch,
            "genome_hash",
            expected.genome_hash.as_str(),
            read.genome_hash.as_str(),
        ),
        (
            V::ParentMismatch,
            "census_digest",
            expected.census_digest.as_str(),
            read.census_digest.as_str(),
        ),
        (
            V::ParentMismatch,
            "revision",
            &expected.revision,
            &read.revision,
        ),
        (
            V::ParentMismatch,
            "seal_id",
            &expected.seal_id,
            &read.seal_id,
        ),
        (
            V::ParentMismatch,
            "design_id",
            &expected.design_id,
            &read.design_id,
        ),
        (
            V::ParentMismatch,
            "scope_id",
            &expected.scope_id,
            &read.scope_id,
        ),
        (
            V::SchemaIncompatible,
            "materializer",
            &expected.materializer,
            &read.materializer,
        ),
        (
            V::SchemaIncompatible,
            "materializer_version",
            &expected.materializer_version,
            &read.materializer_version,
        ),
        (
            V::SchemaIncompatible,
            "materialization_schema",
            &expected.materialization_schema,
            &read.materialization_schema,
        ),
        (
            V::SchemaIncompatible,
            "target_kind",
            &expected.target_kind,
            &read.target_kind,
        ),
        (
            V::SchemaIncompatible,
            "compiler_ir_contract",
            &expected.compiler_ir_contract,
            &read.compiler_ir_contract,
        ),
    ];
    pairs
        .into_iter()
        .filter(|(_, _, want, got)| want != got)
        .map(|(defect, what, want, got)| {
            (
                defect,
                format!("{what}: the manifest names {got}, expected {want}"),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests;
