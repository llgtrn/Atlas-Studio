---
id: atlas.decision.0017.creator-construction-loop
type: decision
status: accepted
canonical: true
---
# ADR 0017 — Creator construction loop: intent → artifact → render → re-observe → verify

## Context

The Creator directive's first end-to-end milestone takes Atlas-native visual semantics, synthesizes an original composition, lowers it to frontend code, renders it, replays its interactions and verifies its constraints, using real production paths.

ADR 0011, 0013, 0015 and 0016 gave Atlas the observation half: layout relations, measured breakpoints, interaction mechanisms and motion inference. This ADR adds the construction half. The same instrument closes the loop.

## Decision

1. **`core::visual::compose::DesignIntent`** — an Atlas-native, ratio-first intent:
   - a hero with media aspect, flex share, gap in `em` and copy font ratio;
   - a grid with column count, item count, item aspect and gap;
   - one breakpoint;
   - a hover-lift mechanism (lift distance, easing keyword, duration).

   `validate` rejects non-positive ratios and unknown easings before lowering.
2. **`lower_to_html`** — deterministic, self-contained HTML/CSS. It emits no external request, image, link or script, and only neutral placeholder text. Nothing from any observed reference can reach the output.
3. **`verify_intent`** checks each intended relation against the evidence the instrument measures on the rendered artifact:
   - aspect ratios at both viewports, within a tolerance propagated from the 1/64 px layout grid;
   - the flex share;
   - the font ratio;
   - both structural changes, bisected to the exact intended breakpoint;
   - a hover-lift affordance on every item;
   - the hover easing and duration, inferred from sampled curves.

   Every check is `SATISFIED`, `VIOLATED` or `UNKNOWN` (ADR 0007), and the overall verdict is their strong-Kleene conjunction.
4. **Runtime and CLI.**
   - `runtime::visual::create_and_verify` writes the artifact, then observes it through all four instrument modes: layout, bisection, stimuli and motion.
   - `verify_artifact` checks an existing artifact against an intent.
   - `atlas-systemizer create --intent I --out page.html [--report R]` exits non-zero with `CREATION_NOT_VERIFIED` unless every check is satisfied.

## Evidence

- **Original intent.** `runtime/tests/fixtures/creator/original-intent.json` is deliberately unlike every reference fixture: 3:2 media, 3:2 share, a 4-column square grid, an 820 px breakpoint, and a 200 ms ease-in-out lift.
- **All 10 checks are SATISFIED on the real render.** For example:
  - media 1.500013 ± 6.3e-5;
  - share 0.600002;
  - both changes at exactly (819, 820);
  - 4/4 hover lifts;
  - `ease-in-out 200ms` inferred for all four items.
- **Tampered artifact.** The same intent is checked against an artifact changed to 4:3 items, `linear` easing and a 799 px breakpoint. Exactly those checks are VIOLATED; the hero relations still hold.
- **Mutation testing: 4 mutants killed.**
  - flex shares swapped in lowering;
  - breakpoint check always satisfied;
  - lift count weakened;
  - easing unchecked.
- **Honest process note.** The breakpoint mutant first survived. cargo fmt had reflowed the test, and the edit that should have added the `grid collapses` assertion silently matched nothing. The assertion now exists, and it kills the mutant.

## Limits and next

- The intent vocabulary is one layout family (hero + grid). Intents are hand-authored; they are not yet synthesized from an extracted genome.
- **Next:**
  - a design genome — mechanisms extracted from observed references, with provenance;
  - recombination of the genome into new intents;
  - multi-candidate generation, with Pareto selection over measured checks.
