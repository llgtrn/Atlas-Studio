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
    let f = Fields::new(&records[0], GRAPH_NODES).unwrap();
    assert_eq!(f.string("name", &strings).unwrap(), "b");

    let required_extra = one_record(
        1,
        Record::new(1)
            .field(1, WIRE_LOCAL_INDEX, REQUIRED, &[1])
            .field(99, WIRE_UTF8, REQUIRED, b"future"),
    );
    let records = parse_records(&required_extra, 7).unwrap();
    assert!(Fields::new(&records[0], GRAPH_NODES).is_err());

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
        Fields::new(&records[0], GRAPH_NODES)
            .unwrap()
            .string("name", &strings)
            .is_err()
    );

    let out_of_table = one_record(1, Record::new(1).field(1, WIRE_LOCAL_INDEX, REQUIRED, &[2]));
    let records = parse_records(&out_of_table, 7).unwrap();
    assert!(
        Fields::new(&records[0], GRAPH_NODES)
            .unwrap()
            .string("name", &strings)
            .is_err()
    );
    let missing = one_record(1, Record::new(1));
    let records = parse_records(&missing, 7).unwrap();
    assert!(
        Fields::new(&records[0], GRAPH_NODES)
            .unwrap()
            .string("name", &strings)
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
fn schema_ids_are_derived_from_definitions_and_distinct() {
    let ids: std::collections::BTreeSet<u64> = schema::SECTIONS
        .iter()
        .map(|def| schema_id(def.section))
        .collect();
    assert_eq!(ids.len(), schema::SECTIONS.len());
    // The id is the hash of the declared definition, dependencies included.
    let facts = schema::section(SEMANTIC_RECORDS).unwrap();
    let text = schema::definition_text(facts);
    assert!(text.starts_with("atlas.wire.v1/census-facts\n"));
    assert!(text.contains("field 4 subject 9 required\n"));
    assert!(text.contains("field 8 revision 9 optional\n"));
    assert!(text.contains("depends string-table "));
    let digest = blake3::hash(text.as_bytes());
    assert_eq!(
        schema_id(SEMANTIC_RECORDS),
        u64::from_le_bytes(digest[..8].try_into().unwrap())
    );
}

/// G68 (Glean): any change to a declared field -- a repurposed tag, a rename, another wire type,
/// requiredness -- or to a dependency's definition is a different schema identity.
#[test]
fn every_definition_change_moves_the_schema_identity() {
    let facts = *schema::section(SEMANTIC_RECORDS).unwrap();
    let base = schema::schema_hash(&facts);
    let fields = facts.records[0].fields;
    let with = |edit: &dyn Fn(&mut Vec<schema::FieldDef>)| {
        let mut changed = fields.to_vec();
        edit(&mut changed);
        let leaked: &'static [schema::FieldDef] = Box::leak(changed.into_boxed_slice());
        let record: &'static [schema::RecordDef] = Box::leak(Box::new([schema::RecordDef {
            fields: leaked,
            ..facts.records[0]
        }]));
        schema::schema_hash(&schema::SectionDef {
            records: record,
            ..facts
        })
    };
    let swap_subject_predicate = |f: &mut Vec<schema::FieldDef>| {
        let (s, p) = (f[3].tag, f[4].tag);
        f[3].tag = p;
        f[4].tag = s;
    };
    for (edit, why) in [
        (
            &swap_subject_predicate as &dyn Fn(&mut Vec<schema::FieldDef>),
            "tags repurposed",
        ),
        (
            &|f: &mut Vec<schema::FieldDef>| f[5].name = "value",
            "field renamed",
        ),
        (
            &|f: &mut Vec<schema::FieldDef>| f[5].wire = WIRE_UTF8,
            "wire type",
        ),
        (
            &|f: &mut Vec<schema::FieldDef>| f[7].required = true,
            "requiredness",
        ),
        (
            &|f: &mut Vec<schema::FieldDef>| f.pop().map(|_| ()).unwrap(),
            "field removed",
        ),
    ] {
        assert_ne!(with(edit), base, "{why}");
    }
    assert_eq!(
        with(&|_| {}),
        base,
        "an unchanged definition keeps its identity"
    );
    // A dependency's definition is part of the identity.
    let without_dependency = schema::schema_hash(&schema::SectionDef {
        depends_on: &[],
        ..facts
    });
    assert_ne!(without_dependency, base);
}

/// A container written under another definition of a section is refused, not misread: its
/// schema id no longer matches this reader's.
#[test]
fn a_container_from_another_schema_definition_is_refused() {
    let bytes = write(&sample()).unwrap();
    let dir = u64_at(&bytes, 52) as usize;
    let count = u64_at(&bytes, 60) as usize / DIRECTORY_ENTRY_LEN;
    let facts_entry = (0..count)
        .map(|i| dir + i * DIRECTORY_ENTRY_LEN)
        .find(|at| u16_at(&bytes, *at) == SEMANTIC_RECORDS)
        .unwrap();
    let mut other = bytes.clone();
    let foreign = schema_id(SEMANTIC_RECORDS) ^ 1;
    other[facts_entry + 8..facts_entry + 16].copy_from_slice(&foreign.to_le_bytes());
    assert_eq!(rejected(&other), "section 6 has an unknown schema id");
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
        encode(&atlas, schema_id)
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
        rejected(&encode(&unordered, schema_id)),
        "records are not in canonical order"
    );
}

/// Table-driven records carry the declared REQUIRED flag per field (so an older reader can skip a
/// newer optional field) and are emitted in ascending tag order whatever the call order.
#[test]
fn table_driven_records_follow_their_declaration_on_the_wire() {
    let strings = Strings(
        [("a".to_owned(), 0), ("b".to_owned(), 1)]
            .into_iter()
            .collect(),
    );
    let mut content = Vec::new();
    Record::of(SEMANTIC_RECORDS, 1)
        .optional_string("span", &strings, Some("a"))
        .string("subject", &strings, "b")
        .optional_string("revision", &strings, None)
        .string("id", &strings, "a")
        .encode(&mut content);
    let records = parse_records(&content, SEMANTIC_RECORDS).unwrap();
    let fields: Vec<(u16, u8)> = records[0].fields.iter().map(|f| (f.tag, f.flags)).collect();
    assert_eq!(fields, [(1, REQUIRED), (4, REQUIRED), (10, 0)]);
}

// --- G86: schema evolution conforms (FlatBuffers `flatc --conform`, absorbed) -----------------

#[test]
fn the_current_schema_is_the_latest_recorded_generation() {
    // Changing `SECTIONS` without appending its definition to the history fails here, so every
    // definition a container could carry stays recorded.
    let generations = schema::recorded_generations(schema::HISTORY);
    let (_, latest) = generations.last().expect("a recorded generation");
    let recorded: String = latest.iter().map(|s| s.text.as_str()).collect();
    let current: String = schema::SECTIONS
        .iter()
        .map(schema::definition_text)
        .collect();
    assert_eq!(recorded, current);
    for section in latest {
        let def = schema::SECTIONS
            .iter()
            .find(|d| d.name == section.name)
            .unwrap();
        assert_eq!(section.schema_id(), schema_id(def.section));
    }
}

#[test]
fn every_recorded_schema_conforms_to_the_current_one() {
    for (generation, sections) in schema::recorded_generations(schema::HISTORY) {
        for recorded in &sections {
            let current = schema::SECTIONS
                .iter()
                .find(|d| d.name == recorded.name)
                .unwrap_or_else(|| panic!("{generation}: section {} dropped", recorded.name));
            assert_eq!(schema::conforms(recorded, current), Ok(()), "{generation}");
            assert!(
                schema::accepted_schema_ids(schema::HISTORY, current.section)
                    .contains(&recorded.schema_id())
            );
        }
    }
}

/// The census-certificate section as a recorded text, edited by `edit`.
fn recorded_certificate(edit: impl Fn(String) -> String) -> schema::RecordedSection {
    let current = schema::section(CENSUS_CERTIFICATE).unwrap();
    let text = edit(schema::definition_text(current));
    let history = format!("generation TEST\n{text}");
    schema::recorded_generations(&history)
        .pop()
        .unwrap()
        .1
        .pop()
        .unwrap()
}

#[test]
fn conformance_refuses_every_breaking_evolution_and_admits_compatible_ones() {
    let current = schema::section(CENSUS_CERTIFICATE).unwrap();
    let same = recorded_certificate(|t| t);
    assert_eq!(schema::conforms(&same, current), Ok(()));
    // Compatible: an older definition without the blocker record kind (new kinds are allowed),
    // or with a field under another name (the wire is tag-addressed).
    let fewer_kinds =
        recorded_certificate(|t| t.replace("record 2 blocker\nfield 1 text 9 required\n", ""));
    assert_eq!(schema::conforms(&fewer_kinds, current), Ok(()));
    let renamed = recorded_certificate(|t| t.replace("field 2 state", "field 2 status"));
    assert_eq!(schema::conforms(&renamed, current), Ok(()));
    // Breaking: a retagged, retyped, removed or newly required field, a dropped record kind.
    let breaking = [
        (
            "field 2 state 9",
            "field 3 state 9",
            "field 3 (state) removed",
        ),
        ("field 2 state 9", "field 2 state 6", "changed wire type"),
        ("record 2 blocker", "record 3 blocker", "record kind 3"),
        (
            "field 2 state 9 required",
            "field 2 state 9 optional",
            "became required",
        ),
        (
            "field 1 text 9 required\n",
            "field 1 text 9 required\nfield 2 note 6 optional\n",
            "field 2 (note) removed",
        ),
        // An older definition without `state`: the current, required `state` is new.
        (
            "field 2 state 9 required\n",
            "",
            "new field 2 (state) is required",
        ),
        (
            "depends string-table",
            "depends root-manifest 00\ndepends string-table",
            "dependency root-manifest dropped",
        ),
    ];
    for (from, to, reason) in breaking {
        let old = recorded_certificate(|t| t.replace(from, to));
        let verdict = schema::conforms(&old, current);
        assert!(
            verdict.as_ref().is_err_and(|e| e.contains(reason)),
            "{from} -> {to}: {verdict:?}"
        );
    }
}

#[test]
fn a_reader_accepts_exactly_the_conforming_recorded_identities() {
    let current = schema::section(CENSUS_CERTIFICATE).unwrap();
    let older =
        schema::definition_text(current).replace("record 2 blocker\nfield 1 text 9 required\n", "");
    let retagged = schema::definition_text(current).replace("field 2 state 9", "field 5 state 9");
    let history = format!("generation OLD\n{older}generation BAD\n{retagged}");
    let ids = schema::accepted_schema_ids(&history, CENSUS_CERTIFICATE);
    let recorded = schema::recorded_generations(&history);
    assert_eq!(ids[0], schema_id(CENSUS_CERTIFICATE));
    assert!(
        ids.contains(&recorded[0].1[0].schema_id()),
        "conforming older definition"
    );
    assert!(
        !ids.contains(&recorded[1].1[0].schema_id()),
        "retagged definition"
    );
    assert_eq!(ids.len(), 2);
}

#[test]
fn a_container_an_older_conforming_writer_produced_is_read_with_its_history() {
    // An older definition of the certificate section (without the blocker record kind) conforms
    // to the current one: a container stamped with its identity is read when the history records
    // it, and refused when it does not.
    let current = schema::section(CENSUS_CERTIFICATE).unwrap();
    let older =
        schema::definition_text(current).replace("record 2 blocker\nfield 1 text 9 required\n", "");
    let history = format!("{}generation OLDER\n{older}", schema::HISTORY);
    let older_id = schema::recorded_generations(&history)
        .pop()
        .unwrap()
        .1
        .pop()
        .unwrap()
        .schema_id();
    let mut atlas = sample();
    atlas.certificate.blockers.clear();
    atlas.canonicalize();
    let bytes = encode(&atlas, |section| {
        if section == CENSUS_CERTIFICATE {
            older_id
        } else {
            schema_id(section)
        }
    });
    let (decoded, _) = read_with_history(&bytes, &history).unwrap();
    assert_eq!(decoded, atlas);
    assert!(
        read(&bytes)
            .unwrap_err()
            .to_string()
            .contains("unknown schema id"),
        "the compiled-in history does not record it"
    );
}
