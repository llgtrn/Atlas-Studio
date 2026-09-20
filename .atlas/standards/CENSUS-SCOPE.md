---
id: atlas.standard.census-scope
type: contract
status: active
canonical: true
---
# Census Scope Standard

Atlas census is multi-resolution and adaptive.

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

A semantic atom is the smallest engineering fact Atlas needs to reason about, for example: a state write, authority/effect check, ownership transfer, call, binding, serialization boundary, lock/atomic operation, event emission, persistent write or invariant guard.

## Adaptive zoom

Atlas MUST NOT fully explode every repository to S10 by default. It deepens census when triggers include:

- architectural importance;
- state mutation;
- persistence/durability;
- concurrency/atomicity;
- authority/security/safety;
- performance hot path;
- external effect;
- ambiguity/conflicting evidence;
- complex dependency/binding;
- compiler-sensitive ownership/layout;
- target plan touches the scope.

## Planning completeness

Every material change must reconcile:

```text
global repository/federation impact
+
local module/function implementation
+
atomic semantic behavior
```

Plans must be able to zoom down and then propagate consequences back up.

## Evidence at every scope

A conclusion at a high scope must be traceable to lower-scope evidence or explicit synthesis. A low-level fact must remain attributable to its repository revision/source span/test/paper/spec or other admitted evidence.
