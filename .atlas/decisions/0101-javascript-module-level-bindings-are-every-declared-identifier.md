---
id: atlas.decision.0101.javascript-module-level-bindings-are-every-declared-identifier
type: decision
status: accepted
canonical: true
---
# ADR 0101 — A JavaScript/TypeScript module-level variable is every identifier a declaration binds outside every function; a name fixes a callee only where its one binding is visible (G188)

## Context

G188 is the HARDENING lane of FULL_OSS_REPLAY R20 (666ghj/mirofish, pin `39d84913`). The replay measured a falsification in the JavaScript/TypeScript frontend (ADR 0068, `adapter::semantic::typescript`).

ADR 0068 and the module doc declare SYMBOL as "definitions of functions, classes, interfaces, type aliases, enums and module-level variables; imported names as declarations", and an error-free parse makes SYMBOL exhaustive: the file's SYMBOL obligation is OBSERVED. The walker recorded a module-level variable only for a `variable_declarator` whose name is a plain identifier. Every other binding outside a function was missing while SYMBOL stayed OBSERVED:

- **Donor.** mirofish `frontend/src/i18n/index.js:8` `for (const path in localeFiles)`: an acorn parse found 70 module-scope bindings and definitions, Atlas recorded 69.
- **Destructuring.** `const { a, b: c, ...r } = o`, `const [d, , ...e] = arr`, `export let { n } = o` and `const { x = 1, y: { z } } = o` recorded nothing.
- **Catch and TypeScript.** A module-level `catch (e)`, `import A = N.B` and `import x = require("m")` recorded nothing.
- **`using`.** A TypeScript `using x = ..` or `await using x = ..` parses as an assignment carrying a `using` token and recorded nothing.
- **Atlas.** The browser scripts missed `const { chromium } = require(..)` and `const { spawn } = require(..)`. Over Atlas's 2,481 tracked JS/TS files, 552 module-level names (633 binding sites) were missing from files whose SYMBOL was OBSERVED.
- **False records in UNKNOWN files.** Namespace and ambient-module members were recorded at module scope. A class method with a computed or string name was walked as module-level code, so its locals became definitions under the class and its calls were attributed to `{module}`.

The CALL resolver fixes a bare callee (or `new C`) to a same-file function or constructor when the name is "bound exactly once in the file and never assigned" (ADR 0068). That check had the same blind spot and produced false DERIVED edges. The cases:

- Loop heads (`for (const f of fs) f()`) were not counted as bindings.
- Destructuring and loop-head writes (`[f] = [g]`, `for (f in o)`) and `f++` were not counted as writes.
- Named function and class expressions were not counted. `const g = function f() { return f(); }` resolved the inner `f` to a module `f`; `const Q = class K { m() { return new K(); } }` resolved to another `K`'s constructor.
- Block functions (`{ function f() {} } f()`, block-scoped in module and strict code) and namespace members (`namespace N { export function g() {} } g()`) were treated as module functions.
- A `with` body was ignored.
- A computed key (`k` in `{ [k]: v } = o`) was counted as a binding.

The module linker (ADR 0073) read exported names without destructuring, enums, namespaces (`export module M {}`, `export namespace M.B {}`), `export import M = N.X` or ambient declarations. A local export shadows `export *`, so a missed name let a star export resolve an import to another module's function. For example, `export * from './fns'; namespace N { export function X() {} } export import M = N.X;` resolved `M` to `fns.ts::M`, while `tsc` resolves it to `N.X`.

## Decision

1. **Module-level variable.** Every identifier bound outside every function region is a module-level variable: at the top level, or in a top-level block, loop or class static block. The binding site is any of:
   - a `var`, `let` or `const` declarator (`declare const` included: it binds a variable);
   - a `for (var|let|const .. in|of ..)` head;
   - a `catch` parameter (treated like a module-level block `const`, which was already recorded);
   - a TypeScript `import A = N.B` alias.

   Destructuring patterns are walked to every identifier they bind. The walk takes a shorthand, a pair's value (never its key, computed keys included), an element, a rest target, and a defaulted target without its default. Each identifier is a SYMBOL definition anchored at the identifier, with the walk's scope (`[]` at the top level, the class for a static block).

   A loop head without a declaration keyword assigns and binds nothing. Other function-local bindings are not recorded. The exception is a function-valued declarator (`const f = () => ..`): as before, it is a function wherever it is, so its name is a definition in its enclosing function's scope.

   A bodiless signature (an overload, `declare function f()`) is not a function (ADR 0068) and not a definition. `declare const x` is a variable, and variables are recorded whether or not they are ambient.
2. **Identity and anchor.** A symbol is its (scope, name). Several bindings of one name in one scope are one record, anchored at the first binding in source order. This is the record contract both extractors apply: a record id is recorded once and the first recording wins (`record_dimension_hit` in the Rust extractor, `emit` here).
3. **Imports.** `import x = require("m")` is an imported name: a declaration under `import m`.
4. **Namespaces and ambient modules.** A namespace or ambient module body declares members of another scope. Its members are scoped under `namespace N` (or `namespace A.B`), `module "m"` or `global`, and are never module-level.
   - A file with such a body keeps its records, but its SYMBOL is UNKNOWN, since the profile does not enumerate those scopes.
   - A bodiless `declare module "m";` declares nothing and changes nothing.
   - A module-level `using`/`await using` declaration also makes the file's SYMBOL UNKNOWN. The assignment-with-`using` shape is also what `using\nx = 1` parses to, and that is not a declaration, so it is not guessed.
   - A class method with a computed or string name keeps the file partial, and its body is a closure region like an object-literal method. A computed method key (`[f()]() {}`, in a class or an object literal) is walked in the enclosing scope, where it is evaluated.
5. **Same-file resolution.** A bare callee `f` at a call site is fixed to a function of the file only when all of the following hold:
   - the function is declared outside every function;
   - its declaration is visible at the call site: the module, block, `switch` body or namespace body its statement sits in, or, for a `var`, the module or namespace body;
   - `f` is bound exactly once in the file, with loop heads, catch parameters, `import =` and `declare function` included (an overload signature does not rebind its implementation);
   - `f` is never assigned, with assignments, destructuring and loop-head assignments and `++`/`--` included;
   - no named function or class expression rebinds `f` around the call (its name binds inside itself only, unless its declarator binds the same name);
   - the file has no `with` statement and no direct `eval(..)` call. A direct eval can declare a `var` in the scope it runs in (sloppy code) or assign any binding it can see (`eval("f = 2")`), so, like `with`, it disables same-file resolution for the whole file. A callee in parentheses or behind TypeScript's type-only wrappers (`(eval)(..)`, `(eval as any)(..)`, `eval!(..)`, `(<any>eval)(..)`, `(eval satisfies any)(..)`) is still a direct eval: the wrappers are unwrapped before the name is compared. An indirect eval (`(0, eval)(..)`, `window.eval(..)`), `new Function(..)` and `globalThis.f = ..` run in the global scope: they cannot touch a module's or a CommonJS file's bindings, and in a script they are the global-scope case of RES-G188-SCRIPT-GLOBAL-SCOPE;
   - module-level code does not reach the call before the declaration gives `f` its value. A class, or a function bound by `let`/`const`, is in its temporal dead zone until its declaration runs; one bound by `var` is `undefined` until then. A function declaration is hoisted with its value. A call in a nested function body is taken to run later;
   - in a script (a file with no top-level `import` or `export`), a call inside a namespace body resolves only to a declaration of that same body, because another script's same-named namespace can merge in a member that shadows an outer function (`namespace N { export function f() {} }`).

   `new C` is fixed to a constructor under the same rule, for a class declaration or a class expression its declarator binds to the class's own name. Only a declaration visible module-wide is a module function for the linker, and an import that a named expression rebinds is not fixed.
6. **One pattern walk.** `pattern_identifiers` serves every use:
   - the binding count;
   - the write check (a member or subscript target writes no binding);
   - module-level SYMBOL definitions;
   - the linker's exported names (`declared_names`): function, class, enum, namespace and module names (a dotted namespace binds its leftmost name, a string-named module binds none), every identifier of an exported destructuring pattern, `export import` aliases, and the names inside `export declare ..`.

   A local ambient declaration, a non-instantiated namespace or a `const enum` shadows `export *` only in the type system. It emits no JavaScript, so at run time the star may supply the name. The import is then left unresolved: the edge is withheld (UNKNOWN), never a wrong one.
7. This ADR amends ADR 0068's SYMBOL and CALL profile text. ADR 0068 is not rewritten, and the module doc of `adapter/src/semantic/typescript/mod.rs` states the rule.

## Consequences

N and N+1 were measured with the same inputs: the release binary at HEAD `45eb07d2` and the binary after the change, running `systemize`, `agent model` and `recensus snapshot`. For Atlas itself, both binaries ran on the same working tree.

- **mirofish (9 JavaScript files).** SYMBOL rose from 69 to 70 (`path`). CALL (61), FUNCTION_IDENTITY (45), FUNCTION_SIGNATURE (45), relations (4, INVOKES 0) and all 334 obligations are identical, and no status changed.
  - An oracle over the TypeScript compiler's parser, independent of tree-sitter, found 0 missing and 0 extra. It covered 47 top-level definitions, 23 import declarations and 12 module-level binding sites, and every anchor line matched.
  - acorn found 39 module-level bindings: 1 was missing before the change and 0 after.
- **Atlas (`agent model --root .`, whose census holds the 3 browser scripts).** SYMBOL rose from 12,641 to 12,643. JavaScript went from 37 to 39 (`chromium`, `spawn`). Every Rust dimension, the 61,763 CALL records, the 7,816 INVOKES and the 3,329 obligations are identical.
- **Atlas's 2,481 tracked JS/TS files** were taken as a git probe with `.atlas/` renamed to `_atlas/`. Atlas extracts 2,462 of them; the other 19 are outside its inventory.
  - SYMBOL rose from 38,538 to 39,128: JavaScript from 16,603 to 17,179, TypeScript from 21,935 to 21,949.
  - The TypeScript-compiler oracle ran over the 2,454 files with an OBSERVED SYMBOL and found 0 missing and 0 extra. It covered 15,521 top-level definitions, 3,824 nested definitions, 18,831 import declarations and 7,230 module-level binding sites. 6,623 anchor lines were checked with 0 mismatches. Before the change, 552 top-level names were missing.
  - acorn found 6,315 module-level bindings in 1,113 files: 539 were missing before the change and 0 after.
  - Three `.d.ts` files went from SYMBOL OBSERVED to UNKNOWN: cytoscape `index.d.ts` (`declare namespace`) and two keycloak `i18next.d.ts` files (`declare module`).
  - FUNCTION_IDENTITY and FUNCTION_SIGNATURE rose from 38,057 to 38,059. These are the two computed-name class methods, now closure regions: `kernel.js:333` and `code_size_base.js:36`, `[Symbol.iterator]()`. ENCLOSES rose by 2.
  - CALL records (186,548) are identical. Resolved calls fell from 24,483 to 24,462 and INVOKES from 16,861 to 16,840 (CALL DERIVED 14,964 → 14,943). No callee changed and nothing new resolved.
  - The 21 withheld resolutions are all in files with a direct `eval`. 18 are `getNewPrototype(..)` calls in the six copies of protobuf's `js_benchmark.js` and `protobufjs_benchmark.js`, whose `getNewPrototype` runs `eval("proto." + name)`. 3 are `loadCy()` calls in cytoscape's `documentation/js/script.js`, which runs `eval(text)`. The edges were very likely true, but a direct eval can assign any binding it sees, so they are withheld (UNKNOWN), not false.
  - The other false-edge shapes above (and the temporal-dead-zone and script-namespace cases) do not occur in this corpus. In the review fixtures (`cx`, `lx`), the 12 calls that resolved before are now unresolved: 7 same-file and 5 cross-module. 10 of them were false edges. The other 2 are withheld: `idx3` exports an alias of an undefined `N`, and `idx6` exports a bodiless ambient module that shadows the name only in the type system.
- **Identity churn.**
  - **SYMBOL.** 236 ids were retired, all namespace or ambient-module members moved from scope `[]` to their scope, in the three files now UNKNOWN: cytoscape 234 became 235 records (193 under `namespace cytoscape`, 42 under `namespace cytoscape/namespace Css`), and keycloak 1 + 1 became `module "i18next"`.
  - **Re-anchored.** Three kept SYMBOL records moved to an earlier binding of the same name in the same scope, which is now recorded and comes first in source order. All three were a module-level loop head before a later `const`:
    - `tools/universal-graph/capability-registration/tests.mjs` `c`: 1214:8 → 759:11;
    - `tools/universal-graph/tests.mjs` `fx`: 149:8 → 81:11;
    - `tools/capabilities/reconcile-evidence.mjs` `e`: 85:8 → 73:42.
  - **Other SYMBOL.** Every other pre-existing SYMBOL id is kept, with its anchor: mirofish kept 69 of 69, the probe 38,299 of 38,538. An identifier declarator starts at its identifier.
  - **FUNCTION_IDENTITY.** None retired, 2 added.
  - **CALL.** One record, `kernel.js:334`, moved from the `{module}` region to its method's closure region, so it has a new id.
- **Tests** (`adapter/src/semantic/typescript/tests.rs`, `modules/tests.rs`):
  - Every JavaScript and TypeScript binding form is recorded, and function, arrow, callback and method locals and an `import =` inside a function are not.
  - Namespace, dotted module, `declare module` and `declare global` members are scoped under them. A module-level `using`/`await using` makes SYMBOL UNKNOWN, while a bodiless ambient module and a function-local `using` do not.
  - A computed-name method body is a region.
  - A plain variable keeps its pre-change id and anchor, a destructured binding is anchored at its identifier, and a name bound twice is anchored at its first binding.
  - Every false-edge fixture stays unresolved: named expressions, `++`, block and namespace functions, `with`, direct `eval`, `import =`, `declare function`, class expressions under another name, calls before a class or `const`/`var` function's declaration, and a script's namespace body.
  - Resolution holds where the one binding is visible: a declarator naming its own expression, an export, a block `const` in its block, a `var` in a top-level `if`, a function declared in a `case` called from another `case`, a hoisted function declaration called before it, a nested call to a later `const`, a module file's namespace body, an overloaded function, and a module function outside a same-named function expression.
  - A computed method key's calls are recorded in the enclosing region, in classes and object literals, and a computed-name method keeps the file partial.
  - Destructured, enum, namespace, module, dotted (three levels: tree-sitter nests `A.B` as a member expression), `export import` and ambient local exports shadow a star export, and a string-named module exports nothing. Block functions and rebound imports are not module facts.
- **Mutants.** There were 61 hand mutants on the new logic (`$S/g188fix/mutants.md`). 60 were killed. One is equivalent: a declaring loop head counted as a write instead of a binding defeats "bound once, never assigned" exactly as the second binding does. A redundant name-kind filter in `declared_names`, found equivalent by mutation, was removed.
- **Residual RES-G188-STAR-NONFUNCTION-CONFLICT, not changed here.**
  - When two star exports provide one name, the linker counts only the branches that resolve to a function. A branch that provides the name as a non-function, or through an unresolved specifier, or deeper than `MAX_LINK_DEPTH` (16) is counted as not providing it, so the import resolves to the other branch's function.
  - Under ES rules the name is ambiguous in the first case, and importing it is a link-time SyntaxError. In the other two cases Atlas cannot see the provider.
  - In every case where the provider exists, the program does not link, so the false edge arises only in a program that does not run.
- **Residual RES-G188-SCRIPT-GLOBAL-SCOPE, older than this change.**
  - Script files (no top-level `import`/`export`) loaded as browser `<script>`s share one global scope. Another script can redefine a module-level function (`function f() {}` in `b.js` loaded after `a.js`), and then `a.js`'s `f()` runs `b.js`'s `f`.
  - Atlas does not know how scripts are loaded: CommonJS files, the common case, are module-scoped by Node's wrapper. It still resolves a script's `f()` to the file's own `f`.
  - The script namespace rule above covers only namespace bodies.
- **Residual RES-G188-TDZ-NESTED-CALL.** A call in a nested function body is taken to run after module-level code. A callback that module-level code invokes immediately (`[1].map(() => f()); const f = () => 1;`) throws before `f` is initialized, yet the call resolves to `f`.
- **Not fixed here.** Calls in a class `extends` clause are not walked (`const C = class f extends (f()) {}`). That stays under RES-G186-TS-UNWALKED-CALL-POSITIONS.
