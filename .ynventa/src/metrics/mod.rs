//! THE metric schema. Every repository emits exactly these metrics, computed by exactly these
//! formulas; the ecosystem aggregate sums numerators and denominators and never averages ratios.
//! Ratios are exact rationals, printed with six decimals.

use crate::formats::json::Json;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ratio {
    pub num: u64,
    pub den: u64,
}

impl Ratio {
    pub fn new(num: u64, den: u64) -> Ratio {
        Ratio { num, den }
    }
    pub fn one(&self) -> bool {
        self.den > 0 && self.num == self.den
    }
    /// Six-decimal rendering, rounded half up, computed in integers.
    pub fn render(&self) -> String {
        if self.den == 0 {
            return "undefined".into();
        }
        let scaled = (self.num as u128 * 2_000_000 + self.den as u128) / (2 * self.den as u128);
        format!("{}.{:06}", scaled / 1_000_000, scaled % 1_000_000)
    }
}

/// One metric: name, formula (part of the schema identity), and value.
pub struct MetricDef {
    pub name: &'static str,
    pub formula: &'static str,
}

pub const DEFINITIONS: &[MetricDef] = &[
    MetricDef { name: "donors_discovered", formula: "declared donors + distinct unregistered external packages + vanished donors" },
    MetricDef { name: "donors_registered", formula: "|donors whose effective state >= REGISTERED ∪ donors registered in any earlier census|; never decreases" },
    MetricDef { name: "donors_censused", formula: "active donors with effective state >= CENSUSED" },
    MetricDef { name: "donors_specified", formula: "active donors with effective state >= SPECIFIED" },
    MetricDef { name: "donors_native_shadow", formula: "active donors with effective state >= NATIVE_SHADOW" },
    MetricDef { name: "donors_parity_proven", formula: "active donors with effective state >= PARITY_PROVEN" },
    MetricDef { name: "donors_cutover", formula: "active donors with effective state >= CUTOVER" },
    MetricDef { name: "donors_extinct", formula: "active donors with effective state = EXTINCT" },
    MetricDef { name: "donors_blocked", formula: "donors with a legal BLOCKED exception" },
    MetricDef { name: "donors_rejected", formula: "donors with a legal REJECTED exception (never participated in any census)" },
    MetricDef { name: "donors_superseded", formula: "donors with a legal SUPERSEDED exception (successor carries packages and capabilities)" },
    MetricDef { name: "donors_active", formula: "donors_registered - donors_rejected - donors_superseded (vanished donors stay active)" },
    MetricDef { name: "capabilities_total", formula: "required capabilities of declared active donors" },
    MetricDef { name: "capabilities_native", formula: "of those, with an existing canonical native replacement that does not use the donor, transitively" },
    MetricDef { name: "capabilities_proven", formula: "of those native, with fresh passing parity proofs" },
    MetricDef { name: "capabilities_remaining", formula: "capabilities_total - capabilities_proven" },
    MetricDef { name: "technologies_total", formula: "declared technologies" },
    MetricDef { name: "technologies_native", formula: "technologies whose effective lifecycle >= NATIVE" },
    MetricDef { name: "technologies_proven", formula: "technologies whose effective lifecycle >= PROVEN" },
    MetricDef { name: "technologies_canonical", formula: "technologies whose effective lifecycle >= CANONICAL" },
    MetricDef { name: "technologies_adopted", formula: "technologies reused natively by another shard (decided by the system linker; 0 within one shard)" },
    MetricDef { name: "runtime_external_edges", formula: "observed foreign references of RUNTIME scope (manifest, process, source reference)" },
    MetricDef { name: "build_external_edges", formula: "observed foreign references of BUILD scope" },
    MetricDef { name: "linked_external_edges", formula: "observed native links of LINKED scope" },
    MetricDef { name: "test_external_edges", formula: "observed foreign references of TEST scope (dev-dependencies, oracles)" },
    MetricDef { name: "external_technology_edges", formula: "distinct (owning node, foreign technology) pairs over every way foreign technology participates: manifests of any scope, imports, links, process calls, source references and held (vendored, copied) donor source" },
    MetricDef { name: "external_closure_packages", formula: "locked external packages (Cargo.lock entries with a source)" },
    MetricDef { name: "canonical_nodes_total", formula: "active physical nodes other than the repository" },
    MetricDef { name: "canonical_nodes_conformant", formula: "of those, existing at a path equal to a valid canonical path" },
    MetricDef { name: "legacy_nodes_remaining", formula: "canonical_nodes_total - canonical_nodes_conformant" },
    MetricDef { name: "unmapped_nodes", formula: "directories holding code that belongs to no declared node" },
    MetricDef { name: "documents_total", formula: "tracked Markdown documents outside held donor source" },
    MetricDef { name: "documents_over_budget", formula: "documents not permitted by the document budget" },
    MetricDef { name: "repository_shape_conformance", formula: "conformant units / units; units = root entries + canonical nodes + documents + code files outside every node" },
    MetricDef { name: "ynventa_protocol_conformance", formula: "passed protocol checks / protocol checks" },
    MetricDef { name: "native_capability_ratio", formula: "capabilities_native / capabilities_total; 1 when no active donor remains, 0 when active donors declare no capability" },
    MetricDef { name: "proof_completion_ratio", formula: "capabilities_proven / capabilities_total; same guards" },
    MetricDef { name: "technology_native_ratio", formula: "technologies_native / technologies_total; 1 when there are none" },
    MetricDef { name: "extinction_ratio", formula: "donors_extinct / (donors_active + distinct unregistered external packages); 1 when the denominator is 0" },
];

/// The V1 success gate.
pub const V1_GATE: &[(&str, &str)] = &[
    ("extinction_ratio", "1.000000"),
    ("runtime_external_edges", "0"),
    ("build_external_edges", "0"),
    ("linked_external_edges", "0"),
    ("external_technology_edges", "0"),
    ("native_capability_ratio", "1.000000"),
    ("proof_completion_ratio", "1.000000"),
    ("repository_shape_conformance", "1.000000"),
    ("ynventa_protocol_conformance", "1.000000"),
];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Counts {
    pub donors_discovered: u64,
    pub donors_registered: u64,
    pub donors_censused: u64,
    pub donors_specified: u64,
    pub donors_native_shadow: u64,
    pub donors_parity_proven: u64,
    pub donors_cutover: u64,
    pub donors_extinct: u64,
    pub donors_blocked: u64,
    pub donors_rejected: u64,
    pub donors_superseded: u64,
    pub unregistered_externals: u64,
    pub capabilities_total: u64,
    pub capabilities_native: u64,
    pub capabilities_proven: u64,
    pub runtime_external_edges: u64,
    pub build_external_edges: u64,
    pub linked_external_edges: u64,
    pub test_external_edges: u64,
    pub external_technology_edges: u64,
    pub technologies_total: u64,
    pub technologies_native: u64,
    pub technologies_proven: u64,
    pub technologies_canonical: u64,
    pub technologies_adopted: u64,
    pub unmapped_nodes: u64,
    pub external_closure_packages: u64,
    pub canonical_nodes_total: u64,
    pub canonical_nodes_conformant: u64,
    pub documents_total: u64,
    pub documents_over_budget: u64,
    pub shape_units: u64,
    pub shape_units_conformant: u64,
    pub protocol_checks: u64,
    pub protocol_checks_passed: u64,
}

impl Counts {
    pub fn add(&mut self, o: &Counts) {
        macro_rules! sum { ($($f:ident),*) => { $(self.$f += o.$f;)* } }
        sum!(
            donors_discovered,
            donors_registered,
            donors_censused,
            donors_specified,
            donors_native_shadow,
            donors_parity_proven,
            donors_cutover,
            donors_extinct,
            donors_blocked,
            donors_rejected,
            donors_superseded,
            unregistered_externals,
            capabilities_total,
            capabilities_native,
            capabilities_proven,
            runtime_external_edges,
            build_external_edges,
            linked_external_edges,
            test_external_edges,
            external_closure_packages,
            canonical_nodes_total,
            canonical_nodes_conformant,
            documents_total,
            documents_over_budget,
            shape_units,
            shape_units_conformant,
            protocol_checks,
            protocol_checks_passed,
            external_technology_edges,
            technologies_total,
            technologies_native,
            technologies_proven,
            technologies_canonical,
            technologies_adopted,
            unmapped_nodes
        );
    }

    /// Every raw count, by field name (schema order of declaration).
    pub fn raw(&self) -> Vec<(&'static str, u64)> {
        vec![
            ("donors_discovered", self.donors_discovered),
            ("donors_registered", self.donors_registered),
            ("donors_censused", self.donors_censused),
            ("donors_specified", self.donors_specified),
            ("donors_native_shadow", self.donors_native_shadow),
            ("donors_parity_proven", self.donors_parity_proven),
            ("donors_cutover", self.donors_cutover),
            ("donors_extinct", self.donors_extinct),
            ("donors_blocked", self.donors_blocked),
            ("donors_rejected", self.donors_rejected),
            ("donors_superseded", self.donors_superseded),
            ("unregistered_externals", self.unregistered_externals),
            ("capabilities_total", self.capabilities_total),
            ("capabilities_native", self.capabilities_native),
            ("capabilities_proven", self.capabilities_proven),
            ("runtime_external_edges", self.runtime_external_edges),
            ("build_external_edges", self.build_external_edges),
            ("linked_external_edges", self.linked_external_edges),
            ("test_external_edges", self.test_external_edges),
            ("external_closure_packages", self.external_closure_packages),
            ("canonical_nodes_total", self.canonical_nodes_total),
            (
                "canonical_nodes_conformant",
                self.canonical_nodes_conformant,
            ),
            ("documents_total", self.documents_total),
            ("documents_over_budget", self.documents_over_budget),
            ("shape_units", self.shape_units),
            ("shape_units_conformant", self.shape_units_conformant),
            ("protocol_checks", self.protocol_checks),
            ("protocol_checks_passed", self.protocol_checks_passed),
            ("external_technology_edges", self.external_technology_edges),
            ("technologies_total", self.technologies_total),
            ("technologies_native", self.technologies_native),
            ("technologies_proven", self.technologies_proven),
            ("technologies_canonical", self.technologies_canonical),
            ("technologies_adopted", self.technologies_adopted),
            ("unmapped_nodes", self.unmapped_nodes),
        ]
    }

    /// Rebuilds counts from `raw()` output; unknown names are ignored.
    pub fn from_raw(raw: &[(String, u64)]) -> Counts {
        let mut c = Counts::default();
        for (k, v) in raw {
            match k.as_str() {
                "donors_discovered" => c.donors_discovered = *v,
                "donors_registered" => c.donors_registered = *v,
                "donors_censused" => c.donors_censused = *v,
                "donors_specified" => c.donors_specified = *v,
                "donors_native_shadow" => c.donors_native_shadow = *v,
                "donors_parity_proven" => c.donors_parity_proven = *v,
                "donors_cutover" => c.donors_cutover = *v,
                "donors_extinct" => c.donors_extinct = *v,
                "donors_blocked" => c.donors_blocked = *v,
                "donors_rejected" => c.donors_rejected = *v,
                "donors_superseded" => c.donors_superseded = *v,
                "unregistered_externals" => c.unregistered_externals = *v,
                "capabilities_total" => c.capabilities_total = *v,
                "capabilities_native" => c.capabilities_native = *v,
                "capabilities_proven" => c.capabilities_proven = *v,
                "runtime_external_edges" => c.runtime_external_edges = *v,
                "build_external_edges" => c.build_external_edges = *v,
                "linked_external_edges" => c.linked_external_edges = *v,
                "test_external_edges" => c.test_external_edges = *v,
                "external_closure_packages" => c.external_closure_packages = *v,
                "canonical_nodes_total" => c.canonical_nodes_total = *v,
                "canonical_nodes_conformant" => c.canonical_nodes_conformant = *v,
                "documents_total" => c.documents_total = *v,
                "documents_over_budget" => c.documents_over_budget = *v,
                "shape_units" => c.shape_units = *v,
                "shape_units_conformant" => c.shape_units_conformant = *v,
                "protocol_checks" => c.protocol_checks = *v,
                "protocol_checks_passed" => c.protocol_checks_passed = *v,
                "external_technology_edges" => c.external_technology_edges = *v,
                "technologies_total" => c.technologies_total = *v,
                "technologies_native" => c.technologies_native = *v,
                "technologies_proven" => c.technologies_proven = *v,
                "technologies_canonical" => c.technologies_canonical = *v,
                "technologies_adopted" => c.technologies_adopted = *v,
                "unmapped_nodes" => c.unmapped_nodes = *v,
                _ => {}
            }
        }
        c
    }

    pub fn donors_active(&self) -> u64 {
        self.donors_registered
            .saturating_sub(self.donors_rejected + self.donors_superseded)
    }

    fn capability_ratio(&self, num: u64) -> Ratio {
        if self.capabilities_total > 0 {
            Ratio::new(num, self.capabilities_total)
        } else if self.donors_active() == 0 && self.unregistered_externals == 0 {
            Ratio::new(1, 1)
        } else {
            Ratio::new(0, 1)
        }
    }

    /// Every metric of the schema, in schema order, as (name, printed value).
    pub fn values(&self) -> Vec<(String, String)> {
        let n = |v: u64| v.to_string();
        let ext_den = self.donors_active() + self.unregistered_externals;
        let extinction = if ext_den == 0 {
            Ratio::new(1, 1)
        } else {
            Ratio::new(self.donors_extinct, ext_den)
        };
        let shape = if self.shape_units == 0 {
            Ratio::new(1, 1)
        } else {
            Ratio::new(self.shape_units_conformant, self.shape_units)
        };
        let proto = if self.protocol_checks == 0 {
            Ratio::new(0, 1)
        } else {
            Ratio::new(self.protocol_checks_passed, self.protocol_checks)
        };
        let v: Vec<(&str, String)> = vec![
            ("donors_discovered", n(self.donors_discovered)),
            ("donors_registered", n(self.donors_registered)),
            ("donors_censused", n(self.donors_censused)),
            ("donors_specified", n(self.donors_specified)),
            ("donors_native_shadow", n(self.donors_native_shadow)),
            ("donors_parity_proven", n(self.donors_parity_proven)),
            ("donors_cutover", n(self.donors_cutover)),
            ("donors_extinct", n(self.donors_extinct)),
            ("donors_blocked", n(self.donors_blocked)),
            ("donors_rejected", n(self.donors_rejected)),
            ("donors_superseded", n(self.donors_superseded)),
            ("donors_active", n(self.donors_active())),
            ("capabilities_total", n(self.capabilities_total)),
            ("capabilities_native", n(self.capabilities_native)),
            ("capabilities_proven", n(self.capabilities_proven)),
            (
                "capabilities_remaining",
                n(self
                    .capabilities_total
                    .saturating_sub(self.capabilities_proven)),
            ),
            ("technologies_total", n(self.technologies_total)),
            ("technologies_native", n(self.technologies_native)),
            ("technologies_proven", n(self.technologies_proven)),
            ("technologies_canonical", n(self.technologies_canonical)),
            ("technologies_adopted", n(self.technologies_adopted)),
            ("runtime_external_edges", n(self.runtime_external_edges)),
            ("build_external_edges", n(self.build_external_edges)),
            ("linked_external_edges", n(self.linked_external_edges)),
            ("test_external_edges", n(self.test_external_edges)),
            (
                "external_technology_edges",
                n(self.external_technology_edges),
            ),
            (
                "external_closure_packages",
                n(self.external_closure_packages),
            ),
            ("canonical_nodes_total", n(self.canonical_nodes_total)),
            (
                "canonical_nodes_conformant",
                n(self.canonical_nodes_conformant),
            ),
            (
                "legacy_nodes_remaining",
                n(self
                    .canonical_nodes_total
                    .saturating_sub(self.canonical_nodes_conformant)),
            ),
            ("unmapped_nodes", n(self.unmapped_nodes)),
            ("documents_total", n(self.documents_total)),
            ("documents_over_budget", n(self.documents_over_budget)),
            ("repository_shape_conformance", shape.render()),
            ("ynventa_protocol_conformance", proto.render()),
            (
                "native_capability_ratio",
                self.capability_ratio(self.capabilities_native).render(),
            ),
            (
                "proof_completion_ratio",
                self.capability_ratio(self.capabilities_proven).render(),
            ),
            (
                "technology_native_ratio",
                if self.technologies_total == 0 {
                    Ratio::new(1, 1)
                } else {
                    Ratio::new(self.technologies_native, self.technologies_total)
                }
                .render(),
            ),
            ("extinction_ratio", extinction.render()),
        ];
        debug_assert_eq!(v.len(), DEFINITIONS.len());
        v.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    /// The V1 gate rows: (metric, required, actual, pass).
    pub fn v1_gate(&self) -> Vec<(String, String, String, bool)> {
        let values = self.values();
        V1_GATE
            .iter()
            .map(|(k, want)| {
                let got = values
                    .iter()
                    .find(|(n, _)| n == k)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                (k.to_string(), want.to_string(), got.clone(), got == *want)
            })
            .collect()
    }

    pub fn to_json(&self) -> Json {
        let mut o = Json::obj().with("schema", crate::protocol::schema_identity());
        for (k, v) in self.values() {
            o = o.with(&k, Json::Number(v));
        }
        o
    }
}

pub fn render_definitions() -> String {
    let mut s = String::new();
    for d in DEFINITIONS {
        s.push_str(&format!("metric {} = {}\n", d.name, d.formula));
    }
    for (k, v) in V1_GATE {
        s.push_str(&format!("v1_gate {k} == {v}\n"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratios_render_exactly() {
        assert_eq!(Ratio::new(1, 3).render(), "0.333333");
        assert_eq!(Ratio::new(2, 3).render(), "0.666667");
        assert_eq!(Ratio::new(3, 3).render(), "1.000000");
        assert_eq!(Ratio::new(0, 7).render(), "0.000000");
    }

    #[test]
    fn denominator_cannot_be_gamed() {
        // Registered donors are counted forever; an active donor without capabilities is 0 %.
        let c = Counts {
            donors_registered: 2,
            donors_extinct: 1,
            ..Counts::default()
        };
        let v = c.values();
        let get = |k: &str| v.iter().find(|(n, _)| n == k).unwrap().1.clone();
        assert_eq!(get("extinction_ratio"), "0.500000");
        assert_eq!(get("native_capability_ratio"), "0.000000");
        // Unregistered externals join the denominator.
        let c = Counts {
            unregistered_externals: 1,
            ..Counts::default()
        };
        assert_eq!(
            c.values()
                .iter()
                .find(|(n, _)| n == "extinction_ratio")
                .unwrap()
                .1,
            "0.000000"
        );
        // Nothing foreign at all: vacuously complete.
        let c = Counts::default();
        assert_eq!(
            c.values()
                .iter()
                .find(|(n, _)| n == "extinction_ratio")
                .unwrap()
                .1,
            "1.000000"
        );
    }

    #[test]
    fn aggregation_sums_then_divides() {
        let a = Counts {
            donors_registered: 1,
            donors_extinct: 1,
            ..Counts::default()
        };
        let b = Counts {
            donors_registered: 3,
            donors_extinct: 0,
            ..Counts::default()
        };
        let mut t = a.clone();
        t.add(&b);
        // Not the mean of 1.0 and 0.0.
        assert_eq!(
            t.values()
                .iter()
                .find(|(n, _)| n == "extinction_ratio")
                .unwrap()
                .1,
            "0.250000"
        );
    }
}
