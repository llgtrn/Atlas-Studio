---
id: adr.0029
type: decision
status: accepted
canonical: true
---
# ADR-0029 — Atlas Systemizer Is an External Development Subsystem

## Status

Accepted.

## Decision

The core System Atlas implementation is extracted from Chronica into the dedicated repository:

```text
llgtrn/Atlas-Systemizer
```

The stable binary/interface name is:

```text
atlas-systemizer
atlas.systemizer.cli.v1
```

The repository name may change independently from this CLI contract. Chronica integrates only through the versioned CLI boundary.

## Boundary

```text
Chronica
  owns:
    canonical product/runtime source
    architecture decisions and invariants
    workspace-side Atlas client configuration
    required CLI contract version
    generated evidence references when intentionally retained

Atlas Systemizer
  owns:
    repository/source indexing
    docs standardization and normalization
    Code Atlas
    Source / Fact / Technology / Design / Engineering graphs
    structural analysis
    dependency closure
    donor/reference technology corpus for Atlas itself
    build / affected-CI planning
    Network/Fleet engineering audit
    refactor/migration planning
    proof orchestration
    Graph Studio implementation
```

Atlas is a development subsystem, not a Chronica runtime component.

```text
core/
runtime/
adapter/
organism/
apps/ui/
      -X-> Atlas crates/libraries

developer / CI
      -> atlas-systemizer CLI
      -> ANALYZE / evidence / bounded plan
      -> normal Chronica authority and Git integration
```

No Chronica production crate or UI runtime may import/link an Atlas crate.

## Stable CLI law

Chronica is allowed to know only the following versioned command surface:

```text
atlas-systemizer contract --format json

atlas-systemizer systemize
  --root <workspace>
  --config <config>
  --out <report>

atlas-systemizer docs audit
  --root <docs-root>
  --config <config>
  --format json

atlas-systemizer code analyze
  --root <source-root>
  --config <config>
  --format json
```

New Atlas capabilities must be introduced behind this stable contract or a new explicitly versioned CLI contract. Internal Atlas crates, graph engines, parsers, donor technologies, storage, and algorithms may evolve without requiring Chronica product-code changes.

## Chronica-side configuration

Chronica declares the client boundary in:

```text
.atlas/systemizer.toml
```

Derived Atlas state belongs under:

```text
.atlas/evidence/
```

and is rebuildable unless an explicit Chronica decision promotes a specific evidence artifact into durable repository history.

## Legacy Atlas implementation

The existing implementation under:

```text
tools/system-atlas/
tools/docs-atlas/
tools/reality-atlas/
```

is a migration baseline only from this decision onward.

It must not receive new Atlas features. Capability migration proceeds into Atlas-Systemizer and each legacy surface is retired after parity/proof. Existing historical code may remain temporarily so canonical main is not broken during extraction.

## Docs standardization

Atlas Systemizer owns the tooling and schemas that standardize documentation.

Chronica owns the meaning and canonical status of its documents.

Atlas may:

- audit taxonomy/frontmatter;
- find duplicate or superseded material;
- build reference/link graphs;
- generate normalization plans;
- check architecture contracts;
- verify docs against code/evidence.

Atlas does not get authority to silently rewrite canonical architectural meaning. Normalization that changes semantics remains reviewable repository ACT.

## Reference technology

Atlas-specific OSS references belong to the Atlas-Systemizer technology/reference corpus rather than becoming Chronica runtime dependencies.

The reference corpus includes the families discussed for:

- parsing and structural code analysis;
- semantic/code-property graphs;
- source fact/cross-reference indexing;
- incremental computation and differential graph maintenance;
- Datalog/relational inference;
- build/dependency graph systems;
- translation candidates;
- Rust semantic/proof tooling;
- graph visualization and layout.

Reference technology is graphinized, compared, and reimplemented into Atlas-native engineering technology over time. It does not define canonical Chronica truth.

## RECORD / ANALYZE / ACT

```text
Atlas reads repository/docs/donor evidence   = RECORD input
Atlas graphs, queries, recommends, plans     = ANALYZE
candidate refactor / translation             = ANALYZE
Chronica code mutation / merge               = ACT
Git + Chronica runtime durable state         = canonical repository/runtime truth
```

Atlas never grants itself execution or merge authority.

## Consequence

After extraction, Chronica product development must not need to know whether Atlas internally uses native Rust graph engines, donor-assisted parsers, a new incremental engine, or a different storage/index strategy.

Chronica's integration contract stays stable while Atlas evolves independently as a dedicated engineering subsystem.
