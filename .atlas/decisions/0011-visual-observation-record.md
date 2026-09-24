---
id: atlas.decision.0011.visual-observation-record
type: decision
status: accepted
canonical: true
---
# ADR 0011 — Visual observation record and ratio-first layout semantics

## Context

The Creator Fabric directive (G42/G43) asks Atlas to observe authorized visual artifacts and extract transferable layout, interaction and motion mechanisms.

Every Creator lane — layout, motion, interaction, visual verification and the construction loop's render → observe → compare step — depends on one missing primitive: a reproducible, provenance-bearing observation of what a browser actually rendered.

The G42 lane proved that the substrate already exists in the container: Playwright 1.56.1 with Chromium 141.

## Decision

1. **`core::visual`** (pure):
   - `VisualObservationReport` is `OBSERVED`. Per viewport it records every element's structural path, box, text size and a fixed list of 18 computed-style properties.
   - `analyze` derives:
     - `LayoutRelation`s: width/viewport, width/parent, aspect ratio, font-size/root, and gap/font-size. Each carries an uncertainty bound propagated from Chromium's 1/64 px layout resolution.
     - `ResponsiveRule`s: flex-direction, grid-column count, sibling stacking and display changes, each stated as a width interval.
   - Everything derived is marked `DERIVED`.
2. **`adapter::browser`** runs the instrument, which is an EXTERNAL_BOUNDARY recorded in every report:
   - the subject must be a regular local file;
   - each viewport gets a fresh context;
   - every request except the subject is aborted;
   - elements are capped at 5000, with truncation recorded.

   It returns raw JSON. The adapter keeps JSON parsing out of production.
3. **`runtime::visual::observe_fixture`** parses the JSON and digests the subject before and after observation, refusing the result if the digests differ. It then derives the semantics.
4. **`atlas-systemizer observe --fixture F [--viewport WxH]... [--out O]`** emits `atlas.visual-report.v1`.
5. **`.atlas/contracts/CREATOR-FABRIC.md`** fixes the authorization, epistemic, originality and provenance rules.

## Evidence

- **Atlas-original fixture** (`runtime/tests/fixtures/visual/responsive-editorial.html`). Its CSS encodes known facts, and observation through the real instrument recovers every one within the stated uncertainty:
  - 16:9 media;
  - 4:3 cards;
  - a 2:1 flex split over 1192 px of track;
  - a font ratio of 1.25;
  - gap/font = 1;
  - flex row → column, grid 3 → 1 columns, and sibling stacking.
- **Reproducibility:** a second observation is identical.
- **Breakpoint localization:** with three viewports, the (true) 700 px breakpoint is localized to the interval (375, 768]. No rule appears between 768 and 1280.
- **Hermeticity:** a fixture linking a sibling stylesheet that would widen the probe to 300 px stays at 100 px.
- **Mutation testing:** 7 mutants killed — network opened, box geometry corrupted, uncertainty zeroed, flex-direction rule dropped, stacking check inverted, grid brackets counted as tracks, and hidden elements related.
- **CI:** the browser tests skip loudly (`SKIP:` on stderr) only where no Playwright module exists. The pure derivation tests always run.

## Consequences and next steps

- **Next Creator primitive:** a stimulus → measurement experiment — the browser used as an instrument rather than a camera. For example, bisecting the viewport width to measure a breakpoint (turning an interval into an `INFERRED` point), then frame sampling for motion.
- The observation record is the input to `creator_visual_verify`: overlap, overflow and legibility verdicts under ADR 0007's Satisfied/Violated/Unknown.
- **Donors:** Playwright (Apache-2.0) and rrweb (MIT) are the frontier references (`.atlas/roadmap/frontier/creator-frontier-2026-09-24.tsv`). Playwright is used as an instrument, not absorbed. No donor source was admitted in this generation.
