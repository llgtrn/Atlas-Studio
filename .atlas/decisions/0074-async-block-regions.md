---
id: atlas.decision.0074.async-block-regions
type: decision
status: accepted
canonical: true
---
# ADR 0074 — Async blocks as executable regions (G159)

## Context

G133 (ADR 0052, NA-CLOSURE-REGIONS) made a closure its own executable region: its calls, effects, state accesses and the rest belong to a CLOSURE function named `{closure@line:column}` and enclosed by the function that defines it.

An `async { .. }` block is the same kind of thing: a future whose body runs when it is polled — later, possibly never, possibly from another task. Until G159 every walker skipped it. The syntactic extractor and the path resolver left its calls outside the CALL profile, so they were never attributed anywhere. The macro-opacity count also skipped its bodies.

## Decision

1. `FunctionDeclarationKind::AsyncBlock` ("ASYNC_BLOCK") names the region of an `async` block.
   - It is named `{async@line:column}`.
   - It is scoped under `fn <enclosing region>`, exactly like a closure.
   - `is_deferred_region` is true for closures and async blocks.
2. The syntactic extractor walks every dimension of an async block's body into that region.
   - Data flow has no parameters there. Captures are uses of the enclosing region's bindings.
   - The enclosing function is never credited with them.
3. The path resolver resolves calls inside an async block with the enclosing region's locals plus the block's own bindings, as it does for closures.
4. The macro-opacity count includes async bodies, since they are now in the CALL profile.
5. Composition encloses an ASYNC_BLOCK region by its defining function (ENCLOSES), so the impact frontier crosses it as it crosses a closure.

## Consequences

- An async block's calls are attributed and resolvable. On the agent fixture, `later`'s async block holds the resolved call to `helper`, and `later` does not.
- Atlas Studio has almost no async blocks, so its own census changes little. The capability matters for async donors and for construction, where a deferred region must not be confused with its creator.
- Async functions (`async fn`) were already functions. What `.await` suspends on and which executor polls a future stay outside every engine (DEBT-CONCURRENCY).
