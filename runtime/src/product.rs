//! Product Foundry orchestration (ADR 0023): compile a root's declared ADL and analyze its
//! products. Digital only -- estimates and hypotheses, never market or supplier facts.

use atlas_core::{
    AdlCompileReport, SourceReport, compile_adl,
    product::{ProductReport, analyze_products},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductAnalysis {
    pub product: ProductReport,
    /// The ADL compile the product model was read from.
    pub adl: AdlCompileReport,
}

pub fn analyze_root(root: impl AsRef<Path>) -> io::Result<ProductAnalysis> {
    let sources = adapter::read_adl_sources(root)?;
    let observed = SourceReport {
        schema: "atlas.product-no-source-observation".into(),
        root: String::new(),
        files_total: 0,
        languages: BTreeMap::new(),
        files: Vec::new(),
    };
    let adl = compile_adl(&sources, &observed);
    let product = analyze_products(&adl.ir.declared.nodes);
    Ok(ProductAnalysis { product, adl })
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::{
        ConstraintVerdict,
        product::{EvidenceBasis, Interval, VariantEconomics},
        quantity::Rational,
    };

    fn fixture() -> ProductAnalysis {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/product/desk-organizer");
        analyze_root(&root).unwrap()
    }

    fn q(n: i128, d: i128) -> Rational {
        Rational::new(n, d).unwrap()
    }

    fn interval(v: &VariantEconomics, name: &str) -> Interval {
        v.metrics
            .iter()
            .find(|m| m.name == name)
            .and_then(|m| m.value.as_ref())
            .unwrap_or_else(|| panic!("{name} is UNKNOWN"))
            .amount
    }

    /// Every expected value below comes from an independent Python `fractions` oracle written
    /// from the economic definitions (G56 evidence), not from this implementation.
    #[test]
    fn the_desk_organizer_economics_match_the_independent_oracle_exactly() {
        let analysis = fixture();
        assert!(
            analysis.adl.diagnostics.is_empty(),
            "{:#?}",
            analysis.adl.diagnostics
        );
        let report = &analysis.product;
        assert!(report.findings.is_empty(), "{:#?}", report.findings);
        assert_eq!(report.variants.len(), 2);
        type Fraction = (i128, i128);
        type Expected<'a> = (&'a str, &'a str, [(&'a str, Fraction, Fraction); 7]);
        let expected: [Expected; 2] = [
            (
                "Small",
                "ORG-S-BLK",
                [
                    ("bom_cost", (271, 200), (11, 4)),
                    ("variable_factory_cost", (391, 190), (79, 18)),
                    ("factory_cost", (801, 380), (185, 36)),
                    ("landed_cost", (1143, 380), (1339, 180)),
                    ("gross_margin", (2981, 180), (9877, 380)),
                    ("gross_margin_ratio", (2981, 4320), (9877, 11020)),
                    ("contribution_margin", (3731, 450), (7293, 380)),
                ],
            ),
            (
                "Large",
                "ORG-L-BLK",
                [
                    ("bom_cost", (109, 50), (22, 5)),
                    ("variable_factory_cost", (278, 95), (56, 9)),
                    ("factory_cost", (1131, 380), (251, 36)),
                    ("landed_cost", (1473, 380), (1669, 180)),
                    ("gross_margin", (4091, 180), (13347, 380)),
                    ("gross_margin_ratio", (4091, 5760), (4449, 4940)),
                    ("contribution_margin", (2839, 225), (10193, 380)),
                ],
            ),
        ];
        for (variant, sku, metrics) in expected {
            let v = report
                .variants
                .iter()
                .find(|v| v.variant == variant)
                .unwrap();
            assert_eq!(v.sku.as_deref(), Some(sku));
            for (name, low, high) in metrics {
                assert_eq!(
                    interval(v, name),
                    Interval {
                        low: q(low.0, low.1),
                        high: q(high.0, high.1)
                    },
                    "{variant} {name}"
                );
            }
            // Nothing stronger than the price hypothesis backs any margin.
            assert_eq!(v.weakest_basis, Some(EvidenceBasis::Hypothesized));
            // Both requirements hold across the whole declared uncertainty.
            for check in &v.requirements {
                assert_eq!(check.verdict, ConstraintVerdict::Satisfied, "{check:?}");
            }
        }
        let small = report
            .variants
            .iter()
            .find(|v| v.variant == "Small")
            .unwrap();
        assert_eq!(
            small.break_even_units,
            Some(Interval {
                low: q(19000, 7293),
                high: q(67500, 3731)
            })
        );
        // No market evidence is on record: demand stays UNKNOWN.
        assert!(
            report.hypotheses[0]
                .market_uncertainty
                .starts_with("UNKNOWN")
        );
    }
}
