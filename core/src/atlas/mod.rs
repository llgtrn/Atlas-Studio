//! Minimum `.atlas` container: ATLAS binary wire v1 writer, reader and validator (G64, ADR 0027).
//!
//! Implements `.atlas/contracts/ATLAS-BINARY-WIRE-FORMAT.md` for one shape: an **unsealed census
//! container** that packages a census's validated semantic state -- its facts, typed obligations,
//! the declared ADL graph and its CensusCertificate. No seal gate exists yet, so the writer refuses
//! to produce anything else: the header carries `FLAG_UNSEALED` and the root manifest says
//! `UNSEALED_CENSUS_CONTAINER` (contract: a candidate/unsealed root "MUST NOT be advertised as
//! canonical sealed `*.atlas`").
//!
//! The wire contract leaves several encodings undefined; this module pins them (ADR 0027):
//! - varints are unsigned LEB128, at most 10 bytes, minimally encoded;
//! - field wire types reuse the AtlasX wire table (1 UVARINT, 6 UTF8, 7 HASH32, 9 LOCAL_INDEX);
//! - `field_flags` bit 0 is REQUIRED; header `flags` bit 0 is UNSEALED;
//! - `schema_id` = the first 8 bytes (little-endian) of BLAKE3(`atlas.wire.v1/<section>`);
//! - the root identity is BLAKE3 over the ROOT_MANIFEST section's decoded content, and the
//!   manifest commits to every other section's type, schema id, content hash and record count;
//! - the string table is sorted by byte order and deduplicated; records reference it by index;
//! - records in every section are in a canonical order, which the reader enforces.
//!
//! Identity never depends on local table positions leaking across artifacts: record contents are
//! strings, and the root identity covers decoded content only.

use crate::identity::{IntegrityDigest, blake3};
use std::collections::BTreeMap;

pub const MAGIC: [u8; 8] = *b"ATLAS\0\x01\0";
pub const HEADER_LEN: usize = 72;
pub const FORMAT_MAJOR: u16 = 1;
pub const FORMAT_MINOR: u16 = 0;
pub const FLAG_UNSEALED: u16 = 1;
pub const DIGEST_BLAKE3_256: u16 = 1;
pub const COMPRESSION_NONE: u16 = 0;
pub const DIRECTORY_ENTRY_LEN: usize = 80;
/// The only seal status this writer may produce.
pub const UNSEALED: &str = "UNSEALED_CENSUS_CONTAINER";
const RECORD_SCHEMA_VERSION: u16 = 1;
const REQUIRED: u8 = 1;

pub const ROOT_MANIFEST: u16 = 1;
pub const STRING_TABLE: u16 = 2;
pub const SEMANTIC_RECORDS: u16 = 6;
pub const GRAPH_NODES: u16 = 7;
pub const GRAPH_EDGES: u16 = 8;
pub const OBLIGATIONS: u16 = 12;
pub const CENSUS_CERTIFICATE: u16 = 17;

const WIRE_UVARINT: u8 = 1;
const WIRE_UTF8: u8 = 6;
const WIRE_HASH32: u8 = 7;
const WIRE_LOCAL_INDEX: u8 = 9;

fn section_name(section: u16) -> &'static str {
    match section {
        ROOT_MANIFEST => "root-manifest",
        STRING_TABLE => "string-table",
        SEMANTIC_RECORDS => "census-facts",
        GRAPH_NODES => "declared-nodes",
        GRAPH_EDGES => "declared-edges",
        OBLIGATIONS => "semantic-obligations",
        CENSUS_CERTIFICATE => "census-certificate",
        _ => "unknown",
    }
}

/// The pinned schema id of a section this module writes.
pub fn schema_id(section: u16) -> u64 {
    let digest = blake3::hash(format!("atlas.wire.v1/{}", section_name(section)).as_bytes());
    u64::from_le_bytes(digest[..8].try_into().expect("8 bytes"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootManifest {
    pub genome_schema: String,
    pub genome_hash: [u8; 32],
    /// The self-recensus census digest of the packaged census (`CensusSnapshot::census_digest`).
    pub census_digest: [u8; 32],
    pub revision: String,
    pub certificate_id: String,
    pub seal: String,
    pub tool: String,
    /// `THIN`: source is retained externally, not embedded.
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CensusFact {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub source_path: String,
    pub revision: Option<String>,
    pub extractor: String,
    pub span: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CensusObligation {
    pub artifact: String,
    pub dimension: String,
    pub extractor: String,
    pub extractor_version: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclaredNodeRecord {
    pub name: String,
    pub kind: String,
    pub origin: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclaredEdgeRecord {
    pub from: String,
    pub relation: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateRecord {
    pub certificate_id: String,
    pub state: String,
    pub blockers: Vec<String>,
}

/// The semantic content of one census container. `canonicalize` puts it in the order the wire
/// requires; `write` refuses content that is not canonical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CensusAtlas {
    pub manifest: RootManifest,
    pub facts: Vec<CensusFact>,
    pub obligations: Vec<CensusObligation>,
    pub nodes: Vec<DeclaredNodeRecord>,
    pub edges: Vec<DeclaredEdgeRecord>,
    pub certificate: CertificateRecord,
}

impl CensusAtlas {
    pub fn canonicalize(&mut self) {
        self.facts.sort();
        self.facts.dedup();
        self.obligations.sort();
        self.obligations.dedup();
        self.nodes.sort();
        self.nodes.dedup();
        self.edges.sort();
        self.edges.dedup();
        self.certificate.blockers.sort();
        self.certificate.blockers.dedup();
    }

    fn is_canonical(&self) -> bool {
        strictly_sorted(&self.facts)
            && strictly_sorted(&self.obligations)
            && strictly_sorted(&self.nodes)
            && strictly_sorted(&self.edges)
            && strictly_sorted(&self.certificate.blockers)
    }
}

fn strictly_sorted<T: Ord>(items: &[T]) -> bool {
    items.windows(2).all(|pair| pair[0] < pair[1])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtlasError {
    /// The writer's input is not canonical or claims a seal no gate produced.
    Refused(String),
    /// The bytes are not a valid container; trust is rejected (contract: reader verification).
    Rejected(String),
}

impl std::fmt::Display for AtlasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused(why) => write!(f, "ATLAS_WRITE_REFUSED: {why}"),
            Self::Rejected(why) => write!(f, "ATLAS_REJECTED: {why}"),
        }
    }
}

fn reject<T>(why: impl Into<String>) -> Result<T, AtlasError> {
    Err(AtlasError::Rejected(why.into()))
}

// ---- encoding -----------------------------------------------------------------------------------

fn uvarint(mut value: u64, out: &mut Vec<u8>) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

struct Record {
    kind: u16,
    payload: Vec<u8>,
}

impl Record {
    fn new(kind: u16) -> Self {
        Self {
            kind,
            payload: Vec::new(),
        }
    }

    fn field(mut self, tag: u16, wire: u8, flags: u8, bytes: &[u8]) -> Self {
        self.payload.extend_from_slice(&tag.to_le_bytes());
        self.payload.push(wire);
        self.payload.push(flags);
        self.payload
            .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        self.payload.extend_from_slice(bytes);
        self
    }

    fn utf8(self, tag: u16, value: &str) -> Self {
        self.field(tag, WIRE_UTF8, REQUIRED, value.as_bytes())
    }

    fn uvarint(self, tag: u16, value: u64) -> Self {
        let mut bytes = Vec::new();
        uvarint(value, &mut bytes);
        self.field(tag, WIRE_UVARINT, REQUIRED, &bytes)
    }

    fn hash(self, tag: u16, value: &[u8; 32]) -> Self {
        self.field(tag, WIRE_HASH32, REQUIRED, value)
    }

    fn index(self, tag: u16, strings: &Strings, value: &str, flags: u8) -> Self {
        let mut bytes = Vec::new();
        uvarint(strings.index(value), &mut bytes);
        self.field(tag, WIRE_LOCAL_INDEX, flags, &bytes)
    }

    fn encode(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.kind.to_le_bytes());
        out.extend_from_slice(&RECORD_SCHEMA_VERSION.to_le_bytes());
        out.extend_from_slice(&(self.payload.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.payload);
    }
}

struct Strings(BTreeMap<String, u64>);

impl Strings {
    fn of(atlas: &CensusAtlas) -> Self {
        let mut all: Vec<&str> = Vec::new();
        for f in &atlas.facts {
            all.extend(
                [
                    &f.id,
                    &f.kind,
                    &f.status,
                    &f.subject,
                    &f.predicate,
                    &f.object,
                    &f.source_path,
                    &f.extractor,
                ]
                .map(String::as_str),
            );
            all.extend(f.revision.as_deref());
            all.extend(f.span.as_deref());
        }
        for o in &atlas.obligations {
            all.extend(
                [
                    &o.artifact,
                    &o.dimension,
                    &o.extractor,
                    &o.extractor_version,
                    &o.status,
                ]
                .map(String::as_str),
            );
        }
        for n in &atlas.nodes {
            all.extend([&n.name, &n.kind, &n.origin].map(String::as_str));
        }
        for e in &atlas.edges {
            all.extend([&e.from, &e.relation, &e.to].map(String::as_str));
        }
        all.push(&atlas.certificate.certificate_id);
        all.push(&atlas.certificate.state);
        all.extend(atlas.certificate.blockers.iter().map(String::as_str));
        all.sort_unstable();
        all.dedup();
        Self(
            all.into_iter()
                .enumerate()
                .map(|(i, s)| (s.to_owned(), i as u64))
                .collect(),
        )
    }

    fn index(&self, value: &str) -> u64 {
        self.0[value]
    }
}

struct Section {
    kind: u16,
    content: Vec<u8>,
    records: u64,
}

fn section(kind: u16, records: Vec<Record>) -> Section {
    let count = records.len() as u64;
    let mut content = Vec::new();
    for record in records {
        record.encode(&mut content);
    }
    Section {
        kind,
        content,
        records: count,
    }
}

/// Encodes a canonical, unsealed census container. The output is a pure function of `atlas`.
pub fn write(atlas: &CensusAtlas) -> Result<Vec<u8>, AtlasError> {
    if atlas.manifest.seal != UNSEALED {
        return Err(AtlasError::Refused(format!(
            "seal status `{}`: no seal gate exists, only {UNSEALED} may be written",
            atlas.manifest.seal
        )));
    }
    if !atlas.is_canonical() {
        return Err(AtlasError::Refused(
            "records are not in canonical order (call canonicalize)".into(),
        ));
    }
    Ok(encode(atlas))
}

/// The encoding itself, without `write`'s refusals (tests use it to build hostile containers).
fn encode(atlas: &CensusAtlas) -> Vec<u8> {
    let strings = Strings::of(atlas);
    let mut body = vec![
        section(
            STRING_TABLE,
            strings
                .0
                .keys()
                .map(|s| Record::new(1).utf8(1, s))
                .collect(),
        ),
        section(
            SEMANTIC_RECORDS,
            atlas
                .facts
                .iter()
                .map(|f| {
                    let mut r = Record::new(1)
                        .index(1, &strings, &f.id, REQUIRED)
                        .index(2, &strings, &f.kind, REQUIRED)
                        .index(3, &strings, &f.status, REQUIRED)
                        .index(4, &strings, &f.subject, REQUIRED)
                        .index(5, &strings, &f.predicate, REQUIRED)
                        .index(6, &strings, &f.object, REQUIRED)
                        .index(7, &strings, &f.source_path, REQUIRED);
                    if let Some(revision) = f.revision.as_deref() {
                        r = r.index(8, &strings, revision, 0);
                    }
                    r = r.index(9, &strings, &f.extractor, REQUIRED);
                    if let Some(span) = f.span.as_deref() {
                        r = r.index(10, &strings, span, 0);
                    }
                    r
                })
                .collect(),
        ),
        section(
            GRAPH_NODES,
            atlas
                .nodes
                .iter()
                .map(|n| {
                    Record::new(1)
                        .index(1, &strings, &n.name, REQUIRED)
                        .index(2, &strings, &n.kind, REQUIRED)
                        .index(3, &strings, &n.origin, REQUIRED)
                })
                .collect(),
        ),
        section(
            GRAPH_EDGES,
            atlas
                .edges
                .iter()
                .map(|e| {
                    Record::new(1)
                        .index(1, &strings, &e.from, REQUIRED)
                        .index(2, &strings, &e.relation, REQUIRED)
                        .index(3, &strings, &e.to, REQUIRED)
                })
                .collect(),
        ),
        section(
            OBLIGATIONS,
            atlas
                .obligations
                .iter()
                .map(|o| {
                    Record::new(1)
                        .index(1, &strings, &o.artifact, REQUIRED)
                        .index(2, &strings, &o.dimension, REQUIRED)
                        .index(3, &strings, &o.extractor, REQUIRED)
                        .index(4, &strings, &o.extractor_version, REQUIRED)
                        .index(5, &strings, &o.status, REQUIRED)
                })
                .collect(),
        ),
        section(
            CENSUS_CERTIFICATE,
            std::iter::once(
                Record::new(1)
                    .index(1, &strings, &atlas.certificate.certificate_id, REQUIRED)
                    .index(2, &strings, &atlas.certificate.state, REQUIRED),
            )
            .chain(
                atlas
                    .certificate
                    .blockers
                    .iter()
                    .map(|b| Record::new(2).index(1, &strings, b, REQUIRED)),
            )
            .collect(),
        ),
    ];
    let m = &atlas.manifest;
    let mut manifest = vec![
        Record::new(1)
            .uvarint(1, u64::from(FORMAT_MAJOR))
            .utf8(2, &m.genome_schema)
            .hash(3, &m.genome_hash)
            .hash(4, &m.census_digest)
            .utf8(5, &m.revision)
            .utf8(6, &m.certificate_id)
            .utf8(7, &m.seal)
            .utf8(8, &m.tool)
            .utf8(9, &m.mode),
    ];
    for s in &body {
        manifest.push(
            Record::new(2)
                .uvarint(1, u64::from(s.kind))
                .uvarint(2, schema_id(s.kind))
                .hash(3, &blake3::hash(&s.content))
                .uvarint(4, s.records),
        );
    }
    let mut sections = vec![section(ROOT_MANIFEST, manifest)];
    sections.append(&mut body);

    let mut out = vec![0u8; HEADER_LEN];
    let mut directory = Vec::new();
    for s in &sections {
        let offset = out.len() as u64;
        out.extend_from_slice(&s.content);
        let len = s.content.len() as u64;
        directory.extend_from_slice(&s.kind.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes()); // section_flags
        directory.extend_from_slice(&COMPRESSION_NONE.to_le_bytes()); // codec
        directory.extend_from_slice(&0u16.to_le_bytes()); // reserved
        directory.extend_from_slice(&schema_id(s.kind).to_le_bytes());
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(&len.to_le_bytes()); // encoded_length
        directory.extend_from_slice(&len.to_le_bytes()); // decoded_length
        directory.extend_from_slice(&s.records.to_le_bytes());
        directory.extend_from_slice(&blake3::hash(&s.content));
    }
    let directory_offset = out.len() as u64;
    out.extend_from_slice(&directory);
    let header = &mut out[..HEADER_LEN];
    header[0..8].copy_from_slice(&MAGIC);
    header[8..10].copy_from_slice(&(HEADER_LEN as u16).to_le_bytes());
    header[10..12].copy_from_slice(&FORMAT_MAJOR.to_le_bytes());
    header[12..14].copy_from_slice(&FORMAT_MINOR.to_le_bytes());
    header[14..16].copy_from_slice(&FLAG_UNSEALED.to_le_bytes());
    header[16..18].copy_from_slice(&DIGEST_BLAKE3_256.to_le_bytes());
    header[18..20].copy_from_slice(&COMPRESSION_NONE.to_le_bytes());
    header[20..52].copy_from_slice(&m.genome_hash);
    header[52..60].copy_from_slice(&directory_offset.to_le_bytes());
    header[60..68].copy_from_slice(&(directory.len() as u64).to_le_bytes());
    out
}

// ---- decoding and verification ------------------------------------------------------------------

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().expect("2 bytes"))
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4 bytes"))
}

fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().expect("8 bytes"))
}

fn read_uvarint(bytes: &[u8]) -> Result<u64, AtlasError> {
    let mut value: u64 = 0;
    for (i, byte) in bytes.iter().enumerate() {
        if i == 9 && *byte > 1 {
            return reject("uvarint overflows u64");
        }
        value |= u64::from(byte & 0x7f) << (7 * i);
        if byte & 0x80 == 0 {
            if i + 1 != bytes.len() {
                return reject("uvarint field has trailing bytes");
            }
            if i > 0 && *byte == 0 {
                return reject("uvarint is not minimally encoded");
            }
            return Ok(value);
        }
        if i == 9 {
            return reject("uvarint longer than 10 bytes");
        }
    }
    reject("truncated uvarint")
}

struct Field<'a> {
    tag: u16,
    wire: u8,
    flags: u8,
    bytes: &'a [u8],
}

struct ParsedRecord<'a> {
    kind: u16,
    fields: Vec<Field<'a>>,
}

fn parse_records(content: &[u8], section: u16) -> Result<Vec<ParsedRecord<'_>>, AtlasError> {
    let mut records = Vec::new();
    let mut at = 0usize;
    while at < content.len() {
        if content.len() - at < 8 {
            return reject(format!("section {section}: truncated record header"));
        }
        let kind = u16_at(content, at);
        let version = u16_at(content, at + 2);
        if version != RECORD_SCHEMA_VERSION {
            return reject(format!(
                "section {section}: record schema version {version}"
            ));
        }
        let len = u32_at(content, at + 4) as usize;
        let start = at + 8;
        let Some(end) = start.checked_add(len).filter(|end| *end <= content.len()) else {
            return reject(format!("section {section}: record payload out of bounds"));
        };
        let payload = &content[start..end];
        let mut fields = Vec::new();
        let mut p = 0usize;
        while p < payload.len() {
            if payload.len() - p < 8 {
                return reject(format!("section {section}: truncated field header"));
            }
            let tag = u16_at(payload, p);
            let wire = payload[p + 2];
            let flags = payload[p + 3];
            let flen = u32_at(payload, p + 4) as usize;
            let fstart = p + 8;
            let Some(fend) = fstart.checked_add(flen).filter(|e| *e <= payload.len()) else {
                return reject(format!("section {section}: field out of bounds"));
            };
            if fields.last().is_some_and(|last: &Field| last.tag >= tag) {
                return reject(format!(
                    "section {section}: field tags not strictly ascending"
                ));
            }
            fields.push(Field {
                tag,
                wire,
                flags,
                bytes: &payload[fstart..fend],
            });
            p = fend;
        }
        records.push(ParsedRecord { kind, fields });
        at = end;
    }
    Ok(records)
}

/// Typed access to one record's fields: every known tag is checked for its wire type, missing
/// required fields and unknown required fields reject, unknown optional fields are skipped.
struct Fields<'a, 'r> {
    record: &'r ParsedRecord<'a>,
    known: &'static [u16],
    section: u16,
}

impl<'a> Fields<'a, '_> {
    fn new<'r>(
        record: &'r ParsedRecord<'a>,
        known: &'static [u16],
        section: u16,
    ) -> Result<Fields<'a, 'r>, AtlasError> {
        for field in &record.fields {
            if !known.contains(&field.tag) && field.flags & REQUIRED != 0 {
                return reject(format!(
                    "section {section}: unknown required field {}",
                    field.tag
                ));
            }
        }
        Ok(Fields {
            record,
            known,
            section,
        })
    }

    fn raw(&self, tag: u16, wire: u8) -> Result<Option<&'a [u8]>, AtlasError> {
        debug_assert!(self.known.contains(&tag));
        match self.record.fields.iter().find(|f| f.tag == tag) {
            None => Ok(None),
            Some(f) if f.wire != wire => reject(format!(
                "section {}: field {tag} has wire type {}, expected {wire}",
                self.section, f.wire
            )),
            Some(f) => Ok(Some(f.bytes)),
        }
    }

    fn required(&self, tag: u16, wire: u8) -> Result<&'a [u8], AtlasError> {
        match self.raw(tag, wire)? {
            Some(bytes) => Ok(bytes),
            None => reject(format!(
                "section {}: missing required field {tag}",
                self.section
            )),
        }
    }

    fn utf8(&self, tag: u16) -> Result<String, AtlasError> {
        String::from_utf8(self.required(tag, WIRE_UTF8)?.to_vec()).or_else(|_| {
            reject(format!(
                "section {}: field {tag} is not UTF-8",
                self.section
            ))
        })
    }

    fn uvarint(&self, tag: u16) -> Result<u64, AtlasError> {
        read_uvarint(self.required(tag, WIRE_UVARINT)?)
    }

    fn hash(&self, tag: u16) -> Result<[u8; 32], AtlasError> {
        self.required(tag, WIRE_HASH32)?.try_into().or_else(|_| {
            reject(format!(
                "section {}: field {tag} is not 32 bytes",
                self.section
            ))
        })
    }

    fn string(&self, tag: u16, strings: &[String]) -> Result<String, AtlasError> {
        self.optional_string(tag, strings)?.map_or_else(
            || {
                reject(format!(
                    "section {}: missing required field {tag}",
                    self.section
                ))
            },
            Ok,
        )
    }

    fn optional_string(&self, tag: u16, strings: &[String]) -> Result<Option<String>, AtlasError> {
        let Some(bytes) = self.raw(tag, WIRE_LOCAL_INDEX)? else {
            return Ok(None);
        };
        let index = read_uvarint(bytes)?;
        match usize::try_from(index).ok().and_then(|i| strings.get(i)) {
            Some(value) => Ok(Some(value.clone())),
            None => reject(format!(
                "section {}: string index {index} outside the string table",
                self.section
            )),
        }
    }
}

fn expect_kind(record: &ParsedRecord, kinds: &[u16], section: u16) -> Result<(), AtlasError> {
    if kinds.contains(&record.kind) {
        Ok(())
    } else {
        reject(format!(
            "section {section}: unknown record kind {}",
            record.kind
        ))
    }
}

struct Entry {
    kind: u16,
    schema: u64,
    offset: u64,
    length: u64,
    records: u64,
    hash: [u8; 32],
}

/// The verified root identity of a container: BLAKE3 over its ROOT_MANIFEST content.
pub fn root_identity(manifest_content: &[u8]) -> IntegrityDigest {
    IntegrityDigest::blake3_256(&blake3::hash(manifest_content))
}

/// Verifies `bytes` in the contract's reader order and decodes it. Any failed step rejects trust.
pub fn read(bytes: &[u8]) -> Result<(CensusAtlas, IntegrityDigest), AtlasError> {
    // 1. magic, version, header bounds.
    if bytes.len() < HEADER_LEN {
        return reject("shorter than the 72-byte header");
    }
    if bytes[0..8] != MAGIC {
        return reject("bad magic");
    }
    if usize::from(u16_at(bytes, 8)) != HEADER_LEN {
        return reject("header_len is not 72");
    }
    let major = u16_at(bytes, 10);
    if major != FORMAT_MAJOR {
        return reject(format!("unknown format major version {major}"));
    }
    let flags = u16_at(bytes, 14);
    if flags & !FLAG_UNSEALED != 0 {
        return reject(format!("unknown header flags {flags:#06x}"));
    }
    // 4. algorithm support.
    if u16_at(bytes, 16) != DIGEST_BLAKE3_256 {
        return reject("unsupported digest algorithm");
    }
    if u16_at(bytes, 18) != COMPRESSION_NONE {
        return reject("unsupported default compression");
    }
    if u32_at(bytes, 68) != 0 {
        return reject("reserved header bytes are not zero");
    }
    let genome_hash: [u8; 32] = bytes[20..52].try_into().expect("32 bytes");
    // 2. directory bounds.
    let dir_offset = u64_at(bytes, 52);
    let dir_len = u64_at(bytes, 60);
    let file_len = bytes.len() as u64;
    let Some(dir_end) = dir_offset.checked_add(dir_len).filter(|e| *e <= file_len) else {
        return reject("section directory out of bounds");
    };
    if dir_offset < HEADER_LEN as u64 || !dir_len.is_multiple_of(DIRECTORY_ENTRY_LEN as u64) {
        return reject("malformed section directory");
    }
    // 3. entry bounds, ordering, non-overlap.
    let mut entries = Vec::new();
    let mut regions = vec![(0u64, HEADER_LEN as u64), (dir_offset, dir_end)];
    for chunk in bytes[dir_offset as usize..dir_end as usize].chunks_exact(DIRECTORY_ENTRY_LEN) {
        if u16_at(chunk, 2) != 0 || u16_at(chunk, 6) != 0 {
            return reject("non-zero section flags or reserved field");
        }
        if u16_at(chunk, 4) != COMPRESSION_NONE {
            return reject("unsupported section codec");
        }
        let entry = Entry {
            kind: u16_at(chunk, 0),
            schema: u64_at(chunk, 8),
            offset: u64_at(chunk, 16),
            length: u64_at(chunk, 24),
            records: u64_at(chunk, 40),
            hash: chunk[48..80].try_into().expect("32 bytes"),
        };
        if u64_at(chunk, 32) != entry.length {
            return reject("decoded length differs from encoded length without a codec");
        }
        let Some(end) = entry
            .offset
            .checked_add(entry.length)
            .filter(|e| *e <= file_len)
        else {
            return reject(format!("section {} out of bounds", entry.kind));
        };
        regions.push((entry.offset, end));
        entries.push(entry);
    }
    let sort_key = |e: &Entry| (e.kind, e.schema, e.hash, e.offset);
    if entries
        .windows(2)
        .any(|pair| sort_key(&pair[0]) >= sort_key(&pair[1]))
    {
        return reject("section directory is not in canonical order");
    }
    regions.sort_unstable();
    // Canonical layout: header, sections and directory tile the file exactly -- no overlap, no
    // gap, no trailing bytes that no hash covers.
    let mut covered = 0u64;
    for (start, end) in &regions {
        if *start != covered {
            return reject(if *start < covered {
                "overlapping regions"
            } else {
                "uncovered bytes between regions"
            });
        }
        covered = *end;
    }
    if covered != file_len {
        return reject("trailing bytes after the last region");
    }
    // 5. exactly one root manifest; 7/8. section hashes and record framing.
    let content = |e: &Entry| &bytes[e.offset as usize..(e.offset + e.length) as usize];
    for e in &entries {
        if blake3::hash(content(e)) != e.hash {
            return reject(format!("section {} content hash mismatch", e.kind));
        }
        if e.schema != schema_id(e.kind) {
            return reject(format!("section {} has an unknown schema id", e.kind));
        }
    }
    let one = |kind: u16| -> Result<&Entry, AtlasError> {
        let mut found = entries.iter().filter(|e| e.kind == kind);
        match (found.next(), found.next()) {
            (Some(e), None) => Ok(e),
            (None, _) => reject(format!("required section {kind} absent")),
            _ => reject(format!("section {kind} appears more than once")),
        }
    };
    let manifest_entry = one(ROOT_MANIFEST)?;
    let manifest_records = parse_records(content(manifest_entry), ROOT_MANIFEST)?;
    if manifest_records.len() as u64 != manifest_entry.records {
        return reject("root manifest record count mismatch");
    }
    let Some((head, commitments)) = manifest_records.split_first() else {
        return reject("empty root manifest");
    };
    expect_kind(head, &[1], ROOT_MANIFEST)?;
    let f = Fields::new(head, &[1, 2, 3, 4, 5, 6, 7, 8, 9], ROOT_MANIFEST)?;
    if f.uvarint(1)? != u64::from(FORMAT_MAJOR) {
        return reject("root manifest wire version mismatch");
    }
    let manifest = RootManifest {
        genome_schema: f.utf8(2)?,
        genome_hash: f.hash(3)?,
        census_digest: f.hash(4)?,
        revision: f.utf8(5)?,
        certificate_id: f.utf8(6)?,
        seal: f.utf8(7)?,
        tool: f.utf8(8)?,
        mode: f.utf8(9)?,
    };
    // 6. Genome/schema compatibility and the seal marker.
    if manifest.genome_hash != genome_hash {
        return reject("header genome hash differs from the root manifest");
    }
    if manifest.seal != UNSEALED || flags & FLAG_UNSEALED == 0 {
        return reject("only unsealed census containers are defined; a seal needs a seal gate");
    }
    // The manifest commits to exactly the other sections.
    let mut committed = Vec::new();
    for record in commitments {
        expect_kind(record, &[2], ROOT_MANIFEST)?;
        let c = Fields::new(record, &[1, 2, 3, 4], ROOT_MANIFEST)?;
        committed.push((c.uvarint(1)?, c.uvarint(2)?, c.hash(3)?, c.uvarint(4)?));
    }
    let actual: Vec<(u64, u64, [u8; 32], u64)> = entries
        .iter()
        .filter(|e| e.kind != ROOT_MANIFEST)
        .map(|e| (u64::from(e.kind), e.schema, e.hash, e.records))
        .collect();
    if committed != actual {
        return reject("root manifest does not commit to exactly the directory's sections");
    }
    // Required sections, framing, record counts and canonical order.
    let records_of = |kind: u16| -> Result<Vec<ParsedRecord<'_>>, AtlasError> {
        let e = one(kind)?;
        let records = parse_records(content(e), kind)?;
        if records.len() as u64 != e.records {
            return reject(format!("section {kind} record count mismatch"));
        }
        Ok(records)
    };
    let mut strings = Vec::new();
    for r in records_of(STRING_TABLE)? {
        expect_kind(&r, &[1], STRING_TABLE)?;
        strings.push(Fields::new(&r, &[1], STRING_TABLE)?.utf8(1)?);
    }
    if !strictly_sorted(&strings) {
        return reject("string table is not sorted and unique");
    }
    let mut facts = Vec::new();
    for r in records_of(SEMANTIC_RECORDS)? {
        expect_kind(&r, &[1], SEMANTIC_RECORDS)?;
        let f = Fields::new(&r, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], SEMANTIC_RECORDS)?;
        facts.push(CensusFact {
            id: f.string(1, &strings)?,
            kind: f.string(2, &strings)?,
            status: f.string(3, &strings)?,
            subject: f.string(4, &strings)?,
            predicate: f.string(5, &strings)?,
            object: f.string(6, &strings)?,
            source_path: f.string(7, &strings)?,
            revision: f.optional_string(8, &strings)?,
            extractor: f.string(9, &strings)?,
            span: f.optional_string(10, &strings)?,
        });
    }
    let mut nodes = Vec::new();
    for r in records_of(GRAPH_NODES)? {
        expect_kind(&r, &[1], GRAPH_NODES)?;
        let f = Fields::new(&r, &[1, 2, 3], GRAPH_NODES)?;
        nodes.push(DeclaredNodeRecord {
            name: f.string(1, &strings)?,
            kind: f.string(2, &strings)?,
            origin: f.string(3, &strings)?,
        });
    }
    let mut edges = Vec::new();
    for r in records_of(GRAPH_EDGES)? {
        expect_kind(&r, &[1], GRAPH_EDGES)?;
        let f = Fields::new(&r, &[1, 2, 3], GRAPH_EDGES)?;
        edges.push(DeclaredEdgeRecord {
            from: f.string(1, &strings)?,
            relation: f.string(2, &strings)?,
            to: f.string(3, &strings)?,
        });
    }
    let mut obligations = Vec::new();
    for r in records_of(OBLIGATIONS)? {
        expect_kind(&r, &[1], OBLIGATIONS)?;
        let f = Fields::new(&r, &[1, 2, 3, 4, 5], OBLIGATIONS)?;
        obligations.push(CensusObligation {
            artifact: f.string(1, &strings)?,
            dimension: f.string(2, &strings)?,
            extractor: f.string(3, &strings)?,
            extractor_version: f.string(4, &strings)?,
            status: f.string(5, &strings)?,
        });
    }
    let certificate_records = records_of(CENSUS_CERTIFICATE)?;
    let Some((cert_head, blocker_records)) = certificate_records.split_first() else {
        return reject("empty certificate section");
    };
    expect_kind(cert_head, &[1], CENSUS_CERTIFICATE)?;
    let c = Fields::new(cert_head, &[1, 2], CENSUS_CERTIFICATE)?;
    let mut certificate = CertificateRecord {
        certificate_id: c.string(1, &strings)?,
        state: c.string(2, &strings)?,
        blockers: Vec::new(),
    };
    for r in blocker_records {
        expect_kind(r, &[2], CENSUS_CERTIFICATE)?;
        certificate
            .blockers
            .push(Fields::new(r, &[1], CENSUS_CERTIFICATE)?.string(1, &strings)?);
    }
    if certificate.certificate_id != manifest.certificate_id {
        return reject("certificate section differs from the root manifest's certificate id");
    }
    let atlas = CensusAtlas {
        manifest,
        facts,
        obligations,
        nodes,
        edges,
        certificate,
    };
    if !atlas.is_canonical() {
        return reject("records are not in canonical order");
    }
    Ok((atlas, root_identity(content(manifest_entry))))
}

#[cfg(test)]
mod tests;
