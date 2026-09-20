---
id: atlas.genome.index
type: reference
status: active
canonical: true
---
# Atlas Genome

`atlas.genome.toml` is the bootstrap, human-auditable source of Atlas hard requirements.

It is not the final packed Genome representation. The target pipeline is:

```text
atlas.genome.toml
      ↓ validate / canonicalize
typed Genome IR
      ↓ pack
atlas-genome.atlas
      ↓ hash/pin
census + plans + *.atlas + *.atlasx + generated repos
```

The compiled Genome artifact must remain inspectable through Atlas tooling even when stored as dense binary. A Genome revision changes the interpretation/enforcement rules and therefore has its own immutable identity.

Hard requirements are not suggestions to coding agents. If implementation cannot satisfy them, the implementation is incomplete or the Genome must be explicitly versioned through an architecture decision.
