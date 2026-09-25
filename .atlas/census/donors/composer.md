---
id: donor-census-composer
type: reference
status: active
canonical: true
---
# Donor Census: Clef / Composer

## Source

- Remote: https://github.com/FidelityFramework/Composer.git
- Repository: FidelityFramework/Composer
- Commit: c46485096c38dc2c9fef9db4ae60caa1905ca1a3
- Git tree: f3a1d7e468bf5dd8dcb4935587ee6b6b0c1e9009
- Branch: main
- Retrieved: 2026-09-22T14:02:00Z (approximate, container clock; see provenance JSON for exact ISO8601)
- Staging mode: FULL_SOURCE_TREE
- Clone path: .atlas/temporary/donors/composer

### Mirror / upstream provenance note

No evidence this repository is a read-only mirror. It reads as the primary, actively-developed
repository: normal single-author-style commit history reachable from `main`, no "this repo is
archived/mirrored, see X" banner in README/CLAUDE.md, a `CLAUDE.md` written for engineers actively
working in this exact checkout (build commands reference `/home/hhh/repos/Composer/...`), and a
`.mcp.json`/`.mise.toml`/`Directory.Build.props` wired for local iterative development (dotnet 10,
Serena MCP server, an overridable `ClefCompilerServiceProject` MSBuild property).

However, the repository is **not self-contained**: `Directory.Build.props` hardcodes
`ClefCompilerServiceProject` to `$(MSBuildThisFileDirectory)../clef/src/Compiler/Clef.Compiler.Service.fsproj`
— an MSBuild `ProjectReference` to a **sibling checkout** of `FidelityFramework/clef` that must sit next
to this repo on disk (`../clef`) for Composer to build at all. A second `ProjectReference` similarly
requires a sibling `../../Thuja/src/Thuja/Thuja.fsproj` checkout (Thuja: TUI foundation; provenance/org
not established from this clone alone — worth the coordinator resolving whether Thuja is also a
FidelityFramework repo). `docs/CCS_Architecture.md` explicitly documents source-level entry points as
GitHub links into `FidelityFramework/clef` (`FidprojLoader.fs`, `SourceResolver.fs`, `ProjectChecker.fs`,
`NativeService.fs`, `Types.fs`, `RangeAnalysis.fs`, `Escape.fs`, `Placement.fs`, `ObligationDischarge.fs`,
all under `clef/src/Compiler/...`). **Verdict: Composer is the primary repo for the middle/back end
(Alex: PSG→MLIR lowering, LLVM/MCU/CIRCT/AIE/GPU backends), but it is not the primary repo for the PSG
itself** — PSG schema, identity, construction, symbol correlation, reachability, enrichment/saturation,
coeffect analysis, and escape analysis all live in `FidelityFramework/clef`, which a sibling census
agent is covering. This split is load-bearing for correct attribution throughout this census; see
`## Mechanisms Census` below.

## Coarse Inventory

- Files observed (excluding `.git`): 789
- Bytes observed: 5,188,304
- Languages/signals: F# (.fs, 188 files), Markdown (.md, 213 files — unusually doc-heavy), Clef source
  samples (.fidproj/.clef, 138+113 files), TOML (.toml, 53), F# script (.fsx, 10), Python (.py, 13 —
  test harness/tooling, not compiler code), PowerShell (BuildAndPack.ps1), MLIR text sample, one `.ld`
  linker script, project/solution files (.fsproj ×17, .slnx ×1)
- Top-level directories: docs, samples, src, tests, tools
- Top-level files: .gitattributes, .gitignore, .mcp.json, .mise.toml, BuildAndPack.ps1, CLAUDE.md,
  Commercial.md, Composer.slnx, Directory.Build.props, FCS_Notes.md, LICENSE, PATENTS.md, README.md,
  psg-scope-representation-exploration.md, update_semantic_graph.py, vivado.jou, vivado.log

Full source tree staged with nested `.git` removed.

## Direct Dependencies

Resolved from `<PackageReference>`/`<ProjectReference>` across all 17 `.fsproj` files (no
paket.dependencies/packages.lock.json present; this repo uses plain NuGet PackageReference, no lockfile
committed — DEPENDENCY_CLOSURE for pinned transitive NuGet versions is therefore not reconstructable from
this clone alone, only the direct version ranges below).

NuGet (direct):
- FSharp.Core 10.1.401
- XParsec 0.3.0 — composable parser-combinator library, used throughout Alex for PSG pattern matching;
  origin not established as FidelityFramework-owned from this clone (no `FidelityFramework/XParsec` link
  found in README/docs) — treat as PARTIAL_TRANSITIVE / EXTERNAL until the coordinator confirms owner
- Argu 6.1.1 (CLI argument parsing)
- FSharp.SystemTextJson 1.2.42 (JSON serialization of intermediates)
- StreamJsonRpc 2.25.29 (Lattice.Server LSP-style editor integration)
- xunit 2.9.2, xunit.runner.visualstudio 2.8.2, Microsoft.NET.Test.Sdk 17.14.1 (test-only)

ProjectReference (sibling checkouts, NOT vendored, required for build):
- `../clef/src/Compiler/Clef.Compiler.Service.fsproj` (FidelityFramework/clef — CCS; hard build dependency,
  overridable via `Directory.Build.local.props`)
- `../../Thuja/src/Thuja/Thuja.fsproj` (Thuja — TUI foundation; org unconfirmed from this clone)

External toolchain (system binaries invoked as subprocesses, not NuGet packages — EXTERNAL_BOUNDARY,
terminal for dependency-closure purposes):
- `mlir-opt`, `mlir-translate` (MLIR/LLVM project) — invoked from `src/BackEnd/LLVM/Lowering.fs`,
  `Pipeline.fs`
- `opt`, `ld.lld` (LLVM project) — final native-binary pipeline
- `clang`/LLVM toolchain implied by backend docs but not directly shelled from the files inspected
- `cvc5` (SMT solver) — referenced throughout docs as the target of obligation discharge, but
  `ObligationDischarge.ofGraph` (in clef) only *projects* obligations to SMT-LIB; CCS_Architecture.md
  states explicitly it "does not invoke cvc5" — so cvc5 integration is DOCUMENTED, not wired, as of this
  pin

Dependency closure status: **DIRECT_ONLY** for NuGet (no lockfile to walk transitively) plus
**PARTIAL_TRANSITIVE** acknowledgment of two hard sibling-repo ProjectReferences (clef, Thuja) that are
out of scope for this agent's clone (clef is a sibling agent's donor; Thuja was not cloned). Never claim
COMPLETE.

## Build Systems Detected

- `.atlas/temporary/donors/composer/Composer.slnx` (new-style .NET solution file)
- `.atlas/temporary/donors/composer/src/Composer.fsproj` (main compiler, `dotnet build`, produces a
  `dotnet tool` named `composer`)
- `.atlas/temporary/donors/composer/src/CCS.Editor/CCS.Editor.fsproj`
- `.atlas/temporary/donors/composer/src/Lattice.Server/Lattice.Server.fsproj`
- 14 `.fsproj` files under `tests/` (xunit-based)
- `.atlas/temporary/donors/composer/Directory.Build.props` (shared MSBuild property injection —
  `ClefCompilerServiceProject` indirection)
- `.atlas/temporary/donors/composer/.mise.toml` (pins `dotnet = "10"` via mise tool manager)
- `.atlas/temporary/donors/composer/BuildAndPack.ps1` (PowerShell packaging script)
- No Makefile, no CMake, no separate CI workflow files observed under this clone's top level (no
  `.github/` directory present in this clone)

## Test / Benchmark Roots Detected

- `.atlas/temporary/donors/composer/tests/Alex.Tests`
- `.atlas/temporary/donors/composer/tests/CCS.Editor.Tests`
- `.atlas/temporary/donors/composer/tests/CortexMTargets`, `DeviceAccess`, `EspImage`, `IOMap`, `MCU`,
  `Mmio`, `NativeCallbacks`, `NativeSequences`, `PlatformCatalog`, `PlatformComposition`,
  `SourceAdmission`, `XtensaLayout` (all xunit-based `.Tests.fsproj`)
- `.atlas/temporary/donors/composer/tests/regression` (primary end-to-end regression runner,
  `Runner.fsx`, drives the `samples/console/FidelityHelloWorld/*` sample corpus and compares against
  intermediate/binary output — this is the closest thing to a semantic-preservation check actually
  exercised in CI-style form)
- `.atlas/temporary/donors/composer/tests/Fixtures`, `ForeignReferences`, `ForeignScalarArrays`,
  `MemoryArrays`, `NativeMath`, `PlatformFormat` (fixture/support directories, some containing Python
  `__pycache__` — indicates a Python-based cross-check harness alongside the F# tests)
- No dedicated `benches/` directory found; performance work is not benchmark-harnessed in this clone

## Major Subsystem Roots

- `src/FrontEnd/` — `CCS/Integration.fs` (thin re-export/consumption layer over CCS's `SemanticGraph`
  types — this is the entire "PSG intake" surface Composer owns), `ProjectLoader.fs` (.fidproj → CCS
  parse+check), `Templates/` (platform project scaffolding)
- `src/MiddleEnd/Alex/` — the real, substantial, Composer-owned code: `Traversal/` (PSGZipper, Huet
  zipper over the CCS-supplied SemanticGraph; `NanopassArchitecture.fs`, single-phase post-order
  traversal where each Witness = one nanopass), `XParsec/` (PSG-specific parser-combinator layer,
  `PSGCombinators.fs` at 1020 lines is the largest single file examined), `Elements/` (9 files, atomic
  MLIR op builders, `module internal`), `Patterns/` (17 files, composable MLIR templates, public),
  `Witnesses/` (27 files, thin ~20-line observers, one per PSG node category), `Dialects/Core/` (MLIR
  type system + serializer), `CodeGeneration/` (type mapping), `Pipeline/` (`MLIRNanopass.fs` — the one
  real MLIR→MLIR pass Composer performs, `CompilationTypes.fs`)
- `src/BackEnd/` — `LLVM/` (mlir-opt/mlir-translate/opt/ld.lld orchestration), `MCU/` (Xtensa/ESP32
  image building, hardware probe), `CIRCT/` (FPGA/HDL lowering), `AIE/` (NPU dialects), `GPU/`
- `src/Core/` — `CompilationOrchestrator.fs`, `CompilerConfig.fs`, `PlatformPipeline.fs`,
  `DeviceAccessEvidence.fs`, `Utilities/IntermediateWriter.fs` (the `-k` intermediate-artifact dumper
  that makes each stage independently inspectable)
- `src/CLI/` — Argu-based command-line interface, diagnostics rendering
- `src/CCS.Editor/`, `src/Lattice.Server/` — LSP-adjacent editor integration (StreamJsonRpc), consumes
  CCS results for hover/diagnostics; explicitly documented as not re-checking with a separate FCS
  instance
- **Not present in this repo**: any `PSG/Builder.fs`, `PSG/Nanopass/*.fs`, `PSG/Reachability.fs` despite
  `CLAUDE.md`'s "Key Files" table listing exactly these paths under `/src/Core/PSG/` — those files do
  not exist in this checkout. `docs/PSG_Nanopass_Architecture.md` line 3 explains why: "PSG construction
  has been moved to CCS (Clef Compiler Services)... This document describes the nanopass principles that
  CCS uses for PSG construction" — i.e. CLAUDE.md's Key Files table is stale documentation left over from
  before that migration; the actual PSG-construction code is in `FidelityFramework/clef`.

## License Evidence

- `.atlas/licenses/donors/composer/LICENSE` — **Apache License 2.0**, dual-licensed with a Commercial
  License option. File header: "DUAL LICENSING NOTICE FOR FIREFLY / Copyright 2025 SpeakEZ Technologies,
  Inc." followed by the full standard Apache-2.0 text.
- `.atlas/licenses/donors/composer/Commercial.md` — describes the commercial licensing track (contact
  SpeakEZ Technologies for terms); copied for completeness, not itself an OSI license.
- `.atlas/licenses/donors/composer/PATENTS.md` — patent notice. Discloses U.S. Provisional Patent
  Application No. 63/786,247, **"System and Method for Zero-Copy Inter-Process Communication Using BARE
  Protocol"**, owned by SpeakEZ Technologies, Inc. This patent is scoped to BAREWire (zero-copy IPC), a
  separate Fidelity Framework component Composer merely integrates with — it does **not** appear to read
  on the PSG/nanopass/coeffect/MLIR-lowering mechanisms this census targets, but the notice is written as
  a blanket "applicable to the Fidelity Framework" statement, so the coordinator should treat any future
  BAREWire-adjacent absorption (IPC, serialization, zero-copy buffers) with this patent explicitly in
  mind. Apache-2.0's own Section 3 patent grant/defensive-termination clause applies to code used under
  that license.
- License identification: **Apache-2.0** (for non-commercial/open-source use; commercial use requires a
  separate commercial license per Commercial.md). Not "UNKNOWN" — a LICENSE file is present and
  unambiguous for the Apache track.

## Mechanisms Census

For every mechanism below, the primary question this donor's own documentation forces onto any census is
**"does this code live in Composer, or in `FidelityFramework/clef` (CCS)?"** — the answer is almost
uniformly "CCS", with Composer owning only the consumption/lowering side. This is stated by Composer's
own architecture docs, not inferred; see `docs/CCS_Architecture.md` and `docs/PSG_Nanopass_Architecture.md`
for the primary citations, both quoted above and below.

### PSG schema

**Owner: `FidelityFramework/clef`** (`Clef.Compiler.PSGSaturation.SemanticGraph.Types`, referenced from
Composer only via `src/FrontEnd/CCS/Integration.fs`'s type aliases: `CCSNode = SemanticNode`,
`CCSGraph = SemanticGraph`, `CCSKind = SemanticKind`, `CCSNodeId = NodeId`, `CCSType = NativeType`).
`docs/CCS_Architecture.md` links the schema source directly:
`https://github.com/FidelityFramework/clef/blob/main/src/Compiler/PSGSaturation/SemanticGraph/Types.fs`.
Composer does not define this schema; it imports and re-exports a narrow accessor surface (~40
functions in Integration.fs: `nodeId`, `nodeType`, `nodeKind`, `nodeChildren`, `bindingInfo`, `isLambda`,
`isVarRef`, `varRefDefinition`, etc.).

### PSG identity

**Owner: `FidelityFramework/clef`.** Node identity is `NodeId` (a wrapped int, per
`Integration.fs:42 nodeIdToInt (NodeId id) = id`), assigned during CCS's structural-construction phase.
Composer's Alex layer never mints new PSG node identities; SSA *value* names (distinct from PSG node
identity) are derived downstream in Composer's own `src/MiddleEnd/Alex/Traversal/Values.fs`, explicitly
documented as "not a pre-assigned CCS coeffect" — i.e. SSA naming is a genuinely Composer-owned
mechanism, layered on top of CCS-owned node identity.

### Structural construction

**Owner: `FidelityFramework/clef`.** `docs/PSG_Nanopass_Architecture.md` describes Phase 1 (SynExpr →
PSG nodes + ChildOf edges) in detail as CCS's responsibility, explicitly contrasting it with an earlier
("What We Had") monolithic-builder design that Composer used to own before a January 2026 architecture
move. Not present as executable code in this clone.

### Symbol correlation

**Owner: `FidelityFramework/clef`.** Phase 2 of the documented pipeline: `ClefSymbol` attachment via
`GetAllUsesOfAllSymbols()` correlated by source range. Documented in detail (including the
SynExpr/ClefExpr correlation problem and a `TypedTreeZipper` design with `TypedFocus`/`PSGFocus`/
`TypedPath`/`PSGPath`/`TypeInfo` fields) in `docs/PSG_Nanopass_Architecture.md`, but the described
`TypedTreeZipper` is itself CCS-side design documentation, not a file present in this clone.

### Reachability

**Owner: `FidelityFramework/clef`**, consumed by Composer. Design: **soft-delete** reachability — nodes
get `IsReachable: bool` marked false, never physically removed, specifically so a downstream typed-tree
zipper retains full structure to navigate. `docs/CCS_Architecture.md` is candid that this is the *only*
current node-state field ("`Live`, `Latent`, and `Fresh` are not the current node-state API"), separate
from a `ReachabilityContext` (`Reachable`/`Unreachable`/`Unknown`) used for diagnostics. Composer's own
`NanopassArchitecture.fs` traversal (`visitAllNodes`) is written to walk the graph as delivered — it does
not itself compute reachability, it consumes CCS's `IsReachable` marks.

### Enrichment / saturation

**Owner: `FidelityFramework/clef`**, under the name **"Baker"** (`Clef.Compiler.Baker.Pipeline`,
imported by Composer only to call `emitBakerIntermediates` for `-k` debug dumps and to read lazily
computed `ModuleClassifications`). Composer's README describes a 5-phase PSG pipeline (Structural
Construction → Symbol Correlation → Reachability → Typed Tree Overlay [Baker] → Enrichment Nanopasses)
as if it were Composer's own — it is not; every phase after Phase 0 (CCS Parse) is CCS/Baker-owned per
the docs above. **Is it genuinely nanopass (many small composable passes) or a differently-named
monolith?** From the CCS-side evidence available in this clone (docs only, not code — the actual Baker
pipeline source is in the sibling repo and out of this agent's scope): the design intent is genuinely
fine-grained (`RangeAnalysis.run`, `Placement.settle`, `Placement.closures`, `Escape` as a distinct
module, `ObligationElaboration`, `ObligationDischarge` are each named as separate producers feeding a
shared `Codata`/`Layouts` structure) — this looks like real nanopass decomposition by name and by the
producer-table in `docs/CCS_Architecture.md`, not vague marketing, but this census cannot verify the Baker
pipeline's actual source structure since it lives in `clef`, out of scope for this agent. The sibling
agent censusing `clef` should verify directly against `Clef.Compiler.Baker.Pipeline` source.

**What Composer itself owns that could be called "nanopass" in its own right**: the Alex Witness
traversal. `src/MiddleEnd/Alex/Traversal/NanopassArchitecture.fs` (403 lines) implements "each witness =
one nanopass, run over a single post-order PSG traversal" — but on inspection this is **not** classic
nanopass architecture (many sequential small IR→IR passes each producing a new IR generation). It is a
**single traversal** with ~27 Witness modules (`src/Composer.fsproj` lists LiteralWitness,
TypeAnnotationWitness, IntrinsicWitness, ControlFlowWitness, MatchWitness, RecordWitness, DUWitness,
BindingWitness, LambdaWitness, etc.) each pattern-matching a disjoint category of PSG node during **one**
post-order walk, directly emitting MLIR ops — no intermediate IR generations between witnesses. This is
architecturally closer to a **single-pass, multi-handler visitor with disjoint dispatch by node category**
than to the nanopass framework's actual definition (Sarkar/Waddell/Dybvig/Keep: each pass reads one
formally-defined input language and produces one formally-defined output language). The "nanopass" naming
for Alex specifically is aspirational/borrowed vocabulary, not a literal match to the cited framework —
this should be flagged, distinctly from CCS/Baker's PSG-enrichment pipeline (Phases 1-5+), which by the
same evidence does look like genuine multi-generation nanopass staging (each phase has an explicit
`PSG_n → PSG_{n+1}` transformation with a named intermediate artifact file, e.g.
`03_psg1.json`→`05_psg2.json`→`06_coeffects.json`).

### Def/use

**Owner: `FidelityFramework/clef`.** Named as `AddDefUseEdges` in the documented Phase 5+ enrichment
sequence (`PSG₆ → AddDefUseEdges → PSG₇`), producing a `SymbolUse` edge kind. Not present as code in this
clone; `VarRef` correlation (`varRefDefinition`) is consumed by Composer via Integration.fs but computed
upstream.

### Coeffect analysis

**Owner: `FidelityFramework/clef`, real (not merely aspirational documentation), but only partially
wired end-to-end.** This is the most theoretically interesting and best-evidenced mechanism in this
donor. Composer's README states the doctrine plainly: "Coeffects Over Runtime — Pre-computed analysis
(SSA assignment, platform resolution, mutability tracking, DU layouts) guides code generation. No runtime
discovery. Coeffects are computed once before Alex witnessing begins." `docs/CCS_Architecture.md`
contains an explicit table, "Coeffects computed during elaboration and saturation," listing concrete
producers with named source files: `RangeAnalysis.run` (integer ranges, in
`clef/.../SemanticGraph/RangeAnalysis.fs`), `Placement.settle`/`Placement.closures` (aggregate layouts
and closure placement, `clef/.../SemanticGraph/Placement.fs`), `Escape` module (escape-kind analysis,
`clef/.../SemanticGraph/Escape.fs`), and `ObligationElaboration`/`ObligationDischarge` (proof obligations
projected to SMT-LIB). Crucially, one coeffect-shaped fact is explicitly documented as **Composer-owned,
not CCS-owned**: SSA names, "Derived during emission from node identity in
[Values.fs](../src/MiddleEnd/Alex/Traversal/Values.fs); not a pre-assigned CCS coeffect" — i.e. the
donor's own docs distinguish a true (CCS-computed, pre-emission) coeffect from an emission-time derived
value, which is a genuinely precise use of the coeffect-system vocabulary (coeffects as *required
context* computed ahead of use, as opposed to effects computed as a result of use) — this is not a loose
or decorative use of the term. Verdict: **real implementation exists** (concrete named modules/functions
with file-line citations), but it lives entirely in `clef`, and even CCS_Architecture.md flags real gaps:
`ObligationDischarge` "returns no solver verdict," does "not invoke cvc5," and a general
editor-facing pending-state contract "remains to be implemented."

### Mutable-state / escape analysis

**Owner: `FidelityFramework/clef`** — `Escape.fs`, producing an `EscapeKind` sum type distinguishing
`StackScoped`, `EscapesViaClosure`, `EscapesViaReturn`, `EscapesViaByRef`, `StaticLifetime`, explicitly
documented as "analysis results, not a one-to-one table." README's "Known Limitations" section (dated
Feb 2026, itself explicitly flagged as historical/stale in the same README) says escape analysis is
"Partial... closure capture detection works, mutable lifetime integration pending" — so even by the
donor's own account, this mechanism is real but incomplete as of the pinned commit; Composer's own side
of it (the code that must *consume* escape-kind facts to decide alloca-vs-heap/arena placement in Alex
witnesses, e.g. `MutableAssignmentWitness.fs`, `BindingWitness.fs`, `VarRefWitness.fs`) does exist in this
clone as real F# source and was spot-checked as present.

### Nanopass boundaries

Covered above under Enrichment/saturation. Summary distinction for the coordinator: **CCS-side PSG
enrichment (Phases 1–5+) is architecturally closer to genuine nanopass staging** (named intermediate
artifacts per phase); **Composer-side Alex traversal ("NanopassArchitecture.fs") is a single-pass
multi-witness visitor that borrows nanopass terminology but does not implement the framework's
defining property of successive small IR-to-IR transformations** — it is one traversal with ~27 disjoint
per-category handlers. Composer's own `MLIRNanopass.fs` module-doc is unusually candid about this;
current code performs exactly **one** MLIR→MLIR step (declaration collection/relocation) and its comment
explicitly states "there is no middle-end lowering to any continuation or net dialect, and no dialect
above the witness boundary for one to target" — contradicting the README's older claim of "4 passes"
(structural folding, declaration collection, type normalization, FFI conversion). This is an internal
documentation/code drift worth flagging as a Discovery.

### PSG → MLIR lowering (does this repo contain the real code, or is it aspirational/lives in clef?)

**Owner: Composer, genuinely and substantially.** This is the one major PSG-adjacent mechanism that
*does* live in this repo as real, non-trivial code, not just documentation: `src/MiddleEnd/Alex/` totals
dozens of files across `Elements/` (9), `Patterns/` (17), `Witnesses/` (27), `Traversal/` (PSGZipper —
a genuine Huet zipper, 247 lines — plus ScopeContext, TransferTypes, CoverageValidation,
NanopassArchitecture, StaticStorageValidation, MLIRTransfer, XDCTransfer, SMTTransfer at 629 lines),
`XParsec/` (PSGCombinators.fs at 1020 lines, the largest inspected file — real composable
pattern-matcher combinators over PSG structure, not a stub), and `Dialects/Core/` (an MLIR type system
+ serializer). The output is genuinely portable MLIR (`memref`, `arith`, `func`, `index`, `scf` dialects
only — enforced as doctrine, see Thin Middle End below), handed to the external `mlir-opt`/
`mlir-translate`/`opt`/`ld.lld` toolchain for final lowering to LLVM IR and native code
(`src/BackEnd/LLVM/Lowering.fs`, `Pipeline.fs`, confirmed via direct grep for `mlir-opt`
`Process.Start` invocation). **Sibling-repo split**: this confirms the task's suspicion in reverse of
its stated framing — the *lowering* (PSG→MLIR) is real and lives in Composer; it is the *PSG itself*
(construction/enrichment/coeffects) that lives in the sibling `clef` repo, not the other way around.
Coordinator should reconcile attribution accordingly: don't credit `clef` for the MLIR lowering, and
don't credit Composer for the PSG/coeffect/saturation machinery.

### Semantic preservation through lowering (is information lost, and is there a detection/prevention mechanism?)

**Real, specific, partially-verified mechanism — the strongest single finding in this census.**
Documented in full in `docs/Thin_Middle_End_Design.md` ("Doctrine: MLIR is kept as thin as possible in
the middle end"). The mechanism has three concrete parts:

1. **Architectural ban, stated as a checkable invariant**: "no llvm dialect, and no semantic dialect, in
   the MLIR witnessed out of the PSG... The count is zero, and it stays zero. This is a checkable
   property of the pipeline." The claim is that all information that could motivate a semantic dialect
   (liveness, escape class, extent, target profile) is already resolved upstream in the CCS/Baker
   saturated graph before Alex ever runs, so MLIR emission is pure transliteration, never
   reconstruction. The doc explicitly frames this against Appel's "SSA is functional programming"
   argument: pushing semantics into ops forces downstream passes to reconstruct upstream knowledge
   approximately; this pipeline's answer is to never let that knowledge leave the graph.
2. **A documented history of dialects being proposed and then dissolved back into standard-dialect
   decompositions** when it was shown the standard dialects sufficed (a closure dialect, a delimited-
   continuations/"DCont" dialect, an early anonymous `builtin.unrealized_conversion_cast` later replaced
   with a named "materialize/scatter" pair governed by a stated round-trip law: "scatter after
   materialize is the identity on the value it carried. The law is checkable.").
3. **A partial verification path via obligation correspondence**: CCS's `ObligationElaboration`/
   `ObligationDischarge` (in `clef`) project proof obligations to SMT-LIB; Composer's own
   `src/MiddleEnd/Alex/Traversal/SMTTransfer.fs` (629 lines, real code, inspected) "participates in the
   source/lowering correspondence scaffold." **However**, `docs/CCS_Architecture.md` is explicit that
   this is incomplete: `ObligationDischarge` "does not invoke cvc5, record a verdict in `CheckResult`, or
   provide a live editor notification... A reported verdict needs an actual dispatch result tied to the
   obligation... Live dispatch and editor evidence status are separate integration work." So: the
   *design* for detecting semantic loss through lowering is real, specific, and partially implemented
   (SMT-LIB projection + a named round-trip law + an architectural zero-semantic-dialect invariant); the
   *automated verification* (actually running cvc5 and gating on the verdict) is documented as not yet
   wired up as of this pin.

## Atlas Comparison

Atlas's own HIR/MIR/LIR/Machine IR pipeline (`.atlas/contracts/COMPILER-IR-PIPELINE.md`) is **contract-
defined, not implemented in code**, as of this census (confirmed: no HIR/MIR/AtlasX compiler code exists
under `core/`, `apps/`; `core/src/semantic/` has typed record *schemas* — `call.rs`, `control_flow.rs`,
`data_flow.rs`, `effect.rs`, `state.rs`, `obligation.rs` — but per the task's ground truth only SYMBOL,
TYPE, FUNCTION_IDENTITY, FUNCTION_SIGNATURE, and the newly-landed CALL fact (dispatch always
`UNRESOLVED`) are actually materialized end-to-end; CONTROL_FLOW/DATA_FLOW/STATE/EFFECT/OWNERSHIP/
CONCURRENCY/PERSISTENCE are named but not implemented). Every comparison below is therefore necessarily
"Composer's/CCS's real-or-partially-real PSG/coeffect architecture vs Atlas's documented-but-unbuilt
HIR/MIR contract" — stated explicitly per mechanism:

- **PSG schema vs Atlas's SemanticRecordHeader/FactKind spine**: Atlas already has (contract-level, and
  partially code-level for the materialized 5 facts) a comparable idea — a typed semantic record with a
  common header (`SEMANTIC-FACTS.md`) rather than CCS's single mutable `SemanticGraph` with typed node
  `Kind`. Structurally different shapes (Atlas: normalized fact records + `UNIVERSAL-GRAPH-CONTRACT.md`'s
  Node/Edge/Binding graph, derived; CCS: one graph *is* the PSG, mutated/enriched in place across nanopass
  phases) — not directly comparable maturity-for-maturity since Atlas's side here is largely DECLARED
  (contract) while CCS's PSG schema is OBSERVED (real code, in the sibling repo).
- **Coeffect analysis vs Atlas's DataFlowFact/StateAccessFact/EffectFact**: Atlas's contract already
  requires (`SEMANTIC-FACTS.md`) that DataFlowFact/StateAccessFact/EffectFact/OwnershipFact exist as
  typed record kinds, but per the task's ground truth these are UNSUPPORTED/not implemented today. CCS's
  coeffect system is a genuinely implemented (if incomplete) precedent for *how* to compute and attach
  this class of fact: precompute once during elaboration/saturation, attach as lazy fields on the graph
  (`SemanticGraph.Codata`, `Layouts`), and forbid downstream stages from re-deriving it. This is a
  concrete implementation pattern Atlas's own HIR/MIR contract does not yet specify a mechanism for (the
  contract says facts must "survive explicitly in the IR" or "as attached typed semantic
  barrier/metadata," which is compatible with but less specific than CCS's coeffect pattern).
- **Semantic preservation through lowering vs Atlas's "Semantic equivalence" section**: Atlas's
  `COMPILER-IR-PIPELINE.md` already lists acceptable evidence classes for stage-transition equivalence
  (structural invariant checking, differential execution, property tests, translation validation, SMT/
  proof checks, model checking, reference backend comparison, runtime trace comparison) — this is a
  broader menu than Composer's specific technique, and Composer's "zero semantic dialects below the
  boundary" + "named round-trip law" pattern fits as one concrete instance of "translation validation" /
  "SMT/proof checks" already anticipated by Atlas's contract. **This is not a gap Atlas's contract
  structure is missing — it is a concrete, well-evidenced example of a technique the contract already
  gestures at generically.** See Blueprint Revision Candidates below for the one place this still
  produces an actionable, bounded suggestion.
- **Nanopass boundaries vs Atlas's stage principle**: Atlas's `COMPILER-IR-PIPELINE.md` "Stage principle"
  (one responsibility level per stage, lossless mapping to a versioned schema, "A stage MUST NOT depend
  on information that was silently discarded by an earlier stage") is a coarser-grained analog of the
  nanopass discipline CCS documents at PSG-phase granularity. Atlas currently defines 5 stages
  (HIR/MIR/LIR/Machine IR + AtlasX source); CCS documents on the order of 9+ PSG phases before Alex even
  starts. Both are DECLARED-vs-OBSERVED asymmetric comparisons on Atlas's side.
- **Escape/mutable-state analysis vs Atlas's OwnershipFact/MIR "ownership moves/borrows/copies" and
  "alias/escape optimization"**: Atlas's MIR contract already names escape/alias analysis as an MIR-stage
  responsibility ("MIR must make enough memory/resource behavior explicit for: ownership validation;
  alias/escape analysis...") and "If alias/ownership facts are unresolved, the compiler must conservatively
  preserve semantics." CCS's `Escape.fs`/`EscapeKind` enum (`StackScoped`, `EscapesViaClosure`,
  `EscapesViaReturn`, `EscapesViaByRef`, `StaticLifetime`) is a concrete, small, well-named sum type that
  could inform Atlas's eventual OwnershipFact/MIR escape representation once that stage is actually built
  — again DECLARED (Atlas) vs OBSERVED-but-incomplete (CCS).
- **PSG → MLIR lowering (Composer/Alex) vs Atlas's MIR → LIR / LIR → Machine IR**: Atlas's contract
  requires each lowering to declare "supported input schema/version... unsupported construct
  behavior... legality checks... semantic-equivalence obligations" — Composer/Alex's Witness/Pattern/
  Element stratification (Witnesses observe-only/codata, Patterns compose, Elements are atomic, with a
  documented rule that "Witnesses physically cannot import Elements") is a concrete, real, working
  instance of enforcing a layered lowering discipline via language-level visibility (F# `module
  internal`) rather than only convention/review — this is a genuinely portable implementation idea (not
  F#-specific: any language with module-private visibility, including Rust's `pub(crate)`/private
  modules, can enforce the same "lower layer cannot be imported by upper layer" invariant at compile
  time). Atlas's contract currently states layering as prose obligation only, with no enforcement
  mechanism named.

## Discoveries

- **[EXTERNAL_BOUNDARY, PSG schema/construction/enrichment]** The PSG itself (schema, identity,
  construction, symbol correlation, reachability, saturation/enrichment, coeffects, escape analysis) is
  not in this repository at all — it is entirely owned by `FidelityFramework/clef`, referenced via a hard
  MSBuild `ProjectReference` to a sibling checkout. Composer cannot even compile without `clef` present
  at `../clef` on disk. Coordinator action: do not attribute PSG/coeffect/nanopass-enrichment findings to
  "Composer" in the cross-donor comparison; attribute them to `clef`/CCS and cross-reference the sibling
  agent's `clef` census for verification against actual source (this census could only verify via
  Composer's own documentation of `clef`'s structure, which is unusually precise and links directly to
  file/line in `clef`, but is still documentation, not source, from this agent's vantage point).
- **[ABSORB_LATER, semantic preservation / thin-middle-end doctrine]** The "zero semantic dialects below
  the witness boundary" architectural invariant, plus the "named round-trip law, checkable" pattern
  (materialize/scatter identity) is a concrete, well-evidenced, genuinely portable idea independent of
  F#/MLIR specifics: a compiler can enforce "no reconstruction of upstream facts downstream" as a
  structural property of its own IR vocabulary rather than only through code review. Relevant to Atlas's
  eventual HIR→MIR→LIR boundary design. See Blueprint Revision Candidates.
  Mechanism-scope: compiler/mir, compiler/lir.
  Provider: Composer (doctrine + Alex enforcement), with corroborating evidence in `clef`'s
  ObligationElaboration/Discharge (not independently verified by this agent).
- **[REFERENCE_ONLY, Witness/Pattern/Element layered-visibility enforcement]** Using host-language module
  visibility (`module internal` in F#) to make an architectural layering rule (observers cannot import
  primitives; must go through the composed-template layer) a compiler-enforced invariant rather than a
  documentation-only convention. Directly portable to Rust module privacy. Mechanism-scope:
  compiler/mir, compiler/lir, general compiler-pass architecture.
  Provider: Composer (`src/MiddleEnd/Alex/Elements/*.fs` marked `module internal`, consumed only via
  `Patterns/*.fs`, observed only via `Witnesses/*.fs`).
- **[REFERENCE_ONLY, documentation honesty pattern]** `docs/CCS_Architecture.md` is unusually precise
  about implementation-vs-aspiration status per mechanism, including phrases like "does not invoke
  cvc5," "not a pre-assigned CCS coeffect," "Live dispatch... [is] separate integration work," and a
  running "current main implementation and limit" column in its coeffects table. This is a genuinely
  good documentation discipline (write the doc so a reader cannot mistake design intent for shipped
  behavior) that resembles Atlas's own EpistemicStatus discipline (OBSERVED vs DECLARED vs UNSUPPORTED).
  Worth noting as a positive precedent for Atlas's own contract/blueprint-writing practice, not a code
  mechanism to absorb.
- **[REJECT, "4 MLIR structural passes" claim]** README.md's architecture diagram claims "MLIR Structural
  Passes (4 passes)" (structural folding, declaration collection, type normalization, FFI conversion),
  but the actual `src/MiddleEnd/Alex/Pipeline/MLIRNanopass.fs` module-doc comment (current code, same
  commit) states there is exactly **one** MLIR-to-MLIR step (declaration collection/relocation) and
  explicitly disclaims any further semantic middle-end lowering. This is stale README content
  contradicted by the code's own doc-comment at the same pin — flagged so the coordinator does not carry
  the "4 passes" claim into any cross-donor comparison table as fact.
  Mechanism-scope: compiler/mir lowering-pass-count claims specifically.
- **[REFERENCE_ONLY, nanopass-terminology precision]** "Nanopass" as applied to Composer's own Alex
  Witness traversal (`NanopassArchitecture.fs`) does not match the cited Nanopass Framework's actual
  definition (many small successive IR-to-IR passes); it is one traversal with ~27 disjoint per-category
  handlers. The CCS-side PSG enrichment pipeline (Phases 1-5+, each with a named intermediate JSON
  artifact) is the part of this donor's architecture that *does* match genuine nanopass staging — but
  that code lives in `clef`, unverified directly by this agent. Coordinator should not credit "Composer"
  wholesale with "true nanopass architecture" without this distinction; the badge in README
  ("Pipeline-25 nanopasses") most likely counts CCS-side PSG phases plus Alex witnesses together, which
  conflates two architecturally different things.
  Mechanism-scope: compiler pass-architecture terminology generally.
- **[ABSORB_LATER, PHG/hyperedge roadmap]** `docs/PSG_Nanopass_Architecture.md`'s addendum ("From PSG to
  PHG") documents a **landed** (2026-09, per the doc, same month as this pin) extension: first-class
  hyperedges on the graph (`Hyperedge {Sources; Target; Class; Role; Ordinal}`), used specifically to
  express multi-way relationships (e.g. an extern/FFI boundary's joint lifetime claim across declaration,
  marshaled arguments, lifetime owner and ABI contract as one atomic hyperedge, rather than several
  pairwise edges that could each individually hold while the joint claim fails) that plain binary
  PSG edges cannot represent without lossy decomposition into auxiliary join/split nodes. This is a
  specific, reasoned answer to a real graph-representation problem (multi-way relationships losing their
  jointness when flattened to binary edges) that Atlas's `UNIVERSAL-GRAPH-CONTRACT.md` does not currently
  address — that contract defines `Node`/`Edge`/`Binding` but `Binding` is described as "an explicit
  connection/realization between endpoints" (still fundamentally pairwise-shaped, not stated as N-ary).
  This is documented design, not verified in code by this agent (the doc says it landed in
  `SemanticGraph.Edges` in `clef`, unverified here) — flagged ABSORB_LATER, mechanism-scope:
  graph/engineering_graph, compiler/hir. See Blueprint Revision Candidates.

## Blueprint Revision Candidates

Two candidates rise to a level worth stating explicitly, both bounded and evidence-based rather than
speculative:

1. **N-ary/hyperedge relationships in the Universal Graph Contract.** Composer/CCS's documented PHG
   (Program Hypergraph) work gives a concrete, well-argued example of a real information-loss failure
   mode in purely-binary-edge graphs: a joint multi-participant constraint (their example: an FFI/extern
   boundary's lifetime claim spanning declaration, marshaled arguments, lifetime owner and ABI contract)
   decomposes into pairwise edges that can each individually hold while the joint claim fails — i.e.
   binary decomposition is provably lossy for this class of fact, not merely inconvenient. Atlas's
   `UNIVERSAL-GRAPH-CONTRACT.md` currently defines only `Node`/`Edge`/`Binding`, all pairwise-shaped by
   the contract's own prose ("A Binding states an explicit connection/realization between endpoints").
   **Recommendation for the coordinator to weigh**: consider whether `UNIVERSAL-GRAPH-CONTRACT.md` should
   explicitly account for N-ary relations (a `Hyperedge` or equivalent primitive with an ordered/typed
   participant set) for cases — plausibly including Atlas's own EffectFact/OwnershipFact/ConcurrencyFact
   once those are materialized — where a single semantic claim genuinely spans more than two identities
   jointly (e.g. an FFI boundary, a multi-variable closure capture, a multi-party transaction). This is
   evidence-based (a real, specific failure mode is named and argued, not just "hypergraphs are cooler"),
   but it is still a design document's argument, not benchmarked/tested code — the coordinator should
   weight it as a real candidate worth evaluating, not as proven necessity.
2. **A named, checkable round-trip law as one concrete instance of "translation validation" in the
   Semantic Equivalence section.** Atlas's `COMPILER-IR-PIPELINE.md` already lists "translation
   validation" and "SMT/proof checks" as acceptable equivalence evidence generically. Composer's
   materialize/scatter round-trip law ("scatter after materialize is the identity on the value it
   carried... checkable") is a small, concrete, low-risk pattern worth citing as a *named example* under
   that section once Atlas's HIR/MIR/LIR lowerings are implemented — not a structural change to the
   contract, just a documented technique. This is a weak/optional candidate: it does not require a
   contract revision since the contract already permits it; flagging mainly so the coordinator has the
   concrete precedent on file when HIR→MIR lowering is actually built.

No other findings in this census rise to the level of requiring a contract/blueprint revision. The
coeffect-computation pattern and the layered-visibility enforcement pattern (Discoveries above) are real
and useful but are implementation techniques Atlas's contracts already permit or are silent on at a level
where a documented example, not a contract change, suffices.

## Recensus Requirements

- Re-census if `FidelityFramework/clef`'s pin used by the sibling census agent changes materially,
  specifically around `SemanticGraph/Types.fs`, `RangeAnalysis.fs`, `Escape.fs`, `Placement.fs`,
  `ObligationDischarge.fs`, or the Baker pipeline — since this census's PSG/coeffect/enrichment findings
  are sourced from Composer's *documentation* of that repo, not its source, cross-verification against
  the sibling agent's direct `clef` census is required before any ABSORB_NOW disposition on PSG/coeffect
  mechanisms.
- Re-census if `Directory.Build.props`'s `ClefCompilerServiceProject` default path or the Thuja
  ProjectReference changes, as either would indicate a restructuring of the Composer/clef boundary this
  census relies on.
- Re-census if `docs/Thin_Middle_End_Design.md`'s "the count is zero" invariant is ever violated
  (a semantic dialect added to the witnessed MLIR region) — this would materially change the semantic-
  preservation-through-lowering finding that is this census's strongest evidence-backed result.
- The dependency closure for NuGet transitive versions was not walked (no lockfile); if Atlas ever
  vendors/depends on XParsec, StreamJsonRpc, or FSharp.SystemTextJson directly, a proper transitive
  resolution (via `dotnet list package --include-transitive` against a real restore, which this
  static-inspection-only census deliberately did not perform) is required first.
- Thuja's organization/provenance was not established from this clone; recensus once that's known would
  let target_owners include TUI-adjacent lanes if relevant.

## Census State

Status: COARSE_CENSUSED + TARGETED_DEEP_CENSUS on the specified PSG/nanopass/coeffect/lowering
mechanism set. This is an admission-stage inventory with a rigorous documentation-sourced deep dive on
the requested mechanisms; it is explicitly **not** a full audit of `FidelityFramework/clef`'s source (out
of this agent's scope, censused by a sibling agent) nor of the full Alex/BackEnd F# source beyond the
files directly inspected (spot-checked: `Integration.fs`, `NanopassArchitecture.fs`, `PSGZipper.fs`,
`MLIRNanopass.fs`, `MLIRGeneration.fs`, `PSGCombinators.fs` size-only, `SMTTransfer.fs` size-only, plus
`.fsproj`/build-system files). Deep behavioral/correctness verification of the ~27 Witness modules, the
17 Pattern modules, and the 9 Element modules was not performed and would require either running the
(untrusted, unexecuted per this task's constraints) build or a further line-by-line reading pass.

## G94 — terminal REFERENCE_ONLY; source extinct

The recorded hyperedge falsification was executed on Atlas's real system graph. Every call claim is already reified as a `CallSite` join node with pairwise caller, callee, position-keyed argument and result edges. All 17,421 reconstruct unambiguously, and 3,244 of them join more than two identities (up to 9), so pairwise decomposition loses nothing.

The thin-middle-end doctrine constrains lowering code that does not exist yet. COMPILER-IR-PIPELINE.md's semantic-equivalence section already anticipates it, so it stays a design reference. The checkout was physically deleted. Evidence: `../../evidence/campaign/30-composer.json`.
