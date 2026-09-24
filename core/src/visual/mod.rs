//! Creator Fabric: visual observation records and ratio-first layout semantics (ADR 0011).
//!
//! A `VisualObservationReport` is what a sandboxed browser instrument measured on one
//! authorized subject (a local fixture), at one or more viewports: per element, its box, a fixed
//! subset of computed style, and its structural path. Every measured value is `OBSERVED` from the
//! instrument, never read from the page's source code -- the report says what was rendered, not
//! how it was implemented.
//!
//! `analyze` derives transferable semantics from those measurements under named rules, each
//! `DERIVED`:
//! - `LayoutRelation`: normalized ratios (width/viewport, width/parent, aspect ratio,
//!   font-size/root font-size, gap/font-size) with an uncertainty bound propagated from the
//!   instrument's layout resolution;
//! - `ResponsiveRule`: a structural change between two observed viewport widths (flex direction,
//!   grid column count, siblings stacking, display), stated as the width interval in which the
//!   change occurs -- never as an exact breakpoint the instrument did not measure.

pub mod compose;

use crate::{EpistemicStatus, IntegrityDigest};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const VISUAL_OBSERVATION_SCHEMA: &str = "atlas.visual-observation.v1";

/// Computed-style properties the instrument records for every element. Fixed and ordered so two
/// observations of the same subject are comparable.
pub const OBSERVED_STYLE_PROPERTIES: [&str; 18] = [
    "display",
    "position",
    "flex-direction",
    "flex-wrap",
    "justify-content",
    "align-items",
    "grid-template-columns",
    "row-gap",
    "column-gap",
    "font-size",
    "font-weight",
    "line-height",
    "border-top-left-radius",
    "opacity",
    "transform",
    "transition-duration",
    "animation-name",
    "visibility",
];

/// Chromium lays out in 1/64 px units; every box edge is exact to that resolution.
pub const LAYOUT_RESOLUTION_PX: f64 = 1.0 / 64.0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VisualSubject {
    /// The subject as given to the instrument (a local fixture path).
    pub locator: String,
    /// BLAKE3-256 of the exact bytes the instrument loaded.
    pub content_digest: IntegrityDigest,
    /// Why Atlas may observe it. v1 admits only `LOCAL_FIXTURE`.
    pub authorization: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObservationInstrument {
    pub engine: String,
    pub engine_version: String,
    pub driver: String,
    pub driver_version: String,
    /// Network policy in force: every request other than the subject itself is aborted.
    pub network: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElementObservation {
    /// Structural identity, stable across viewports: `body>main:1>section:2` (tag and 1-based
    /// index among same-tag siblings).
    pub path: String,
    pub parent: Option<String>,
    pub tag: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub style: BTreeMap<String, String>,
    /// Number of direct text characters (whitespace-trimmed), a content-free size signal.
    pub text_chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewportObservation {
    pub width: u32,
    pub height: u32,
    pub document_width: f64,
    pub document_height: f64,
    pub root_font_size_px: f64,
    pub elements: Vec<ElementObservation>,
    /// True when the instrument's element cap was reached; accounting is then incomplete.
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualObservationReport {
    pub schema: String,
    pub status: EpistemicStatus,
    pub subject: VisualSubject,
    pub instrument: ObservationInstrument,
    pub viewports: Vec<ViewportObservation>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LayoutRelationKind {
    WidthOverViewport,
    WidthOverParent,
    AspectRatio,
    FontSizeOverRoot,
    GapOverFontSize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayoutRelation {
    pub element: String,
    pub viewport_width: u32,
    pub relation: LayoutRelationKind,
    pub value: f64,
    /// Worst-case absolute error of `value` from the instrument's layout resolution.
    pub uncertainty: f64,
    pub status: EpistemicStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "change")]
pub enum ResponsiveChange {
    FlexDirection {
        narrow: String,
        wide: String,
    },
    GridColumns {
        narrow: usize,
        wide: usize,
    },
    /// `first` and `second` are consecutive children: side by side at the wider viewport,
    /// `second` below `first` at the narrower one.
    SiblingsStack {
        first: String,
        second: String,
    },
    Display {
        narrow: String,
        wide: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResponsiveRule {
    pub element: String,
    /// The change happens at some width in `(narrower, wider]`; the instrument observed only
    /// these two widths.
    pub narrower: u32,
    pub wider: u32,
    pub change: ResponsiveChange,
    pub status: EpistemicStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct VisualAnalysis {
    pub relations: Vec<LayoutRelation>,
    pub responsive_rules: Vec<ResponsiveRule>,
}

fn px(value: Option<&String>) -> Option<f64> {
    value?.strip_suffix("px")?.trim().parse::<f64>().ok()
}

/// Error bound of `numerator / denominator` when each is exact only to `LAYOUT_RESOLUTION_PX`.
fn ratio_uncertainty(numerator: f64, denominator: f64) -> f64 {
    let e = LAYOUT_RESOLUTION_PX;
    ((numerator + e) / (denominator - e) - numerator / denominator)
        .abs()
        .max((numerator / denominator - (numerator - e) / (denominator + e)).abs())
}

/// Grid column count from a computed `grid-template-columns` value (`none` = no grid).
fn grid_columns(value: &str) -> Option<usize> {
    if value == "none" || value.is_empty() {
        return None;
    }
    // Computed values resolve to a space-separated track list, e.g. `300px 300px` or
    // `[a] 1fr [b] 2fr`; named-line brackets are not tracks.
    let mut depth = 0;
    let mut tracks = 0;
    let mut in_token = false;
    for ch in value.chars() {
        match ch {
            '[' => depth += 1,
            ']' => depth -= 1,
            '(' => {
                depth += 1;
                in_token = true;
            }
            ')' => depth -= 1,
            c if c.is_whitespace() && depth == 0 => {
                if in_token {
                    tracks += 1;
                }
                in_token = false;
            }
            _ if depth == 0 => in_token = true,
            _ => {}
        }
    }
    if in_token {
        tracks += 1;
    }
    Some(tracks)
}

fn is_rendered(element: &ElementObservation) -> bool {
    element.width > 0.0
        && element.height > 0.0
        && element.style.get("display").map(String::as_str) != Some("none")
}

fn push_ratio(
    relations: &mut Vec<LayoutRelation>,
    viewport_width: u32,
    element: &str,
    relation: LayoutRelationKind,
    numerator: f64,
    denominator: f64,
) {
    if denominator > LAYOUT_RESOLUTION_PX {
        relations.push(LayoutRelation {
            element: element.to_owned(),
            viewport_width,
            relation,
            value: numerator / denominator,
            uncertainty: ratio_uncertainty(numerator, denominator),
            status: EpistemicStatus::Derived,
        });
    }
}

/// Relations for one viewport's rendered elements.
fn relations_at(viewport: &ViewportObservation) -> Vec<LayoutRelation> {
    let by_path: BTreeMap<&str, &ElementObservation> = viewport
        .elements
        .iter()
        .map(|element| (element.path.as_str(), element))
        .collect();
    let mut relations = Vec::new();
    for element in viewport.elements.iter().filter(|e| is_rendered(e)) {
        push_ratio(
            &mut relations,
            viewport.width,
            &element.path,
            LayoutRelationKind::WidthOverViewport,
            element.width,
            f64::from(viewport.width),
        );
        if let Some(parent) = element.parent.as_deref().and_then(|p| by_path.get(p))
            && is_rendered(parent)
        {
            push_ratio(
                &mut relations,
                viewport.width,
                &element.path,
                LayoutRelationKind::WidthOverParent,
                element.width,
                parent.width,
            );
        }
        push_ratio(
            &mut relations,
            viewport.width,
            &element.path,
            LayoutRelationKind::AspectRatio,
            element.width,
            element.height,
        );
        let font_size = px(element.style.get("font-size"));
        if element.text_chars > 0
            && let Some(font_size) = font_size
        {
            // Font sizes are computed values, not layout boxes: exact, so only the ratio itself.
            relations.push(LayoutRelation {
                element: element.path.clone(),
                viewport_width: viewport.width,
                relation: LayoutRelationKind::FontSizeOverRoot,
                value: font_size / viewport.root_font_size_px,
                uncertainty: 0.0,
                status: EpistemicStatus::Derived,
            });
        }
        let is_container = matches!(
            element.style.get("display").map(String::as_str),
            Some("flex" | "inline-flex" | "grid" | "inline-grid")
        );
        if is_container
            && let (Some(gap), Some(font_size)) = (px(element.style.get("column-gap")), font_size)
            && gap > 0.0
        {
            relations.push(LayoutRelation {
                element: element.path.clone(),
                viewport_width: viewport.width,
                relation: LayoutRelationKind::GapOverFontSize,
                value: gap / font_size,
                uncertainty: 0.0,
                status: EpistemicStatus::Derived,
            });
        }
    }
    relations
}

fn side_by_side(first: &ElementObservation, second: &ElementObservation) -> bool {
    let e = LAYOUT_RESOLUTION_PX;
    second.x >= first.x + first.width - e && second.y < first.y + first.height - e
}

fn stacked(first: &ElementObservation, second: &ElementObservation) -> bool {
    second.y >= first.y + first.height - LAYOUT_RESOLUTION_PX
}

/// Structural changes between two viewports (`narrow.width < wide.width`).
fn rules_between(narrow: &ViewportObservation, wide: &ViewportObservation) -> Vec<ResponsiveRule> {
    let wide_by_path: BTreeMap<&str, &ElementObservation> = wide
        .elements
        .iter()
        .map(|element| (element.path.as_str(), element))
        .collect();
    let mut rules = Vec::new();
    let mut rule = |element: &str, change| {
        rules.push(ResponsiveRule {
            element: element.to_owned(),
            narrower: narrow.width,
            wider: wide.width,
            change,
            status: EpistemicStatus::Derived,
        })
    };
    for element in &narrow.elements {
        let Some(other) = wide_by_path.get(element.path.as_str()) else {
            continue;
        };
        let style =
            |e: &ElementObservation, key: &str| e.style.get(key).cloned().unwrap_or_default();
        if style(element, "display") != style(other, "display") {
            rule(
                &element.path,
                ResponsiveChange::Display {
                    narrow: style(element, "display"),
                    wide: style(other, "display"),
                },
            );
            continue;
        }
        let is_flex = matches!(style(element, "display").as_str(), "flex" | "inline-flex");
        if is_flex && style(element, "flex-direction") != style(other, "flex-direction") {
            rule(
                &element.path,
                ResponsiveChange::FlexDirection {
                    narrow: style(element, "flex-direction"),
                    wide: style(other, "flex-direction"),
                },
            );
        }
        if let (Some(narrow_columns), Some(wide_columns)) = (
            grid_columns(&style(element, "grid-template-columns")),
            grid_columns(&style(other, "grid-template-columns")),
        ) && narrow_columns != wide_columns
        {
            rule(
                &element.path,
                ResponsiveChange::GridColumns {
                    narrow: narrow_columns,
                    wide: wide_columns,
                },
            );
        }
    }
    // Consecutive rendered siblings that sit side by side when wide and stack when narrow.
    let mut children: BTreeMap<&str, Vec<&ElementObservation>> = BTreeMap::new();
    for element in narrow.elements.iter().filter(|e| is_rendered(e)) {
        if let Some(parent) = element.parent.as_deref() {
            children.entry(parent).or_default().push(element);
        }
    }
    for (parent, siblings) in children {
        for pair in siblings.windows(2) {
            let (Some(first_wide), Some(second_wide)) = (
                wide_by_path.get(pair[0].path.as_str()),
                wide_by_path.get(pair[1].path.as_str()),
            ) else {
                continue;
            };
            if side_by_side(first_wide, second_wide) && stacked(pair[0], pair[1]) {
                rule(
                    parent,
                    ResponsiveChange::SiblingsStack {
                        first: pair[0].path.clone(),
                        second: pair[1].path.clone(),
                    },
                );
            }
        }
    }
    rules
}

/// Derives layout relations at every observed viewport and responsive rules between every pair
/// of adjacent observed widths.
pub fn analyze(report: &VisualObservationReport) -> VisualAnalysis {
    let mut viewports: Vec<&ViewportObservation> = report.viewports.iter().collect();
    viewports.sort_by_key(|viewport| viewport.width);
    let mut analysis = VisualAnalysis::default();
    for viewport in &viewports {
        analysis.relations.extend(relations_at(viewport));
    }
    for pair in viewports.windows(2) {
        if pair[0].width < pair[1].width {
            analysis
                .responsive_rules
                .extend(rules_between(pair[0], pair[1]));
        }
    }
    analysis
}

impl VisualAnalysis {
    pub fn relation(
        &self,
        element: &str,
        viewport_width: u32,
        relation: LayoutRelationKind,
    ) -> Option<&LayoutRelation> {
        self.relations.iter().find(|candidate| {
            candidate.element == element
                && candidate.viewport_width == viewport_width
                && candidate.relation == relation
        })
    }
}

/// A stimulus the instrument applies to one element (ADR 0015).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Stimulus {
    Hover,
    Click,
    ClickTwice,
    Focus,
}

/// One property of one element that differed from the pre-stimulus baseline after the page
/// settled (all animations finished). `before` is `None` when the property was absent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObservedChange {
    pub element: String,
    pub property: String,
    pub before: Option<String>,
    pub after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InteractionObservation {
    pub target: String,
    pub stimulus: Stimulus,
    /// Animations that were running and awaited before measuring.
    pub animations: usize,
    pub changes: Vec<ObservedChange>,
    pub status: EpistemicStatus,
}

/// An abstract interaction mechanism classified from observed changes under a named rule --
/// what the element does, never how its source implements it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "mechanism")]
pub enum InteractionMechanism {
    /// Hover changes the target's own presentation (transform, shadow, colour, opacity).
    HoverAffordance {
        target: String,
        properties: Vec<String>,
    },
    /// Click shows/hides or opens/expands elements (`controlled`).
    Disclosure {
        target: String,
        controlled: Vec<String>,
    },
    /// A second click returns every non-focus property to the baseline.
    Toggle { target: String },
    /// Keyboard focus changes the target's presentation beyond focus itself.
    FocusIndicator {
        target: String,
        properties: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InteractionState {
    pub id: String,
    /// Non-focus changes relative to the baseline state `S0`.
    pub changes: Vec<ObservedChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InteractionTransition {
    pub from: String,
    pub target: String,
    pub stimulus: Stimulus,
    pub to: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct InteractionAnalysis {
    pub mechanisms: Vec<InteractionMechanism>,
    pub states: Vec<InteractionState>,
    pub transitions: Vec<InteractionTransition>,
    pub status: Option<EpistemicStatus>,
}

const PRESENTATION: [&str; 5] = [
    "transform",
    "box-shadow",
    "background-color",
    "color",
    "opacity",
];
const VISIBILITY: [&str; 5] = [
    "display",
    "visibility",
    "height",
    "attr:open",
    "attr:aria-hidden",
];

fn within(element: &str, target: &str) -> bool {
    element == target || element.starts_with(&format!("{target}>"))
}

fn meaningful(changes: &[ObservedChange]) -> Vec<ObservedChange> {
    let mut kept: Vec<ObservedChange> = changes
        .iter()
        .filter(|change| change.property != "focused")
        .cloned()
        .collect();
    kept.sort();
    kept
}

/// Classifies mechanisms and builds the interaction state graph (`S0` = baseline; one state per
/// distinct settled change-set). Everything here is `DERIVED` from `OBSERVED` changes.
pub fn analyze_interactions(observations: &[InteractionObservation]) -> InteractionAnalysis {
    let mut analysis = InteractionAnalysis {
        states: vec![InteractionState {
            id: "S0".into(),
            changes: Vec::new(),
        }],
        status: Some(EpistemicStatus::Derived),
        ..Default::default()
    };
    let state_of = |changes: Vec<ObservedChange>, states: &mut Vec<InteractionState>| {
        if let Some(existing) = states.iter().find(|state| state.changes == changes) {
            return existing.id.clone();
        }
        let id = format!("S{}", states.len());
        states.push(InteractionState {
            id: id.clone(),
            changes,
        });
        id
    };
    let observation = |target: &str, stimulus| {
        observations
            .iter()
            .find(|o| o.target == target && o.stimulus == stimulus)
    };
    let mut targets: Vec<&str> = observations.iter().map(|o| o.target.as_str()).collect();
    targets.sort();
    targets.dedup();
    for target in targets {
        if let Some(hover) = observation(target, Stimulus::Hover) {
            let mut properties: Vec<String> = hover
                .changes
                .iter()
                .filter(|c| {
                    within(&c.element, target) && PRESENTATION.contains(&c.property.as_str())
                })
                .map(|c| c.property.clone())
                .collect();
            properties.sort();
            properties.dedup();
            if !properties.is_empty() {
                analysis
                    .mechanisms
                    .push(InteractionMechanism::HoverAffordance {
                        target: target.into(),
                        properties,
                    });
            }
        }
        if let Some(click) = observation(target, Stimulus::Click) {
            let changes = meaningful(&click.changes);
            let mut candidates: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
            for c in changes
                .iter()
                .filter(|c| VISIBILITY.contains(&c.property.as_str()) && c.element != target)
            {
                candidates
                    .entry(c.element.as_str())
                    .or_default()
                    .push(c.property.as_str());
            }
            // An ancestor whose only change is its height is reflow caused by a controlled
            // descendant appearing, not a controlled element itself (observed on real pages:
            // `body` grows when a panel is disclosed).
            let controlled: Vec<String> = candidates
                .iter()
                .filter(|(element, properties)| {
                    let reflow_only = properties.iter().all(|p| *p == "height");
                    let is_ancestor = candidates
                        .keys()
                        .any(|other| other != *element && within(other, element));
                    !(reflow_only && is_ancestor)
                })
                .map(|(element, _)| (*element).to_owned())
                .collect();
            let expands = changes
                .iter()
                .any(|c| c.element == target && c.property == "attr:aria-expanded");
            if !controlled.is_empty() || expands {
                analysis.mechanisms.push(InteractionMechanism::Disclosure {
                    target: target.into(),
                    controlled,
                });
            }
            if !changes.is_empty() {
                let to = state_of(changes, &mut analysis.states);
                analysis.transitions.push(InteractionTransition {
                    from: "S0".into(),
                    target: target.into(),
                    stimulus: Stimulus::Click,
                    to: to.clone(),
                });
                if let Some(twice) = observation(target, Stimulus::ClickTwice)
                    && meaningful(&twice.changes).is_empty()
                {
                    analysis.mechanisms.push(InteractionMechanism::Toggle {
                        target: target.into(),
                    });
                    analysis.transitions.push(InteractionTransition {
                        from: to,
                        target: target.into(),
                        stimulus: Stimulus::Click,
                        to: "S0".into(),
                    });
                }
            }
        }
        if let Some(focus) = observation(target, Stimulus::Focus) {
            let mut properties: Vec<String> = meaningful(&focus.changes)
                .into_iter()
                .filter(|c| c.element == target)
                .map(|c| c.property)
                .collect();
            properties.dedup();
            if !properties.is_empty() {
                analysis
                    .mechanisms
                    .push(InteractionMechanism::FocusIndicator {
                        target: target.into(),
                        properties,
                    });
            }
        }
    }
    analysis
}

/// The Web Animations timing the engine reports for an animation -- declared by the page,
/// `OBSERVED` by the instrument, and used as ground truth to validate curve inference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeclaredTiming {
    pub duration_ms: f64,
    pub delay_ms: f64,
    pub easing: String,
}

/// One animation's property curve sampled at deterministic seek times (ADR 0016).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MotionObservation {
    pub target: String,
    pub stimulus: Stimulus,
    pub element: String,
    pub property: Option<String>,
    pub declared: DeclaredTiming,
    /// `(time_ms, computed value)` pairs across the active interval.
    pub samples: Vec<(f64, String)>,
    pub status: EpistemicStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EasingCandidate {
    pub name: String,
    pub control_points: [f64; 4],
    /// Root-mean-square distance between the sampled progress and this curve.
    pub rms_error: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MotionInference {
    pub element: String,
    pub property: Option<String>,
    /// Active duration spanned by the samples.
    pub duration_ms: f64,
    /// The best-fitting candidate; `None` when the curve fits none within tolerance.
    pub inferred_easing: Option<String>,
    /// Every candidate, best first -- the alternatives are part of the inference.
    pub candidates: Vec<EasingCandidate>,
    /// Progress left [0, 1] at some sample: a spring or elastic curve no cubic keyword can model.
    pub overshoot: bool,
    /// Whether the inference matches the declared easing, when one is declared.
    pub agrees_with_declared: Option<bool>,
    pub status: EpistemicStatus,
}

/// The CSS keyword easings (CSS Easing Functions Level 1).
pub const KEYWORD_EASINGS: [(&str, [f64; 4]); 5] = [
    ("linear", [0.0, 0.0, 1.0, 1.0]),
    ("ease", [0.25, 0.1, 0.25, 1.0]),
    ("ease-in", [0.42, 0.0, 1.0, 1.0]),
    ("ease-out", [0.0, 0.0, 0.58, 1.0]),
    ("ease-in-out", [0.42, 0.0, 0.58, 1.0]),
];

/// Maximum RMS progress error for a candidate to be accepted as the inferred easing.
pub const EASING_TOLERANCE: f64 = 0.01;

/// `y` of the cubic Bezier through (0,0), (x1,y1), (x2,y2), (1,1) at abscissa `x`, found by
/// bisection on the monotone `x(t)` (x1, x2 in [0, 1]).
pub fn cubic_bezier(control: [f64; 4], x: f64) -> f64 {
    let [x1, y1, x2, y2] = control;
    let coordinate = |a: f64, b: f64, t: f64| {
        let u = 1.0 - t;
        3.0 * u * u * t * a + 3.0 * u * t * t * b + t * t * t
    };
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        if coordinate(x1, x2, mid) < x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    coordinate(y1, y2, (lo + hi) / 2.0)
}

fn numbers(value: &str) -> Vec<f64> {
    value
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == 'e'))
        .filter_map(|token| token.parse::<f64>().ok())
        .collect()
}

/// Normalized progress per sample of the numeric component that moves the most.
fn progress(samples: &[(f64, String)]) -> Option<Vec<f64>> {
    let vectors: Vec<Vec<f64>> = samples.iter().map(|(_, v)| numbers(v)).collect();
    let width = vectors.first()?.len();
    if width == 0 || vectors.iter().any(|v| v.len() != width) {
        return None;
    }
    let (first, last) = (vectors.first()?, vectors.last()?);
    let component = (0..width).max_by(|a, b| {
        (last[*a] - first[*a])
            .abs()
            .total_cmp(&(last[*b] - first[*b]).abs())
    })?;
    let span = last[component] - first[component];
    if span.abs() < 1e-9 {
        return None;
    }
    Some(
        vectors
            .iter()
            .map(|v| (v[component] - first[component]) / span)
            .collect(),
    )
}

/// Infers each sampled animation's easing from its curve alone and ranks every keyword
/// candidate; `INFERRED`, with the declared easing (when present) as validation.
pub fn infer_motion(observations: &[MotionObservation]) -> Vec<MotionInference> {
    observations
        .iter()
        .filter_map(|observation| {
            let progress = progress(&observation.samples)?;
            let steps = (progress.len() - 1) as f64;
            let mut candidates: Vec<EasingCandidate> = KEYWORD_EASINGS
                .iter()
                .map(|(name, control)| {
                    let squared: f64 = progress
                        .iter()
                        .enumerate()
                        .map(|(k, p)| (p - cubic_bezier(*control, k as f64 / steps)).powi(2))
                        .sum();
                    EasingCandidate {
                        name: (*name).into(),
                        control_points: *control,
                        rms_error: (squared / progress.len() as f64).sqrt(),
                    }
                })
                .collect();
            candidates.sort_by(|a, b| a.rms_error.total_cmp(&b.rms_error));
            let overshoot = progress.iter().any(|p| *p < -1e-6 || *p > 1.0 + 1e-6);
            let inferred_easing = candidates
                .first()
                .filter(|best| best.rms_error <= EASING_TOLERANCE && !overshoot)
                .map(|best| best.name.clone());
            let (first, last) = (observation.samples.first()?, observation.samples.last()?);
            Some(MotionInference {
                element: observation.element.clone(),
                property: observation.property.clone(),
                duration_ms: last.0 - first.0,
                agrees_with_declared: inferred_easing
                    .as_ref()
                    .map(|easing| *easing == observation.declared.easing),
                inferred_easing,
                candidates,
                overshoot,
                status: EpistemicStatus::Inferred,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(
        path: &str,
        parent: Option<&str>,
        rect: [f64; 4],
        style: &[(&str, &str)],
    ) -> ElementObservation {
        ElementObservation {
            path: path.into(),
            parent: parent.map(Into::into),
            tag: path
                .rsplit('>')
                .next()
                .unwrap()
                .split(':')
                .next()
                .unwrap()
                .into(),
            x: rect[0],
            y: rect[1],
            width: rect[2],
            height: rect[3],
            style: style
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
            text_chars: 0,
        }
    }

    fn viewport(width: u32, elements: Vec<ElementObservation>) -> ViewportObservation {
        ViewportObservation {
            width,
            height: 800,
            document_width: f64::from(width),
            document_height: 800.0,
            root_font_size_px: 16.0,
            elements,
            truncated: false,
        }
    }

    fn report(viewports: Vec<ViewportObservation>) -> VisualObservationReport {
        VisualObservationReport {
            schema: VISUAL_OBSERVATION_SCHEMA.into(),
            status: EpistemicStatus::Observed,
            subject: VisualSubject {
                locator: "fixture.html".into(),
                content_digest: IntegrityDigest::of_bytes(b"fixture"),
                authorization: "LOCAL_FIXTURE".into(),
            },
            instrument: ObservationInstrument {
                engine: "chromium".into(),
                engine_version: "test".into(),
                driver: "playwright".into(),
                driver_version: "test".into(),
                network: "BLOCKED_EXCEPT_SUBJECT".into(),
            },
            viewports,
        }
    }

    /// A row of two cards at 1000 px (flex row, 2:1) that stacks at 400 px (flex column).
    fn row_then_stack() -> VisualObservationReport {
        let row = [
            ("display", "flex"),
            ("flex-direction", "row"),
            ("column-gap", "16px"),
            ("font-size", "16px"),
        ];
        let column = [
            ("display", "flex"),
            ("flex-direction", "column"),
            ("column-gap", "16px"),
            ("font-size", "16px"),
        ];
        report(vec![
            viewport(
                1000,
                vec![
                    element("body>main:1", Some("body"), [0.0, 0.0, 960.0, 300.0], &row),
                    element(
                        "body>main:1>div:1",
                        Some("body>main:1"),
                        [0.0, 0.0, 640.0, 360.0],
                        &[("display", "block")],
                    ),
                    element(
                        "body>main:1>div:2",
                        Some("body>main:1"),
                        [656.0, 0.0, 304.0, 360.0],
                        &[("display", "block")],
                    ),
                ],
            ),
            viewport(
                400,
                vec![
                    element(
                        "body>main:1",
                        Some("body"),
                        [0.0, 0.0, 400.0, 600.0],
                        &column,
                    ),
                    element(
                        "body>main:1>div:1",
                        Some("body>main:1"),
                        [0.0, 0.0, 400.0, 225.0],
                        &[("display", "block")],
                    ),
                    element(
                        "body>main:1>div:2",
                        Some("body>main:1"),
                        [0.0, 241.0, 400.0, 100.0],
                        &[("display", "block")],
                    ),
                ],
            ),
        ])
    }

    #[test]
    fn ratios_are_derived_with_resolution_bounded_uncertainty() {
        let analysis = analyze(&row_then_stack());
        let hero = analysis
            .relation("body>main:1>div:1", 400, LayoutRelationKind::AspectRatio)
            .unwrap();
        assert!((hero.value - 16.0 / 9.0).abs() <= hero.uncertainty + 1e-12);
        assert!(hero.uncertainty > 0.0 && hero.uncertainty < 1e-3);
        assert_eq!(hero.status, EpistemicStatus::Derived);
        let share = analysis
            .relation(
                "body>main:1>div:1",
                1000,
                LayoutRelationKind::WidthOverParent,
            )
            .unwrap();
        assert!((share.value - 2.0 / 3.0).abs() < 1e-9);
        let gap = analysis
            .relation("body>main:1", 1000, LayoutRelationKind::GapOverFontSize)
            .unwrap();
        assert_eq!(gap.value, 1.0);
    }

    #[test]
    fn a_row_that_stacks_yields_direction_and_stacking_rules_as_an_interval() {
        let rules = analyze(&row_then_stack()).responsive_rules;
        assert!(rules.iter().any(|rule| rule.element == "body>main:1"
            && rule.change
                == ResponsiveChange::FlexDirection {
                    narrow: "column".into(),
                    wide: "row".into()
                }
            && (rule.narrower, rule.wider) == (400, 1000)));
        assert!(rules.iter().any(|rule| rule.change
            == ResponsiveChange::SiblingsStack {
                first: "body>main:1>div:1".into(),
                second: "body>main:1>div:2".into()
            }));
        assert!(
            rules
                .iter()
                .all(|rule| rule.status == EpistemicStatus::Derived)
        );
    }

    #[test]
    fn identical_viewports_yield_no_rules_and_hidden_elements_no_relations() {
        let mut observed = row_then_stack();
        observed.viewports[1] = observed.viewports[0].clone();
        observed.viewports[1].width = 1200;
        assert!(analyze(&observed).responsive_rules.is_empty());

        let mut hidden = row_then_stack();
        hidden.viewports[0].elements[2]
            .style
            .insert("display".into(), "none".into());
        let analysis = analyze(&hidden);
        assert!(
            analysis
                .relation(
                    "body>main:1>div:2",
                    1000,
                    LayoutRelationKind::WidthOverViewport
                )
                .is_none()
        );
        assert!(analysis.responsive_rules.iter().any(|rule| matches!(
            &rule.change,
            ResponsiveChange::Display { wide, .. } if wide == "none"
        )));
    }

    fn change(element: &str, property: &str, before: &str, after: &str) -> ObservedChange {
        ObservedChange {
            element: element.into(),
            property: property.into(),
            before: Some(before.into()),
            after: after.into(),
        }
    }

    fn observed(
        target: &str,
        stimulus: Stimulus,
        changes: Vec<ObservedChange>,
    ) -> InteractionObservation {
        InteractionObservation {
            target: target.into(),
            stimulus,
            animations: 0,
            changes,
            status: EpistemicStatus::Observed,
        }
    }

    #[test]
    fn interaction_mechanisms_and_state_graph_are_derived_from_observed_changes() {
        let button = "body>button:1";
        let panel = "body>div:1";
        let tile = "body>div:2";
        let observations = vec![
            observed(
                tile,
                Stimulus::Hover,
                vec![change(
                    tile,
                    "transform",
                    "none",
                    "matrix(1, 0, 0, 1, 0, -4)",
                )],
            ),
            observed(tile, Stimulus::Click, vec![]),
            observed(button, Stimulus::Hover, vec![]),
            observed(
                button,
                Stimulus::Click,
                vec![
                    change(button, "attr:aria-expanded", "false", "true"),
                    change(panel, "display", "none", "block"),
                    change(button, "focused", "false", "true"),
                ],
            ),
            observed(
                button,
                Stimulus::ClickTwice,
                vec![change(button, "focused", "false", "true")],
            ),
            observed(
                button,
                Stimulus::Focus,
                vec![change(button, "focused", "false", "true")],
            ),
        ];
        let analysis = analyze_interactions(&observations);
        assert!(
            analysis
                .mechanisms
                .contains(&InteractionMechanism::HoverAffordance {
                    target: tile.into(),
                    properties: vec!["transform".into()]
                })
        );
        assert!(
            analysis
                .mechanisms
                .contains(&InteractionMechanism::Disclosure {
                    target: button.into(),
                    controlled: vec![panel.into()]
                })
        );
        assert!(analysis.mechanisms.contains(&InteractionMechanism::Toggle {
            target: button.into()
        }));
        assert!(
            !analysis
                .mechanisms
                .iter()
                .any(|m| matches!(m, InteractionMechanism::FocusIndicator { .. })),
            "focus alone is not a focus indicator"
        );
        assert_eq!(analysis.states.len(), 2);
        assert_eq!(analysis.transitions.len(), 2);
        assert_eq!(
            analysis.transitions[1].to, "S0",
            "the toggle returns to the baseline"
        );
        assert_eq!(analysis.status, Some(EpistemicStatus::Derived));
    }

    #[test]
    fn an_ancestor_that_only_reflows_is_not_controlled() {
        let button = "body>button:1";
        let observations = vec![observed(
            button,
            Stimulus::Click,
            vec![
                change("body", "height", "300", "340"),
                change("body>div:1", "display", "none", "block"),
                change("body>div:1", "height", "0", "40"),
            ],
        )];
        assert!(analyze_interactions(&observations).mechanisms.contains(
            &InteractionMechanism::Disclosure {
                target: button.into(),
                controlled: vec!["body>div:1".into()]
            }
        ));
    }

    #[test]
    fn a_hover_that_only_changes_another_element_is_not_the_targets_affordance() {
        let observations = vec![observed(
            "body>a:1",
            Stimulus::Hover,
            vec![change("body>div:9", "opacity", "1", "0.5")],
        )];
        assert!(analyze_interactions(&observations).mechanisms.is_empty());
    }

    #[test]
    fn a_click_that_does_not_return_is_not_a_toggle() {
        let button = "body>button:1";
        let observations = vec![
            observed(
                button,
                Stimulus::Click,
                vec![change("body>p:1", "display", "none", "block")],
            ),
            observed(
                button,
                Stimulus::ClickTwice,
                vec![change("body>p:1", "display", "none", "block")],
            ),
        ];
        let analysis = analyze_interactions(&observations);
        assert!(
            !analysis
                .mechanisms
                .iter()
                .any(|m| matches!(m, InteractionMechanism::Toggle { .. }))
        );
        assert!(
            analysis
                .mechanisms
                .iter()
                .any(|m| matches!(m, InteractionMechanism::Disclosure { .. }))
        );
    }

    fn sampled(control: [f64; 4], transform: impl Fn(f64) -> f64) -> MotionObservation {
        MotionObservation {
            target: "t".into(),
            stimulus: Stimulus::Hover,
            element: "e".into(),
            property: Some("transform".into()),
            declared: DeclaredTiming {
                duration_ms: 400.0,
                delay_ms: 0.0,
                easing: "ease-out".into(),
            },
            samples: (0..=20)
                .map(|k| {
                    let x = f64::from(k) / 20.0;
                    (
                        400.0 * x,
                        format!(
                            "matrix(1, 0, 0, 1, 0, {})",
                            transform(cubic_bezier(control, x)) * -20.0
                        ),
                    )
                })
                .collect(),
            status: EpistemicStatus::Observed,
        }
    }

    #[test]
    fn cubic_bezier_matches_known_points() {
        assert!((cubic_bezier([0.0, 0.0, 1.0, 1.0], 0.3) - 0.3).abs() < 1e-12);
        assert!(
            (cubic_bezier([0.42, 0.0, 0.58, 1.0], 0.5) - 0.5).abs() < 1e-9,
            "ease-in-out is symmetric"
        );
        // Chromium sampled ease-out at 25% as 7.56276/20 = 0.378138.
        assert!((cubic_bezier([0.0, 0.0, 0.58, 1.0], 0.25) - 0.378138).abs() < 1e-5);
    }

    #[test]
    fn keyword_curves_match_chromium_sampled_values() {
        // Independent oracle: Chromium 141's own easing evaluation (opacity 0 -> 1 animation,
        // paused and seeked to 25/50/75%), recorded 2026-09-24 (ADR 0016).
        let chromium: [(&str, [f64; 3]); 5] = [
            ("linear", [0.25, 0.5, 0.75]),
            ("ease", [0.408511, 0.802403, 0.960459]),
            ("ease-in", [0.0934647, 0.315357, 0.621862]),
            ("ease-out", [0.378138, 0.684643, 0.906535]),
            ("ease-in-out", [0.129162, 0.5, 0.870838]),
        ];
        for ((name, control), (oracle_name, values)) in KEYWORD_EASINGS.iter().zip(chromium) {
            assert_eq!(*name, oracle_name);
            for (x, expected) in [0.25, 0.5, 0.75].into_iter().zip(values) {
                assert!(
                    (cubic_bezier(*control, x) - expected).abs() < 1e-5,
                    "{name} at {x}: {} vs Chromium {expected}",
                    cubic_bezier(*control, x)
                );
            }
        }
    }

    #[test]
    fn a_custom_curve_matching_no_keyword_is_not_forced_into_one() {
        // cubic-bezier(0.9, 0.1, 0.1, 0.9); Chromium samples it as 0.0609247 / 0.5 / 0.939075.
        let custom = [0.9, 0.1, 0.1, 0.9];
        assert!((cubic_bezier(custom, 0.25) - 0.0609247).abs() < 1e-5);
        let inference = &infer_motion(&[sampled(custom, |p| p)])[0];
        assert!(!inference.overshoot);
        assert_eq!(inference.inferred_easing, None, "{inference:?}");
        assert!(inference.candidates[0].rms_error > EASING_TOLERANCE);
        assert_eq!(inference.candidates.len(), 5, "alternatives still reported");
    }

    #[test]
    fn each_keyword_curve_is_recovered_from_samples_alone() {
        for (name, control) in KEYWORD_EASINGS {
            let inference = &infer_motion(&[sampled(control, |p| p)])[0];
            assert_eq!(
                inference.inferred_easing.as_deref(),
                Some(name),
                "{inference:?}"
            );
            assert_eq!(inference.status, EpistemicStatus::Inferred);
            assert_eq!(inference.candidates.len(), 5, "alternatives are kept");
            assert!(inference.candidates[1].rms_error > inference.candidates[0].rms_error);
        }
    }

    #[test]
    fn an_overshooting_curve_is_flagged_and_matches_no_keyword() {
        // Damped oscillation 1 - e^(-6p) cos(10p): overshoots to ~1.06 near p = 0.4.
        let spring = |p: f64| 1.0 - (-6.0 * p).exp() * (10.0 * p).cos();
        let inference = &infer_motion(&[sampled([0.0, 0.0, 1.0, 1.0], spring)])[0];
        assert!(inference.overshoot);
        assert_eq!(inference.inferred_easing, None);
        assert_eq!(inference.agrees_with_declared, None);
    }

    #[test]
    fn grid_column_counts_ignore_named_lines_and_functions() {
        assert_eq!(grid_columns("300px 300px 300px"), Some(3));
        assert_eq!(
            grid_columns("[full-start] 100px [main] 1fr [full-end]"),
            Some(2)
        );
        assert_eq!(grid_columns("minmax(100px, 1fr) 200px"), Some(2));
        assert_eq!(grid_columns("none"), None);
    }
}
