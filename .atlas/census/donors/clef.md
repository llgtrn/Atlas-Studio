---
id: donor-census-clef
type: reference
status: active
canonical: true
---
# Donor Census: Clef Compiler / CCS

## Source

- Remote: https://github.com/FidelityFramework/clef.git
- Repository: FidelityFramework/clef
- Commit: e94fa905f7f13c2b8216b355e9d84b17dceef5c8
- Git tree: 872e641465cc3b1df8dcb21901d0a7315e89f0be
- Branch: main
- Retrieved: 2026-09-22T14:05:13Z
- Staging mode: FULL_SOURCE_TREE
- Clone path: .atlas/temporary/donors/clef

### FidelityFramework mirror/upstream note (explicit finding)

Strong, explicit evidence that GitHub is a *publication* target rather than the primary development
surface, found in `docs/handoffs/Repository_History.md` and `README.md`:

- The maintained history was rewritten on 2026-09-20 ("History boundary": new root at retained-content
  snapshot of commit `3e39442be`, 2026-02-18). 209 of 210 remote branches and all 143 inherited tags
  were deleted atomically from the GitHub-visible `main` publication.
- The doc references `/home/hhh/repo-archives/clef-thinning-2026-09-20/` as "An external recovery
  archive ... holds the verified original Git bundle... It is not part of the repository or required
  to build it," and states "Old server objects may remain until **Forgejo** performs garbage
  collection" — i.e. the canonical git host for pre-migration/working history is a self-hosted Forgejo
  instance, not GitHub. GitHub's `FidelityFramework/clef` is downstream of that.
  A validation clone was done over **SSH** ("A fresh SSH clone at `c1491aa`"), consistent with a
  private/self-hosted origin distinct from the public HTTPS GitHub remote used for this census.
- `README.md` "Getting Started" gives a literal local path for the sibling Composer checkout:
  `dotnet build /home/hhh/repos/Composer/src/Composer.fsproj` — confirming a colocated, multi-repo
  local workspace (`/home/hhh/repos/`) is the real development environment; the commit author on HEAD
  (`houstonhaynes`, alias `hhh`) matches that path's username.
- `docs/fidelity/README.md` explicitly defers implementation-roadmap authority to the sibling
  Composer repository ("The canonical implementation roadmap is Composer's [Clef language completion
  analysis](../../../Composer/docs/...)... The documents here supply CCS design details and
  historical context; older strategy and orientation prose does not supply a competing work
  sequence.") and states plainly: **"CCS builds inside Composer's solution as a project reference;
  it is not a standalone .NET library."**

Conclusion: this is not a passive read-only fork-mirror of an unrelated upstream (unlike, say, a pure
GitHub mirror of another GitHub org) — it is a **curated, history-rewritten publication of an actively
developed, privately-hosted (Forgejo, SSH-accessed) monorepo-like workspace** in which Composer is the
solution root and CCS is consumed as an in-solution project reference. Treat GitHub HEAD as a lagging,
periodically-republished snapshot, not a live development branch, and expect further silent history
rewrites.

### Relationship to `FidelityFramework/Composer` (explicit finding)

- **No source/build dependency from clef → Composer** was found. `src/Compiler/Clef.Compiler.Service.fsproj`
  has exactly two sibling `ProjectReference`s: `BAREWire` and `Fidelity.Data` (both resolved via relative
  paths to sibling checkouts, e.g. `../../../BAREWire/src/BAREWire.fsproj`, with a `.worktrees/name`
  fallback path pattern). No Composer path, submodule or package reference exists in this repo.
- **Dependency runs the other way**: `Clef.Compiler.Service.fsproj` declares
  `<InternalsVisibleTo Include="Composer" />`, and `docs/fidelity/README.md` states "CCS builds inside
  Composer's solution as a project reference; it is not a standalone .NET library." So Composer's
  solution is the one that references CCS as a project, not vice versa.
- **Design-document coupling is tight and one-directional-by-authority**: CCS's own docs
  (`docs/fidelity/README.md`, `README.md`) repeatedly defer to `../../../Composer/docs/*` files
  (Clef_Language_Completion_Analysis.md, Obligation_Residency_Design.md, Proof_Composition_Architecture.md,
  M-01-DialectAdmission.md) as the canonical roadmap/architecture source. None of those Composer docs
  are present in this clone (expected — out of scope for this census; the Composer-side census covers
  them).
  - **Coordinator note**: the PSG (Program Semantic Graph) data structures that this repo defines
    (`src/Compiler/PSGSaturation/SemanticGraph/Types.fs` et al.) are the exact structures Composer's
    "Alex" middle end is documented to consume/witness. Provenance for "PSG shape/authoring" belongs to
    clef; provenance for "PSG consumption/lowering to MLIR" belongs to Composer. Attribute accordingly
    — do not let either census double-claim the PSG type definitions themselves (they physically live
    in this repo).
  - A third, undonated component, **Lattice** (editor/LSP tooling) and **Atelier** (IDE host), are
    referenced extensively (`docs/fidelity/phg/Lattice_Consumer_Contract.md`) as PSG *consumers* but
    their implementations are not in this repository, nor (per the same doc) fully built anywhere yet
    ("scheduled" / "not designed" appear throughout section 2-3 of that contract). Flag for the
    coordinator: Lattice/Atelier are not yet a real, in-repo donor candidate — the contract is a design
    document, not shipped code.

## Coarse Inventory

- Files observed: 406 (post `.git` removal)
- Bytes observed: 6,895,170 (~6.9 MB)
- Languages/signals: F# (.fs 276, .fsi 58 — signature files used pervasively), FsLex (.fsl x2),
  FsYacc (.fsy x2), F# script (.fsx x2), Markdown (33), MSBuild XML (.props x5, .targets x4,
  .fsproj x4, .sln x1), JSON (x2, incl. `.mcp.json`), TSV (x1, commit map), SVG (x1, icon-adjacent),
  strong-name key (.snk x1), shell script (x1, `drift-gate.sh`), resx (x1). No C/C++/Rust/Python
  source present — this is a pure F#/.NET codebase (naming-implied language confirmed by source).
- Top-level directories: `buildtools/`, `docs/`, `src/`, `tests/`
- Top-level files: `.editorconfig`, `.fantomasignore`, `.gitattributes`, `.gitignore`, `.mcp.json`,
  `Clef.Compiler.Service.sln`, `Directory.Build.props`, `Directory.Build.targets`,
  `FSharp.Profiles.props`, `FSharpBuild.Directory.Build.props`, `FSharpBuild.Directory.Build.targets`,
  `LICENSE`, `NuGet.config`, `README.md`, `attributions.md`, `global.json`, `icon.png`
- `.mcp.json` present at repo root, configuring a local `serena-local` SSE MCP server
  (`http://localhost:8000/sse`) — a development-tooling artifact (Serena semantic-code-navigation
  MCP), not part of the compiler; matches the stray `src/Compiler/.serena` directory also found in the
  clone. Both are local-tooling residue, not shipped product code — noted, not censused further.

Full source tree staged with nested `.git` removed.

## Direct Dependencies

Resolved from `src/Compiler/Clef.Compiler.Service.fsproj` (the only library project; `buildtools/`
tools and the test project are separately manifested):

| Package | Version | Role |
|---|---|---|
| `FSharp.Core` | 10.1.401 | F# core runtime library (Microsoft) |
| `XParsec` | 0.3.0 | Parser combinators — used for Baker saturation (`Baker/Ingredients/*`); README also names XParsec as powering PSG traversal generally and, per the Fidelity table, header parsing in the sibling `Farscape` repo |
| `FSharp.Json` | 0.4.1 | Pure F# JSON serializer (no BCL/reflection dependency) — used for diagnostic/phase-artifact JSON output |
| `BAREWire` (ProjectReference) | source, sibling repo `FidelityFramework/barewire` | Binary encoding / memory mapping / zero-copy IPC; also the schema compiler for wire-shaped PSG projections per `Lattice_Consumer_Contract.md` |
| `Fidelity.Data` (ProjectReference) | source, sibling repo (not in the Fidelity table but referenced by relative path) | TOML/XML/YAML/JSON/CSV parsing for `.fidproj` project-file loading |

`buildtools/fslex` and `buildtools/fsyacc` are **vendored, in-tree copies of Microsoft's FsLex/FsYacc**
parser-generator tools (from dotnet/fsharp), built as local tools (not NuGet-restored) and invoked via
`ProjectReference ... ReferenceOutputAssembly="false"` to generate `pars.fs`/`lex.fs` etc. at build
time. Attribute this to Microsoft/dotnet-fsharp, not to Clef/CCS authorship.

**Dependency closure status: PARTIAL_TRANSITIVE — stopped at depth 1 (direct-only for NuGet
packages).** No lockfile (`packages.lock.json`) is checked in and no `paket.lock` exists — the
`NuGet.config` lists many `dotnet*`/`vssdk*` package source feeds but that is source configuration,
not a resolved dependency graph. `FSharp.Core`, `XParsec` and `FSharp.Json` were not walked to their
own transitive dependencies (this would require restoring the project, which step 3 forbids as an
execution/network action beyond static manifest inspection). `BAREWire` and `Fidelity.Data` are
project references to **sibling repositories not present in this clone** (paths resolve to
`../../../BAREWire` and `../../../Fidelity.Data` relative to `src/Compiler/`, i.e. outside this
donor's tree) — their own dependency closures are out of scope for this census and are a distinct
donor-admission decision for the coordinator (BAREWire and Fidelity.Data are FidelityFramework
projects in their own right, not censused here). Composer is explicitly NOT a dependency of this
repo (see Composer-relationship note above) — nothing to resolve there.

## Build Systems Detected

- `.atlas/temporary/donors/clef/Clef.Compiler.Service.sln` — top-level solution
- `.atlas/temporary/donors/clef/src/Compiler/Clef.Compiler.Service.fsproj` — the compiler-service library (net10.0, SDK-style MSBuild)
- `.atlas/temporary/donors/clef/buildtools/fslex/fslex.fsproj` — vendored FsLex tool
- `.atlas/temporary/donors/clef/buildtools/fsyacc/fsyacc.fsproj` — vendored FsYacc tool
- `.atlas/temporary/donors/clef/tests/Clef.Compiler.Service.Tests/Clef.Compiler.Service.Tests.fsproj`
- `global.json` pins .NET SDK `9.0.100` with `rollForward: latestMajor` (actual `TargetFramework` in
  the fsproj is `net10.0` — the SDK pin is a floor, not the runtime target)
- No Paket, no Cake/FAKE/Nuke build script found; plain `dotnet build`/`dotnet test` per README

## Test / Benchmark Roots Detected

- `.atlas/temporary/donors/clef/tests/Clef.Compiler.Service.Tests/` (main xunit/expecto-style F# test project — not enumerated further; framework choice not confirmed without opening the fsproj's PackageReferences)
- `.atlas/temporary/donors/clef/tests/LanguageSurface/` (separate test root, distinct from the main test project — likely language-conformance/surface-syntax fixtures)
- 70 `.fs`/`.fsi` files total under `tests/`
- No dedicated `benches/` directory found; `docs/handoffs/Repository_History.md` claims "The cleaned
  source passed all 1,006 CCS tests" as a pre-migration validation fact (DECLARED, not verified by this
  census — no test execution was performed per the static-inspection mandate)

## Major Subsystem Roots

`src/Compiler/`:
- `SyntaxTree/` — inherited FCS lexer/parser (fslex/fsyacc grammars `lex.fsl`/`pars.fsy`,
  `pplex.fsl`/`pppars.fsy` for the preprocessor), syntax tree types, `LexFilter.fs` (offside-rule
  indentation handling)
- `NativeTypedTree/` — the Clef-specific type checker: `NativeTypes.fs`, `DimensionAlgebra.fs`,
  `MeasureEnvironment.fs`, `Unify.fs` (union-find unification), `NameResolution.fs`,
  `SRTPResolution.fs`, `Expressions/*` (modular per-construct checking handlers),
  `Infrastructure/*` (nanopass phase config/emission), `NativeService.fs` (public API surface,
  parse→check pipeline)
- `PSGSaturation/SemanticGraph/` — the Program Semantic Graph itself: node/hyperedge `Types.fs`,
  `Core.fs`, `NodeBuilder.fs`, `Reachability.fs`, `Traversal.fs`, `Diagnostics.fs`, plus ~25 saturation
  concern modules (closures, obligations, platform residence, string/byte storage, range analysis,
  escape analysis, curry/function-pointer handling)
- `Baker/` — "post-construction semantic enrichment" nanopass engine: `Pipeline.fs`, `ShadowAST.fs`,
  `Ingredients/` (combinator primitives, explicitly "XParsec-centric"), `Recipes/` (~30 HOF
  decomposition templates for closures, sequences, collections, obligations)
- `Nanopass/` — generic elaboration infrastructure (`Recipe.fs`, `FanOut.fs`, `FoldIn.fs`,
  `Monomorphization.fs`) plus the ordered pass list itself (intrinsic elaboration → program init →
  Baker saturation → closure/sequence passes → obligation discharge)
- `Project/` — `FidprojLoader.fs`, `SourceResolver.fs`, `ProjectChecker.fs` (`.fidproj` project-file
  loading and orchestrated checking; the "no MSBuild integration" boundary from README)
- `Driver/GraphChecking/` — inherited FCS file-dependency-graph infrastructure (`Graph.fs`,
  `DependencyResolution.fs`, `GraphProcessing.fs`, `TrieMapping.fs`) — used for source-file
  ordering/parallel-checking scheduling, not for the PSG itself
- `Facilities/`, `Utilities/` — inherited FCS infrastructure that survived pruning (diagnostics
  logger, caching, range/position tracking, `BuildGraph.fs`, `AsyncMemoize.fs`)

`buildtools/` — vendored FsLex/FsYacc (Microsoft-owned, MIT).

`docs/fidelity/` — design-of-record documents, notably `phg/` ("PSG-to-PHG plan", the **Design
Supersession Register**, and `drift-gate.sh`, a lint that fails CI on retired vocabulary appearing
anywhere in the doc corpus — an unusually rigorous doc/code-drift discipline worth noting for its own
sake, independent of any code absorption).

## License Evidence

- `.atlas/licenses/donors/clef/LICENSE` — **MIT License**, `Copyright (c) Microsoft Corporation. All
  rights reserved.`
- `README.md` states explicitly: "Original work is copyright Microsoft Corporation. Modifications are
  copyright Braidpoint." ("Braidpoint" appears to be the legal/organizational entity behind
  SpeakEZ Technologies / the Fidelity Framework; not independently verified beyond this README line.)
- `attributions.md` (8.4 KB) lists dozens of named individual contributors to F#/dotnet-fsharp
  features this codebase inherited (lexer, parser, SRTP, FSharp.Core) — retained verbatim from
  upstream dotnet/fsharp as a community-attribution record.
- `PackageLicenseExpression` in the fsproj: `MIT` (self-declared for the `Clef.Compiler.Service`
  NuGet package).
- Single, unambiguous license: **MIT**. No dual-licensing, no CLA notices, no additional
  per-subdirectory LICENSE files found.

## Mechanisms Census

### 1. Parsing (source → syntax tree)

**Provider: inherited from Microsoft's dotnet/fsharp (FCS), lightly extended by clef.** The lexer
(`lex.fsl`) and parser (`pars.fsy`) are FsLex/FsYacc grammar files compiled at build time by the
vendored `buildtools/fslex`/`buildtools/fsyacc` tools (also Microsoft-derived). `SyntaxTree.fs`,
`SyntaxTreeOps.fs`, `LexFilter.fs` (the offside/indentation rule engine), `LexHelpers.fs`,
`ParseHelpers.fs`, `WarnScopes.fs` and `PrettyNaming.fs` are the surviving FCS front-end modules,
copyright-headed "Copyright (c) Microsoft Corporation." README states this directly: "full Clef
syntax via the inherited FCS lexer and parser, extended for Clef constructs." No independent claim
of a from-scratch parser was found; this is a fork/extension, not a new implementation. `NativeService.fs`'s
`parseString`/`parseStringWithDefaults` wrap this into a stable `ParseResult` API.

### 2. Symbol / name resolution

**Provider: clef's own `NativeTypedTree/NameResolution.fs`, replacing FCS's removed
`NameResolution.fs`/`InfoReader`/`infos.fs` machinery.** This is a genuinely new implementation, not
inherited: the module doc frames it as "compositional... a codata/coeffect pattern" — resolvers are
`string -> ResolvedBinding option` functions, and `open` declarations compose resolver functions
(most-recent-first) rather than accumulating into a mutable symbol table. It explicitly and
structurally excludes BCL/.NET-assembly-derived bindings ("BCL is structurally impossible (BCL
bindings never added)") — this is closed-world resolution against Clef's own module graph
(`PSGSaturation.SemanticGraph`) only, with special-cased handling for DU-case constructors, inline
function bodies (kept for transparent expansion) and module-level vs. local bindings (relevant to
closure-capture analysis). This is real, working symbol resolution but scoped entirely to
source-defined names — there is no assembly-metadata reading path at all (`NO_IL_ASSEMBLY_IMPORT` is
a compile-time define baked into the fsproj, and `import.fs`/`TypeHierarchy.fs`/`infos.fs` are
explicitly deleted, per fsproj comments, as "BCL cruft").

### 3. Native type resolution / inference

**Provider: clef's own `NativeTypedTree/Unify.fs` + `NativeTypes.fs` + `DimensionAlgebra.fs` +
`MeasureEnvironment.fs` + `SRTPResolution.fs`, replacing FCS's removed `ConstraintSolver.fs`
("Core of type inference") wholesale.** This is a **real Hindley-Milner-style inference engine**, not
a thin nominal checker or a naming-only layer:
- Type variables (`NativeType.TVar`) with **union-find** storage (`NativeTypedTree/UnionFind.fs`),
  path compression, an explicit **occurs check** (`InfiniteType` error), and structural unification
  over function types, tuples (struct/reference), byrefs, native pointers, lazy/seq/list/map/set
  types, and `TForall` (polymorphic/generalized types) — `generalizeTopLevelFunction` in
  `NativeService.fs` performs let-generalization scoped by enclosing binder (`quantifiedByEnclosing`),
  the textbook HM generalization step.
- Layered on top of that HM core is a **second, independent unification domain for units-of-measure /
  hardware dimensions** (`DimensionAlgebra.fs`, `MeasureEnvironment.fs`): a genuine constraint-solving
  system over "numeric format, memory region, access kind, and tensor shape" (README's own words),
  with its own error taxonomy (`MeasureMismatch`, `NoIntegerSolution` — solving `v^k * residual = 1`
  over integer exponents, `MeasureExponentOutOfRange`). This is materially more than F#'s existing
  units-of-measure feature; it is described as propagating "the way Hindley-Milner propagates
  polymorphism" but is a distinct algebra, not reused HM machinery.
- **SRTP (statically resolved type parameters) resolution** (`SRTPResolution.fs`, 526 lines) resolves
  against **source-defined witnesses** — Clef's own "Alloy witness hierarchy" — explicitly not via
  .NET reflection/method-table lookup (README: "not .NET method tables"). This replaces FCS's
  `MethodCalls.fs`/`infos.fs`-based member overload resolution.
- Explicit, deliberate exclusions are documented in the fsproj as `<!-- ... REMOVED -->` comments:
  `import.fs`, `TypeHierarchy.fs`, `infos.fs`, `AccessibilityLogic.fs`, `InfoReader.fs`,
  `AttributeChecking.fs`, `TypeRelations.fs`, `NicePrint.fs`, `AugmentWithHashCompare.fs`,
  `SignatureConformance.fs`, `MethodOverrides.fs`, `MethodCalls.fs`, `PatternMatchCompilation.fs`,
  `ConstraintSolver.fs` — i.e., **the entire IL/BCL-mediated half of F#'s real type checker was
  deleted and a new, narrower, but still real, inference engine was built in its place.** This is the
  single most important architectural fact for the Atlas comparison below.

### 4. Language-service / incremental-analysis architecture

**Provider: not implemented in this repository; only a design contract exists, attributed to a
separate, not-yet-built component ("Lattice").** No LSP/JSON-RPC/`textDocument/*` protocol code was
found anywhere in the tree (checked by content grep across all `.fs*` files — zero matches for
`languageserver`/`textDocument`/`jsonrpc`/`lsp`). `NativeService.fs`'s actual public surface is a
**batch, whole-compilation API**: `parseAndCheck: source -> fileName -> ParseAndCheckResult`,
`checkParsedInputs`, `checkModuleDeclarations` — there is no per-edit incremental re-check entry
point, no document-version tracking, no cursor-position query function (`hover`, `definition`,
`references` etc. do not exist as callable functions in this codebase). What *does* exist is
`docs/fidelity/phg/Lattice_Consumer_Contract.md`, an unusually precise **design-of-record contract**
enumerating the intended query surface (`nodeAt`, `hover`, `diagnostics`, `scope`, `definition`,
`references`, `rename`, `declarationRoots`, `inlayHints`, `semanticTokens`, `signatureHelp`,
`reachability`, `graphUpdated` notification, etc.) with an explicit status column per row: most rows
are `exists` (meaning the *underlying graph fact* exists, e.g. every node has `Range`/`Parent`), but
the *query/index* itself is repeatedly marked `index` ("the data exists but the query that serves it
has not been written") or `scheduled` or `not designed`. `graphUpdated` (the one thing that would make
this genuinely incremental/live) is explicitly `scheduled`, not built. One relevant internal
optimization exists inside the batch checker itself: `NativeService.fs` tracks
`solvedConstraintCount` (a mutable int) so that `solveNewConstraints` only re-solves constraints
accumulated since the last solve within a single compilation — this is intra-compilation incremental
constraint solving, not cross-edit/session incrementality, and should not be conflated with a real
language service.

### 5. Compiler-service boundary / API to the PSG (and onward to Composer)

**Provider: clef's own `PSGSaturation/SemanticGraph/Types.fs` (data) + `NativeService.fs` (API) +
the Baker/Nanopass pipeline (construction).** The PSG is explicitly framed as "the unified
representation for Composer" (module doc comment, verbatim) — a hypergraph of typed nodes and
first-class hyperedges (not a typed AST/tree): `WitnessResolution` (SRTP witness records),
`InterpolatedPart`, `IntrinsicModule` and dozens of further node/edge kinds live here. Saturation is
performed by an ordered nanopass pipeline (`Nanopass/*.fs`, driven by `Baker/Pipeline.fs` and the
`PhaseConfig`/`PhaseEmitter` infrastructure) that runs *after* initial construction: elaboration →
program initialization → "Baker saturation" → closure/sequence passes → obligation
discharge/elaboration → range/platform/escape/layout passes. Proof obligations are emitted as graph
citizens (not side-channel diagnostics) and, per README, "dispatched to cvc5 at design time" — an SMT
solver integration for correctness obligations, a mechanism with no analogue anywhere in Atlas.
**The actual repo-to-repo boundary**: CCS does not export a stable serialized wire format to Composer
in this repo today — `Lattice_Consumer_Contract.md` §3 states the "Graph projection export" (a
BAREWire-schema-derived serialization of the saturated graph) is `scheduled`, and today's real
interchange with Composer is (a) in-process, via `InternalsVisibleTo Include="Composer"` plus the
project-reference relationship documented above (Composer's build literally links against CCS's
compiled output), and (b) file-based JSON phase artifacts (`01_psg0` … `05_psg2`, obligation ledgers
`06a`/`06b`) emitted by `PhaseEmitter.fs` for debugging/comparison (`ccs_diff` artifacts), not a
production wire protocol. So: the PSG *type schema* is a real, working boundary; the PSG *transport*
across a process/service boundary is a documented future design, not present code.

## Atlas Comparison

- **Parsing** — Atlas's Rust `SourceFrontend`/extractor uses `syn` (a real, independent Rust parser
  crate), analogous in role to clef's inherited FCS lexer/parser, but the provenance differs sharply:
  Atlas's parser is a clean third-party dependency; clef's is a forked, modified copy of Microsoft's
  own compiler source living in-tree. No direct absorption opportunity — different implementation
  languages (F# vs Rust) make source-level reuse moot; NOT a candidate for absorption.
- **Symbol resolution** — Atlas's Rust extractor currently emits `SymbolFact`/`FunctionIdentity`
  records from syntax alone (per `core/src/semantic/function.rs`, `core/src/semantic/symbol.rs`);
  clef's `NameResolution.fs` "resolver composition over `open` declarations" pattern is an elegant,
  closed-world design, but it is *architecturally coupled to owning the entire module graph* (its own
  `PSGSaturation.SemanticGraph`) — it presumes the checker already has full graph residence for every
  name it resolves. Rust's `use`-resolution problem (external crates, glob imports, macro-generated
  items, trait-method resolution via type-directed lookup) is a fundamentally larger and differently
  shaped problem than Clef's closed-world module system. REFERENCE_ONLY: instructive as a *pattern*
  (compose resolvers instead of accumulating mutable state) but not directly portable.
- **Native type resolution vs. Atlas's deliberately-unresolved CALL dimension (the key comparison)** —
  This is the central finding of this census. Atlas's own Rust extractor
  (`core/src/semantic/call.rs`) states, verbatim, in its module doc comment: *"The Rust extractor has
  no rustc-backed name resolution (never will, per every prior wave's epistemic discipline)... An
  unqualified path or a method-call receiver's type is not determinable from `syn` alone.
  `dispatch`/`callees` are therefore always `Unresolved`/`[]` from this extractor today."* This is a
  **deliberate, contractually-recorded epistemic-honesty boundary** (`.atlas/contracts/SEMANTIC-EXTRACTION.md`
  §"Dynamic behavior" and the extractor-identity obligations), not a gap Atlas failed to close.
  clef's `Unify.fs`/`NativeTypes.fs`/`SRTPResolution.fs` demonstrate that **real type
  resolution/inference is achievable without a full BCL/IL-reading compiler** — the NativeTypedTree
  checker is proof that a from-scratch, source-only HM-style inferencer is buildable in isolation from
  .NET's reflection/assembly-metadata machinery (the `NO_IL_ASSEMBLY_IMPORT`/`NO_TYPEPROVIDERS`
  defines and the explicit deletion list prove this was a conscious design, not an accident). *However*,
  the scale of the undertaking is the disqualifying fact for direct reuse: clef's checker required
  rebuilding `ConstraintSolver.fs`-equivalent inference, a closed-world name resolver, an SRTP/witness
  resolution system, and a full nanopass saturation pipeline — thousands of lines of new F# across
  `NativeTypedTree/` and `Baker/` — to serve **one language it fully owns and controls the semantics
  of**. Rust is not a language Atlas owns; building rustc-equivalent (or even a meaningfully-scoped
  subset of) type/trait/generic resolution to unblock `CallDispatchKind::StaticResolved` would
  require exactly the kind of infrastructure investment Atlas's own contract explicitly forecloses
  ("never rustc/full name resolution", per the call.rs comment and `SEMANTIC-EXTRACTION.md`'s
  "AI/model analysis MUST NOT emit OBSERVED source facts" discipline). **Disposition: REFERENCE_ONLY
  for this specific R4.5 comparison** — clef proves the pattern is *possible in principle* for a
  compiler that owns its whole language, but it is not a bounded, honest path to real CALL resolution
  for Atlas's Rust lane; it would require Atlas to become a Rust type checker, which is out of scope
  by explicit prior contract. See Blueprint Revision Candidates below for the narrower, non-blueprint
  observation this does still support.
- **Language-service/incremental architecture** — Atlas has no equivalent concept in scope today (R4
  is a batch extraction pipeline, not an editor service). clef itself does not have a *built*
  incremental language service either — only a design contract (`Lattice_Consumer_Contract.md`) for
  one. Nothing to absorb; EXTERNAL_BOUNDARY (both sides currently out of scope for this dimension).
- **PSG / compiler-service boundary** — Atlas's `ExtractionBatch` (per
  `.atlas/contracts/SEMANTIC-EXTRACTION.md`) and clef's PSG are answering a structurally similar
  question (what typed, evidence-linked facts does a source-analysis stage hand downstream?) but at
  very different maturity/ambition levels: clef's PSG is a hypergraph with obligation nodes, SMT
  dispatch and saturation passes; Atlas's `ExtractionBatch` is an explicitly-scoped bootstrap envelope
  the contract itself says "MUST NOT become the permanent universal model." The obligation-ledger
  pattern (typed proof obligations as first-class graph citizens with an explicit UNKNOWN/discharged
  state machine) is conceptually close to Atlas's own `EpistemicStatus` taxonomy
  (OBSERVED/DECLARED/DERIVED/INFERRED/HYPOTHESIS/CONFLICT/UNKNOWN/UNSUPPORTED/IGNORED) — both systems
  independently arrived at "never silently drop an obligation, always carry its resolution state."
  This convergence is worth noting as validation of Atlas's existing design, not as something to
  import (Atlas's taxonomy is already in place and, if anything, more general-purpose than clef's
  SMT-specific ledger). REFERENCE_ONLY / validates-existing-design.

## Discoveries

- **D1 [mechanism: repository provenance]** GitHub `FidelityFramework/clef` is a periodically
  republished, history-rewritten export of a privately-hosted Forgejo repository developed in a local
  multi-repo workspace (`/home/hhh/repos/`); evidenced in `docs/handoffs/Repository_History.md` and
  `README.md`. Disposition: **REFERENCE_ONLY** (provenance/process fact, not code to absorb; flagged
  for coordinator so re-census expectations are set correctly — expect drift/rewrites, and expect the
  GitHub main to be less current than the private history it descends from).
- **D2 [mechanism: type resolution]** A working, from-scratch, source-only Hindley-Milner-style
  unification engine (`Unify.fs`) plus a second, independent units-of-measure/hardware-dimension
  unification domain (`DimensionAlgebra.fs`) coexisting cleanly via a shared union-find substrate.
  Disposition: **REFERENCE_ONLY** — real, working, well-evidenced infrastructure, but built for a
  language clef fully owns; not directly portable into Atlas's Rust-extraction lane per Atlas's own
  "never rustc" contract discipline (see Atlas Comparison above). Worth another look if Atlas ever
  gains an OWNED, closed-world DSL/IR of its own that needs real inference (e.g. a future Atlas query
  or projection language) — not for CALL resolution on donor Rust code.
- **D3 [mechanism: obligation/proof tracking]** Typed proof obligations as first-class graph nodes,
  discharged via SMT (cvc5) at design time and re-derived at build time from the emitted artifact
  ("Required unresolved obligations remain explicit; creating an obligation does not establish that it
  has been discharged" — README, near-verbatim echo of Atlas's own EpistemicStatus discipline).
  Disposition: **REFERENCE_ONLY / validates existing Atlas design** — no code to absorb, but useful
  as independent-convergence evidence when justifying Atlas's EpistemicStatus taxonomy to stakeholders.
- **D4 [mechanism: symbol resolution]** The `Resolver = string -> ResolvedBinding option` /
  open-declaration-composition pattern in `NameResolution.fs` is a clean, mutation-free design for
  scope resolution. Disposition: **REFERENCE_ONLY** — architecturally interesting, not a bounded fit
  for Rust's `use`/crate/trait resolution surface (see Atlas Comparison).
  D5 [mechanism: doc/code drift discipline] `docs/fidelity/phg/drift-gate.sh` — a CI-enforced lint
  that fails the build if retired/superseded vocabulary appears anywhere in the documentation corpus,
  paired with an explicit `Design_Supersession_Register.md` naming what was retired and why.
  Disposition: **REFERENCE_ONLY** — a process/tooling idea (not Rust/Atlas-stack code) that the
  coordinator may want to flag to Atlas's own docs/process owners independent of any donor-code
  absorption; not scored further here as it is out of this census's code-mechanism scope.
- **D6 [mechanism: compiler-service boundary]** No production wire-format/serialization boundary
  between CCS and Composer exists yet in this repo (project-reference/in-process linkage only; graph
  export to a BAREWire-schema wire format is `scheduled`, not built). Disposition:
  **EXTERNAL_BOUNDARY** — nothing to absorb; noted so the coordinator does not expect a ready-made
  "PSG wire protocol" artifact from this donor.
- **D7 [mechanism: license/attribution]** Confirmed clean, single-license MIT provenance with an
  explicit, unusually thorough per-contributor `attributions.md` inherited from dotnet/fsharp.
  Disposition: **REFERENCE_ONLY** (licensing hygiene note only).

## Blueprint Revision Candidates

None. The one plausible candidate — using clef's real-inference pattern as a template for adding
bounded real resolution to Atlas's R4.5 `CALL` dimension — was evaluated explicitly in the Atlas
Comparison section above and rejected as a blueprint-level change: it would require Atlas to build
Rust-language-owning compiler infrastructure (type/trait/generic resolution), which contradicts
Atlas's own standing contract ("never rustc/full name resolution", `core/src/semantic/call.rs`
module doc; `SEMANTIC-EXTRACTION.md`'s AI/model-analysis restriction). If a bounded, non-full-checker
lesson is wanted later, the narrowest honest version suggested by this donor is *purely syntactic,
same-crate, `use`-aware name disambiguation* (resolve an unqualified call target only when exactly one
same-named function is in local/`use`-imported lexical scope, no type/trait resolution at all) — but
no such mechanism was found built anywhere in this donor (clef's `NameResolution.fs` presumes full
graph residence, not a standalone lexical pass), so this is a speculative extrapolation, not
evidence-backed, and is intentionally not elevated to a blueprint candidate here.

## Recensus Requirements

- Re-clone and re-diff if the coordinator wants current state — given the confirmed history-rewrite
  practice (D1), do not assume this pin remains reachable/ancestor-stable indefinitely; treat future
  `git log` on this remote as potentially non-linear relative to this census's commit.
  `e94fa905f7f13c2b8216b355e9d84b17dceef5c8` was `HEAD` of `main` at retrieval time
  (2026-09-22T14:05:13Z), only 2 days after the 2026-09-20 history migration — expect further commits
  and possibly further republication events.
- No dependency-lockfile-based transitive closure was performed (PARTIAL_TRANSITIVE, depth 1) — a
  future census with network/build access could restore the project and capture the full NuGet graph
  for `FSharp.Core`/`XParsec`/`FSharp.Json`, and separately admit `BAREWire` and `Fidelity.Data` as
  their own donor candidates to complete the dependency picture.
- If the coordinator pursues the Composer-side census in parallel (per this task's brief, censused by
  a sibling agent), reconcile the PSG-type-definition attribution per the note in "Relationship to
  Composer" above — the types physically live here, not there.

## Census State

Status: COARSE_CENSUSED + TARGETED_DEEP_CENSUS (parsing, symbol resolution, native type resolution,
language-service architecture, compiler-service/PSG boundary). This is an admission-stage inventory
plus a targeted deep read of the five requested mechanisms; it is not a full line-by-line audit of
`Baker/`, `Nanopass/`, or the ~25 `PSGSaturation/SemanticGraph/*` saturation-pass modules, which
remain uncensused at the mechanism level beyond the summary given in "Major Subsystem Roots" and the
PSG-boundary mechanism section above.
