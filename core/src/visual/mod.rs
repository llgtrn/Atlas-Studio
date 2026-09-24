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
