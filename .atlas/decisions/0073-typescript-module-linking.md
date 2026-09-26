---
id: atlas.decision.0073.typescript-module-linking
type: decision
status: accepted
canonical: true
---
# ADR 0073 — TypeScript/JavaScript module linking: the second CALL engine (G158)

## Context

FULL_OSS_REPLAY R5 replayed xyflow/xyflow at its historical pin `0a1f9575b256`. xyflow is a TypeScript monorepo:
- `@xyflow/system` is the framework-agnostic core;
- `@xyflow/react` and `@xyflow/svelte` build on it;
- the examples import from the packages.

The replay also measured TypeScript on the support ladder for the first time (ADR 0071). TypeScript reached L6: 451 artifacts and 2,249 composed functions.

Atlas_N (capability epoch E6) resolved 147 call edges and left 4,796 call sites unresolved. The challenge question was: what does a change to `getBezierPath` (`packages/system/src/utils/edges/bezier-edge.ts`) put at risk?
- Atlas_N answered: nothing. The function had no callers.
- The source shows 10 TypeScript callers:
  - React's `BezierEdge` and `ConnectionLine`;
  - 8 example components.
- It also shows 3 Svelte components, which Atlas cannot see.

The G152 profile resolves a bare identifier only to a function of its own file. Every callee reached through an `import` stayed unresolved.

## Decision

1. **A second CALL engine for TypeScript/JavaScript**, `atlas.resolution.typescript-modules`. It links modules the way ECMAScript does, and it never guesses.
   - **Module facts**, read with the same syntax substrate and binding rules as the extractor:
     - named and default imports; type-only and namespace imports bind no value;
     - exports: declarations, `export { a as b }`, `export default f`;
     - re-exports: `export { a } from 's'`;
     - star exports: `export * from 's'`;
     - module-level functions whose name is bound exactly once and never assigned.
   - **Specifiers:**
     - A relative specifier names the first existing file in a fixed order:
       - the path itself;
       - a `.js`/`.jsx`/`.mjs`/`.cjs` spelling of a `.ts`/`.tsx`/`.mts`/`.cts` source;
       - the script extensions;
       - then `index` files.
     - A bare specifier names a workspace package only when exactly one inventoried `package.json` declares that `name` and a `source` entry that is in the inventory.
     - Registry packages, subpaths, `tsconfig` path aliases and package `exports` maps name nothing.
   - **Bindings:**
     - An imported name follows exports, then explicit re-exports, then star exports, to the function that defines it. An explicit export wins over a star export.
     - Two star exports that provide different functions under one name are ambiguous, and nothing is resolved.
     - A cycle, or a chain deeper than 16 hops, resolves to nothing.
2. **Same claim, second engine.**
   - A CALL claim is observed again under the new engine's identity when both hold:
     - its callee is a bare identifier bound once in its file;
     - that binding is an import resolved as above.
   - The observation keeps the same `record_id`, sets `dispatch: STATIC_RESOLVED`, and names the defining function's FunctionIdentity record as its callee. Its status is DERIVED, and its evidence names the file chain.
   - The engine's CALL obligation is UNKNOWN on every TypeScript/JavaScript artifact. The following are outside it:
     - member calls;
     - namespace imports;
     - JSX elements;
     - dynamic `import()`;
     - `require`;
     - aliases.
3. **RESOURCE is canonical.** G157 left RESOURCE out of the canonical dimension list that every extractor is asked to account for. Each syntactic extractor therefore silently omitted it. It is now requested from every extractor, and the syntactic extractors answer UNSUPPORTED.

## Consequences

- **The same pin at E6 and E7:**
  - resolved call edges went from 147 to 845;
  - relations went from 1,440 to 2,138;
  - unresolved call sites went from 4,796 to 4,043;
  - TypeScript functions with a DERIVED relation went from 1,748 to 1,895.
- **The challenge question:**
  - `getBezierPath` has 10 callers, exactly the 10 TypeScript call sites in the source;
  - a random sample of 25 of the 698 new edges was checked against the source, and all 25 were correct.
- **What stays out:**
  - The 3 Svelte callers stay invisible. `.svelte` is L0, because there is no frontend for it.
  - Member calls (`store.getState()`), JSX component uses and hooks called through objects stay unresolved.
- **The next attacks are NA-MULTILANGUAGE's remaining parts:**
  - member and `this` calls;
  - types and data flow;
  - a Svelte frontend.
