//! The ROOT_MANIFEST object (G187, M13) falsified: its schema table pinned to its version; the
//! root identity independent of the field that carries it and of entry order; every flipped
//! byte, an undeclared field, a second record, an out-of-order entry and a malformed value
//! refused with the codec's typed defects; arbitrary bytes refused, never a panic.

use super::*;
use crate::atlas::u32_at;
use crate::atlasx::codec::{HEADER_LEN, RECORD_HEADER_LEN, decode};

type D = CodecDefect;

fn digest(byte: u8) -> IntegrityDigest {
    IntegrityDigest::blake3_256(&[byte; 32])
}

fn entry(byte: u8, class: u16) -> ObjectEntry {
    let hash = digest(byte);
    ObjectEntry {
        relative_path: format!(
            "functions/{}.atlasx",
            &hash.as_str()[IntegrityDigest::BLAKE3_256_PREFIX.len()..]
        ),
        object_class: class,
        object_schema_version: 1,
        decoded_content_hash: hash,
        decoded_length: 385,
        required: true,
        logical_record_count: 1,
    }
}

fn manifest() -> AtlasxManifest {
    let mut m = AtlasxManifest {
        root_id: digest(0),
        parent_root: digest(1),
        genome_hash: digest(2),
        design_id: "design:fixture".into(),
        scope_id: "self/FUNCTIONS/design-function-roots".into(),
        target_kind: "NONE".into(),
        materializer: "atlas_core::atlasx::materialize".into(),
        materializer_version: "1".into(),
        materialization_schema: "atlas.atlasx-materialization.v1".into(),
        compiler_ir_contract: "UNKNOWN".into(),
        objects: vec![entry(3, 4), entry(4, 4)],
        census_digest: digest(5),
        revision: "git:fixture".into(),
        seal_id: "seal:fixture".into(),
    };
    m.root_id = root_identity(&m).unwrap();
    m
}

fn kinds(result: Result<(ObjectHeader, AtlasxManifest), Defects>) -> Vec<CodecDefect> {
    result.unwrap_err().into_iter().map(|(d, _)| d).collect()
}

/// A ROOT_MANIFEST object around `payload`, with a correct header and hash.
fn forge(payload: &[u8]) -> Vec<u8> {
    codec::object(
        ObjectHeader::of(CLASS_ROOT_MANIFEST, MANIFEST_SCHEMA_VERSION, payload),
        payload,
    )
}

/// Framed records or fields, split: (kind or tag, the whole framed item).
type Framed = Vec<(u16, Vec<u8>)>;

/// The framed records of `content`, split: (kind, the whole framed record).
fn split_records(content: &[u8]) -> Framed {
    let mut out = Vec::new();
    let mut at = 0;
    while at < content.len() {
        let len = crate::atlas::u64_at(content, at + 8) as usize;
        let end = at + RECORD_HEADER_LEN + len;
        out.push((crate::atlas::u16_at(content, at), content[at..end].to_vec()));
        at = end;
    }
    out
}

/// The framed fields of one record's payload: (tag, the whole framed field).
fn split_fields(payload: &[u8]) -> Vec<(u16, Vec<u8>)> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < payload.len() {
        let end = at + codec::FIELD_HEADER_LEN + u32_at(payload, at + 4) as usize;
        out.push((crate::atlas::u16_at(payload, at), payload[at..end].to_vec()));
        at = end;
    }
    out
}

/// `record`'s header over a new payload of `fields`.
fn reframe(record: &[u8], fields: &[(u16, Vec<u8>)]) -> Vec<u8> {
    let payload: Vec<u8> = fields.iter().flat_map(|(_, f)| f.clone()).collect();
    let mut out = record[..8].to_vec();
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(&payload);
    out
}

/// The manifest `bytes` with its one record's fields edited, re-framed and re-hashed.
fn edit_fields(bytes: &[u8], edit: impl FnOnce(&mut Vec<(u16, Vec<u8>)>)) -> Vec<u8> {
    let records = split_records(&bytes[HEADER_LEN..]);
    let record = &records[0].1;
    let mut fields = split_fields(&record[RECORD_HEADER_LEN..]);
    edit(&mut fields);
    forge(&reframe(record, &fields))
}

fn raw_field(tag: u16, wire: u8, flags: u8, bytes: &[u8]) -> Vec<u8> {
    let mut out = tag.to_le_bytes().to_vec();
    out.push(wire);
    out.push(flags);
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
    out
}

#[test]
fn the_schema_table_is_pinned_to_its_version() {
    assert_eq!(
        (MANIFEST_SCHEMA_VERSION, schema_digest().as_str()),
        (
            1,
            "blake3-256:2a37c35fde0e9dfa05c6eb3428273a34a83ed90bc91fc5ed1d5f4fa1820c3eb4"
        ),
        "a changed ROOT_MANIFEST table needs a new schema version"
    );
    // The contract's tags 1 to 10 and 14 keep their meaning; 11-13, 15 and 16 are undeclared.
    let tags: Vec<u16> = MANIFEST_SCHEMA[0].fields.iter().map(|f| f.tag).collect();
    assert_eq!(tags, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 14, 17, 18, 19]);
    let entry: Vec<u16> = MANIFEST_SCHEMA[1].fields.iter().map(|f| f.tag).collect();
    assert_eq!(entry, [1, 2, 3, 4, 5, 6, 7]);
    assert!(
        MANIFEST_SCHEMA
            .iter()
            .flat_map(|d| d.fields)
            .all(|f| f.required)
    );
}

#[test]
fn the_manifest_round_trips_and_its_root_identity_excludes_tag_1() {
    let m = manifest();
    let bytes = encode_manifest(&m).unwrap();
    let (header, read) = read_manifest(&bytes).unwrap();
    assert_eq!(read, m);
    assert_eq!(header.object_class, CLASS_ROOT_MANIFEST);
    assert_eq!(header.object_schema_version, MANIFEST_SCHEMA_VERSION);
    // Entries are in the contract's order: class, then decoded hash, then path.
    assert_eq!(read.objects, [entry(3, 4), entry(4, 4)]);
    let mut classes = m.clone();
    classes.objects[1].object_class = 3;
    let (_, read) = read_manifest(&encode_manifest(&classes).unwrap()).unwrap();
    assert_eq!(read.objects[0].object_class, 3);
    // The root id is BLAKE3 of the record payload without field 1, and field 1 carries it.
    let record = &split_records(&bytes[HEADER_LEN..])[0].1;
    let fields = split_fields(&record[RECORD_HEADER_LEN..]);
    assert_eq!(fields[0].0, 1);
    assert_eq!(
        &fields[0].1[codec::FIELD_HEADER_LEN..],
        digest_bytes(&m.root_id)
    );
    let rest: Vec<u8> = fields[1..].iter().flat_map(|(_, f)| f.clone()).collect();
    assert_eq!(m.root_id, IntegrityDigest::of_bytes(&rest));
    // Whatever root id is carried, the identity is the same; any other field moves it.
    let mut other = m.clone();
    other.root_id = digest(9);
    assert_eq!(root_identity(&other).unwrap(), m.root_id);
    for edit in [
        (|m: &mut AtlasxManifest| m.seal_id.push('x')) as fn(&mut AtlasxManifest),
        |m| m.objects[0].decoded_length += 1,
        |m| m.objects[1].required = false,
        |m| m.genome_hash = digest(7),
        |m| m.revision.push('x'),
    ] {
        let mut changed = m.clone();
        edit(&mut changed);
        assert_ne!(root_identity(&changed).unwrap(), m.root_id);
    }
    // The order objects are given in never moves it.
    let mut reversed = m.clone();
    reversed.objects.reverse();
    assert_eq!(root_identity(&reversed).unwrap(), m.root_id);
    assert_eq!(encode_manifest(&reversed).unwrap(), bytes);
    // A manifest is not a FUNCTIONS object, and a FUNCTIONS object is not a manifest.
    assert_eq!(decode(&bytes).defects[0].0, D::ClassMismatch);
    let functions = include_bytes!("../codec/fixture.bin");
    assert!(codec::read(functions).is_ok());
    assert_eq!(kinds(read_manifest(functions)), [D::ClassMismatch]);
}

#[test]
fn every_flipped_byte_is_refused() {
    let bytes = encode_manifest(&manifest()).unwrap();
    for at in 0..bytes.len() {
        let mut flipped = bytes.clone();
        flipped[at] ^= 0x01;
        let found = kinds(read_manifest(&flipped));
        assert!(!found.is_empty(), "byte {at}");
        if at >= HEADER_LEN {
            assert_eq!(found, [D::DigestMismatch], "byte {at}");
        }
    }
    for len in 0..bytes.len() {
        assert!(read_manifest(&bytes[..len]).is_err(), "{len}");
    }
}

#[test]
fn the_writer_refuses_what_it_cannot_write_canonically() {
    let refused = |edit: fn(&mut AtlasxManifest)| {
        let mut m = manifest();
        edit(&mut m);
        encode_manifest(&m)
            .unwrap_err()
            .into_iter()
            .map(|(d, _)| d)
            .collect::<Vec<_>>()
    };
    assert_eq!(refused(|m| m.objects.clear()), [D::MissingRequiredField]);
    assert_eq!(
        refused(|m| m.objects[1] = m.objects[0].clone()),
        [D::DuplicateIdentity]
    );
    assert_eq!(
        refused(|m| {
            m.objects[1].relative_path = m.objects[0].relative_path.clone();
        }),
        [D::DuplicateIdentity]
    );
    assert_eq!(
        refused(|m| m.objects[0].relative_path.clear()),
        [D::MalformedValue]
    );
    assert_eq!(refused(|m| m.seal_id.clear()), [D::MalformedValue]);
    assert_eq!(refused(|m| m.design_id.clear()), [D::MalformedValue]);
    assert_eq!(
        refused(|m| m.compiler_ir_contract.clear()),
        [D::MalformedValue]
    );
}

#[test]
fn the_reader_refuses_undeclared_fields_records_and_malformed_values() {
    let bytes = encode_manifest(&manifest()).unwrap();
    // A profile reference (tag 11): declared by the contract, not by this schema.
    let profile = edit_fields(&bytes, |fields| {
        let at = fields.iter().position(|(t, _)| *t == 14).unwrap();
        fields.insert(at, (11, raw_field(11, wire::RECORD, 0, &[])));
    });
    assert_eq!(kinds(read_manifest(&profile)), [D::UndeclaredOptionalField]);
    let barrier = edit_fields(&bytes, |fields| {
        let at = fields.iter().position(|(t, _)| *t == 14).unwrap();
        fields.insert(at, (13, raw_field(13, wire::UTF8, 1, b"barrier")));
    });
    assert_eq!(kinds(read_manifest(&barrier)), [D::UndeclaredRequiredField]);
    // A required field dropped.
    let no_seal = edit_fields(&bytes, |fields| fields.retain(|(t, _)| *t != 19));
    assert_eq!(kinds(read_manifest(&no_seal)), [D::MissingRequiredField]);
    let no_root = edit_fields(&bytes, |fields| fields.retain(|(t, _)| *t != 1));
    assert_eq!(kinds(read_manifest(&no_root)), [D::MissingRequiredField]);
    // A hash that is not 32 bytes.
    let short = edit_fields(&bytes, |fields| {
        fields[0].1 = raw_field(1, wire::HASH32, 1, &[0; 31]);
    });
    assert_eq!(kinds(read_manifest(&short)), [D::MalformedValue]);
    // Two records, none, and a record of an undeclared or embedded-only kind.
    let payload = &bytes[HEADER_LEN..];
    assert_eq!(
        kinds(read_manifest(&forge(&[payload, payload].concat()))),
        [D::DuplicateIdentity]
    );
    assert_eq!(kinds(read_manifest(&forge(&[]))), [D::EmptyObject]);
    for kind in [OBJECT_ENTRY, 3, 0] {
        let mut other = payload.to_vec();
        other[..2].copy_from_slice(&kind.to_le_bytes());
        assert_eq!(
            kinds(read_manifest(&forge(&other))),
            [D::UndeclaredRecordKind],
            "{kind}"
        );
    }
    // Object entries out of the contract's order, and malformed entry values.
    let entries = |edit: &dyn Fn(&mut Framed)| {
        edit_fields(&bytes, |fields| {
            let at = fields.iter().position(|(t, _)| *t == 14).unwrap();
            let field = &fields[at].1;
            let mut records = split_records(&field[codec::FIELD_HEADER_LEN..]);
            edit(&mut records);
            let content: Vec<u8> = records.iter().flat_map(|(_, r)| r.clone()).collect();
            fields[at].1 = raw_field(14, wire::RECORD, 1, &content);
        })
    };
    assert_eq!(
        kinds(read_manifest(&entries(&|records| records.reverse()))),
        [D::NonCanonical]
    );
    let entry_field = |tag: u16, value: Vec<u8>| {
        move |records: &mut Vec<(u16, Vec<u8>)>| {
            let record = records[0].1.clone();
            let mut fields = split_fields(&record[RECORD_HEADER_LEN..]);
            let at = fields.iter().position(|(t, _)| *t == tag).unwrap();
            let wire = fields[at].1[2];
            fields[at].1 = raw_field(tag, wire, 1, &value);
            records[0].1 = reframe(&record, &fields);
        }
    };
    // BOOL is one byte, 0 or 1; a class or schema must fit a u16; a varint must be minimal.
    for (tag, value) in [
        (6, vec![2]),
        (6, vec![]),
        (6, vec![1, 0]),
        (2, vec![0x80, 0x80, 0x04]),
        (3, vec![0x80, 0x80, 0x04]),
        (5, vec![0x80, 0x00]),
    ] {
        assert_eq!(
            kinds(read_manifest(&entries(&entry_field(tag, value.clone())))),
            [D::MalformedValue],
            "{tag} {value:?}"
        );
    }
    // A value in range reads back as written: an entry not required.
    let (_, optional) = read_manifest(&entries(&entry_field(6, vec![0]))).unwrap();
    assert!(!optional.objects[0].required);
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
fn arbitrary_input_is_refused_never_a_panic() {
    let mut rng = Rng(0x243f_6a88_85a3_08d3);
    let payload = encode_manifest(&manifest()).unwrap()[HEADER_LEN..].to_vec();
    for round in 0..4000 {
        let len = rng.below(300);
        let noise: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
        assert!(read_manifest(&noise).is_err());
        if let Ok((_, m)) = read_manifest(&forge(&noise)) {
            assert_eq!(encode_manifest(&m).unwrap(), forge(&noise), "{round}");
        }
        let mut edited = payload.clone();
        for _ in 0..1 + rng.below(4) {
            let at = rng.below(edited.len());
            match rng.below(3) {
                0 => edited[at] = rng.next() as u8,
                1 => {
                    edited.remove(at);
                }
                _ => edited.insert(at, rng.next() as u8),
            }
        }
        let bytes = forge(&edited);
        match read_manifest(&bytes) {
            Ok((_, m)) => assert_eq!(encode_manifest(&m).unwrap(), bytes, "{round}"),
            Err(defects) => assert!(!defects.is_empty()),
        }
    }
}
