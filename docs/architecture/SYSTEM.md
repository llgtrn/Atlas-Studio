---
id: atlas.architecture.system
type: architecture
status: canonical
canonical: true
---
# Atlas System Architecture

## System Model

Atlas manages an external engineering graph over ideas, repositories, docs, source, donors, technology primitives, target designs, mirror workers, CI shards and proof evidence. A target repository need not contain Atlas-specific metadata; Atlas can keep fleet and orchestration state centrally.

## Responsibilities

atlas-docs owns hard documentation admission and docs-plan generation. atlas-source/fact/graph own source observation. atlas-oss discovers and allocates donor/reference technology. atlas-technology extracts Technology Genomes. atlas-design builds target Design Graphs and reconstructs design languages. atlas-translate produces candidate code/design translations. atlas-invent composes invention stages. atlas-mirror creates optional one-to-one mirror plans. atlas-ci distributes verification. atlas-fleet observes configured repositories. atlas-proof governs evidence requirements.

## Boundaries

Atlas may observe many repositories simultaneously. Each invention session has one canonical target lineage. Parallel workers operate only in exact-SHA mirrors or isolated branches and must reconverge. Target repositories do not need Atlas runtime libraries, services or metadata files.

## Runtime Ownership

The stable external surface is the atlas-systemizer CLI/API. Atlas implementation and Graph Studio live only in Atlas-Systemizer. Systems built by Atlas continue to run when Atlas is absent.

## Data and Effect Flow

Idea or target repo -> docs plan/gate -> source/fact graph -> OSS discovery -> Technology Graph -> Target Design Graph -> implementation/mirror plan -> coding -> distributed CI/proof -> reconvergence -> target repository -> refresh graphs.

## Failure and Recovery

Missing docs blocks coding. Stale target SHA invalidates mirror/work plans. Mirror divergence requires rebase/regeneration. Failed translation remains a candidate. Failed proof blocks reconvergence. Derived graphs can be rebuilt.

## Evidence

North Star, blueprint, contracts, exact target SHA, donor provenance, technology graph, target design graph, mirror lineage, translation lineage, CI shards, differential tests and final integration SHA form the engineering evidence chain.

## Verification

Machine tests must prove docs admission, graph-before-code, exact mirror lineage, one canonical target, donor runtime-dependency prohibition, candidate-only translation, distributed CI aggregation and reconvergence safety.
