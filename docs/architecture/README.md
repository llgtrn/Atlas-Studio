---
id: atlas.architecture.router
type: architecture
status: canonical
canonical: true
---
# Atlas Architecture Router

## System Model

Atlas Systemizer observes configured repositories, validates mandatory documentation and repository contracts, builds derived engineering graphs, and prepares bounded single-repository coding work.

## Responsibilities

Atlas owns engineering standards, docs admission, fleet observation, source/fact/technology/design graph tooling, bounded planning, refactor planning, build impact analysis, and proof orchestration.

## Boundaries

Atlas is not Chronica runtime, canonical world truth, merge authority, or a second product universe. It may analyze many repositories but mutates at most one selected repository per coding session.

## Runtime Ownership

Atlas implementation lives in this repository under crates and apps/studio. Chronica and Development Cells consume only the stable atlas-systemizer CLI contract and repository standards.

## State and Effects

Fleet heads, source facts, graph projections, docs reports, and work plans are derived and rebuildable. Repository mutation and merge remain explicit external ACT operations.

## Dependencies

Atlas may reference OSS technology donors and Git/GitHub connectivity for development analysis. Chronica runtime must not depend on Atlas availability.
