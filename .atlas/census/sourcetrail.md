---
id: donor-census-sourcetrail
type: reference
status: active
canonical: true
---
# Donor Census: Sourcetrail

## Source

- Remote: https://github.com/CoatiSoftware/Sourcetrail.git
- Commit: 4b1b0e4fd19c4af235fef12b0564c05348f5f6d3
- Branch: master
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: DEEP_CENSUSED for the storage slice (G72): `src/lib/data/storage/sqlite`.

## Native Replacement

apps/ui graph and source navigation

## Campaign decision (G72, first-50 #13)

Sourcetrail's index is an element/occurrence model (`SqliteIndexStorage.cpp`: element, node, edge, symbol, source_location, occurrence, local_symbol), filled by external clang/java indexers.

Atlas already separates element identity (G66 descriptors) from per-site occurrences (CALL/DATA_FLOW records with spans). Symbol records are definitions only. The one occurrence Atlas drops, a same-scope `cfg` alternate definition, is already surfaced as AMBIGUOUS. There is no navigation consumer.

REFERENCE_ONLY. The checkout (2,408 files, 74 MB) was physically deleted. Evidence: `../evidence/campaign/13-sourcetrail.json`.

