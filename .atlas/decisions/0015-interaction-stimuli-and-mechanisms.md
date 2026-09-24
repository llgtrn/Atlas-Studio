---
id: atlas.decision.0015.interaction-stimuli-and-mechanisms
type: decision
status: accepted
canonical: true
---
# ADR 0015 — Interaction stimuli, observed state changes and abstract interaction mechanisms

## Context

The Creator directive asks Atlas to model interaction states and to infer mechanisms from behaviour, never claiming to know hidden source. ADR 0011 and ADR 0013 observed only static layout. The next step for the instrument is to apply stimuli to a subject and measure how it responds.

## Decision

1. **Interaction mode in the instrument** (`adapter::browser::interact_fixture_raw`). Candidate targets are:
   - native interactive elements;
   - ARIA role/tabindex elements;
   - elements with `cursor: pointer`.

   Each target receives four stimuli — `HOVER`, `CLICK`, `CLICK_TWICE` and `FOCUS` — each in a fresh hermetic context. After every stimulus the instrument:
   - awaits all running animations, recording how many it awaited;
   - diffs a snapshot against the pre-stimulus baseline. The snapshot covers per-element interaction properties, box size, ARIA/`open` attributes and focus.
2. **Typed observations and derived analysis in `core::visual`.** The observations are `ObservedChange`s and `InteractionObservation`s (`OBSERVED`). `analyze_interactions` produces a `DERIVED` analysis:
   - **`HOVER_AFFORDANCE`:** hovering changes the target's own presentation.
   - **`DISCLOSURE`:** a click changes the display, visibility, height, `open` or `aria-hidden` of other elements (`controlled`), or toggles `aria-expanded`. An ancestor whose only change is its height is reflow, not control.
   - **`TOGGLE`:** a second click returns every property except focus to the baseline.
   - **`FOCUS_INDICATOR`:** focus changes the target's presentation beyond focus itself.
   - **State graph:** state `S0` is the baseline, with one state per distinct settled change-set. Transitions are recorded, including a toggle's return to `S0`.
3. **`atlas-systemizer observe --fixture F --interact`** emits `atlas.interaction-report.v1`.

These are abstract mechanisms: what an element does, not how its source implements it. They are the reusable grammar the originality rule in `CREATOR-FABRIC.md` asks for.

## Evidence

- **Fixture.** The Atlas-original `interactive-mechanisms.html` contains a lifting tile, a menu button with a panel, a `details`/`summary` pair and a static paragraph. From it, the instrument recovers:
  - `HOVER_AFFORDANCE` for the tile, on transform and box-shadow;
  - `DISCLOSURE` and `TOGGLE` for the button (controlling exactly the panel) and for the summary (controlling exactly the `details`);
  - `FOCUS_INDICATOR` (outline) for both;
  - nothing for the static paragraph;
  - a state graph `S0 ⇄ S1` for the button and `S0 ⇄ S2` for the summary.
- **Settled end state.** The 150 ms hover transition was awaited, so the recorded end state is the settled `matrix(1, 0, 0, 1, 0, -4)`.
- **A real browser falsified the first rule.** It counted `body` as controlled because `body` grew when the panel appeared. The rule was refined, with a regression test, before commit.
- **Mutation testing.** Five mutants killed:
  - no settling;
  - focus counted toward toggling;
  - reflow ancestors kept;
  - hover credited from other elements (this one first survived; a test was added);
  - toggle declared without a return to baseline.

## Next

- Frame sampling during transitions, so motion parameters (duration, easing, spring) can be `INFERRED` with alternatives.
- Keyboard-sequence stimuli.
- Visual verification verdicts (overlap, overflow, reachability) over these records.
