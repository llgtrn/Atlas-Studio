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

