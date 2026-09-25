---
id: atlas.census.donors.wasm-component-model
type: donor-census
status: active
canonical: true
---
# WebAssembly Component Model Donor Census

## Source

- Donor: `WebAssembly Component Model`
- Remote: `https://github.com/WebAssembly/component-model`
- Commit: `5b724da8c634c4f2b2d2b3f6a4e9e13b4eb01596`
- Clone path: `.atlas/temporary/donors/wasm-component-model`
- License: Apache-2.0, root `LICENSE` (pointer) + `LICENSE-APACHE` (full text), both preserved in the staged tree
- Donor gap: `ATLAS_INTERFACE_COMPONENT_CAPABILITY_BOUNDARY_SEMANTICS`
- Quarantine: none required — verifier reported no `.claude`/`.codex`/`.cursor`/`CLAUDE.md`/`AGENTS.md`/`.mcp.json`
  paths anywhere in this donor's tree.

## Framing (read this before the rest)

This donor is censused for Atlas's **`atlas.interface.*` / `atlas.component.*` / `atlas.capability.*` / FFI-boundary
semantics** — how Atlas should describe the typed surface a unit of code exposes/consumes and how values cross
that surface — **not** for Atlas's core execution IR (AtlasX/HIR/MIR/LIR, governed by
`ATLAS-TO-ATLASX.md`/`COMPILER-IR-SCHEMAS.md`). The Component Model is almost entirely design/specification prose
(Markdown + EBNF-in-Markdown + a Python-pseudocode Canonical ABI) plus WIT grammar material; there is essentially
no executable implementation in this repository to run (and none was executed, per the quarantine instructions —
static reading only). Atlas is **not** adopting the Component Model, WIT, or the Canonical ABI as a runtime
dependency; this is architecture study toward Atlas's own interface/boundary vocabulary.

## Coarse Inventory

- Files observed: 103
- Bytes observed: ~2.7 MB (`du -sh` = 2.7M)
- Top-level structure: `design/` (the actual spec-in-progress: `design/mvp/` — MVP-era design docs, `design/high-level/`),
  `spec/` (a thinner pointer/README layer), `test/` (async/binary/linking/resources/validation test-suite
  scaffolding, largely `.wast`-adjacent, not deep-read), `.github/workflows/` (CI)
- Core design documents under `design/mvp/` (line counts from direct read): `WIT.md` (2497 lines — the WIT text
  format spec), `CanonicalABI.md` (5048 lines — the value-lifting/lowering spec, the largest single document),
  `Explainer.md` (3496 lines — the component/AST-level spec: imports/exports, component types, invariants),
  `Binary.md` (603 lines — binary encoding of components), `Concurrency.md` (1606 lines — the async task/stream/
  future model), `Linking.md` (344 lines), `FutureFeatures.md` (74 lines)

## Mechanisms Census (read from `design/mvp/WIT.md`, `CanonicalABI.md`, `Explainer.md`)

### WIT: worlds, interfaces, imports/exports

- **Interface** (`WIT.md` "WIT Interfaces" section): "a collection of functions and types... can be thought of as
  an instance in the WebAssembly Component Model." A worked example (`interface host { log: func(msg: string); }`)
  is shown compiling to a component-level `(import "local:demo/host" (instance (export "log" (func (param "msg"
  string)))))` — i.e. an interface is literally an instance type: a named bag of typed functions and types,
  nothing more.
- **World** (`WIT.md` "WIT Worlds" section): "a complete description of both imports and exports of a component...
  can be thought of as an equivalent of a `component` type in the component model." Verified from the worked
  example: a `world` with `import`/`export` statements compiles directly to a `(type $my-world (component (import
  ...) (export ...)))`. Worlds can `include` other worlds (union of imports/exports, "Union of Worlds with
  `include`", `WIT.md:267`) and de-duplicate shared interfaces (`WIT.md:311`), with explicit name-conflict
  resolution via a `with` renaming clause (`WIT.md:342`).
  - **Direct relevance to Atlas**: a `world` is structurally exactly "the full typed contract of one deployable
    unit — what it needs (imports) and what it offers (exports)" — this is the closest existing prior-art shape
    for an `atlas.component.*` manifest: a world *is* a component type, not a separate artifact bolted onto one.
- **Package/namespace structure** (`WIT.md` "Package Names", `WIT.md:34`): three-part `namespace:package@version`
  naming (e.g. `wasi:clocks@1.2.0`), with nested-package syntax (`package local:a { interface foo {} }`) for
  inline multi-package files. This is a genuine, verified naming scheme for capability/interface provenance that
  Atlas's own `atlas.interface.*`/`atlas.capability.*` identifiers could structurally mirror (namespace + package
  + optional semver, rather than an unstructured string).

### Type constructors: `resource` / `record` / `variant` / `result` / `flags` / `enum`

All six verified directly from `WIT.md`'s "Items: type" section with worked EBNF and examples:

- **`record`** (`WIT.md:1663`): a named product type with named fields (`record pair { x: u32, y: u32 }`) —
  standard struct semantics, "instances of a `record` always have their fields defined."
- **`flags`** (`WIT.md:1693`): a named bitset (`flags properties { lego, marvel-superhero, supervillan }`), each
  member a named bit — explicitly noted as having "a bit flags representation in the canonical ABI," i.e. this is
  not sugar for a set-of-strings, it is a real fixed-width bit-packed wire representation.
- **`variant`** (`WIT.md:1716`): a tagged union / sum type, each case optionally carrying a payload type
  (`variant filter { all, none, some(list<string>) }`) — "instances of the type match exactly one of the variants
  listed."
- **`enum`** (`WIT.md:1748`): explicitly "semantically equivalent to a `variant` where none of the cases have a
  payload type," special-cased only because it "may have a different representation in the language ABIs" — i.e.
  `enum` is not a fourth independent construct, it is a documented, verified specialization of `variant` for
  payload-free cases, which is a clean modeling lesson (don't invent a parallel construct for what is really a
  constrained case of an existing one).
- **`resource`** (`WIT.md:1774`, the largest and most structurally significant of the six): "an abstract type for
  an entity with a lifetime that can only be passed around indirectly via handle values." Verified mechanics:
  resources support `constructor`, instance methods, static functions, and (gated, 📡) getters/setters, all of
  which **desugar to plain functions taking an explicit handle parameter** (e.g. `write: func(bytes: list<u8>)` on
  a resource desugars to `%[method]blob.write: func(self: borrow<blob>, bytes: list<u8>)`) — resources are sugar
  over handle-passing functions, not a separate runtime primitive. Two handle kinds exist and are semantically
  distinct (verified in the "Handles" section, `WIT.md:1974`): an **owned** handle transfers unique ownership (the
  resource is destroyed when the owning handle is dropped) and a **borrowed** handle (`borrow<T>`) is a temporary
  loan for the call's duration only — this owned-vs-borrowed split is a directly relevant reference model for
  `atlas.capability.*` if Atlas capabilities need an analogous unique-vs-loaned distinction at a boundary.
- **`result<T, E>`** appears as a built-in parameterized type (not a user-declared "item" construct like the five
  above) usable directly in signatures, e.g. `stat-file: func(path: string) -> result<stat>;` and fallible resource
  constructors (`WIT.md:1774`'s `resource blob2 { constructor(init: list<u8>) -> result<blob2>; }`) — i.e. fallible
  operations are typed as ordinary values, not out-of-band exceptions, consistent across both plain functions and
  resource constructors.

### Component boundaries

- **`Explainer.md`'s "Component Invariants"** section (verified, `Explainer.md:3073`) is the sharpest statement of
  what a component boundary actually guarantees, and is worth quoting structurally: (1) a component enters a
  permanent **"lockdown" state after any trap**, checked at every subsequent component-function call, so "after a
  trap, it's no longer possible to observe the internal state of a component instance" — the boundary is a hard
  failure-containment wall, explicitly analogized in-source to "the invariants provided by a traditional Operating
  System to user-space code running inside a process"; (2) for `async` functions, core-wasm execution is
  "run-to-completion" within one component instance unless the component opts into more concurrency (stackful
  async, cooperative threads) — i.e. the boundary also constrains concurrency semantics, not just type shape.
- **Components import/export only component-level functions, never raw Core WebAssembly functions** (same
  section): "Component validation rules only allow a component to import and export component-level functions,
  not Core WebAssembly functions. Because component-level functions can only be produced or consumed by Canonical
  ABI `lift` and `lower` definitions... the Component Model is able to define and enforce invariants." This is the
  structural reason the boundary is trustworthy: every cross-boundary call is forced through a single, specified
  translation layer (Canonical ABI lift/lower), never a raw memory-sharing shortcut.

### Canonical ABI: how values cross the boundary

Verified from `CanonicalABI.md`'s actual structure (a genuine algorithmic spec, not just prose — expressed as
literal Python-pseudocode classes and functions, e.g. `MemInst`, `LiftOptions`, `LiftLowerOptions`,
`CanonicalOptions`, `CanonicalABI.md:936` onward):

- **Despecialization / Type Predicates / Alignment / Element Size** (`CanonicalABI.md` section list, verified
  headings) are the staged pre-computation steps before any value crosses the boundary: reduce sugar types
  (`enum`→`variant`, etc.) to their canonical form, then compute per-type alignment and flat byte size.
- **Loading / Storing** are the linear-memory-level read/write operations for the *non-flattened* (memory-based)
  representation of complex values.
- **Flattening / Flat Lifting / Flat Lowering** are the *register-based* fast path: small values are packed
  directly into core Wasm function parameters/results (flattened) rather than spilled to linear memory, with an
  explicit flattening algorithm and a fallback to the memory path for values too large to flatten.
- **`CanonicalOptions`** (verified dataclass, `CanonicalABI.md:1000`-ish) carries the actual configurable knobs of
  a boundary crossing: `string_encoding` (default `'utf8'`), `memory` (which linear memory to read/write),
  `realloc` (a callback the runtime uses to grow guest memory for outbound allocations), `post_return` (a cleanup
  hook), `async_`/`callback` (concurrency mode). This is a concrete, load-bearing design precedent: **a boundary
  crossing is parameterized by an explicit, enumerable options record**, not an ad hoc convention — directly
  relevant to how an `atlas.capability.*`/FFI-boundary record might need its own explicit, enumerable
  crossing-options shape (encoding, allocator, cleanup, sync/async mode) rather than leaving those as unstated
  assumptions.
- **`canon lift` / `canon lower`** (named in the table of contents, the actual definitions) are the two directions:
  `lift` converts core-Wasm-level values (ints/floats/memory) into component-level typed values for a component
  function body to consume; `lower` does the reverse for calling out. This lift/lower pairing is the Component
  Model's name for exactly the "boundary marshaling" concept Atlas's own FFI-boundary semantics need a name and
  shape for.

## Atlas Comparison

- **Atlas today**: no `atlas.interface.*`/`atlas.component.*`/`atlas.capability.*` contract was found under
  `.atlas/contracts/` at the time of this census (grep for these prefixes across `.atlas/` returned no matches
  outside donor trees) — this donor gap is real and currently unfilled, not merely under-specified.
  `SELECTED-DESIGN.md` and `ATLAS-TO-ATLASX.md` govern decision/materialization concerns adjacent to but distinct
  from a typed interface-boundary vocabulary.
- **What WIT/Canonical ABI offers as reference architecture, not as an adoption target**: (1) a `world` as "the
  complete typed contract of a deployable unit" is a clean target shape for an `atlas.component.*` manifest; (2)
  the six type constructors (`record`/`flags`/`variant`/`enum`/`resource`/`result`) are a compact, well-tested
  vocabulary for describing typed data crossing a boundary, with `resource`'s owned/borrowed handle split being
  the most novel and most directly relevant idea for `atlas.capability.*` if Atlas capabilities need an
  ownership-transfer-vs-temporary-loan distinction; (3) the Canonical ABI's explicit `CanonicalOptions` record
  (encoding/allocator/cleanup/async-mode as named, enumerable fields) is a strong precedent for making an
  `atlas.*` FFI-boundary crossing's assumptions explicit and inspectable rather than implicit; (4) the "components
  only import/export component-level functions, never raw core functions, and lockdown-after-trap" invariant is a
  strong precedent for how Atlas might want to specify that a capability boundary is a hard failure-containment
  wall, not merely a type-checked call.
- **What is explicitly out of scope for this donor**: none of this censuses or recommends Atlas's core execution
  IR design. The Component Model's own core-Wasm layer (what actually executes) is a different donor
  (`wasm-spec`, censused separately) — this document only concerns the interface/type/boundary layer sitting
  above that execution layer.

## Disposition (per concept)

| Concept | Disposition | Notes |
|---|---|---|
| `world` as complete import/export contract | **ADAPT** | Strong target shape for `atlas.component.*`; reimplement the concept (a component's full typed surface, one record) natively, not the WIT syntax. |
| `record`/`flags`/`variant`/`enum` type constructors | **ADAPT** | A compact, verified, well-understood vocabulary for typed boundary data; worth mirroring the *shape* for `atlas.interface.*` typed payloads. |
| `resource` + owned/borrowed handle split | **STUDY** | The most novel idea here; worth a dedicated design pass before committing `atlas.capability.*` to an ownership-transfer-vs-loan model — verify Atlas actually needs this distinction before adopting it. |
| `result<T,E>` as ordinary typed value (no exceptions) | **ADAPT** | Simple, already broadly convergent with Rust/Atlas's own likely error-handling conventions; low-risk to mirror. |
| Namespace:package@version naming for interfaces | **ADAPT** | Directly reusable structural idea for `atlas.interface.*`/`atlas.capability.*` identifier shape. |
| Canonical ABI's explicit `CanonicalOptions` (encoding/allocator/cleanup/async) | **STUDY** | Strong precedent for making boundary-crossing assumptions explicit; needs Atlas-specific design work, not a direct port (Atlas's runtime model differs from core-Wasm's linear memory). |
| Flattening / flat-lift / flat-lower register-packing optimization | **DEFER** | A real performance optimization for a stack-machine ABI; only relevant once Atlas has a concrete low-level calling convention to optimize — premature to adopt now. |
| Component "lockdown after trap" invariant | **STUDY** | Valuable framing for capability-boundary failure containment; needs mapping onto Atlas's own error/panic model before adoption. |
| Concurrency model (`async`/streams/futures/backpressure, `Concurrency.md`) | **DEFER** | Real and substantial (1606 lines) but not read in depth for this census (out of the assigned mechanism set); recensus if Atlas's capability boundary needs an async contract. |
| Binary encoding of components (`Binary.md`) | **DEFER** | Not read in depth here; the assigned focus was WIT/type-constructors/boundary/Canonical-ABI concepts, not the component binary format itself. |

### Things NOT to copy

- Do not add any WIT tooling, `wit-bindgen`, or Component Model runtime library as an Atlas dependency — this
  census evaluates the *interface vocabulary and boundary-crossing concepts* for an Atlas-native design, not code
  reuse.
- Do not adopt WIT's textual syntax verbatim as `atlas.interface.*`'s surface syntax — the concepts (worlds,
  records/variants/resources, owned/borrowed handles) are the donor value, not the concrete grammar.
- Do not treat the Canonical ABI's linear-memory-specific mechanics (flattening thresholds, `realloc` callback
  convention, core-Wasm value types) as directly portable — Atlas's own runtime/memory model is not core Wasm's,
  so only the *shape* of "boundary crossings are parameterized by an explicit options record" is a donor idea, not
  the specific byte-level mechanics.
- Do not cite this donor as evidence for Atlas's core execution IR (AtlasX/HIR/MIR/LIR) design — it is strictly an
  interface/boundary-layer donor.

## Known Risks / Gaps in This Census

- `Concurrency.md` (1606 lines, the async task/stream/future/backpressure model) and `Binary.md` (603 lines, the
  binary encoding of components) were located and their table-of-contents inspected but not deep-read; both may
  matter to a future `atlas.capability.*`/FFI-boundary contract with async semantics or a wire encoding
  requirement, and are flagged for recensus rather than covered here.
- `test/` (async/binary/linking/resources/validation test suites) was inventoried but not read; it could offer
  additional worked examples of edge-case boundary semantics not covered by the prose documents alone.
- No execution of any tooling in this repository was performed (there is little to execute — this is
  overwhelmingly a specification repository — but this is noted per the mandatory quarantine instructions).
- This census does not attempt to reconcile WIT's model with WASI's specific interface packages (`wasi:*`), which
  are a separate, much larger ecosystem built on top of this donor's concepts and out of scope here.

## Census State

CENSUSED. Targeted mechanism census complete for WIT worlds/interfaces/imports-exports, the six type constructors
(`resource`/`record`/`variant`/`result`/`flags`/`enum`), component boundary invariants, and Canonical ABI
lift/lower/options concepts, all read from real source under `design/mvp/WIT.md`, `design/mvp/CanonicalABI.md`, and
`design/mvp/Explainer.md`, explicitly scoped to Atlas's interface/component/capability/FFI-boundary donor gap.

## G98 — terminal REFERENCE_ONLY; source extinct

No capability boundary exists to test the owned-vs-borrowed question. `core/src/capability` holds work-admission contracts, and `atlas.interface.*` construction is TARGET. The question is an Atlas construction-design decision, so it moved into ASIR-CONSTRUCTION-MODEL.md together with its falsification and the ADAPT shapes as target vocabulary. A Rust-derived interface already carries move-vs-borrow in its OWNERSHIP facts. The checkout was physically deleted. Evidence: `../../evidence/campaign/34-wasm-component-model.json`.
