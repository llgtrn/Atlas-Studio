---
id: atlas.decision.0052.closure-regions
type: decision
status: accepted
canonical: true
---
# ADR 0052 — Closure bodies as their own executable regions (G133, NA-CLOSURE-REGIONS)

## Context

Every Rust walker (CALL, CONTROL_FLOW, DATA_FLOW, STATE, EFFECT, OWNERSHIP, CONCURRENCY, PERSISTENCE) and the path-resolution engine refused to enter a closure body.

- **Why.** A closure runs later, possibly never and possibly from another caller, so attributing its calls to the enclosing function would be false. With no identity for a closure, its contents were simply outside the census.
- **The cost.** The G74 measurement counted 1,476 invisible calls. G117 found the scoped thread spawns hidden there. Every impact residual had to say "calls inside closure bodies are not censused at all".
- **Pressure.** This was the pressure map's P0 queue head, blocking `DEBT-CALL`, `DEBT-CONCURRENCY`, `DEBT-EFFECT`, `DEBT-DATA_FLOW`, `DEBT-OWNERSHIP`, `DEBT-STATE` and `DEBT-CONTROL_FLOW`.

## Decision

1. **A closure is its own executable region.**
   - **Identity.** It gets a `FunctionIdentity` with `declaration_kind = CLOSURE`, named `{closure@line:column}` and scoped under `fn <enclosing region>`; a nested closure's scope continues the chain. Its identity is structural, like any function's.
   - **Where regions start.** The CALL walker starts a region when it meets a closure. The eight dimension builders then run on the closure body (`walk_region`), attributed to the closure's record. Every other walker keeps skipping closure bodies, so nothing is attributed twice.
   - **Parameters** are the region's own data-flow definitions.
   - **Exclusions.** A closure outside any function (a `const` or `static` initializer) stays outside the profile. So do `async` blocks.
2. **The path-resolution engine walks closure bodies too.** Its locals are the enclosing region's (captures) plus the closure's own bindings, so a parameter that shadows a function name stays a local.
3. **Consistent profile.** Opaque-macro counting follows the same profile (closure bodies counted, `async` blocks not), as does the independent call-site enumeration oracle.
4. **Composition.** A closure region is a `FunctionBehavior` of kind `CLOSURE` whose `enclosing` names the region that defines it, `DERIVED` from its scope. It has no declared signature, so no visibility.

## Evidence

- **The self census:**
  - 2,591 closure regions, every one linked to its enclosing region;
  - 3,319 call sites now attributed to closures;
  - resolved call edges rising from 3,514 to 3,786;
  - 97 effect sites inside closures now visible.
- **An authority finding the old census could not see.** Two CLI test functions spawn the CLI binary from inside a closure (`AtlasCli: PROCESS_SPAWN`). The world model's invariants otherwise held (`verify` HELD).
- **Mission M1's falsification finding is resolved.** M1 (G123) found 8 of the 13 textual `.label(` calls of `lens.rs` invisible because they sat in closures. `impact(Index::label)` now names 16 INFERRED candidate call sites in 12 functions, up from 6 in 5. Benchmark B07 no longer expects the residual to name `NA-CLOSURE-REGIONS`.
- **The adapter test** covers a closure's call belonging to it, not to its enclosing function; the nested naming; the const-initializer and `async` exclusions; and closure parameters as definitions.
- **The fixture** shows a closure's resolved call to `helper` being the closure's, linked to `apply`.
- **The resolution test** shows a shadowing parameter staying local.
- **The independent enumeration oracle** agrees on every real source file.

Seven mutants were each caught by a test:

- closures skipped again;
- a closure body attributed to its enclosing function;
- a closure scope not nested;
- opaque sites inside closures uncounted;
- closure parameters unbound (the first form of this assertion was too weak and was strengthened);
- the resolver skipping closure bodies;
- closure parameters not shadowing.

## What stays open

- **Async blocks.** They are deferred regions outside the profile.
- **The G117 scoped spawns** sit inside closures but are method calls on the scope handle. They wait on receiver typing (`NA-CALL-TYPE-RESIDUAL`), not on regions.
- **Invocation is unknown.** Nothing records that a function invokes the closure it defines. The `enclosing` link is structural, not a call.
