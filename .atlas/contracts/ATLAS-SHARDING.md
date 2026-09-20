---
id: atlas.contract.atlas-sharding
type: contract
status: active
canonical: true
---
# Logical Atlas Sharding Contract

One logical Atlas does not require one physical file, repository or storage location.

```text
Logical Atlas Root
    ↓
root manifest + root hash
    ↓
content-addressed immutable shards
    ↓
one or many repositories / artifact stores / object stores
```

## Invariants

- ONE_LOGICAL_ATLAS_MAY_HAVE_MANY_PHYSICAL_SHARDS.
- SHARD_LOCATION_IS_NOT_SEMANTIC_IDENTITY.
- GLOBAL_IDENTITIES_SURVIVE_RELOCATION.
- EVERY_SHARD_HAS_CONTENT_HASH_AND_PARENT_ROOT_LINEAGE.
- ROOT_MANIFEST_COMMITS_TO_ALL_REQUIRED_SHARD_HASHES.
- CROSS_SHARD_BINDINGS_USE_STABLE_GLOBAL_IDENTITIES.
- FEDERATION/SHARDING_MUST_NOT_CREATE_COMPETING_TRUTH.
- LAZY_FETCH_AND_PARTIAL_MATERIALIZATION_ARE_ALLOWED.
- MUTABLE_WORKING_STATE_IS_NOT HIDDEN INSIDE IMMUTABLE SEALED SHARDS.

## Semantic sharding

Atlas SHOULD prefer semantically meaningful and query-efficient shard boundaries: repository, domain, module, graph family, evidence family, donor/research family, function shard or other Genome-approved partition. Size-based splitting may be added beneath semantic partitions.

## Incremental publication

If one scope changes, unchanged content-addressed shards are reused. The changed shard and root manifest obtain new hashes; the logical root identity changes accordingly.

## Shared immutable knowledge

Multiple logical Atlases may reference identical immutable content-addressed knowledge shards. Sharing does not merge repository ownership or mutable canonical state.

## Portable modes

A sealed Atlas may support:

- THIN: semantic artifact plus authenticated source/evidence references;
- FAT: semantic artifact plus admitted compressed source/evidence blobs;
- SHARDED: either mode split across authenticated locations.

Size is not a reason to discard engineering meaning. Completeness outranks compactness.
