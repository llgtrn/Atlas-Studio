---
id: atlas.architecture.responsibilities
type: architecture
status: active
canonical: true
---
# Atlas Systemizer responsibilities

The subsystem is organized by semantic engineering responsibility rather than product silo.

```text
atlas-source       repository/source observations
atlas-fact         source facts and cross references
atlas-graph        shared derived graph primitives
atlas-technology   Technology Genome / primitive graph
atlas-design       target Design Graph and graph diff
atlas-docs         documentation standardization/audit
atlas-build        build / dependency / affected-CI graph
atlas-refactor     bounded rewrite/migration planning
atlas-fleet        cross-repository engineering network
atlas-proof        development proof/evidence orchestration
atlas-core         subsystem orchestration
atlas-cli          stable atlas.systemizer.cli.v1 boundary
```

All crates are engineering-plane code. Chronica product/runtime code consumes only the CLI contract.

New internal technology must fit an existing responsibility or introduce an explicit architecture decision before creating another crate.
