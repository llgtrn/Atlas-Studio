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

## Verification

Run dependency closure scans and production-like runtime tests without donor technology available.
