# Atlas Studio

Atlas Studio is an independent engineering-world compiler and system invention environment.

It reads repositories, documentation, donor technology and engineering evidence; compiles observed and declared knowledge into typed engineering semantics; and is being refounded toward the pipeline:

```text
sources / donors / evidence / ADL
        ↓
secure ingestion
        ↓
corpus.atlas
        ↓
semantic compilation
        ↓
world.atlasx/
        ↓
query / synthesize / materialize / verify
```

Atlas Studio is not owned by any target product runtime. Target repositories remain sovereign mutation boundaries.

## Current compatibility boundary

The current compatibility binary remains `atlas-systemizer` and the compatibility API remains `atlas.systemizer.cli.v1` while the canonical product identity migrates to Atlas Studio.

```text
atlas-systemizer contract --format json
atlas-systemizer systemize --root <workspace> --out <report>
atlas-systemizer docs audit --root <docs-root>
atlas-systemizer code analyze --root <source-root>
```

## Implementation policy

- Backend/compiler/runtime: Rust.
- Frontend: TypeScript/TSX.
- `.atlas/` is the sole repository knowledge/control root.
- Donor OSS is evidence and technology reference, not a permanent runtime owner.
- Canonical implementation responsibilities are `core/`, `runtime/`, `adapter/`, plus thin application projections under `apps/`.
- New engine behavior must not be added to legacy JavaScript tooling.
- Generated or inferred state is not canonical truth without verification and evidence.
