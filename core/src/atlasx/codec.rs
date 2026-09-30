//! The AtlasX object codec (G183, construction node M12, ADR 0095; `contracts/
//! ATLASX-BINARY-WIRE-FORMAT.md`).
//!
//! One object class exists: FUNCTIONS (4), with one top-level record kind, FUNCTION_SIGNATURE. A
//! record is one construction-IR function signature (`construction::IrFunction`): its identity,
//! name, owner, dispatch, visibility, documentation, parameters, result, body fingerprint and
//! census lineage. The function's HIR body is not a field of this record kind and is not encoded.
//!
//! The writer is canonical. The same logical object always gives the same bytes:
//! - the 80-byte header: magic `ATLSX\0\1\0`, major 1, minor 0, no flags, class FUNCTIONS, object
//!   schema 1, digest BLAKE3_256, codec NONE, `encoded_length == decoded_length`, reserved 0;
//! - `decoded_content_hash` is Atlas's own BLAKE3 (`identity::blake3`) of the payload;
//! - records are ordered by their stable identity bytes (the function id). The contract's
//!   further keys, record kind and record-content hash, never decide: the object has one
//!   top-level kind and refuses two records with one identity;
//! - fields ascend by tag, and each field's tag, wire type and required flag (`field_flags` bit 0)
//!   come only from the schema table (`FUNCTIONS_SCHEMA`);
//! - parameters keep their declared order; lineage is a set, sorted bytewise and deduplicated;
//!   an empty optional sequence is an absent field.
//!
//! The address of an object is the lowercase hex of its `decoded_content_hash`, the canonical
//! filename rule (`<address>.atlasx`).
//!
//! The reader follows the contract's verification order for one object: header, bounds, decoded
//! hash, record framing, field framing, class/schema constraints. It then re-encodes what it
//! decoded and requires the same bytes, so no non-canonical form is accepted, and the writer's own
//! refusals (an empty object, a repeated or empty identity) apply to what is read. Every refusal is a
//! typed `CodecDefect`. Arbitrary input is refused, never a panic. No buffer is sized from a length
//! or count read from the input, and nesting is bounded (`MAX_DEPTH`).
//!
//! Unknown fields: the contract lets a reader skip an unknown optional field and requires it to
//! reject an unknown required one. Minor 0 is the only minor, and the reader refuses any other, so
//! no field outside the table can be a compatible extension. The reader therefore refuses every
//! undeclared field: UNDECLARED_REQUIRED_FIELD when its `field_flags` mark it required,
//! UNDECLARED_OPTIONAL_FIELD otherwise.
//!
//! This is the codec only (M12). What a decoded object does not prove is listed in every report
//! (`CODEC_NOT_VERIFIED`): the manifest and root identity, cross-object references, lineage to the
//! parent Atlas, and every other class. The materializer (M11) and the validator (M13) do not exist.

use crate::atlas::{read_uvarint, u16_at, u32_at, u64_at, uvarint};
use crate::construction::{
    ConstructionInputKind, ConstructionModule, Dispatch, IrFunction, IrParam, module_identity,
    validate_module,
};
use crate::identity::{IntegrityDigest, blake3};
use serde::{Deserialize, Serialize};

pub const CODEC_SCHEMA_VERSION: &str = "atlas.atlasx-codec.v1";

pub const MAGIC: [u8; 8] = *b"ATLSX\0\x01\0";
pub const HEADER_LEN: usize = 80;
pub const FORMAT_MAJOR: u16 = 1;
/// The only minor. A reader of minor 0 refuses any other (see the module docs).
pub const FORMAT_MINOR: u16 = 0;
/// No header flag is defined in v1.
pub const HEADER_FLAGS: u16 = 0;
pub const DIGEST_BLAKE3_256: u16 = 1;
/// Declared by the contract; not supported here.
pub const DIGEST_SHA256: u16 = 2;
pub const CODEC_NONE: u16 = 0;
/// Declared by the contract; not supported here.
pub const CODEC_ZSTD: u16 = 1;

/// The object class this codec reads and writes.
pub const CLASS_FUNCTIONS: u16 = 4;
/// Classes 1..=21 are the contract's core classes; only FUNCTIONS is supported.
pub const CORE_CLASSES: u16 = 21;
/// The object schema of FUNCTIONS objects: the table `FUNCTIONS_SCHEMA` pinned by `schema_digest`.
pub const FUNCTIONS_SCHEMA_VERSION: u16 = 1;
pub const RECORD_SCHEMA_VERSION: u16 = 1;

pub const RECORD_HEADER_LEN: usize = 16;
pub const FIELD_HEADER_LEN: usize = 8;
/// `field_flags` bit 0: the field is required. No other bit is defined.
pub const FIELD_REQUIRED: u8 = 1;
/// The largest payload read or written (64 MiB), checked before anything else is read.
pub const MAX_DECODED_LENGTH: u64 = 1 << 26;
/// The deepest record nesting read: a top-level record is depth 1. FUNCTIONS needs 2.
pub const MAX_DEPTH: usize = 8;

/// The AtlasX v1 wire types.
pub mod wire {
    pub const INVALID: u8 = 0;
    pub const UVARINT: u8 = 1;
    pub const SVARINT: u8 = 2;
    pub const FIXED32: u8 = 3;
    pub const FIXED64: u8 = 4;
    pub const BYTES: u8 = 5;
    pub const UTF8: u8 = 6;
    pub const HASH32: u8 = 7;
    pub const GLOBAL_ID: u8 = 8;
    pub const LOCAL_INDEX: u8 = 9;
    pub const RECORD: u8 = 10;
    pub const PACKED: u8 = 11;
    pub const BOOL: u8 = 12;
}

/// Record kinds of the FUNCTIONS schema.
pub const FUNCTION_SIGNATURE: u16 = 1;
/// Embedded only: one parameter of a signature.
pub const PARAM: u16 = 2;
/// Embedded only: one census record a signature was lifted from.
pub const LINEAGE_REF: u16 = 3;

/// Dispatch codes of the `dispatch` field (UVARINT).
const DISPATCH_CODES: [(Dispatch, u64); 3] = [
    (Dispatch::FreeFunction, 1),
    (Dispatch::InherentMethod, 2),
    (Dispatch::AssociatedFunction, 3),
];

/// One declared field: the only place a name meets a tag, a wire type and a required flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldDef {
    pub name: &'static str,
    pub tag: u16,
    pub wire: u8,
    pub required: bool,
    /// For a RECORD field: the kind of every embedded record (one or more; never zero).
    pub nested: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordDef {
    pub kind: u16,
    pub name: &'static str,
    /// Whether the record may stand at the top of the payload; otherwise it is embedded only.
    pub top_level: bool,
    pub fields: &'static [FieldDef],
}

const fn req(name: &'static str, tag: u16, wire: u8) -> FieldDef {
    FieldDef {
        name,
        tag,
        wire,
        required: true,
        nested: 0,
    }
}

const fn opt(name: &'static str, tag: u16, wire: u8) -> FieldDef {
    FieldDef {
        required: false,
        ..req(name, tag, wire)
    }
}

const fn records(name: &'static str, tag: u16, nested: u16, required: bool) -> FieldDef {
    FieldDef {
        name,
        tag,
        wire: wire::RECORD,
        required,
        nested,
    }
}

/// The FUNCTIONS object schema, version `FUNCTIONS_SCHEMA_VERSION`.
pub const FUNCTIONS_SCHEMA: &[RecordDef] = &[
    RecordDef {
        kind: FUNCTION_SIGNATURE,
        name: "function-signature",
        top_level: true,
        fields: &[
            req("function_id", 1, wire::GLOBAL_ID),
            req("name", 2, wire::UTF8),
            opt("owner", 3, wire::UTF8),
            req("dispatch", 4, wire::UVARINT),
            req("visibility", 5, wire::UTF8),
            opt("documentation", 6, wire::UTF8),
            records("params", 7, PARAM, false),
            opt("result", 8, wire::UTF8),
            opt("body_fingerprint", 9, wire::HASH32),
            records("lineage", 10, LINEAGE_REF, true),
        ],
    },
    RecordDef {
        kind: PARAM,
        name: "param",
        top_level: false,
        fields: &[
            req("name", 1, wire::UTF8),
            req("type_spelling", 2, wire::UTF8),
        ],
    },
    RecordDef {
        kind: LINEAGE_REF,
        name: "lineage-ref",
        top_level: false,
        fields: &[req("record", 1, wire::GLOBAL_ID)],
    },
];

fn record_def(kind: u16) -> Option<&'static RecordDef> {
    FUNCTIONS_SCHEMA.iter().find(|d| d.kind == kind)
}

impl RecordDef {
    fn field(&self, name: &str) -> &'static FieldDef {
        self.fields
            .iter()
            .find(|f| f.name == name)
            .expect("a field declared in the table")
    }
}

/// The digest of the schema table's definition. It moves with any change to a record kind or to
/// a field's tag, name, wire type, required flag or nested kind, and a test pins it to
/// `FUNCTIONS_SCHEMA_VERSION`: a changed table needs a new schema version.
pub fn schema_digest() -> IntegrityDigest {
    let mut text =
        format!("atlasx class {CLASS_FUNCTIONS} FUNCTIONS schema {FUNCTIONS_SCHEMA_VERSION}\n");
    for def in FUNCTIONS_SCHEMA {
        text.push_str(&format!(
            "record {} {} top_level={}\n",
            def.kind, def.name, def.top_level
        ));
        for f in def.fields {
            text.push_str(&format!(
                "field {} {} wire={} required={} nested={}\n",
                f.tag, f.name, f.wire, f.required, f.nested
            ));
        }
    }
    IntegrityDigest::of_bytes(text.as_bytes())
}

/// What a decoded object does not prove. M12 is the codec only.
pub const CODEC_NOT_VERIFIED: &[&str] = &[
    "the root manifest: this object's entry, relative path, required flag and logical record count (M13)",
    "the AtlasX root identity and the parent Atlas, Genome and SelectedDesign compatibility (M13)",
    "cross-object references: owner names and parameter and result type spellings are carried, not resolved (M13)",
    "lineage: census record ids are carried as GLOBAL_IDs, not resolved against the parent Atlas root (M11, M13)",
    "that the records are the materialization of an admitted parent (M11)",
    "function bodies: FUNCTION_SIGNATURE carries none, and no body record kind is declared",
    "object classes other than FUNCTIONS, the ZSTD codec and the SHA256 digest (all refused, never decoded)",
    "the source module's census inputs: that they resolve in the container it names, and its lineage against the census and the parent Atlas (encoding checks the module's identity and validates it against its own declared inputs only)",
];

crate::vocabulary_enum! {
    /// Whether an AtlasX object's bytes decode (M12).
    pub enum CodecVerdict {
        Decoded => "DECODED",
        Refused => "REFUSED",
    }
}

crate::vocabulary_enum! {
    /// Why the AtlasX codec refuses bytes or an object to write, in the reader's verification
    /// order, then the writer's refusals of its source module.
    pub enum CodecDefect {
        /// Fewer bytes than the header, or than `encoded_length` declares.
        Truncated => "TRUNCATED",
        BadMagic => "BAD_MAGIC",
        BadHeaderLen => "BAD_HEADER_LEN",
        UnsupportedMajor => "UNSUPPORTED_MAJOR",
        UnsupportedMinor => "UNSUPPORTED_MINOR",
        /// A header flag bit, none of which is defined in v1.
        UnknownHeaderFlags => "UNKNOWN_HEADER_FLAGS",
        /// Not the object class read (FUNCTIONS).
        ClassMismatch => "CLASS_MISMATCH",
        UnsupportedSchemaVersion => "UNSUPPORTED_SCHEMA_VERSION",
        /// SHA256 (declared, not supported), INVALID or unknown.
        UnsupportedDigestAlgorithm => "UNSUPPORTED_DIGEST_ALGORITHM",
        /// ZSTD (declared, not supported) or unknown.
        UnsupportedCodec => "UNSUPPORTED_CODEC",
        ReservedNonzero => "RESERVED_NONZERO",
        /// A length over `MAX_DECODED_LENGTH`, or a field too long to frame.
        LengthLimit => "LENGTH_LIMIT",
        /// Encoded and decoded lengths differ under codec NONE, or bytes follow the payload.
        LengthMismatch => "LENGTH_MISMATCH",
        /// The payload is not what `decoded_content_hash` authenticates.
        DigestMismatch => "DIGEST_MISMATCH",
        /// A truncated record header or a record payload out of bounds.
        RecordFraming => "RECORD_FRAMING",
        /// A truncated field header or a field out of bounds.
        FieldFraming => "FIELD_FRAMING",
        /// Field tags not strictly ascending: out of order or duplicated.
        FieldOrder => "FIELD_ORDER",
        /// Wire type INVALID (0) or outside the v1 table.
        InvalidWireType => "INVALID_WIRE_TYPE",
        /// Records nested deeper than `MAX_DEPTH`.
        DepthLimit => "DEPTH_LIMIT",
        /// A record kind the schema does not declare where it stands.
        UndeclaredRecordKind => "UNDECLARED_RECORD_KIND",
        UnsupportedRecordVersion => "UNSUPPORTED_RECORD_VERSION",
        /// A record flag bit, none of which is defined in v1.
        RecordFlags => "RECORD_FLAGS",
        /// An undeclared field marked required.
        UndeclaredRequiredField => "UNDECLARED_REQUIRED_FIELD",
        /// An undeclared field marked optional (no minor extends the schema; see the module docs).
        UndeclaredOptionalField => "UNDECLARED_OPTIONAL_FIELD",
        /// `field_flags` with an undefined bit, or a required flag the schema does not declare.
        FieldFlags => "FIELD_FLAGS",
        /// A declared field carried with another wire type.
        WireTypeMismatch => "WIRE_TYPE_MISMATCH",
        MissingRequiredField => "MISSING_REQUIRED_FIELD",
        /// A value its wire type or field does not admit: not UTF-8, an empty identity, a varint
        /// that is not minimal, an unknown dispatch code, a hash not 32 bytes, a sequence with no
        /// element.
        MalformedValue => "MALFORMED_VALUE",
        /// Two records with one stable identity.
        DuplicateIdentity => "DUPLICATE_IDENTITY",
        /// An object without a record.
        EmptyObject => "EMPTY_OBJECT",
        /// Well-formed, but not the bytes the canonical writer gives for what it decodes to.
        NonCanonical => "NON_CANONICAL",
        /// The object's file is not named by its address.
        AddressMismatch => "ADDRESS_MISMATCH",
        /// Writing only: the construction module's id is not its content identity.
        SourceModuleUnverified => "SOURCE_MODULE_UNVERIFIED",
        /// Writing only: `construction::validate_module` refuses the module (checked against the
        /// census records it declares as inputs).
        SourceModuleInvalid => "SOURCE_MODULE_INVALID",
    }
}

pub type Defects = Vec<(CodecDefect, String)>;

fn defect<T>(kind: CodecDefect, detail: impl Into<String>) -> Result<T, Defects> {
    Err(vec![(kind, detail.into())])
}

fn settle(mut defects: Defects) -> Defects {
    defects.sort();
    defects.dedup();
    defects
}

/// One FUNCTION_SIGNATURE record: a construction-IR function without its body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionSignatureRecord {
    pub function_id: String,
    pub name: String,
    pub owner: Option<String>,
    pub dispatch: Dispatch,
    pub visibility: String,
    pub documentation: Option<String>,
    /// In declaration order.
    pub params: Vec<IrParam>,
    pub result: Option<String>,
    /// `blake3-256:<hex>`, carried as its 32 bytes.
    pub body_fingerprint: Option<String>,
    /// The census records the function was lifted from; a set, canonically sorted.
    pub lineage: Vec<String>,
}

impl FunctionSignatureRecord {
    pub fn of(function: &IrFunction) -> Self {
        Self {
            function_id: function.id.clone(),
            name: function.name.clone(),
            owner: function.owner.clone(),
            dispatch: function.dispatch,
            visibility: function.visibility.clone(),
            documentation: function.documentation.clone(),
            params: function.params.clone(),
            result: function.result.clone(),
            body_fingerprint: function.body_fingerprint.clone(),
            lineage: function.lineage.clone(),
        }
    }

    /// The canonical form: lineage sorted bytewise and deduplicated.
    pub fn canonicalize(&mut self) {
        self.lineage.sort();
        self.lineage.dedup();
    }
}

/// The fixed 80-byte object header, as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectHeader {
    pub magic: [u8; 8],
    pub header_len: u16,
    pub format_major: u16,
    pub format_minor: u16,
    pub flags: u16,
    pub object_class: u16,
    pub object_schema_version: u16,
    pub digest_algorithm: u16,
    pub codec: u16,
    pub encoded_length: u64,
    pub decoded_length: u64,
    pub decoded_content_hash: [u8; 32],
    pub reserved: u64,
}

impl ObjectHeader {
    /// The header a canonical FUNCTIONS object with `payload` carries.
    pub fn functions(payload: &[u8]) -> Self {
        let length = payload.len() as u64;
        Self {
            magic: MAGIC,
            header_len: HEADER_LEN as u16,
            format_major: FORMAT_MAJOR,
            format_minor: FORMAT_MINOR,
            flags: HEADER_FLAGS,
            object_class: CLASS_FUNCTIONS,
            object_schema_version: FUNCTIONS_SCHEMA_VERSION,
            digest_algorithm: DIGEST_BLAKE3_256,
            codec: CODEC_NONE,
            encoded_length: length,
            decoded_length: length,
            decoded_content_hash: blake3::hash(payload),
            reserved: 0,
        }
    }

    /// `bytes`' first 80 bytes as header fields, unchecked; `None` when there are fewer.
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let h = bytes.get(..HEADER_LEN)?;
        let mut magic = [0; 8];
        magic.copy_from_slice(&h[..8]);
        let mut hash = [0; 32];
        hash.copy_from_slice(&h[40..72]);
        Some(Self {
            magic,
            header_len: u16_at(h, 8),
            format_major: u16_at(h, 10),
            format_minor: u16_at(h, 12),
            flags: u16_at(h, 14),
            object_class: u16_at(h, 16),
            object_schema_version: u16_at(h, 18),
            digest_algorithm: u16_at(h, 20),
            codec: u16_at(h, 22),
            encoded_length: u64_at(h, 24),
            decoded_length: u64_at(h, 32),
            decoded_content_hash: hash,
            reserved: u64_at(h, 72),
        })
    }

    pub fn to_bytes(&self) -> [u8; HEADER_LEN] {
        let mut out = [0; HEADER_LEN];
        out[..8].copy_from_slice(&self.magic);
        for (at, value) in [
            (8, self.header_len),
            (10, self.format_major),
            (12, self.format_minor),
            (14, self.flags),
            (16, self.object_class),
            (18, self.object_schema_version),
            (20, self.digest_algorithm),
            (22, self.codec),
        ] {
            out[at..at + 2].copy_from_slice(&value.to_le_bytes());
        }
        out[24..32].copy_from_slice(&self.encoded_length.to_le_bytes());
        out[32..40].copy_from_slice(&self.decoded_length.to_le_bytes());
        out[40..72].copy_from_slice(&self.decoded_content_hash);
        out[72..80].copy_from_slice(&self.reserved.to_le_bytes());
        out
    }

    /// The object's address: its filename stem.
    pub fn address(&self) -> String {
        object_address(&self.decoded_content_hash)
    }

    pub fn digest(&self) -> IntegrityDigest {
        IntegrityDigest::blake3_256(&self.decoded_content_hash)
    }

    /// Every header field that is not what a v1 FUNCTIONS object carries.
    fn defects(&self) -> Defects {
        use CodecDefect as D;
        let mut out = Defects::new();
        if self.magic != MAGIC {
            out.push((D::BadMagic, format!("magic {:02x?}", self.magic)));
        }
        if usize::from(self.header_len) != HEADER_LEN {
            out.push((D::BadHeaderLen, format!("header_len {}", self.header_len)));
        }
        if self.format_major != FORMAT_MAJOR {
            out.push((D::UnsupportedMajor, format!("major {}", self.format_major)));
        }
        if self.format_minor != FORMAT_MINOR {
            out.push((D::UnsupportedMinor, format!("minor {}", self.format_minor)));
        }
        if self.flags != HEADER_FLAGS {
            out.push((D::UnknownHeaderFlags, format!("flags {:#06x}", self.flags)));
        }
        if self.object_class != CLASS_FUNCTIONS {
            let known = (1..=CORE_CLASSES).contains(&self.object_class);
            out.push((
                D::ClassMismatch,
                format!(
                    "class {} ({}), expected FUNCTIONS ({CLASS_FUNCTIONS})",
                    self.object_class,
                    if known {
                        "a core class this codec does not read"
                    } else {
                        "not a core class"
                    }
                ),
            ));
        } else if self.object_schema_version != FUNCTIONS_SCHEMA_VERSION {
            out.push((
                D::UnsupportedSchemaVersion,
                format!("FUNCTIONS schema {}", self.object_schema_version),
            ));
        }
        if self.digest_algorithm != DIGEST_BLAKE3_256 {
            let what = if self.digest_algorithm == DIGEST_SHA256 {
                "SHA256, not supported"
            } else {
                "not a digest algorithm"
            };
            out.push((
                D::UnsupportedDigestAlgorithm,
                format!("digest algorithm {} ({what})", self.digest_algorithm),
            ));
        }
        if self.codec != CODEC_NONE {
            let what = if self.codec == CODEC_ZSTD {
                "ZSTD, not supported"
            } else {
                "not a codec"
            };
            out.push((
                D::UnsupportedCodec,
                format!("codec {} ({what})", self.codec),
            ));
        }
        if self.reserved != 0 {
            out.push((D::ReservedNonzero, format!("reserved {:#x}", self.reserved)));
        }
        out
    }
}

/// The lowercase hex of a decoded content hash: an object's canonical filename stem.
pub fn object_address(hash: &[u8; 32]) -> String {
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

// ---- writing ------------------------------------------------------------------------------------

/// One record's fields, written by name through the table and emitted in ascending tag order.
struct RecordWriter {
    def: &'static RecordDef,
    fields: Vec<(u16, Vec<u8>)>,
}

impl RecordWriter {
    fn new(kind: u16) -> Self {
        Self {
            def: record_def(kind).expect("a record kind of the table"),
            fields: Vec::new(),
        }
    }

    fn put(&mut self, name: &str, bytes: &[u8]) -> Result<(), Defects> {
        let def = self.def.field(name);
        let Ok(len) = u32::try_from(bytes.len()) else {
            return defect(
                CodecDefect::LengthLimit,
                format!("field `{name}`: {} bytes", bytes.len()),
            );
        };
        let mut out = Vec::with_capacity(FIELD_HEADER_LEN + bytes.len());
        out.extend_from_slice(&def.tag.to_le_bytes());
        out.push(def.wire);
        out.push(if def.required { FIELD_REQUIRED } else { 0 });
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(bytes);
        self.fields.push((def.tag, out));
        Ok(())
    }

    fn text(&mut self, name: &str, value: &str) -> Result<(), Defects> {
        self.put(name, value.as_bytes())
    }

    fn optional_text(&mut self, name: &str, value: Option<&str>) -> Result<(), Defects> {
        value.map_or(Ok(()), |v| self.text(name, v))
    }

    /// A RECORD field of the embedded `records`; absent when there are none.
    fn embedded(&mut self, name: &str, records: Vec<RecordWriter>) -> Result<(), Defects> {
        if records.is_empty() {
            return Ok(());
        }
        let mut bytes = Vec::new();
        for record in records {
            record.frame(&mut bytes);
        }
        self.put(name, &bytes)
    }

    fn payload(mut self) -> Vec<u8> {
        self.fields.sort_by_key(|(tag, _)| *tag);
        self.fields
            .into_iter()
            .flat_map(|(_, bytes)| bytes)
            .collect()
    }

    fn frame(self, out: &mut Vec<u8>) {
        let kind = self.def.kind;
        let payload = self.payload();
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&RECORD_SCHEMA_VERSION.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        out.extend_from_slice(&payload);
    }
}

fn signature_record(record: &FunctionSignatureRecord) -> Result<RecordWriter, Defects> {
    use CodecDefect as D;
    let mut record = record.clone();
    record.canonicalize();
    let id = &record.function_id;
    if id.is_empty() {
        return defect(D::MalformedValue, "a function with an empty function_id");
    }
    if record.name.is_empty() {
        return defect(D::MalformedValue, format!("{id}: an empty name"));
    }
    if record.lineage.is_empty() {
        return defect(D::MissingRequiredField, format!("{id}: no lineage"));
    }
    if record.lineage.iter().any(String::is_empty) {
        return defect(
            D::MalformedValue,
            format!("{id}: an empty lineage record id"),
        );
    }
    let mut w = RecordWriter::new(FUNCTION_SIGNATURE);
    w.text("function_id", id)?;
    w.text("name", &record.name)?;
    w.optional_text("owner", record.owner.as_deref())?;
    let code = DISPATCH_CODES
        .iter()
        .find(|(d, _)| *d == record.dispatch)
        .map(|(_, c)| *c)
        .expect("every Dispatch has a code");
    let mut varint = Vec::new();
    uvarint(code, &mut varint);
    w.put("dispatch", &varint)?;
    w.text("visibility", &record.visibility)?;
    w.optional_text("documentation", record.documentation.as_deref())?;
    let mut params = Vec::with_capacity(record.params.len());
    for param in &record.params {
        let mut p = RecordWriter::new(PARAM);
        p.text("name", &param.name)?;
        p.text("type_spelling", &param.type_spelling)?;
        params.push(p);
    }
    w.embedded("params", params)?;
    w.optional_text("result", record.result.as_deref())?;
    if let Some(fingerprint) = &record.body_fingerprint {
        let Ok(digest) = IntegrityDigest::parse(fingerprint) else {
            return defect(
                D::MalformedValue,
                format!("{id}: body_fingerprint `{fingerprint}` is not a blake3-256 digest"),
            );
        };
        // `parse` admits exactly 64 lowercase hex digits after the prefix.
        let hex = &digest.as_str().as_bytes()[IntegrityDigest::BLAKE3_256_PREFIX.len()..];
        let nibble = |c: u8| {
            if c.is_ascii_digit() {
                c - b'0'
            } else {
                c - b'a' + 10
            }
        };
        let bytes: Vec<u8> = hex
            .chunks(2)
            .map(|pair| (nibble(pair[0]) << 4) | nibble(pair[1]))
            .collect();
        w.put("body_fingerprint", &bytes)?;
    }
    let mut refs = Vec::with_capacity(record.lineage.len());
    for id in &record.lineage {
        let mut l = RecordWriter::new(LINEAGE_REF);
        l.text("record", id)?;
        refs.push(l);
    }
    w.embedded("lineage", refs)?;
    Ok(w)
}

/// The canonical bytes of the FUNCTIONS object holding `records`, header included. The same set
/// of records in any order gives the same bytes.
pub fn encode_functions(records: &[FunctionSignatureRecord]) -> Result<Vec<u8>, Defects> {
    use CodecDefect as D;
    let mut defects = Defects::new();
    let mut framed: Vec<(&str, Vec<u8>)> = Vec::with_capacity(records.len());
    for record in records {
        match signature_record(record) {
            Ok(w) => {
                let mut bytes = Vec::new();
                w.frame(&mut bytes);
                framed.push((record.function_id.as_str(), bytes));
            }
            Err(e) => defects.extend(e),
        }
    }
    if records.is_empty() {
        defects.push((D::EmptyObject, "no function signature to write".into()));
    }
    framed.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    for pair in framed.windows(2) {
        if pair[0].0 == pair[1].0 {
            defects.push((
                D::DuplicateIdentity,
                format!("two records for {}", pair[0].0),
            ));
        }
    }
    if !defects.is_empty() {
        return Err(settle(defects));
    }
    let payload: Vec<u8> = framed.into_iter().flat_map(|(_, bytes)| bytes).collect();
    if payload.len() as u64 > MAX_DECODED_LENGTH {
        return defect(
            D::LengthLimit,
            format!("payload of {} bytes", payload.len()),
        );
    }
    let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
    out.extend_from_slice(&ObjectHeader::functions(&payload).to_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

/// A FUNCTIONS object written from a construction module.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncodedObject {
    pub schema: String,
    /// The module the records were taken from, as it names itself.
    pub module_id: String,
    pub object_class: u16,
    pub object_schema_version: u16,
    /// The filename stem: lowercase hex of the decoded content hash.
    pub address: String,
    pub digest: IntegrityDigest,
    pub decoded_length: u64,
    pub records: usize,
    /// Functions whose HIR body FUNCTION_SIGNATURE does not carry, by id. Their bodies are not
    /// in the object.
    pub bodies_not_encoded: Vec<String>,
    pub not_verified: Vec<String>,
    #[serde(skip)]
    pub bytes: Vec<u8>,
}

/// M12: the FUNCTIONS object of `module`'s function signatures. The module must verify first:
/// its id is its content identity, and `construction::validate_module` finds nothing against the
/// census records the module itself declares as inputs (whether they resolve in the container it
/// names is not checked here; see `CODEC_NOT_VERIFIED`).
pub fn encode_module(module: &ConstructionModule) -> Result<EncodedObject, Defects> {
    use CodecDefect as D;
    let mut defects = Defects::new();
    if module.module_id != module_identity(module) {
        defects.push((
            D::SourceModuleUnverified,
            format!("the identity of {} does not verify", module.module_id),
        ));
    }
    let declared = module
        .inputs
        .iter()
        .filter(|i| i.kind == ConstructionInputKind::CensusRecord)
        .map(|i| i.reference.clone())
        .collect();
    for v in validate_module(module, &declared) {
        if v.code != "MODULE_ID" {
            defects.push((D::SourceModuleInvalid, format!("{}: {}", v.code, v.detail)));
        }
    }
    if !defects.is_empty() {
        return Err(settle(defects));
    }
    let records: Vec<FunctionSignatureRecord> = module
        .functions
        .iter()
        .map(FunctionSignatureRecord::of)
        .collect();
    let bytes = encode_functions(&records)?;
    let header = ObjectHeader::parse(&bytes).expect("the writer writes a header");
    let mut bodies_not_encoded: Vec<String> = module
        .functions
        .iter()
        .filter(|f| f.body.is_some())
        .map(|f| f.id.clone())
        .collect();
    bodies_not_encoded.sort();
    Ok(EncodedObject {
        schema: CODEC_SCHEMA_VERSION.into(),
        module_id: module.module_id.clone(),
        object_class: header.object_class,
        object_schema_version: header.object_schema_version,
        address: header.address(),
        digest: header.digest(),
        decoded_length: header.decoded_length,
        records: records.len(),
        bodies_not_encoded,
        not_verified: not_verified(),
        bytes,
    })
}

// ---- reading ------------------------------------------------------------------------------------

/// One record as framed, before its schema is consulted.
struct Frame<'a> {
    kind: u16,
    version: u16,
    flags: u32,
    fields: Vec<FieldFrame<'a>>,
}

struct FieldFrame<'a> {
    tag: u16,
    wire: u8,
    flags: u8,
    bytes: &'a [u8],
    /// For a RECORD field: its embedded records, framed.
    embedded: Vec<Frame<'a>>,
}

/// Record and field framing of `content` at nesting `depth`: bounds, strictly ascending tags,
/// wire types in the v1 table, embedded records framed in turn. No schema is consulted.
fn frames(content: &[u8], depth: usize) -> Result<Vec<Frame<'_>>, Defects> {
    use CodecDefect as D;
    if depth > MAX_DEPTH {
        return defect(D::DepthLimit, format!("records nested {depth} deep"));
    }
    let mut out = Vec::new();
    let mut at = 0usize;
    while at < content.len() {
        if content.len() - at < RECORD_HEADER_LEN {
            return defect(D::RecordFraming, format!("truncated record header at {at}"));
        }
        let kind = u16_at(content, at);
        let version = u16_at(content, at + 2);
        let flags = u32_at(content, at + 4);
        let declared = u64_at(content, at + 8);
        let start = at + RECORD_HEADER_LEN;
        let Some(end) = usize::try_from(declared)
            .ok()
            .and_then(|len| start.checked_add(len))
            .filter(|end| *end <= content.len())
        else {
            return defect(
                D::RecordFraming,
                format!("record at {at}: payload of {declared} bytes out of bounds"),
            );
        };
        let payload = &content[start..end];
        let mut fields: Vec<FieldFrame> = Vec::new();
        let mut p = 0usize;
        while p < payload.len() {
            if payload.len() - p < FIELD_HEADER_LEN {
                return defect(
                    D::FieldFraming,
                    format!("record at {at}: truncated field header"),
                );
            }
            let tag = u16_at(payload, p);
            let wire_type = payload[p + 2];
            let field_flags = payload[p + 3];
            let len = u32_at(payload, p + 4);
            let fstart = p + FIELD_HEADER_LEN;
            let Some(fend) = usize::try_from(len)
                .ok()
                .and_then(|len| fstart.checked_add(len))
                .filter(|end| *end <= payload.len())
            else {
                return defect(
                    D::FieldFraming,
                    format!("record at {at}: field {tag} of {len} bytes out of bounds"),
                );
            };
            if let Some(last) = fields.last()
                && last.tag >= tag
            {
                return defect(
                    D::FieldOrder,
                    format!("record at {at}: field {tag} after field {}", last.tag),
                );
            }
            if wire_type == wire::INVALID || wire_type > wire::BOOL {
                return defect(
                    D::InvalidWireType,
                    format!("record at {at}: field {tag} has wire type {wire_type}"),
                );
            }
            let bytes = &payload[fstart..fend];
            let embedded = if wire_type == wire::RECORD {
                frames(bytes, depth + 1)?
            } else {
                Vec::new()
            };
            fields.push(FieldFrame {
                tag,
                wire: wire_type,
                flags: field_flags,
                bytes,
                embedded,
            });
            p = fend;
        }
        out.push(Frame {
            kind,
            version,
            flags,
            fields,
        });
        at = end;
    }
    Ok(out)
}

/// One framed record checked against its declaration: supported version, no record flag, every
/// field declared with its flags and wire type, every required field present.
struct Declared<'f, 'a> {
    def: &'static RecordDef,
    frame: &'f Frame<'a>,
}

fn declared<'f, 'a>(
    frame: &'f Frame<'a>,
    def: &'static RecordDef,
) -> Result<Declared<'f, 'a>, Defects> {
    use CodecDefect as D;
    let what = def.name;
    if frame.version != RECORD_SCHEMA_VERSION {
        return defect(
            D::UnsupportedRecordVersion,
            format!("{what}: record schema version {}", frame.version),
        );
    }
    if frame.flags != 0 {
        return defect(
            D::RecordFlags,
            format!("{what}: record flags {:#x}", frame.flags),
        );
    }
    for field in &frame.fields {
        if field.flags & !FIELD_REQUIRED != 0 {
            return defect(
                D::FieldFlags,
                format!("{what}: field {} flags {:#04x}", field.tag, field.flags),
            );
        }
        let Some(d) = def.fields.iter().find(|d| d.tag == field.tag) else {
            return if field.flags & FIELD_REQUIRED != 0 {
                defect(
                    D::UndeclaredRequiredField,
                    format!("{what}: undeclared required field {}", field.tag),
                )
            } else {
                defect(
                    D::UndeclaredOptionalField,
                    format!("{what}: undeclared optional field {}", field.tag),
                )
            };
        };
        if (field.flags & FIELD_REQUIRED != 0) != d.required {
            return defect(
                D::FieldFlags,
                format!(
                    "{what}: field {} ({}) flags {:#04x}",
                    d.tag, d.name, field.flags
                ),
            );
        }
        if field.wire != d.wire {
            return defect(
                D::WireTypeMismatch,
                format!(
                    "{what}: field {} ({}) has wire type {}, declared {}",
                    d.tag, d.name, field.wire, d.wire
                ),
            );
        }
        if d.wire == wire::RECORD {
            if field.embedded.is_empty() {
                return defect(
                    D::MalformedValue,
                    format!("{what}: field `{}` embeds no record", d.name),
                );
            }
            if let Some(e) = field.embedded.iter().find(|e| e.kind != d.nested) {
                return defect(
                    D::UndeclaredRecordKind,
                    format!(
                        "{what}: field `{}` embeds kind {}, declared {}",
                        d.name, e.kind, d.nested
                    ),
                );
            }
        }
    }
    for d in def.fields.iter().filter(|d| d.required) {
        if !frame.fields.iter().any(|f| f.tag == d.tag) {
            return defect(
                D::MissingRequiredField,
                format!("{what}: field {} ({})", d.tag, d.name),
            );
        }
    }
    Ok(Declared { def, frame })
}

impl<'f, 'a> Declared<'f, 'a> {
    fn raw(&self, name: &str) -> Option<&'f FieldFrame<'a>> {
        let tag = self.def.field(name).tag;
        self.frame.fields.iter().find(|f| f.tag == tag)
    }

    fn optional_text(&self, name: &str) -> Result<Option<String>, Defects> {
        let Some(field) = self.raw(name) else {
            return Ok(None);
        };
        match std::str::from_utf8(field.bytes) {
            Ok(text) => Ok(Some(text.to_owned())),
            Err(_) => defect(
                CodecDefect::MalformedValue,
                format!("{}: `{name}` is not UTF-8", self.def.name),
            ),
        }
    }

    /// A required UTF8 or GLOBAL_ID field (presence is checked by `declared`; an empty identity
    /// is refused by the writer when the reader re-encodes).
    fn text(&self, name: &str) -> Result<String, Defects> {
        Ok(self.optional_text(name)?.unwrap_or_default())
    }

    fn uvarint(&self, name: &str) -> Result<u64, Defects> {
        let bytes = self.raw(name).map_or(&[][..], |f| f.bytes);
        read_uvarint(bytes).or_else(|e| {
            defect(
                CodecDefect::MalformedValue,
                format!("{}: `{name}`: {e}", self.def.name),
            )
        })
    }

    fn optional_hash(&self, name: &str) -> Result<Option<[u8; 32]>, Defects> {
        let Some(field) = self.raw(name) else {
            return Ok(None);
        };
        match <[u8; 32]>::try_from(field.bytes) {
            Ok(hash) => Ok(Some(hash)),
            Err(_) => defect(
                CodecDefect::MalformedValue,
                format!(
                    "{}: `{name}` is {} bytes, not 32",
                    self.def.name,
                    field.bytes.len()
                ),
            ),
        }
    }

    fn embedded(&self, name: &str) -> &'f [Frame<'a>] {
        self.raw(name).map_or(&[], |f| f.embedded.as_slice())
    }
}

fn decode_signature(frame: &Frame) -> Result<FunctionSignatureRecord, Defects> {
    use CodecDefect as D;
    let Some(def) = record_def(frame.kind).filter(|d| d.top_level) else {
        return defect(
            D::UndeclaredRecordKind,
            format!("top-level record kind {}", frame.kind),
        );
    };
    let r = declared(frame, def)?;
    let code = r.uvarint("dispatch")?;
    let Some(dispatch) = DISPATCH_CODES
        .iter()
        .find(|(_, c)| *c == code)
        .map(|(d, _)| *d)
    else {
        return defect(D::MalformedValue, format!("dispatch code {code}"));
    };
    let mut params = Vec::new();
    for frame in r.embedded("params") {
        let p = declared(frame, record_def(PARAM).expect("declared"))?;
        params.push(IrParam {
            name: p.text("name")?,
            type_spelling: p.text("type_spelling")?,
        });
    }
    let mut lineage = Vec::new();
    for frame in r.embedded("lineage") {
        let l = declared(frame, record_def(LINEAGE_REF).expect("declared"))?;
        lineage.push(l.text("record")?);
    }
    Ok(FunctionSignatureRecord {
        function_id: r.text("function_id")?,
        name: r.text("name")?,
        owner: r.optional_text("owner")?,
        dispatch,
        visibility: r.text("visibility")?,
        documentation: r.optional_text("documentation")?,
        params,
        result: r.optional_text("result")?,
        body_fingerprint: r
            .optional_hash("body_fingerprint")?
            .map(|h| IntegrityDigest::blake3_256(&h).into()),
        lineage,
    })
}

/// M12: the FUNCTIONS object in `bytes`, verified in the contract's order for one object, with
/// its header; or every header defect together, otherwise the first defect found.
pub fn read(bytes: &[u8]) -> Result<(ObjectHeader, Vec<FunctionSignatureRecord>), Defects> {
    use CodecDefect as D;
    // 1. The header.
    let Some(header) = ObjectHeader::parse(bytes) else {
        return defect(
            D::Truncated,
            format!("{} bytes, shorter than the header", bytes.len()),
        );
    };
    let defects = header.defects();
    if !defects.is_empty() {
        return Err(settle(defects));
    }
    // 2. Bounds, before anything the lengths describe is read.
    for (what, length) in [
        ("encoded", header.encoded_length),
        ("decoded", header.decoded_length),
    ] {
        if length > MAX_DECODED_LENGTH {
            return defect(
                D::LengthLimit,
                format!("{what}_length {length} over {MAX_DECODED_LENGTH}"),
            );
        }
    }
    if header.encoded_length != header.decoded_length {
        return defect(
            D::LengthMismatch,
            format!(
                "encoded_length {} and decoded_length {} under codec NONE",
                header.encoded_length, header.decoded_length
            ),
        );
    }
    let payload = &bytes[HEADER_LEN..];
    let actual = payload.len() as u64;
    if actual < header.encoded_length {
        return defect(
            D::Truncated,
            format!("{actual} payload bytes of {}", header.encoded_length),
        );
    }
    if actual > header.encoded_length {
        return defect(
            D::LengthMismatch,
            format!("{} bytes after the payload", actual - header.encoded_length),
        );
    }
    // 3. The decoded hash.
    if blake3::hash(payload) != header.decoded_content_hash {
        return defect(
            D::DigestMismatch,
            format!("the payload does not hash to {}", header.address()),
        );
    }
    // 4. Record and field framing.
    let frames = frames(payload, 1)?;
    // 5. Class/schema constraints.
    let records = frames
        .iter()
        .map(decode_signature)
        .collect::<Result<Vec<_>, _>>()?;
    // 6. Canonical form: exactly the bytes the writer gives for what was decoded. The writer's
    // refusals apply to what is read by this one rule: an empty object (EMPTY_OBJECT), two records
    // with one identity (DUPLICATE_IDENTITY), an empty identity or lineage id (MALFORMED_VALUE).
    if encode_functions(&records)? != bytes {
        return defect(
            D::NonCanonical,
            "not the canonical encoding of its records (record or lineage order)",
        );
    }
    Ok((header, records))
}

/// The typed result of reading one object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodecReport {
    pub schema: String,
    pub verdict: CodecVerdict,
    /// Every header defect together, otherwise the first defect found; typed, with its detail,
    /// sorted.
    pub defects: Defects,
    /// Only when DECODED: the object's address (filename stem) and digest.
    pub address: Option<String>,
    pub digest: Option<IntegrityDigest>,
    pub object_class: Option<u16>,
    pub object_schema_version: Option<u16>,
    pub decoded_length: Option<u64>,
    pub records: Option<Vec<FunctionSignatureRecord>>,
    /// What a DECODED object does not prove (`CODEC_NOT_VERIFIED`).
    pub not_verified: Vec<String>,
}

fn not_verified() -> Vec<String> {
    CODEC_NOT_VERIFIED.iter().map(|s| (*s).to_owned()).collect()
}

/// M12: the verdict on `bytes` as a FUNCTIONS object.
pub fn decode(bytes: &[u8]) -> CodecReport {
    let mut report = CodecReport {
        schema: CODEC_SCHEMA_VERSION.into(),
        verdict: CodecVerdict::Refused,
        defects: Defects::new(),
        address: None,
        digest: None,
        object_class: None,
        object_schema_version: None,
        decoded_length: None,
        records: None,
        not_verified: not_verified(),
    };
    match read(bytes) {
        Ok((header, records)) => {
            report.verdict = CodecVerdict::Decoded;
            report.address = Some(header.address());
            report.digest = Some(header.digest());
            report.object_class = Some(header.object_class);
            report.object_schema_version = Some(header.object_schema_version);
            report.decoded_length = Some(header.decoded_length);
            report.records = Some(records);
        }
        Err(defects) => report.defects = settle(defects),
    }
    report
}

/// `decode`, and the object must be named by its address: `<address>.atlasx`.
pub fn decode_named(file_name: &str, bytes: &[u8]) -> CodecReport {
    let mut report = decode(bytes);
    if let Some(address) = &report.address
        && file_name != format!("{address}.atlasx")
    {
        report.defects = vec![(
            CodecDefect::AddressMismatch,
            format!("`{file_name}` is not `{address}.atlasx`"),
        )];
        report.verdict = CodecVerdict::Refused;
        report.address = None;
        report.digest = None;
        report.object_class = None;
        report.object_schema_version = None;
        report.decoded_length = None;
        report.records = None;
    }
    report
}

#[cfg(test)]
mod tests;
