---
id: atlas.decision.0016.motion-curve-inference
type: decision
status: accepted
canonical: true
---
# ADR 0016 — Motion: deterministic curve sampling and validated easing inference

## Context

The Creator directive calls for three things. Atlas should extract reusable motion mechanisms: duration, delay, easing and spring behaviour. It must not copy timing values without understanding their role. It must infer hidden state by controlled experiment, recording alternatives and falsification, and must never present a black-box inference as knowledge of the source.

## Decision

1. **Motion mode in the instrument.** The instrument hovers each interactive candidate in a fresh hermetic context. For every animation that starts, it:
   - pauses the animation;
   - seeks it to 21 evenly spaced times across its active interval (delay + k·duration/20);
   - reads the animated property at each time.

   Seeking makes sampling deterministic, with no wall-clock jitter. The Web Animations timing (duration, delay, easing) is recorded as `DeclaredTiming`. It is `OBSERVED`: the page declared it and the engine reports it.
2. **Inference from the curve alone** (`core::visual::infer_motion`). The curve is normalized to the progress of the numeric component that moves most. It is then ranked against the five CSS keyword curves by RMS error, using `cubic_bezier` (bisection on the monotone x(t)).
   - The result is `INFERRED`, and every alternative keeps its residual.
   - A curve is accepted as a keyword only if its RMS error is at most 0.01 and it has no overshoot.
   - A curve that leaves [0, 1] is flagged `overshoot`. That covers spring and elastic motion no cubic keyword can model; such a curve is never forced into a keyword.
   - `agrees_with_declared` validates the inference against the declared timing wherever one exists.
3. **Command:** `atlas-systemizer observe --fixture F --motion` emits `atlas.motion-report.v1`.

The inference method is validated where ground truth exists — CSS transitions — before it is applied where none exists: JS- or requestAnimationFrame-driven motion.

## Evidence

- **Fixture results.** On the Atlas-original fixture `motion-curves.html`, which has one transition per keyword, all five easings are inferred from their samples alone. Each agrees with Chromium's declared timing.
  - Residuals are at most 6e-7.
  - The runner-up is always at least 0.07 away, against a tolerance of 0.01.
  - Durations are 250–600 ms, and the 100 ms delay is honoured.
- **Independent oracle.** Chromium's own sampling of each keyword at 25%, 50% and 75% matches `cubic_bezier` to within 1e-5.
- **Negative cases.**
  - A custom `cubic-bezier(0.9, 0.1, 0.1, 0.9)` matches no keyword and is not forced into one.
  - A damped oscillation is flagged `overshoot`.
- **Mutation testing: 6 mutants killed.**
  - wrong `ease` control points;
  - a tolerance loosened to 10;
  - overshoot ignored;
  - coarse bisection;
  - the smallest-moving component chosen;
  - sampling that ignores the delay.

  The first two initially survived, because the unit test generated samples from the same table it checked. The Chromium oracle and the custom-curve test now kill them.

## Next

- Inference for JS/requestAnimationFrame motion under controlled virtual time (Playwright clock).
- Spring parameter fitting (stiffness, damping, mass ratio) as `INFERRED` with alternatives, for curves flagged `overshoot`.
