---
id: atlas.decision.0098.javascript-call-sites-anchored-at-the-callee-name
type: decision
status: accepted
canonical: true
---
# ADR 0098 — A JavaScript/TypeScript call site is anchored at its callee's name, so the calls of a chain stay distinct (G186)

## Context

G186 is the HARDENING lane of FULL_OSS_REPLAY R19 (666ghj/bettafish, pin `c4ca6360`). The replay measured a falsification in the JavaScript/TypeScript frontend (ADR 0068, `adapter::semantic::typescript`).

`CallSiteIdentity::span` is defined by the G74 contract in `core/src/semantic/call.rs`: a call site is anchored at the callee's name token, so the calls of a chain stay distinct. The Rust extractor follows it (`call_anchor` in `adapter/src/semantic/rust/mod.rs`). The TypeScript frontend did not. It anchored every call and `new` expression at the expression's first token. Every call of a chain shares that token: `a(b).c()`, `page.locator(x).first().hover()`, `new Box().open()`, `(async () => {..})().catch(..)`, and any call whose callee is a call.

Those calls therefore got one `CallSiteIdentity` (function, line and column). The extractor records a record id once. The walk meets the outer call first, so the outer call was recorded and the inner calls vanished. They were not resolved, not unresolved and not unnamed.

- **Donor.** On bettafish's 7 JavaScript files Atlas recorded 29,100 CALL records against 30,118 call and `new` expressions counted by an independent acorn parse. 1,018 sites were silent.
- **Atlas.** In its own browser scripts (`adapter/src/browser/*.js`) Atlas recorded 189 of 210 sites. For example, `observe.js:150` `await page.locator(toSelector(target)).first().hover();` recorded `hover` but not `locator` or `first`.
- **Minimal repro.** For `function viaChain() { return inner().m(); }`, Atlas recorded `viaChain` with one unresolved call (`m`) and no edge to `inner`. `impact inner` answered 0 callers and 0 candidate sites.

Three answers were false or understated as a result: `impact`'s `candidate_sites` and `unnamed_sites`, `hypothesis invokes`'s "N unresolved call sites … spelled with the callee's name", and the unresolved-site notes of `trace` and `understand`. No verdict reached NO_PATH or FALSIFIED on the evidence (JavaScript CALL coverage is UNKNOWN), but the counts behind them were wrong.

## Decision

1. **The anchor is the callee's name token**, per callee form (`call_anchor` in `adapter/src/semantic/typescript/mod.rs`):
   - For `f()`, `f<T>()`, `f?.()` and a tagged template `` f`..` ``, the anchor is the identifier `f`.
   - For `x.m()`, `x?.m()` and `x.#m()`, it is the property name `m`.
   - For `new C()` and `new C`, it is the constructor name `C`. For `new ns.C()`, it is the property `C`.
   - For `super()` and `import(..)`, it is the keyword, which is the whole callee.
   - Any other callee is not a name: `(f)()`, `a()()`, `(async () => ..)()`, `a[k]()` and `new (make())()`. The anchor is then the first token of the argument list: its `(`, or a tagged template's backtick. For a `new` with no arguments whose constructor is not a name (`new (f())`), it is the `new` keyword.

   Each of these tokens belongs to exactly one call or `new` expression. This mirrors the Rust extractor, which anchors at the method identifier, the last path segment, or the argument list's `(`.
2. **One code path serves every grammar.** `.ts`, `.mts`, `.cts`, `.tsx`, `.js`, `.mjs`, `.cjs` and `.jsx` all go through `Context::defer_call`. The test corpus runs under `.js`, `.ts`, `.tsx` and `.mjs`. The import-linking engine (`runtime::census::typescript_modules`) re-observes the adapter's claims by their `record_id` and span and derives no anchor of its own, so no other place derives a call anchor.
3. **The identity changes wherever the contract says it must.** The anchor stays the same only when the name token is the expression's first token: a bare `f()`, `super()` and `import()`. Every member, `new` and non-name call site gets a new identity. Keeping the old identity for member calls that did not collide would make a site's identity depend on its neighbours. The G74 contract rules that out.

## Consequences

Before and after were measured with the same inputs: the release binary before and after the change, `agent model` and `recensus snapshot`, and Atlas's HEAD `6a2896e1` exported without `.atlas`.

- **bettafish.** CALL records rose from 29,100 to 30,118. This equals acorn's 30,118 call and `new` expressions in each of the 7 files, so the gap is closed with no remainder. INVOKES rose from 18 to 19 and no edge was lost. The new edge is `chart.js` `te` → `Jt::constructor`, from `new Jt(t).saturate(.5)`, and it was read correct in the source. Unnamed unresolved sites rose from 1,382 to 1,407.
- **Atlas, JavaScript/TypeScript.** CALL records rose from 43,961 to 47,048. That is +2,977 from the extractor and +110 from import linking. INVOKES from JavaScript rose from 6,252 to 6,437 (+185 edges) and none was lost. Sampled new edges are inner calls of chains, and each was read correct: `text(value).toLowerCase()`, `asArray(x).map(..)`, `nextQueue(owned).slice(..)`, `all(db, ..).map(..)`, `loadContracts(..).map(..)` and `normalizeIds(ids).map(..)`. The browser scripts went from 189 to 210 sites, equal to acorn's count. 21 sites were missing before, in 14 functions.
- **Atlas, Rust.** Nothing changed. There are 59,217 CALL records and 7,472 INVOKES edges, with the same evidence. All 156 Rust artifacts have identical record counts and semantic digests, and every Rust function's call facts are identical.
- **Identity churn.** An acorn oracle reproduced the pre-change record count exactly. On bettafish, 8,413 of 29,100 CALL record ids are kept and 20,687 are retired. Of the kept ids, 63 now name the inner bare call rather than the outer chain call; the id is the same but the site is different. All 18 pre-existing INVOKES edges keep their endpoints, and their evidence ids change because they are `new` sites. On Atlas's 472 `.js`/`.mjs` files, 18,972 of 40,294 ids are kept (727 repointed) and 21,322 are retired. 472 of 477 JavaScript/TypeScript artifacts change semantic digest, so the census digest changes. No Rust or other artifact changes.
- **Tests.** In `adapter/src/semantic/typescript/tests.rs`, one test checks the anchor of every callee form in four grammars and that the identities are distinct. Another checks the minimal repro: `inner` is resolved DERIVED and `.m` is unresolved. A third checks that a bare call keeps its identity from before the change. A fourth checks that each of the three browser scripts has one CALL record per call and `new` node found by an independent tree walk. In `runtime/src/agent.rs`, a test checks that `impact inner` counts `viaChain` as a caller and `viaMember`'s `.inner()` as a candidate site, that `invokes:viaChain:inner` is VALIDATED, and that `make()()` counts as an unnamed site. 12 hand mutants on the anchor were all killed.
- **Unchanged.** What resolves is unchanged: a bare name bound once at module level, a same-file constructor, and imports linked by ADR 0073. Member calls stay unresolved. The census of calls outside what the walker visits is also unchanged: class heritage (`extends mixin(B)`) and class-level decorators (`@dec(f()) class K`); calls in computed keys and in field and method decorators are recorded. A wrapped member callee (`a.b!()`, `(a.b)()`) is a callee that is not a path under the G74 wording and is anchored at its argument list, still a distinct site. No such site occurs in bettafish's files or in Atlas's browser scripts, where the per-file totals equal acorn's.
