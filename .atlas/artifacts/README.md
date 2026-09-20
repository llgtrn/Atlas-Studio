---
id: atlas.artifacts.index
type: reference
status: active
canonical: true
---
# Artifacts

Durable compiled artifact identities/manifests belong here when implemented.

Expected classes:

- `atlas-genome.atlas` — compiled hard-requirement Genome;
- `<system>.atlas` or logical-root manifest — dense engineering/design artifact identity;
- immutable content-addressed Atlas shards;
- shard/root integrity manifests and signatures;
- product lineage manifests linking Atlas root, AtlasX root, Genome, compiler, target profiles and final artifact hashes;
- benchmark/profile attestations deliberately retained as durable evidence.

`<system>.atlasx/` is generally an expanded executable materialization and may be rebuilt from pinned Atlas root + selection + Genome + compiler inputs.

Large rebuildable scans, graph dumps, compiler temporaries and inspection exports belong in `.atlas/.cache/`.

Human-readable exports never replace logical root/shard or product integrity semantics.
