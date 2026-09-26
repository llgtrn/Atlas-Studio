use super::rust::emit;
use super::*;

fn input(kind: ConstructionInputKind, reference: &str) -> ConstructionInput {
    ConstructionInput {
        kind,
        reference: reference.into(),
    }
}

fn observed_enum() -> IrType {
    IrType {
        id: "type:Mode".into(),
        name: "Mode".into(),
        kind: Some(TypeKind::Enum),
        visibility: Some("pub".into()),
        documentation: Some("How much is fetched.".into()),
        derives: Some(vec!["Debug".into(), "Clone".into(), "Serialize".into()]),
        attributes: Some(vec!["serde(rename_all = \"SCREAMING_SNAKE_CASE\")".into()]),
        variants: vec![
            IrVariant {
                name: "Remote".into(),
                documentation: None,
                attributes: Some(vec![]),
                lineage: vec!["r:variant-remote".into()],
            },
            IrVariant {
                name: "Local".into(),
                documentation: Some("On disk.".into()),
                attributes: Some(vec![]),
                lineage: vec!["r:variant-local".into()],
            },
        ],
        lineage: vec!["r:type".into()],
    }
}

fn function() -> IrFunction {
    IrFunction {
        id: "fn:Mode::is_local".into(),
        name: "is_local".into(),
        owner: Some("Mode".into()),
        dispatch: Dispatch::InherentMethod,
        visibility: "pub".into(),
        documentation: None,
        params: vec![IrParam {
            name: "self".into(),
            type_spelling: "Self".into(),
        }],
        result: Some("bool".into()),
        body_fingerprint: Some("blake3-256:00".into()),
        lineage: vec!["r:fn".into()],
    }
}

fn module() -> ConstructionModule {
    let mut module = ConstructionModule {
        schema: CONSTRUCTION_IR_SCHEMA.into(),
        module_id: String::new(),
        target: "core/src/mode.rs::Mode".into(),
        construction_target: BOOTSTRAP_CONSTRUCTION_TARGET.into(),
        container_root: "blake3-256:container".into(),
        inputs: ["r:type", "r:variant-remote", "r:variant-local", "r:fn"]
            .into_iter()
            .map(|r| input(ConstructionInputKind::CensusRecord, r))
            .chain([input(ConstructionInputKind::Design, "design:1")])
            .collect(),
        types: vec![observed_enum()],
        functions: vec![function()],
        gaps: vec![ConstructionGap {
            kind: GapKind::BodyUnobserved,
            subject: "fn:Mode::is_local".into(),
            missing: "lowerable body".into(),
            debt: "DEBT-CONSTRUCTION_IR".into(),
        }],
    };
    module.module_id = module_identity(&module);
    module
}

fn container() -> BTreeSet<String> {
    ["r:type", "r:variant-remote", "r:variant-local", "r:fn"]
        .into_iter()
        .map(String::from)
        .collect()
}

fn codes(violations: &[Violation]) -> Vec<&str> {
    violations.iter().map(|v| v.code.as_str()).collect()
}

fn reidentify(mut module: ConstructionModule) -> ConstructionModule {
    module.module_id = module_identity(&module);
    module
}

#[test]
fn a_module_built_only_from_container_records_and_a_design_is_valid() {
    assert_eq!(validate_module(&module(), &container()), vec![]);
}

#[test]
fn lineage_reaching_outside_the_declared_inputs_is_refused() {
    let mut m = module();
    m.types[0].variants[1].lineage = vec!["source:core/src/mode.rs".into()];
    let m = reidentify(m);
    assert!(codes(&validate_module(&m, &container())).contains(&"LINEAGE_OUTSIDE_INPUTS"));
}

#[test]
fn an_input_that_does_not_resolve_in_the_container_is_refused() {
    let mut m = module();
    m.inputs
        .push(input(ConstructionInputKind::CensusRecord, "r:elsewhere"));
    let m = reidentify(m);
    assert!(codes(&validate_module(&m, &container())).contains(&"INPUT_NOT_IN_CONTAINER"));
}

#[test]
fn construction_without_a_design_is_refused() {
    let mut m = module();
    m.inputs.retain(|i| i.kind != ConstructionInputKind::Design);
    let m = reidentify(m);
    assert!(codes(&validate_module(&m, &container())).contains(&"NO_DESIGN"));
}

#[test]
fn an_unobserved_element_without_its_gap_is_refused() {
    let mut m = module();
    m.types[0].derives = None;
    let m = reidentify(m);
    let found = validate_module(&m, &container());
    assert!(
        found
            .iter()
            .any(|v| v.code == "SILENT_GAP" && v.detail.contains("DERIVES_UNOBSERVED")),
        "{found:?}"
    );
    // Every function carries a body gap in v0.
    let mut m = module();
    m.gaps.clear();
    let m = reidentify(m);
    assert!(
        validate_module(&m, &container())
            .iter()
            .any(|v| v.detail.contains("BODY_UNOBSERVED"))
    );
}

#[test]
fn a_tampered_module_identity_is_refused() {
    let mut m = module();
    m.types[0].name = "Other".into();
    assert!(codes(&validate_module(&m, &container())).contains(&"MODULE_ID"));
}

#[test]
fn the_backend_emits_an_observed_enum_deterministically_and_never_a_body() {
    let m = module();
    let first = emit(&m);
    assert_eq!(first, emit(&m));
    assert_eq!(first.emitted, vec!["type:Mode".to_string()]);
    assert!(first.omitted[0].starts_with("fn:Mode::is_local"));
    let source = &first.source;
    assert!(source.contains("use serde::{Serialize};"), "{source}");
    assert!(source.contains("#[derive(Debug, Clone, Serialize)]"));
    assert!(source.contains("#[serde(rename_all = \"SCREAMING_SNAKE_CASE\")]"));
    // Declaration order is kept: it is what a derived Ord compares.
    let remote = source.find("    Remote,").unwrap();
    let local = source.find("    Local,").unwrap();
    assert!(remote < local);
    assert!(source.contains("    /// On disk.\n    Local,"));
    assert!(!source.contains("fn is_local"));
}

#[test]
fn the_backend_never_guesses_what_atlas_did_not_observe() {
    for strip in 0..4 {
        let mut m = module();
        let ty = &mut m.types[0];
        match strip {
            0 => ty.kind = None,
            1 => ty.derives = None,
            2 => ty.attributes = None,
            _ => ty.visibility = None,
        }
        let out = emit(&m);
        assert!(out.emitted.is_empty(), "strip {strip}: {out:?}");
        assert!(!out.source.contains("enum Mode"));
        assert!(out.omitted[0].contains("unobserved"));
    }
    let mut m = module();
    m.types[0].variants[0].attributes = None;
    assert!(emit(&m).emitted.is_empty());
    let mut m = module();
    m.types[0].derives = Some(vec!["Arbitrary".into()]);
    let out = emit(&m);
    assert!(out.emitted.is_empty() && out.omitted[0].contains("Arbitrary"));
}

fn report(module: &ConstructionModule) -> SelfReconstructionReport {
    let mut r = SelfReconstructionReport {
        schema: RECONSTRUCTION_REPORT_SCHEMA.into(),
        report_id: String::new(),
        level: SelfHostingLevel::Sh1,
        target: module.target.clone(),
        construction_target: BOOTSTRAP_CONSTRUCTION_TARGET.into(),
        module_id: module.module_id.clone(),
        design_ref: "design:1".into(),
        design_state: crate::design::DesignState::Validated,
        comparison_ref: None,
        construction_inputs: module.inputs.clone(),
        oracle_uses: vec![OracleUse {
            kind: OracleKind::OracleExecution,
            reference: "crate:atlas_core::mode::Mode".into(),
            purpose: "behavioral differential".into(),
        }],
        shadow: Some(ShadowArtifact {
            path: ".atlas/.cache/shadow/mode/src/lib.rs".into(),
            content_hash: "blake3-256:11".into(),
            backend: rust::RUST_BACKEND.into(),
            toolchain: "rustc 1.90.0".into(),
            emitted: vec!["type:Mode".into()],
            omitted: vec![],
        }),
        checks: vec![
            EquivalenceCheck {
                kind: EquivalenceKind::Behavioral,
                subject: "type:Mode".into(),
                property: "wire".into(),
                result: CheckResult::Equivalent,
                detail: String::new(),
            },
            EquivalenceCheck {
                kind: EquivalenceKind::Semantic,
                subject: "type:Mode".into(),
                property: "variants".into(),
                result: CheckResult::Equivalent,
                detail: String::new(),
            },
        ],
        gaps: module.gaps.clone(),
        declared_variations: vec![],
        verdict: ReconstructionVerdict::ConstructionGap,
    };
    r.report_id = report_identity(&r);
    r
}

fn reissue(mut r: SelfReconstructionReport) -> SelfReconstructionReport {
    r.report_id = report_identity(&r);
    r
}

#[test]
fn a_partial_reconstruction_is_a_construction_gap_never_equivalent() {
    let m = module();
    let r = report(&m);
    assert_eq!(decide(&r), ReconstructionVerdict::ConstructionGap);
    assert_eq!(validate_report(&r, &m), vec![]);
    let mut claimed = r.clone();
    claimed.verdict = ReconstructionVerdict::ReconstructedEquivalent;
    assert!(codes(&validate_report(&reissue(claimed), &m)).contains(&"VERDICT"));
}

#[test]
fn equivalence_needs_no_gap_and_both_kinds_of_evidence() {
    let mut m = module();
    m.functions.clear();
    m.gaps.clear();
    let m = reidentify(m);
    let r = report(&m);
    assert_eq!(decide(&r), ReconstructionVerdict::ReconstructedEquivalent);
    let mut varied = r.clone();
    varied.declared_variations = vec!["doc comments dropped".into()];
    assert_eq!(
        decide(&varied),
        ReconstructionVerdict::ReconstructedWithDeclaredVariation
    );
    for kind in [EquivalenceKind::Behavioral, EquivalenceKind::Semantic] {
        let mut half = r.clone();
        half.checks.retain(|c| c.kind != kind);
        assert_eq!(decide(&half), ReconstructionVerdict::VerificationFailed);
    }
}

#[test]
fn a_mismatch_outranks_a_gap_and_semantic_outranks_behavioral() {
    let m = module();
    let mut r = report(&m);
    r.checks[0].result = CheckResult::Mismatch;
    assert_eq!(decide(&r), ReconstructionVerdict::VerificationFailed);
    r.checks[1].result = CheckResult::Mismatch;
    assert_eq!(decide(&r), ReconstructionVerdict::SemanticMismatch);
}

#[test]
fn nothing_built_is_a_gap_when_gaps_explain_it_and_unsupported_otherwise() {
    let m = module();
    let mut r = report(&m);
    r.shadow = None;
    assert_eq!(decide(&r), ReconstructionVerdict::ConstructionGap);
    r.gaps.clear();
    assert_eq!(decide(&r), ReconstructionVerdict::Unsupported);
}

#[test]
fn an_oracle_is_never_a_construction_input_and_a_shadow_is_never_unverified() {
    let m = module();
    let mut r = report(&m);
    r.oracle_uses[0].reference = "r:type".into();
    assert!(codes(&validate_report(&reissue(r), &m)).contains(&"ORACLE_AS_INPUT"));
    let mut r = report(&m);
    r.oracle_uses.clear();
    assert!(codes(&validate_report(&reissue(r), &m)).contains(&"UNVERIFIED"));
    let mut r = report(&m);
    r.gaps.clear();
    let found = codes(&validate_report(&reissue(r), &m))
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
    assert!(found.contains(&"GAPS_MISMATCH".to_string()), "{found:?}");
}
