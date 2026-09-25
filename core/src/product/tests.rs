use super::*;
use crate::language::adl::SourceSpan;

fn node(kind: &str, name: &str, attributes: &[(&str, &str)]) -> DeclaredNode {
    DeclaredNode {
        id: format!("declared-node:{name}"),
        name: name.into(),
        node_kind: kind.into(),
        attributes: attributes
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect(),
        origin: "test".into(),
        span: SourceSpan {
            path: "test.adl".into(),
            line: 1,
            column: 1,
        },
    }
}

fn r(text: &str) -> Rational {
    Rational::parse_decimal(text).unwrap()
}

/// A minimal product: one part of 100 g of a 20..30 USD/kg material, priced 10..12 USD.
fn base() -> Vec<DeclaredNode> {
    vec![
        node(
            "Product",
            "P",
            &[("currency", "USD"), ("planned_volume", "100")],
        ),
        node("Part", "Body", &[]),
        node(
            "Material",
            "Resin",
            &[
                ("price", "20..30 USD/kg"),
                ("basis", "ESTIMATED"),
                ("source", "assumption"),
            ],
        ),
        node(
            "BomLine",
            "L1",
            &[("parent", "P"), ("item", "Body"), ("quantity", "1")],
        ),
        node(
            "BomLine",
            "L2",
            &[("parent", "Body"), ("item", "Resin"), ("quantity", "100 g")],
        ),
        node(
            "CostItem",
            "Setup",
            &[
                ("product", "P"),
                ("stage", "fixed"),
                ("amount", "100..200 USD"),
                ("basis", "ESTIMATED"),
                ("source", "assumption"),
            ],
        ),
        node(
            "Price",
            "Pr",
            &[
                ("product", "P"),
                ("amount", "10..12 USD"),
                ("basis", "HYPOTHESIZED"),
                ("source", "hypothesis"),
            ],
        ),
    ]
}

fn metric<'a>(v: &'a VariantEconomics, name: &str) -> Option<&'a Value> {
    v.metrics
        .iter()
        .find(|m| m.name == name)
        .unwrap()
        .value
        .as_ref()
}

#[test]
fn money_and_rates_parse_exactly_and_refuse_nonsense() {
    let (amount, currency, per) = parse_money("0.60..1.20 USD").unwrap();
    assert_eq!((amount.low, amount.high), (r("0.6"), r("1.2")));
    assert_eq!(currency, "USD");
    assert!(per.is_none());
    let (_, _, per) = parse_money("15 USD/kg").unwrap();
    assert_eq!(
        per.unwrap().dimension,
        Quantity::parse("1 kg").unwrap().dimension
    );
    for bad in ["12 usd", "12", "3..2 USD", "-1 USD", "1 USD/furlong"] {
        assert!(parse_money(bad).is_err(), "{bad}");
    }
    let rate = parse_rate("5..10%").unwrap();
    assert_eq!((rate.low, rate.high), (r("0.05"), r("0.1")));
    for bad in ["100%", "5", "10..5%", "-1%"] {
        assert!(parse_rate(bad).is_err(), "{bad}");
    }
}

#[test]
fn the_bom_explodes_exactly_through_parts_to_priced_leaves() {
    let report = analyze_products(&base());
    assert!(report.findings.is_empty(), "{:#?}", report.findings);
    let v = &report.variants[0];
    assert_eq!(
        v.variant, "P",
        "a product without variants is its own variant"
    );
    assert_eq!(v.bom.len(), 1);
    assert_eq!(v.bom[0].path, "P>Body>Resin");
    assert_eq!(v.bom[0].units, r("0.1"), "100 g of a per-kg price");
    let bom = metric(v, "bom_cost").unwrap();
    assert_eq!((bom.amount.low, bom.amount.high), (r("2"), r("3")));
    assert_eq!(bom.basis, EvidenceBasis::Estimated);
    // Amortization 100..200 / 100 units; the planned volume is a hypothesis.
    let amort = metric(v, "tooling_amortization").unwrap();
    assert_eq!((amort.amount.low, amort.amount.high), (r("1"), r("2")));
    assert_eq!(amort.basis, EvidenceBasis::Hypothesized);
    assert_eq!(v.weakest_basis, Some(EvidenceBasis::Hypothesized));
}

#[test]
fn an_estimate_without_declared_uncertainty_is_refused_and_unknown_propagates() {
    let mut nodes = base();
    nodes[2]
        .attributes
        .insert("price".into(), "25 USD/kg".into());
    let report = analyze_products(&nodes);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.subject == "Resin" && f.message.contains("declare its uncertainty"))
    );
    let v = &report.variants[0];
    for name in [
        "bom_cost",
        "factory_cost",
        "landed_cost",
        "gross_margin",
        "gross_margin_ratio",
    ] {
        assert!(
            metric(v, name).is_none(),
            "{name} must be UNKNOWN, never zero"
        );
    }
    assert!(v.break_even_units.is_none());
}

#[test]
fn a_quoted_value_must_cite_a_declared_quote() {
    let mut nodes = base();
    nodes[2]
        .attributes
        .insert("price".into(), "25 USD/kg".into());
    nodes[2].attributes.insert("basis".into(), "QUOTED".into());
    let refused = analyze_products(&nodes);
    assert!(
        refused
            .findings
            .iter()
            .any(|f| f.verdict == ConstraintVerdict::Violated
                && f.message.contains("supplier pricing is never assumed"))
    );
    nodes[2].attributes.insert("quote".into(), "Q1".into());
    nodes.push(node(
        "Quote",
        "Q1",
        &[("supplier", "S"), ("date", "2026-09-24")],
    ));
    let accepted = analyze_products(&nodes);
    assert!(accepted.findings.is_empty(), "{:#?}", accepted.findings);
    let bom = metric(&accepted.variants[0], "bom_cost").unwrap();
    assert_eq!(bom.amount, Interval::point(r("2.5")));
    assert_eq!(bom.basis, EvidenceBasis::Quoted);
}

#[test]
fn a_value_without_a_source_is_unknown() {
    let mut nodes = base();
    nodes[2].attributes.remove("source");
    let report = analyze_products(&nodes);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.message.contains("no `source`"))
    );
    assert!(metric(&report.variants[0], "bom_cost").is_none());
}

#[test]
fn quantities_are_dimension_checked_against_the_pricing_unit() {
    let mut nodes = base();
    nodes[4]
        .attributes
        .insert("quantity".into(), "100 mm".into());
    let report = analyze_products(&nodes);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.subject == "L2" && f.verdict == ConstraintVerdict::Violated)
    );
    assert!(metric(&report.variants[0], "bom_cost").is_none());
}

#[test]
fn bom_cycles_are_refused() {
    let mut nodes = base();
    nodes.push(node(
        "BomLine",
        "Loop",
        &[("parent", "Body"), ("item", "Body"), ("quantity", "1")],
    ));
    let report = analyze_products(&nodes);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.message.contains("BOM cycle"))
    );
    assert!(metric(&report.variants[0], "bom_cost").is_none());
}

#[test]
fn variant_scoped_lines_apply_only_to_their_variant() {
    let mut nodes = base();
    nodes.push(node("Variant", "A", &[("product", "P")]));
    nodes.push(node("Variant", "B", &[("product", "P")]));
    nodes[4].attributes.insert("variants".into(), "A".into());
    nodes.push(node(
        "BomLine",
        "L3",
        &[
            ("parent", "Body"),
            ("item", "Resin"),
            ("quantity", "200 g"),
            ("variants", "B"),
        ],
    ));
    let report = analyze_products(&nodes);
    let cost = |name: &str| {
        let v = report.variants.iter().find(|v| v.variant == name).unwrap();
        metric(v, "bom_cost").unwrap().amount
    };
    assert_eq!(
        cost("A"),
        Interval {
            low: r("2"),
            high: r("3")
        }
    );
    assert_eq!(
        cost("B"),
        Interval {
            low: r("4"),
            high: r("6")
        }
    );
}

#[test]
fn margins_use_dependency_aware_forms_and_stay_within_bounds() {
    let report = analyze_products(&base());
    let v = &report.variants[0];
    // landed = bom 2..3 + amort 1..2 (no scrap, no landing items) = 3..5; price 10..12.
    let ratio = metric(v, "gross_margin_ratio").unwrap();
    assert_eq!(ratio.amount.low, r("0.5"), "1 - 5/10");
    assert_eq!(ratio.amount.high, r("0.75"), "1 - 3/12");
    let one = Rational::ONE;
    assert_ne!(
        ratio.amount.high.checked_cmp(one),
        Some(std::cmp::Ordering::Greater)
    );
    // contribution excludes amortized fixed costs: 10..12 - 2..3 = 7..10.
    let c = metric(v, "contribution_margin").unwrap();
    assert_eq!((c.amount.low, c.amount.high), (r("7"), r("10")));
    // break-even = fixed / contribution = 100/10 .. 200/7.
    let be = v.break_even_units.unwrap();
    assert_eq!(be.low, r("10"));
    assert_eq!(be.high, Rational::new(200, 7).unwrap());
}

#[test]
fn requirements_are_decided_over_the_whole_interval() {
    let requirement = |limit: &str, comparison: &str| {
        let mut nodes = base();
        nodes.push(node(
            "ProductRequirement",
            "R",
            &[
                ("product", "P"),
                ("metric", "landed_cost"),
                ("comparison", comparison),
                ("limit", limit),
            ],
        ));
        analyze_products(&nodes).variants[0].requirements[0].verdict
    };
    // landed_cost is 3..5 USD.
    assert_eq!(requirement("5 USD", "<="), ConstraintVerdict::Satisfied);
    assert_eq!(requirement("4 USD", "<="), ConstraintVerdict::Unknown);
    assert_eq!(requirement("2.99 USD", "<="), ConstraintVerdict::Violated);
    assert_eq!(requirement("3 USD", ">="), ConstraintVerdict::Satisfied);
    assert_eq!(requirement("5.01 USD", ">="), ConstraintVerdict::Violated);
    assert_eq!(
        requirement("4 EUR", "<="),
        ConstraintVerdict::Unknown,
        "no silent currency conversion"
    );
    assert_eq!(requirement("4 USD", "=="), ConstraintVerdict::Unknown);
}

#[test]
fn no_break_even_is_claimed_when_contribution_can_be_negative() {
    let mut nodes = base();
    nodes[6]
        .attributes
        .insert("amount".into(), "1..12 USD".into());
    let report = analyze_products(&nodes);
    assert!(report.variants[0].break_even_units.is_none());
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.message.contains("no break-even"))
    );
}

#[test]
fn market_demand_is_never_asserted_without_evidence() {
    let mut nodes = base();
    nodes.push(node(
        "ProductHypothesis",
        "H",
        &[("problem", "x"), ("evidence", "Survey1")],
    ));
    let report = analyze_products(&nodes);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.subject == "H" && f.verdict == ConstraintVerdict::Violated)
    );
    assert!(
        report.hypotheses[0]
            .market_uncertainty
            .starts_with("UNKNOWN")
    );
    assert_eq!(report.hypotheses[0].status, EpistemicStatus::Hypothesis);
    nodes.push(node("MarketEvidence", "Survey1", &[("kind", "survey")]));
    let cited = analyze_products(&nodes);
    assert_eq!(cited.hypotheses[0].evidence, ["Survey1"]);
    assert_eq!(
        cited.hypotheses[0].status,
        EpistemicStatus::Hypothesis,
        "citation alone never validates demand"
    );
}

#[test]
fn a_foreign_currency_is_never_converted_silently() {
    let mut nodes = base();
    nodes[2]
        .attributes
        .insert("price".into(), "20..30 EUR/kg".into());
    let report = analyze_products(&nodes);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.subject == "Resin" && f.message.contains("ever converted silently")),
        "{:#?}",
        report.findings
    );
    assert!(metric(&report.variants[0], "bom_cost").is_none());
    assert!(metric(&report.variants[0], "landed_cost").is_none());
}

#[test]
fn foreign_cost_items_and_prices_are_findings_and_sums_never_mix_currencies() {
    let mut nodes = base();
    nodes[5]
        .attributes
        .insert("amount".into(), "100..200 EUR".into());
    nodes[6]
        .attributes
        .insert("amount".into(), "10..12 GBP".into());
    let report = analyze_products(&nodes);
    for subject in ["Setup", "Pr"] {
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.subject == subject && f.message.contains("ever converted silently")),
            "{subject}: {:#?}",
            report.findings
        );
    }
    let v = &report.variants[0];
    assert!(metric(v, "fixed_costs").is_none());
    assert!(metric(v, "price").is_none());
    // The single enforcement point every derived sum goes through.
    let value = |currency: &str| Value {
        currency: currency.into(),
        amount: Interval::point(r("1")),
        basis: EvidenceBasis::Estimated,
        source: "t".into(),
    };
    assert!(sum(&[&value("USD"), &value("EUR")], "USD", "t").is_none());
    assert_eq!(
        sum(&[&value("USD"), &value("USD")], "USD", "t")
            .unwrap()
            .amount,
        Interval::point(r("2"))
    );
}
