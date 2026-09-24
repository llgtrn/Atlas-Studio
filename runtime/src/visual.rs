//! Creator Fabric orchestration: observe an authorized visual subject through the browser
//! instrument and derive its visual semantics (ADR 0011).

use atlas_core::{
    EpistemicStatus, IntegrityDigest,
    visual::{
        ElementObservation, ObservationInstrument, ResponsiveChange, ResponsiveRule,
        VISUAL_OBSERVATION_SCHEMA, ViewportObservation, VisualAnalysis, VisualObservationReport,
        VisualSubject, analyze,
    },
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io, path::Path};

/// The observation plus everything derived from it -- what `atlas-systemizer observe` emits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualReport {
    pub schema: String,
    pub observation: VisualObservationReport,
    pub analysis: VisualAnalysis,
    /// Present when breakpoints were measured by bisection (`observe --bisect`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub breakpoints: Vec<InferredBreakpoint>,
}

/// A responsive change located by controlled experiment: the instrument observed the narrow state
/// at `narrow_width` and the wide state at `wide_width = narrow_width + 1`. `INFERRED`, because
/// locating it assumes the change is a single monotone transition between the two bracketing
/// widths (true of CSS media/container queries; stated, not presumed silently).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InferredBreakpoint {
    pub element: String,
    pub change: ResponsiveChange,
    pub narrow_width: u32,
    pub wide_width: u32,
    pub probes: usize,
    pub status: EpistemicStatus,
    pub assumption: String,
}

#[derive(Deserialize)]
struct RawElement {
    path: String,
    parent: Option<String>,
    tag: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    style: BTreeMap<String, String>,
    text_chars: usize,
}

#[derive(Deserialize)]
struct RawViewport {
    width: u32,
    height: u32,
    document_width: f64,
    document_height: f64,
    root_font_size_px: f64,
    elements: Vec<RawElement>,
    truncated: bool,
}

#[derive(Deserialize)]
struct RawObservation {
    engine: String,
    engine_version: String,
    driver: String,
    driver_version: String,
    viewports: Vec<RawViewport>,
}

/// Parses the instrument's raw JSON into a typed, `OBSERVED` report about `subject`.
pub fn parse_observation(raw: &str, subject: VisualSubject) -> io::Result<VisualObservationReport> {
    let raw: RawObservation = serde_json::from_str(raw).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("instrument output: {e}"),
        )
    })?;
    Ok(VisualObservationReport {
        schema: VISUAL_OBSERVATION_SCHEMA.into(),
        status: EpistemicStatus::Observed,
        subject,
        instrument: ObservationInstrument {
            engine: raw.engine,
            engine_version: raw.engine_version,
            driver: raw.driver,
            driver_version: raw.driver_version,
            network: "BLOCKED_EXCEPT_SUBJECT".into(),
        },
        viewports: raw
            .viewports
            .into_iter()
            .map(|viewport| ViewportObservation {
                width: viewport.width,
                height: viewport.height,
                document_width: viewport.document_width,
                document_height: viewport.document_height,
                root_font_size_px: viewport.root_font_size_px,
                truncated: viewport.truncated,
                elements: viewport
                    .elements
                    .into_iter()
                    .map(|element| ElementObservation {
                        path: element.path,
                        parent: element.parent,
                        tag: element.tag,
                        x: element.x,
                        y: element.y,
                        width: element.width,
                        height: element.height,
                        style: element.style,
                        text_chars: element.text_chars,
                    })
                    .collect(),
            })
            .collect(),
    })
}

/// Observes a local fixture at the given viewports and derives its layout relations and
/// responsive rules. The subject's bytes are digested before and after the browser loads them;
/// if they differ, the observation is refused rather than attributed to either version.
pub fn observe_fixture(fixture: &Path, viewports: &[(u32, u32)]) -> io::Result<VisualReport> {
    let before = IntegrityDigest::of_bytes(&fs::read(fixture)?);
    let raw = adapter::browser::observe_fixture_raw(fixture, viewports)?;
    let after = IntegrityDigest::of_bytes(&fs::read(fixture)?);
    if before != after {
        return Err(io::Error::other(
            "observation subject changed while it was being observed",
        ));
    }
    let observation = parse_observation(
        &raw,
        VisualSubject {
            locator: fixture.to_string_lossy().into_owned(),
            content_digest: before,
            authorization: "LOCAL_FIXTURE".into(),
        },
    )?;
    let analysis = analyze(&observation);
    Ok(VisualReport {
        schema: "atlas.visual-report.v1".into(),
        observation,
        analysis,
        breakpoints: Vec::new(),
    })
}

/// Width-indexed single-viewport observations of one subject, all required to share its digest.
struct Probes<'a> {
    fixture: &'a Path,
    height: u32,
    digest: Option<IntegrityDigest>,
    by_width: BTreeMap<u32, ViewportObservation>,
    base: Option<VisualObservationReport>,
}

impl Probes<'_> {
    fn at(&mut self, width: u32) -> io::Result<ViewportObservation> {
        if let Some(observed) = self.by_width.get(&width) {
            return Ok(observed.clone());
        }
        let report = observe_fixture(self.fixture, &[(width, self.height)])?;
        let digest = report.observation.subject.content_digest.clone();
        if *self.digest.get_or_insert(digest.clone()) != digest {
            return Err(io::Error::other(
                "observation subject changed between bisection probes",
            ));
        }
        let viewport = report.observation.viewports[0].clone();
        self.base.get_or_insert(report.observation);
        self.by_width.insert(width, viewport.clone());
        Ok(viewport)
    }

    /// Whether `rule`'s change has already happened between `narrow` and `width`.
    fn changed_by(&mut self, rule: &ResponsiveRule, width: u32) -> io::Result<bool> {
        let narrow = self.at(rule.narrower)?;
        let probe = self.at(width)?;
        let mut pair = self.base.clone().expect("probed at least once");
        pair.viewports = vec![narrow, probe];
        Ok(analyze(&pair)
            .responsive_rules
            .iter()
            .any(|candidate| candidate.element == rule.element && candidate.change == rule.change))
    }
}

/// Observes `fixture` at `narrow` and `wide`, then locates every responsive change between them
/// to a single pixel by bisection -- the browser as a measuring instrument: stimulus (viewport
/// width) -> measurement (layout) -> inference (breakpoint).
pub fn bisect_breakpoints(
    fixture: &Path,
    narrow: u32,
    wide: u32,
    height: u32,
) -> io::Result<VisualReport> {
    if narrow >= wide {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "bisection needs narrow < wide",
        ));
    }
    let mut report = observe_fixture(fixture, &[(wide, height), (narrow, height)])?;
    let mut probes = Probes {
        fixture,
        height,
        digest: Some(report.observation.subject.content_digest.clone()),
        by_width: BTreeMap::new(),
        base: None,
    };
    for rule in report.analysis.responsive_rules.clone() {
        let (mut lo, mut hi) = (rule.narrower, rule.wider);
        let before = probes.by_width.len();
        while hi - lo > 1 {
            let mid = lo + (hi - lo) / 2;
            if probes.changed_by(&rule, mid)? {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        report.breakpoints.push(InferredBreakpoint {
            element: rule.element.clone(),
            change: rule.change.clone(),
            narrow_width: lo,
            wide_width: hi,
            probes: probes.by_width.len() - before,
            status: EpistemicStatus::Inferred,
            assumption: "single monotone transition between the bracketing widths".into(),
        });
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::visual::{LayoutRelationKind, ResponsiveChange};

    fn fixture() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/visual/responsive-editorial.html")
    }

    /// End-to-end through the real instrument. The fixture's CSS encodes known facts (16:9 media,
    /// 2:1 flex split with 24 px gap and 32 px padding, 4:3 cards, 3-column grid that collapses to
    /// 1 below 700 px, row that becomes a column); the derived semantics must recover exactly
    /// those, and a second observation must be identical. Skipped -- loudly -- only where no
    /// browser instrument exists (e.g. a CI runner without Playwright).
    #[test]
    fn observing_the_editorial_fixture_recovers_its_declared_design_relations() {
        if adapter::browser::playwright_module().is_none() {
            eprintln!("SKIP: no Playwright browser instrument in this environment");
            return;
        }
        let report = observe_fixture(&fixture(), &[(1280, 800), (375, 800)]).unwrap();
        assert_eq!(report.observation.status, EpistemicStatus::Observed);
        assert_eq!(report.observation.instrument.engine, "chromium");
        let analysis = &report.analysis;
        let near = |element: &str, width: u32, kind: LayoutRelationKind, expected: f64| {
            let relation = analysis
                .relation(element, width, kind)
                .unwrap_or_else(|| panic!("{element} @{width} {kind:?}"));
            assert!(
                (relation.value - expected).abs() <= relation.uncertainty + 1e-9,
                "{element} @{width} {kind:?}: {} vs {expected} (±{})",
                relation.value,
                relation.uncertainty
            );
        };
        let media = "body>section:1>div:1";
        let copy = "body>section:1>div:2";
        let card = "body>section:2>div:1";
        near(media, 1280, LayoutRelationKind::AspectRatio, 16.0 / 9.0);
        near(media, 375, LayoutRelationKind::AspectRatio, 16.0 / 9.0);
        near(card, 1280, LayoutRelationKind::AspectRatio, 4.0 / 3.0);
        // flex 2:1 over (1280 - 2*32 padding - 24 gap) = 1192 px of track.
        near(
            media,
            1280,
            LayoutRelationKind::WidthOverViewport,
            (1192.0 * 2.0 / 3.0) / 1280.0,
        );
        near(
            copy,
            1280,
            LayoutRelationKind::FontSizeOverRoot,
            20.0 / 16.0,
        );
        near(
            "body>section:2",
            1280,
            LayoutRelationKind::GapOverFontSize,
            1.0,
        );

        let rules = &analysis.responsive_rules;
        assert!(rules.iter().any(|rule| rule.element == "body>section:1"
            && rule.change
                == ResponsiveChange::FlexDirection {
                    narrow: "column".into(),
                    wide: "row".into()
                }
            && (rule.narrower, rule.wider) == (375, 1280)));
        assert!(rules.iter().any(|rule| rule.element == "body>section:2"
            && rule.change == ResponsiveChange::GridColumns { narrow: 1, wide: 3 }));
        assert!(rules.iter().any(|rule| rule.change
            == ResponsiveChange::SiblingsStack {
                first: media.into(),
                second: copy.into()
            }));

        let again = observe_fixture(&fixture(), &[(1280, 800), (375, 800)]).unwrap();
        assert_eq!(again, report, "observation is reproducible");
    }

    #[test]
    fn the_instrument_loads_nothing_but_the_subject() {
        if adapter::browser::playwright_module().is_none() {
            eprintln!("SKIP: no Playwright browser instrument in this environment");
            return;
        }
        let probe =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/visual/hermetic-probe.html");
        let report = observe_fixture(&probe, &[(800, 600)]).unwrap();
        let element = report.observation.viewports[0]
            .elements
            .iter()
            .find(|element| element.path == "body>div:1")
            .unwrap();
        assert_eq!(
            element.width, 100.0,
            "the sibling stylesheet (300px) must have been blocked"
        );
    }

    #[test]
    fn bisection_measures_the_fixture_breakpoint_to_the_pixel() {
        if adapter::browser::playwright_module().is_none() {
            eprintln!("SKIP: no Playwright browser instrument in this environment");
            return;
        }
        // The fixture's CSS: `@media (max-width: 699px)` -> narrow state up to 699, wide from 700.
        let report = bisect_breakpoints(&fixture(), 375, 1280, 800).unwrap();
        assert!(report.breakpoints.len() >= 3, "{:?}", report.breakpoints);
        for breakpoint in &report.breakpoints {
            assert_eq!(
                (breakpoint.narrow_width, breakpoint.wide_width),
                (699, 700),
                "{breakpoint:?}"
            );
            assert_eq!(breakpoint.status, EpistemicStatus::Inferred);
        }
        assert!(
            report.breakpoints.iter().map(|b| b.probes).sum::<usize>() <= 12,
            "probes are cached and shared across rules"
        );
    }

    #[test]
    fn instrument_output_that_is_not_an_observation_is_rejected() {
        let subject = VisualSubject {
            locator: "x".into(),
            content_digest: IntegrityDigest::of_bytes(b"x"),
            authorization: "LOCAL_FIXTURE".into(),
        };
        assert!(parse_observation("{\"engine\": 1}", subject.clone()).is_err());
        assert!(parse_observation("not json", subject).is_err());
    }
}
