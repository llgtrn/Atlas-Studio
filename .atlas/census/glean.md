---
id: donor-census-glean
type: reference
status: active
canonical: true
---
# Donor Census: Glean

## Source

- Remote: https://github.com/facebookincubator/Glean.git
- Commit: 2a48dea4cddb316d3b8bd65b54965cb9a7855c52
- Branch: main
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: DEEP_CENSUSED.

Deep census evidence: `.atlas/census/donors/source-intelligence-lane-2026-09-20.md`.

Observed mechanisms: Angle typed schema, source spans, source queries/statements/patterns, typechecked query IR, fact generators, storage/write queues and benchmark suites.

Decision: TARGET_MAPPED. Glean principles feed `core/model` typed facts and future `runtime/query` typed query IR. Runtime dependency remains `REFERENCE_ONLY`; Atlas will not adopt Glean's Haskell storage service or Angle as ADL.

## Native Replacement

core facts and durable fact store

## Campaign decision (G68, first-50 #9)

The recorded cheapest falsification became executable once G64 wrote `.atlas` sections. G64's name-derived `schema_id` let a version that repurposed fact field tags be silently misread by the current reader (measured: `atlas verify` exit 0).

Absorbed Glean's definition-derived schema identity as `core::atlas::schema`: declared tables that writer and reader address by name, and a `schema_id` hashed from the definition and its dependencies. The same emulation is now refused. REFERENCE_ONLY: Angle, derived predicates, the fact database and the query IR.

Terminal: ABSORBED. The checkout (15,358 files, 152 MB) was physically deleted. Evidence: `../evidence/campaign/09-glean.json`.

