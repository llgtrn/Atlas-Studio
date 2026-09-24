---
id: atlas.decision.0025.census-certificate
type: decision
status: accepted
canonical: true
---
# ADR 0025 — CensusCertificate v2 implemented: closure is computed, never claimed

## Context

P0 (census kernel closure) needs the planned reconciliation certificate, and so does exit criterion B. The contract (`CENSUS-CERTIFICATE.md`) and the machine schema (`schemas/census-certificate.schema.json`, v2) existed, but no code produced a certificate.

## Decision

1. **`atlas_core::certificate::certify(report, inputs)`** builds a `CensusCertificate` whose fields are exactly the v2 schema's. Every section uses `deny_unknown_fields`, and a test checks every section's required and allowed keys against the schema file. The sections and what they account:

   | section | what it records |
   |---|---|
   | corpus identity | BLAKE3 over inventoried path, disposition and content digest; the pinned revision |
   | Genome hash | the Genome file |
   | inventory accounting | admitted and accounted artifacts |
   | dependency accounting | nodes, edges, and source-backed workspace members reconciled against the inventory; registry packages as explicit terminal boundaries; unresolved references; closure root hash |
   | semantic accounting | facts, obligations accounted, coverage, unknown/unsupported/ignored/conflict totals, dynamic dependency obligations, ADL binding gaps |
   | normalization | totals and a normalized root hash |
   | provenance completeness | counted per missing field |
   | reconciliation | conflicts |
   | replay fixed point | every census pass has the same digest |
   | independent engines | per (artifact, dimension) |

2. **State follows blockers**, and every unmet condition is a typed blocker:
   - DRAFT: inventory not accounted.
   - CENSUSED: reconciliation, provenance or replay unmet.
   - RECONCILED: other blockers remain.
   - CLOSED: only the `.atlas` root is missing.
   - SEALED: an `.atlas` root exists and there are no blockers.

   No scope policy in the self-scope permits unknowns, so they block.
3. **Entry points:** `runtime::certificate::certificate(root, passes)`, and `atlas-systemizer census certificate`, which exits `CENSUS_NOT_CLOSED` below CLOSED.

## Evidence (Atlas self-scope)

The state is **CENSUSED**. The certificate found gaps no earlier generation had surfaced:
- **Incomplete provenance:** 230 facts carry no revision (ADL-derived facts) and 120 carry no extractor (obligations for the 10 artifacts no extractor covers).
- **An uncensused workspace member:** `atlas-cli` (`apps/cli`), reconciled from the dependency closure's own manifest evidence against the inventory. Four workspace members are source-backed; three are censused.
- **Single-engine extraction:** one extractor per (artifact, dimension), against the Genome's `multi_engine_reconciliation_required`.
- **Uncovered dimensions:** 8 dimensions have UNKNOWN coverage.
- **Unresolved facts:** 672 UNKNOWN and 120 UNSUPPORTED facts.
- **Dynamic dependencies:** 9 dynamic dependency obligations.
- **No `.atlas` root.**

The replay fixed point converges: 2 passes, digests identical. Dependency closure is closed (27 nodes, 39 edges, 20 registry terminal boundaries). Normalization has no conflicts.

Mutation testing: 10 of 10 mutants killed. Two first-battery survivors (the RECONCILED→CLOSED boundary, and an unasserted UNKNOWN_FACTS blocker) were closed by testing a provenance-repaired variant of the real report.

## Next

Each blocker is now a measurable census target, and closing one is a self-recensus-proven generation:
- G60: `atlas-cli` in scope, and source frontends for the UNKNOWN artifacts;
- then provenance completeness;
- then ADL ← census (P2).
