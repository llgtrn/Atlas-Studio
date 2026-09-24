use super::*;

fn fact(id: &str, status: &str, revision: Option<&str>, span: Option<&str>) -> CensusFact {
    CensusFact {
        id: id.into(),
        kind: "SourceArtifact".into(),
        status: status.into(),
        subject: "artifact:core/src/lib.rs".into(),
        predicate: "disposition".into(),
        object: "PARSED".into(),
        source_path: "core/src/lib.rs".into(),
        revision: revision.map(Into::into),
        extractor: "atlas.source.rust.bootstrap.v1".into(),
        span: span.map(Into::into),
    }
}

fn sample() -> CensusAtlas {
    let mut atlas = CensusAtlas {
        manifest: RootManifest {
            genome_schema: "atlas.genome.v1".into(),
            genome_hash: blake3::hash(b"genome"),
            census_digest: blake3::hash(b"census"),
            revision: "git:abc123".into(),
            certificate_id: "census-certificate:blake3-256:00".into(),
            seal: UNSEALED.into(),
            tool: "atlas-systemizer 0.1.0".into(),
            mode: "THIN".into(),
        },
        facts: vec![
            fact("f:3", "UNSUPPORTED", Some("git:abc123"), None),
            fact("f:1", "OBSERVED", Some("git:abc123"), Some("12:4")),
            fact("f:2", "UNKNOWN", None, None),
            fact("f:ü", "CONFLICT", Some("git:abc123"), Some("1:1")),
        ],
        obligations: vec![CensusObligation {
            artifact: "artifact:core/src/lib.rs".into(),
            dimension: "CALL".into(),
            extractor: "atlas.rust.source-semantic.v1".into(),
            extractor_version: "0.1.0".into(),
            status: "UNKNOWN".into(),
        }],
        nodes: vec![DeclaredNodeRecord {
            name: "Core".into(),
            kind: "Runtime".into(),
            origin: "declared".into(),
        }],
        edges: vec![DeclaredEdgeRecord {
            from: "Runtime".into(),
            relation: "depends_on".into(),
            to: "Core".into(),
        }],
        certificate: CertificateRecord {
            certificate_id: "census-certificate:blake3-256:00".into(),
            state: "RECONCILED".into(),
            blockers: vec![
                "COVERAGE_UNKNOWN: CALL".into(),
                "ATLAS_ROOT_ABSENT: x".into(),
            ],
        },
    };
    atlas.canonicalize();
    atlas
}

fn rejected(bytes: &[u8]) -> String {
    match read(bytes) {
        Err(AtlasError::Rejected(why)) => why,
        other => panic!("expected rejection, got {other:?}"),
    }
}

#[test]
fn a_census_container_round_trips_exactly_and_deterministically() {
    let atlas = sample();
    let bytes = write(&atlas).unwrap();
    assert_eq!(&bytes[..8], b"ATLAS\0\x01\0");
    assert_eq!(u16_at(&bytes, 14) & FLAG_UNSEALED, FLAG_UNSEALED);
    let (decoded, root) = read(&bytes).unwrap();
    assert_eq!(
        decoded, atlas,
        "UNKNOWN/UNSUPPORTED/CONFLICT and absent fields survive"
    );
    assert_eq!(
        write(&decoded).unwrap(),
        bytes,
        "re-encoding is byte-identical"
    );
    let (_, again) = read(&write(&sample()).unwrap()).unwrap();
    assert_eq!(root, again);
    // The root identity covers content: any semantic change moves it.
    let mut changed = sample();
    changed.facts[0].status = "DERIVED".into();
    changed.canonicalize();
    assert_ne!(read(&write(&changed).unwrap()).unwrap().1, root);
    // ...including a change only in a non-manifest section, through the manifest commitments.
    let mut blocker = sample();
    blocker.certificate.blockers.push("ZZZ: new".into());
    assert_ne!(read(&write(&blocker).unwrap()).unwrap().1, root);
}

#[test]
fn the_writer_refuses_a_seal_it_cannot_prove_and_non_canonical_content() {
    let mut sealed = sample();
    sealed.manifest.seal = "SEALED".into();
    assert!(matches!(write(&sealed), Err(AtlasError::Refused(_))));
    let mut unordered = sample();
    unordered.facts.reverse();
    assert!(matches!(write(&unordered), Err(AtlasError::Refused(_))));
    let mut duplicated = sample();
    duplicated.edges.push(duplicated.edges[0].clone());
    assert!(matches!(write(&duplicated), Err(AtlasError::Refused(_))));
}

#[test]
fn every_single_byte_corruption_is_rejected() {
    let bytes = write(&sample()).unwrap();
    for at in 0..bytes.len() {
        if (12..14).contains(&at) {
            continue; // format_minor: any minor of major 1 is readable by contract
        }
        let mut corrupt = bytes.clone();
        corrupt[at] ^= 0x01;
        assert!(
            read(&corrupt).is_err(),
            "flipping byte {at} of {} was accepted",
            bytes.len()
        );
    }
}

#[test]
fn every_truncation_and_extension_is_rejected() {
    let bytes = write(&sample()).unwrap();
    for len in 0..bytes.len() {
        assert!(
            read(&bytes[..len]).is_err(),
            "truncated to {len} was accepted"
        );
    }
    let mut extended = bytes.clone();
    extended.push(0);
    assert_eq!(rejected(&extended), "trailing bytes after the last region");
}

#[test]
fn reader_steps_reject_with_their_reason() {
    let bytes = write(&sample()).unwrap();
    let with = |at: usize, value: &[u8]| {
        let mut b = bytes.clone();
        b[at..at + value.len()].copy_from_slice(value);
        b
    };
    assert_eq!(rejected(&with(0, b"ATLAX")), "bad magic");
    assert_eq!(
        rejected(&with(10, &2u16.to_le_bytes())),
        "unknown format major version 2"
    );
    assert_eq!(
        rejected(&with(16, &2u16.to_le_bytes())),
        "unsupported digest algorithm"
    );
    assert_eq!(
        rejected(&with(18, &1u16.to_le_bytes())),
        "unsupported default compression"
    );
    assert_eq!(
        rejected(&with(14, &0u16.to_le_bytes())),
        "only unsealed census containers are defined; a seal needs a seal gate"
    );
    assert_eq!(
        rejected(&with(20, &[0u8; 32])),
        "header genome hash differs from the root manifest"
    );
    // Swapping two directory entries breaks the canonical directory order.
    let dir = u64_at(&bytes, 52) as usize;
    let mut swapped = bytes.clone();
    let (a, b) = (dir, dir + DIRECTORY_ENTRY_LEN);
    let first = swapped[a..a + DIRECTORY_ENTRY_LEN].to_vec();
    let second = swapped[b..b + DIRECTORY_ENTRY_LEN].to_vec();
    swapped[a..a + DIRECTORY_ENTRY_LEN].copy_from_slice(&second);
    swapped[b..b + DIRECTORY_ENTRY_LEN].copy_from_slice(&first);
    assert_eq!(
        rejected(&swapped),
        "section directory is not in canonical order"
    );
    // A second copy of the manifest's entry (same bounds) overlaps it.
    let mut doubled = bytes.clone();
    let entry = doubled[dir..dir + DIRECTORY_ENTRY_LEN].to_vec();
    doubled.splice(dir..dir, entry);
    let new_len = u64_at(&doubled, 60) + DIRECTORY_ENTRY_LEN as u64;
    doubled[60..68].copy_from_slice(&new_len.to_le_bytes());
    assert!(read(&doubled).is_err());
}

fn one_record(kind: u16, record: Record) -> Vec<u8> {
    let mut content = Vec::new();
    Record { kind, ..record }.encode(&mut content);
    content
}

#[test]
fn field_framing_skips_unknown_optional_fields_and_rejects_unknown_required_ones() {
    let strings = vec!["a".to_owned(), "b".to_owned()];
    let optional_extra = one_record(
        1,
        Record::new(1)
            .field(1, WIRE_LOCAL_INDEX, REQUIRED, &[1])
            .field(99, WIRE_UTF8, 0, b"future"),
    );
    let records = parse_records(&optional_extra, 7).unwrap();
    let f = Fields::new(&records[0], &[1], 7).unwrap();
    assert_eq!(f.string(1, &strings).unwrap(), "b");

    let required_extra = one_record(
        1,
        Record::new(1)
            .field(1, WIRE_LOCAL_INDEX, REQUIRED, &[1])
            .field(99, WIRE_UTF8, REQUIRED, b"future"),
    );
    let records = parse_records(&required_extra, 7).unwrap();
    assert!(Fields::new(&records[0], &[1], 7).is_err());

    let descending = one_record(
        1,
        Record::new(1)
            .field(2, WIRE_LOCAL_INDEX, REQUIRED, &[0])
            .field(1, WIRE_LOCAL_INDEX, REQUIRED, &[0]),
    );
    assert!(parse_records(&descending, 7).is_err());

    // A valid string index under the wrong wire type: only the wire-type check rejects it.
    let wrong_wire = one_record(1, Record::new(1).field(1, WIRE_UVARINT, REQUIRED, &[1]));
    let records = parse_records(&wrong_wire, 7).unwrap();
    assert!(
        Fields::new(&records[0], &[1], 7)
            .unwrap()
            .string(1, &strings)
            .is_err()
    );

    let out_of_table = one_record(1, Record::new(1).field(1, WIRE_LOCAL_INDEX, REQUIRED, &[2]));
    let records = parse_records(&out_of_table, 7).unwrap();
    assert!(
        Fields::new(&records[0], &[1], 7)
            .unwrap()
            .string(1, &strings)
            .is_err()
    );
    let missing = one_record(1, Record::new(1));
    let records = parse_records(&missing, 7).unwrap();
    assert!(
        Fields::new(&records[0], &[1], 7)
            .unwrap()
            .string(1, &strings)
            .is_err()
    );
}

#[test]
fn uvarints_are_minimal_bounded_and_exact() {
    for value in [0u64, 1, 127, 128, 300, u64::from(u32::MAX), u64::MAX] {
        let mut bytes = Vec::new();
        uvarint(value, &mut bytes);
        assert_eq!(read_uvarint(&bytes).unwrap(), value);
    }
    assert!(read_uvarint(&[]).is_err(), "empty");
    assert!(read_uvarint(&[0x80]).is_err(), "truncated");
    assert!(read_uvarint(&[0x80, 0x00]).is_err(), "not minimal");
    assert!(read_uvarint(&[0x01, 0x00]).is_err(), "trailing bytes");
    let mut overflow = vec![0xff; 9];
    overflow.push(0x02);
    assert!(read_uvarint(&overflow).is_err(), "overflows u64");
    assert!(read_uvarint(&[0xff; 11]).is_err(), "too long");
}

#[test]
fn schema_ids_are_pinned_and_distinct() {
    let ids: std::collections::BTreeSet<u64> = [
        ROOT_MANIFEST,
        STRING_TABLE,
        SEMANTIC_RECORDS,
        GRAPH_NODES,
        GRAPH_EDGES,
        OBLIGATIONS,
        CENSUS_CERTIFICATE,
    ]
    .into_iter()
    .map(schema_id)
    .collect();
    assert_eq!(ids.len(), 7);
    let expected = blake3::hash(b"atlas.wire.v1/root-manifest");
    assert_eq!(
        schema_id(ROOT_MANIFEST),
        u64::from_le_bytes(expected[..8].try_into().unwrap())
    );
}

/// Rebuilds a container after `edit` changes one section's content: the directory hash and
/// length are recomputed, and -- when `recommit` -- so are the root manifest's commitments. The
/// result is internally hash-consistent, so only the semantic checks can reject it.
fn resealed(bytes: &[u8], kind: u16, recommit: bool, edit: impl Fn(&mut Vec<u8>)) -> Vec<u8> {
    let dir = u64_at(bytes, 52) as usize;
    let count = u64_at(bytes, 60) as usize / DIRECTORY_ENTRY_LEN;
    let mut sections: Vec<(u16, Vec<u8>, u64)> = (0..count)
        .map(|i| {
            let e = &bytes[dir + i * DIRECTORY_ENTRY_LEN..];
            let (offset, len) = (u64_at(e, 16) as usize, u64_at(e, 24) as usize);
            (
                u16_at(e, 0),
                bytes[offset..offset + len].to_vec(),
                u64_at(e, 40),
            )
        })
        .collect();
    for (k, content, _) in &mut sections {
        if *k == kind {
            edit(content);
        }
    }
    if recommit {
        let head_len = 8 + u32_at(&sections[0].1, 4) as usize;
        let mut manifest = sections[0].1[..head_len].to_vec();
        for (k, content, records) in &sections[1..] {
            Record::new(2)
                .uvarint(1, u64::from(*k))
                .uvarint(2, schema_id(*k))
                .hash(3, &blake3::hash(content))
                .uvarint(4, *records)
                .encode(&mut manifest);
        }
        sections[0].1 = manifest;
    }
    let mut out = bytes[..HEADER_LEN].to_vec();
    let mut directory = Vec::new();
    for (k, content, records) in &sections {
        let offset = out.len() as u64;
        out.extend_from_slice(content);
        directory.extend_from_slice(&k.to_le_bytes());
        directory.extend_from_slice(&[0; 6]);
        directory.extend_from_slice(&schema_id(*k).to_le_bytes());
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(&(content.len() as u64).to_le_bytes());
        directory.extend_from_slice(&(content.len() as u64).to_le_bytes());
        directory.extend_from_slice(&records.to_le_bytes());
        directory.extend_from_slice(&blake3::hash(content));
    }
    let directory_offset = out.len() as u64;
    out.extend_from_slice(&directory);
    out[52..60].copy_from_slice(&directory_offset.to_le_bytes());
    out
}

/// Splits a section's content into its raw records.
fn raw_records(content: &[u8]) -> Vec<Vec<u8>> {
    let mut records = Vec::new();
    let mut at = 0;
    while at < content.len() {
        let end = at + 8 + u32_at(content, at + 4) as usize;
        records.push(content[at..end].to_vec());
        at = end;
    }
    records
}

#[test]
fn hash_consistent_forgeries_are_rejected_by_the_semantic_checks() {
    let bytes = write(&sample()).unwrap();
    assert_eq!(
        read(&resealed(&bytes, STRING_TABLE, true, |_| {}))
            .unwrap()
            .1,
        read(&bytes).unwrap().1,
        "the reseal helper itself preserves a valid container"
    );
    let flip_last = |c: &mut Vec<u8>| {
        let last = c.len() - 1;
        c[last] ^= 1;
    };
    // A rewritten section whose directory hash was fixed but which the manifest never committed
    // to: the root identity would not cover it.
    assert_eq!(
        rejected(&resealed(&bytes, CENSUS_CERTIFICATE, false, flip_last)),
        "root manifest does not commit to exactly the directory's sections"
    );
    let swap_first_two = |c: &mut Vec<u8>| {
        let records = raw_records(c);
        let mut swapped = records.clone();
        swapped.swap(0, 1);
        *c = swapped.concat();
    };
    assert_eq!(
        rejected(&resealed(&bytes, STRING_TABLE, true, swap_first_two)),
        "string table is not sorted and unique"
    );
    assert_eq!(
        rejected(&resealed(&bytes, SEMANTIC_RECORDS, true, swap_first_two)),
        "records are not in canonical order"
    );
    let unknown_kind = |c: &mut Vec<u8>| c[0] = 9;
    assert_eq!(
        rejected(&resealed(&bytes, GRAPH_EDGES, true, unknown_kind)),
        "section 8: unknown record kind 9"
    );
    let drop_last = |c: &mut Vec<u8>| {
        let records = raw_records(c);
        *c = records[..records.len() - 1].concat();
    };
    assert_eq!(
        rejected(&resealed(&bytes, SEMANTIC_RECORDS, true, drop_last)),
        "section 6 record count mismatch"
    );
    // A certificate section for another certificate than the manifest names.
    let other = {
        let mut atlas = sample();
        atlas.certificate.certificate_id = "census-certificate:other".into();
        encode(&atlas)
    };
    assert_eq!(
        rejected(&other),
        "certificate section differs from the root manifest's certificate id"
    );
    // Non-canonical content that `write` would refuse is rejected on read as well.
    let mut unordered = sample();
    unordered.nodes.push(DeclaredNodeRecord {
        name: "A".into(),
        kind: "Runtime".into(),
        origin: "declared".into(),
    });
    assert_eq!(
        rejected(&encode(&unordered)),
        "records are not in canonical order"
    );
}
