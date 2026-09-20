# Atlas Systemizer

Atlas Systemizer is Chronica's external **development engineering subsystem**.

It reads repositories, documentation, donor technology and engineering evidence; graphinizes them into derived Source / Fact / Technology / Design / Engineering graphs; standardizes documentation; and emits bounded analysis, work plans and proof requirements.

It is **not** part of the Chronica runtime and has no merge or execution authority.

## Stable boundary

The binary is `atlas-systemizer` and the compatibility API is `atlas.systemizer.cli.v1`.

```text
atlas-systemizer contract --format json
atlas-systemizer systemize --root <workspace> --config <config> --out <report>
atlas-systemizer docs audit --root <docs-root> --config <config> --format json
atlas-systemizer code analyze --root <source-root> --config <config> --format json
```

Chronica product crates must never import Atlas crates. Developer and CI workflows call the CLI only.

## Implementation policy

- Engineering/backend implementation: Rust.
- Graph Studio frontend, when materialized: TypeScript/TSX.
- Reference OSS is technology donor material, not a permanent runtime substrate.
- Atlas output is ANALYZE/evidence, never canonical Chronica truth.
