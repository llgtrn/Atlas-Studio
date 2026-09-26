//! Declared record schemas of the census container and their derived identities (G68, ADR 0030).
//!
//! Absorbed from Glean (first-50 #9): a schema's identity is a hash of its definition and of the
//! definitions it depends on (`glean/db/Glean/Database/Schema/ComputeIds.hs`: `hashBinary (ref,
//! definition)`, strongly-connected groups folded through one cycle hash). G64 derived
//! `schema_id` from the section's *name* only, so repurposing a field tag in writer and reader
//! together left the id unchanged and an older reader silently decoded the new meaning (measured:
//! subject/predicate swapped, `atlas verify` exit 0). Here every field is addressed by name through
//! these tables, the only place a name meets a tag, and `schema_id` hashes the tables: any change
//! to a field's tag, name, wire type or requiredness, or to a table a section depends on, is a new
//! incompatible schema identity, which the reader refuses.
//!
//! The dependency graph here is acyclic (every section depends at most on the string table), so
//! Glean's cycle folding is not needed; a cycle would be a programming error caught by the test
//! that computes every schema id.

use super::{
    CENSUS_CERTIFICATE, DIAGNOSTICS, EVIDENCE, GRAPH_EDGES, GRAPH_NODES, OBLIGATIONS,
    ROOT_MANIFEST, SEMANTIC_RECORDS, STRING_TABLE, WIRE_BOOL, WIRE_HASH32, WIRE_LOCAL_INDEX,
    WIRE_PACKED, WIRE_RECORD, WIRE_UTF8, WIRE_UVARINT,
};
use crate::identity::blake3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldDef {
    pub name: &'static str,
    pub tag: u16,
    pub wire: u8,
    pub required: bool,
    /// G147: for a `RECORD` field, the embedded record kind (declared in the same section).
    pub nested: u16,
    /// G147: the field holds a sequence (a `RECORD` field of several embedded records, or a
    /// `PACKED` field of several scalars) rather than one value.
    pub repeated: bool,
    /// G147: for a `PACKED` field, the wire type of each element.
    pub element: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordDef {
    pub kind: u16,
    pub name: &'static str,
    pub fields: &'static [FieldDef],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionDef {
    pub section: u16,
    pub name: &'static str,
    pub records: &'static [RecordDef],
    /// Sections whose encoding this one's fields reference (string indices).
    pub depends_on: &'static [u16],
}

const fn req(name: &'static str, tag: u16, wire: u8) -> FieldDef {
    FieldDef {
        name,
        tag,
        wire,
        required: true,
        nested: 0,
        repeated: false,
        element: 0,
    }
}

const fn opt(name: &'static str, tag: u16, wire: u8) -> FieldDef {
    FieldDef {
        name,
        tag,
        wire,
        required: false,
        nested: 0,
        repeated: false,
        element: 0,
    }
}

/// G147: one embedded record of `kind` (required).
const fn one(name: &'static str, tag: u16, kind: u16) -> FieldDef {
    FieldDef {
        nested: kind,
        ..req(name, tag, WIRE_RECORD)
    }
}

/// G147: an optional embedded record of `kind` (an `Option` of a struct or enum).
const fn maybe(name: &'static str, tag: u16, kind: u16) -> FieldDef {
    FieldDef {
        nested: kind,
        ..opt(name, tag, WIRE_RECORD)
    }
}

/// G147: a sequence of embedded records of `kind`; an empty sequence is an absent field.
const fn many(name: &'static str, tag: u16, kind: u16) -> FieldDef {
    FieldDef {
        nested: kind,
        repeated: true,
        ..opt(name, tag, WIRE_RECORD)
    }
}

/// G147: a sequence of strings (string-table indices); an empty sequence is an absent field.
const fn strings(name: &'static str, tag: u16) -> FieldDef {
    FieldDef {
        repeated: true,
        element: WIRE_LOCAL_INDEX,
        ..opt(name, tag, WIRE_PACKED)
    }
}

const fn typed(name: &'static str, kind: u16, fields: &'static [FieldDef]) -> RecordDef {
    RecordDef { kind, name, fields }
}

// G147: embedded-only record kinds of the SEMANTIC_RECORDS section.
pub const REVISION: u16 = 100;
pub const SCOPE: u16 = 101;
pub const EXTRACTOR: u16 = 102;
pub const PROVENANCE: u16 = 103;
pub const SPAN: u16 = 104;
pub const PLACE_REF: u16 = 105;
pub const PLACE_RESOLVED: u16 = 106;
pub const DOCUMENTATION: u16 = 107;
pub const TYPE_IDENTITY: u16 = 108;
pub const SYMBOL_IDENTITY: u16 = 109;
pub const FUNCTION_OWNER: u16 = 110;
pub const FUNCTION_IDENTITY: u16 = 111;
pub const FUNCTION_PARAMETER: u16 = 112;
pub const FUNCTION_SIGNATURE: u16 = 113;
pub const CALL_SITE: u16 = 114;
pub const CONTROL_FLOW_EDGE: u16 = 115;
pub const CONTROL_FLOW_BLOCK: u16 = 116;
pub const VALUE: u16 = 117;
pub const STATE_ACCESS: u16 = 118;
pub const EFFECT: u16 = 119;
pub const OWNERSHIP: u16 = 120;
pub const CONCURRENCY: u16 = 121;
pub const PERSISTENCE: u16 = 122;
/// G153 (ADR 0069): the declared shape of a type, variant or field definition.
pub const DECLARATION: u16 = 123;
/// Record kinds at and above this one are only ever embedded in a `RECORD` field.
pub const FIRST_EMBEDDED_KIND: u16 = 100;

/// G147: the common header of every typed semantic record (`SemanticRecordHeader`), embedding
/// the family's `subject` kind.
const fn header(subject: u16) -> [FieldDef; 10] {
    [
        req("record_id", 1, WIRE_LOCAL_INDEX),
        req("dimension", 2, WIRE_LOCAL_INDEX),
        req("status", 3, WIRE_LOCAL_INDEX),
        one("subject", 4, subject),
        one("scope", 5, SCOPE),
        req("repository", 6, WIRE_LOCAL_INDEX),
        one("revision", 7, REVISION),
        one("extractor", 8, EXTRACTOR),
        strings("evidence_refs", 9),
        one("provenance", 10, PROVENANCE),
    ]
}

const HEADER_FUNCTION_IDENTITY: [FieldDef; 10] = header(FUNCTION_IDENTITY);
const HEADER_FUNCTION_SIGNATURE: [FieldDef; 10] = header(FUNCTION_SIGNATURE);
const HEADER_SYMBOL: [FieldDef; 10] = header(SYMBOL_IDENTITY);
const HEADER_TYPE: [FieldDef; 10] = header(TYPE_IDENTITY);
const HEADER_CALL: [FieldDef; 10] = header(CALL_SITE);
const HEADER_CONTROL_FLOW: [FieldDef; 10] = header(CONTROL_FLOW_BLOCK);
const HEADER_DATA_FLOW: [FieldDef; 10] = header(VALUE);
const HEADER_STATE: [FieldDef; 10] = header(STATE_ACCESS);
const HEADER_EFFECT: [FieldDef; 10] = header(EFFECT);
const HEADER_OWNERSHIP: [FieldDef; 10] = header(OWNERSHIP);
const HEADER_CONCURRENCY: [FieldDef; 10] = header(CONCURRENCY);
const HEADER_PERSISTENCE: [FieldDef; 10] = header(PERSISTENCE);

pub const SECTIONS: &[SectionDef] = &[
    SectionDef {
        section: ROOT_MANIFEST,
        name: "root-manifest",
        records: &[
            RecordDef {
                kind: 1,
                name: "manifest",
                fields: &[
                    req("wire_version", 1, WIRE_UVARINT),
                    req("genome_schema", 2, WIRE_UTF8),
                    req("genome_hash", 3, WIRE_HASH32),
                    req("census_digest", 4, WIRE_HASH32),
                    req("revision", 5, WIRE_UTF8),
                    req("certificate_id", 6, WIRE_UTF8),
                    req("seal", 7, WIRE_UTF8),
                    req("tool", 8, WIRE_UTF8),
                    req("mode", 9, WIRE_UTF8),
                ],
            },
            RecordDef {
                kind: 2,
                name: "section-commitment",
                fields: &[
                    req("section_type", 1, WIRE_UVARINT),
                    req("schema_id", 2, WIRE_UVARINT),
                    req("content_hash", 3, WIRE_HASH32),
                    req("record_count", 4, WIRE_UVARINT),
                ],
            },
        ],
        depends_on: &[],
    },
    SectionDef {
        section: STRING_TABLE,
        name: "string-table",
        records: &[RecordDef {
            kind: 1,
            name: "string",
            fields: &[req("value", 1, WIRE_UTF8)],
        }],
        depends_on: &[],
    },
    SectionDef {
        section: SEMANTIC_RECORDS,
        name: "census-facts",
        records: &[
            RecordDef {
                kind: 1,
                name: "fact",
                fields: &[
                    req("id", 1, WIRE_LOCAL_INDEX),
                    req("kind", 2, WIRE_LOCAL_INDEX),
                    req("status", 3, WIRE_LOCAL_INDEX),
                    req("subject", 4, WIRE_LOCAL_INDEX),
                    req("predicate", 5, WIRE_LOCAL_INDEX),
                    req("object", 6, WIRE_LOCAL_INDEX),
                    req("source_path", 7, WIRE_LOCAL_INDEX),
                    opt("revision", 8, WIRE_LOCAL_INDEX),
                    req("extractor", 9, WIRE_LOCAL_INDEX),
                    opt("span", 10, WIRE_LOCAL_INDEX),
                ],
            },
            // G147 (ADR 0063): every typed semantic record, one kind per observation family, named
            // exactly as the family is tagged in its serde form; a shared header embeds the
            // family's subject. Kinds 100 and above are embedded-only.
            typed("FunctionIdentity", 2, &HEADER_FUNCTION_IDENTITY),
            typed("FunctionSignature", 3, &HEADER_FUNCTION_SIGNATURE),
            typed("Symbol", 4, &HEADER_SYMBOL),
            typed("Type", 5, &HEADER_TYPE),
            typed("Call", 6, &HEADER_CALL),
            typed("ControlFlow", 7, &HEADER_CONTROL_FLOW),
            typed("DataFlow", 8, &HEADER_DATA_FLOW),
            typed("State", 9, &HEADER_STATE),
            typed("Effect", 10, &HEADER_EFFECT),
            typed("Ownership", 11, &HEADER_OWNERSHIP),
            typed("Concurrency", 12, &HEADER_CONCURRENCY),
            typed("Persistence", 13, &HEADER_PERSISTENCE),
            typed(
                "revision",
                REVISION,
                &[
                    req("kind", 1, WIRE_LOCAL_INDEX),
                    req("value", 2, WIRE_LOCAL_INDEX),
                ],
            ),
            typed("scope", SCOPE, &[strings("segments", 1)]),
            typed(
                "extractor",
                EXTRACTOR,
                &[
                    req("id", 1, WIRE_LOCAL_INDEX),
                    req("version", 2, WIRE_LOCAL_INDEX),
                ],
            ),
            typed(
                "provenance",
                PROVENANCE,
                &[
                    req("source_path", 1, WIRE_LOCAL_INDEX),
                    maybe("source_revision", 2, REVISION),
                    req("extractor", 3, WIRE_LOCAL_INDEX),
                    opt("content_hash", 4, WIRE_LOCAL_INDEX),
                    opt("span", 5, WIRE_LOCAL_INDEX),
                ],
            ),
            typed(
                "source-span",
                SPAN,
                &[
                    req("path", 1, WIRE_LOCAL_INDEX),
                    req("line", 2, WIRE_UVARINT),
                    req("column", 3, WIRE_UVARINT),
                ],
            ),
            // An enum kind: `variant` names the variant; a variant with data carries it in the
            // field named after it.
            typed(
                "place-ref",
                PLACE_REF,
                &[
                    req("variant", 1, WIRE_LOCAL_INDEX),
                    maybe("Resolved", 2, PLACE_RESOLVED),
                ],
            ),
            typed(
                "place-resolved",
                PLACE_RESOLVED,
                &[
                    req("dimension", 1, WIRE_LOCAL_INDEX),
                    req("record_id", 2, WIRE_LOCAL_INDEX),
                ],
            ),
            typed(
                "documentation",
                DOCUMENTATION,
                &[
                    req("summary", 1, WIRE_LOCAL_INDEX),
                    req("lines", 2, WIRE_UVARINT),
                ],
            ),
            typed(
                "declaration",
                DECLARATION,
                &[
                    req("item", 1, WIRE_LOCAL_INDEX),
                    req("visibility", 2, WIRE_LOCAL_INDEX),
                    strings("derives", 3),
                    strings("attributes", 4),
                    opt("shape", 5, WIRE_LOCAL_INDEX),
                ],
            ),
            typed(
                "type-identity",
                TYPE_IDENTITY,
                &[
                    req("repository", 1, WIRE_LOCAL_INDEX),
                    one("revision", 2, REVISION),
                    one("scope", 3, SCOPE),
                    req("name", 4, WIRE_LOCAL_INDEX),
                    opt("canonical", 5, WIRE_LOCAL_INDEX),
                    req("path", 6, WIRE_LOCAL_INDEX),
                ],
            ),
            typed(
                "symbol-identity",
                SYMBOL_IDENTITY,
                &[
                    req("repository", 1, WIRE_LOCAL_INDEX),
                    one("revision", 2, REVISION),
                    one("scope", 3, SCOPE),
                    req("name", 4, WIRE_LOCAL_INDEX),
                    req("role", 5, WIRE_LOCAL_INDEX),
                    req("path", 6, WIRE_LOCAL_INDEX),
                    maybe("documentation", 7, DOCUMENTATION),
                    maybe("declaration", 8, DECLARATION),
                ],
            ),
            typed(
                "function-owner",
                FUNCTION_OWNER,
                &[
                    maybe("target", 1, TYPE_IDENTITY),
                    opt("trait_path", 2, WIRE_LOCAL_INDEX),
                ],
            ),
            typed(
                "function-identity",
                FUNCTION_IDENTITY,
                &[
                    req("repository", 1, WIRE_LOCAL_INDEX),
                    one("revision", 2, REVISION),
                    req("language", 3, WIRE_LOCAL_INDEX),
                    one("scope", 4, SCOPE),
                    one("symbol", 5, SYMBOL_IDENTITY),
                    one("span", 6, SPAN),
                    req("generated", 7, WIRE_BOOL),
                    req("declaration_kind", 8, WIRE_LOCAL_INDEX),
                    one("owner", 9, FUNCTION_OWNER),
                    strings("generics", 10),
                ],
            ),
            typed(
                "function-parameter",
                FUNCTION_PARAMETER,
                &[
                    req("name", 1, WIRE_LOCAL_INDEX),
                    one("type_identity", 2, TYPE_IDENTITY),
                ],
            ),
            typed(
                "function-signature",
                FUNCTION_SIGNATURE,
                &[
                    one("function", 1, FUNCTION_IDENTITY),
                    many("parameters", 2, FUNCTION_PARAMETER),
                    maybe("return_type", 3, TYPE_IDENTITY),
                    strings("generics", 4),
                    opt("abi", 5, WIRE_LOCAL_INDEX),
                    req("visibility", 6, WIRE_LOCAL_INDEX),
                    req("is_async", 7, WIRE_BOOL),
                    req("is_unsafe", 8, WIRE_BOOL),
                    req("is_extern", 9, WIRE_BOOL),
                    opt("body_fingerprint", 10, WIRE_LOCAL_INDEX),
                ],
            ),
            typed(
                "call-site",
                CALL_SITE,
                &[
                    req("repository", 1, WIRE_LOCAL_INDEX),
                    one("revision", 2, REVISION),
                    req("function", 3, WIRE_LOCAL_INDEX),
                    one("span", 4, SPAN),
                    req("dispatch", 5, WIRE_LOCAL_INDEX),
                    strings("callees", 6),
                    many("arguments", 7, PLACE_REF),
                    one("result", 8, PLACE_REF),
                    opt("callee_spelling", 9, WIRE_LOCAL_INDEX),
                ],
            ),
            typed(
                "control-flow-edge",
                CONTROL_FLOW_EDGE,
                &[
                    req("kind", 1, WIRE_LOCAL_INDEX),
                    opt("target", 2, WIRE_LOCAL_INDEX),
                ],
            ),
            typed(
                "control-flow-block",
                CONTROL_FLOW_BLOCK,
                &[
                    req("repository", 1, WIRE_LOCAL_INDEX),
                    one("revision", 2, REVISION),
                    req("function", 3, WIRE_LOCAL_INDEX),
                    req("block_index", 4, WIRE_UVARINT),
                    req("kind", 5, WIRE_LOCAL_INDEX),
                    req("is_entry", 6, WIRE_BOOL),
                    many("successors", 7, CONTROL_FLOW_EDGE),
                ],
            ),
            typed(
                "value",
                VALUE,
                &[
                    req("repository", 1, WIRE_LOCAL_INDEX),
                    one("revision", 2, REVISION),
                    req("function", 3, WIRE_LOCAL_INDEX),
                    req("name", 4, WIRE_LOCAL_INDEX),
                    one("span", 5, SPAN),
                    req("role", 6, WIRE_LOCAL_INDEX),
                    req("is_parameter", 7, WIRE_BOOL),
                    req("is_return_flow", 8, WIRE_BOOL),
                    req("resolution", 9, WIRE_LOCAL_INDEX),
                    opt("resolved_definition", 10, WIRE_LOCAL_INDEX),
                    strings("projection", 11),
                ],
            ),
            typed(
                "state-access",
                STATE_ACCESS,
                &[
                    req("repository", 1, WIRE_LOCAL_INDEX),
                    one("revision", 2, REVISION),
                    req("function", 3, WIRE_LOCAL_INDEX),
                    one("scope", 4, SCOPE),
                    req("name", 5, WIRE_LOCAL_INDEX),
                    one("span", 6, SPAN),
                    req("kind", 7, WIRE_LOCAL_INDEX),
                    req("resolution", 8, WIRE_LOCAL_INDEX),
                ],
            ),
            typed(
                "effect",
                EFFECT,
                &[
                    req("repository", 1, WIRE_LOCAL_INDEX),
                    one("revision", 2, REVISION),
                    req("function", 3, WIRE_LOCAL_INDEX),
                    req("category", 4, WIRE_LOCAL_INDEX),
                    one("span", 5, SPAN),
                ],
            ),
            typed(
                "ownership",
                OWNERSHIP,
                &[
                    req("repository", 1, WIRE_LOCAL_INDEX),
                    one("revision", 2, REVISION),
                    req("function", 3, WIRE_LOCAL_INDEX),
                    req("name", 4, WIRE_LOCAL_INDEX),
                    one("span", 5, SPAN),
                    req("kind", 6, WIRE_LOCAL_INDEX),
                    req("resolution", 7, WIRE_LOCAL_INDEX),
                ],
            ),
            typed(
                "concurrency",
                CONCURRENCY,
                &[
                    req("repository", 1, WIRE_LOCAL_INDEX),
                    one("revision", 2, REVISION),
                    req("function", 3, WIRE_LOCAL_INDEX),
                    req("kind", 4, WIRE_LOCAL_INDEX),
                    one("span", 5, SPAN),
                ],
            ),
            typed(
                "persistence",
                PERSISTENCE,
                &[
                    req("repository", 1, WIRE_LOCAL_INDEX),
                    one("revision", 2, REVISION),
                    req("function", 3, WIRE_LOCAL_INDEX),
                    req("kind", 4, WIRE_LOCAL_INDEX),
                    one("span", 5, SPAN),
                    one("place", 6, PLACE_REF),
                    req("resolution", 7, WIRE_LOCAL_INDEX),
                ],
            ),
        ],
        depends_on: &[STRING_TABLE],
    },
    SectionDef {
        section: GRAPH_NODES,
        name: "declared-nodes",
        records: &[RecordDef {
            kind: 1,
            name: "node",
            fields: &[
                req("name", 1, WIRE_LOCAL_INDEX),
                req("kind", 2, WIRE_LOCAL_INDEX),
                req("origin", 3, WIRE_LOCAL_INDEX),
            ],
        }],
        depends_on: &[STRING_TABLE],
    },
    SectionDef {
        section: GRAPH_EDGES,
        name: "declared-edges",
        records: &[RecordDef {
            kind: 1,
            name: "edge",
            fields: &[
                req("from", 1, WIRE_LOCAL_INDEX),
                req("relation", 2, WIRE_LOCAL_INDEX),
                req("to", 3, WIRE_LOCAL_INDEX),
            ],
        }],
        depends_on: &[STRING_TABLE],
    },
    // G147: the census's evidence records and extraction diagnostics.
    SectionDef {
        section: EVIDENCE,
        name: "evidence",
        records: &[
            RecordDef {
                kind: 1,
                name: "evidence",
                fields: &[
                    req("id", 1, WIRE_LOCAL_INDEX),
                    req("kind", 2, WIRE_LOCAL_INDEX),
                    req("path", 3, WIRE_LOCAL_INDEX),
                    req("summary", 4, WIRE_LOCAL_INDEX),
                    maybe("revision", 5, 2),
                ],
            },
            typed(
                "revision",
                2,
                &[
                    req("kind", 1, WIRE_LOCAL_INDEX),
                    req("value", 2, WIRE_LOCAL_INDEX),
                ],
            ),
        ],
        depends_on: &[STRING_TABLE],
    },
    SectionDef {
        section: DIAGNOSTICS,
        name: "diagnostics",
        records: &[RecordDef {
            kind: 1,
            name: "diagnostic",
            fields: &[
                req("id", 1, WIRE_LOCAL_INDEX),
                req("code", 2, WIRE_LOCAL_INDEX),
                opt("dimension", 3, WIRE_LOCAL_INDEX),
                req("message", 4, WIRE_LOCAL_INDEX),
            ],
        }],
        depends_on: &[STRING_TABLE],
    },
    SectionDef {
        section: OBLIGATIONS,
        name: "semantic-obligations",
        records: &[RecordDef {
            kind: 1,
            name: "obligation",
            fields: &[
                req("artifact", 1, WIRE_LOCAL_INDEX),
                req("dimension", 2, WIRE_LOCAL_INDEX),
                req("extractor", 3, WIRE_LOCAL_INDEX),
                req("extractor_version", 4, WIRE_LOCAL_INDEX),
                req("status", 5, WIRE_LOCAL_INDEX),
            ],
        }],
        depends_on: &[STRING_TABLE],
    },
    SectionDef {
        section: CENSUS_CERTIFICATE,
        name: "census-certificate",
        records: &[
            RecordDef {
                kind: 1,
                name: "certificate",
                fields: &[
                    req("certificate_id", 1, WIRE_LOCAL_INDEX),
                    req("state", 2, WIRE_LOCAL_INDEX),
                ],
            },
            RecordDef {
                kind: 2,
                name: "blocker",
                fields: &[req("text", 1, WIRE_LOCAL_INDEX)],
            },
        ],
        depends_on: &[STRING_TABLE],
    },
];

pub fn section(section: u16) -> Option<&'static SectionDef> {
    SECTIONS.iter().find(|def| def.section == section)
}

pub fn record(section_type: u16, kind: u16) -> Option<&'static RecordDef> {
    section(section_type)?
        .records
        .iter()
        .find(|def| def.kind == kind)
}

impl FieldDef {
    /// G147: the definition-text suffix of a field's shape -- empty for a scalar field, so every
    /// definition written before G147 keeps its text and its schema identity.
    pub fn shape(&self) -> String {
        let mut shape = String::new();
        if self.nested != 0 {
            shape.push_str(&format!(" record={}", self.nested));
        }
        if self.repeated {
            shape.push_str(" repeated");
        }
        if self.element != 0 {
            shape.push_str(&format!(" element={}", self.element));
        }
        shape
    }
}

impl RecordDef {
    /// The declared field `name`. A name the table lacks is a programming error in this codec,
    /// never input-dependent, so it panics (every name is exercised by the round-trip tests).
    pub fn field(&self, name: &str) -> FieldDef {
        self.fields
            .iter()
            .copied()
            .find(|field| field.name == name)
            .unwrap_or_else(|| panic!("record `{}` declares no field `{name}`", self.name))
    }
}

/// The canonical text a section's schema hash covers: its name, every record kind with every
/// field (tag, name, wire type, requiredness), and the hash of every section it depends on.
pub fn definition_text(def: &SectionDef) -> String {
    let mut text = format!("atlas.wire.v1/{}\n", def.name);
    for record in def.records {
        text.push_str(&format!("record {} {}\n", record.kind, record.name));
        for field in record.fields {
            text.push_str(&format!(
                "field {} {} {} {}{}\n",
                field.tag,
                field.name,
                field.wire,
                if field.required {
                    "required"
                } else {
                    "optional"
                },
                field.shape()
            ));
        }
    }
    for dependency in def.depends_on {
        let dependency = section(*dependency).expect("declared dependency");
        let hash = schema_hash(dependency);
        text.push_str(&format!("depends {} ", dependency.name));
        for byte in hash {
            text.push_str(&format!("{byte:02x}"));
        }
        text.push('\n');
    }
    text
}

pub fn schema_hash(def: &SectionDef) -> [u8; 32] {
    blake3::hash(definition_text(def).as_bytes())
}

/// Every definition this module has ever written, oldest first (G86): `generation <id>` lines,
/// each followed by the `definition_text` of every section of that generation.
pub const HISTORY: &str = super::schema_history::HISTORY;

/// A recorded field: `(tag, name, wire, required, shape)` -- `shape` is `FieldDef::shape`.
pub type RecordedField = (u16, String, u8, bool, String);

/// A recorded record kind: `(kind, name, fields)`.
pub type RecordedRecord = (u16, String, Vec<RecordedField>);

/// One section definition read back from its recorded `definition_text`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedSection {
    pub name: String,
    pub records: Vec<RecordedRecord>,
    pub depends_on: Vec<String>,
    /// The recorded definition text, whose hash is the section's schema identity.
    pub text: String,
}

/// The recorded generations: `(generation id, sections)`. Malformed history is a programming
/// error (the file is compiled in and checked by tests), so it panics.
pub fn recorded_generations(history: &str) -> Vec<(String, Vec<RecordedSection>)> {
    let mut generations: Vec<(String, Vec<RecordedSection>)> = Vec::new();
    for line in history.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let mut words = line.split(' ');
        match words.next() {
            Some("generation") => {
                let id = words.next().expect("generation id");
                generations.push((id.to_owned(), Vec::new()));
                continue;
            }
            Some(first) if first.starts_with("atlas.wire.v1/") => {
                let sections = &mut generations.last_mut().expect("generation line").1;
                sections.push(RecordedSection {
                    name: first.trim_start_matches("atlas.wire.v1/").to_owned(),
                    records: Vec::new(),
                    depends_on: Vec::new(),
                    text: String::new(),
                });
            }
            Some("record") => {
                let section = current(&mut generations);
                let kind = words
                    .next()
                    .and_then(|k| k.parse().ok())
                    .expect("record kind");
                let name = words.next().expect("record name").to_owned();
                section.records.push((kind, name, Vec::new()));
            }
            Some("field") => {
                let section = current(&mut generations);
                let tag = words
                    .next()
                    .and_then(|t| t.parse().ok())
                    .expect("field tag");
                let name = words.next().expect("field name").to_owned();
                let wire = words
                    .next()
                    .and_then(|w| w.parse().ok())
                    .expect("field wire");
                let required = words.next() == Some("required");
                let shape: String = words.map(|word| format!(" {word}")).collect();
                let record = section.records.last_mut().expect("field inside a record");
                record.2.push((tag, name, wire, required, shape));
            }
            Some("depends") => {
                let section = current(&mut generations);
                section
                    .depends_on
                    .push(words.next().expect("dependency name").to_owned());
            }
            other => panic!("malformed schema history line {other:?}: {line}"),
        }
        let section = current(&mut generations);
        section.text.push_str(line);
        section.text.push('\n');
    }
    generations
}

fn current(generations: &mut [(String, Vec<RecordedSection>)]) -> &mut RecordedSection {
    generations
        .last_mut()
        .and_then(|(_, sections)| sections.last_mut())
        .expect("a line inside a section")
}

impl RecordedSection {
    pub fn schema_id(&self) -> u64 {
        let digest = blake3::hash(self.text.as_bytes());
        u64::from_le_bytes(digest[..8].try_into().expect("8 bytes"))
    }
}

/// Whether content written under the recorded definition `old` reads correctly under `new`
/// (absorbed from FlatBuffers' `flatc --conform`, first-50 #22): the section keeps its name and
/// dependencies; no record kind disappears; every recorded field keeps its tag and wire type and
/// is never removed (deprecate, never delete) and never becomes required; a field `new` adds is
/// optional. A renamed field with the same tag and wire type conforms: the wire is tag-addressed.
pub fn conforms(old: &RecordedSection, new: &SectionDef) -> Result<(), String> {
    if old.name != new.name {
        return Err(format!("section renamed: {} -> {}", old.name, new.name));
    }
    for dependency in &old.depends_on {
        let kept = new
            .depends_on
            .iter()
            .any(|d| section(*d).is_some_and(|d| d.name == *dependency));
        if !kept {
            return Err(format!("{}: dependency {dependency} dropped", old.name));
        }
    }
    for (kind, record_name, fields) in &old.records {
        let Some(record) = new.records.iter().find(|r| r.kind == *kind) else {
            return Err(format!(
                "{}: record kind {kind} ({record_name}) removed",
                old.name
            ));
        };
        for (tag, field_name, wire, required, shape) in fields {
            let Some(field) = record.fields.iter().find(|f| f.tag == *tag) else {
                return Err(format!(
                    "{}/{}: field {tag} ({field_name}) removed",
                    old.name, record.name
                ));
            };
            if field.wire != *wire {
                return Err(format!(
                    "{}/{}: field {tag} changed wire type {wire} -> {}",
                    old.name, record.name, field.wire
                ));
            }
            if field.required && !required {
                return Err(format!(
                    "{}/{}: field {tag} became required",
                    old.name, record.name
                ));
            }
            if field.shape() != *shape {
                return Err(format!(
                    "{}/{}: field {tag} changed shape `{}` -> `{}`",
                    old.name,
                    record.name,
                    shape.trim(),
                    field.shape().trim()
                ));
            }
        }
        for field in record.fields {
            let recorded = fields.iter().any(|(tag, ..)| *tag == field.tag);
            if !recorded && field.required {
                return Err(format!(
                    "{}/{}: new field {} ({}) is required",
                    old.name, record.name, field.tag, field.name
                ));
            }
        }
    }
    Ok(())
}

/// The schema identities a reader accepts for `section_type`: the current definition's, and
/// every recorded definition of that section which conforms to it (G86). Content written under a
/// conforming definition decodes with the current tables; anything else is refused.
pub fn accepted_schema_ids(history: &str, section_type: u16) -> Vec<u64> {
    let Some(current) = section(section_type) else {
        return Vec::new();
    };
    let current_id = {
        let digest = schema_hash(current);
        u64::from_le_bytes(digest[..8].try_into().expect("8 bytes"))
    };
    let mut ids = vec![current_id];
    for (_, sections) in recorded_generations(history) {
        for recorded in sections.iter().filter(|s| s.name == current.name) {
            let id = recorded.schema_id();
            if !ids.contains(&id) && conforms(recorded, current).is_ok() {
                ids.push(id);
            }
        }
    }
    ids
}
