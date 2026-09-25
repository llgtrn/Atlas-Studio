---
id: donor-census-sigil-lang
type: reference
status: active
canonical: true
---
# Donor Census: Sigil

## Source

- Remote: https://github.com/pzalutski-pixel/sigil-lang.git
- Repository: pzalutski-pixel/sigil-lang
- Commit: 9c7de956d926fba2f9d5c0598b35c57fb6399f7f
- Git tree: 39e4e7de6a449648ba66521e4a082deb7aa13220
- Branch: main
- Retrieved: 2026-09-22T14:02:00Z
- Staging mode: FULL_SOURCE_TREE
- Clone path: .atlas/temporary/donors/sigil-lang

### Provenance note

`pzalutski-pixel/sigil-lang` shows no fork/mirror indicators (no `.github` template pointing elsewhere, no upstream remote recorded, README frames the project as the author's own original work — "an experimental, AI-native programming language" by a single named copyright holder). Treated as **canonical/original origin**, not a fork or mirror of another project, on the evidence available from a single shallow clone (no GitHub API fork-parent check was performed — this is a static, source-only census).

## Coarse Inventory

- Files observed: 581
- Bytes observed: 2,499,402
- Languages/signals: Rust (compiler, 24 `.rs` files), C (runtime, 11 `.c`/3 `.h` files), Sigil source (`.sigil`, 63 files; `.beh` behavior-contract files, 256 files), Markdown (77, mostly spec/docs/experiment session logs), Batch/PowerShell/Shell build scripts (61 `.bat`, 4 `.ps1`, 1 `.sh`), Python (8, tooling scripts), TOML (Cargo manifest), JSON (1 lockfile-adjacent `.lock`/manifest data)
- Top-level directories: `.github`, `compiler`, `docs`, `examples`, `experiments`, `runtime`, `skills`, `stdlib`
- Top-level files: `.gitattributes`, `.gitignore`, `LICENSE`, `README.md`, `build.bat`, `clean.bat`

Full source tree staged with nested `.git` removed.

## Direct Dependencies

From `compiler/Cargo.toml` (exact versions resolved in `compiler/Cargo.lock`):

| Crate | Declared | Locked version | Role |
|---|---|---|---|
| `thiserror` | 1.0 | 1.0.69 | error type derive |
| `anyhow` | 1.0 | 1.0.100 | error handling |
| `inkwell` | 0.5, feature `llvm18-0` | 0.5.0 | safe Rust bindings over LLVM 18 (compiler's actual LLVM interface) |
| `sha2` | 0.10 | 0.10.9 | **contract-hash computation** (SHA-256; see Mechanisms Census) |
| `hex` | 0.4 | 0.4.3 | hex encoding of hash bytes |
| `serde` (+derive) | 1.0 | 1.0.228 (+ `serde_core` 1.0.228, `serde_derive` 1.0.228) | incremental-cache manifest (de)serialization |
| `serde_json` | 1.0 | 1.0.149 | cache manifest JSON format |
| `tempfile` (dev) | 3.10 | 3.24.0 | test scaffolding only |

## Transitive Dependencies

`compiler/Cargo.lock` resolves **49 packages total** (including `sigil-compiler` itself), a Cargo-computed fixed point for the Rust dependency graph. Full resolved set: `anyhow`, `bitflags`, `block-buffer`, `cc`, `cfg-if`, `cpufeatures`, `crypto-common`, `digest`, `either`, `errno`, `fastrand`, `find-msvc-tools`, `generic-array`, `getrandom`, `hex`, `inkwell` 0.5.0, `inkwell_internals` 0.10.0, `itoa`, `lazy_static`, `libc`, `linux-raw-sys`, **`llvm-sys` 180.0.0** (LLVM 18 FFI — confirms `inkwell` is a real, compiling LLVM binding, not an aspirational README claim), `memchr`, `once_cell`, `proc-macro2`, `quote`, `r-efi`, `regex-lite`, `rustix`, `semver`, `serde`, `serde_core`, `serde_derive`, `serde_json`, `sha2` 0.10.9, `shlex`, `sigil-compiler`, `syn`, `tempfile`, `thiserror`, `thiserror-impl`, `typenum`, `version_check`, `wasip2`, `windows-link`, `windows-sys`, `wit-bindgen`, `zmij`.

**Closure status: COMPLETE for the Rust/Cargo dependency graph** — `Cargo.lock` is itself the tool-computed fixed point of manifest resolution, so no deeper crate-source reading was needed to enumerate names/versions. **Not verified**: the internal source of each of the 48 non-Sigil crates was not individually read (out of scope for a coarse+targeted census); only `inkwell`/`llvm-sys`/`sha2` were read where they bear directly on the census-directed mechanisms. The C runtime (`runtime/src/*.c`) has **no package manager** — its dependency closure is the host libc/pthreads/BSD-sockets/Win32 APIs only, an explicit **system/toolchain terminal boundary** (POSIX + Win32), not a resolvable package graph.

## Build Systems Detected

- `.atlas/temporary/donors/sigil-lang/compiler/Cargo.toml` + `Cargo.lock` — Rust compiler crate (`cargo build`)
- `.atlas/temporary/donors/sigil-lang/build.bat` (root) and `compiler/build.bat`, `examples/build.bat`, `runtime` build scripts — Windows batch driving `cargo build` + a C compiler for the runtime + `llvm-ar`/linker steps
- No CMake/Make for the C runtime observed; it is built via the batch/shell scripts invoking a system C compiler directly (not executed — static inspection only, per instructions)
- `.github/workflows/ci.yml` — GitHub Actions CI (Linux x86-64 and macOS arm64, per `README`/status doc)

## Test / Benchmark Roots Detected

- `.atlas/temporary/donors/sigil-lang/compiler/tests/integration.rs` — Rust integration tests
- Unit tests embedded `#[cfg(test)]` throughout compiler modules (`hash.rs`, `graph/validate.rs`, `graph/builder.rs`, `semantic/mod.rs`, `cfg.rs`, `lexer.rs`, `parser.rs`, etc.)
- `examples/` (9 example programs: `hello-world`, `file-copy`, `http-server`, `kv-store`, `channel-test`, `parallel-test`, `producer-consumer`, `spawn-test`, `test-lib`) function as the project's integration/behavior tests — per the donor's own `STATUS.md`, these are "its integration tests until equivalent automated coverage exists" for the concurrency runtime specifically
- No dedicated `benches/` directory; `STATUS.md` states explicitly: "Performance benchmarks: None — 'Compiles via LLVM' is not a measured speed claim"
- `experiments/http-relay/` — AI-authored-code trial logs (SESSION.md transcripts), not automated tests; evidentiary material for the donor's own "does the idea help" learnability claim, not a test suite

## Major Subsystem Roots

- `compiler/src/` — Rust native compiler (lexer, parser, AST, semantic analysis, CFG/dataflow, behavior graph, LLVM codegen, linker, incremental cache)
- `runtime/src/` + `runtime/include/` — C runtime (memory/scope allocator, scheduler, worker threads, channels, task, file/net/os/console/time syscalls edge)
- `stdlib/units/` — 92 `.beh`-contracted standard-library behaviors, written in Sigil itself over the C runtime's NATIVE primitives
- `docs/spec/` — language specification (`COMPOSITION.md`, `CONCURRENCY.md`, `CONTRACT-SCHEMA.md`, `GRAPH-FORMAT.md`, `MEMORY-MODEL.md`, `PRIMITIVE-LAYER.md`, `PROJECT-STRUCTURE.md`, `SAFETY-RULES.md`, `TYPE-SYSTEM.md`)
- `examples/` — 9 end-to-end example programs
- `experiments/http-relay/` — AI-agent authoring trials (the "does the idea actually help" evidence referenced in the README)
- `skills/sigil-authoring/SKILL.md` — a packaged authoring skill for AI agents writing Sigil

## License Evidence

- `.atlas/licenses/donors/sigil-lang/LICENSE` (copied verbatim from the clone root; original left in place in the clone tree)
- **License: Apache License, Version 2.0**, full standard text (190 lines), with an explicit copyright notice at the end of the file: `Copyright 2025-2026 Peter Zalutski`
- No other `LICENSE`/`COPYING` files found anywhere else in the tree (single root license governs the whole repository, including the C runtime and stdlib `.beh` files — no separate/conflicting licensing observed for any subdirectory)

## Mechanisms Census

### Behavior contract identity

**Provider: Sigil's own compiler (`compiler/src/graph/mod.rs`, `compiler/src/hash.rs`).** A "behavior" is a named contract (`INPUT`/`OUTPUT` ports, `GUARANTEES`, `REQUIRES` list) plus an optional implementation body. `BehaviorNode::compute_hash()` / `update_hash()` derive the behavior's identity **only from its contract surface** (inputs, outputs, requires-with-pinned-hashes, guarantees) — the implementation body is explicitly **not** part of the hash input. Identity is therefore "the interface is the identity," not "the code is the identity." Verified by direct code reading of `graph/mod.rs` lines ~283-299 and `hash.rs`.

### Behavior dependency hashing (content-addressing)

**Provider: Sigil's own compiler, using the `sha2` crate (RustCrypto) as the underlying primitive.** `compiler/src/hash.rs::compute_contract_hash` hashes a canonical byte serialization of `INPUTS:`/`OUTPUTS:`/`REQUIRES:`/`GUARANTEES:` sections (field name bytes, a 1-byte type tag, little-endian size, and for requires the dependency's own pinned hash string) through **SHA-256**, then **truncates to the first 4 bytes (32 bits) and hex-encodes them to an 8-character hash**. This is a verified, falsifiable, load-bearing detail: the hash function used for real is SHA-256-then-truncated-to-32-bits, not full SHA-256, not BLAKE3, not FNV. A dependency reference (`REQUIRES name@hash`) pins that 8-hex-char value; `graph/validate.rs::check_hash_consistency` recomputes the current contract hash of the referenced behavior and flags `StaleDependency` if it no longer matches the pin — "the hash is the version," in the donor's own words (`STATUS.md`).

### Graph validation

**Provider: Sigil's own compiler (`compiler/src/graph/validate.rs`, wired into `main.rs` "Phase 5.5").** `validate_graph()` runs four whole-graph passes over the `ProjectGraph`: (1) `check_dependencies_resolve` — every `REQUIRES` name resolves to a known local or library behavior; (2) `check_hash_consistency` — as above; (3) `check_circular_dependencies` — Kahn's-algorithm topological sort over the `REQUIRES` adjacency, any leftover in-degree means a cycle; (4) `check_transitive_purity` — a `pure`-guaranteed behavior may not transitively depend (even indirectly) on any non-pure behavior. A separate, explicit **edge-level** graph model (`Edge`, `add_edge`, `EdgeError`, `Port`-to-`Port` typed/sized connections, single-source-per-input-port rule) and a gap detector (`gaps::find_gaps`, `completeness_pct`, `topological_sort` over edges) are fully implemented and unit-tested in the same module tree but **explicitly marked dead code and not wired into the V1 compile pipeline** (`#![allow(dead_code)]` at the top of `graph/mod.rs`, with an inline doc comment stating V1 derives wiring implicitly from `CALL` references instead of materializing `Edge` values). This is an honest, source-confirmed gap between what is built-and-tested and what actually runs.

### Contract → implementation checking

**Provider: Sigil's own compiler (`compiler/src/semantic/mod.rs`).** Two distinct mechanisms, both verified by reading the code, and they are *not* the same thing:
1. **Self-consistency check** (`validate_contract`, Section 14.3): if a behavior file declares a `HASH` field, the compiler recomputes the contract hash and errors (`HashMismatch`) if it disagrees with the declared value. This is a syntactic self-consistency check on the contract's own header, not a check of implementation-vs-contract behavior.
2. **Guarantee-vs-implementation static analysis** (`validate_guarantees`, Section 14.6, plus `forbidden_compute_op` and the CFG-based `analyze_data_flow`): this *is* real contract-to-body checking — e.g. a `pure`-declared behavior's implementation is scanned for `CHANNEL` ops, non-`OUTPUT` writes, and pattern-`MEMORY` access (`PureViolation` if found); `no_alloc` is checked against unscoped `ALLOC`s; `writes_output` is checked via CFG all-paths-write analysis. Per the donor's own `STATUS.md`, these are explicitly "structural" checks (no channels / no non-output writes / no MEMORY / no non-pure deps), **not a semantic referential-transparency proof** — the donor itself is honest that "pure" is enforced by a decidable syntactic rule, not by proving the function has no side effects in the mathematical sense.

### Primitive/SSA-style graph

**Provider: mixed — Sigil's own CFG builder for the basic-block analysis, and LLVM (via `inkwell`) for the actual SSA form.** There is **no** Sigil-authored SSA graph structure. `compiler/src/cfg.rs` builds a basic-block **control-flow graph** (`BasicBlock`, `Terminator`, `CFG`) per behavior, used only for static-analysis passes (initialization-before-use, leak/use-after-free/double-free, all-paths-terminate, all-paths-write-output) — it is a CFG for verification, not an SSA value-graph for optimization. Actual SSA form is obtained for free from **LLVM itself**: the codegen module (`compiler/src/codegen/mod.rs`) allocates every value as a stack `alloca` and lets LLVM's own `mem2reg` pass promote allocas to SSA registers/phi nodes (confirmed by an explicit code comment: "Allocated in the entry block; LLVM mem2reg promotes them back to SSA/phi"). So: **the "SSA-style primitive graph" claim, if read as "Sigil has its own SSA IR," is not supported by the source** — the SSA form that exists is LLVM's, reached by the conventional alloca+mem2reg idiom, not a Sigil-native primitive graph.

### Compiler lowering / "is LLVM really used?"

**Provider: LLVM 18, via the `inkwell` 0.5.0 safe-binding crate, which in turn pulls the real `llvm-sys` 180.0.0 FFI crate (confirmed in `Cargo.lock`, not just asserted in `README`).** Verified pipeline, read directly from source: `compiler/src/ast.rs` (parsed AST) → `compiler/src/graph/*` (contract/behavior graph, validated) → `compiler/src/codegen/mod.rs` + `codegen/expr.rs` + `codegen/extern_decl.rs` (direct AST-to-LLVM-IR translation using `inkwell::builder::Builder`, one LLVM function per behavior) → `TargetMachine::write_to_file(&module, FileType::Object, path)` (native object code emitted **through LLVM's own backend**, confirmed at `codegen/mod.rs:380-383` — **no external `llc` process is invoked**; LLVM's C++ codegen runs in-process via the FFI binding) → `compiler/src/linker.rs` invokes an **external system linker**: `llvm-ar` (via `std::process::Command`) to build `.lib`/archive files, and the platform's own linker to produce the final executable (MSVC `link.exe`, located via `vswhere`, on Windows; `cc`/`clang` on Linux/macOS, per `#[cfg(target_os = "linux"/"macos")]` blocks calling `Command::new(LINKER)`). There is **no staged IR** (no separate HIR/MIR/LIR akin to Atlas's planned pipeline) — the AST/behavior-graph lowers essentially in one step to LLVM IR, and all further staging (SSA construction, register allocation, instruction selection, machine code) is entirely LLVM's responsibility, opaque to Sigil's own source.

### What semantic information survives through compilation

Verified from source and `STATUS.md`: contract shape (ports/types/sizes), `pure`/`no_alloc`/`writes_output` guarantees, and the leaf/composite distinction are all **enforced at compile time and then discarded** — none of them are carried into the emitted LLVM IR or object code as metadata; they exist purely as compiler-internal verification gates. The **contract hash itself** does not appear in the emitted binary either (no build-ID/content-hash embedding was found in `codegen/mod.rs` or `linker.rs`); it lives only in source-adjacent artifacts (`.beh` files, the incremental-cache manifest `compiler/src/cache.rs`). So: **semantic verification happens once, at compile time, and the binary carries no self-describing trace of the contract that produced it** — a materially different posture from a system that embeds content-addressed identity into the artifact itself.

## Atlas Comparison

### Content-addressed identity: Sigil's contract hash vs Atlas's SemanticRecordId/RawObservationId

- **Atlas (current, `core/src/semantic/observation.rs`, `core/src/identity/mod.rs`):** `SemanticRecordId` ("claim identity") is derived from a family's `identity_key()` only (e.g. repo/revision/scope/name), independent of which extractor produced it or what evidence backs it. `RawObservationId` is a *separate* identity covering claim identity + extractor id/version + epistemic status + evidence refs + provenance + the full typed payload — explicitly designed so two independent extractors (or the same extractor re-observing) can share a `record_id()` while remaining distinguishable at the raw-observation level (per code comment, matching `.atlas/contracts/SEMANTIC-EXTRACTION.md` and `NORMALIZATION.md`: "RawRecordId != NormalizedRecordId"). The underlying hash primitive, `stable_id()`, is **FNV-1a, 64-bit** (`0xcbf29ce484222325` offset basis / `0x100000001b3` prime) — a fast non-cryptographic hash, not collision-resistant against adversarial input, explicitly a bootstrap mechanism per its module context.
- **Sigil:** one flat identity per behavior — the contract hash — derived only from the contract's declared surface (no separate "raw observation vs normalized claim" split; no extractor/evidence provenance folded into the hash at all). Underlying primitive: **SHA-256, truncated to 32 bits (8 hex chars)**.
- **Assessment:** these solve different problems and are not directly substitutable. Atlas's split exists because Atlas ingests facts from multiple independent, possibly-disagreeing extractors over evolving evidence — a problem Sigil does not have (a behavior's contract has exactly one author-declared source of truth, no multi-extractor reconciliation). Sigil's single-hash model is comparably shaped to what Atlas's **RawObservationId already does at the "collapse a canonical content surface to a fixed identity" level**, not a novel structure. On raw hash strength: Atlas's 64-bit FNV-1a and Sigil's 32-bit-truncated-SHA-256 are both non-collision-resistant *as deployed* (32 bits truncated SHA-256 is dramatically weaker than untruncated SHA-256 would be, and weaker than Atlas's 64-bit FNV-1a for accidental-collision purposes at any real behavior-count scale); neither is a cryptographic identity guarantee. This reinforces rather than reduces the existing BLAKE3 donor's relevance (`.atlas/provenance/donors/blake3.json`, already target-owned by `compiler`/`runtime/binary`/`core/atlas-format`) — Sigil does not introduce a stronger hash primitive than what Atlas already has queued.

### Contract checking: Sigil's guarantee-vs-implementation analysis vs Atlas's CodingAdmission/constraint model

- **Atlas (current, `core/src/constraint/mod.rs`):** `CodingAdmission` and `validate_manifest()` are a **repository-manifest admission gate** — they check that a `RepoManifest`'s fixed fields (schema, system_kind, backend/frontend language, root paths, boolean policy flags like `coding_requires_docs_gate`, `graph_before_code_required`) match required constants, and return a list of violation strings. This is a coarse, whole-repo eligibility check, not a per-function or per-behavior semantic verifier.
- **Sigil:** `validate_guarantees` + `forbidden_compute_op` + CFG dataflow analysis are a **per-behavior static verifier** that inspects each behavior's own implementation body against its own declared guarantees (`pure`, `no_alloc`, `writes_output`), with a whole-graph pass (`check_transitive_purity`) extending `pure` transitively across the dependency graph.
- **Assessment:** these are not peers at the same altitude — Atlas's `CodingAdmission` is closer to Sigil's *manifest-level* checks (e.g. "does this repo/behavior file even have the required header fields") than to Sigil's *guarantee verification*. Atlas has **no current analog** to Sigil's per-behavior structural purity/no-alloc/writes-output CFG verification; that capability is genuinely absent from `core/src/constraint/`. Whether it is *needed* depends on whether Atlas's compiler pipeline (HIR/MIR per `COMPILER-IR-PIPELINE.md`) intends to carry and enforce comparable behavior-level guarantees — the contract says MIR "must make ownership/effect/concurrency/persistence barriers represented" and preserve them through lowering, which is a stronger and broader ambition than Sigil's three guarantees, but Atlas's MIR/verifier stage is not yet materialized to compare against in practice (see Semantic Depth below).

### Graph model: Sigil's ProjectGraph vs Atlas's EngineeringGraph / UNIVERSAL-GRAPH-CONTRACT

- **Atlas (current, `core/src/graph/mod.rs`):** `EngineeringGraph` (`Node`/`Edge`/`Binding`/`Fact`, all string-typed `kind`/`identity` fields plus a generic `BTreeMap<String,String>` attributes bag) is built by `build_source_graph`/`build_repository_graph`/`build_system_graph` from census/provenance evidence — it is a **repository/system-evidence graph** (what does this codebase contain and how is it wired), matching the `UNIVERSAL-GRAPH-CONTRACT.md` spine (`Identity/Scope/Node/Edge/Binding/State/Event/...`) at a coarse, largely untyped-string level today (the contract itself flags the current `SemanticFact{subject,predicate,object}` shape as a bootstrap-only envelope that "MUST NOT become the permanent universal model").
- **Sigil:** `ProjectGraph` (`BehaviorNode`/`Port`/`Edge`/`ContractRef`) is a **program-level execution-semantics graph** — it *is* the compiled program's own representation (nodes are behaviors with typed, sized ports; edges are typed data-flow connections; the graph is walked for compilation order and dependency validation), not a graph *about* a codebase.
- **Assessment:** these operate at different layers and are not substitutable. Sigil's `ProjectGraph` is the closer analog to what Atlas's **AtlasX/HIR** stage is meant to become (per `COMPILER-IR-PIPELINE.md`: "HIR must retain... functions... interfaces/capabilities... bindings") once that stage is materialized as a program-native graph IR, not to Atlas's current `EngineeringGraph`, which serves Atlas's own repository-census purpose. No comparison at the AtlasX/HIR level is possible yet because that Atlas stage does not exist in `core/` today (see Semantic Depth Achieved).

### Compiler pipeline staging: Sigil's single-step AST→LLVM-IR vs Atlas's HIR→MIR→LIR→Machine IR

- **Atlas (contract, `.atlas/contracts/COMPILER-IR-PIPELINE.md`):** specifies a five-stage canonical pipeline (AtlasX → HIR → MIR → LIR → Machine IR → object/link), each with its own identity, invariant-preservation, and maturity-gate requirements, explicitly to prevent semantic information (effect/authority/ownership/concurrency/persistence barriers) from being silently discarded during lowering.
- **Sigil:** has **no staged IR of its own**. The behavior/contract graph (post-validation) lowers essentially directly to LLVM IR in `codegen/mod.rs`; whatever staging exists past that point (SSA construction, instruction selection, register allocation) is entirely internal to LLVM, opaque to and unverified by Sigil's own source. Sigil's `docs/STATUS.md` and this census's "what semantic information survives through compilation" section both establish that Sigil's compile-time guarantees (pure/no_alloc/writes_output) are checked once and then **dropped** — they do not survive into IR metadata for a later stage to re-verify or exploit.
- **Assessment:** Sigil is architecturally the simpler of the two designs (single-pass native compiler bolted directly onto LLVM), which is a reasonable, working V1 engineering choice for its scope, but it is not evidence for or against Atlas's staged-IR ambition — Sigil never attempted staged IR, so there is no comparative outcome data (e.g. no evidence Sigil's simpler approach loses or preserves more semantic information than a staged approach would, only that it doesn't attempt to preserve semantics past compile time at all).

## Discoveries

1. **`REQUIRES name@hash` dependency pinning with whole-graph stale-hash + cycle + transitive-purity validation, wired into the real compile path** — `graph/validate.rs`, Phase 5.5 of `main.rs`. Disposition: **REFERENCE_ONLY**. It is a clean, small, well-tested worked example of "the hash is the version" dependency pinning at language scope; useful as a design reference if/when Atlas's own AtlasX/HIR stage needs an analogous dependency-identity-pin validator, but it is Rust source over Sigil's own bespoke contract format — not directly portable, and Atlas has no current per-behavior compilation unit to attach it to.
2. **Truncated-SHA-256 (32-bit) contract hash is the actual identity primitive, not full SHA-256** — `hash.rs`. Disposition: **REJECT** (as a hash strategy to imitate). Verified via source read of `compute_contract_hash`: only the first 4 bytes of the SHA-256 digest are kept. This is a real collision-resistance weakness at scale (birthday bound ~2^16 behaviors) that the donor's own README/STATUS.md do not flag; Atlas should not adopt 32-bit truncation as a pattern even for a bootstrap identity scheme, since BLAKE3 is already queued as the intended stronger primitive for content-addressed identity work (`.atlas/provenance/donors/blake3.json`).
3. **Purity/no_alloc/writes_output declared explicitly as *structural*, not semantic, by the donor itself** — `docs/STATUS.md`, corroborated by `forbidden_compute_op`/`validate_guarantees` source. Disposition: **REFERENCE_ONLY**. Valuable as a template for how to phrase compiler-diagnostic honesty (Atlas's own contracts insist on EpistemicStatus rigor — DECLARED vs OBSERVED vs DERIVED — and Sigil's STATUS.md independently arrives at a similar "declared guarantee, structurally enforced, not semantically proven" distinction worth citing as convergent practice, not something to import as code).
4. **The explicit `Edge`/gap-detector graph machinery exists, is unit-tested, but is `#[allow(dead_code)]` and unused by V1** — `graph/mod.rs`, `graph/gaps.rs`. Disposition: **EXTERNAL_BOUNDARY** (not currently usable evidence of a working mechanism — it is unexercised code, explicitly flagged as such by its own authors' doc comments). Any future recensus of this donor, if it reaches a V2 where this machinery is wired in, should re-examine it as a possible graph-completeness-checking reference.
5. **LLVM used for real via `inkwell`/`llvm-sys`, with native object emission in-process (no `llc` shell-out) and an external-linker-only final step** — verified via `Cargo.lock` + `codegen/mod.rs` + `linker.rs`. Disposition: **REFERENCE_ONLY**. Confirms the README/STATUS.md's LLVM claims are true, source-backed engineering (not marketing) — useful precedent if Atlas's own `COMPILER-IR-PIPELINE.md` "External native backends" section (LLVM/Cranelift/WASM) is ever exercised, as a real example of the inkwell integration pattern and its external-linker dependency surface (`llvm-ar`, MSVC `link.exe`/`cc`).
6. **No semantic information (contract hash, guarantees) survives into the emitted binary or object file** — verified absence, `codegen/mod.rs`/`linker.rs` read in full for any content-hash/build-id embedding; none found. Disposition: **REFERENCE_ONLY** (negative finding, useful for scoping expectations — this donor is not a model for "materialization carries content-addressed identity into the artifact," which is closer to what Atlas's own `ATLAS-FORMAT.md`/materialization contracts already aim for independently).

## Blueprint Revision Candidates

None. No mechanism in this donor is falsifiably superior, for a load-bearing Atlas capability, to Atlas's current or contractually-specified approach:

- The contract-hash/dependency-pin mechanism is a smaller, narrower version of identity concerns Atlas's `SemanticRecordId`/`RawObservationId` split and `NORMALIZATION.md` already address more completely (multi-extractor reconciliation, evidence lineage) — Sigil's version has no multi-source reconciliation problem to solve in the first place.
- The truncated-SHA-256 hash is measurably *weaker* than either Atlas's current FNV-1a-64 bootstrap identity or the BLAKE3 donor already queued for content-addressing work — not a candidate to adopt.
- The structural guarantee-checking (pure/no_alloc/writes_output) is real and tested, but it operates on a much smaller semantic surface (3 guarantees) than what `COMPILER-IR-PIPELINE.md`'s MIR contract already specifies Atlas must eventually enforce (effect/authority/ownership/concurrency/persistence/transaction barriers) — it is a proof that *some* version of structural guarantee-checking is buildable and testable, not evidence that Atlas's broader planned scope is wrong or should be narrowed to match Sigil's.
- The single-pass AST→LLVM-IR compiler (no staged HIR/MIR/LIR) is a valid, working engineering choice for Sigil's narrower scope, but Atlas's `COMPILER-IR-PIPELINE.md` staged design exists specifically to prevent the kind of semantic-information loss that Sigil's own census section ("what semantic information survives through compilation") shows *does* happen in Sigil (guarantees checked once, then dropped, not carried through to the binary). This is evidence *for* Atlas's staged approach being the more conservative choice, not against it.

## Recensus Requirements

- If Sigil's V2 (explicit `Edge`-graph wiring, error ports, expression-AST desugarer, mentioned throughout the source as "V2 items") lands, recensus the graph-validation and gap-detector mechanisms, since they are currently unexercised dead code.
- If Atlas's own AtlasX/HIR stage becomes materialized in `core/`, revisit the "Graph model" and "Compiler pipeline staging" comparison sections above — both were written against the *contract* text only, since no AtlasX/HIR implementation exists yet to compare Sigil's `ProjectGraph`/single-pass lowering against directly.
- This census did not execute any donor code, tests, or build scripts (static inspection only, per instructions); a future recensus with executable sandboxing could verify the claimed `cargo test` and example-program behavior rather than relying on the donor's own `STATUS.md` claims (which were corroborated against source but not run).
- Individual source of the 48 non-Sigil transitive Rust crates was not read; if any of them (particularly `inkwell`/`llvm-sys`, `sha2`, `serde_json`) become independently interesting as donors in their own right, they need their own dedicated census entries rather than being folded into this one.

## Census State

Status: **DEEP_CENSUS_ACTIVE**. Coarse inventory, direct/transitive dependency resolution (complete for the Rust/Cargo graph; the C runtime has no package graph to resolve, only a system/toolchain boundary), and a targeted deep census of the six requested mechanisms (behavior contract identity, dependency hashing, graph validation, contract-vs-implementation checking, primitive/SSA graph, LLVM lowering, and semantic-survival-through-compilation) are complete and source-verified. Not yet done: execution-based verification (donor code was not run, per instructions), and a full read of the 48 non-Sigil transitive crate sources.

## G92 — terminal REFERENCE_ONLY; source extinct

The census left no open item, and neither recensus trigger has fired: Sigil V2 has not landed at the pin, and Atlas has no materialized AtlasX/HIR stage. The one capability Atlas lacks, per-behavior structural guarantee checks, belongs to the unimplemented MIR barrier contract, whose scope is wider. The 32-bit truncated contract hash stays REJECTED as a pattern. The checkout was physically deleted, and it was not a DC1 Cargo test input. Evidence: `../../evidence/campaign/28-sigil-lang.json`.
