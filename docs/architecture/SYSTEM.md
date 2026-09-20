---
id: atlas.architecture.system
type: architecture
status: canonical
canonical: true
---
# Atlas System Architecture

## System Model

Configured repositories are read-only Fleet nodes. Each repository exposes mandatory docs, repository manifest, exact Git head, source tree, and evidence. Atlas compiles those inputs into derived engineering graphs and coding admission state.

## Responsibilities

atlas-docs validates documentation. atlas-repo validates repository discipline. atlas-fleet resolves configured repositories and enforces one-repository coding sessions. atlas-source and atlas-fact observe source. atlas-graph, atlas-technology and atlas-design derive engineering meaning. atlas-build, atlas-refactor and atlas-proof plan bounded engineering work and verification.

## Boundaries

Fleet observation may span all configured repositories. Coding admission selects exactly one repository. Atlas-generated plans cannot merge themselves. Chronica product/runtime crates never import Atlas crates.

## Runtime Ownership

The atlas-systemizer binary is the stable external interface. Internal crate boundaries may evolve without forcing Chronica product code to change, provided the versioned CLI contract remains compatible.

## Data and Effect Flow

fleet/repos.yaml -> read-only connection -> exact heads -> repo/docs audit -> source/fact analysis -> graph projections -> select repository -> coding admission -> bounded work plan -> external coding/CI/review -> refreshed observation.

## Failure and Recovery

Connection failure remains explicit per repository. Documentation or repository gate failure blocks work preparation. Failed coding remains on its branch/worktree and does not alter canonical main. Derived Atlas state may be discarded and rebuilt.

## Evidence

Exact repository SHA, docs gate report, repo gate report, graph evidence, work scope, tests, CI and integration SHA provide engineering evidence.

## Verification

Cargo fmt, clippy, tests, CLI contract tests, docs-gate tests, fleet single-repository tests, repository audit tests and integration evidence are required before a capability is considered proven.
