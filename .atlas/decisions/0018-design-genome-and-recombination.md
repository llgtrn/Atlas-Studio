---
id: atlas.decision.0018.design-genome-and-recombination
type: decision
status: accepted
canonical: true
---
# ADR 0018 — Design genome and originality-enforced recombination

## Context

The Creator directive asks Atlas to work as UNDERSTAND → ABSTRACT → RECOMBINE → CREATE, never SCRAPE → COPY:

- extract design mechanisms, not screenshots, with provenance and confidence;
- recombine many examples into original designs that no single reference contains.

ADRs 0011–0017 cover observation and construction. This ADR adds the abstraction and recombination steps between them.

## Decision

1. **`core::visual::genome`**:
   - **`approximate_ratio`** returns the smallest-denominator fraction within tolerance of a measured ratio. For example, 1.77783 becomes 16:9, not 7:4. The tolerance is 2e-3; it is justified by the 1/64 px layout grid and by keeping neighbouring ratios apart.
   - **`extract_mechanisms`** abstracts typed `DesignMechanism`s from one reference's evidence:
     - `SPLIT_HERO`: media aspect, flex share and breakpoint. Derived from a FLEX_DIRECTION rule, the two children's boxes and the bisected breakpoint.
     - `COLLAPSING_GRID`: columns, items, item aspect and breakpoint.
     - `HOVER_LIFT`: lift in px from the settled transform, plus easing and duration from curve inference. Marked `INFERRED`.

     Each mechanism carries its source locator, BLAKE3 digest and evidence list. It never carries markup, text or assets.
   - **`recombine`** builds a `DesignIntent` from a hero, a grid and a lift mechanism, plus declared `variations`. It refuses:
     - fewer than 2 distinct sources;
     - any mechanism whose parameters would be reproduced wholesale — every mechanism needs at least one varied parameter;
     - unknown mechanisms.

     Every intent parameter records its origin: `inherited:<mechanism id>`, `varied:<recipe>` or `default`.
2. **Runtime and CLI**:
   - `runtime::visual::extract_genome` observes each reference through every instrument mode.
   - `recombine_and_create` runs RECOMBINE → CREATE → VERIFY (ADR 0017).
   - `atlas-systemizer genome --fixture A --fixture B [--out G]` extracts the genome.
   - `atlas-systemizer create --genome G --recipe R --out page.html` recombines, creates and verifies.

## Evidence

- **Extraction, from observation alone.** For the two Atlas-original references:
  - the editorial page yields `16:9`, `2:1`, `4:3`, 3 columns and 700 px, matching its CSS exactly;
  - the interactive page yields a 4 px lift over 150 ms `ease-out`, matching its CSS, marked INFERRED.
- **Recombination and creation.** A recipe drawing on both sources, with at least one variation per mechanism, creates a page that verifies SATISFIED on every relation. This was shown both in the test and through the CLI (share 3:2, breakpoint 900, square items, 8 px lift).
- **Mutation testing.** 5 mutants were killed:
  - originality check disabled;
  - single source accepted;
  - ratio tolerance widened to 5e-2;
  - lift sign corrupted;
  - smallest denominator skipped.

## Limits and next

- The mechanism vocabulary is three kinds, and the recipe's variations are authored rather than searched.
- Next: multi-candidate generation over the genome, with measured objectives and Pareto preservation. The objectives are verification pass, distance from sources (novelty), and measurable readability/accessibility proxies. This is where Futures meets Creator, and aesthetic scores are never treated as objective truth.
