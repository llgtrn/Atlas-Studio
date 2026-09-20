---
id: atlas.genome.index
type: reference
status: active
canonical: true
---
# Atlas Genome

`atlas.genome.toml` is the bootstrap human-auditable source of Atlas hard requirements.

Target pipeline:

```text
atlas.genome.toml
  ↓ validate / canonicalize
typed Genome IR
  ↓ pack
atlas-genome.atlas
  ↓ hash/pin
census
  ↓
CensusCertificate / logical *.atlas
  ↓
*.atlasx / compiler / optimization / product evidence
```

Genome controls not only repository admission but also census closure, function accounting, UNKNOWN policy, logical Atlas sharding, materialization determinism, compiler phase capabilities, semantic optimization barriers, production profiles and evidence/recensus requirements.

A Genome revision creates a new immutable identity and may invalidate prior census, materialization or optimization assumptions. Old artifacts remain interpretable only under the Genome that produced them.

Hard requirements are not agent suggestions. Failure to satisfy them means the artifact/run is incomplete unless the Genome itself is explicitly versioned through accepted architecture change.
