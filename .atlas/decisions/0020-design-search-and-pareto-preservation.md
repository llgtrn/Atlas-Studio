---
id: atlas.decision.0020.design-search-and-pareto-preservation
type: decision
status: accepted
canonical: true
---
# ADR 0020 — Design search with measured objectives and Pareto preservation

## Context

ADR 0018 recombines one authored recipe. The Creator directive asks for search: many original candidates, scored on multiple objectives, with no aesthetic score treated as objective truth. This is the first place where Futures (candidates as hypotheses, explicit objectives) meets Creator.

## Decision

1. **`core::visual::search`**:
   - **Candidates.** `generate_recipes(genome, count)` deterministically enumerates recipes over a 3×3×3 cube of options, one parameter varied per mechanism. It traverses the cube diagonal-first, so small counts still vary every axis. Every recipe passes `recombine`'s originality rules: at least 2 sources, and every mechanism varied. There are at most 27 recipes, each option triple visited exactly once.
   - **Measurements.** `measure(layout, wide, narrow)` is taken from the re-observed render:
     - **copy characters per line** = copy width / (font size × 0.5 em). The 0.5 em mean advance is a stated assumption, not a glyph measurement.
     - **narrow content height**: the lowest observed element's bottom edge. It is not `scrollHeight`, which Chromium floors at the viewport height (measured: short pages all read 800 px). The value is absent when the observation was truncated, because it would then only be a lower bound.
   - **Objectives.** `design_objectives()` returns three, each with its `meaning` stated in the report:
     - maximize novelty (Σ |ln(varied/source)|): distance from the references, not quality;
     - minimize |characters per line − 66| (Bringhurst): a line-length proxy, not legibility;
     - minimize narrow content height: scroll burden.
   - **Selection.** `dominates`, `pareto_front` and `verified_front`. Only candidates whose verdict is `Satisfied` **and** that are fully measured compete. An unmeasured objective is never counted as a zero, and `Unknown` does not count as verified.
2. **Runtime.**
   - `CreationReport` becomes `atlas.creation-report.v2` and carries `measurements`. They never feed the verdict.
   - `search_designs(genome, count, out_dir)` runs GENERATE → RECOMBINE → CREATE → VERIFY → MEASURE → SELECT.
   - Each `SearchCandidate` is `HYPOTHESIS` until verified, then `DERIVED`. A refused recipe records its refusal.
   - `atlas.design-search-report.v1` keeps the whole front as a set: `pareto_front` names the non-dominated candidates, and none is called "best".
3. **CLI.** `atlas-systemizer search --genome G [--count N] --out-dir D [--out R]` exits `NO_VERIFIED_CANDIDATE` when the front is empty.

## Evidence

- **Real front.** Measured in Chromium 141 over the genome of the two Atlas-original references, with 3 candidates, all verified SATISFIED:

  | Candidate | Novelty | \|cpl − 66\| | Narrow height |
  |---|---|---|---|
  | candidate-1 | 1.438 | 22.27 | 1183.7 px |
  | candidate-2 | 1.553 | 22.27 | 654.6 px |
  | candidate-3 | 1.674 | 0.40 | 1137.9 px |

  Candidate-2 dominates candidate-1. Candidates 2 and 3 trade off, so the front is {candidate-2, candidate-3}; the end-to-end test pins this. Through the CLI with `--count 4`, the front is {2, 3, 4}.
- **Mutation testing.** 11 mutants were killed:
  - Maximize inverted;
  - ties treated as domination;
  - unverified candidates competing;
  - the 0.5 em advance dropped;
  - truncation ignored;
  - height taken without the element's offset;
  - deviation left signed;
  - novelty left signed;
  - the traversal collapsed to duplicate triples;
  - the count ignored;
  - candidate status inverted.
- **A survivor exposed dead code.** The first battery's "dedupe disabled" mutant survived because distinct triples always produce distinct recipes. The dedupe was removed, and the test now enumerates the whole cube (27 distinct recipes).

## Limits and next

- **Objectives are proxies.**
  - Characters per line assumes a mean advance.
  - Novelty counts `grid.items` separately, although it follows `grid.columns`, so column changes weigh double.
  - Mathematically equal novelties occur (e.g. ln(16/3) twice).
- **Search is exhaustive enumeration over a fixed cube**, not optimization. There are no priors or posteriors, and nothing is learned between runs.
- **Next.** Contrast/accessibility measurements (these need colour in the intent), and self-improvement: the front's results informing the next option ladders (Creator milestone 3).
