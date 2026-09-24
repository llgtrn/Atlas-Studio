//! Cross-engine differential evidence (ADR 0022): the same subject observed through two browser
//! instruments, normalized into the same Atlas semantics, and compared at the semantic level.
//!
//! Engines are instruments; neither is ground truth. Agreement is judged on Atlas relations within
//! the combined uncertainty each instrument declares (its layout resolution), not on bit-identical
//! pixels. Every disagreement is kept and starts `UNCLASSIFIED`; a classification is attached
//! only from a recorded investigation, never by preferring one engine.

use super::{
    InteractionAnalysis, InteractionMechanism, LayoutRelationKind, ResponsiveChange,
    VisualObservationReport, analyze,
};
use crate::IntegrityDigest;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Style properties whose computed values carry layout structure; compared exactly.
pub const STRUCTURAL_PROPERTIES: [&str; 4] = ["display", "position", "flex-direction", "flex-wrap"];

/// Classification of one disagreement, attached from an investigation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "class")]
pub enum DifferenceClass {
    /// Not yet investigated.
    Unclassified,
    AtlasBug,
    /// Behaviour specific to the named engine.
    EngineSpecific {
        engine: String,
    },
    /// The named engine does not (yet) implement what the subject uses.
    EngineLimitation {
        engine: String,
    },
    /// The named instrument (driver/harness), not the engine, cannot do what the subject needs
    /// (e.g. isolate the network).
    InstrumentLimitation {
        instrument: String,
    },
    SpecAmbiguity,
    MeasurementVariance,
    /// Investigated without a conclusion.
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EngineIdentity {
    pub engine: String,
    pub engine_version: String,
    pub driver: String,
    pub driver_version: String,
    pub network: String,
}

/// One compared item on which the engines disagree. `values` are in `engines` order;
/// `None` = the engine produced no such item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Disagreement {
    pub item: String,
    pub values: Vec<Option<String>>,
    pub classification: DifferenceClass,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineDifferentialReport {
    pub schema: String,
    pub subject_digest: IntegrityDigest,
    /// `LAYOUT`, `BREAKPOINTS` or `INTERACTION`.
    pub operation: String,
    pub engines: Vec<EngineIdentity>,
    /// Items compared and found in agreement.
    pub agreements: usize,
    pub disagreements: Vec<Disagreement>,
    /// Disagreements not yet classified (`UNCLASSIFIED`).
    pub unresolved: usize,
    /// No disagreement at all: the engines reach the same Atlas conclusions on this subject.
    pub semantic_agreement: bool,
    /// Whether the comparison is evidence of engine independence at all.
    pub independence: Independence,
}

/// What a differential can and cannot show. Agreement between two drivers of the same engine
/// shows driver independence only; it is never reported as cross-engine evidence.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Independence {
    /// Different rendering engines (e.g. Blink vs LibWeb).
    IndependentEngines,
    /// One engine reached through different drivers (e.g. Playwright vs WebDriver).
    SameEngineDifferentDriver,
    /// The same engine and driver (a repeatability check).
    SameInstrument,
    /// An engine family is not recognised.
    Unknown,
}

/// The rendering-engine family of a reported engine name.
pub fn engine_family(engine: &str) -> Option<&'static str> {
    match engine.to_ascii_lowercase().as_str() {
        "chromium" | "chrome" | "chrome-headless-shell" | "headless_shell" | "msedge" => {
            Some("blink")
        }
        "ladybird" => Some("libweb"),
        "firefox" => Some("gecko"),
        "webkit" | "safari" => Some("webkit"),
        _ => None,
    }
}

pub fn independence(engines: &[EngineIdentity]) -> Independence {
    let families: Option<BTreeSet<&str>> =
        engines.iter().map(|e| engine_family(&e.engine)).collect();
    match families {
        None => Independence::Unknown,
        Some(families) if families.len() > 1 => Independence::IndependentEngines,
        Some(_) => {
            let drivers: BTreeSet<&str> = engines.iter().map(|e| e.driver.as_str()).collect();
            if drivers.len() > 1 {
                Independence::SameEngineDifferentDriver
            } else {
                Independence::SameInstrument
            }
        }
    }
}

pub const DIFFERENTIAL_SCHEMA: &str = "atlas.engine-differential.v1";

impl EngineDifferentialReport {
    fn new(
        subject_digest: IntegrityDigest,
        operation: &str,
        engines: Vec<EngineIdentity>,
        agreements: usize,
        disagreements: Vec<Disagreement>,
    ) -> Self {
        let unresolved = disagreements
            .iter()
            .filter(|d| d.classification == DifferenceClass::Unclassified)
            .count();
        Self {
            independence: independence(&engines),
            schema: DIFFERENTIAL_SCHEMA.into(),
            subject_digest,
            operation: operation.into(),
            engines,
            semantic_agreement: disagreements.is_empty(),
            agreements,
            disagreements,
            unresolved,
        }
    }

    /// Attaches recorded classifications (item -> class). Unmatched disagreements stay
    /// `UNCLASSIFIED`; classifying never turns a disagreement into an agreement.
    pub fn classify(&mut self, recorded: &BTreeMap<String, DifferenceClass>) {
        for disagreement in &mut self.disagreements {
            if let Some(class) = recorded.get(&disagreement.item) {
                disagreement.classification = class.clone();
            }
        }
        self.unresolved = self
            .disagreements
            .iter()
            .filter(|d| d.classification == DifferenceClass::Unclassified)
            .count();
    }
}

fn identity(report: &VisualObservationReport) -> EngineIdentity {
    let i = &report.instrument;
    EngineIdentity {
        engine: i.engine.clone(),
        engine_version: i.engine_version.clone(),
        driver: i.driver.clone(),
        driver_version: i.driver_version.clone(),
        network: i.network.clone(),
    }
}

/// Accumulates keyed items from two sides and judges each pair.
struct Tally {
    agreements: usize,
    disagreements: Vec<Disagreement>,
}

impl Tally {
    fn new() -> Self {
        Self {
            agreements: 0,
            disagreements: Vec::new(),
        }
    }

    fn compare<T>(
        &mut self,
        left: BTreeMap<String, T>,
        mut right: BTreeMap<String, T>,
        agree: impl Fn(&T, &T) -> bool,
        show: impl Fn(&T) -> String,
    ) {
        for (item, a) in left {
            match right.remove(&item) {
                Some(b) if agree(&a, &b) => self.agreements += 1,
                b => self.disagreements.push(Disagreement {
                    item,
                    values: vec![Some(show(&a)), b.as_ref().map(&show)],
                    classification: DifferenceClass::Unclassified,
                }),
            }
        }
        for (item, b) in right {
            self.disagreements.push(Disagreement {
                item,
                values: vec![None, Some(show(&b))],
                classification: DifferenceClass::Unclassified,
            });
        }
    }
}

/// Compares two layout observations of the same subject at the same viewports, through Atlas's own
/// analysis of each. Refuses subjects or viewport sets that differ (that is not a differential).
pub fn compare_layout(
    left: &VisualObservationReport,
    right: &VisualObservationReport,
) -> Result<EngineDifferentialReport, String> {
    if left.subject.content_digest != right.subject.content_digest {
        return Err("the two observations are of different subjects".into());
    }
    let widths = |r: &VisualObservationReport| -> BTreeSet<(u32, u32)> {
        r.viewports.iter().map(|v| (v.width, v.height)).collect()
    };
    if widths(left) != widths(right) {
        return Err("the two observations use different viewports".into());
    }
    let mut tally = Tally::new();
    // Element presence and structural computed style.
    let structure = |r: &VisualObservationReport| -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        for viewport in &r.viewports {
            for element in &viewport.elements {
                for property in STRUCTURAL_PROPERTIES {
                    let value = element.style.get(property).cloned().unwrap_or_default();
                    out.insert(
                        format!("style {property} {} @{}", element.path, viewport.width),
                        value,
                    );
                }
                let columns = element
                    .style
                    .get("grid-template-columns")
                    .map(|v| super::grid_columns(v))
                    .unwrap_or_default();
                out.insert(
                    format!("grid-columns {} @{}", element.path, viewport.width),
                    columns.map_or("none".into(), |c| c.to_string()),
                );
            }
        }
        out
    };
    tally.compare(
        structure(left),
        structure(right),
        |a, b| a == b,
        Clone::clone,
    );
    // Normalized relations, agreeing within the combined declared uncertainty.
    let relations = |r: &VisualObservationReport| -> BTreeMap<String, (f64, f64)> {
        analyze(r)
            .relations
            .into_iter()
            .map(|rel| {
                (
                    format!(
                        "relation {} {} @{}",
                        kind_label(&rel.relation),
                        rel.element,
                        rel.viewport_width
                    ),
                    (rel.value, rel.uncertainty),
                )
            })
            .collect()
    };
    tally.compare(
        relations(left),
        relations(right),
        |a, b| (a.0 - b.0).abs() <= a.1 + b.1 + 1e-12,
        |v| format!("{:.6} (±{:.1e})", v.0, v.1),
    );
    // Responsive structure between the observed viewports.
    let rules = |r: &VisualObservationReport| -> BTreeMap<String, ()> {
        analyze(r)
            .responsive_rules
            .into_iter()
            .map(|rule| {
                (
                    format!(
                        "responsive {} {} ({}..{})",
                        change_label(&rule.change),
                        rule.element,
                        rule.narrower,
                        rule.wider
                    ),
                    (),
                )
            })
            .collect()
    };
    tally.compare(rules(left), rules(right), |_, _| true, |_| "present".into());
    Ok(EngineDifferentialReport::new(
        left.subject.content_digest.clone(),
        "LAYOUT",
        vec![identity(left), identity(right)],
        tally.agreements,
        tally.disagreements,
    ))
}

/// A located responsive transition: `(element, change, narrow_width, wide_width)`.
pub type LocatedBreakpoint = (String, ResponsiveChange, u32, u32);

/// Compares bisected breakpoints: agreement iff both engines locate the same change of the same
/// element to the same one-pixel boundary.
pub fn compare_breakpoints(
    subject_digest: IntegrityDigest,
    engines: Vec<EngineIdentity>,
    left: &[LocatedBreakpoint],
    right: &[LocatedBreakpoint],
) -> EngineDifferentialReport {
    let keyed = |list: &[LocatedBreakpoint]| -> BTreeMap<String, (u32, u32)> {
        list.iter()
            .map(|(element, change, narrow, wide)| {
                (
                    format!("breakpoint {} {element}", change_label(change)),
                    (*narrow, *wide),
                )
            })
            .collect()
    };
    let mut tally = Tally::new();
    tally.compare(
        keyed(left),
        keyed(right),
        |a, b| a == b,
        |(n, w)| format!("({n}, {w})"),
    );
    EngineDifferentialReport::new(
        subject_digest,
        "BREAKPOINTS",
        engines,
        tally.agreements,
        tally.disagreements,
    )
}

/// Compares interaction semantics: the abstract mechanisms (kind + target, with their property /
/// controlled sets) and the state-graph transitions -- not engine-internal change lists.
pub fn compare_interactions(
    subject_digest: IntegrityDigest,
    engines: Vec<EngineIdentity>,
    left: &InteractionAnalysis,
    right: &InteractionAnalysis,
) -> EngineDifferentialReport {
    let mechanisms = |a: &InteractionAnalysis| -> BTreeMap<String, BTreeSet<String>> {
        a.mechanisms
            .iter()
            .map(|m| match m {
                InteractionMechanism::HoverAffordance { target, properties } => (
                    format!("mechanism HOVER_AFFORDANCE {target}"),
                    properties.iter().cloned().collect(),
                ),
                InteractionMechanism::Disclosure { target, controlled } => (
                    format!("mechanism DISCLOSURE {target}"),
                    controlled.iter().cloned().collect(),
                ),
                InteractionMechanism::Toggle { target } => {
                    (format!("mechanism TOGGLE {target}"), BTreeSet::new())
                }
                InteractionMechanism::FocusIndicator { target, properties } => (
                    format!("mechanism FOCUS_INDICATOR {target}"),
                    properties.iter().cloned().collect(),
                ),
            })
            .collect()
    };
    let transitions = |a: &InteractionAnalysis| -> BTreeMap<String, ()> {
        a.transitions
            .iter()
            .map(|t| {
                (
                    format!(
                        "transition {} --{:?} {}--> {}",
                        t.from, t.stimulus, t.target, t.to
                    ),
                    (),
                )
            })
            .collect()
    };
    let mut tally = Tally::new();
    tally.compare(
        mechanisms(left),
        mechanisms(right),
        |a, b| a == b,
        |set| set.iter().cloned().collect::<Vec<_>>().join(","),
    );
    tally.compare(
        transitions(left),
        transitions(right),
        |_, _| true,
        |_| "present".into(),
    );
    EngineDifferentialReport::new(
        subject_digest,
        "INTERACTION",
        engines,
        tally.agreements,
        tally.disagreements,
    )
}

fn kind_label(kind: &LayoutRelationKind) -> String {
    format!("{kind:?}")
}

fn change_label(change: &ResponsiveChange) -> String {
    format!("{change:?}")
}

#[cfg(test)]
mod tests {
    use super::super::tests::{element, report, viewport};
    use super::*;
    use crate::visual::{InteractionTransition, Stimulus};

    fn observed(engine: &str, resolution: f64, child_width: f64) -> VisualObservationReport {
        let layout = |width: u32, direction: &str, child: f64| {
            viewport(
                width,
                vec![
                    element(
                        "body>section:1",
                        Some("body"),
                        [0.0, 0.0, f64::from(width), 300.0],
                        &[("display", "flex"), ("flex-direction", direction)],
                    ),
                    element(
                        "body>section:1>div:1",
                        Some("body>section:1"),
                        [0.0, 0.0, child, 100.0],
                        &[("display", "block")],
                    ),
                ],
            )
        };
        let mut r = report(vec![
            layout(400, "column", 400.0),
            layout(1000, "row", child_width),
        ]);
        r.instrument.engine = engine.into();
        r.instrument.layout_resolution_px = resolution;
        r
    }

    #[test]
    fn engines_agree_semantically_within_their_declared_resolution() {
        // Chromium lays out 1/64 px; another engine reporting 1/64 px off agrees at the relation.
        let chromium = observed("chromium", 1.0 / 64.0, 666.640625);
        let other = observed("ladybird", 1.0 / 64.0, 666.625);
        let report = compare_layout(&chromium, &other).unwrap();
        assert!(report.semantic_agreement, "{:#?}", report.disagreements);
        assert!(report.agreements > 10);
        assert_eq!(report.engines[1].engine, "ladybird");
    }

    #[test]
    fn uncertainty_comes_from_the_declared_instrument_resolution() {
        let fine = analyze(&observed("a", 1.0 / 64.0, 666.0));
        let coarse = analyze(&observed("b", 1.0, 666.0));
        let pick = |a: &super::super::VisualAnalysis| {
            a.relations
                .iter()
                .find(|r| {
                    r.element == "body>section:1>div:1"
                        && r.relation == LayoutRelationKind::WidthOverViewport
                        && r.viewport_width == 1000
                })
                .unwrap()
                .uncertainty
        };
        // n/d with n = 666, d = 1000, both exact to e: (n + e)/(d - e) - n/d.
        let bound = |e: f64| (666.0 + e) / (1000.0 - e) - 666.0 / 1000.0;
        assert!(
            (pick(&fine) - bound(1.0 / 64.0)).abs() < 1e-12,
            "{}",
            pick(&fine)
        );
        assert!(
            (pick(&coarse) - bound(1.0)).abs() < 1e-12,
            "{}",
            pick(&coarse)
        );
        // A 2 px difference is a disagreement at 1/64 px resolution but agrees at 2 px resolution.
        let fine_pair = compare_layout(
            &observed("a", 1.0 / 64.0, 666.0),
            &observed("b", 1.0 / 64.0, 668.0),
        )
        .unwrap();
        assert!(!fine_pair.semantic_agreement);
        let coarse_pair =
            compare_layout(&observed("a", 2.0, 666.0), &observed("b", 2.0, 668.0)).unwrap();
        assert!(
            coarse_pair.semantic_agreement,
            "{:#?}",
            coarse_pair.disagreements
        );
    }

    #[test]
    fn a_real_difference_is_kept_unclassified_until_investigated() {
        let chromium = observed("chromium", 1.0 / 64.0, 666.640625);
        let other = observed("ladybird", 1.0 / 64.0, 640.0);
        let mut report = compare_layout(&chromium, &other).unwrap();
        assert!(!report.semantic_agreement);
        assert!(report.unresolved > 0);
        assert_eq!(report.unresolved, report.disagreements.len());
        let item = report.disagreements[0].item.clone();
        let mut recorded = BTreeMap::new();
        recorded.insert(
            item,
            DifferenceClass::EngineLimitation {
                engine: "ladybird".into(),
            },
        );
        report.classify(&recorded);
        assert_eq!(report.unresolved, report.disagreements.len() - 1);
        assert!(
            !report.semantic_agreement,
            "classifying never makes agreement"
        );
    }

    #[test]
    fn structural_style_and_responsive_rules_are_compared() {
        let chromium = observed("chromium", 1.0 / 64.0, 666.640625);
        let mut other = observed("ladybird", 1.0 / 64.0, 666.640625);
        // The other engine never switches to a row: display structure and the rule both differ.
        for element in &mut other.viewports[1].elements {
            if element.path == "body>section:1" {
                element
                    .style
                    .insert("flex-direction".into(), "column".into());
            }
        }
        let report = compare_layout(&chromium, &other).unwrap();
        let items: Vec<&str> = report
            .disagreements
            .iter()
            .map(|d| d.item.as_str())
            .collect();
        assert!(
            items
                .iter()
                .any(|i| i.starts_with("style flex-direction body>section:1 @1000"))
        );
        assert!(
            items.iter().any(|i| i.starts_with("responsive ")),
            "{items:?}"
        );
    }

    #[test]
    fn two_drivers_of_one_engine_are_never_reported_as_independent_engines() {
        let id = |engine: &str, driver: &str| EngineIdentity {
            engine: engine.into(),
            engine_version: "141".into(),
            driver: driver.into(),
            driver_version: "x".into(),
            network: "n".into(),
        };
        assert_eq!(
            independence(&[
                id("chromium", "playwright"),
                id("chrome", "webdriver:chromedriver")
            ]),
            Independence::SameEngineDifferentDriver
        );
        assert_eq!(
            independence(&[
                id("chromium", "playwright"),
                id("ladybird", "webdriver:ladybird")
            ]),
            Independence::IndependentEngines
        );
        assert_eq!(
            independence(&[id("chromium", "playwright"), id("chromium", "playwright")]),
            Independence::SameInstrument
        );
        assert_eq!(
            independence(&[id("chromium", "playwright"), id("mystery", "x")]),
            Independence::Unknown
        );
        let a = observed("chromium", 1.0 / 64.0, 666.0);
        let b = observed("ladybird", 1.0 / 64.0, 666.0);
        assert_eq!(
            compare_layout(&a, &b).unwrap().independence,
            Independence::IndependentEngines
        );
    }

    #[test]
    fn grid_structure_compares_track_counts_not_track_strings() {
        let with_grid = |tracks: &str| {
            let mut r = observed("chromium", 1.0 / 64.0, 666.640625);
            for element in &mut r.viewports[1].elements {
                if element.path == "body>section:1" {
                    element
                        .style
                        .insert("grid-template-columns".into(), tracks.into());
                }
            }
            r
        };
        let item = "grid-columns body>section:1 @1000";
        let same_count =
            compare_layout(&with_grid("300px 300px"), &with_grid("299.5px 300.5px")).unwrap();
        assert!(!same_count.disagreements.iter().any(|d| d.item == item));
        let differ =
            compare_layout(&with_grid("300px 300px"), &with_grid("200px 200px 200px")).unwrap();
        let found = differ
            .disagreements
            .iter()
            .find(|d| d.item == item)
            .unwrap();
        assert_eq!(found.values, vec![Some("2".into()), Some("3".into())]);
    }

    #[test]
    fn different_subjects_or_viewports_are_not_a_differential() {
        let a = observed("chromium", 1.0 / 64.0, 666.0);
        let mut b = observed("ladybird", 1.0 / 64.0, 666.0);
        b.subject.content_digest = IntegrityDigest::of_bytes(b"other");
        assert!(compare_layout(&a, &b).is_err());
        let mut c = observed("ladybird", 1.0 / 64.0, 666.0);
        c.viewports.pop();
        assert!(compare_layout(&a, &c).is_err());
    }

    #[test]
    fn breakpoints_agree_only_on_the_same_one_pixel_boundary() {
        let change = ResponsiveChange::FlexDirection {
            narrow: "column".into(),
            wide: "row".into(),
        };
        let digest = IntegrityDigest::of_bytes(b"s");
        let a = vec![("body>section:1".to_owned(), change.clone(), 699, 700)];
        let same = compare_breakpoints(digest.clone(), vec![], &a, &a.clone());
        assert!(same.semantic_agreement && same.agreements == 1);
        let b = vec![("body>section:1".to_owned(), change, 700, 701)];
        let off = compare_breakpoints(digest.clone(), vec![], &a, &b);
        assert_eq!(
            off.disagreements[0].values,
            vec![Some("(699, 700)".into()), Some("(700, 701)".into())]
        );
        let missing = compare_breakpoints(digest, vec![], &a, &[]);
        assert_eq!(missing.disagreements[0].values[1], None);
    }

    #[test]
    fn interactions_compare_abstract_mechanisms_and_transitions() {
        let digest = IntegrityDigest::of_bytes(b"s");
        let analysis = |props: &[&str]| InteractionAnalysis {
            mechanisms: vec![
                InteractionMechanism::HoverAffordance {
                    target: "body>a:1".into(),
                    properties: props.iter().map(|p| (*p).into()).collect(),
                },
                InteractionMechanism::Toggle {
                    target: "body>button:1".into(),
                },
            ],
            states: vec![],
            transitions: vec![InteractionTransition {
                from: "S0".into(),
                target: "body>button:1".into(),
                stimulus: Stimulus::Click,
                to: "S1".into(),
            }],
            status: None,
        };
        let same = compare_interactions(
            digest.clone(),
            vec![],
            &analysis(&["transform"]),
            &analysis(&["transform"]),
        );
        assert!(same.semantic_agreement);
        assert_eq!(same.agreements, 3);
        let differ = compare_interactions(
            digest,
            vec![],
            &analysis(&["transform"]),
            &analysis(&["transform", "box-shadow"]),
        );
        assert_eq!(differ.disagreements.len(), 1);
        assert!(
            differ.disagreements[0]
                .item
                .starts_with("mechanism HOVER_AFFORDANCE")
        );
    }
}
