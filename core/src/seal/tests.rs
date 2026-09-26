use super::*;

const DIMENSIONS: [&str; 12] = [
    "SYMBOL",
    "TYPE",
    "FUNCTION_IDENTITY",
    "FUNCTION_SIGNATURE",
    "CALL",
    "CONTROL_FLOW",
    "DATA_FLOW",
    "STATE",
    "EFFECT",
    "OWNERSHIP",
    "CONCURRENCY",
    "PERSISTENCE",
];

/// Every kind REQUIRED_CLEAR, the dynamic obligations an enumerated count, the unsealed root out
/// of scope; every dimension HARD.
fn strict() -> SealPolicy {
    let mut rules: BTreeMap<BlockerKind, Disposition> = BlockerKind::ALL
        .into_iter()
        .map(|kind| (kind, Disposition::RequiredClear))
        .collect();
    rules.insert(
        BlockerKind::DynamicDependencyObligations,
        Disposition::AcceptedBoundedResidual {
            bound: ResidualBound::AtMost(9),
            reason: "the nine dynamic dependency classes".into(),
        },
    );
    rules.insert(
        BlockerKind::AtlasRootUnsealed,
        Disposition::OutOfScope {
            reason: "the seal is what seals the root".into(),
        },
    );
    let mut policy = SealPolicy {
        schema: SEAL_POLICY_SCHEMA_VERSION.into(),
        policy_id: String::new(),
        scope: "atlas-self".into(),
        verification: VerificationPolicy::library_package(),
        certificate: ScopePolicy {
            scope: "atlas-self".into(),
            rules,
        },
        hard_dimensions: DIMENSIONS.map(str::to_owned).to_vec(),
        integrity_verdict_required: "ELIGIBLE".into(),
    };
    policy.policy_id = policy_identity(&policy);
    policy
}

fn restamp(mut policy: SealPolicy) -> SealPolicy {
    policy.policy_id = policy_identity(&policy);
    policy
}

#[test]
fn every_certificate_blocker_code_is_typed_and_an_unknown_one_is_refused() {
    for kind in BlockerKind::ALL {
        let text = format!("{}: detail", kind.as_str());
        assert_eq!(parse_blocker(&text), Ok((kind, "detail".to_owned())));
    }
    assert_eq!(
        parse_blocker("SEMANTIC_OBLIGATIONS_NOT_ACCOUNTED"),
        Ok((BlockerKind::SemanticObligationsNotAccounted, String::new()))
    );
    assert!(
        parse_blocker("SOMETHING_NEW: x")
            .unwrap_err()
            .contains("unknown blocker code")
    );
    // The vocabulary is exactly the certificate's: every code the certificate module writes.
    let source = include_str!("../certificate/mod.rs");
    for kind in BlockerKind::ALL {
        let code = kind.as_str();
        let written = source.contains(&format!("\"{code}")) || code.starts_with("COVERAGE_");
        assert!(written, "{code} is not a certificate blocker");
    }
}

#[test]
fn the_strict_policy_is_valid_and_no_provider_can_weaken_it_past_the_bounds() {
    let policy = strict();
    assert_eq!(validate_policy(&policy), Vec::<String>::new());
    let problems = |edit: &dyn Fn(&mut SealPolicy)| {
        let mut changed = policy.clone();
        edit(&mut changed);
        validate_policy(&restamp(changed))
    };
    for kind in BlockerKind::NEVER_PERMITTED {
        let found = problems(&|p| {
            p.certificate.rules.insert(
                kind,
                Disposition::OutOfScope {
                    reason: "convenient".into(),
                },
            );
        });
        assert_eq!(found, [format!("{kind} can never be permitted")]);
    }
    assert_eq!(
        problems(&|p| {
            p.certificate.rules.insert(
                BlockerKind::CoverageUnknown,
                Disposition::AcceptedBoundedResidual {
                    bound: ResidualBound::Enumerated(vec!["CALL".into()]),
                    reason: "not yet".into(),
                },
            );
        }),
        ["COVERAGE_UNKNOWN is permitted for a HARD dimension"]
    );
    // A dimension that is not HARD may be permitted.
    assert_eq!(
        problems(&|p| {
            p.hard_dimensions.retain(|d| d != "CALL");
            p.certificate.rules.insert(
                BlockerKind::CoverageUnknown,
                Disposition::AcceptedBoundedResidual {
                    bound: ResidualBound::Enumerated(vec!["CALL".into()]),
                    reason: "declared non-critical".into(),
                },
            );
        }),
        Vec::<String>::new()
    );
    assert_eq!(
        problems(&|p| {
            p.certificate.rules.remove(&BlockerKind::UnknownFacts);
        }),
        ["UNKNOWN_FACTS has no disposition"]
    );
    assert_eq!(
        problems(&|p| p.integrity_verdict_required = "INCOMPLETE".into()),
        ["a seal requires an ELIGIBLE integrity report"]
    );
    assert_eq!(
        problems(&|p| p.certificate.scope = "other".into()),
        ["the certificate policy is for another scope"]
    );
    // An edited policy whose identity was not restamped.
    let mut edited = policy.clone();
    edited.hard_dimensions.pop();
    assert!(validate_policy(&edited)[0].contains("is not the policy's identity"));
}

#[test]
fn a_certificate_is_eligible_only_when_every_blocker_is_permitted_and_permitted_ones_stay_listed() {
    let policy = strict();
    let verdict = evaluate_certificate(
        "census-certificate:x",
        &[
            "DYNAMIC_DEPENDENCY_OBLIGATIONS: 9".into(),
            "ATLAS_ROOT_UNSEALED: the packaged .atlas is an unsealed census container".into(),
        ],
        &policy,
    );
    assert_eq!(verdict.verdict, ScopeVerdict::Eligible);
    assert_eq!(verdict.refused, Vec::<String>::new());
    assert_eq!(
        verdict.permitted.len(),
        2,
        "permitted residuals are never dropped"
    );
    assert_eq!(verdict.policy_id, policy.policy_id);
    let refused = |blocker: &str| evaluate_certificate("c", &[blocker.to_owned()], &policy).refused;
    assert_eq!(
        refused("DYNAMIC_DEPENDENCY_OBLIGATIONS: 10"),
        ["DYNAMIC_DEPENDENCY_OBLIGATIONS: 10: beyond the permitted bound"]
    );
    assert_eq!(
        refused("DYNAMIC_DEPENDENCY_OBLIGATIONS: nine"),
        ["DYNAMIC_DEPENDENCY_OBLIGATIONS: nine: beyond the permitted bound"]
    );
    assert_eq!(
        refused("COVERAGE_UNKNOWN: CALL"),
        ["COVERAGE_UNKNOWN: CALL: required clear"]
    );
    assert!(refused("A_NEW_BLOCKER: x")[0].contains("unknown blocker code"));
    let mut partial = policy.clone();
    partial.certificate.rules.remove(&BlockerKind::UnknownFacts);
    assert!(
        evaluate_certificate("c", &["UNKNOWN_FACTS: 3".into()], &partial).refused[0]
            .contains("does not classify UNKNOWN_FACTS")
    );
    // An enumerated residual admits exactly its members.
    let mut enumerated = policy;
    enumerated.hard_dimensions.retain(|d| d != "CALL");
    enumerated.certificate.rules.insert(
        BlockerKind::CoverageUnknown,
        Disposition::AcceptedBoundedResidual {
            bound: ResidualBound::Enumerated(vec!["CALL".into()]),
            reason: "declared non-critical".into(),
        },
    );
    let enumerated = restamp(enumerated);
    let eval = |b: &str| evaluate_certificate("c", &[b.to_owned()], &enumerated).verdict;
    assert_eq!(eval("COVERAGE_UNKNOWN: CALL"), ScopeVerdict::Eligible);
    assert_eq!(eval("COVERAGE_UNKNOWN: STATE"), ScopeVerdict::NotEligible);
}

#[test]
fn the_policy_identity_covers_every_rule() {
    let policy = strict();
    let mut changed = policy.clone();
    changed.certificate.rules.insert(
        BlockerKind::DynamicDependencyObligations,
        Disposition::AcceptedBoundedResidual {
            bound: ResidualBound::AtMost(8),
            reason: "the nine dynamic dependency classes".into(),
        },
    );
    assert_ne!(policy_identity(&changed), policy.policy_id);
    let round: SealPolicy = serde_json::from_str(&serde_json::to_string(&policy).unwrap()).unwrap();
    assert_eq!(round, policy);
    assert_eq!(policy_identity(&round), policy.policy_id);
}
