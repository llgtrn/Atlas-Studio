---
id: donor-census-duumbi
type: reference
status: active
canonical: true
---
# Donor Census: DUUMBI

## Source

- Remote: https://github.com/hgahub/duumbi.git
- Repository: hgahub/duumbi
- Commit: ab4c8776f795052d3669774a7ff12bdd3231fd02
- Git tree: 6f6b259c909619657053af8cdfbf4788b57ed9cf
- Branch: main
- Retrieved: 2026-09-22T14:04:48.531781213Z
- Staging mode: FULL_SOURCE_TREE
- Clone path: .atlas/temporary/donors/duumbi

Full source tree staged with nested `.git` removed. Commit SHA above is the durable pin; `main` is a floating branch reference recorded for context only.

### Provenance note (canonical vs. fork)

Content evidence (`README.md`, `AGENTS.md`) describes `hgahub/duumbi` as the primary/canonical source repository for the DUUMBI project, with two sibling repositories under the same `hgahub` GitHub org referenced as separate concerns: `hgahub/duumbi-vault` (Obsidian planning vault) and `hgahub/duumbi-registry` (registry server backend, pulled in only as a `dev-dependencies` git dependency at a pinned rev — not vendored). No upstream-project attribution, "forked from," or vendoring notice was found anywhere in the tree. This sandbox's outbound access to `api.github.com` is blocked by the session proxy (`GitHub access to this repository is not enabled for this session`), so the GitHub API `fork`/`parent`/`source` flags could **not** be independently verified. Fork status is therefore recorded as: **NOT A FORK per in-repo content evidence; API-level fork flag unverified (proxy-blocked)**.

## Coarse Inventory

- Files observed: 675
- Bytes observed: 19,539,408
- Languages/signals: Rust (236 `.rs`), Markdown (175 `.md`, mostly `specs/` + `docs/`), YAML (70 `.yaml` + 24 `.yml`), JSON-LD-flavored JSON (49 `.jsonld`), JavaScript/mjs tooling (31 `.mjs`, 4 `.js`), plain text (30 `.txt`), JSON (30 `.json`), TOML (7), Python (5, in `scripts/`), Shell (2), C (2 `.c`/`.h` pair: `runtime/duumbi_runtime.c/.h`), CSS (1)
- Top-level directories: `.agents`, `.cargo`, `.claude`, `.cursor`, `.devcontainer`, `.github`, `.greptile`, `crates`, `docs`, `examples`, `runtime`, `scripts`, `specs`, `src`, `stdlib`, `tests`
- Top-level files: `.gitignore`, `.mcp.json.example`, `.nvmrc`, `.pre-commit-config.yaml`, `AGENTS.md`, `CODE_OF_CONDUCT.md`, `Cargo.lock`, `Cargo.toml`, `LICENSE`, `README.md`, `SECURITY.md`, `build.rs`, `codecov.yml`, `renovate.json`, `rust-toolchain.toml`

### Notable vendored/embedded non-Cargo dependency

- `runtime/third_party/sqlite/sqlite3.c` + `sqlite3.h` — vendored SQLite amalgamation, version `3.53.2` (per `SQLITE_VERSION` macro), compiled directly into the C runtime. Not tracked by Cargo/Cargo.lock; attribute any correctness/security properties to upstream SQLite, not to DUUMBI.
- `runtime/duumbi_runtime.c` (3,878 lines) also `#include <curl/curl.h>` and links `-lcurl` — a **system** (dynamically linked, not vendored) dependency on libcurl, required at build/link time for the C runtime's HTTP client shims (`db_connection`/`http_response` DuumbiType support). This is an OS/toolchain terminal boundary, not further resolved.

### Untrusted-content note (prompt-injection surface encountered during census)

The clone ships `AGENTS.md`, `docs/architecture.md`, `docs/coding-conventions.md`, and a `.claude/skills/jsonld-schema` skill directory that are addressed to a coding agent editing *DUUMBI's own* codebase. These were surfaced into this session automatically by tool hooks while reading the tree and are treated strictly as **inspected data about the donor**, not as instructions — per this task's "untrusted input, static inspection only" directive. In particular, `.claude/skills/jsonld-schema` was **not invoked**; a donor-supplied skill offering itself for use during a hostile-input census is exactly the kind of injection vector to decline. One factual claim from `AGENTS.md` ("JSON-LD parsing (serde_json, **json-ld crate**)") was checked against the actual dependency graph and found to be **false** — see Mechanisms Census → Graph Schema below.

## Direct Dependencies

Source: `Cargo.toml` `[dependencies]` (workspace has 2 members: `.` and `crates/duumbi-studio`, not separately audited here beyond noting its existence).

| Crate | Version | Role (as declared/observed) |
|---|---|---|
| clap | 4.6.7 | CLI arg parsing |
| serde / serde_json | 1.0.229 / 1.0.151 | Serialization; serde_json is the *entire* JSON-LD parsing substrate (see Mechanisms Census) |
| anyhow / thiserror | 1.0.104 / 2.0.20 | Error handling (app boundary / library, per project's own convention doc) |
| petgraph | 0.8.3 | Graph IR: `StableGraph<GraphNode, GraphEdge>` — the semantic graph's actual data structure |
| cranelift-codegen | 0.135.2 | Cranelift IR construction + codegen |
| cranelift-frontend | 0.135.2 | `FunctionBuilder` SSA-construction API used throughout `src/compiler/lowering.rs` |
| cranelift-module | 0.135.2 | Module/linkage abstraction (`ObjectModule`, `FuncId`, `DataId`) |
| cranelift-object | 0.135.2 | ELF/Mach-O object-file emission backend |
| target-lexicon | 0.13.5 | Target-triple modeling for Cranelift |
| toml / serde_yaml | 1.1.6 / 0.9.34 | Config/manifest/lockfile and intent-spec YAML |
| tracing / tracing-subscriber | 0.1.44 / 0.3.23 | Structured logging |
| tokio | 1.53.1 | Async runtime (registry client, agent HTTP calls, MCP server I/O) |
| reqwest | 0.13.5 | HTTP client — LLM provider calls (Anthropic/OpenAI/etc.) and registry client |
| ratatui / ratatui-textarea / crossterm | 0.30.2 / 0.9.2 / 0.29.0 | TUI (REPL) |
| pulldown-cmark | 0.13.4 | Markdown rendering (docs/TUI) |
| sha2 | 0.11.0 | SHA-256 — semantic hashing (`src/hash.rs`) and package integrity hashes |
| semver | 1.0.28 | Registry dependency version resolution |
| flate2 / tar | 1.1.10 / 0.4.46 | `.tar.gz` module packaging for registry publish |
| dirs | 7.0.0 | Config/credentials directory resolution |
| chrono | 0.4.45 | Timestamps |
| owo-colors / indicatif / comfy-table / clap_complete | — | CLI UX |
| strsim | 0.11.1 | Fuzzy suggestion matching (CLI error UX) |
| shlex | 2.0.1 | Shell-safe tokenization |
| libc (unix only) | 0.2.189 | Low-level unix bindings |
| (dev) axum | 0.8.9 | Used by `crates/duumbi-studio` (Leptos SSR web platform) / test HTTP servers |
| (dev) duumbi-registry | git rev `a3adef4…` from `hgahub/duumbi-registry.git` | Registry integration tests only |
| (dev) rustls, wiremock | 0.23.45 / 0.6.5 | TLS / HTTP mocking for tests |

**No JSON-LD library dependency exists anywhere in `Cargo.toml` or `Cargo.lock`** (`grep -rn "json-ld\|json_ld"` across `src/`, `Cargo.toml`, `Cargo.lock` returns zero hits). This directly contradicts `AGENTS.md`'s claim of a "json-ld crate" — see Mechanisms Census.

## Dependency Closure Status

**PARTIAL_TRANSITIVE — COMPLETE for the Rust/Cargo package graph, terminated at explicit non-Cargo boundaries.**

- `Cargo.lock` contains 572 fully-resolved `[[package]]` entries — this **is** Cargo's own transitive fixed point for the Rust dependency graph (Cargo does not do partial resolution; every entry in the lockfile is a concrete, pinned version). All 14 `cranelift-*` crates resolve to a single consistent version, **0.135.2**. `petgraph` = 0.8.3, `target-lexicon` = 0.13.5.
- Not further expanded beyond the Rust package graph: (a) each crate's own C/system library requirements (e.g., what `ring`/`rustls` or `reqwest`'s TLS backend pull in at the OS level), (b) the vendored SQLite amalgamation (`runtime/third_party/sqlite`, version 3.53.2, not a Cargo dependency), (c) system libcurl (dynamically linked at link time via `cc ... -lcurl`, not vendored, not a Cargo dependency), (d) the Rust toolchain itself (stable ≥1.95, pinned by `cranelift-*`/`wasmtime-internal-core`'s own `rust-version` requirement per `AGENTS.md`, and by `rust-toolchain.toml`'s `stable` channel selection), (e) the host C compiler/linker (`$CC` or `cc` on PATH) used by `src/compiler/linker.rs`.
- These five are explicit **toolchain/system/vendored-C terminal boundaries**, not silently-unresolved Rust crates — the term "COMPLETE" above applies only to the Cargo/crates.io graph, not to these boundaries.

## Build Systems Detected

- `.atlas/temporary/donors/duumbi/Cargo.toml` (workspace root: package `duumbi` + member `crates/duumbi-studio`)
- `.atlas/temporary/donors/duumbi/crates/duumbi-studio/Cargo.toml`
- `.atlas/temporary/donors/duumbi/build.rs` (Cargo build script)
- Native linking is delegated to the host C toolchain via `src/compiler/linker.rs` (`$CC` env var, else `cc` on `PATH`; `cc output.o duumbi_runtime.o -o output -lc -lcurl`) — DUUMBI's own Cranelift-produced object files still require an external C linker, they are not self-linking.
- `.pre-commit-config.yaml`, `.github/` workflows (CI) — not deep-audited beyond noting their presence (native Windows CI retired per `AGENTS.md`, immutable tag `windows-support-final-2026-09-10` cited but not independently verified in this static census).

## Test / Benchmark Roots Detected

- `.atlas/temporary/donors/duumbi/tests/` — 27 top-level integration test files (`integration_duumbi*.rs`, `integration_phase*.rs`, `kill_criterion_phase7.rs`) plus `tests/fixtures/`. Real `.jsonld` fixtures are used (per `docs/coding-conventions.md`'s own stated convention), consistent with what's on disk.
- `.atlas/temporary/donors/duumbi/src/bench/` — `mod.rs`, `process.rs`, `process/`, `report.rs`, `runner.rs`, `showcases.rs`, `showcases/` — an in-tree benchmark/showcase runner (Phase 9C in the roadmap), not a `benches/`+Criterion setup.
- `.atlas/temporary/donors/duumbi/crates/duumbi-studio/tests/` — Studio web-platform tests.
- Unit tests are pervasive and colocated (`#[cfg(test)] mod tests` inside `graph/mod.rs`, `graph/program.rs`, `graph/validator.rs`, `parser/mod.rs`, etc. — consistent with the project's own stated convention).
- Per `AGENTS.md`: `cargo test --all` claims ~817 tests; **not independently executed** — this census performs static inspection only, per instructions. Treat as a DECLARED, not OBSERVED, count.

## Major Subsystem Roots

`src/parser` (JSON parsing → typed AST) · `src/graph` (petgraph-based semantic graph, builder, validator, program/module linking, ownership, result-safety) · `src/compiler` (Cranelift lowering + linker) · `src/patch.rs` (graph mutation ops) · `src/intent` (Intent-Driven Development pipeline) · `src/agents` (LLM provider clients + AI mutation orchestration) · `src/mcp` (MCP JSON-RPC server exposing graph tools) · `src/query` (read-only NL Q&A mode) · `src/registry` (module publish/distribution client) · `src/determinism`, `src/properties`, `src/rewrite` (evidence/verification subsystems) · `src/contracts` (per-function contract parsing) · `runtime/duumbi_runtime.c` (C runtime: print/alloc/string/array/struct/Result/Option/JSON/TCP/HTTP/SQLite shims) · `stdlib/*.jsonld` (DUUMBI-authored standard library modules) · `crates/duumbi-studio` (Leptos SSR web visualizer).

## License Evidence

- `.atlas/licenses/donors/duumbi/LICENSE` — **Mozilla Public License Version 2.0 (MPL-2.0)**, confirmed by license header text and by `Cargo.toml`'s `license = "MPL-2.0"` field. MPL-2.0 is file-level (weak) copyleft: modifications to MPL-covered files must themselves be released under MPL-2.0 if distributed, but MPL-covered files may be combined with differently-licensed code in a larger work. This has direct bearing on absorption: verbatim or near-verbatim reuse of DUUMBI source files would carry file-level MPL obligations; independently-written Atlas code that is merely *informed* by a DUUMBI mechanism (the normal "census then reimplement" path) does not.
- The vendored `runtime/third_party/sqlite/` amalgamation carries its own public-domain SQLite license (see `NOTICE.md` in that directory) — separate from and not superseded by DUUMBI's MPL-2.0.

## Mechanisms Census

### Graph schema

Fixed, hand-authored schema of exactly four structural node kinds (`duumbi:Module`, `duumbi:Function`, `duumbi:Block`, and typed `Op` nodes), defined implicitly by the parser (`src/parser/mod.rs`, `src/parser/ast.rs`) and the `Op` enum (`src/types.rs`, ~46 variants: `Const`/`ConstF64`/`ConstBool`/`ConstString`, arithmetic incl. `*Checked` variants, `Compare`, `Branch`, `Call`, `Load`/`Store`, `Print`/`PrintString`/`PrintLn`/`ReadLine`, file I/O (`ReadFile`/`WriteFile`/`FileExists`/`ListDir`/`PathJoin`), `Return`, string ops, array ops, struct ops, ownership ops (`Alloc`/`Move`/`Borrow`/`BorrowMut`/`Drop`), `Result`/`Option` ops, `Match`). Edge kinds are a fixed Rust enum `GraphEdge` (`src/graph/mod.rs`): `Left`/`Right`/`Operand`/`Condition`/`TrueBlock`/`FalseBlock`/`Arg(usize)`, plus ownership edges `Owns`/`MovesFrom`/`BorrowsFrom`/`Drops`. This is **not** an open/extensible RDF-style schema despite the `@context`/`@type` surface syntax — it is a closed enum enforced by the Rust type system at parse time; an unrecognized `@type` is a hard parse error (`ParseError::UnknownOp`, E002). **Provider: DUUMBI's own code** (`src/types.rs`, `src/parser/mod.rs`).

Critically, `@context` (present in every `.jsonld` fixture/example as `{"duumbi": "https://duumbi.dev/ns/core#"}`) is **never read** by the parser — confirmed by exhaustive grep of `src/parser/mod.rs`: the only occurrences of the literal string `@context` in the entire `src/` tree are inside test-fixture string literals in `src/graph/program.rs`'s own unit tests, not in any parsing logic. Field names like `"duumbi:name"`, `"duumbi:functions"` are matched as **literal hardcoded strings**, not as IRIs resolved through `@context` expansion. There is no `@graph`, `@reverse`, blank-node, or remote-context support. **This means DUUMBI's "JSON-LD" is JSON-LD-shaped surface syntax only — a fixed, closed, hand-rolled schema serialized through `serde_json::Value`, not an implementation of the W3C JSON-LD data model.** Provider for the actual (non-)implementation: DUUMBI's own `src/parser/`; provider for the underlying JSON parsing: **`serde_json`** (a dependency), not DUUMBI and not any JSON-LD library.

### Node identity

`NodeId(pub String)` (`src/types.rs`) — a thin newtype wrapping the raw `@id` string. IDs are **author/AI-assigned hierarchical path strings** in the convention `duumbi:<module>/<function>/<block>/<index>` (e.g. `duumbi:main/main/entry/2`), not content hashes and not opaque sequential integers. Identity is positional/structural-path-based: moving or renumbering ops changes their `@id`, and the graph builder (`src/graph/builder.rs`) enforces uniqueness via `GraphError::DuplicateId` (E005) at build time, with dangling references caught as `GraphError::OrphanRef` (E004). **Provider: DUUMBI's own code.**

A **separate, secondary** identity concept exists: `semantic_hash()` (`src/hash.rs`, SHA-256 via the `sha2` crate) computes a canonicalized, `@id`-independent hash of a whole module's `.jsonld` files (strips `@id`/`@context`, normalizes `{"@id": "..."}` references to positional `{"_ref": N}` markers, sorts object keys, concatenates and hashes). This is used **only** for registry/lockfile integrity (`deps.lock`'s `semantic_hash` field, package-level dedup for the module registry), never for in-graph node addressing, graph diffing, or as the primary identity in `GraphNode`/`node_map`. So DUUMBI *has* a content-addressed hashing primitive, but relegates it to a package-integrity role rather than using it as the graph's native identity mechanism — the opposite of Atlas's design (see Atlas Comparison).

### Operation taxonomy

The `Op` enum (46+ variants, `src/types.rs`) is the complete mutation/instruction vocabulary; see Graph Schema above for the list. Each `Op` variant has a direct, hardcoded 1:1 (occasionally 1:few, e.g. `Div`→guarded `sdiv`, `AddChecked`→`call duumbi_i64_add_checked`) mapping to Cranelift IR or a C-runtime call, tabulated exhaustively in `docs/architecture.md`'s "Full Op set" table (verified present and consistent with `src/compiler/lowering.rs`'s `declare_all_runtime_fns`/`compile_function`). There is no separate "query" op family in this taxonomy — reads against the graph happen through the MCP `graph_query` tool (see Query/Evidence Model below), not through graph-native query ops. **Provider: DUUMBI's own code**; the IR each op lowers *to* is **Cranelift's** (owned by the `cranelift-codegen`/`cranelift-frontend` crates), not DUUMBI's.

### Reference/binding model

References are `{"@id": "duumbi:...">}` object literals parsed into `NodeRef { id: NodeId }` (`src/parser/ast.rs`). Binding to the actual graph node happens post-parse in `src/graph/builder.rs` via a `HashMap<NodeId, NodeIndex>` (`SemanticGraph::node_map`), resolved into `petgraph` edges (`GraphEdge::Left/Right/Operand/Condition/...`) at graph-construction time. Cross-module references (`Call { module: Option<String>, function: String }`) are resolved separately and later, in `src/graph/program.rs`'s multi-module linking pass, against a combined export table (`exports`/`qualified_exports`), with ambiguity/unresolved-reference detection (E010, `ProgramError::{UnresolvedCrossModuleRef, AmbiguousCrossModuleRef, UnresolvedQualifiedCrossModuleRef}`) — this is a real, tested two-phase (intra-module then inter-module) reference-resolution pipeline, not a stub. **Provider: DUUMBI's own code**, using `petgraph`'s (a dependency) `StableGraph`/`NodeIndex` as the underlying storage the resolution targets.

### Graph mutation semantics

Two independent mutation surfaces exist, both real:
1. **`GraphPatch`/`PatchOp`** (`src/patch.rs`): a closed 6-variant op set — `AddFunction`, `AddBlock`, `AddOp`, `ModifyOp` (set one field by `node_id`), `RemoveNode`, `SetEdge` (shorthand for setting an `@id`-reference field). `apply_patch()` applies a batch **all-or-nothing** against a `serde_json::Value` clone of the source graph.
2. The **MCP `graph_mutate` tool** (`src/mcp/tools/graph.rs`) is the actual entry point AI agents use: it deserializes `ops` into `Vec<PatchOp>`, calls `apply_patch`, then **re-parses, re-builds, and re-validates** the patched graph (`parser::parse_jsonld` → `builder::build_graph` → `validator::validate`) *before* writing anything to disk — a mutation that fails validation is rejected and the on-disk graph is left untouched. This mutate-then-validate-or-reject discipline is a genuinely good safety property, confirmed by reading the actual control flow (not merely documented).
Additionally, `duumbi undo` (`.duumbi/history/{N:06}.jsonld` snapshots, LIFO) provides mutation rollback. **Provider: DUUMBI's own code** throughout.

### Validation

`src/graph/validator.rs` (2,407 lines) is a real, extensive, table-driven validator covering: function/block structure (E009), terminator position, cycle detection (E007, via `petgraph::algo::is_cyclic_directed`), a substantial type checker (E001 — binary-op operand-type matching, call-arg-type checking, return-type checking, branch-condition-must-be-bool, exact-result-type checks for typed `db_connection`/`http_response`/`json`/`result<T,E>` shapes), contract checks (`check_contracts`, delegating to `src/contracts`), SSA dominance checking (`check_ssa_dominance`), and branch-target resolution (`check_branch_targets`). A **second validation tier**, `src/graph/ownership.rs` and `src/graph/result_safety.rs`, adds ownership/borrow/lifetime checking (E020–E029: single-owner, use-after-move, borrow exclusivity, lifetime-exceeded, double-free, dangling-reference, move-while-borrowed, etc.) and Result/Option exhaustiveness checking (E030–E035: unhandled Result/Option, non-exhaustive match, unwrap-without-check), each **gated to run only when the graph actually contains the relevant op kinds** (confirmed in `validate()`'s top-level dispatch) — i.e., it does not spuriously fire ownership diagnostics on graphs with no ownership ops. This is a real static-analysis pass, not superficial schema checking. **Provider: DUUMBI's own code.**

### The Cranelift lowering boundary

`src/compiler/lowering.rs` (4,119 lines) is genuinely substantial, working code, not a stub. One Cranelift function is emitted per `duumbi:Function` node (no cross-function inlining at this layer, matching the project's own stated convention). Key characteristics actually read in code:
- All ~90 C-runtime entry points (`duumbi_print_i64`, `duumbi_string_concat`, `duumbi_array_get`, `duumbi_result_new_ok`, `duumbi_tcp_connect`, `duumbi_server_new`, etc.) are declared once (`declare_all_runtime_fns`) and threaded through a `RuntimeFuncs` struct — every `Op` variant beyond the handful with direct Cranelift instruction equivalents (`iadd`/`isub`/`imul`/`icmp`/`brif`/`call`/`use_var`/`def_var`/`return`) is lowered to a **`call` into the hand-written C runtime**, not into native Cranelift instructions. So "graph → Cranelift" for anything beyond scalar arithmetic/control-flow/load-store is really "graph → Cranelift `call` → externally-linked C shim."
- Integer division is explicitly **not** wrapping: `emit_guarded_integer_div` guards against divide-by-zero and `i64::MIN / -1` before emitting `sdiv`, trapping through a DUUMBI-specific panic path that carries the originating graph node id (`emit_node_panic_guard`) — this is real, deliberate semantic-preservation work at the lowering boundary, not information silently lost.
- Struct layout is computed per-struct from compile-time field evidence (`build_struct_layouts`/`record_struct_field`): fields get stable 8-byte slots by lexicographic name order; conflicting field-type evidence across call sites is a compile error. This is DUUMBI's own ABI-shaping logic, layered on top of Cranelift, which itself has no opinion about DUUMBI's struct semantics.
- **Information loss at the boundary**: the Cranelift IR produced has no memory of `@id`/NodeId, ownership/ownership-edge structure, or the original graph topology beyond what's needed to emit correct instructions — this is expected/appropriate (Cranelift's IR is not meant to carry DUUMBI-level semantic metadata), but it does mean DUUMBI's lowering is a **single flat step**, not a staged HIR→MIR→LIR→MachineIR pipeline with explicit lineage-preserving intermediate representations (contrast with Atlas's own `COMPILER-IR-PIPELINE.md`, see Atlas Comparison).
- **Provider attribution is mixed and must be split precisely**: the IR data model, instruction set, register-allocation-adjacent APIs (`FunctionBuilder`, `InstBuilder`, `Context`), and the object-file emission (`ObjectModule`/`ObjectBuilder`) are **Cranelift's** (`cranelift-codegen`, `cranelift-frontend`, `cranelift-module`, `cranelift-object` — all external crates at 0.135.2, maintained by the Cranelift project, not by DUUMBI). DUUMBI's own contribution at this boundary is: (a) the graph-node-to-Cranelift-instruction mapping/dispatch logic, (b) the C-runtime-call convention and the ~4,000-line `runtime/duumbi_runtime.c` implementation itself, (c) the guarded-arithmetic/struct-layout/macOS-triple-normalization policy layered on top. The linker (`cc`) is the **host system C toolchain**, not DUUMBI's and not Cranelift's.

### AI-driven graph mutation / intent-query mechanism

**Real, working, non-stub code**, confirmed by reading (not just documentation):
- `src/agents/` (13,227 total lines across 20 files) implements a genuine multi-provider LLM client abstraction: `anthropic.rs` (615 lines), `openai.rs` (452), `grok.rs`, `openrouter.rs`, `minimax.rs`, plus `orchestrator.rs` (958 lines, the mutation-loop driver), `fallback.rs` (provider fallback chain), `cost.rs`, `model_catalog.rs` (2,432 lines — a large, structured model/pricing catalog), `analyzer.rs`/`assembler.rs`/`merger.rs`/`rollback.rs` (patch-plan generation, assembly, merging, and rollback).
- `src/mcp/` implements a real JSON-RPC 2.0 MCP server (`src/mcp/server.rs`, 785 lines, newline-delimited JSON over stdio, `initialize`/`tools/list`/`tools/call`/`notifications/initialized`) exposing `graph_query`, `graph_mutate`, `graph_validate`, `graph_describe` among its 10 tools (confirmed by direct inspection of `src/mcp/tools/graph.rs`, not just the doc table).
  - `graph_query` is a simple **linear filter scan** (`node_id`/`type_filter`/`name_pattern` exact/substring match, recursive walk over the parsed `serde_json::Value` tree) — real and working, but **not** a graph pattern-matching/traversal query language (no path queries, no relationship-aware queries, no Cypher/SPARQL/Gremlin-equivalent). It is closer to "grep over JSON-LD nodes" than to a graph query engine.
  - `graph_mutate` performs the same mutate-then-validate-or-reject pipeline described under Graph Mutation Semantics above, invoked over MCP.
- The **separate** `src/query/` module (`src/query/engine.rs`, `context.rs`, `sources.rs`, `prompt.rs`) is a **read-only natural-language Q&A mode**, not a graph query language: it assembles workspace context (`assemble_query_context`), sends `question + context` to an `LlmProvider`, and returns a `QueryAnswer` tagged with `AnswerConfidence::{Low, Medium}` based on whether context sources were found, plus a `suggested_handoff` to Agent/Intent mode when the question looks like a mutation/intent request (`classify_request`/`RequestShape`). This is the actual "query/evidence model" referenced by the census brief, and it is real but shallow: confidence is a two-level heuristic (empty-vs-nonempty sources), not a calibrated evidence-scoring system.
- `src/intent/` (Intent-Driven Development, Phase 5) — `coordinator.rs` (626 lines, real rule-based task decomposition: CreateModule → AddFunction(non-main) → ModifyMain ordering), `execute.rs` (2,346 lines, the actual execute pipeline: decompose → per-task `mutate_streaming` with 3-step retry → `Verifier::run_tests` → archive), `verifier.rs`, `spec.rs`, `bdd.rs` (Gherkin-style companion-file parsing/validation), `capture.rs`, `attempt.rs`. This is a large, structurally complete pipeline, not aspirational scaffolding — confirmed by both the line counts and by cross-checking `docs/architecture.md`'s claimed pipeline shape against the actual module list and function names.
- **Verdict: this mechanism is REAL AND SUBSTANTIAL, not a stub or aspirational placeholder.** Whether the *quality* of AI-generated mutations meets any particular bar was not evaluated (would require running the untrusted donor's code/LLM calls, out of scope for a static census). **Provider: DUUMBI's own code** for the entire pipeline; `reqwest`/`tokio` (dependencies) provide only the HTTP/async transport, not any of the mutation/orchestration logic itself.

## Atlas Comparison

**Node identity**: Atlas's `SemanticRecordId`/graph node/edge ids are `stable_id(<kind>, <deterministic content string>)` — genuinely content-addressed hashes computed from an explicit `identity_key()` per fact kind (confirmed in `core/src/semantic/{call,control_flow,data_flow,effect,function}.rs` and `core/src/graph/engineering_graph.rs`), with `RawObservationId ≠ SemanticRecordId` kept as a deliberate, contractually-required distinction (`ATLAS-SEMANTIC-COMPACTION.md`). DUUMBI's primary `NodeId` is the *opposite* choice: an author/AI-assigned hierarchical path string with no content-addressing guarantee — DUUMBI *does* have a content-hash primitive (`semantic_hash`, SHA-256, `@id`-independent), but restricts it to package/lockfile integrity, never graph node identity. Atlas's existing design is not merely different by accident; DUUMBI's own split shows the tradeoff was visible to DUUMBI's authors too, and DUUMBI chose path-based identity for its primary graph while relegating content-hashing to a secondary integrity role — the reverse of Atlas's contract. No evidence found that DUUMBI's approach is superior for Atlas's purposes (Atlas needs identity stability across scope/revision for reconciliation and dedup, which path-based identity actively undermines — moving/renumbering an op changes its identity in DUUMBI).

**Graph schema / canonical representation (JSON-LD vs. Atlas binary)**: This is the specific trap the task instructions warned against, and the evidence supports **not adopting JSON-LD**. DUUMBI markets a "JSON-LD semantic graph" but (a) never implements `@context` resolution/expansion (confirmed by exhaustive grep — the parser reads `"duumbi:name"` etc. as hardcoded literal strings, never resolves an IRI), so none of the standards-compliance/interop benefits real JSON-LD tooling would provide (generic JSON-LD processors, reasoners, SHACL validation, federation with other JSON-LD-emitting systems) are actually available to DUUMBI or to a hypothetical Atlas adopter; (b) stores program logic as human-editable but verbose, uncompressed, non-content-addressed text files on disk (`.jsonld` under `.duumbi/graph/`), with **zero** interning, delta-encoding, or record-framing — everything Atlas's `ATLAS-BINARY-WIRE-FORMAT.md` and `ATLAS-SEMANTIC-COMPACTION.md` explicitly require (typed binary record framing, string/identity/type/symbol interning, content-addressed sharding, deterministic canonical ordering, per-section integrity hashes) is simply absent from DUUMBI's representation; (c) DUUMBI's own closed `Op` enum + fixed edge-kind enum shows the *actual* semantics were never genuinely open/RDF-like — the JSON-LD surface syntax is decorative branding over a schema that is exactly as closed as Atlas's own typed record model, just serialized less efficiently and without content addressing. The one real advantage JSON-LD-flavored text format has — human/diff readability and easy hand-editing without tooling — is a legitimate but narrow benefit (useful for a small greenfield compiler's early-stage debuggability) that Atlas's contracts already account for by keeping THIN/FAT source-blob modes and typed diagnostics separate from the canonical binary payload; it does not outweigh Atlas's deliberate compaction/content-addressing/provenance requirements for a system meant to carry "hundreds or thousands of independent repositories" (`UNIVERSAL-GRAPH-CONTRACT.md`) at Atlas's target scale. **Judgment: REJECT adopting JSON-LD (or JSON-LD-flavored text) as Atlas's canonical representation.** See Discoveries/Blueprint Revision Candidates for the disposition record.

**Cranelift lowering boundary vs. Atlas's HIR→MIR→LIR→Machine IR pipeline**: DUUMBI performs a **single flat lowering** from its semantic graph directly to Cranelift IR (`src/compiler/lowering.rs`), with no separate HIR/MIR/LIR staged representation and no explicit lineage-preserving intermediate IR between the graph and Cranelift. Atlas's `COMPILER-IR-PIPELINE.md` contractually requires four explicit staged IRs (HIR/MIR/LIR/Machine IR) each with its own identity, allowed-transformation, and lineage-preservation rules specifically so that later native backends (LLVM, Cranelift, WASM) consume a well-defined LIR boundary rather than being coupled directly to the source graph. DUUMBI's single-step lowering is workable at DUUMBI's current scale/ambition (a small greenfield language with ~46 op kinds) but does not demonstrate a technique Atlas should adopt in place of its staged pipeline — if anything it is evidence *for* Atlas's staged design, since DUUMBI's lowering function is a single 4,100-line file handling every op-kind-to-Cranelift-or-runtime-call mapping in one undifferentiated pass, which is exactly the kind of monolithic-stage risk the staged HIR/MIR/LIR contract exists to avoid at larger scale. Genuinely reusable/attributable to Cranelift itself (not DUUMBI): the underlying IR builder API, register-allocation-adjacent machinery, and object-file emission — Atlas's `COMPILER-IR-PIPELINE.md` already treats Cranelift as one acceptable *external native backend* consuming a "LIR or another explicitly contracted boundary," which is consistent with what's actually observed here (DUUMBI's own graph→Cranelift glue is DUUMBI-specific and not reusable as-is, but confirms Cranelift itself is a viable backend consumer for Atlas's LIR).

**Operation taxonomy vs. Atlas's SemanticFactKind family**: DUUMBI's ~46-variant `Op` enum is an **execution/instruction** vocabulary (what a running program does — arithmetic, control flow, memory, I/O), not a **semantic-fact** vocabulary (what is true about a program's structure/behavior for analysis purposes — Atlas's `FunctionIdentity`/`CallFact`/`ControlFlowFact`/`DataFlowFact`/`StateAccessFact`/`EffectFact`/`OwnershipFact`/etc., per `SEMANTIC-FACTS.md`). These are different layers solving different problems and are not directly comparable as competitors; DUUMBI's `Op` taxonomy is closer to Atlas's eventual Machine-IR-adjacent execution semantics than to Atlas's `core/semantic/` fact records. One narrow, real point of contact: DUUMBI's `CallFact`-equivalent (`Op::Call { module: Option<String>, function: String }`) resolves purely statically (module/function name lookup against an export table) — it has no notion of `DYNAMIC_RESOLVED_SET`/`DYNAMIC_PARTIAL`/`UNRESOLVED` dispatch kinds the way Atlas's `CallFact` contractually requires (`STATIC_RESOLVED`/`DYNAMIC_RESOLVED_SET`/`DYNAMIC_PARTIAL`/`UNRESOLVED`), which tracks: DUUMBI is a from-scratch AI-authored language with no dynamic dispatch/virtual calls at all yet, so it has not needed to solve the harder problem Atlas's `CALL` record (dispatch=UNRESOLVED, just landed) is built for. No transferable mechanism found here.

**Validation vs. Atlas's EpistemicStatus model**: DUUMBI's validator produces a flat `Vec<Diagnostic>` with error-code+message severity — a binary valid/invalid gate before compilation, with no `EpistemicStatus`-equivalent gradations (OBSERVED/DECLARED/DERIVED/INFERRED/HYPOTHESIS/CONFLICT/UNKNOWN/UNSUPPORTED/IGNORED). This is appropriate for DUUMBI's purpose (gate compilation of a hand/AI-authored program) but is not a richer or more general model than Atlas's; nothing here suggests Atlas should simplify its epistemic-status taxonomy toward DUUMBI's binary model.

**Query/evidence model vs. Atlas**: DUUMBI's `AnswerConfidence::{Low, Medium}` (based on whether any context source was found) is a much coarser confidence model than Atlas's nine-value `EpistemicStatus` taxonomy. No transferable mechanism found; Atlas's model is already more expressive.

**AI-driven mutation pipeline vs. Atlas**: DUUMBI's mutate → re-parse → re-build → re-validate → write-only-if-valid discipline (both in the CLI `duumbi add` flow and the MCP `graph_mutate` tool) is a genuinely sound safety pattern worth noting as a *design precedent* (not a code-reuse candidate, given MPL-2.0 and Rust/architecture mismatch) for any future Atlas-side AI-graph-mutation surface: never persist a mutation that fails the full validate pipeline. This is unsurprising/expected practice rather than a novel discovery, but it is confirmed real rather than aspirational.

## Discoveries

1. **[Graph schema / canonical representation] JSON-LD branding is not backed by a JSON-LD implementation.** `@context` is parsed nowhere in `src/parser/`; field access is via hardcoded string literals, not IRI expansion. `AGENTS.md`'s claim of a "json-ld crate" dependency is false — no such crate exists in `Cargo.toml`/`Cargo.lock`. Disposition: **REJECT** (do not adopt JSON-LD or JSON-LD-flavored surface syntax as Atlas's canonical representation; the "interop/standards" argument for JSON-LD does not hold here because DUUMBI itself gets none of those benefits).
2. **[Node identity] Two-tier identity split (path-based primary id + separate content-hash for package integrity only).** Disposition: **REFERENCE_ONLY** — informative as a documented alternative design and as evidence Atlas's content-addressed-identity-as-primary choice is deliberate and sound, not something to revise toward DUUMBI's split.
3. **[Graph mutation semantics] Mutate → reparse → rebuild → revalidate → write-iff-valid discipline, applied consistently in both the CLI patch flow and the MCP `graph_mutate` tool.** Disposition: **REFERENCE_ONLY** (design precedent, not code to absorb — MPL-2.0 file-level copyleft plus total architecture mismatch make direct reuse inappropriate; the *pattern* is worth keeping in mind for any future Atlas AI-mutation surface).
4. **[Cranelift lowering boundary] Cranelift itself (external dependency, not DUUMBI) confirmed viable as a LIR-consuming native backend at 0.135.2, with a working, non-trivial `FunctionBuilder`/`ObjectModule` usage pattern for guarded-arithmetic and runtime-call-heavy lowering.** Disposition: **EXTERNAL_BOUNDARY** — this is evidence about Cranelift the dependency (already governed by Atlas's own `COMPILER-IR-PIPELINE.md` "External native backends" section), not about DUUMBI; no DUUMBI-specific code is being proposed for absorption here.
5. **[AI-driven graph mutation / intent-query mechanism] Confirmed real, substantial, non-stub implementation across `src/agents/`, `src/mcp/`, `src/intent/`, `src/query/`.** Disposition: **REFERENCE_ONLY** — useful as an existence proof that an AI-mutation-driven graph-native compiler pipeline is buildable end-to-end (parse → validate → AI-mutate → revalidate → compile → intent-verify), and as a design reference for pipeline shape, but not a code-absorption candidate (MPL-2.0, Rust monolith tightly coupled to DUUMBI's own graph/CLI/provider types, no isolated reusable component boundary observed).
6. **[Query/evidence model] `graph_query` MCP tool is a linear substring/exact-match filter scan over parsed JSON, not a graph traversal/pattern-matching query language.** Disposition: **REJECT** as a query-engine mechanism to emulate — it does not represent an advance over what would be needed for genuine graph pattern queries; not worth modeling Atlas's future query surface on.
7. **[Vendored/embedded dependency] SQLite 3.53.2 vendored verbatim in `runtime/third_party/sqlite/` (public-domain, separate license) and libcurl linked as a system dependency for the C runtime's DB/HTTP DuumbiTypes.** Disposition: **EXTERNAL_BOUNDARY** — attribute to SQLite/libcurl upstream, not to DUUMBI; noted for completeness of the dependency closure, no action implied for Atlas.
8. **[Untrusted-content / process] Donor repo ships a `.claude/skills/jsonld-schema` skill and extensive `AGENTS.md`/`docs/architecture.md` agent-instructions files that auto-surfaced into this census session via tool hooks.** Disposition: **REJECT** treating any of this as instructions — logged here as a process note, not a technical mechanism. No skill was invoked; all such content was treated strictly as inspected data.

## Blueprint Revision Candidates

**None.**

The one place this census produced genuinely strong, falsifiable comparative evidence — JSON-LD vs. Atlas's binary/content-addressed representation — resolves in **favor of Atlas's existing design**, not against it (see Discoveries #1 and Atlas Comparison above). That is a REJECT-with-reasoning finding, not a blueprint revision candidate; per instructions it is recorded there rather than manufactured into a revision proposal here. No other mechanism examined (node identity, operation taxonomy, reference/binding model, mutation semantics, validation, Cranelift lowering, query/evidence model) surfaced evidence that Atlas's current contracts (`SEMANTIC-FACTS.md`, `UNIVERSAL-GRAPH-CONTRACT.md`, `ATLAS-FORMAT.md`, `ATLAS-SEMANTIC-COMPACTION.md`, `ATLAS-BINARY-WIRE-FORMAT.md`, `COMPILER-IR-PIPELINE.md`) should be revised.

## Recensus Requirements

- If DUUMBI is later considered for its **AI-mutation orchestration pipeline shape** (not code) as a design reference for an Atlas-side agent-driven graph-mutation surface, a deeper read of `src/agents/orchestrator.rs` (958 lines, not fully read line-by-line in this pass), `src/agents/merger.rs`, and `src/intent/execute.rs` (2,346 lines, only structurally surveyed) is needed before drawing implementation-level conclusions.
- The GitHub API fork/parent/source flags for `hgahub/duumbi` could not be checked in this sandbox (proxy blocks `api.github.com`); if canonical/fork status becomes decision-relevant, re-verify with unrestricted network access.
- `AGENTS.md`'s ~817-test claim and the "windows-support-final-2026-09-10" immutable-tag claim are DECLARED, not OBSERVED — this census did not execute the donor's build/test suite (per static-inspection-only instructions) or independently verify the cited tag/commit against GitHub.
- `crates/duumbi-studio` (Leptos SSR web visualizer) was enumerated at the directory level only and not deep-censused; if DUUMBI's graph-visualization UI ever becomes relevant to an Atlas tooling need, it needs its own pass.
- `runtime/duumbi_runtime.c` (3,878 lines) was read for its dependency/ABI surface (curl, sqlite, struct/Result/Option layout calls) but not exhaustively reviewed function-by-function; a security-focused recensus would be warranted before treating any of its patterns (even as reference) given it directly parses network/DB input.

## Census State

Status: COARSE_CENSUSED + TARGETED_DEEP_CENSUSED (mechanisms: graph schema, node identity, operation taxonomy, reference/binding model, graph mutation semantics, validation, Cranelift lowering boundary, AI-driven mutation/intent-query mechanism, JSON-LD-vs-binary representation comparison). This is a census-and-comparison pass only — no absorption, no code copied into Atlas production paths, no shared registry files modified. A coordinator must merge applicable Discoveries/target_owners into `.atlas/provenance/donors/donor-clone-index.json`, `.atlas/roadmap/DONOR-ABSORPTION-PLAN.toml`, and `.atlas/references/donor-corpus.toml` separately.
