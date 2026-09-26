---
id: atlas.decision.0075.compiled-package-entries
type: decision
status: accepted
canonical: true
---
# ADR 0075 — Workspace packages entered through their declared compiler directories (G160)

## Context

FULL_OSS_REPLAY R6 revalidated GitNexus at its pin `06ce60beb674`. Its E2 verdict was re-opened at E7, because E7 added cross-file TypeScript module resolution, the first condition of its trigger. Atlas E8 resolved 45,345 call edges on the pin; E2 had recorded 21,045.

One package boundary stayed dark. `gitnexus` and `gitnexus-web` import 495 names from the workspace package `gitnexus-shared`. That package's `package.json` declares only compiled entries (`main: dist/index.js`, `types: dist/index.d.ts`, an `exports` map into `dist/`) and no `source` field.

The challenge question was: who is affected by a change to `scoreImpactRisk`, GitNexus's impact risk scoring in `gitnexus-shared`?
- Atlas E8 answered: nobody.
- The source shows three sets of callers:
  - the local backend's `_runImpactBFS`;
  - the web impact tool;
  - eight test cases.

## Decision

1. **A package declaring no `source` is entered at the source its compiled entries are emitted from, when its own `tsconfig.json` declares how.** A compiled entry is one of:
   - `types` or `typings`;
   - `module` or `main`;
   - every string of the `.` export; subpath exports are ignored.
2. **The mapping.** When the sibling `tsconfig.json` declares both `compilerOptions.rootDir` and `outDir`:
   - an entry under `outDir` maps to the same path under `rootDir`;
   - `.d.ts` and `.js` map from `.ts` or `.tsx`, `.mjs` from `.mts`, and `.cjs` from `.cts`;
   - this is the compiler's own emit rule, read from the package's declarations.
3. **Every mapped entry must name the same existing file.** Otherwise the package has no entry and nothing is resolved:
   - a missing `rootDir` or `outDir`;
   - an entry whose source is not in the inventory;
   - two entries naming different sources.
4. **Parsing.** `tsconfig.json` is parsed as JSON with comments and trailing commas. `extends` is not followed; an inherited `rootDir` or `outDir` counts as undeclared.

## Consequences

- **The same pin at E8 and E9:**
  - resolved call edges went from 45,345 to 45,746;
  - unresolved call sites went from 207,456 to 207,005.
- **The challenge question:**
  - `scoreImpactRisk` has 10 callers, all correct against the source: `_runImpactBFS`, the web impact tool's closure, and the eight test closures;
  - `makeScopeId` went from 0 to 26 callers;
  - a random sample of 20 of the 401 new edges was checked, and all 20 were correct.
- **Still unresolved:** member calls, `this` calls, namespace imports and JSX. GitNexus's other re-open conditions, execution flows and hunk seeds, are still absent from Atlas.
