---
id: atlas.docs.readme
type: reference
status: active
canonical: true
---
# Atlas Studio Knowledge Root

`.atlas/` is the sole repository knowledge/control root for Atlas Studio. Do not confuse this directory with the `*.atlas` binary artifact format.

Canonical architecture:

```text
admitted source / OSS / DeepWiki / papers / specs / evidence
                            ↓
                    adaptive census
                            ↓
                 engineering synthesis
                            ↓
                     <repo>.atlas
                dense binary design world
                            ↓
                  materialize / compile
                            ↓
                    <repo>.atlasx/
             expanded executable repo image
                            ↓
                      compiler phases
                            ↓
                         binary
                            ↓
              verification + recensus
                            ↺
```

Durable contracts, architecture, genome source, provenance, licenses and verified evidence belong under `.atlas/`. Rebuildable scans and temporary compiler products belong in `.atlas/.cache/`. Donor checkouts belong in `.atlas/temporary/`.

The Atlas Genome is authored at `.atlas/genome/atlas.genome.toml`. The future compiled genome artifact is a binary `atlas-genome.atlas` and every census, plan, `*.atlas`, `*.atlasx/` and generated repository must pin its genome identity/hash.
