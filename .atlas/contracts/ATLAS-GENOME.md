---
id: atlas.contract.genome
type: contract
status: active
canonical: true
---
# Atlas Genome Contract

Atlas Genome is the deterministic hard-requirement layer that governs how Atlas observes, reasons, plans, materializes and verifies engineering systems.

The human-auditable source is `.atlas/genome/atlas.genome.toml`. The intended compiled form is a binary `atlas-genome.atlas`. Every `*.atlas`, `*.atlasx/`, plan and generated repository must pin the exact Genome version/hash used to create it.

## Genome responsibilities

Genome MUST define at least:

- universal identity and graph grammar;
- scope lattice and adaptive census triggers;
- evidence/provenance and epistemic classes;
- planning completeness requirements;
- ATLAS and ATLASX format/version contracts;
- materialization language/target policy;
- verification/recensus requirements;
- cross-repository federation contract;
- donor absorption/extinction policy;
- security/admission boundaries;
- compiler phase capabilities and forbidden shortcuts.

Genome is not an LLM prompt and not probabilistic weights. It is machine-checkable, versioned and auditable. It may be physically packaged like a model artifact, but the meaning of each requirement must remain inspectable.

## Determinism

For pinned Genome, input revisions, admitted evidence and compiler version:

```text
same selected design
  → same AtlasX structure
  → same semantic hashes
```

Physical binaries may vary only where the target/toolchain contract explicitly admits nondeterminism.

## Change policy

A Genome change creates a new Genome identity. Atlas must determine which artifacts/scopes require invalidation or recensus. Old artifacts remain attributable to the old Genome; they are never silently reinterpreted under a new one.

## Hard principle

Atlas may become smarter, but it may not become semantically inconsistent across agents, repositories or compiler phases. Genome is the mechanism that prevents that drift.
