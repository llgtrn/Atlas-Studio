---
id: atlas.decision.0037.generation-product-match
type: decision
status: accepted
canonical: true
---
# ADR 0037 — A generation's product matches its committed tree (in-toto, absorbed)

## Context

Each self-building generation is a step in Atlas's own construction chain. Its materials are the pre-change census (`pre.json`), and its product is the post-change census it proves (`post.json`). `generation_ledger_self_recensus_chain` already enforced in-toto's MATCH rule on materials: a generation's pre-change census must equal the previous generation's post-change census.

The product side was unchecked. After each commit, `atlas pack` of the clean HEAD records the committed tree's census in `atlas.json`, the equivalent of an in-toto inspection that re-derives a step's product. No test compared it with the proven product. A generation could therefore prove one census and commit a tree with a different one, and the chain would still pass.

The in-toto specification (§4.3.3.2, "MATCH rule behavior") states the rule's purpose: to tie steps together by their materials and products and to "force products to match with products of previous steps". The first-50 campaign reached in-toto as #45. It was never admitted: the specification was read from a transient scratch fetch, pinned by sha256 in the campaign evidence.

## Decision

1. **The check.** For every generation whose evidence directory holds `atlas.json`, the chain test requires that file's `census_digest` to equal the generation's post-change census digest.
2. **Required evidence.** `atlas.json` must exist for every generation except the eight recorded before `atlas pack` existed (G57–G63, G65) and the newest generation. The newest generation's evidence lands in the commit after it.
3. **Not adopted.** Layouts, signed link metadata, functionary keys and sublayouts stay REFERENCE_ONLY. Atlas verifies its own chain by recomputation, not by signatures, and publishes nothing to third parties.

## Consequences

- All 44 generations with clean-HEAD evidence already match, so the rule records a property that held and now makes it binding.
- Falsification:
  - A tampered committed digest (G100) fails the chain test.
  - A removed `atlas.json` (G90) fails it as a missing inspection.
