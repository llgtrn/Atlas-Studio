---
id: atlas.docs.readme
type: reference
status: active
canonical: true
---
# Atlas Studio Knowledge Root

`.atlas/` is the sole repository knowledge/control root for Atlas Studio. It is distinct from the `*.atlas` binary artifact format.

Canonical lifecycle has two semantic ingress paths that converge before publication:

```text
admitted existing reality                authored Atlas Development Language
→ strict census + completeness proof     → parse/elaborate typed declarations
                \                         /
                 → canonical typed semantic world
                 → normalize/reconcile
                 → research/donor-genesis/invention/selection
                 → SEALED logical *.atlas
→ optional content-addressed shards
→ deterministic *.atlasx/
→ world/semantic optimization
→ HIR/MIR/LIR/Machine IR
→ codegen/LTO/link/post-link
→ product
→ runtime profile/PGO/auto-tuning
→ evidence + recensus
```

Durable architecture, contracts, Genome source, provenance, licenses and deliberately admitted evidence belong under `.atlas/`. Rebuildable scans and reports belong in `.atlas/.cache/`. Donor checkouts belong in `.atlas/temporary/` only until absorption/extinction gates are met.

Read `INDEX.md` for the mandatory route. The Genome source is `.atlas/genome/atlas.genome.toml`; the intended compiled form is `.atlas/artifacts/atlas-genome.atlas`.

Human-readable graph/artifact exports are projections. They do not replace the binary logical Atlas or its root/shard integrity model.

The current `.atlas/declared/*.adl` syntax is ADL0, the bootstrap declaration subset of the future Atlas Development Language. It is not evidence that the full development language is complete. See `contracts/ATLAS-DEVELOPMENT-LANGUAGE.md` and `contracts/ADL-TO-ATLAS.md`.

A `*.atlas` is semantic compression, not merely compressed source. FAT artifacts may embed source/evidence blobs, but typed semantics remain authoritative.
