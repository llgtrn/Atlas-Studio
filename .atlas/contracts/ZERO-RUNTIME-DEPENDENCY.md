---
id: atlas.contract.zero-runtime-dependency
type: contract
status: active
canonical: true
---
# Zero Runtime Dependency Contract

## Hard Invariants

A technology may be labeled NATIVE_ZERO_DEP only when its production runtime can execute without donor libraries, donor services, donor binaries, donor databases or donor processes.

## Interfaces

Build/test tooling may use reference tools temporarily. Production runtime interfaces may not require them.

## State and Durability

Proof records donor lineage, native implementation SHA and dependency scan.

## Authority

Atlas may recommend extinction only after proof. It cannot relabel transitional technology as native.

## Evidence

Dependency graph, binary/package scan, runtime launch proof, differential tests and rollback evidence.

## Recovery

If a hidden dependency is found, downgrade maturity and restore the dependency-removal work plan.

## Clean donor-disappearance proof

NATIVE_ZERO_DEP requires more than removal of imports.

Before an EXTINCT claim, execute the isolated extinction probe in `RECURSIVE-SELF-CENSUS.md`: donor source unavailable, Atlas-controlled substitutes absent, silent donor re-fetch disabled, then build/test/recensus and run relevant scenarios.

Cached donor source, generated donor artifacts required for rebuild, helper binaries, donor services, build tools or hidden environment lookups count as dependencies when required by the claimed native scope.

Where R8 source-independent materialization exists, exercise that continuation/rebuild path too.

## Verification

Run dependency closure scans, clean-environment rebuilds and production-like runtime tests with donor technology genuinely unavailable.

If a later stronger generation finds hidden dependence, supersede the old conclusion with corrective evidence and a new state lineage; never silently rewrite history.
