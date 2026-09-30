//! The AtlasX root manifest, first slice (G187, construction node M13, ADR 0099; `contracts/
//! ATLASX-BINARY-WIRE-FORMAT.md`, "Root manifest object", "ObjectEntry", "AtlasX root identity").
//!
//! `manifest.atlasx` is one ROOT_MANIFEST (class 1) object, written and read through the object
//! codec's framing: the same 80-byte header, record and tagged-field framing, BLAKE3 decoded hash
//! and canonical re-encode rule as FUNCTIONS, under its own schema table (`MANIFEST_SCHEMA`,
//! version `MANIFEST_SCHEMA_VERSION`, pinned by `schema_digest`). It holds exactly one
//! ROOT_MANIFEST record:
//! - tags 1 to 10 as the contract numbers them: the root id, the parent Atlas root, the parent's
//!   Genome hash, the SelectedDesign, the materialized scope, the target kind, the materializer's
//!   identity and version, the materialization schema and the compiler IR contract;
//! - tag 14, one OBJECT_ENTRY per object (path, class, schema, decoded hash, decoded length,
//!   required flag, record count), sorted by class, then decoded hash, then path;
//! - tags 17 to 19, beyond the contract's 16: the parent's census digest, revision and seal id,
//!   so that the parent binding the precondition admitted is canonical, not only in the JSON
//!   result (ADR 0099 declares the extension).
//!
//! The repeated tags 11 (profiles), 12 (external bindings), 13 (semantic barriers), 15 (dynamic
//! obligations) and 16 (compatibility requirements) are not declared: this slice materializes
//! none, an absent repeated field is an empty one, and a manifest carrying one is refused as an
//! undeclared field, never read as understood.
//!
//! The AtlasX root identity is the BLAKE3 (the digest the header declares) of the canonical
//! ROOT_MANIFEST record payload without field 1: every other field, framed, in ascending tag
//! order. Field 1 then carries those 32 bytes. A reader recomputes it (`root_identity`).

use super::codec::{
    self, CodecDefect, Defects, ObjectHeader, RecordDef, RecordWriter, declared, def_in,
    digest_bytes, frames, payload_of, wire,
};
use crate::identity::IntegrityDigest;
use serde::{Deserialize, Serialize};

/// The ROOT_MANIFEST object class.
pub const CLASS_ROOT_MANIFEST: u16 = 1;
/// The object schema of ROOT_MANIFEST objects: the table `MANIFEST_SCHEMA`.
pub const MANIFEST_SCHEMA_VERSION: u16 = 1;
/// The only canonical file not named by its content (`ATLASX-BINARY-WIRE-FORMAT.md`).
pub const MANIFEST_FILE: &str = "manifest.atlasx";

/// Record kinds of the ROOT_MANIFEST schema.
pub const ROOT_MANIFEST: u16 = 1;
/// Embedded only: one canonical object of the root.
pub const OBJECT_ENTRY: u16 = 2;

const fn field(name: &'static str, tag: u16, wire: u8) -> codec::FieldDef {
    codec::FieldDef {
        name,
        tag,
        wire,
        required: true,
        nested: 0,
    }
}

/// The ROOT_MANIFEST object schema, version `MANIFEST_SCHEMA_VERSION`. Every declared field is
/// required.
pub const MANIFEST_SCHEMA: &[RecordDef] = &[
    RecordDef {
        kind: ROOT_MANIFEST,
        name: "root-manifest",
        top_level: true,
        fields: &[
            field("atlasx_root_id", 1, wire::HASH32),
            field("parent_atlas_root_id", 2, wire::HASH32),
            field("genome_hash", 3, wire::HASH32),
            field("selected_design_id", 4, wire::GLOBAL_ID),
            field("materialized_scope_id", 5, wire::UTF8),
            field("target_kind", 6, wire::UTF8),
            field("materializer_identity", 7, wire::UTF8),
            field("materializer_version", 8, wire::UTF8),
            field("materialization_schema_version", 9, wire::UTF8),
            field("compiler_ir_contract_version", 10, wire::UTF8),
            codec::FieldDef {
                nested: OBJECT_ENTRY,
                ..field("object_entry", 14, wire::RECORD)
            },
            field("parent_census_digest", 17, wire::HASH32),
            field("parent_revision", 18, wire::UTF8),
            field("parent_seal_id", 19, wire::GLOBAL_ID),
        ],
    },
    RecordDef {
        kind: OBJECT_ENTRY,
        name: "object-entry",
        top_level: false,
        fields: &[
            field("relative_path", 1, wire::UTF8),
            field("object_class", 2, wire::UVARINT),
            field("object_schema_version", 3, wire::UVARINT),
            field("decoded_content_hash", 4, wire::HASH32),
            field("decoded_length", 5, wire::UVARINT),
            field("required", 6, wire::BOOL),
            field("logical_record_count", 7, wire::UVARINT),
        ],
    },
];

/// The digest of the ROOT_MANIFEST table's definition; a test pins it to its version.
pub fn schema_digest() -> IntegrityDigest {
    codec::schema_digest_of(
        CLASS_ROOT_MANIFEST,
        "ROOT_MANIFEST",
        MANIFEST_SCHEMA_VERSION,
        MANIFEST_SCHEMA,
    )
}

/// One canonical object of the root, as the manifest lists it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObjectEntry {
    /// `<class directory>/<decoded-content-hash-hex>.atlasx`, relative to the root.
    pub relative_path: String,
    pub object_class: u16,
    pub object_schema_version: u16,
    pub decoded_content_hash: IntegrityDigest,
    pub decoded_length: u64,
    pub required: bool,
    pub logical_record_count: u64,
}

impl ObjectEntry {
    /// The contract's order: class, then decoded hash, then path.
    fn key(&self) -> (u16, &str, &str) {
        (
            self.object_class,
            self.decoded_content_hash.as_str(),
            &self.relative_path,
        )
    }
}

/// The ROOT_MANIFEST record, decoded.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtlasxManifest {
    /// Tag 1: the AtlasX root identity (`root_identity`).
    pub root_id: IntegrityDigest,
    pub parent_root: IntegrityDigest,
    /// The parent container's Genome hash, carried; Genome compatibility is not verified.
    pub genome_hash: IntegrityDigest,
    pub design_id: String,
    pub scope_id: String,
    pub target_kind: String,
    pub materializer: String,
    pub materializer_version: String,
    pub materialization_schema: String,
    pub compiler_ir_contract: String,
    /// Sorted by the contract's order.
    pub objects: Vec<ObjectEntry>,
    pub census_digest: IntegrityDigest,
    pub revision: String,
    pub seal_id: String,
}

/// The ROOT_MANIFEST record of `manifest`, its root id field included only when `with_root`.
/// Refuses an empty identity or text field, an object list that is empty, and two entries with
/// one path.
fn record(manifest: &AtlasxManifest, with_root: bool) -> Result<RecordWriter, Defects> {
    use CodecDefect as D;
    let mut w = RecordWriter::of(MANIFEST_SCHEMA, ROOT_MANIFEST);
    if with_root {
        w.put("atlasx_root_id", &digest_bytes(&manifest.root_id))?;
    }
    w.put("parent_atlas_root_id", &digest_bytes(&manifest.parent_root))?;
    w.put("genome_hash", &digest_bytes(&manifest.genome_hash))?;
    w.put(
        "parent_census_digest",
        &digest_bytes(&manifest.census_digest),
    )?;
    for (name, value) in [
        ("selected_design_id", &manifest.design_id),
        ("materialized_scope_id", &manifest.scope_id),
        ("target_kind", &manifest.target_kind),
        ("materializer_identity", &manifest.materializer),
        ("materializer_version", &manifest.materializer_version),
        (
            "materialization_schema_version",
            &manifest.materialization_schema,
        ),
        (
            "compiler_ir_contract_version",
            &manifest.compiler_ir_contract,
        ),
        ("parent_revision", &manifest.revision),
        ("parent_seal_id", &manifest.seal_id),
    ] {
        if value.is_empty() {
            return codec::defect(
                D::MalformedValue,
                format!("root-manifest: an empty `{name}`"),
            );
        }
        w.text(name, value)?;
    }
    if manifest.objects.is_empty() {
        return codec::defect(D::MissingRequiredField, "root-manifest: no object entry");
    }
    let mut objects: Vec<&ObjectEntry> = manifest.objects.iter().collect();
    objects.sort_by(|a, b| a.key().cmp(&b.key()));
    let mut paths: Vec<&str> = objects.iter().map(|o| o.relative_path.as_str()).collect();
    paths.sort_unstable();
    if let Some(pair) = paths.windows(2).find(|pair| pair[0] == pair[1]) {
        return codec::defect(
            D::DuplicateIdentity,
            format!("two entries for `{}`", pair[0]),
        );
    }
    let mut entries = Vec::with_capacity(objects.len());
    for object in objects {
        if object.relative_path.is_empty() {
            return codec::defect(D::MalformedValue, "object-entry: an empty `relative_path`");
        }
        let mut e = RecordWriter::of(MANIFEST_SCHEMA, OBJECT_ENTRY);
        e.text("relative_path", &object.relative_path)?;
        e.uvarint("object_class", object.object_class.into())?;
        e.uvarint("object_schema_version", object.object_schema_version.into())?;
        e.put(
            "decoded_content_hash",
            &digest_bytes(&object.decoded_content_hash),
        )?;
        e.uvarint("decoded_length", object.decoded_length)?;
        e.put("required", &[u8::from(object.required)])?;
        e.uvarint("logical_record_count", object.logical_record_count)?;
        entries.push(e);
    }
    w.embedded("object_entry", entries)?;
    Ok(w)
}

/// The AtlasX root identity of `manifest`: BLAKE3 of its canonical ROOT_MANIFEST record payload
/// with field 1 omitted. The root id `manifest` carries takes no part.
pub fn root_identity(manifest: &AtlasxManifest) -> Result<IntegrityDigest, Defects> {
    let fields = record(manifest, false)?.payload();
    Ok(IntegrityDigest::of_bytes(&fields))
}

/// The canonical bytes of `manifest.atlasx` for `manifest`, header included, carrying the root id
/// `manifest` names (a writer sets it to `root_identity` first).
pub fn encode_manifest(manifest: &AtlasxManifest) -> Result<Vec<u8>, Defects> {
    // No length limit is checked here: a manifest over `MAX_DECODED_LENGTH` is refused when it is
    // read back, which the materializer does before staging it.
    let mut payload = Vec::new();
    record(manifest, true)?.frame(&mut payload);
    let header = ObjectHeader::of(CLASS_ROOT_MANIFEST, MANIFEST_SCHEMA_VERSION, &payload);
    Ok(codec::object(header, &payload))
}

/// A UVARINT field that must fit a u16 (a class or schema version).
fn narrow(value: u64, what: &str) -> Result<u16, Defects> {
    u16::try_from(value).or_else(|_| {
        codec::defect(
            CodecDefect::MalformedValue,
            format!("object-entry: `{what}` {value} is not a u16"),
        )
    })
}

/// The ROOT_MANIFEST object in `bytes`, verified in the contract's order for one object (header,
/// bounds, decoded hash, framing, schema, canonical form), with its header. It must hold exactly
/// one ROOT_MANIFEST record. The root id it carries is returned as read, not recomputed.
pub fn read_manifest(bytes: &[u8]) -> Result<(ObjectHeader, AtlasxManifest), Defects> {
    use CodecDefect as D;
    let (header, payload) = payload_of(
        bytes,
        CLASS_ROOT_MANIFEST,
        "ROOT_MANIFEST",
        MANIFEST_SCHEMA_VERSION,
    )?;
    let frames = frames(payload, 1)?;
    let frame = match frames.as_slice() {
        [frame] => frame,
        [] => return codec::defect(D::EmptyObject, "no root-manifest record"),
        more => {
            return codec::defect(
                D::DuplicateIdentity,
                format!(
                    "{} top-level records, exactly one root-manifest",
                    more.len()
                ),
            );
        }
    };
    let Some(def) = def_in(MANIFEST_SCHEMA, frame.kind).filter(|d| d.top_level) else {
        return codec::defect(
            D::UndeclaredRecordKind,
            format!("top-level record kind {}", frame.kind),
        );
    };
    let r = declared(frame, def)?;
    let digest = |name: &str| r.hash(name).map(|h| IntegrityDigest::blake3_256(&h));
    let mut objects = Vec::new();
    for frame in r.embedded("object_entry") {
        let e = declared(
            frame,
            def_in(MANIFEST_SCHEMA, OBJECT_ENTRY).expect("declared"),
        )?;
        objects.push(ObjectEntry {
            relative_path: e.text("relative_path")?,
            object_class: narrow(e.uvarint("object_class")?, "object_class")?,
            object_schema_version: narrow(
                e.uvarint("object_schema_version")?,
                "object_schema_version",
            )?,
            decoded_content_hash: IntegrityDigest::blake3_256(&e.hash("decoded_content_hash")?),
            decoded_length: e.uvarint("decoded_length")?,
            required: e.boolean("required")?,
            logical_record_count: e.uvarint("logical_record_count")?,
        });
    }
    let manifest = AtlasxManifest {
        root_id: digest("atlasx_root_id")?,
        parent_root: digest("parent_atlas_root_id")?,
        genome_hash: digest("genome_hash")?,
        design_id: r.text("selected_design_id")?,
        scope_id: r.text("materialized_scope_id")?,
        target_kind: r.text("target_kind")?,
        materializer: r.text("materializer_identity")?,
        materializer_version: r.text("materializer_version")?,
        materialization_schema: r.text("materialization_schema_version")?,
        compiler_ir_contract: r.text("compiler_ir_contract_version")?,
        objects,
        census_digest: digest("parent_census_digest")?,
        revision: r.text("parent_revision")?,
        seal_id: r.text("parent_seal_id")?,
    };
    // Canonical form: the writer's bytes for what was read, so entry order, an empty field and a
    // repeated path are refused by the writer's own rules.
    if encode_manifest(&manifest)? != bytes {
        return codec::defect(
            D::NonCanonical,
            "not the canonical encoding of its manifest (object entry order)",
        );
    }
    Ok((header, manifest))
}

#[cfg(test)]
mod tests;
