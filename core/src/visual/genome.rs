//! Design genome: abstract mechanisms extracted from observed references, with provenance, and
//! their recombination into new design intents (ADR 0018).
//!
//! UNDERSTAND -> ABSTRACT -> RECOMBINE -> CREATE, never SCRAPE -> COPY
//! (`.atlas/contracts/CREATOR-FABRIC.md`): a mechanism records *relations* (small rational ratios
//! fitted within measurement uncertainty, a column change, a lift, an easing) plus where they were
//! observed -- never markup, text or assets. A recombination must draw on at least two distinct
//! sources and must not reproduce any single source mechanism's parameters wholesale; every
//! parameter of the resulting intent names the mechanism it came from or the recipe that varied it.

use super::{
    InteractionAnalysis, InteractionMechanism, InteractionObservation, MotionInference,
    ResponsiveChange, VisualAnalysis, VisualObservationReport,
    compose::{DesignIntent, GridIntent, HeroIntent},
};
use crate::{EpistemicStatus, IntegrityDigest};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Smallest-denominator fraction `p/q` (q <= `max_denominator`) within absolute `tolerance` of
/// `value`, found by trying each denominator in increasing order.
pub fn approximate_ratio(value: f64, tolerance: f64, max_denominator: u32) -> Option<(u32, u32)> {
    if !(value.is_finite() && value > 0.0) {
        return None;
    }
    (1..=max_denominator).find_map(|q| {
        let p = (value * f64::from(q)).round();
        (p >= 1.0 && (p / f64::from(q) - value).abs() <= tolerance).then_some((p as u32, q))
    })
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MechanismKind {
    /// A two-child flex row (media + copy) that stacks below a breakpoint.
    SplitHero,
    /// A grid whose column count collapses below a breakpoint.
    CollapsingGrid,
    /// Hovering lifts an element (translate) with a transition easing and duration.
    HoverLift,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MechanismSource {
    pub locator: String,
    pub subject_digest: IntegrityDigest,
    /// The observation/analysis records the mechanism was abstracted from.
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesignMechanism {
    /// `<kind>@<first 12 hex of the source digest>#<element>`.
    pub id: String,
    pub kind: MechanismKind,
    /// Relation parameters, e.g. `media_aspect = 16:9`, `breakpoint_px = 700`.
    pub parameters: BTreeMap<String, String>,
    pub source: MechanismSource,
    /// `DERIVED` for measured relations; `INFERRED` when a parameter comes from curve inference.
    pub status: EpistemicStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesignGenome {
    pub schema: String,
    pub mechanisms: Vec<DesignMechanism>,
}

fn short(digest: &IntegrityDigest) -> String {
    digest
        .as_str()
        .trim_start_matches(IntegrityDigest::BLAKE3_256_PREFIX)
        .chars()
        .take(12)
        .collect()
}

fn ratio_text((p, q): (u32, u32)) -> String {
    format!("{p}:{q}")
}

/// Relative tolerance for fitting measured ratios to small fractions: the 1/64 px layout grid
/// yields ~1e-4 on the observed element sizes; 2e-3 leaves margin without admitting neighbours
/// such as 16:9 vs 7:4 (1.7778 vs 1.75).
const RATIO_TOLERANCE: f64 = 2e-3;

/// Everything observed on one reference subject.
pub struct ReferenceEvidence<'a> {
    pub observation: &'a VisualObservationReport,
    pub analysis: &'a VisualAnalysis,
    /// Bisected breakpoints: (element, change, narrow_width, wide_width).
    pub breakpoints: &'a [(String, ResponsiveChange, u32, u32)],
    pub interactions: &'a InteractionAnalysis,
    pub interaction_observations: &'a [InteractionObservation],
    pub motion: &'a [MotionInference],
}

fn children_at<'a>(
    observation: &'a VisualObservationReport,
    width: u32,
    parent: &str,
) -> Vec<&'a super::ElementObservation> {
    observation
        .viewports
        .iter()
        .find(|v| v.width == width)
        .map(|v| {
            v.elements
                .iter()
                .filter(|e| e.parent.as_deref() == Some(parent))
                .collect()
        })
        .unwrap_or_default()
}

/// Abstracts mechanisms from one reference's evidence.
pub fn extract_mechanisms(evidence: &ReferenceEvidence<'_>) -> Vec<DesignMechanism> {
    let observation = evidence.observation;
    let digest = &observation.subject.content_digest;
    let wide = observation
        .viewports
        .iter()
        .map(|v| v.width)
        .max()
        .unwrap_or(0);
    let breakpoint = |element: &str, change: &ResponsiveChange| {
        evidence
            .breakpoints
            .iter()
            .find(|(e, c, _, _)| e == element && c == change)
            .map(|(_, _, _, hi)| *hi)
    };
    let source = |evidence: Vec<String>| MechanismSource {
        locator: observation.subject.locator.clone(),
        subject_digest: digest.clone(),
        evidence,
    };
    let mut mechanisms = Vec::new();
    for rule in &evidence.analysis.responsive_rules {
        match &rule.change {
            ResponsiveChange::FlexDirection {
                narrow,
                wide: wide_direction,
            } if narrow == "column" && wide_direction == "row" => {
                let children = children_at(observation, wide, &rule.element);
                let [media, copy] = children.as_slice() else {
                    continue;
                };
                let (Some(aspect), Some(share), Some(bp)) = (
                    approximate_ratio(media.width / media.height, RATIO_TOLERANCE * 2.0, 32),
                    approximate_ratio(media.width / copy.width, RATIO_TOLERANCE, 16),
                    breakpoint(&rule.element, &rule.change),
                ) else {
                    continue;
                };
                mechanisms.push(DesignMechanism {
                    id: format!("split-hero@{}#{}", short(digest), rule.element),
                    kind: MechanismKind::SplitHero,
                    parameters: BTreeMap::from([
                        ("media_aspect".into(), ratio_text(aspect)),
                        ("share".into(), ratio_text(share)),
                        ("breakpoint_px".into(), bp.to_string()),
                    ]),
                    source: source(vec![
                        format!("responsive rule FLEX_DIRECTION on {}", rule.element),
                        format!("bisected breakpoint {bp}px"),
                        format!("boxes of {} and {} at {wide}px", media.path, copy.path),
                    ]),
                    status: EpistemicStatus::Derived,
                });
            }
            ResponsiveChange::GridColumns {
                narrow,
                wide: columns,
            } if *narrow == 1 => {
                let children = children_at(observation, wide, &rule.element);
                let Some(item) = children.first() else {
                    continue;
                };
                let (Some(aspect), Some(bp)) = (
                    approximate_ratio(item.width / item.height, RATIO_TOLERANCE * 2.0, 32),
                    breakpoint(&rule.element, &rule.change),
                ) else {
                    continue;
                };
                mechanisms.push(DesignMechanism {
                    id: format!("collapsing-grid@{}#{}", short(digest), rule.element),
                    kind: MechanismKind::CollapsingGrid,
                    parameters: BTreeMap::from([
                        ("columns".into(), columns.to_string()),
                        ("items".into(), children.len().to_string()),
                        ("item_aspect".into(), ratio_text(aspect)),
                        ("breakpoint_px".into(), bp.to_string()),
                    ]),
                    source: source(vec![
                        format!(
                            "responsive rule GRID_COLUMNS 1<-{columns} on {}",
                            rule.element
                        ),
                        format!("bisected breakpoint {bp}px"),
                    ]),
                    status: EpistemicStatus::Derived,
                });
            }
            _ => {}
        }
    }
    for mechanism in &evidence.interactions.mechanisms {
        let InteractionMechanism::HoverAffordance { target, properties } = mechanism else {
            continue;
        };
        if !properties.contains(&"transform".to_owned()) {
            continue;
        }
        let lift = evidence
            .interaction_observations
            .iter()
            .find(|o| o.target == *target && o.stimulus == super::Stimulus::Hover)
            .and_then(|o| {
                o.changes
                    .iter()
                    .find(|c| c.element == *target && c.property == "transform")
            })
            .and_then(|change| {
                let numbers: Vec<f64> = change
                    .after
                    .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
                    .filter_map(|t| t.parse().ok())
                    .collect();
                // matrix(a, b, c, d, tx, ty): a pure upward translation lifts by -ty.
                (numbers.len() == 6 && numbers[5] < 0.0).then(|| -numbers[5])
            });
        let motion = evidence.motion.iter().find(|m| m.element == *target);
        let (Some(lift), Some(motion)) = (lift, motion) else {
            continue;
        };
        let Some(easing) = motion.inferred_easing.clone() else {
            continue;
        };
        mechanisms.push(DesignMechanism {
            id: format!("hover-lift@{}#{target}", short(digest)),
            kind: MechanismKind::HoverLift,
            parameters: BTreeMap::from([
                ("lift_px".into(), format!("{}", lift.round() as i64)),
                ("easing".into(), easing),
                (
                    "duration_ms".into(),
                    format!("{}", motion.duration_ms.round() as i64),
                ),
            ]),
            source: source(vec![
                format!("HOVER_AFFORDANCE on {target}"),
                "transform delta after settled hover".into(),
                format!("easing inferred from sampled curve of {target}"),
            ]),
            status: EpistemicStatus::Inferred,
        });
    }
    mechanisms
}

/// A request to recombine genome mechanisms into a new intent. `variations` override named
/// parameters (`hero.media_aspect`, `grid.columns`, `lift.duration_ms`, ...) with new values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Recombination {
    pub name: String,
    pub hero: String,
    pub grid: String,
    pub lift: String,
    #[serde(default)]
    pub variations: BTreeMap<String, String>,
}

/// Where one intent parameter came from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParameterProvenance {
    pub parameter: String,
    pub value: String,
    /// `inherited:<mechanism id>` or `varied:<recombination name>`.
    pub origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecombinedIntent {
    pub intent: DesignIntent,
    pub provenance: Vec<ParameterProvenance>,
    pub distinct_sources: usize,
    pub varied_parameters: usize,
}

fn parse_ratio(text: &str) -> Result<(u32, u32), String> {
    let (p, q) = text.split_once(':').ok_or(format!("`{text}` is not p:q"))?;
    Ok((
        p.trim().parse().map_err(|_| format!("`{text}`"))?,
        q.trim().parse().map_err(|_| format!("`{text}`"))?,
    ))
}

/// RECOMBINE: builds an intent from three genome mechanisms plus declared variations, refusing
/// recombinations that draw on fewer than two sources or that reproduce any source mechanism's
/// parameters wholesale (originality, `CREATOR-FABRIC.md`).
pub fn recombine(
    genome: &DesignGenome,
    recipe: &Recombination,
) -> Result<RecombinedIntent, String> {
    let find = |id: &str, kind: MechanismKind| {
        genome
            .mechanisms
            .iter()
            .find(|m| m.id == id && m.kind == kind)
            .ok_or_else(|| format!("no {kind:?} mechanism `{id}` in the genome"))
    };
    let hero = find(&recipe.hero, MechanismKind::SplitHero)?;
    let grid = find(&recipe.grid, MechanismKind::CollapsingGrid)?;
    let lift = find(&recipe.lift, MechanismKind::HoverLift)?;
    let sources: BTreeSet<&str> = [hero, grid, lift]
        .iter()
        .map(|m| m.source.subject_digest.as_str())
        .collect();
    if sources.len() < 2 {
        return Err("a recombination must draw on at least two distinct sources".into());
    }
    let mut provenance = Vec::new();
    let mut varied_per_mechanism: BTreeMap<&str, usize> = BTreeMap::new();
    let mut value =
        |group: &str, name: &str, mechanism: &DesignMechanism| -> Result<String, String> {
            let key = format!("{group}.{name}");
            let (value, origin) = match recipe.variations.get(&key) {
                Some(varied) => {
                    *varied_per_mechanism.entry(group_of(group)).or_default() += 1;
                    (varied.clone(), format!("varied:{}", recipe.name))
                }
                None => (
                    mechanism
                        .parameters
                        .get(name)
                        .cloned()
                        .ok_or_else(|| format!("{} has no `{name}`", mechanism.id))?,
                    format!("inherited:{}", mechanism.id),
                ),
            };
            provenance.push(ParameterProvenance {
                parameter: key,
                value: value.clone(),
                origin,
            });
            Ok(value)
        };
    fn group_of(group: &str) -> &'static str {
        match group {
            "hero" => "hero",
            "grid" => "grid",
            _ => "lift",
        }
    }
    let media_aspect = parse_ratio(&value("hero", "media_aspect", hero)?)?;
    let share = parse_ratio(&value("hero", "share", hero)?)?;
    let breakpoint: u32 = value("hero", "breakpoint_px", hero)?
        .parse()
        .map_err(|_| "hero.breakpoint_px".to_owned())?;
    let columns: u32 = value("grid", "columns", grid)?
        .parse()
        .map_err(|_| "grid.columns".to_owned())?;
    let items: u32 = value("grid", "items", grid)?
        .parse()
        .map_err(|_| "grid.items".to_owned())?;
    let item_aspect = parse_ratio(&value("grid", "item_aspect", grid)?)?;
    let lift_px: u32 = value("lift", "lift_px", lift)?
        .parse()
        .map_err(|_| "lift.lift_px".to_owned())?;
    let easing = value("lift", "easing", lift)?;
    let duration: u32 = value("lift", "duration_ms", lift)?
        .parse()
        .map_err(|_| "lift.duration_ms".to_owned())?;
    for group in ["hero", "grid", "lift"] {
        if varied_per_mechanism.get(group).copied().unwrap_or(0) == 0 {
            return Err(format!(
                "originality: the {group} mechanism would be reproduced wholesale; vary at least one of its parameters"
            ));
        }
    }
    let intent = DesignIntent {
        name: recipe.name.clone(),
        root_font_px: 16,
        breakpoint_px: breakpoint,
        hero: HeroIntent {
            media_aspect,
            share,
            gap_em: 1.5,
            copy_font_ratio: 1.25,
        },
        grid: GridIntent {
            columns,
            items,
            item_aspect,
            gap_em: 1.0,
            hover_lift_px: lift_px,
            hover_easing: easing,
            hover_duration_ms: duration,
        },
    };
    intent.validate()?;
    for (parameter, value) in [
        ("root_font_px", "16"),
        ("hero.gap_em", "1.5"),
        ("hero.copy_font_ratio", "1.25"),
        ("grid.gap_em", "1"),
    ] {
        provenance.push(ParameterProvenance {
            parameter: parameter.into(),
            value: value.into(),
            origin: "default".into(),
        });
    }
    let varied_parameters = provenance
        .iter()
        .filter(|p| p.origin.starts_with("varied:"))
        .count();
    Ok(RecombinedIntent {
        intent,
        provenance,
        distinct_sources: sources.len(),
        varied_parameters,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratios_are_fitted_to_the_smallest_fraction_within_tolerance() {
        assert_eq!(
            approximate_ratio(16.0 / 9.0 + 5e-5, 2e-3, 32),
            Some((16, 9))
        );
        assert_eq!(approximate_ratio(1.3333, 2e-3, 32), Some((4, 3)));
        assert_eq!(approximate_ratio(2.0, 1e-6, 16), Some((2, 1)));
        assert_eq!(
            approximate_ratio(0.6180339887, 1e-6, 16),
            None,
            "irrational: no small fraction"
        );
        assert_eq!(
            approximate_ratio(1.75, 2e-3, 32),
            Some((7, 4)),
            "not confused with 16:9"
        );
        assert_eq!(approximate_ratio(f64::NAN, 1e-3, 8), None);
    }

    fn mechanism(
        kind: MechanismKind,
        id: &str,
        digest: &[u8],
        parameters: &[(&str, &str)],
    ) -> DesignMechanism {
        DesignMechanism {
            id: id.into(),
            kind,
            parameters: parameters
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
            source: MechanismSource {
                locator: "ref".into(),
                subject_digest: IntegrityDigest::of_bytes(digest),
                evidence: vec![],
            },
            status: EpistemicStatus::Derived,
        }
    }

    fn genome() -> DesignGenome {
        DesignGenome {
            schema: "atlas.design-genome.v1".into(),
            mechanisms: vec![
                mechanism(
                    MechanismKind::SplitHero,
                    "hero-a",
                    b"A",
                    &[
                        ("media_aspect", "16:9"),
                        ("share", "2:1"),
                        ("breakpoint_px", "700"),
                    ],
                ),
                mechanism(
                    MechanismKind::CollapsingGrid,
                    "grid-a",
                    b"A",
                    &[
                        ("columns", "3"),
                        ("items", "3"),
                        ("item_aspect", "4:3"),
                        ("breakpoint_px", "700"),
                    ],
                ),
                mechanism(
                    MechanismKind::HoverLift,
                    "lift-b",
                    b"B",
                    &[
                        ("lift_px", "4"),
                        ("easing", "ease-out"),
                        ("duration_ms", "150"),
                    ],
                ),
            ],
        }
    }

    fn recipe(variations: &[(&str, &str)]) -> Recombination {
        Recombination {
            name: "r".into(),
            hero: "hero-a".into(),
            grid: "grid-a".into(),
            lift: "lift-b".into(),
            variations: variations
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
        }
    }

    #[test]
    fn recombination_tracks_every_parameter_origin() {
        let recombined = recombine(
            &genome(),
            &recipe(&[
                ("hero.breakpoint_px", "820"),
                ("grid.columns", "4"),
                ("grid.items", "4"),
                ("lift.duration_ms", "220"),
            ]),
        )
        .unwrap();
        assert_eq!(recombined.distinct_sources, 2);
        assert_eq!(recombined.intent.hero.media_aspect, (16, 9));
        assert_eq!(recombined.intent.grid.columns, 4);
        assert_eq!(recombined.intent.grid.hover_easing, "ease-out");
        let origin = |p: &str| {
            recombined
                .provenance
                .iter()
                .find(|x| x.parameter == p)
                .unwrap()
                .origin
                .clone()
        };
        assert_eq!(origin("hero.media_aspect"), "inherited:hero-a");
        assert_eq!(origin("grid.columns"), "varied:r");
        assert_eq!(origin("lift.easing"), "inherited:lift-b");
        assert_eq!(recombined.varied_parameters, 4);
    }

    #[test]
    fn copying_a_mechanism_wholesale_or_from_one_source_is_refused() {
        let err = recombine(
            &genome(),
            &recipe(&[("hero.breakpoint_px", "820"), ("grid.columns", "4")]),
        )
        .unwrap_err();
        assert!(
            err.contains("lift mechanism would be reproduced wholesale"),
            "{err}"
        );
        let mut single = genome();
        single.mechanisms[2].source.subject_digest = IntegrityDigest::of_bytes(b"A");
        let err = recombine(
            &single,
            &recipe(&[
                ("hero.breakpoint_px", "820"),
                ("grid.columns", "4"),
                ("lift.lift_px", "6"),
            ]),
        )
        .unwrap_err();
        assert!(err.contains("two distinct sources"), "{err}");
        let err = recombine(
            &genome(),
            &Recombination {
                hero: "missing".into(),
                ..recipe(&[])
            },
        )
        .unwrap_err();
        assert!(err.contains("no SplitHero mechanism"), "{err}");
    }
}
