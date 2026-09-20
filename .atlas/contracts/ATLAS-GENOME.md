---
id: atlas.contract.genome
type: contract
status: active
canonical: true
---
# Atlas Genome Contract

Atlas Genome is the deterministic hard-requirement layer governing admission, census, invention, ATLAS publication, AtlasX materialization, compilation, optimization, verification and product evidence.

The human-auditable source is `.atlas/genome/atlas.genome.toml`. The intended compiled form is `atlas-genome.atlas`. Every census certificate, logical Atlas root, AtlasX root, compiler run and generated product pins the exact Genome identity/hash.

## Genome responsibilities

Genome MUST define:

- universal identity/graph/binding/evidence/temporal grammar;
- complete scope lattice and adaptive-depth rules;
- inventory/function/semantic obligation closure;
- UNKNOWN/UNSUPPORTED/conflict policy;
- fixed-point and seal requirements;
- research/invention epistemic transitions;
- ATLAS binary/sharding/content-addressing rules;
- ATLASX deterministic materialization rules;
- deployment/hardware/workload profiles;
- compiler IR phase capabilities;
- optimization freedoms and semantic barriers;
- verification/recensus/product-evidence requirements;
- donor absorption/extinction rules;
- cross-repository federation/security boundaries.

## Completeness principle

Adaptive depth may reduce semantic detail for low-risk scopes but may never erase the existence/accounting of a discovered function/artifact. Silent omission is forbidden.

Critical scopes may require semantic-atom closure and UNKNOWN = 0 before SEALED.

## Optimization principle

Optimization is allowed to change physical structure aggressively, including binding resolution, fusion, memory layout, scheduling and code generation, but may not silently change required graph, authority, temporal, evidence, transaction, safety, recovery or external-interface semantics.

## Determinism

For pinned Genome, logical Atlas root, selected design, compiler version and target profiles:

```text
same semantic inputs
  → same AtlasX semantic hashes
  → reproducible optimization decision lineage
```

Physical nondeterminism is allowed only where explicitly declared by target/toolchain policy.

## Change policy

A Genome change creates a new identity and invalidation/recensus impact. Old artifacts remain attributable to their original Genome and are never silently reinterpreted.
