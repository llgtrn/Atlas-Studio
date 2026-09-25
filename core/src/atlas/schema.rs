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
    CENSUS_CERTIFICATE, GRAPH_EDGES, GRAPH_NODES, OBLIGATIONS, ROOT_MANIFEST, SEMANTIC_RECORDS,
    STRING_TABLE, WIRE_HASH32, WIRE_LOCAL_INDEX, WIRE_UTF8, WIRE_UVARINT,
};
use crate::identity::blake3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldDef {
    pub name: &'static str,
    pub tag: u16,
    pub wire: u8,
    pub required: bool,
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
    }
}

const fn opt(name: &'static str, tag: u16, wire: u8) -> FieldDef {
    FieldDef {
        name,
        tag,
        wire,
        required: false,
    }
}

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
        records: &[RecordDef {
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
        }],
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
                "field {} {} {} {}\n",
                field.tag,
                field.name,
                field.wire,
                if field.required {
                    "required"
                } else {
                    "optional"
                }
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
