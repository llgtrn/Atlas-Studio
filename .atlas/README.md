---
id: atlas.docs.readme
type: reference
status: active
canonical: true
---
# Atlas Studio Knowledge Root

`.atlas/` is the repository knowledge/control root for Atlas Studio.

It is NOT the same thing as the `*.atlas` binary artifact format.

The artifact model is fixed by `contracts/ARTIFACT-LAYERING.md`:

~~~text
ADL      = Intent Artifact
*.atlas  = Canonical Semantic Artifact
*.atlasx = Closed-World System Capsule
~~~

## Canonical lifecycle

~~~text
existing reality / OSS / dependencies
        +
Human + AI natural-language-first ADL
        ↓
strict census + typed elaboration
        ↓
research / mechanism comparison / Jev-class typed decisions
        ↓
external candidate synthesis / code generation
        ↓
generated-code ingestion + census
        ↓
semantic / security / dependency / license / test / benchmark / proof gates
        ↓
authorized SelectedDesign
        ↓
resolved typed semantic world
        ↓
SEALED logical Atlas
        ↓
provider-independent deterministic compaction
        ↓
*.atlas binary
        ↓
recursive selected-system + dependency + runtime + resource + reproduction closure
        ↓
*.atlasx binary capsule
        ↓
HIR → MIR → LIR → Machine IR / delegated backend
        ↓
product
        ↓
runtime evidence / PGO / recensus
        ↺
~~~

## ADL

ADL is the collaborative authoring/interface layer for human + AI.

It may be primarily natural language.

It carries goals, requirements, constraints, preferences, non-goals, security rules, acceptance criteria, degrees of freedom, and typed references/annotations.

ADL is NOT canonical execution semantics.

Raw prose must be elaborated/resolved into typed Atlas semantics before seal.

The current `.atlas/declared/*.adl` syntax is ADL0, a bootstrap subset. It is not evidence that full ADL is complete.

See:

- `contracts/ATLAS-DEVELOPMENT-LANGUAGE.md`;
- `contracts/HUMAN-AI-ADL-AUTHORING.md`;
- `contracts/ADL-TO-ATLAS.md`.

## `*.atlas`

A canonical `*.atlas` is a binary semantic artifact.

It encodes exact typed meaning for its declared scope.

It is not compressed source and does not require an LLM to reinterpret ADL.

Human-readable outputs from inspect/disasm/explain/export are projections only.

A sealed Atlas must remain meaningful if:

- original ADL is deleted;
- donor checkout is deleted after valid extinction;
- external AI providers disappear.

See:

- `contracts/ATLAS-FORMAT.md`;
- `contracts/ATLAS-BINARY-WIRE-FORMAT.md`;
- `contracts/ATLAS-SEMANTIC-COMPACTION.md`.

## `*.atlasx`

A canonical `*.atlasx` is ONE binary closed-world system capsule.

It binds selected Atlas semantics to the transitive dependency/runtime/resource/build/reproduction closure required by its declared capsule profile.

The old canonical `<system>.atlasx/` directory model is superseded.

An unpacked directory is only an inspection/workspace projection.

AtlasX wire v2 is the canonical single-file capsule format.

See:

- `contracts/ATLAS-TO-ATLASX.md`;
- `contracts/ATLASX-FORMAT.md`;
- `contracts/ATLASX-BINARY-WIRE-FORMAT.md`.

## Repository storage classes

Durable architecture, contracts, Genome source, provenance, licenses, and deliberately admitted evidence belong under `.atlas/`.

Rebuildable scans/reports belong in `.atlas/.cache/`.

Donor checkouts belong in `.atlas/temporary/` only until absorption/extinction gates are satisfied.

Donor content is untrusted input and MUST remain quarantined from live agent/tool discovery conventions according to `contracts/DONOR-WORKBENCH-ISOLATION.md`.

## Genome

The Genome source is:

~~~text
.atlas/genome/atlas.genome.toml
~~~

The intended compiled semantic artifact is:

~~~text
.atlas/artifacts/atlas-genome.atlas
~~~

That artifact follows the same `*.atlas` semantic role as every other Atlas binary artifact.

## Source and evidence retention

THIN Atlas may authenticate source/evidence externally.

FAT Atlas may embed source/evidence blobs.

Neither mode changes semantic authority: typed Atlas semantics remain canonical.

For selected systems, any donor/dependency bytes still required for build/runtime/reproducibility/legal duty must be embedded or cryptographically pinned by AtlasX before physical source extinction is allowed.

## Mandatory reading

Start at `INDEX.md`.

For any work touching ADL, artifact encoding, materialization, compiler handoff, donor extinction, or system packaging, read `contracts/ARTIFACT-LAYERING.md` before implementation.
