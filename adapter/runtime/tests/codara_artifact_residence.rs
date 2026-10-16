//! Public caller compatibility for the source-local Codara artifact route.
//! These checks preserve legacy reports and identities; they confer no target admission.

use runtime::{atlasx as historical, codara::artifact as codara};
use serde_json::Value;
use std::{
    any::type_name,
    fs,
    path::{Path, PathBuf},
};

const REPOSITORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const PARITY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../core/src/codara/artifact/codec/parity"
);

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "atlas-runtime-public-artifact-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn both_public_routes_preserve_all_seven_complete_named_reports() {
    let oracle: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../core/src/codara/artifact/codec/parity/full-evidence.json"
    )))
    .unwrap();
    assert_eq!(
        oracle["test_only_oracle"],
        "atlas.functions-full-evidence.v1"
    );
    let cases = oracle["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 7);
    let scratch = Scratch::new("named-reports");
    for case in cases {
        let label = case["case"].as_str().unwrap();
        let fixture = Path::new(PARITY).join(case["fixture"].as_str().unwrap());
        let bytes = fs::read(&fixture).unwrap();
        assert_eq!(bytes.len() as u64, case["input_bytes"].as_u64().unwrap());
        let dir = scratch.0.join(label);
        fs::create_dir(&dir).unwrap();
        let object = dir.join(case["file_name"].as_str().unwrap());
        fs::write(&object, &bytes).unwrap();

        // Cross-route result annotations also exercise existing downstream type signatures.
        let old_result: codara::CodecReport = historical::decode_object(&object).unwrap();
        let new_result: historical::CodecReport = codara::decode_object(&object).unwrap();
        assert_eq!(old_result, new_result, "{label}: public routes diverged");
        assert_eq!(
            serde_json::to_value(&new_result).unwrap(),
            case["source_report"],
            "{label}: ordered defects, checked evidence or refusal changed"
        );
        assert_eq!(
            serde_json::to_vec_pretty(&old_result).unwrap(),
            serde_json::to_vec_pretty(&new_result).unwrap(),
            "{label}: serialized report bytes diverged"
        );
        assert_eq!(fs::read(&object).unwrap(), bytes, "{label}: input changed");
        assert_eq!(
            fs::read(&fixture).unwrap(),
            bytes,
            "{label}: fixture changed"
        );
    }
}

#[test]
fn admission_callers_share_declared_inputs_results_and_historical_type_identity() {
    assert_eq!(
        type_name::<codara::ParentFiles<'_>>(),
        "runtime::atlasx::ParentFiles"
    );
    assert_eq!(
        type_name::<codara::ParentFiles<'_>>(),
        type_name::<historical::ParentFiles<'_>>()
    );
    assert_eq!(
        type_name::<codara::Declared>(),
        type_name::<historical::Declared>()
    );

    // Read the repository's existing declarations rather than inventing an authority fixture.
    let declared: historical::Declared = codara::read_declared(REPOSITORY).unwrap();
    let declared: codara::Declared = declared;
    let scratch = Scratch::new("admission-inputs");
    let atlas = scratch.0.join("malformed.atlas");
    let verification = scratch.0.join("verification.json");
    let integrity = scratch.0.join("integrity.json");
    fs::write(&atlas, b"not a census container").unwrap();
    fs::write(&verification, b"{}").unwrap();
    fs::write(&integrity, b"{}").unwrap();
    let files = codara::ParentFiles {
        atlas: &atlas,
        design: None,
        verification: &verification,
        integrity: &integrity,
        declared: &declared,
    };
    let files: historical::ParentFiles<'_> = files;
    let old_result: codara::Precondition = historical::precondition(&files).unwrap();
    let new_result: historical::Precondition = codara::precondition(&files).unwrap();
    assert_eq!(old_result, new_result);
    assert_eq!(new_result.verdict, codara::PreconditionVerdict::Refused);
    assert!(new_result.admitted.is_none());
    assert!(!new_result.reasons.is_empty());
}

#[test]
fn both_routes_replay_the_same_encoded_object_and_payload_without_overwrite() {
    let scratch = Scratch::new("immutable-replay");
    let module = Path::new(REPOSITORY).join("core/src/codara/artifact/codec/fixture_module.json");
    let old_result: codara::Encoded = historical::encode_module(&module, &scratch.0).unwrap();
    let (encoded, object) = old_result.unwrap();
    let bytes = fs::read(&object).unwrap();
    let new_result: historical::Encoded = codara::encode_module(&module, &scratch.0).unwrap();
    let (replayed, replayed_path) = new_result.unwrap();
    assert_eq!(replayed, encoded);
    assert_eq!(replayed_path, object);
    assert_eq!(fs::read(&object).unwrap(), bytes);

    let payload = scratch.0.join("payload.bin");
    let old_report: codara::CodecReport =
        historical::export_codara_payload(&object, &payload).unwrap();
    let payload_bytes = fs::read(&payload).unwrap();
    let new_report: historical::CodecReport =
        codara::export_codara_payload(&object, &payload).unwrap();
    assert_eq!(old_report, new_report);
    assert_eq!(new_report.verdict, codara::CodecVerdict::Decoded);
    assert_eq!(payload_bytes.len() as u64, encoded.decoded_length);
    assert_eq!(fs::read(&payload).unwrap(), payload_bytes);

    let conflict = scratch.0.join("prior-evidence.bin");
    fs::write(&conflict, b"prior immutable evidence").unwrap();
    let old_error = historical::export_codara_payload(&object, &conflict).unwrap_err();
    let new_error = codara::export_codara_payload(&object, &conflict).unwrap_err();
    assert_eq!(old_error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(old_error.kind(), new_error.kind());
    assert_eq!(old_error.to_string(), new_error.to_string());
    assert_eq!(fs::read(&conflict).unwrap(), b"prior immutable evidence");
    assert_eq!(fs::read(&object).unwrap(), bytes);
}
