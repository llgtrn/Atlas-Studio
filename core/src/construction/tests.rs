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
        body: None,
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

// --- G181 (M14, ADR 0094): lowered bodies ------------------------------------------------------

fn hir(
    id: usize,
    kind: HirNodeKind,
    result_type: &str,
    value: Option<&str>,
    operands: Vec<usize>,
) -> HirNode {
    HirNode {
        id,
        kind,
        result_type: result_type.into(),
        value: value.map(String::from),
        intrinsic: (kind == HirNodeKind::Intrinsic).then_some(HirIntrinsic::Ne),
        operands,
        arms: vec![],
        lineage: vec!["r:fn".into()],
    }
}

/// `self != Self::Remote` over a `Copy + PartialEq` Mode, and no body gap.
fn bodied() -> ConstructionModule {
    let mut m = module();
    m.types[0].derives = Some(vec![
        "Debug".into(),
        "Clone".into(),
        "Copy".into(),
        "PartialEq".into(),
        "Serialize".into(),
    ]);
    m.functions[0].body = Some(HirBody {
        root: 2,
        nodes: vec![
            hir(0, HirNodeKind::Copy, "type:Mode", Some("self"), vec![]),
            hir(
                1,
                HirNodeKind::Const,
                "type:Mode",
                Some("type:Mode::Remote"),
                vec![],
            ),
            hir(2, HirNodeKind::Intrinsic, "bool", None, vec![0, 1]),
        ],
    });
    m.gaps.clear();
    reidentify(m)
}

fn body_codes(m: ConstructionModule) -> Vec<String> {
    validate_module(&reidentify(m), &container())
        .into_iter()
        .map(|v| v.code)
        .collect()
}

#[test]
fn a_typed_resolved_body_is_valid_and_emitted_inside_an_impl() {
    let m = bodied();
    assert_eq!(validate_module(&m, &container()), vec![]);
    let out = emit(&m);
    assert_eq!(out.emitted, ["type:Mode", "fn:Mode::is_local"]);
    assert!(out.omitted.is_empty(), "{:?}", out.omitted);
    assert!(
        out.source.contains(
            "\nimpl Mode {\n    pub fn is_local(self) -> bool {\n        self != Self::Remote\n    }\n}\n"
        ),
        "{}",
        out.source
    );
    assert_eq!(out, emit(&m), "deterministic");
}

#[test]
fn a_body_and_its_gap_exclude_each_other() {
    let mut m = bodied();
    m.gaps = module().gaps;
    assert!(body_codes(m).contains(&"BODY_GAP_CONTRADICTION".to_string()));
    let mut m = bodied();
    m.functions[0].body = None;
    assert!(body_codes(m).contains(&"SILENT_GAP".to_string()));
}

#[test]
fn malformed_untyped_or_unresolved_bodies_are_refused_with_their_defect() {
    type Mutation = fn(&mut ConstructionModule);
    let cases: Vec<(&str, Mutation)> = vec![
        ("BODY_DANGLING_CHILD", |m| {
            m.functions[0].body.as_mut().unwrap().nodes[2].operands[1] = 7
        }),
        ("BODY_DANGLING_CHILD", |m| {
            m.functions[0].body.as_mut().unwrap().nodes[2].operands[1] = 2
        }),
        ("BODY_DANGLING_CHILD", |m| {
            m.functions[0].body.as_mut().unwrap().root = 9
        }),
        ("BODY_UNTYPED_NODE", |m| {
            m.functions[0].body.as_mut().unwrap().nodes[1].result_type = "type:Other".into()
        }),
        ("BODY_UNRESOLVED_REFERENCE", |m| {
            m.functions[0].body.as_mut().unwrap().nodes[1].value = Some("type:Mode::Gone".into())
        }),
        ("BODY_UNRESOLVED_REFERENCE", |m| {
            m.functions[0].body.as_mut().unwrap().nodes[0].value = Some("other".into())
        }),
        ("BODY_MALFORMED_NODE", |m| {
            m.functions[0].body.as_mut().unwrap().nodes[2].intrinsic = None
        }),
        ("BODY_MALFORMED_NODE", |m| {
            m.functions[0].body.as_mut().unwrap().nodes[1].id = 5
        }),
        ("BODY_CAPABILITY_UNOBSERVED", |m| {
            m.types[0]
                .derives
                .as_mut()
                .unwrap()
                .retain(|d| d != "PartialEq")
        }),
        ("BODY_CAPABILITY_UNOBSERVED", |m| {
            m.types[0].derives.as_mut().unwrap().retain(|d| d != "Copy")
        }),
        ("BODY_TYPE_MISMATCH", |m| {
            m.functions[0].result = Some("u8".into())
        }),
        ("BODY_TYPE_MISMATCH", |m| {
            let b = m.functions[0].body.as_mut().unwrap();
            b.nodes[1] = hir(1, HirNodeKind::Const, "bool", Some("true"), vec![]);
        }),
        ("LINEAGE_OUTSIDE_INPUTS", |m| {
            m.functions[0].body.as_mut().unwrap().nodes[1].lineage = vec!["source:mode.rs".into()]
        }),
    ];
    for (code, mutate) in cases {
        let mut m = bodied();
        mutate(&mut m);
        let found = body_codes(m);
        assert!(found.contains(&code.to_string()), "{code}: {found:?}");
    }
}

fn arm(patterns: &[&str], value: usize) -> HirArm {
    HirArm {
        patterns: patterns
            .iter()
            .map(|p| match *p {
                "_" => HirPattern::Wildcard,
                v => HirPattern::Variant(format!("type:Mode::{v}")),
            })
            .collect(),
        value,
    }
}

/// `match self { Self::Remote => false, _ => true }` with `arms`.
fn matching(arms: Vec<HirArm>) -> ConstructionModule {
    let mut m = bodied();
    let mut node = hir(3, HirNodeKind::Match, "bool", None, vec![0]);
    node.arms = arms;
    m.functions[0].body = Some(HirBody {
        root: 3,
        nodes: vec![
            hir(0, HirNodeKind::Copy, "type:Mode", Some("self"), vec![]),
            hir(1, HirNodeKind::Const, "bool", Some("false"), vec![]),
            hir(2, HirNodeKind::Const, "bool", Some("true"), vec![]),
            node,
        ],
    });
    reidentify(m)
}

#[test]
fn a_match_must_cover_every_variant_and_emits_its_arms_in_order() {
    let m = matching(vec![arm(&["Remote"], 1), arm(&["_"], 2)]);
    assert_eq!(validate_module(&m, &container()), vec![]);
    let source = emit(&m).source;
    assert!(
        source.contains("        match self {\n            Self::Remote => false,\n            _ => true,\n        }\n"),
        "{source}"
    );
    let all = matching(vec![arm(&["Remote"], 1), arm(&["Local"], 2)]);
    assert_eq!(validate_module(&all, &container()), vec![]);
    let alternatives = matching(vec![arm(&["Remote", "Local"], 1)]);
    assert!(
        emit(&alternatives)
            .source
            .contains("Self::Remote | Self::Local => false,")
    );
    let partial = matching(vec![arm(&["Remote"], 1)]);
    assert!(body_codes(partial).contains(&"BODY_NON_EXHAUSTIVE_MATCH".to_string()));
    let stray = matching(vec![
        arm(&["Remote"], 1),
        arm(&["Elsewhere"], 2),
        arm(&["_"], 2),
    ]);
    assert!(body_codes(stray).contains(&"BODY_UNRESOLVED_REFERENCE".to_string()));
    let mixed = matching(vec![arm(&["Remote"], 1), arm(&["_"], 0)]);
    assert!(body_codes(mixed).contains(&"BODY_TYPE_MISMATCH".to_string()));
}

#[test]
fn the_backend_emits_a_method_only_with_its_body_its_owner_and_spellable_parameters() {
    let mut m = bodied();
    m.types[0].visibility = None;
    let out = emit(&m);
    assert!(out.emitted.is_empty(), "{out:?}");
    assert!(
        out.omitted
            .iter()
            .any(|o| o.contains("owner type is not emitted"))
    );
    assert!(!out.source.contains("fn is_local"));
    let mut m = bodied();
    m.functions[0].params.push(IrParam {
        name: "x".into(),
        type_spelling: "u8".into(),
    });
    let out = emit(&m);
    assert_eq!(out.emitted, ["type:Mode"]);
    assert!(out.omitted[0].contains("parameters other than `self`"));
    let out = emit(&module());
    assert!(out.omitted[0].contains("no lowerable body"), "{out:?}");
}
