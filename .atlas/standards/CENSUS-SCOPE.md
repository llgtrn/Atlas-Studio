---
id: atlas.standard.census-scope
type: contract
status: active
canonical: true
---
# Census Scope Standard

Atlas census is exhaustive in accounting and adaptive in semantic depth.

## Scope lattice

```text
S0  Federation / multi-repository world
S1  Repository
S2  System / domain
S3  Package / crate / application
S4  Module
S5  File
S6  Type / trait / interface / component
S7  Function / method
S8  Control-flow block
S9  Statement / expression
S10 Semantic atom
```

Every admitted artifact and every discovered S7 function/method is accounted for. Adaptive zoom decides how far below S7 to lower, except critical scopes where Genome may force S10 closure.

The scope lattice is not confined to one repository. Dependency closure can expand one S1/S3 scope into additional repository/package scopes recursively. Those dependency scopes remain sovereign identities in the same S0 federation and are linked by typed dependency edges.

## Semantic atom examples

CALL, LOAD, STORE, READ_STATE, WRITE_STATE, TRANSITION, EMIT_EVENT, AUTH_CHECK, BORROW, MOVE, COPY, ALLOC, FREE, LOCK, UNLOCK, ATOMIC, PERSIST, NETWORK_SEND, FFI_CALL, BRANCH, PANIC and RETURN.

## Dependency expansion

Before a root scope can close, Atlas applies `../contracts/DEPENDENCY-CENSUS.md` for every admitted resolution context.

Direct and transitive runtime, build, dev/test, proc-macro/codegen, toolchain, native/system, dynamic/plugin and external capability dependencies are explicit. Source-backed dependencies enter the same scope lattice; opaque/external boundaries receive explicit terminal records.

Semantic depth may adapt inside a dependency, but dependency existence and edge accounting may not.

## Mandatory lenses

At each applicable scope Atlas evaluates identity, structure, type/schema, dependency, control flow, data flow, state, event, temporal, effect, authority/security, concurrency, persistence/recovery, performance, interface/capability, binding, evidence, provenance, testing, external interaction, ownership, invariants, unknowns and invention opportunities.

## Adaptive triggers

Deepen on architecture importance, state mutation, durability, concurrency, authority/security/safety, performance hot path, external effect, evidence conflict, complex/dynamic binding, compiler-sensitive ownership/layout or planned change.

## Multi-direction reconciliation

Bottom-up facts aggregate to enclosing scopes. Top-down claims must resolve downward to concrete implementations/evidence. Mismatches are blockers/conflicts, not silently tolerated.

## Dynamic behavior

Reflection, dynamic loading, macros/proc-macros, generated code, FFI, SQL/shell strings, feature flags, environment/config-driven behavior and plugin discovery require explicit resolved/dynamic/unknown binding records.

## Completion

High-level completeness is not a percentage alone. A scope closes only when its Genome-defined obligation set is fully accounted as verified, explicit none, explicit unknown, explicit conflict or explicit unsupported, and any policy-forbidden unresolved state is zero.
