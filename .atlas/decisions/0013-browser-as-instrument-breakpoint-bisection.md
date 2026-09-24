---
id: atlas.decision.0013.browser-as-instrument-breakpoint-bisection
type: decision
status: accepted
canonical: true
---
# ADR 0013 — Browser as a measuring instrument: breakpoint bisection

## Context

ADR 0011 states a responsive change only as the interval between two observed widths. The Creator directive asks Atlas to use the browser as an instrument: stimulus, then measurement, then inference. Hidden state is to be recovered by controlled experiment, with the hypothesis, observations, inferred value, assumptions and falsification all recorded.

## Decision

`runtime::visual::bisect_breakpoints(fixture, narrow, wide, height)` works in three steps:

1. It observes the subject at `narrow` and at `wide`.
2. For each derived `ResponsiveRule`, it bisects the viewport width. At each probe it re-observes the subject and asks whether the same rule — same element, same change — already appears between `narrow` and the probe.
3. It records an `InferredBreakpoint`:
   - `narrow_width`, the largest width observed in the narrow state;
   - `wide_width = narrow_width + 1`;
   - the number of probes;
   - status `INFERRED`, with the stated assumption "single monotone transition between the bracketing widths".

Supporting rules:

- Probes are cached by width and shared across rules.
- Every probe must carry the same subject digest; otherwise the run is refused.
- `atlas-systemizer observe --fixture F --bisect` bisects between the narrowest and widest requested viewports.

## Evidence

- On the Atlas-original editorial fixture (CSS `@media (max-width: 699px)`), all five rules are measured at exactly **(699, 700)**. Those rules are flex direction, grid columns, and three sibling stackings.
- The measurement used 10 browser probes in total.
- **Mutation testing.** Two mutants were killed:
  - stopping the bisection at 8 px precision;
  - inverting the probe decision.

## Consequences

- This is the first Creator inference produced by experiment rather than by passive observation.
- The same stimulus-and-measurement loop applies next to hover, click and scroll stimuli and to frame sampling for motion. Spring and easing parameters will be `INFERRED`, carrying their alternatives.
