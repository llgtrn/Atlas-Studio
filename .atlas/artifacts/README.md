---
id: atlas.artifacts.index
type: reference
status: active
canonical: true
---
# Artifacts

Durable compiled engineering artifacts belong here when implemented.

Expected classes include:

- `atlas-genome.atlas` — compiled hard-requirement Genome artifact;
- `<system>.atlas` — dense binary engineering/design artifact;
- manifests, root hashes and signatures for those artifacts;
- optionally packaged ATLASX distributions or target compiler artifacts where retention is useful.

`<system>.atlasx/` is normally an expanded executable workspace/materialization and may be rebuildable from its parent `*.atlas` plus pinned Genome/compiler inputs. Temporary build products and rebuildable reports belong in `.atlas/.cache/`.

Human-readable exports are inspection projections and do not replace binary artifact semantics.
