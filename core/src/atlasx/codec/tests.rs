//! M12 falsified on a real construction-IR input: the SR1-3 module (G181,
//! `.atlas/evidence/self-reconstruction/SR1/attempt3/module.json`, copied as
//! `fixture_module.json`) encodes to the committed `fixture.bin` byte for byte and reads back to
//! the same records at the committed address; every flipped byte, every truncation, every
//! undeclared field and every non-canonical form is refused with its typed defect; arbitrary
//! input is refused, never a panic.

use super::*;
use crate::construction::ConstructionModule;

use CodecDefect as D;

const FIXTURE: &[u8] = include_bytes!("fixture.bin");
/// The fixture object's address: the lowercase hex of its decoded content hash.
const FIXTURE_ADDRESS: &str = "5b8223605edbd24c18a1033e9007ffef0c7d7d8385a56a4ce6ca3cf92a1241ed";

fn module() -> ConstructionModule {
    serde_json::from_str(include_str!("fixture_module.json")).expect("the fixture module parses")
}

fn fixture_record() -> FunctionSignatureRecord {
    FunctionSignatureRecord::of(&module().functions[0])
}

/// The defect kinds of a refusal, in order.
fn kinds(defects: &Defects) -> Vec<CodecDefect> {
    defects.iter().map(|(k, _)| *k).collect()
}

fn refused(bytes: &[u8]) -> Vec<CodecDefect> {
    let report = decode(bytes);
    assert_eq!(report.verdict, CodecVerdict::Refused, "{report:#?}");
    assert!(report.address.is_none() && report.records.is_none());
    assert!(!report.defects.is_empty());
    kinds(&report.defects)
}

/// A record of `kind` framed with explicit version, flags and payload.
fn raw_record(kind: u16, version: u16, flags: u32, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(&version.to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

fn record(kind: u16, payload: &[u8]) -> Vec<u8> {
    raw_record(kind, RECORD_SCHEMA_VERSION, 0, payload)
}

fn raw_field(tag: u16, wire: u8, flags: u8, bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&tag.to_le_bytes());
    out.push(wire);
    out.push(flags);
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
    out
}

/// A field as the table declares it.
fn field(record: u16, name: &str, bytes: &[u8]) -> Vec<u8> {
    let def = record_def(record).unwrap().field(name);
    raw_field(def.tag, def.wire, u8::from(def.required), bytes)
}

/// An object around `payload` whose header is valid and whose hash authenticates it, so the
/// reader reaches framing and schema checks.
fn forge(payload: &[u8]) -> Vec<u8> {
    let mut out = ObjectHeader::functions(payload).to_bytes().to_vec();
    out.extend_from_slice(payload);
    out
}

fn with_header(bytes: &[u8], edit: impl FnOnce(&mut ObjectHeader)) -> Vec<u8> {
    let mut header = ObjectHeader::parse(bytes).unwrap();
    edit(&mut header);
    let mut out = header.to_bytes().to_vec();
    out.extend_from_slice(&bytes[HEADER_LEN..]);
    out
}

/// The fields of a minimal valid signature `id`, by name, so a test can drop or replace one.
fn signature_fields(id: &str) -> Vec<(&'static str, Vec<u8>)> {
    let lineage = record(LINEAGE_REF, &field(LINEAGE_REF, "record", b"semantic:X:1"));
    vec![
        (
            "function_id",
            field(FUNCTION_SIGNATURE, "function_id", id.as_bytes()),
        ),
        ("name", field(FUNCTION_SIGNATURE, "name", b"f")),
        ("dispatch", field(FUNCTION_SIGNATURE, "dispatch", &[1])),
        (
            "visibility",
            field(FUNCTION_SIGNATURE, "visibility", b"pub"),
        ),
        ("lineage", field(FUNCTION_SIGNATURE, "lineage", &lineage)),
    ]
}

fn signature(fields: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let payload: Vec<u8> = fields.iter().flat_map(|(_, b)| b.clone()).collect();
    record(FUNCTION_SIGNATURE, &payload)
}

/// `signature_fields(id)` with `name` replaced by `bytes` (or dropped when `None`).
fn signature_with(id: &str, name: &str, bytes: Option<Vec<u8>>) -> Vec<u8> {
    let mut fields = signature_fields(id);
    let at = fields.iter().position(|(n, _)| *n == name).unwrap();
    match bytes {
        Some(bytes) => fields[at].1 = bytes,
        None => {
            fields.remove(at);
        }
    }
    signature(&fields)
}

fn several() -> Vec<FunctionSignatureRecord> {
    let base = fixture_record();
    let mut free = base.clone();
    free.function_id = "fn:core/src/a.rs::helper".into();
    free.name = "helper".into();
    free.owner = None;
    free.dispatch = Dispatch::FreeFunction;
    free.params = vec![
        IrParam {
            name: "b".into(),
            type_spelling: "u8".into(),
        },
        IrParam {
            name: "a".into(),
            type_spelling: "&str".into(),
        },
    ];
    free.result = None;
    free.body_fingerprint = None;
    free.documentation = Some("Helps.".into());
    free.lineage = vec![
        "semantic:FUNCTION_SIGNATURE:2".into(),
        "semantic:FUNCTION_IDENTITY:1".into(),
    ];
    let mut assoc = base.clone();
    assoc.function_id = "fn:core/src/donor/mod.rs::MaterializationMode::new".into();
    assoc.name = "new".into();
    assoc.dispatch = Dispatch::AssociatedFunction;
    assoc.params = Vec::new();
    assoc.result = Some("Self".into());
    assoc.documentation = Some(String::new());
    let mut unicode = base.clone();
    unicode.function_id = "fn:z/é.rs::ünïcode".into();
    unicode.name = "ünïcode".into();
    vec![base, free, assoc, unicode]
}

// ---- the fixture --------------------------------------------------------------------------------

#[test]
fn the_fixture_module_encodes_to_the_committed_bytes_and_reads_back() {
    let module = module();
    let encoded = encode_module(&module).expect("the SR1-3 module encodes");
    assert_eq!(
        encoded.bytes, FIXTURE,
        "the committed fixture is the canonical encoding"
    );
    assert_eq!(encoded.address, FIXTURE_ADDRESS);
    assert_eq!(
        encoded.digest.as_str(),
        format!("blake3-256:{FIXTURE_ADDRESS}")
    );
    assert_eq!(encoded.records, 1);
    assert_eq!(encoded.object_class, CLASS_FUNCTIONS);
    assert_eq!(encoded.decoded_length as usize, FIXTURE.len() - HEADER_LEN);
    assert_eq!(encoded.module_id, module.module_id);
    // `is_local` carries a HIR body, which this record kind does not: it is reported, not dropped
    // silently.
    assert_eq!(
        encoded.bodies_not_encoded,
        vec![module.functions[0].id.clone()]
    );
    assert_eq!(encoded.not_verified, not_verified());

    let report = decode(FIXTURE);
    assert_eq!(report.verdict, CodecVerdict::Decoded, "{report:#?}");
    assert!(report.defects.is_empty());
    assert_eq!(report.address.as_deref(), Some(FIXTURE_ADDRESS));
    assert_eq!(report.digest, Some(encoded.digest.clone()));
    assert_eq!(report.object_class, Some(CLASS_FUNCTIONS));
    assert_eq!(report.object_schema_version, Some(FUNCTIONS_SCHEMA_VERSION));
    assert_eq!(report.decoded_length, Some(encoded.decoded_length));
    let records = report.records.clone().unwrap();
    assert_eq!(records, vec![fixture_record()]);
    assert_eq!(
        encode_functions(&records).unwrap(),
        FIXTURE,
        "read then written: byte for byte"
    );
    assert_eq!(report.not_verified.len(), CODEC_NOT_VERIFIED.len());
    // The report is JSON-stable: it serializes and parses back to itself.
    let text = serde_json::to_string(&report).unwrap();
    assert_eq!(serde_json::from_str::<CodecReport>(&text).unwrap(), report);
    assert!(text.contains("\"verdict\":\"DECODED\""), "{text}");
}

#[test]
fn the_address_is_the_native_blake3_of_the_payload_and_agrees_with_the_reference_crate() {
    let payload = &FIXTURE[HEADER_LEN..];
    let native = blake3::hash(payload);
    let reference: [u8; 32] = *::blake3::hash(payload).as_bytes();
    assert_eq!(
        native, reference,
        "native BLAKE3 against the reference crate"
    );
    assert_eq!(object_address(&native), FIXTURE_ADDRESS);
    assert_eq!(
        ObjectHeader::parse(FIXTURE).unwrap().decoded_content_hash,
        native
    );
    assert_eq!(object_address(&[0xab; 32]), "ab".repeat(32));
    // A decoded object named by its address is admitted; any other name is refused.
    let named = decode_named(&format!("{FIXTURE_ADDRESS}.atlasx"), FIXTURE);
    assert_eq!(named.verdict, CodecVerdict::Decoded);
    for name in [
        format!("{}.atlasx", FIXTURE_ADDRESS.to_uppercase()),
        FIXTURE_ADDRESS.to_owned(),
        "manifest.atlasx".to_owned(),
        format!("{}.atlasx", "0".repeat(64)),
    ] {
        let report = decode_named(&name, FIXTURE);
        assert_eq!(kinds(&report.defects), vec![D::AddressMismatch], "{name}");
        assert_eq!(report.verdict, CodecVerdict::Refused);
        assert!(report.records.is_none() && report.address.is_none() && report.digest.is_none());
    }
}

#[test]
fn the_header_is_the_contract_layout() {
    let h = &FIXTURE[..HEADER_LEN];
    assert_eq!(&h[..8], b"ATLSX\0\x01\0");
    assert_eq!(u16_at(h, 8), 80);
    assert_eq!(u16_at(h, 10), 1, "major");
    assert_eq!(u16_at(h, 12), 0, "minor");
    assert_eq!(u16_at(h, 14), 0, "flags");
    assert_eq!(u16_at(h, 16), 4, "FUNCTIONS");
    assert_eq!(u16_at(h, 18), 1, "object schema");
    assert_eq!(u16_at(h, 20), 1, "BLAKE3_256");
    assert_eq!(u16_at(h, 22), 0, "NONE");
    assert_eq!(u64_at(h, 24), (FIXTURE.len() - HEADER_LEN) as u64);
    assert_eq!(u64_at(h, 32), (FIXTURE.len() - HEADER_LEN) as u64);
    assert_eq!(u64_at(h, 72), 0, "reserved");
    // The first record: FUNCTION_SIGNATURE, record schema 1, no flags, its length.
    let p = &FIXTURE[HEADER_LEN..];
    assert_eq!(u16_at(p, 0), FUNCTION_SIGNATURE);
    assert_eq!(u16_at(p, 2), 1);
    assert_eq!(u32_at(p, 4), 0);
    assert_eq!(u64_at(p, 8) as usize, p.len() - RECORD_HEADER_LEN);
    // Its first field: function_id, GLOBAL_ID, required.
    assert_eq!(u16_at(p, 16), 1);
    assert_eq!(p[18], wire::GLOBAL_ID);
    assert_eq!(p[19], FIELD_REQUIRED);
    let header = ObjectHeader::parse(FIXTURE).unwrap();
    assert_eq!(
        header.to_bytes().as_slice(),
        h,
        "parse and to_bytes are inverse"
    );
}

#[test]
fn the_schema_table_is_pinned_to_its_version() {
    // A change to the table (a tag, name, wire type, required flag or nested kind) moves this
    // digest: bump FUNCTIONS_SCHEMA_VERSION with it, never repurpose a tag.
    assert_eq!(
        (FUNCTIONS_SCHEMA_VERSION, schema_digest().as_str()),
        (
            1,
            "blake3-256:d20d9e38072de7ff7731f328e12189b20b75d7236c8378f8c95181e377ebb047"
        )
    );
    for def in FUNCTIONS_SCHEMA {
        let tags: Vec<u16> = def.fields.iter().map(|f| f.tag).collect();
        assert!(
            tags.windows(2).all(|p| p[0] < p[1]),
            "{}: tags ascend",
            def.name
        );
        for f in def.fields {
            assert!((wire::UVARINT..=wire::BOOL).contains(&f.wire), "{}", f.name);
            assert_eq!(f.wire == wire::RECORD, f.nested != 0, "{}", f.name);
            if f.nested != 0 {
                assert!(!record_def(f.nested).unwrap().top_level);
            }
        }
    }
}

// ---- canonical writing --------------------------------------------------------------------------

#[test]
fn encoding_is_deterministic_and_independent_of_input_order() {
    let records = several();
    let canonical = encode_functions(&records).unwrap();
    assert_eq!(encode_functions(&records).unwrap(), canonical);
    // Every permutation of the four records gives the same bytes.
    let mut count = 0;
    let mut order = [0usize, 1, 2, 3];
    permute(&mut order, 0, &mut |order| {
        let permuted: Vec<_> = order.iter().map(|i| records[*i].clone()).collect();
        assert_eq!(encode_functions(&permuted).unwrap(), canonical, "{order:?}");
        count += 1;
    });
    assert_eq!(count, 24);
    // Lineage is a set: reversed and duplicated, the bytes do not move.
    let mut shuffled = records.clone();
    for r in &mut shuffled {
        r.lineage.reverse();
        r.lineage.push(r.lineage[0].clone());
    }
    assert_eq!(encode_functions(&shuffled).unwrap(), canonical);
    // Parameters keep their order: swapping two is another object.
    let mut swapped = records.clone();
    swapped[1].params.reverse();
    assert_ne!(encode_functions(&swapped).unwrap(), canonical);
    // It reads back to the canonical records, sorted by identity.
    let (_, decoded) = read(&canonical).unwrap();
    let mut expected = records.clone();
    for r in &mut expected {
        r.canonicalize();
    }
    expected.sort_by(|a, b| a.function_id.cmp(&b.function_id));
    assert_eq!(decoded, expected);
    assert_eq!(decode(&canonical).verdict, CodecVerdict::Decoded);
    // Some("") and None are different objects.
    let mut none = records.clone();
    none[2].documentation = None;
    assert_ne!(encode_functions(&none).unwrap(), canonical);
    assert_eq!(read(&encode_functions(&none).unwrap()).unwrap().1.len(), 4);
}

fn permute(items: &mut [usize; 4], k: usize, visit: &mut impl FnMut(&[usize; 4])) {
    if k == items.len() {
        visit(items);
        return;
    }
    for i in k..items.len() {
        items.swap(k, i);
        permute(items, k + 1, visit);
        items.swap(k, i);
    }
}

#[test]
fn the_writer_refuses_what_it_cannot_write_canonically() {
    let base = fixture_record();
    let err = encode_functions(&[]).unwrap_err();
    assert_eq!(kinds(&err), vec![D::EmptyObject]);
    let err = encode_functions(&[base.clone(), base.clone()]).unwrap_err();
    assert_eq!(kinds(&err), vec![D::DuplicateIdentity]);
    let mut empty_id = base.clone();
    empty_id.function_id.clear();
    assert_eq!(
        kinds(&encode_functions(&[empty_id]).unwrap_err()),
        vec![D::MalformedValue]
    );
    let mut no_lineage = base.clone();
    no_lineage.lineage.clear();
    assert_eq!(
        kinds(&encode_functions(&[no_lineage]).unwrap_err()),
        vec![D::MissingRequiredField]
    );
    let mut empty_lineage = base.clone();
    empty_lineage.lineage.push(String::new());
    assert_eq!(
        kinds(&encode_functions(&[empty_lineage]).unwrap_err()),
        vec![D::MalformedValue]
    );
    for fingerprint in ["37306332", "sha256:00", "blake3-256:ABCDEF", ""] {
        let mut bad = base.clone();
        bad.body_fingerprint = Some(fingerprint.into());
        assert_eq!(
            kinds(&encode_functions(&[bad]).unwrap_err()),
            vec![D::MalformedValue],
            "{fingerprint}"
        );
    }
    // Several defects are all reported, sorted and deduplicated.
    let mut a = base.clone();
    a.lineage.clear();
    let mut b = base.clone();
    b.function_id.clear();
    let err = encode_functions(&[a.clone(), a, b]).unwrap_err();
    assert_eq!(
        kinds(&err),
        vec![D::MissingRequiredField, D::MalformedValue]
    );
    let mut no_name = base.clone();
    no_name.name.clear();
    assert_eq!(
        kinds(&encode_functions(&[no_name]).unwrap_err()),
        vec![D::MalformedValue]
    );
    // Read back, an empty name is refused by the same writer rule.
    let bytes = signature_with("fn:a", "name", Some(field(FUNCTION_SIGNATURE, "name", b"")));
    assert_eq!(refused(&forge(&bytes)), vec![D::MalformedValue]);
}

/// Re-stamps `module`'s id as its content identity.
fn reidentified(mut module: ConstructionModule) -> ConstructionModule {
    module.module_id = crate::construction::module_identity(&module);
    module
}

#[test]
fn the_source_module_must_verify_before_it_is_encoded() {
    let module = module();
    assert!(encode_module(&module).is_ok(), "the SR1-3 module verifies");
    // A forged id.
    let mut forged = module.clone();
    forged.module_id = "construction-module:forged".into();
    assert_eq!(
        kinds(&encode_module(&forged).unwrap_err()),
        vec![D::SourceModuleUnverified]
    );
    // An edit that keeps the old id no longer verifies either.
    let mut edited = module.clone();
    edited.functions[0].visibility = "pub(crate)".into();
    assert_eq!(
        kinds(&encode_module(&edited).unwrap_err()),
        vec![D::SourceModuleUnverified]
    );
    // Re-stamped but invalid: lineage outside the declared inputs; no inputs at all.
    let mut outside = module.clone();
    outside.functions[0].lineage = vec!["semantic:NOT_AN_INPUT:0".into()];
    let err = encode_module(&reidentified(outside)).unwrap_err();
    assert_eq!(kinds(&err), vec![D::SourceModuleInvalid]);
    assert!(err[0].1.starts_with("LINEAGE_OUTSIDE_INPUTS"), "{err:?}");
    let mut bare = module.clone();
    bare.inputs.clear();
    let err = encode_module(&reidentified(bare)).unwrap_err();
    assert!(
        err.iter().all(|(k, _)| *k == D::SourceModuleInvalid),
        "{err:?}"
    );
    assert!(
        err.iter().any(|(_, d)| d.starts_with("NO_DESIGN")),
        "{err:?}"
    );
    // The review's forgery: a forged id, no inputs, lineage outside them and an empty name.
    let mut all = module.clone();
    all.module_id = "construction-module:forged".into();
    all.inputs.clear();
    all.functions[0].lineage = vec!["semantic:NOT_AN_INPUT:0".into()];
    all.functions[0].name.clear();
    let err = encode_module(&all).unwrap_err();
    assert!(kinds(&err).contains(&D::SourceModuleUnverified), "{err:?}");
    assert!(kinds(&err).contains(&D::SourceModuleInvalid), "{err:?}");
    // A module that verifies but names a function "" is refused by the writer.
    let mut unnamed = module.clone();
    unnamed.functions[0].name.clear();
    assert_eq!(
        kinds(&encode_module(&reidentified(unnamed)).unwrap_err()),
        vec![D::MalformedValue]
    );
    // A module without functions verifies, and is refused as an empty object.
    let mut empty = module.clone();
    empty.functions.clear();
    let err = encode_module(&reidentified(empty)).unwrap_err();
    assert_eq!(kinds(&err), vec![D::EmptyObject], "{err:?}");
}

// ---- refusal: bytes, lengths, digest ------------------------------------------------------------

/// The defect a flip of the header byte at `at` must produce, given the flipped value.
fn header_defect(at: usize, flipped: &[u8]) -> CodecDefect {
    let h = ObjectHeader::parse(flipped).unwrap();
    match at {
        0..8 => D::BadMagic,
        8..10 => D::BadHeaderLen,
        10..12 => D::UnsupportedMajor,
        12..14 => D::UnsupportedMinor,
        14..16 => D::UnknownHeaderFlags,
        16..18 => D::ClassMismatch,
        18..20 => D::UnsupportedSchemaVersion,
        20..22 => D::UnsupportedDigestAlgorithm,
        22..24 => D::UnsupportedCodec,
        24..32 if h.encoded_length > MAX_DECODED_LENGTH => D::LengthLimit,
        32..40 if h.decoded_length > MAX_DECODED_LENGTH => D::LengthLimit,
        24..40 => D::LengthMismatch,
        40..72 => D::DigestMismatch,
        72..80 => D::ReservedNonzero,
        _ => unreachable!(),
    }
}

#[test]
fn every_flipped_byte_is_refused_with_its_defect() {
    for at in 0..FIXTURE.len() {
        for mask in [0x01u8, 0x80, 0xff] {
            let mut bytes = FIXTURE.to_vec();
            bytes[at] ^= mask;
            let expected = if at < HEADER_LEN {
                header_defect(at, &bytes)
            } else {
                D::DigestMismatch
            };
            assert_eq!(refused(&bytes), vec![expected], "byte {at} ^ {mask:#04x}");
        }
    }
}

#[test]
fn every_truncation_and_extension_is_refused() {
    for len in 0..FIXTURE.len() {
        assert_eq!(refused(&FIXTURE[..len]), vec![D::Truncated], "{len} bytes");
    }
    for extra in [&[0u8][..], &[0; 16], b"trailing"] {
        let mut bytes = FIXTURE.to_vec();
        bytes.extend_from_slice(extra);
        assert_eq!(refused(&bytes), vec![D::LengthMismatch]);
    }
}

#[test]
fn header_fields_are_refused_each_with_its_defect() {
    let cases: Vec<(Vec<u8>, CodecDefect)> = vec![
        (
            with_header(FIXTURE, |h| h.format_major = 2),
            D::UnsupportedMajor,
        ),
        (
            with_header(FIXTURE, |h| h.format_major = 0),
            D::UnsupportedMajor,
        ),
        (
            with_header(FIXTURE, |h| h.format_minor = 1),
            D::UnsupportedMinor,
        ),
        (with_header(FIXTURE, |h| h.flags = 1), D::UnknownHeaderFlags),
        (
            with_header(FIXTURE, |h| h.object_class = 3),
            D::ClassMismatch,
        ),
        (
            with_header(FIXTURE, |h| h.object_class = 1),
            D::ClassMismatch,
        ),
        (
            with_header(FIXTURE, |h| h.object_class = 1024),
            D::ClassMismatch,
        ),
        (
            with_header(FIXTURE, |h| h.object_schema_version = 2),
            D::UnsupportedSchemaVersion,
        ),
        (
            with_header(FIXTURE, |h| h.digest_algorithm = DIGEST_SHA256),
            D::UnsupportedDigestAlgorithm,
        ),
        (
            with_header(FIXTURE, |h| h.digest_algorithm = 0),
            D::UnsupportedDigestAlgorithm,
        ),
        (
            with_header(FIXTURE, |h| h.codec = CODEC_ZSTD),
            D::UnsupportedCodec,
        ),
        (with_header(FIXTURE, |h| h.codec = 7), D::UnsupportedCodec),
        (
            with_header(FIXTURE, |h| h.reserved = 1 << 63),
            D::ReservedNonzero,
        ),
        (with_header(FIXTURE, |h| h.header_len = 72), D::BadHeaderLen),
        (
            with_header(FIXTURE, |h| h.magic = *b"ATLAS\0\x01\0"),
            D::BadMagic,
        ),
    ];
    for (bytes, expected) in cases {
        assert_eq!(refused(&bytes), vec![expected]);
    }
    // Every header defect is reported at once, sorted and deduplicated.
    let all = with_header(FIXTURE, |h| {
        h.reserved = 9;
        h.codec = CODEC_ZSTD;
        h.format_major = 3;
        h.magic = [0; 8];
    });
    assert_eq!(
        refused(&all),
        vec![
            D::BadMagic,
            D::UnsupportedMajor,
            D::UnsupportedCodec,
            D::ReservedNonzero
        ]
    );
    // An all-zero header: minor 0, no flags, codec NONE and reserved 0 are the valid values.
    assert_eq!(
        refused(&[0u8; HEADER_LEN]),
        vec![
            D::BadMagic,
            D::BadHeaderLen,
            D::UnsupportedMajor,
            D::ClassMismatch,
            D::UnsupportedDigestAlgorithm
        ]
    );
    // The schema version is judged only for the class read.
    assert_eq!(
        refused(&with_header(FIXTURE, |h| {
            h.object_class = 5;
            h.object_schema_version = 9;
        })),
        vec![D::ClassMismatch]
    );
}

#[test]
fn oversized_lengths_are_refused_before_anything_is_read() {
    for length in [MAX_DECODED_LENGTH + 1, u64::MAX, 1 << 40] {
        let both = with_header(FIXTURE, |h| {
            h.encoded_length = length;
            h.decoded_length = length;
        });
        assert_eq!(refused(&both), vec![D::LengthLimit], "{length}");
        let decoded = with_header(FIXTURE, |h| h.decoded_length = length);
        assert_eq!(refused(&decoded), vec![D::LengthLimit], "{length}");
        // The same on a bare header: nothing is sized from the declared length.
        let bare = with_header(&forge(&[])[..HEADER_LEN], |h| h.encoded_length = length);
        assert_eq!(refused(&bare), vec![D::LengthLimit]);
    }
    // At the limit the length is admitted, and the missing bytes are TRUNCATED.
    let at_limit = with_header(FIXTURE, |h| {
        h.encoded_length = MAX_DECODED_LENGTH;
        h.decoded_length = MAX_DECODED_LENGTH;
    });
    assert_eq!(refused(&at_limit), vec![D::Truncated]);
    // Encoded and decoded lengths agree but not with the bytes present.
    let short = FIXTURE.len() as u64 - HEADER_LEN as u64 - 1;
    let shorter = with_header(FIXTURE, |h| {
        h.encoded_length = short;
        h.decoded_length = short;
    });
    assert_eq!(refused(&shorter), vec![D::LengthMismatch]);
    let longer = with_header(FIXTURE, |h| {
        h.encoded_length = short + 2;
        h.decoded_length = short + 2;
    });
    assert_eq!(refused(&longer), vec![D::Truncated]);
}

#[test]
fn a_rehashed_payload_edit_passes_the_digest_and_is_caught_later() {
    // The digest authenticates bytes, not meaning: re-stamping the hash over an edited payload
    // passes step 3, and the edit must be refused by what follows.
    let mut payload = FIXTURE[HEADER_LEN..].to_vec();
    let at = payload.windows(3).position(|w| w == b"pub").unwrap();
    payload[at + 2] = b'p';
    let (_, records) = read(&forge(&payload)).unwrap();
    assert_eq!(
        records[0].visibility, "pup",
        "a re-stamped object is another object"
    );
    assert_ne!(object_address(&blake3::hash(&payload)), FIXTURE_ADDRESS);
    // The first field's wire type, after the 16-byte record header and the 2-byte tag.
    payload[RECORD_HEADER_LEN + 2] = wire::INVALID;
    assert_eq!(refused(&forge(&payload)), vec![D::InvalidWireType]);
}

// ---- refusal: framing ---------------------------------------------------------------------------

#[test]
fn framing_defects_are_refused() {
    let good = signature(&signature_fields("fn:a"));
    assert_eq!(decode(&forge(&good)).verdict, CodecVerdict::Decoded);
    // A record header cut short (by one byte, and more), and a partial record after a whole one.
    assert_eq!(
        refused(&forge(&good[..RECORD_HEADER_LEN - 1])),
        vec![D::RecordFraming]
    );
    assert_eq!(refused(&forge(&good[..10])), vec![D::RecordFraming]);
    let mut trailing = good.clone();
    trailing.extend_from_slice(&[1, 0, 1, 0]);
    assert_eq!(refused(&forge(&trailing)), vec![D::RecordFraming]);
    // A record payload longer than what remains, up to u64::MAX.
    for len in [
        u64::MAX,
        u64::MAX / 2,
        (good.len() - RECORD_HEADER_LEN + 1) as u64,
    ] {
        let mut bytes = good.clone();
        bytes[8..16].copy_from_slice(&len.to_le_bytes());
        assert_eq!(refused(&forge(&bytes)), vec![D::RecordFraming], "{len}");
    }
    // A field header cut short, and a field longer than its record, up to u32::MAX.
    for tail in [&[11u8, 0, 6][..], &[11, 0, 6, 0, 0, 0, 0]] {
        let mut cut = signature_fields("fn:a");
        cut.push(("tail", tail.to_vec()));
        assert_eq!(
            refused(&forge(&signature(&cut))),
            vec![D::FieldFraming],
            "{tail:?}"
        );
    }
    for len in [u32::MAX, 1 << 31, 1000] {
        let mut bad = field(FUNCTION_SIGNATURE, "name", b"f");
        bad[4..8].copy_from_slice(&len.to_le_bytes());
        let bytes = signature_with("fn:a", "name", Some(bad));
        assert_eq!(refused(&forge(&bytes)), vec![D::FieldFraming], "{len}");
    }
    // Inside an embedded record too.
    let lineage = record(LINEAGE_REF, &[1, 0, 8, 1, 9, 0, 0, 0, b'x']);
    let bytes = signature_with(
        "fn:a",
        "lineage",
        Some(field(FUNCTION_SIGNATURE, "lineage", &lineage)),
    );
    assert_eq!(refused(&forge(&bytes)), vec![D::FieldFraming]);
}

#[test]
fn out_of_order_and_duplicate_tags_are_refused() {
    let mut swapped = signature_fields("fn:a");
    swapped.swap(1, 2);
    assert_eq!(refused(&forge(&signature(&swapped))), vec![D::FieldOrder]);
    let mut duplicated = signature_fields("fn:a");
    duplicated.insert(2, ("name", field(FUNCTION_SIGNATURE, "name", b"g")));
    assert_eq!(
        refused(&forge(&signature(&duplicated))),
        vec![D::FieldOrder]
    );
    // Inside an embedded record.
    let param = record(
        PARAM,
        &[
            field(PARAM, "type_spelling", b"u8"),
            field(PARAM, "name", b"x"),
        ]
        .concat(),
    );
    let mut fields = signature_fields("fn:a");
    fields.insert(4, ("params", field(FUNCTION_SIGNATURE, "params", &param)));
    assert_eq!(refused(&forge(&signature(&fields))), vec![D::FieldOrder]);
}

#[test]
fn wire_types_outside_the_table_or_the_schema_are_refused() {
    for wire_type in [wire::INVALID, 13, 0xff] {
        let bad = raw_field(2, wire_type, FIELD_REQUIRED, b"f");
        let bytes = signature_with("fn:a", "name", Some(bad));
        assert_eq!(
            refused(&forge(&bytes)),
            vec![D::InvalidWireType],
            "{wire_type}"
        );
    }
    for (name, wire_type) in [
        ("name", wire::BYTES),
        ("function_id", wire::UTF8),
        ("dispatch", wire::FIXED32),
        ("visibility", wire::GLOBAL_ID),
    ] {
        let def = record_def(FUNCTION_SIGNATURE).unwrap().field(name);
        let bad = raw_field(def.tag, wire_type, FIELD_REQUIRED, b"\x01");
        let bytes = signature_with("fn:a", name, Some(bad));
        assert_eq!(refused(&forge(&bytes)), vec![D::WireTypeMismatch], "{name}");
    }
    // A RECORD field where the schema declares UTF8 is framed first, then refused by the schema.
    let bad = raw_field(2, wire::RECORD, FIELD_REQUIRED, &record(PARAM, &[]));
    let bytes = signature_with("fn:a", "name", Some(bad));
    assert_eq!(refused(&forge(&bytes)), vec![D::WireTypeMismatch]);
}

#[test]
fn nesting_deeper_than_the_limit_is_refused() {
    // A chain of RECORD fields `depth` records deep, under a valid signature's last field.
    let chain = |depth: usize| {
        let mut inner = record(LINEAGE_REF, &field(LINEAGE_REF, "record", b"x"));
        for _ in 2..depth {
            inner = record(
                LINEAGE_REF,
                &raw_field(1, wire::RECORD, FIELD_REQUIRED, &inner),
            );
        }
        signature_with(
            "fn:a",
            "lineage",
            Some(field(FUNCTION_SIGNATURE, "lineage", &inner)),
        )
    };
    assert_eq!(decode(&forge(&chain(2))).verdict, CodecVerdict::Decoded);
    // Framed up to MAX_DEPTH, then refused by the schema (a RECORD where GLOBAL_ID is declared).
    assert_eq!(
        refused(&forge(&chain(MAX_DEPTH))),
        vec![D::WireTypeMismatch]
    );
    assert_eq!(refused(&forge(&chain(MAX_DEPTH + 1))), vec![D::DepthLimit]);
    assert_eq!(refused(&forge(&chain(200))), vec![D::DepthLimit]);
}

// ---- refusal: schema ----------------------------------------------------------------------------

#[test]
fn an_undeclared_field_is_refused() {
    // After the last declared tag, before the first, inside an embedded record.
    for flags in [FIELD_REQUIRED, 0] {
        let expected = if flags == FIELD_REQUIRED {
            D::UndeclaredRequiredField
        } else {
            D::UndeclaredOptionalField
        };
        let mut after = signature_fields("fn:a");
        after.push(("extra", raw_field(11, wire::UTF8, flags, b"x")));
        assert_eq!(refused(&forge(&signature(&after))), vec![expected]);
        // Tags 1 to 10 are all declared: tag 0 is the undeclared one before them.
        let mut before = signature_fields("fn:a");
        before.insert(0, ("extra", raw_field(0, wire::UTF8, flags, b"x")));
        assert_eq!(refused(&forge(&signature(&before))), vec![expected]);
        let param = record(
            PARAM,
            &[
                field(PARAM, "name", b"x"),
                field(PARAM, "type_spelling", b"u8"),
                raw_field(3, wire::BOOL, flags, &[1]),
            ]
            .concat(),
        );
        let mut fields = signature_fields("fn:a");
        fields.insert(4, ("params", field(FUNCTION_SIGNATURE, "params", &param)));
        assert_eq!(refused(&forge(&signature(&fields))), vec![expected]);
    }
    // The same field with a correct hash and in canonical position is still refused: skipping it
    // would admit bytes the object's identity covers but no reader understands.
    let mut extra = signature_fields("fn:a");
    extra.push(("extra", raw_field(0xffff, wire::BYTES, 0, b"")));
    assert_eq!(
        refused(&forge(&signature(&extra))),
        vec![D::UndeclaredOptionalField]
    );
}

#[test]
fn field_flags_must_be_the_declared_ones() {
    let def = record_def(FUNCTION_SIGNATURE).unwrap();
    // A required field marked optional; an optional field marked required; an undefined bit.
    let name = raw_field(2, wire::UTF8, 0, b"f");
    assert_eq!(
        refused(&forge(&signature_with("fn:a", "name", Some(name)))),
        vec![D::FieldFlags]
    );
    let mut owner = signature_fields("fn:a");
    owner.insert(
        2,
        (
            "owner",
            raw_field(def.field("owner").tag, wire::UTF8, FIELD_REQUIRED, b"T"),
        ),
    );
    assert_eq!(refused(&forge(&signature(&owner))), vec![D::FieldFlags]);
    for flags in [2u8, 0x80, FIELD_REQUIRED | 4] {
        let name = raw_field(2, wire::UTF8, flags, b"f");
        assert_eq!(
            refused(&forge(&signature_with("fn:a", "name", Some(name)))),
            vec![D::FieldFlags],
            "{flags}"
        );
        let extra = raw_field(11, wire::UTF8, flags, b"f");
        let mut fields = signature_fields("fn:a");
        fields.push(("extra", extra));
        assert_eq!(
            refused(&forge(&signature(&fields))),
            vec![D::FieldFlags],
            "{flags}"
        );
    }
}

#[test]
fn a_missing_required_field_is_refused() {
    for name in ["function_id", "name", "dispatch", "visibility", "lineage"] {
        assert_eq!(
            refused(&forge(&signature_with("fn:a", name, None))),
            vec![D::MissingRequiredField],
            "{name}"
        );
    }
    let param = record(PARAM, &field(PARAM, "name", b"x"));
    let mut fields = signature_fields("fn:a");
    fields.insert(4, ("params", field(FUNCTION_SIGNATURE, "params", &param)));
    assert_eq!(
        refused(&forge(&signature(&fields))),
        vec![D::MissingRequiredField]
    );
    let lineage = record(LINEAGE_REF, &[]);
    let bytes = signature_with(
        "fn:a",
        "lineage",
        Some(field(FUNCTION_SIGNATURE, "lineage", &lineage)),
    );
    assert_eq!(refused(&forge(&bytes)), vec![D::MissingRequiredField]);
}

#[test]
fn record_kinds_versions_and_flags_are_the_declared_ones() {
    let fields: Vec<u8> = signature_fields("fn:a")
        .into_iter()
        .flat_map(|(_, b)| b)
        .collect();
    for kind in [0, PARAM, LINEAGE_REF, 9, u16::MAX] {
        assert_eq!(
            refused(&forge(&record(kind, &fields))),
            vec![D::UndeclaredRecordKind],
            "{kind}"
        );
    }
    for version in [0, 2, u16::MAX] {
        let bytes = raw_record(FUNCTION_SIGNATURE, version, 0, &fields);
        assert_eq!(
            refused(&forge(&bytes)),
            vec![D::UnsupportedRecordVersion],
            "{version}"
        );
    }
    for flags in [1, 1 << 31] {
        let bytes = raw_record(FUNCTION_SIGNATURE, RECORD_SCHEMA_VERSION, flags, &fields);
        assert_eq!(refused(&forge(&bytes)), vec![D::RecordFlags], "{flags}");
    }
    // An embedded record of another kind than the field declares, or of an unsupported version.
    let wrong = record(PARAM, &field(PARAM, "name", b"x"));
    let bytes = signature_with(
        "fn:a",
        "lineage",
        Some(field(FUNCTION_SIGNATURE, "lineage", &wrong)),
    );
    assert_eq!(refused(&forge(&bytes)), vec![D::UndeclaredRecordKind]);
    let old = raw_record(LINEAGE_REF, 2, 0, &field(LINEAGE_REF, "record", b"x"));
    let bytes = signature_with(
        "fn:a",
        "lineage",
        Some(field(FUNCTION_SIGNATURE, "lineage", &old)),
    );
    assert_eq!(refused(&forge(&bytes)), vec![D::UnsupportedRecordVersion]);
    let flagged = raw_record(
        LINEAGE_REF,
        RECORD_SCHEMA_VERSION,
        1,
        &field(LINEAGE_REF, "record", b"x"),
    );
    let bytes = signature_with(
        "fn:a",
        "lineage",
        Some(field(FUNCTION_SIGNATURE, "lineage", &flagged)),
    );
    assert_eq!(refused(&forge(&bytes)), vec![D::RecordFlags]);
}

#[test]
fn malformed_values_are_refused() {
    let f = |name: &str, bytes: &[u8]| field(FUNCTION_SIGNATURE, name, bytes);
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("dispatch", f("dispatch", &[0x81, 0x00])), // not minimal
        ("dispatch", f("dispatch", &[])),
        ("dispatch", f("dispatch", &[0x80])), // truncated
        ("dispatch", f("dispatch", &[1, 1])), // trailing
        ("dispatch", f("dispatch", &[0])),
        ("dispatch", f("dispatch", &[4])),
        ("dispatch", f("dispatch", &[0xff; 11])),
        ("name", f("name", &[0xff, 0xfe])),
        ("visibility", f("visibility", &[0xc3])),
        ("function_id", f("function_id", b"")),
        ("function_id", f("function_id", &[0x80])),
        ("lineage", f("lineage", &[])),
        (
            "lineage",
            f(
                "lineage",
                &record(LINEAGE_REF, &field(LINEAGE_REF, "record", b"")),
            ),
        ),
    ];
    for (name, bytes) in cases {
        let forged = forge(&signature_with("fn:a", name, Some(bytes.clone())));
        assert_eq!(
            refused(&forged),
            vec![D::MalformedValue],
            "{name} {bytes:02x?}"
        );
    }
    // Optional fields: a hash that is not 32 bytes, an empty parameter list, bad UTF-8.
    for (at, name, bytes) in [
        (4, "body_fingerprint", vec![0u8; 31]),
        (4, "body_fingerprint", vec![0u8; 33]),
        (4, "params", Vec::new()),
        (2, "owner", vec![0xc0, 0x80]),
        (4, "documentation", vec![0xed, 0xa0, 0x80]),
        (4, "result", vec![0xff]),
    ] {
        let mut fields = signature_fields("fn:a");
        fields.insert(at, (name, f(name, &bytes)));
        fields.sort_by_key(|(n, _)| record_def(FUNCTION_SIGNATURE).unwrap().field(n).tag);
        assert_eq!(
            refused(&forge(&signature(&fields))),
            vec![D::MalformedValue],
            "{name}"
        );
    }
    // A malformed parameter name.
    let param = record(
        PARAM,
        &[
            field(PARAM, "name", &[0xff]),
            field(PARAM, "type_spelling", b"u8"),
        ]
        .concat(),
    );
    let mut fields = signature_fields("fn:a");
    fields.insert(4, ("params", f("params", &param)));
    assert_eq!(
        refused(&forge(&signature(&fields))),
        vec![D::MalformedValue]
    );
}

#[test]
fn every_dispatch_code_reads_back() {
    for (dispatch, code) in DISPATCH_CODES {
        let bytes = signature_with(
            "fn:a",
            "dispatch",
            Some(field(FUNCTION_SIGNATURE, "dispatch", &[code as u8])),
        );
        let (_, records) = read(&forge(&bytes)).unwrap();
        assert_eq!(records[0].dispatch, dispatch);
    }
}

#[test]
fn duplicate_identities_empty_objects_and_non_canonical_forms_are_refused() {
    let a = signature(&signature_fields("fn:a"));
    let b = signature(&signature_fields("fn:b"));
    assert_eq!(
        decode(&forge(&[a.clone(), b.clone()].concat())).verdict,
        CodecVerdict::Decoded
    );
    // Records out of identity order.
    assert_eq!(
        refused(&forge(&[b.clone(), a.clone()].concat())),
        vec![D::NonCanonical]
    );
    // Two records with one identity, adjacent or not.
    assert_eq!(
        refused(&forge(&[a.clone(), a.clone()].concat())),
        vec![D::DuplicateIdentity]
    );
    assert_eq!(
        refused(&forge(&[a.clone(), b.clone(), a.clone()].concat())),
        vec![D::DuplicateIdentity]
    );
    // No record at all.
    assert_eq!(refused(&forge(&[])), vec![D::EmptyObject]);
    // Lineage out of order, or repeated.
    let lineage = |ids: &[&[u8]]| -> Vec<u8> {
        let refs: Vec<u8> = ids
            .iter()
            .flat_map(|id| record(LINEAGE_REF, &field(LINEAGE_REF, "record", id)))
            .collect();
        signature_with(
            "fn:a",
            "lineage",
            Some(field(FUNCTION_SIGNATURE, "lineage", &refs)),
        )
    };
    assert_eq!(
        decode(&forge(&lineage(&[b"a", b"b"]))).verdict,
        CodecVerdict::Decoded
    );
    assert_eq!(
        refused(&forge(&lineage(&[b"b", b"a"]))),
        vec![D::NonCanonical]
    );
    assert_eq!(
        refused(&forge(&lineage(&[b"a", b"a"]))),
        vec![D::NonCanonical]
    );
    // The fixture's records, reordered, re-hashed.
    let canonical = encode_functions(&several()).unwrap();
    let (_, records) = read(&canonical).unwrap();
    let mut reversed = Vec::new();
    for r in records.iter().rev() {
        let one = encode_functions(std::slice::from_ref(r)).unwrap();
        reversed.extend_from_slice(&one[HEADER_LEN..]);
    }
    assert_eq!(refused(&forge(&reversed)), vec![D::NonCanonical]);
}

// ---- arbitrary input ----------------------------------------------------------------------------

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
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let payload = FIXTURE[HEADER_LEN..].to_vec();
    let canonical = encode_functions(&several()).unwrap()[HEADER_LEN..].to_vec();
    for round in 0..4000 {
        // Random bytes, raw and behind a valid header (so framing and schema see them).
        let len = rng.below(300);
        let noise: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
        assert_eq!(decode(&noise).verdict, CodecVerdict::Refused);
        let behind = forge(&noise);
        if let Ok((_, records)) = read(&behind) {
            assert_eq!(encode_functions(&records).unwrap(), behind, "{round}");
        }
        // Several random edits of a real payload, re-hashed: whatever it decodes to must be
        // canonical, so decoding and writing again gives the same bytes.
        let mut edited = if round % 2 == 0 {
            payload.clone()
        } else {
            canonical.clone()
        };
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
        match read(&bytes) {
            Ok((_, records)) => assert_eq!(encode_functions(&records).unwrap(), bytes, "{round}"),
            Err(defects) => assert!(!defects.is_empty()),
        }
    }
}

#[test]
fn every_report_says_what_it_does_not_verify() {
    for bytes in [FIXTURE, &[][..], &FIXTURE[..HEADER_LEN]] {
        let report = decode(bytes);
        assert_eq!(report.schema, CODEC_SCHEMA_VERSION);
        assert_eq!(report.not_verified, not_verified());
    }
    for needle in [
        "manifest",
        "root identity",
        "cross-object",
        "lineage",
        "M11",
        "bodies",
        "ZSTD",
        "SHA256",
        "source module",
    ] {
        assert!(
            CODEC_NOT_VERIFIED.iter().any(|s| s.contains(needle)),
            "{needle}"
        );
    }
}
