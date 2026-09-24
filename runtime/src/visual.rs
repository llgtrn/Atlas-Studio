//! Creator Fabric orchestration: observe an authorized visual subject through the browser
//! instrument and derive its visual semantics (ADR 0011).

use atlas_core::{
    EpistemicStatus, IntegrityDigest,
    visual::{
        ElementObservation, ObservationInstrument, VISUAL_OBSERVATION_SCHEMA, ViewportObservation,
        VisualAnalysis, VisualObservationReport, VisualSubject, analyze,
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
    })
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
