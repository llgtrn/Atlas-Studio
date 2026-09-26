use super::*;

fn tensor(name: &str, dtype: TensorDtype, shape: &[u64], start: u64, end: u64) -> PhysicalTensor {
    PhysicalTensor {
        name: name.into(),
        dtype,
        shape: shape.to_vec(),
        payload: PayloadRange { start, end },
        digest: IntegrityDigest::of_bytes(name.as_bytes())
            .as_str()
            .to_owned(),
    }
}

fn census(tensors: Vec<PhysicalTensor>, payload_bytes: u64) -> WeightCensus {
    WeightCensus {
        schema: WEIGHT_CENSUS_SCHEMA.into(),
        format: WeightFormat::SafeTensors,
        artifact_digest: IntegrityDigest::of_bytes(b"artifact").as_str().to_owned(),
        artifact_bytes: 16 + payload_bytes,
        payload_offset: 16,
        payload_bytes,
        metadata: BTreeMap::new(),
        tensors,
        roles: EpistemicStatus::Unknown,
        roles_basis: "fixture".into(),
    }
}

#[test]
fn a_census_whose_ranges_tile_the_payload_validates() {
    let c = census(
        vec![
            tensor("a", TensorDtype::F32, &[2, 3], 0, 24),
            tensor("b", TensorDtype::BF16, &[4], 24, 32),
            tensor("scalar", TensorDtype::U8, &[], 32, 33),
        ],
        33,
    );
    assert_eq!(validate(&c), Vec::<String>::new());
    assert_eq!(census_identity(&c), census_identity(&c.clone()));
}

#[test]
fn every_storage_inconsistency_is_named() {
    let problems = |c: &WeightCensus| validate(c).join(" | ");
    let base = || tensor("a", TensorDtype::F32, &[2], 0, 8);
    // dtype x shape disagrees with the range.
    let wrong = census(vec![tensor("a", TensorDtype::F32, &[3], 0, 8)], 8);
    assert!(problems(&wrong).contains("holds not the 12 bytes"));
    // An element count that overflows u64.
    let huge = census(vec![tensor("a", TensorDtype::F64, &[u64::MAX, 2], 0, 8)], 8);
    assert!(problems(&huge).contains("element count overflows"));
    // Overlap and hole.
    let overlap = census(vec![base(), tensor("b", TensorDtype::F32, &[2], 4, 12)], 12);
    assert!(problems(&overlap).contains("overlap or leave a hole"));
    let hole = census(vec![base(), tensor("b", TensorDtype::F32, &[2], 9, 17)], 17);
    assert!(problems(&hole).contains("overlap or leave a hole"));
    // A range outside the payload, and a payload the tensors do not cover.
    assert!(problems(&census(vec![base()], 4)).contains("cover 8 of the 4"));
    assert!(problems(&census(vec![base()], 9)).contains("cover 8 of the 9"));
    // A duplicate name, a malformed digest, a claimed role.
    let twice = census(vec![base(), tensor("a", TensorDtype::F32, &[2], 8, 16)], 16);
    assert!(problems(&twice).contains("appears twice"));
    let mut bad_digest = census(vec![base()], 8);
    bad_digest.tensors[0].digest = "blake3-256:no".into();
    assert!(problems(&bad_digest).contains("malformed digest"));
    let mut claimed = census(vec![base()], 8);
    claimed.roles = EpistemicStatus::Derived;
    assert!(problems(&claimed).contains("semantic roles"));
    let mut untiled = census(vec![base()], 8);
    untiled.artifact_bytes += 1;
    assert!(problems(&untiled).contains("do not tile"));
}

#[test]
fn exact_reconstruction_is_claimable_only_where_information_survives() {
    assert!(PreservationClass::Lossless.exact_reconstruction_claimable(false));
    assert!(PreservationClass::ReversibleWithRetained.exact_reconstruction_claimable(true));
    assert!(!PreservationClass::ReversibleWithRetained.exact_reconstruction_claimable(false));
    assert!(!PreservationClass::Lossy.exact_reconstruction_claimable(true));
    assert_eq!(expected_bytes(TensorDtype::BF16, &[3, 5]), Ok(30));
    assert_eq!(
        expected_bytes(TensorDtype::U8, &[]),
        Ok(1),
        "a scalar holds one element"
    );
    assert_eq!(
        expected_bytes(TensorDtype::I64, &[u64::MAX]),
        Err(SizeError::Overflow)
    );
    // An element count that wraps to zero must not pass as an empty tensor.
    assert_eq!(
        expected_bytes(TensorDtype::U8, &[1 << 32, 1 << 32]),
        Err(SizeError::Overflow)
    );
}

/// G166 (replay R9): the format's whole element-type table; sub-byte elements pack across
/// bytes and must fill whole bytes, as the reference reader requires.
#[test]
fn sub_byte_and_wide_dtypes_are_sized_in_bits() {
    // The reference reader's table at the replayed pin, in its order.
    let names = [
        "BOOL",
        "F4",
        "F6_E2M3",
        "F6_E3M2",
        "U8",
        "I8",
        "F8_E5M2",
        "F8_E4M3",
        "F8_E8M0",
        "F8_E4M3FNUZ",
        "F8_E5M2FNUZ",
        "I16",
        "U16",
        "F16",
        "BF16",
        "I32",
        "U32",
        "F32",
        "C64",
        "F64",
        "I64",
        "U64",
    ];
    let bits: Vec<(&str, u64)> = names
        .iter()
        .map(|name| {
            let dtype: TensorDtype = serde_json::from_value(serde_json::json!(name))
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(dtype.as_str(), *name);
            (*name, dtype.element_bits())
        })
        .collect();
    for (name, expected) in [
        ("F4", 4),
        ("F6_E2M3", 6),
        ("F6_E3M2", 6),
        ("F8_E8M0", 8),
        ("F8_E4M3FNUZ", 8),
        ("F8_E5M2FNUZ", 8),
        ("C64", 64),
        ("BOOL", 8),
        ("BF16", 16),
    ] {
        assert!(bits.contains(&(name, expected)), "{name}");
    }
    assert_eq!(expected_bytes(TensorDtype::F4, &[2, 4]), Ok(4));
    assert_eq!(expected_bytes(TensorDtype::F6E2M3, &[4]), Ok(3));
    assert_eq!(expected_bytes(TensorDtype::C64, &[3]), Ok(24));
    assert_eq!(
        expected_bytes(TensorDtype::F4, &[3]),
        Err(SizeError::Misaligned { bits: 12 })
    );
    assert_eq!(
        expected_bytes(TensorDtype::F6E3M2, &[]),
        Err(SizeError::Misaligned { bits: 6 }),
        "a sub-byte scalar fills no byte"
    );
    assert_eq!(expected_bytes(TensorDtype::F4, &[0]), Ok(0));
}
