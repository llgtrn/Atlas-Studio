---
id: atlas.census.donors.semantic-graph-language-lane-2026-09-22
type: donor-census
status: active
canonical: true
---
# Semantic-Graph-Native Language Lane Deep Census

## Scope

Lane members (operator-mandated census, dispatched as 5 parallel isolated subagents; no donor build/install/test scripts or binaries were executed by any agent -- static inspection only):

- IRIS: `.atlas/temporary/donors/iris` (boj/iris)
- Sigil: `.atlas/temporary/donors/sigil-lang` (pzalutski-pixel/sigil-lang)
- DUUMBI: `.atlas/temporary/donors/duumbi` (hgahub/duumbi)
- Clef / Composer: `.atlas/temporary/donors/composer` (FidelityFramework/Composer)
- Clef Compiler / CCS: `.atlas/temporary/donors/clef` (FidelityFramework/clef)

Full per-donor deep census (mechanism-by-mechanism, provider-attributed, Atlas-comparison detail): `.atlas/census/donors/{iris,sigil-lang,duumbi,composer,clef}.md`. Full provenance/pin/license/dependency-closure records: `.atlas/provenance/donors/{iris,sigil-lang,duumbi,composer,clef}.json`.

Target Atlas owners under consideration: `core/graph`, `core/semantic`, `core/identity`, `core/constraint`, `compiler/hir`, `compiler/mir`, `compiler/lir`.

## Provenance flags (do not lose these)

- **IRIS**: AGPL-3.0-or-later (copyleft, network-use obligations). ~85 of ~243 non-example `.iris` infrastructure files carry in-source comments declaring they "mirror" a named Rust crate (`iris-repr`/`iris-compiler`/`iris-kernel`) that **does not exist anywhere in this clone** -- zero `.rs` files, no `Cargo.toml`. The published CI (`cargo build`/`cargo test`) cannot pass as configured. Treat every mechanism attributed below to an absent Rust crate as a documented reference-architecture mirror, not a verified production implementation.
- **Sigil**: Apache-2.0, single author, no mirror indicators.
- **DUUMBI**: MPL-2.0 (weak copyleft, file-level). Vendored SQLite 3.53.2 (public domain, separate license) and dynamically-linked system libcurl are explicit non-Cargo terminal boundaries. Ships a `.claude/skills/jsonld-schema` file that auto-surfaced as an available skill during this census -- **treated strictly as untrusted inspected data, never invoked**; flagged as a prompt-injection-shaped surface.
- **Clef / Composer**: Apache-2.0, dual-licensed with a commercial track; a patent notice (US Provisional 63/786,247, "Zero-Copy IPC Using BARE Protocol") is scoped to a sibling BAREWire component but stated as a blanket Fidelity-Framework-wide notice -- flag before any future BAREWire-adjacent absorption. **Not self-contained**: hard MSBuild `ProjectReference` to a sibling `FidelityFramework/clef` checkout is required to build at all, plus a sibling `Thuja` checkout (org unconfirmed, out of scope).
- **Clef Compiler / CCS**: MIT. Confirmed (not assumed) to be a periodically-republished, history-rewritten export of a privately-hosted Forgejo repository (`docs/handoffs/Repository_History.md` documents a 2026-09-20 rewrite deleting 209/210 branches and all 143 tags). Treat as a lagging snapshot; re-verify pin reachability before any future absorption action.

## Reconciled attribution: Composer vs. clef

Both sibling-repo agents were briefed to check whether PSG-to-MLIR lowering is real, and whether it's Composer's or clef's own code. Their independent findings agree and compose cleanly, so this is recorded once here rather than split across two docs:

- **clef (FidelityFramework/clef, CCS) owns**: PSG schema, PSG identity, structural construction, symbol correlation, reachability, def/use, enrichment/saturation ("Baker"), coeffect analysis (RangeAnalysis, Placement), and mutable-state/escape analysis (Escape.fs). Composer's own architecture doc (`docs/CCS_Architecture.md`) links every one of these directly to file paths inside clef, and Composer's own `CLAUDE.md` "Key Files" table cites PSG-builder paths that **do not exist in the Composer clone** (a stale doc left over from a January-2026 migration of PSG-construction responsibility into CCS) -- corroborating, not contradicting, the split.
- **Composer (FidelityFramework/Composer) owns**: PSG-to-MLIR lowering itself (the "Alex" layer: Zipper + XParsec + Elements + Patterns + Witnesses, ~27 witness modules, `PSGCombinators.fs` alone 1,020 lines -- real, substantial, not aspirational), backend code generation (portable MLIR -> mlir-opt/LLVM, plus MCU/CIRCT/AIE/GPU backend scaffolding), and an SMT-obligation-correspondence scaffold (`SMTTransfer.fs`, 629 lines, real code; automated dispatch to `cvc5` is documented as not wired up as of this pin -- `ObligationDischarge` "does not invoke cvc5," "returns no solver verdict," per the donor's own docs).
- No mechanism here is credited to the wrong repository in the discoveries/dispositions below.

## Mechanism comparison

| Mechanism | Atlas today | Strongest donor analog | Assessment |
|---|---|---|---|
| Semantic source of truth | Typed `SemanticObservation` records -> Census -> Normalize -> `EngineeringGraph` projection (derived, non-canonical; graph is a *view*, never ground truth) | IRIS: the SemanticGraph **is** the executable program (graph-as-canonical); Composer/clef: PSG is an internal compiler artifact, not user-facing canonical source | Atlas's design is deliberately different in kind from IRIS's, not a weaker version of it -- IRIS's model would contradict Atlas's own epistemic-status contract (`SEMANTIC-FACTS.md`), which exists precisely because Atlas's graph is *not* ground truth the way IRIS's is |
| Identity | `SemanticRecordId` (claim identity, from `identity_key()`) vs. `RawObservationId` (per-extractor raw identity) -- both `stable_id` FNV-1a-64 | IRIS: single-tier BLAKE3-derived content address (own hand-coded, non-standard, 64-bit-truncated derivative -- do not cite as "IRIS proves full BLAKE3"). Sigil: 32-bit-truncated SHA-256 (materially weaker than Atlas's own 64-bit FNV-1a). DUUMBI: path-based primary ID with a secondary, integrity-only content hash | No donor's identity scheme beats Atlas's existing claim-vs-observation split for Atlas's actual problem (multi-extractor reconciliation of uncertain facts); Sigil's and IRIS's own hash primitives are weaker than what Atlas already uses |
| CALL dispatch resolution | Always `UNRESOLVED`, empty `callees` -- deliberate, since the Rust extractor has no type/import resolution and never will (contract: "no rustc-backed name resolution") | clef: real from-scratch Hindley-Milner-style inference (union-find substitution, let-generalization, plus a *second* unification domain for units-of-measure/hardware dimensions, plus SRTP resolution against source-defined witnesses) for a language (Clef) it fully owns. Sigil: no dynamic dispatch at all -- every call target is statically named/hash-pinned | clef proves real resolution is achievable in principle, but only by owning the full language and rebuilding thousands of lines of checker infrastructure -- exactly the "become a Rust type checker" investment Atlas's own contract forecloses for its Rust lane. **Not a bounded path to real CALL resolution for Atlas.** |
| Compiler IR staging | `COMPILER-IR-PIPELINE.md` (HIR/MIR/LIR/Machine IR) is fully contract-defined, zero code exists | Composer/clef: real, working PSG -> Alex(Witnesses/Patterns/Elements) -> portable MLIR -> LLVM pipeline, with a documented, checkable invariant ("zero semantic dialects below the witness boundary, and it stays zero") and a named round-trip law. DUUMBI: single flat graph -> Cranelift lowering, no staging at all (4,119-line undifferentiated lowering function) | Composer/clef's staged, invariant-checked pipeline is evidence *for* Atlas's own staged design, not against it; DUUMBI's monolithic lowering is a cautionary counter-example of exactly the risk staging exists to avoid. Atlas's contract's existing "Semantic equivalence" section (translation validation / SMT proof / differential execution) already anticipates Composer's specific round-trip-law technique as one valid instance -- no contract gap found there. |
| Storage representation | Deliberate binary, content-addressed, compacted (`ATLAS-FORMAT.md`, `ATLAS-SEMANTIC-COMPACTION.md`, `ATLAS-BINARY-WIRE-FORMAT.md`) | DUUMBI: JSON-LD-branded, but `@context` is never actually parsed or IRI-expanded anywhere in the codebase (confirmed by exhaustive grep) -- field access is hardcoded string literals. AGENTS.md's own claim of a "json-ld crate" dependency is false per Cargo.lock. | **Explicit judgment: reject adopting JSON-LD.** DUUMBI gets zero real standards/interop benefit from its own JSON-LD surface, so there is nothing to import; its actual semantics (closed Op/edge enums) are exactly as closed as Atlas's typed records already are. This resolves in favor of Atlas's existing binary design. |
| Contract/behavior verification | `CodingAdmission` (repo-manifest eligibility gate: schema/system_kind/path/boolean flags) -- not a per-function semantic verifier | Sigil: real per-behavior static analysis (pure/no_alloc/writes_output guarantees checked against CFG, honestly self-labeled "structural" not a full referential-transparency proof) plus declared-hash-vs-recomputed-hash consistency. DUUMBI: mutate -> reparse -> rebuild -> revalidate -> write-iff-valid discipline (confirmed real in both CLI and MCP paths) | Genuine gap: Atlas has no current analog to Sigil's per-behavior CFG-based guarantee checking. Not elevated to a blueprint revision this census (no active Atlas capability it would unblock yet -- CONTROL_FLOW is only now materializing in R4.6), but worth recensus once Atlas's own CFG/constraint machinery is real. |
| Self-modification / evolution admission | No analog exists in Atlas | IRIS: correctness-gated (100% trace match) admission is real; the *performance*-gate term is a concrete stub (`performance_gate.iris` hardcodes the slowdown comparison to a constant, never passing the candidate's actual measured timing) | REFERENCE_ONLY -- no current Atlas self-modification loop to compare against or gate; noted as a future recensus trigger if Atlas ever builds one. |
| Formal verification / proof kernel | No analog exists in Atlas | IRIS: a genuinely real Lean 4 proof kernel (2,941 lines, zero `sorry` in the 20 core inference rules, 7 `sorry` remain in metatheory, explicitly flagged by IRIS's own authors as pending a tactic port) | REFERENCE_ONLY -- strongest single mechanism found across all five donors, but nothing in current Atlas scope to attach it to. |

## Discoveries (mechanism-scoped dispositions)

| Discovery | Provider | Disposition |
|---|---|---|
| SemanticGraph node/edge/identity model | IRIS (own `.iris` code; canonical Rust impl absent from this clone) | REFERENCE_ONLY |
| BLAKE3-derived content addressing (non-standard, truncated derivative) | IRIS | REFERENCE_ONLY (already covered by Atlas's separately-admitted `blake3` donor at full fidelity; IRIS's own code is a weaker derivative, undercutting not strengthening any case for it) |
| Lean 4 proof kernel | IRIS | REFERENCE_ONLY |
| Self-modification correctness/performance gate (performance term stubbed) | IRIS | REFERENCE_ONLY |
| Evolution engine (NSGA-II/lexicase/novelty/MAP-Elites) | IRIS | REFERENCE_ONLY |
| REQUIRES@hash dependency pinning + whole-graph stale/cycle/transitive-purity validation | Sigil (own code) | ABSORB_LATER / REFERENCE_ONLY |
| 32-bit-truncated-SHA-256 hash primitive | Sigil (via RustCrypto `sha2`) | REJECT (weaker than Atlas's existing FNV-1a-64 and the queued full-fidelity `blake3` donor) |
| Structural (not semantic) per-behavior guarantee checking, honestly self-labeled | Sigil (own code) | REFERENCE_ONLY |
| Real inkwell/llvm-sys LLVM 18 FFI, in-process object emission, external-linker-only final step | Sigil (own code + `inkwell`/`llvm-sys` dependencies) | REFERENCE_ONLY |
| JSON-LD-branded graph representation (never actually parsed as JSON-LD) | DUUMBI (own code) | REJECT |
| Two-tier node identity (path-based primary + integrity-only secondary hash) | DUUMBI (own code) | REFERENCE_ONLY |
| Mutate -> reparse -> rebuild -> revalidate -> write-iff-valid discipline | DUUMBI (own code) | REFERENCE_ONLY |
| Cranelift confirmed viable as an external native lowering target (single flat graph->Cranelift, no IR staging) | Cranelift (dependency) + DUUMBI (own glue code) | EXTERNAL_BOUNDARY |
| AI-driven mutation / intent pipeline, real end-to-end (not stub) | DUUMBI (own code) | REFERENCE_ONLY |
| `graph_query` MCP tool is a linear filter scan, not a query language | DUUMBI (own code) | REJECT as a query-engine model |
| Thin-middle-end doctrine: "zero semantic dialects below the witness boundary, and it stays zero," with a named checkable round-trip law | Composer (own code + docs) | ABSORB_LATER (relevant to `compiler/mir`, `compiler/lir` once those exist) |
| Witness/Pattern/Element layered-visibility enforcement via language module privacy | Composer (own code) | REFERENCE_ONLY |
| Program Hypergraph (PHG) / hyperedge extension for N-ary relationships | Composer (documented; code presence unverified by this census) | ABSORB_LATER, see Blueprint Revision Candidates below |
| PSG schema/identity/construction/symbol-correlation/reachability/enrichment-saturation/coeffect/escape analysis | clef (own code, per the reconciled attribution above) | EXTERNAL_BOUNDARY (real and substantial, but presumes full closed-world graph residence for a self-owned language -- doesn't fit Rust's `use`/crate/trait resolution surface Atlas actually needs to handle) |
| Real from-scratch Hindley-Milner + dimensional-analysis + SRTP type inference | clef (own code) | REFERENCE_ONLY (see CALL-dispatch row above -- proves the pattern possible, not a bounded path for Atlas's Rust lane) |
| Obligation-ledger pattern (typed obligations as graph citizens, explicit discharge state) | clef (own code) | REFERENCE_ONLY / validates Atlas's existing `EpistemicStatus` design choice, nothing to import |
| Doc/code drift-gate CI lint | clef (process, not a code mechanism) | REFERENCE_ONLY, out of code-mechanism scope |

## Blueprint Revision Candidates

**One candidate surfaced, deliberately NOT elevated to a `BlueprintRevisionDecision` this census:**

Composer's own design docs argue, with a specific worked example (a joint multi-participant FFI-boundary lifetime claim across declaration/arguments/owner/ABI contract), that decomposing an N-ary semantic relationship into pairwise edges can lose information -- each pairwise edge individually holds while the joint claim fails. This is a real, evidence-backed argument that `UNIVERSAL-GRAPH-CONTRACT.md` may eventually need to account for hyperedges/N-ary relations. It is recorded here as **worth evaluating when Atlas's own graph model next needs to represent a genuinely N-ary relationship**, not as a proven necessity today: it is a design document's argument on the donor side, corroborated only by Composer's own docs (not independently benchmarked or falsified by either censusing agent), and Atlas has no current load-bearing capability this would unblock. Per `.atlas/contracts/BLUEPRINT-EVOLUTION.md`, elevating this to a real `BlueprintRevisionDecision` requires deep census of the actual PHG implementation (unverified in code by this census), a concrete Atlas use case that pairwise edges demonstrably cannot represent, and a falsification/benchmark step -- none of which exist yet. **Disposition: noted, deferred, not selected.**

No other candidate across any of the five donors rose to blueprint-revision level. Every other strong mechanism found (real LLVM/MLIR lowering, real type inference, real proof kernels, real per-behavior contract checking) was evaluated and explicitly rejected as a blueprint-revision trigger because it either presumes infrastructure Atlas's own contracts deliberately forecloses (full language ownership, a Rust type checker), is already covered at equal-or-greater fidelity by an existing admitted donor (BLAKE3) or existing Atlas primitive (FNV-1a-64 identity), or has no current Atlas capability to attach to yet (CONTROL_FLOW/DATA_FLOW/STATE/EFFECT/OWNERSHIP/CONCURRENCY/PERSISTENCE remain UNSUPPORTED in Atlas as of this census).

## Recensus Requirements

- Recensus IRIS's control-flow/data-flow/effect node-kind taxonomy once Atlas's own CONTROL_FLOW (R4.6, in progress) / DATA_FLOW / EFFECT dimensions go from UNSUPPORTED to real.
- Recensus Sigil's per-behavior CFG-based guarantee checking once Atlas's own CFG (R4.6) and constraint machinery mature far enough to make a concrete comparison possible.
- Recensus Composer's PHG/hyperedge extension directly in code (not via Composer's documentation of it) if it becomes relevant to an active Atlas graph-model question.
- Recensus DUUMBI's AI-mutation orchestration shape (`src/agents/orchestrator.rs`, `src/intent/execute.rs`) line-by-line if a deeper design reference is later wanted.
- Recensus clef's pin before any future absorption action, given confirmed history-rewrite practice upstream.
- Reconcile Composer's PSG/coeffect/enrichment attribution against clef's actual source (this doc's reconciliation is sourced from Composer's *documentation* of clef, cross-checked against clef's independent census -- both agents agree, but neither did a joint line-by-line diff) before any `ABSORB_NOW`/`ABSORB_LATER` disposition changes to a PSG/coeffect mechanism.
- Verify hgahub/duumbi's fork/parent status via the GitHub API once network access allows it (blocked by this sandbox's proxy during this census; only in-repo content evidence was used).

## Decision

- All five lane donors are `DEEP_CENSUS_ACTIVE` (IRIS, Sigil) or `COARSE_CENSUSED_PLUS_TARGETED_DEEP_CENSUS` (DUUMBI, Composer, clef) as of this census.
- Runtime dependency status for all five: `REFERENCE_ONLY` (or `NOT_APPLICABLE` for Composer, `EXTERNAL_BOUNDARY` for the Cranelift/mlir-opt/cvc5 toolchain edges) -- no donor here is a build/runtime dependency of Atlas.
- Zero mechanisms across all five donors met the evidence bar for `ABSORB_NOW`.
- No `BlueprintRevisionDecision` opened this census; one candidate (hyperedges/N-ary graph relations) explicitly deferred with reasoning, not silently dropped.
- No W-wave assignment changed in `.atlas/roadmap/DONOR-ABSORPTION-PLAN.toml` -- none of these donors reached the bar that would pull a wave forward (`pull_forward_requires = "ABSORB_NOW"` is the existing gate for `LATER_DEFAULT`-maturity waves, and no mechanism here reached `ABSORB_NOW`).
