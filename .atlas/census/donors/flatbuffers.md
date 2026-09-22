---
id: donor-census-flatbuffers
type: reference
status: active
canonical: true
---
# Donor Census: FlatBuffers

## Source

- Remote: https://github.com/google/flatbuffers.git
- Repository: google/flatbuffers
- Commit: b8431fbcd7a5c71817f314e18b332c0648554efa
- Git tree: 0b3c731daf52a90658ab86e4912075ea4b7da735
- Branch: master
- Retrieved: 2026-09-21T03:00:34.9575636Z
- Staging mode: FULL_SOURCE_TREE
- Clone path: .atlas/temporary/donors/flatbuffers

### Framing (explicit, per absorption brief)

Censused as a **binary/canonical encoding donor**, with specific focus on schema evolution rules, direct/zero-copy access, vtable-based binary layout, forward/backward compatibility, and (deep-dive requirement) **Rust support** in `rust/`, for lessons applicable to Atlas's own Rust-native `.atlas` reader/writer. This document supersedes the prior `COARSE_CENSUSED` state (an inventory-only stub); everything below is read from real source/spec files, not summarized from README.

## Coarse Inventory

- Files observed: 1947
- Bytes observed: 13,950,171 (~13.9 MB on disk pre-quarantine; staged `du -sh` = 19M including filesystem block overhead)
- Languages/signals: TypeScript, Rust, Python, C/C++ header, Java, FlatBuffers schema (`.fbs`), JavaScript, C++, Markdown, Go, Shell
- Top-level directories: `.bazelci`, `.github`, `android`, `bazel`, `benchmarks`, `CMake`, `dart`, `docs`, `examples`, `go`, `goldens`, `grpc`, `include`, `java`, `js`, `kotlin`, `lobster`, `lua`, `mjs`, `net`, `nim`, `php`, `python`, `reflection`, `rust`, `samples`, `scripts`, `snap`, `src`, `swift`, `tests`, `ts`
- Top-level files: `.bazelignore`, `.bazelrc`, `.clang-format`, `.clang-tidy`, `.editorconfig`, `.gitattributes`, `.gitignore`, `.npmrc`, `build_defs.bzl`, `BUILD.bazel`, `CHANGELOG.md`, `CMakeLists.txt`, `composer.json`, `CONTRIBUTING.md`, `eslint.config.mjs`, `extensions.bzl`, `FlatBuffers.podspec`, `Formatters.md`, `library.json`, `LICENSE`, `MODULE.bazel`, `package.json`, `Package.swift`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`, `README.md`, `SECURITY.md`, `swift.swiftformat`, `tsconfig.json`, `tsconfig.mjs.json`, `typescript.bzl`

Full source tree staged with nested `.git` removed.

## Build Systems Detected

`BUILD.bazel`, `CMakeLists.txt`, `MODULE.bazel` (root); per-language builds under `android/`, `bazel/`, `go/BUILD.bazel`, `grpc/`, `java/pom.xml`, `kotlin/*.gradle.kts`, `reflection/BUILD.bazel`, `rust/flatbuffers/Cargo.toml`, `rust/flexbuffers/Cargo.toml`, `rust/reflection/Cargo.toml`, plus Rust-specific test crates `tests/rust_no_std_compilation_test/Cargo.toml`, `tests/rust_reflection_test/Cargo.toml`, `tests/rust_serialize_test/Cargo.toml`, `tests/rust_usage_test/Cargo.toml`. None executed, per the hard no-execution rule — read as text/manifests only.

## License Evidence

- `.atlas/licenses/donors/flatbuffers/LICENSE` — read directly.
- **License: Apache License, Version 2.0**, standard full text, verified.

## Mechanisms Census

### Binary layout — vtables and offsets

**Provider: `docs/source/internals.md` (read in full) cross-checked against `rust/flatbuffers/src/vtable.rs` (read directly).** FlatBuffers has two object kinds:

- **Structs**: fixed-layout, always stored **inline** in their parent, fields aligned to their own size and the struct aligned to its largest member, independent of the underlying compiler's own struct-layout rules — chosen specifically for cross-platform binary-compatible layout. Structs have **no versioning/extensibility** by design; they exist purely for maximum-compactness fixed data.
- **Tables**: referred to by **offset** (never inline), and instead of a fixed field layout, every table instance is prefixed by a **signed offset (`soffset_t`) to a vtable**. The vtable is itself a small array of `voffset_t` (`u16`) entries: `[vtable_byte_size, object_byte_size, field_0_offset, field_1_offset, ...]`. **Vtables are shared/deduplicated** across any table instances that happen to have identical field-presence patterns — a real space optimization, not merely a lookup indirection.
- **Field access mechanics, verified from `rust/flatbuffers/src/vtable.rs`**: `VTable::num_fields()` computes `(vtable_byte_size / SIZE_VOFFSET) - 2` (the "-2" accounts for the two header slots — vtable size and object size — that aren't field offsets), and every generated accessor's constant field-offset is **checked against the vtable's own recorded field count before use** — this is exactly the mechanism that lets old generated code safely read new-format data (offset out of range → field absent → default value) and new code safely read old data (same check, same fallback), without any explicit format version number anywhere in the file. The absence of a version field is explicit, documented project policy (`docs/source/internals.md`, "Format identification" section): *"the format itself does not need a version number ... versioning is something that is intrinsically part of the format."*

### Schema evolution rules

**Provider: `docs/source/evolution.md` (read in full, 277 lines) — the project's own normative rule set with worked before/after examples.** The load-bearing rules, all directly verified from the source text:

- **New fields MUST be appended to the end of the table definition** (unless every field carries an explicit `id` attribute, in which case wire order is decoupled from declaration order — directly analogous to Cap'n Proto's ordinal numbering, verified independently in that donor's census).
- **Fields must never be removed**, only stopped-being-written and optionally marked `deprecated` (which additionally suppresses accessor/setter codegen to actively discourage continued use) — removal-by-omission, not removal-by-deletion, exactly mirroring Cap'n Proto's rule.
- **Renaming tables/fields is free** — names are never serialized to the wire, only field IDs/offsets are, so renames can't affect binary compatibility (again, directly analogous to Cap'n Proto).
- The doc includes explicit **"Improper Addition"** and **"Improper Deletion"** worked examples showing exactly how inserting a field in the middle (rather than the end) silently corrupts data for both old-code-reads-new-data and new-code-reads-old-data directions — concrete, falsifiable failure-mode documentation, useful as a template for how Atlas should document its own evolution rules.
- **Union evolution**: new variants may be appended freely; inserting in the middle is unsafe *unless* explicit discriminant values are assigned (`A = 1, another_a: A = 3, B = 2`), which is the union-equivalent of the field-`id` escape hatch.
- **Changing a field's default value is called out as NOT OK** for already-shipped schemas, because old data that never wrote a value relied on the *old* compiled-in default — a subtlety worth carrying into any Atlas evolution-rules doc, since it's easy to overlook "default value" as part of the wire contract.
- A dedicated **`flatc --conform <base.fbs> <new.fbs>`** compiler flag mechanically checks whether a new schema properly evolves from a base schema, returning nonzero with explicit errors on violation — i.e. FlatBuffers ships a **tool that enforces its own evolution rules automatically**, rather than relying on developer discipline alone. This is a concrete, buildable idea for Atlas: a `.atlas`-schema-conformance-checker CLI mode, not just a documented convention.

### Direct/zero-copy serialized access

**Provider: `docs/source/internals.md`, cross-checked against `rust/flatbuffers/src/follow.rs` and `rust/flatbuffers/src/table.rs`.** Because table fields are reached via vtable offset rather than a fixed struct layout, and because the vtable offset check described above makes "is this field present" an O(1) bounds check rather than a parse step, FlatBuffers achieves **field access with zero deserialization pass** — a reader holding a `&[u8]` and a root offset can read any field directly. The project is explicit that this comes at a real, named cost relative to the packed-inline alternative: "padding takes space on the wire" is `no` in rkyv's own feature-comparison table (verified independently in the rkyv census) specifically because FlatBuffers tables **don't store unset fields at all** (vtable entry = 0 / absent → default), trading a vtable-indirection cost for genuinely sparse storage of optional fields — a real, quantifiable design tradeoff against Cap'n Proto's XOR-default-implies-zero approach (which stores every field's slot even if it's the default, relying on packing/compression to reclaim the space).

### Compatibility model (forward/backward)

Directly enabled by the same two mechanisms above: (1) vtable field-count bounds-checking makes forward compatibility ("old code, new data") safe by construction — an out-of-range field offset always means "absent, use default," never "corrupt read"; (2) append-only field ordering plus never-reuse-a-removed-slot makes backward compatibility ("new code, old data") safe — a field simply reads as its default if the old data's vtable doesn't include it. Both directions are handled by the **same single mechanism** (vtable bounds-checking), which is a notably economical design compared to needing separate rules for each direction.

### Rust support (`rust/` — deep-dive requirement)

**Provider: `docs/source/languages/rust.md` plus direct reads of `rust/flatbuffers/src/{vtable.rs, verifier.rs, builder.rs, follow.rs, table.rs}`.**

- The Rust crate (`rust/flatbuffers/`) is a **from-scratch Rust implementation of the reader/writer/vtable logic**, not a thin FFI wrapper over the C++ implementation — directly relevant precedent for Atlas wanting its own native Rust reader/writer rather than binding a C library.
- `VTable<'a>` (in `vtable.rs`) wraps a `&'a [u8]` plus a `loc: usize` and is constructed via an explicit `unsafe fn init` whose safety contract is documented inline ("`buf` must contain a valid vtable at `loc`") — i.e. the crate separates a cheap, `unsafe`, precondition-documented fast construction path from a **separate, safe verification layer**.
- That separate safe layer is `verifier.rs` (629 lines): a dedicated `InvalidFlatbuffer` error enum with variants like `MissingRequiredField`, `InconsistentUnion`, each carrying an `ErrorTrace`/`ErrorTraceDetail` (vector index, table field name, union variant — with source position) for **precise, debuggable validation failures**, distinguished explicitly in-source from a separate class of "DoS detecting errors" that intentionally carry no extra trace detail (to avoid the trace itself becoming a resource-exhaustion vector) — a directly reusable idea for any Atlas validator: rich traces for data errors, minimal/cheap traces for adversarial-input-shaped errors.
- The Rust crate exposes both **fallible (`try_create_string`, `try_push`, `try_finish` returning `Result<T, A::Error>`) and panicking builder APIs**, plus a custom-`Allocator` trait (`unsafe impl Allocator for MyAllocator`) so the builder can run over a caller-supplied allocation strategy — relevant if Atlas wants a `no_std`/fixed-capacity-buffer construction mode for its own writer, not just a `Vec<u8>`-backed default.
- `rust_no_std_compilation_test` in `tests/` confirms the crate is actively tested for `no_std` compilation, not merely claimed to support it.

## Test / Benchmark Roots Detected

- `tests/` (root-level, cross-language conformance suite) including `rust_usage_test/` (benches + integration tests), `rust_reflection_test/`, `rust_serialize_test/`, `rust_no_std_compilation_test/`
- `benchmarks/CMakeLists.txt`
- `dart/test`, `grpc/tests`, `java/src/test`, `tests/annotated_binary/tests`, `tests/nim/tests`, `tests/swift/Tests`, `tests/ts/com/company/test`

## Major Subsystem Roots

`.bazelci`, `.github`, `android`, `bazel`, `benchmarks`, `CMake`, `dart`, `docs`, `examples`, `go`, `goldens`, `grpc`, `include`, `java`, `js`, `kotlin`, `lobster`, `lua`, `mjs`, `net`, `nim`, `php`, `python`, `reflection`, `rust`, `samples`, `scripts`, `snap`, `src`, `swift`, `tests`, `ts`

## Disposition per Concept

| Concept | Disposition | Notes |
|---|---|---|
| Vtable-indirected field access with bounds-check-implies-default semantics | **ADAPT** | The single most reusable mechanism here: one bounds check per field access, no version field needed anywhere, forward/backward compatibility both fall out of it for free. Strong candidate for `.atlas` table-like (optional-field-heavy) records. |
| Append-only field ordering + never-remove, deprecate-instead rule | **ADAPT** | Directly reusable evolution-rule wording for an Atlas schema-evolution contract; near-identical to Cap'n Proto's rule, giving two independent donors converging on the same discipline. |
| `flatc --conform` style automatic evolution-conformance checking | **ADAPT** | Concrete, buildable idea: ship a `.atlas`-schema conformance-checker rather than relying on code review alone to catch evolution-rule violations. |
| Sparse storage of unset/optional fields (vs. Cap'n Proto's always-store-default-and-compress approach) | **STUDY** | A real, named tradeoff (vtable indirection cost vs. genuinely-zero wire cost for absent fields) that Atlas should decide on deliberately, informed by both this donor and the Cap'n Proto census, not default to either without comparing. |
| Rust `unsafe`-fast-construct + separate safe `verifier.rs` validation layer, with distinct rich-trace-for-data-errors vs. minimal-trace-for-DoS-errors error design | **ADAPT** | Directly applicable pattern for an Atlas Rust reader: keep the hot unsafe path small and preconditioned, put validation in one clearly-separated module, and deliberately withhold detailed traces for errors that look adversarial rather than accidental. |
| Fallible builder API (`try_*` returning `Result`) + pluggable `Allocator` trait for `no_std`/fixed-capacity use | **STUDY** | Worth considering if Atlas wants a writer usable in constrained/embedded or allocation-tracked contexts; not obviously needed for a first version. |
| Full multi-language codegen surface (Java/Go/Swift/Kotlin/Dart/PHP/etc.) | **REJECT (out of scope)** | Atlas's own need is a Rust-native reader/writer; the other 10+ language backends were not censused and are not relevant to this donor's mandate here. |

## Things NOT to Copy

- **Never take a build/runtime dependency on the `flatc` compiler binary or the `flatbuffers`/`flatbuffers-derive` Rust crates as shipped** — this is architecture study of the binary-layout and evolution-rule *design*, not adoption of FlatBuffers' own toolchain.
- Do not copy the exact vtable byte-width choices (`u16` `voffset_t`, etc.) into `.atlas` without an independent sizing decision — the *mechanism* (indirection table + bounds-check-implies-default) is the transferable asset, not FlatBuffers' specific integer widths, which were chosen for FlatBuffers' own size/reach tradeoffs.
- Do not adopt FlatBuffers' full multi-language `flatc` code-generation architecture as a model for Atlas's tooling; it was not censused here and is a much larger surface than the binary-format question this donor was brought in to answer.

## Known Risks / Gaps in This Census

- The C++ reference implementation (`src/`, `include/flatbuffers/`) was read only for cross-reference (`docs/source/internals.md` describes it); the Rust crate (`rust/flatbuffers/src/`) was the primary code read for the Rust-specific deep-dive requirement, and only a subset of its files (`vtable.rs`, `verifier.rs`, partial `builder.rs`/`follow.rs`/`table.rs`) were read directly — the full ~3,700-line Rust crate was not read line-by-line.
- `flexbuffers` (schema-less FlatBuffers variant, `rust/flexbuffers/`) and the `reflection/` runtime-reflection subsystem were inventoried but not deep-censused; both could carry additional lessons (flexbuffers in particular for a possible schema-less Atlas debug/interchange mode) left for a follow-up pass.
- `CHANGELOG.md` at the repo root was not read for a historical account of actual breaking changes across FlatBuffers' own versions — the evolution-rules analysis above is based entirely on the current, normative `docs/source/evolution.md` spec, not on a review of how well FlatBuffers has actually held to it historically.
