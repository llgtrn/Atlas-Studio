# Atlas Systemizer

Atlas Systemizer is an independent **software engineering compiler / systemization engine**.

It reads repositories, documentation, donor technology and engineering evidence; compiles them into facts, nodes, edges and bindings; and emits bounded analysis, work plans, graph queries and proof requirements.

It is not owned by any product runtime. It can be pointed at unrelated repositories and must preserve the target project as the sovereign mutation boundary.

## Stable boundary

The binary is `atlas-systemizer` and the compatibility API is `atlas.systemizer.cli.v1`.

```text
atlas-systemizer contract --format json
atlas-systemizer systemize --root <workspace> --out <report>
atlas-systemizer docs audit --root <docs-root>
atlas-systemizer code analyze --root <source-root>
```

External projects integrate through the CLI or future adapter contracts. Atlas internals are not the canonical truth of the repositories it studies.

## Implementation policy

- Engineering/backend implementation: Rust.
- UI implementation, when materialized: TypeScript/TSX.
- Reference OSS is technology donor material, not a permanent runtime substrate.
- Atlas output is analysis, evidence and bounded change proposals unless a WorkRun explicitly verifies and records a mutation.
