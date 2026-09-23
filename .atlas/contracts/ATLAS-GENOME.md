---
id: atlas.contract.genome
type: contract
status: active
canonical: true
---
# Atlas Genome Contract

Atlas Genome is the deterministic hard-requirement layer governing admission, census, Human+AI authoring, external-provider orchestration, invention, candidate-code admission, ATLAS publication, AtlasX materialization, compilation, optimization, verification and product evidence.

The human-auditable source is `.atlas/genome/atlas.genome.toml`. The intended compiled form is `atlas-genome.atlas`. Every census certificate, logical Atlas root, AtlasX root, compiler run and generated product pins the exact Genome identity/hash.

## Genome responsibilities

Genome MUST define:

- universal identity/graph/binding/evidence/temporal grammar;
- architectural load-bearing classification, falsifiable invariants, impact closure and collapse-prevention gates;
- complete scope lattice and adaptive-depth rules;
- inventory/function/semantic obligation closure;
- UNKNOWN/UNSUPPORTED/conflict policy;
- fixed-point and seal requirements;
- research/invention epistemic transitions;
- Human+AI authoring boundaries;
- external provider roles/least-privilege/receipts;
- constraint-envelope requirements;
- candidate/decision/change-set admission;
- generated-code census/security/dependency/license gates;
- selection authority modes;
- logical seal-before-compaction requirements;
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

## Architectural integrity principle

Genome MUST make architecture preservation an admission condition rather than an informal review preference.

At minimum the active policy MUST be able to require:

- a pinned ArchitecturalIntegrityEnvelope for material architecture-bearing candidates;
- explicit LOAD_BEARING / STRUCTURAL / REPLACEABLE / DECORATIVE classification where architecture policy needs it;
- at least one falsification condition for every HARD architecture invariant;
- dependency-aware architectural impact closure after semantic change;
- rejection on any observed HARD architectural violation;
- UNKNOWN/CONFLICT to remain unresolved rather than being promoted to PASS;
- load-bearing replacement equivalence evidence or an explicitly SELECTED BlueprintRevisionDecision;
- exact ArchitecturalIntegrityReport evidence at logical seal;
- materialization-time revalidation for invariants affected by binding/profile/closure expansion.

Compile success, local tests and provider confidence MUST NOT override a HARD architectural violation.

The normative contract is ARCHITECTURAL-INTEGRITY.md and the machine profiles are architectural-integrity-envelope.schema.json and architectural-integrity-report.schema.json.

## External intelligence principle

External intelligence may accelerate search, ranking, synthesis and verification.

Genome MUST ensure:

- research output remains research evidence;
- decision scores remain proposals;
- generated source remains untrusted until censused;
- provider roles do not gain canonical-write authority;
- selected design authority is explicit;
- sealed artifacts remain provider-independent;
- canonical post-seal compaction is mechanical.

The machine-readable hard requirements live in `.atlas/genome/atlas.genome.toml`.

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
