//! Product Foundry, first slice (ADR 0023): a Product Semantic IR over declared ADL entities --
//! variant-aware BOM graph, exact money intervals with an evidence basis on every value, and the
//! unit-economics chain from component cost to break-even volume.
//!
//! Anti-hype by construction: every commercial number is an interval with a named basis
//! (HYPOTHESIZED .. PRODUCTION_MEASURED); an estimate without declared uncertainty is refused; a
//! QUOTED value must cite a declared `Quote`; a derived value carries the weakest basis of its
//! inputs; anything missing is UNKNOWN and propagates as UNKNOWN, never as zero.

use crate::{
    ConstraintVerdict, DeclaredNode,
    quantity::{Quantity, Rational, parse_unit_expression},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const PRODUCT_REPORT_SCHEMA: &str = "atlas.product-report.v1";

/// How strongly a commercial value is known, weakest first.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceBasis {
    /// A guess to be tested (e.g. a price hypothesis).
    Hypothesized,
    /// An estimate with declared uncertainty and a stated source/assumption.
    Estimated,
    /// A supplier quote on record (`Quote` entity).
    Quoted,
    Prototyped,
    Validated,
    ProductionMeasured,
}

impl EvidenceBasis {
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "HYPOTHESIZED" => Self::Hypothesized,
            "ESTIMATED" => Self::Estimated,
            "QUOTED" => Self::Quoted,
            "PROTOTYPED" => Self::Prototyped,
            "VALIDATED" => Self::Validated,
            "PRODUCTION_MEASURED" => Self::ProductionMeasured,
            _ => return None,
        })
    }

    /// Point values (no declared range) are admissible only from a quote or better.
    pub fn admits_point_value(self) -> bool {
        self >= Self::Quoted
    }
}

/// An exact closed interval `[low, high]` of money in one currency.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Interval {
    pub low: Rational,
    pub high: Rational,
}

impl Interval {
    pub fn point(value: Rational) -> Self {
        Self {
            low: value,
            high: value,
        }
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        Some(Self {
            low: self.low.checked_add(other.low)?,
            high: self.high.checked_add(other.high)?,
        })
    }

    pub fn checked_sub(self, other: Self) -> Option<Self> {
        Some(Self {
            low: self.low.checked_add(other.high.checked_neg()?)?,
            high: self.high.checked_add(other.low.checked_neg()?)?,
        })
    }

    /// Scales by a non-negative exact factor.
    pub fn scale(self, factor: Rational) -> Option<Self> {
        Some(Self {
            low: self.low.checked_mul(factor)?,
            high: self.high.checked_mul(factor)?,
        })
    }

    /// Product of two non-negative intervals.
    pub fn mul_nonnegative(self, other: Self) -> Option<Self> {
        Some(Self {
            low: self.low.checked_mul(other.low)?,
            high: self.high.checked_mul(other.high)?,
        })
    }

    pub fn is_nonnegative(self) -> bool {
        self.low.checked_cmp(Rational::ZERO) != Some(std::cmp::Ordering::Less)
    }
}

/// A commercial value: an interval in a currency, its basis, and where it came from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Value {
    pub currency: String,
    pub amount: Interval,
    pub basis: EvidenceBasis,
    /// Declared source/assumption, or the derivation that produced it.
    pub source: String,
}

/// Parses `<decimal>[..<decimal>] <CCY>[/<unit expression>]`, returning the interval, currency
/// and the per-quantity unit (`None` = per item).
pub fn parse_money(text: &str) -> Result<(Interval, String, Option<Quantity>), String> {
    let (number, rest) = text
        .trim()
        .split_once(char::is_whitespace)
        .ok_or_else(|| format!("`{text}`: expected `<amount> <CCY>`"))?;
    let (currency, per) = match rest.trim().split_once('/') {
        Some((c, unit)) => (c.trim(), Some(unit.trim())),
        None => (rest.trim(), None),
    };
    if currency.len() != 3 || !currency.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(format!("`{text}`: `{currency}` is not an ISO 4217 code"));
    }
    let decimal = |s: &str| {
        Rational::parse_decimal(s.trim()).ok_or_else(|| format!("`{text}`: `{s}` is not a decimal"))
    };
    let amount = match number.split_once("..") {
        Some((low, high)) => {
            let (low, high) = (decimal(low)?, decimal(high)?);
            if low.checked_cmp(high) == Some(std::cmp::Ordering::Greater) {
                return Err(format!("`{text}`: range low exceeds high"));
            }
            Interval { low, high }
        }
        None => Interval::point(decimal(number)?),
    };
    if !amount.is_nonnegative() {
        return Err(format!("`{text}`: negative amounts are not prices"));
    }
    let per = match per {
        None => None,
        Some(unit) => {
            let (factor, dimension) =
                parse_unit_expression(unit).map_err(|e| format!("`{text}`: {e}"))?;
            Some(Quantity::new(factor, dimension))
        }
    };
    Ok((amount, currency.to_owned(), per))
}

/// Parses `<decimal>[..<decimal>] %` into a fraction interval in `[0, 1)`.
pub fn parse_rate(text: &str) -> Result<Interval, String> {
    let number = text
        .trim()
        .strip_suffix('%')
        .ok_or_else(|| format!("`{text}`: expected a percentage"))?;
    let hundred = Rational::integer(100);
    let decimal = |s: &str| {
        Rational::parse_decimal(s.trim())
            .and_then(|r| r.checked_div(hundred))
            .ok_or_else(|| format!("`{text}`: `{s}` is not a decimal"))
    };
    let rate = match number.split_once("..") {
        Some((low, high)) => Interval {
            low: decimal(low)?,
            high: decimal(high)?,
        },
        None => Interval::point(decimal(number)?),
    };
    let one = Rational::ONE;
    if !rate.is_nonnegative()
        || rate.high.checked_cmp(one) != Some(std::cmp::Ordering::Less)
        || rate.low.checked_cmp(rate.high) == Some(std::cmp::Ordering::Greater)
    {
        return Err(format!(
            "`{text}`: a rate must lie in [0%, 100%) with low <= high"
        ));
    }
    Ok(rate)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductFinding {
    pub subject: String,
    pub verdict: ConstraintVerdict,
    pub message: String,
}

/// One exploded BOM leaf for one variant: how much of which priced item, at what cost.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BomLeaf {
    /// Path from the product root, e.g. `Organizer>Body>PLA`.
    pub path: String,
    pub item: String,
    /// Exact multiplier of the item's pricing unit.
    pub units: Rational,
    pub cost: Value,
}

/// A derived metric of one variant: `None` = UNKNOWN (an input was missing or unusable).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Metric {
    pub name: String,
    pub value: Option<Value>,
    /// Inputs (other metric names or entity names) this metric was derived from.
    pub inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequirementCheck {
    pub requirement: String,
    pub metric: String,
    pub comparison: String,
    pub limit: String,
    pub verdict: ConstraintVerdict,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VariantEconomics {
    pub product: String,
    pub variant: String,
    pub sku: Option<String>,
    pub bom: Vec<BomLeaf>,
    pub metrics: Vec<Metric>,
    /// Units needed to recover fixed costs, as an interval: `None` = UNKNOWN or no break-even.
    pub break_even_units: Option<Interval>,
    pub requirements: Vec<RequirementCheck>,
    /// The weakest basis among every value this variant's economics rest on.
    pub weakest_basis: Option<EvidenceBasis>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HypothesisRecord {
    pub name: String,
    pub problem: String,
    pub target_customer: String,
    /// Cited `MarketEvidence` entities that exist.
    pub evidence: Vec<String>,
    /// `HYPOTHESIZED` until evidence is cited; market demand is never asserted.
    pub status: String,
    pub market_uncertainty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductReport {
    pub schema: String,
    pub hypotheses: Vec<HypothesisRecord>,
    pub variants: Vec<VariantEconomics>,
    pub findings: Vec<ProductFinding>,
}

/// Commercial facts are declared per product; these metric names are what requirements may
/// constrain.
pub const METRICS: [&str; 12] = [
    "bom_cost",
    "tooling_amortization",
    "variable_factory_cost",
    "factory_cost",
    "variable_landed_cost",
    "landed_cost",
    "price",
    "channel_variable_cost",
    "gross_margin",
    "contribution_margin",
    "gross_margin_ratio",
    "fixed_costs",
];

struct Model<'a> {
    by_name: BTreeMap<&'a str, &'a DeclaredNode>,
    nodes: &'a [DeclaredNode],
    findings: Vec<ProductFinding>,
}

impl<'a> Model<'a> {
    fn finding(&mut self, subject: &str, verdict: ConstraintVerdict, message: String) {
        self.findings.push(ProductFinding {
            subject: subject.into(),
            verdict,
            message,
        });
    }

    fn of_kind(&self, kind: &'a str) -> impl Iterator<Item = &'a DeclaredNode> + 'a {
        self.nodes.iter().filter(move |n| n.node_kind == kind)
    }

    fn attr(&self, node: &'a DeclaredNode, key: &str) -> Option<&'a str> {
        node.attributes.get(key).map(String::as_str)
    }

    /// The node's `basis`, validated: a QUOTED basis must cite an existing `Quote`.
    fn basis(&mut self, node: &DeclaredNode) -> Option<EvidenceBasis> {
        let Some(text) = node.attributes.get("basis") else {
            self.finding(
                &node.name,
                ConstraintVerdict::Unknown,
                "no `basis` declared: the value's evidence level is unknown".into(),
            );
            return None;
        };
        let Some(basis) = EvidenceBasis::parse(text) else {
            self.finding(
                &node.name,
                ConstraintVerdict::Violated,
                format!("unknown basis `{text}`"),
            );
            return None;
        };
        if basis >= EvidenceBasis::Quoted {
            let quote = node.attributes.get("quote");
            let exists = quote.is_some_and(|q| {
                self.by_name
                    .get(q.as_str())
                    .is_some_and(|n| n.node_kind == "Quote")
            });
            if !exists {
                self.finding(
                    &node.name,
                    ConstraintVerdict::Violated,
                    format!(
                        "basis {text} requires `quote = <Quote entity>`; supplier pricing is never assumed"
                    ),
                );
                return None;
            }
        }
        Some(basis)
    }

    /// A money attribute with its basis and source. Estimates must declare a range.
    fn money(&mut self, node: &DeclaredNode, key: &str) -> Option<(Value, Option<Quantity>)> {
        let text = node.attributes.get(key)?.clone();
        let basis = self.basis(node)?;
        let (amount, currency, per) = match parse_money(&text) {
            Ok(parsed) => parsed,
            Err(message) => {
                self.finding(&node.name, ConstraintVerdict::Violated, message);
                return None;
            }
        };
        if amount.low == amount.high && !basis.admits_point_value() && amount.low != Rational::ZERO
        {
            self.finding(
                &node.name,
                ConstraintVerdict::Unknown,
                format!(
                    "`{key} = {text}` is a {basis:?} point value: declare its uncertainty as a range (low..high)"
                ),
            );
            return None;
        }
        let source = node
            .attributes
            .get("source")
            .cloned()
            .unwrap_or_else(|| "UNSTATED".into());
        if source == "UNSTATED" {
            self.finding(
                &node.name,
                ConstraintVerdict::Unknown,
                format!("`{key}` has no `source`: provenance of the number is unknown"),
            );
            return None;
        }
        Some((
            Value {
                currency,
                amount,
                basis,
                source: format!("{}: {source}", node.name),
            },
            per,
        ))
    }

    fn rate(&mut self, node: &DeclaredNode, key: &str) -> Option<(Interval, EvidenceBasis)> {
        let text = node.attributes.get(key)?.clone();
        let basis = self.basis(node)?;
        let rate = match parse_rate(&text) {
            Ok(rate) => rate,
            Err(message) => {
                self.finding(&node.name, ConstraintVerdict::Violated, message);
                return None;
            }
        };
        if rate.low == rate.high && !basis.admits_point_value() && rate.low != Rational::ZERO {
            self.finding(
                &node.name,
                ConstraintVerdict::Unknown,
                format!(
                    "`{key} = {text}` is a {basis:?} point rate: declare its uncertainty as a range"
                ),
            );
            return None;
        }
        if !node.attributes.contains_key("source") {
            self.finding(
                &node.name,
                ConstraintVerdict::Unknown,
                format!("`{key}` has no `source`: provenance of the rate is unknown"),
            );
            return None;
        }
        Some((rate, basis))
    }
}

/// A BOM line quantity: a bare decimal is a dimensionless count; anything else must be a
/// dimensioned quantity (`85 g`, `0.3 m`).
fn line_quantity(text: &str) -> Result<Quantity, crate::QuantityError> {
    match Rational::parse_decimal(text.trim()) {
        Some(count) => Ok(Quantity::new(count, crate::Dimension::DIMENSIONLESS)),
        None => Quantity::parse(text),
    }
}

fn applies_to(line: &DeclaredNode, variant: &str) -> bool {
    match line.attributes.get("variants") {
        None => true,
        Some(list) => list.split('|').map(str::trim).any(|v| v == variant),
    }
}

/// `item` is a priced leaf (`Material`/`Component` with `price`) or an `Assembly`/`Part` with its
/// own BOM lines. Explodes `parent` for `variant`, multiplying quantities down the tree.
fn explode(
    model: &mut Model<'_>,
    parent: &str,
    variant: &str,
    multiplier: Rational,
    path: &str,
    visiting: &mut BTreeSet<String>,
    leaves: &mut Vec<BomLeaf>,
) -> bool {
    if !visiting.insert(parent.to_owned()) {
        model.finding(
            parent,
            ConstraintVerdict::Violated,
            format!("BOM cycle through `{parent}`"),
        );
        return false;
    }
    let lines: Vec<&DeclaredNode> = model
        .of_kind("BomLine")
        .filter(|l| l.attributes.get("parent").map(String::as_str) == Some(parent))
        .filter(|l| applies_to(l, variant))
        .collect();
    let mut complete = true;
    for line in lines {
        let Some(item_name) = model.attr(line, "item") else {
            model.finding(&line.name, ConstraintVerdict::Violated, "no `item`".into());
            complete = false;
            continue;
        };
        let Some(item) = model.by_name.get(item_name).copied() else {
            model.finding(
                &line.name,
                ConstraintVerdict::Violated,
                format!("item `{item_name}` is not declared"),
            );
            complete = false;
            continue;
        };
        let quantity = match line.attributes.get("quantity").map(|q| line_quantity(q)) {
            Some(Ok(q)) => q,
            Some(Err(e)) => {
                model.finding(
                    &line.name,
                    ConstraintVerdict::Violated,
                    format!("quantity: {e}"),
                );
                complete = false;
                continue;
            }
            None => {
                model.finding(
                    &line.name,
                    ConstraintVerdict::Unknown,
                    "no `quantity`".into(),
                );
                complete = false;
                continue;
            }
        };
        let child_path = format!("{path}>{item_name}");
        match item.node_kind.as_str() {
            "Assembly" | "Part" => {
                if quantity.dimension != crate::Dimension::DIMENSIONLESS {
                    model.finding(
                        &line.name,
                        ConstraintVerdict::Violated,
                        format!("`{item_name}` is counted: quantity must be dimensionless"),
                    );
                    complete = false;
                    continue;
                }
                let Some(next) = multiplier.checked_mul(quantity.si_value) else {
                    complete = false;
                    continue;
                };
                complete &= explode(
                    model,
                    item_name,
                    variant,
                    next,
                    &child_path,
                    visiting,
                    leaves,
                );
            }
            "Material" | "Component" => {
                let Some((price, per)) = model.money(item, "price") else {
                    model.finding(
                        item_name,
                        ConstraintVerdict::Unknown,
                        "no usable `price`: its cost is UNKNOWN".into(),
                    );
                    complete = false;
                    continue;
                };
                // units = line quantity / pricing unit, which must be dimensionless.
                let per = per.unwrap_or_else(|| {
                    Quantity::new(Rational::ONE, crate::Dimension::DIMENSIONLESS)
                });
                let units = match quantity.checked_div(&per) {
                    Ok(ratio) if ratio.dimension == crate::Dimension::DIMENSIONLESS => {
                        ratio.si_value
                    }
                    Ok(ratio) => {
                        model.finding(
                            &line.name,
                            ConstraintVerdict::Violated,
                            format!(
                                "quantity of `{item_name}` has dimension {} relative to its price unit",
                                ratio.dimension
                            ),
                        );
                        complete = false;
                        continue;
                    }
                    Err(e) => {
                        model.finding(&line.name, ConstraintVerdict::Violated, e.to_string());
                        complete = false;
                        continue;
                    }
                };
                let Some(units) = units.checked_mul(multiplier) else {
                    complete = false;
                    continue;
                };
                let Some(amount) = price.amount.scale(units) else {
                    complete = false;
                    continue;
                };
                leaves.push(BomLeaf {
                    path: child_path,
                    item: item_name.to_owned(),
                    units,
                    cost: Value {
                        amount,
                        source: format!("{} x {}", units, price.source),
                        ..price
                    },
                });
            }
            other => {
                model.finding(
                    &line.name,
                    ConstraintVerdict::Violated,
                    format!("`{item_name}` is a {other}, not a BOM item"),
                );
                complete = false;
            }
        }
    }
    visiting.remove(parent);
    complete
}

fn sum(values: &[&Value], currency: &str, name: &str) -> Option<Value> {
    let mut amount = Interval::point(Rational::ZERO);
    let mut basis = EvidenceBasis::ProductionMeasured;
    for value in values {
        if value.currency != currency {
            return None;
        }
        amount = amount.checked_add(value.amount)?;
        basis = basis.min(value.basis);
    }
    Some(Value {
        currency: currency.into(),
        amount,
        basis,
        source: format!("derived: {name}"),
    })
}

/// Analyzes every `Product` with its variants: BOM explosion, unit economics and requirements.
pub fn analyze_products(nodes: &[DeclaredNode]) -> ProductReport {
    let mut model = Model {
        by_name: nodes.iter().map(|n| (n.name.as_str(), n)).collect(),
        nodes,
        findings: Vec::new(),
    };
    let hypotheses = model
        .of_kind("ProductHypothesis")
        .map(|h| {
            let cited: Vec<String> = h
                .attributes
                .get("evidence")
                .map(|list| {
                    list.split('|')
                        .map(str::trim)
                        .filter(|e| !e.is_empty())
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            let (found, missing): (Vec<_>, Vec<_>) = cited.into_iter().partition(|e| {
                model
                    .by_name
                    .get(e.as_str())
                    .is_some_and(|n| n.node_kind == "MarketEvidence")
            });
            (h, found, missing)
        })
        .collect::<Vec<_>>();
    let mut hypothesis_records = Vec::new();
    for (h, found, missing) in hypotheses {
        for m in &missing {
            model.finding(
                &h.name,
                ConstraintVerdict::Violated,
                format!("cited evidence `{m}` is not a declared MarketEvidence"),
            );
        }
        hypothesis_records.push(HypothesisRecord {
            name: h.name.clone(),
            problem: h.attributes.get("problem").cloned().unwrap_or_default(),
            target_customer: h
                .attributes
                .get("target_customer")
                .cloned()
                .unwrap_or_default(),
            market_uncertainty: if found.is_empty() {
                "UNKNOWN: no market evidence cited".into()
            } else {
                format!(
                    "evidence cited ({}); demand not established by citation alone",
                    found.len()
                )
            },
            evidence: found,
            status: "HYPOTHESIZED".into(),
        });
    }

    let mut variants_out = Vec::new();
    let products: Vec<&DeclaredNode> = model.of_kind("Product").collect();
    for product in products {
        let currency = product
            .attributes
            .get("currency")
            .cloned()
            .unwrap_or_default();
        if currency.len() != 3 {
            model.finding(
                &product.name,
                ConstraintVerdict::Violated,
                "a product declares one `currency` (ISO 4217)".into(),
            );
            continue;
        }
        let mut variants: Vec<&DeclaredNode> = model
            .of_kind("Variant")
            .filter(|v| v.attributes.get("product") == Some(&product.name))
            .collect();
        // A product without declared variants is its own single variant.
        let implicit = DeclaredNode {
            node_kind: "Variant".into(),
            attributes: BTreeMap::new(),
            ..product.clone()
        };
        if variants.is_empty() {
            variants.push(&implicit);
        }
        for variant in variants {
            variants_out.push(variant_economics(&mut model, product, variant, &currency));
        }
    }
    ProductReport {
        schema: PRODUCT_REPORT_SCHEMA.into(),
        hypotheses: hypothesis_records,
        variants: variants_out,
        findings: model.findings,
    }
}

fn variant_economics(
    model: &mut Model<'_>,
    product: &DeclaredNode,
    variant: &DeclaredNode,
    currency: &str,
) -> VariantEconomics {
    let mut leaves = Vec::new();
    let complete = explode(
        model,
        &product.name,
        &variant.name,
        Rational::ONE,
        &product.name,
        &mut BTreeSet::new(),
        &mut leaves,
    );
    let mut metrics: BTreeMap<&'static str, Metric> = BTreeMap::new();
    let mut put = |name: &'static str, value: Option<Value>, inputs: Vec<String>| {
        metrics.insert(
            name,
            Metric {
                name: name.into(),
                value,
                inputs,
            },
        );
    };
    let foreign: Vec<&BomLeaf> = leaves
        .iter()
        .filter(|l| l.cost.currency != currency)
        .collect();
    for leaf in &foreign {
        model.finding(
            &leaf.item,
            ConstraintVerdict::Violated,
            format!(
                "priced in {} but the product is costed in {currency}: no currency is ever converted silently",
                leaf.cost.currency
            ),
        );
    }
    let complete = complete && foreign.is_empty();
    let leaf_values: Vec<&Value> = leaves.iter().map(|l| &l.cost).collect();
    let bom_cost = if complete && !leaves.is_empty() {
        sum(&leaf_values, currency, "bom_cost")
    } else {
        None
    };
    put(
        "bom_cost",
        bom_cost.clone(),
        leaves.iter().map(|l| l.path.clone()).collect(),
    );

    // Cost items attached to the product (per unit or fixed), optionally variant-scoped.
    let items: Vec<&DeclaredNode> = model
        .of_kind("CostItem")
        .filter(|c| c.attributes.get("product") == Some(&product.name))
        .filter(|c| applies_to(c, &variant.name))
        .collect();
    let mut per_unit: BTreeMap<String, Vec<(String, Value)>> = BTreeMap::new();
    let mut fixed: Vec<(String, Value)> = Vec::new();
    let mut unknown_items = Vec::new();
    for item in items {
        let stage = item.attributes.get("stage").cloned().unwrap_or_default();
        match model.money(item, "amount") {
            Some((value, _)) if value.currency != currency => {
                model.finding(
                    &item.name,
                    ConstraintVerdict::Violated,
                    format!(
                        "amount in {} but the product is costed in {currency}: no currency is ever converted silently",
                        value.currency
                    ),
                );
                unknown_items.push(item.name.clone());
            }
            Some((value, per)) if per.is_none() => {
                if stage == "fixed" {
                    fixed.push((item.name.clone(), value));
                } else if ["factory", "landing", "channel"].contains(&stage.as_str()) {
                    per_unit
                        .entry(stage)
                        .or_default()
                        .push((item.name.clone(), value));
                } else {
                    model.finding(
                        &item.name,
                        ConstraintVerdict::Violated,
                        format!("`stage = {stage}`: expected factory, landing, channel or fixed"),
                    );
                    unknown_items.push(item.name.clone());
                }
            }
            Some(_) => {
                model.finding(
                    &item.name,
                    ConstraintVerdict::Violated,
                    "a cost item amount is per product unit, not per quantity".into(),
                );
                unknown_items.push(item.name.clone());
            }
            None => unknown_items.push(item.name.clone()),
        }
    }
    let stage_sum = |stage: &str| -> Option<Value> {
        let values: Vec<&Value> = per_unit
            .get(stage)
            .map_or(Vec::new(), |v| v.iter().map(|(_, v)| v).collect());
        sum(&values, currency, stage)
    };
    let names = |stage: &str| -> Vec<String> {
        per_unit
            .get(stage)
            .map_or(Vec::new(), |v| v.iter().map(|(n, _)| n.clone()).collect())
    };
    let any_unknown = !unknown_items.is_empty();

    // Planned volume (for tooling amortization and break-even context).
    let volume = product
        .attributes
        .get("planned_volume")
        .and_then(|v| match v.split_once("..") {
            Some((lo, hi)) => Some(Interval {
                low: Rational::parse_decimal(lo.trim())?,
                high: Rational::parse_decimal(hi.trim())?,
            }),
            None => Rational::parse_decimal(v.trim()).map(Interval::point),
        });
    let fixed_values: Vec<&Value> = fixed.iter().map(|(_, v)| v).collect();
    let fixed_costs = if any_unknown {
        None
    } else {
        sum(&fixed_values, currency, "fixed_costs")
    };
    let tooling_amortization = match (&fixed_costs, volume) {
        (Some(f), Some(v))
            if v.low.checked_cmp(Rational::ZERO) == Some(std::cmp::Ordering::Greater) =>
        {
            // Per unit: fixed / volume, widest at the smallest volume.
            let low = f.amount.low.checked_div(v.high);
            let high = f.amount.high.checked_div(v.low);
            low.zip(high).map(|(low, high)| Value {
                currency: currency.into(),
                amount: Interval { low, high },
                // Planned volume is a hypothesis, so the amortization can be no stronger.
                basis: f.basis.min(EvidenceBasis::Hypothesized),
                source: "derived: fixed_costs / planned_volume (volume is a hypothesis)".into(),
            })
        }
        _ => None,
    };
    put(
        "fixed_costs",
        fixed_costs.clone(),
        fixed.iter().map(|(n, _)| n.clone()).collect(),
    );
    put(
        "tooling_amortization",
        tooling_amortization.clone(),
        vec!["fixed_costs".into(), "planned_volume".into()],
    );

    // Scrap is yield loss on variable production: every good unit carries the material and
    // factory work of 1 / (1 - scrap) started units. (ERPNext instead nets byproduct value out of
    // raw-material cost -- a different mechanism; see ADR 0023.)
    let scrap = model
        .of_kind("CostRate")
        .filter(|r| r.attributes.get("product") == Some(&product.name))
        .find(|r| r.attributes.get("kind").map(String::as_str) == Some("scrap"))
        .and_then(|r| model_rate(model, r));
    let variable_factory = (|| {
        if any_unknown {
            return None;
        }
        let base = sum(
            &[bom_cost.as_ref()?, &stage_sum("factory")?],
            currency,
            "variable_factory_cost",
        )?;
        let (scrap_rate, scrap_basis) = scrap.unwrap_or((
            Interval::point(Rational::ZERO),
            EvidenceBasis::ProductionMeasured,
        ));
        let one = Rational::ONE;
        let yield_low = one.checked_add(scrap_rate.high.checked_neg()?)?;
        let yield_high = one.checked_add(scrap_rate.low.checked_neg()?)?;
        Some(Value {
            amount: Interval {
                low: base.amount.low.checked_div(yield_high)?,
                high: base.amount.high.checked_div(yield_low)?,
            },
            basis: base.basis.min(scrap_basis),
            ..base
        })
    })();
    let mut variable_inputs = vec!["bom_cost".into()];
    variable_inputs.extend(names("factory"));
    variable_inputs.push("scrap".into());
    put(
        "variable_factory_cost",
        variable_factory.clone(),
        variable_inputs,
    );
    let factory_cost = (|| {
        sum(
            &[variable_factory.as_ref()?, tooling_amortization.as_ref()?],
            currency,
            "factory_cost",
        )
    })();
    put(
        "factory_cost",
        factory_cost.clone(),
        vec![
            "variable_factory_cost".into(),
            "tooling_amortization".into(),
        ],
    );

    let landing = if any_unknown {
        None
    } else {
        stage_sum("landing")
    };
    let landed_cost = (|| {
        sum(
            &[factory_cost.as_ref()?, landing.as_ref()?],
            currency,
            "landed_cost",
        )
    })();
    let mut landed_inputs = vec!["factory_cost".into()];
    landed_inputs.extend(names("landing"));
    put("landed_cost", landed_cost.clone(), landed_inputs.clone());
    // Variable landed cost: landed cost without the amortized fixed costs.
    let variable_landed = (|| {
        sum(
            &[variable_factory.as_ref()?, landing.as_ref()?],
            currency,
            "variable_landed_cost",
        )
    })();
    let mut variable_landed_inputs = vec!["variable_factory_cost".into()];
    variable_landed_inputs.extend(names("landing"));
    put(
        "variable_landed_cost",
        variable_landed.clone(),
        variable_landed_inputs,
    );

    // Price hypothesis for this variant (variant-specific first, else product-wide).
    let price_node = model
        .of_kind("Price")
        .find(|p| p.attributes.get("variant") == Some(&variant.name))
        .or_else(|| {
            model.of_kind("Price").find(|p| {
                p.attributes.get("product") == Some(&product.name)
                    && !p.attributes.contains_key("variant")
            })
        });
    let price = price_node
        .and_then(|p| model.money(p, "amount"))
        .and_then(|(v, per)| per.is_none().then_some(v))
        .filter(|v| {
            let same = v.currency == currency;
            if !same {
                model.finding(
                    &price_node.expect("priced").name,
                    ConstraintVerdict::Violated,
                    format!(
                        "price in {} but the product is costed in {currency}: no currency is ever converted silently",
                        v.currency
                    ),
                );
            }
            same
        });
    put(
        "price",
        price.clone(),
        price_node.map(|p| vec![p.name.clone()]).unwrap_or_default(),
    );

    // Price-proportional channel rates (platform fee, returns and warranty reserves).
    let rates: Vec<(Interval, EvidenceBasis, String)> = model
        .of_kind("CostRate")
        .filter(|r| r.attributes.get("product") == Some(&product.name))
        .filter(|r| r.attributes.get("kind").map(String::as_str) != Some("scrap"))
        .filter_map(|r| model_rate(model, r).map(|(i, b)| (i, b, r.name.clone())))
        .collect();
    let rate_total = rates.iter().try_fold(
        (
            Interval::point(Rational::ZERO),
            EvidenceBasis::ProductionMeasured,
        ),
        |(total, basis), (rate, b, _)| Some((total.checked_add(*rate)?, basis.min(*b))),
    );
    let channel_items = if any_unknown {
        None
    } else {
        stage_sum("channel")
    };
    let channel = (|| {
        let (price, (rates, rates_basis), items) =
            (price.as_ref()?, rate_total?, channel_items.as_ref()?);
        let share = rates.mul_nonnegative(price.amount)?;
        Some(Value {
            amount: items.amount.checked_add(share)?,
            basis: items.basis.min(rates_basis).min(price.basis),
            ..items.clone()
        })
    })();
    let mut channel_inputs = names("channel");
    channel_inputs.extend(rates.iter().map(|(_, _, n)| n.clone()));
    channel_inputs.push("price".into());
    put("channel_variable_cost", channel.clone(), channel_inputs);

    // Dependency-aware closed forms: price appears once in each, so the intervals are exact
    // (naive interval subtraction would widen them, e.g. a margin ratio above 1).
    let gross_margin = (|| {
        let (p, l) = (price.as_ref()?, landed_cost.as_ref()?);
        Some(Value {
            currency: currency.into(),
            amount: p.amount.checked_sub(l.amount)?,
            basis: p.basis.min(l.basis),
            source: "derived: price - landed_cost".into(),
        })
    })();
    put(
        "gross_margin",
        gross_margin.clone(),
        vec!["price".into(), "landed_cost".into()],
    );
    // contribution = price * (1 - rates) - variable_landed - channel items.
    let contribution = (|| {
        let (p, (rates, rates_basis), v, items) = (
            price.as_ref()?,
            rate_total?,
            variable_landed.as_ref()?,
            channel_items.as_ref()?,
        );
        let one = Rational::ONE;
        let keep_low = one.checked_add(rates.high.checked_neg()?)?;
        let keep_high = one.checked_add(rates.low.checked_neg()?)?;
        let revenue = Interval {
            low: p.amount.low.checked_mul(keep_low)?,
            high: p.amount.high.checked_mul(keep_high)?,
        };
        Some(Value {
            currency: currency.into(),
            amount: revenue.checked_sub(v.amount)?.checked_sub(items.amount)?,
            basis: p.basis.min(rates_basis).min(v.basis).min(items.basis),
            source: "derived: price x (1 - channel rates) - variable_landed_cost - channel items"
                .into(),
        })
    })();
    put(
        "contribution_margin",
        contribution.clone(),
        vec![
            "price".into(),
            "channel_variable_cost".into(),
            "variable_landed_cost".into(),
        ],
    );
    // ratio = 1 - landed / price: monotone in both, so the bounds are exact.
    let ratio = (|| {
        let (p, l) = (price.as_ref()?, landed_cost.as_ref()?);
        if p.amount.low.checked_cmp(Rational::ZERO) != Some(std::cmp::Ordering::Greater) {
            return None;
        }
        let one = Rational::ONE;
        Some(Value {
            currency: "RATIO".into(),
            amount: Interval {
                low: one.checked_add(l.amount.high.checked_div(p.amount.low)?.checked_neg()?)?,
                high: one.checked_add(l.amount.low.checked_div(p.amount.high)?.checked_neg()?)?,
            },
            basis: p.basis.min(l.basis),
            source: "derived: 1 - landed_cost / price".into(),
        })
    })();
    put(
        "gross_margin_ratio",
        ratio,
        vec!["landed_cost".into(), "price".into()],
    );

    let break_even_units = (|| {
        let (f, c) = (fixed_costs.as_ref()?, contribution.as_ref()?);
        if c.amount.low.checked_cmp(Rational::ZERO) != Some(std::cmp::Ordering::Greater) {
            model.finding(
                &variant.name,
                ConstraintVerdict::Unknown,
                "contribution margin is not positive across its range: no break-even can be derived".into(),
            );
            return None;
        }
        Some(Interval {
            low: f.amount.low.checked_div(c.amount.high)?,
            high: f.amount.high.checked_div(c.amount.low)?,
        })
    })();

    let requirements = model
        .of_kind("ProductRequirement")
        .filter(|r| {
            r.attributes.get("product") == Some(&product.name) && applies_to(r, &variant.name)
        })
        .map(|r| check_requirement(r, &metrics))
        .collect();
    let weakest_basis = metrics
        .values()
        .filter_map(|m| m.value.as_ref().map(|v| v.basis))
        .min();
    let sku = model
        .of_kind("Sku")
        .find(|s| s.attributes.get("variant") == Some(&variant.name))
        .map(|s| {
            s.attributes
                .get("code")
                .cloned()
                .unwrap_or_else(|| s.name.clone())
        });
    VariantEconomics {
        product: product.name.clone(),
        variant: variant.name.clone(),
        sku,
        bom: leaves,
        metrics: metrics.into_values().collect(),
        break_even_units,
        requirements,
        weakest_basis,
    }
}

fn model_rate(model: &mut Model<'_>, node: &DeclaredNode) -> Option<(Interval, EvidenceBasis)> {
    model.rate(node, "rate")
}

/// `metric <=|>= limit` over an interval: SATISFIED only if the whole interval satisfies it,
/// VIOLATED only if the whole interval violates it, UNKNOWN otherwise or when the metric is
/// UNKNOWN.
fn check_requirement(node: &DeclaredNode, metrics: &BTreeMap<&str, Metric>) -> RequirementCheck {
    let get = |k: &str| node.attributes.get(k).cloned().unwrap_or_default();
    let (metric, comparison, limit) = (get("metric"), get("comparison"), get("limit"));
    let mut check = RequirementCheck {
        requirement: node.name.clone(),
        metric: metric.clone(),
        comparison: comparison.clone(),
        limit: limit.clone(),
        verdict: ConstraintVerdict::Unknown,
        explanation: String::new(),
    };
    let Some(value) = metrics.get(metric.as_str()).and_then(|m| m.value.as_ref()) else {
        check.explanation = format!("`{metric}` is UNKNOWN or not a metric");
        return check;
    };
    let limit_value = if value.currency == "RATIO" {
        parse_rate(&limit).map(|i| i.low)
    } else {
        parse_money(&limit).and_then(|(i, c, per)| {
            if c != value.currency || per.is_some() || i.low != i.high {
                Err(format!(
                    "limit `{limit}` must be a point amount in {}",
                    value.currency
                ))
            } else {
                Ok(i.low)
            }
        })
    };
    let limit_value = match limit_value {
        Ok(v) => v,
        Err(e) => {
            check.explanation = e;
            return check;
        }
    };
    use std::cmp::Ordering::*;
    let (lo, hi) = (value.amount.low, value.amount.high);
    let cmp = |a: Rational, b: Rational| a.checked_cmp(b).unwrap_or(Equal);
    let (all_ok, all_bad) = match comparison.as_str() {
        "<=" => (
            cmp(hi, limit_value) != Greater,
            cmp(lo, limit_value) == Greater,
        ),
        ">=" => (cmp(lo, limit_value) != Less, cmp(hi, limit_value) == Less),
        other => {
            check.explanation = format!("comparison `{other}` must be <= or >=");
            return check;
        }
    };
    check.verdict = if all_ok {
        ConstraintVerdict::Satisfied
    } else if all_bad {
        ConstraintVerdict::Violated
    } else {
        ConstraintVerdict::Unknown
    };
    check.explanation = format!(
        "{metric} in [{lo}, {hi}] ({:?}) {comparison} {limit}",
        value.basis
    );
    check
}

#[cfg(test)]
mod tests;
