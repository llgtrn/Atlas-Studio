---
id: donor-census-wasmtime
type: reference
status: active
canonical: true
---
# Donor Census: Wasmtime

## Source

- Remote: https://github.com/bytecodealliance/wasmtime.git
- Repository: bytecodealliance/wasmtime
- Commit: 7ad2e732ab9ca8665d3cdd91f9c395315eeafc81
- Git tree: 81feb43a4f161837256cd4b23f4c05b9b7ff2608
- Branch: main
- Retrieved: 2026-09-21T03:00:34.9575636Z
- Staging mode: FULL_SOURCE_TREE
- Clone path: .atlas/temporary/donors/wasmtime (corrected -- this record previously said `.atlas/temporary/wasmtime`, which does not exist; verified against the real filesystem)

## Coarse Inventory

- Files observed: 7426
- Bytes observed: 77190754
- Languages/signals: WAT, Rust, Markdown, TOML, WIT, C++ header, C++, C/C++ header, Shell, C, YAML, JavaScript
- Top-level directories: .agents, .claude, .github, benches, ci, cranelift, crates, docs, examples, fuzz, pulley, scripts, src, supply-chain, tests, winch
- Top-level files: .gitattributes, .gitignore, .gitmodules, ADOPTERS.md, AGENTS.md, build.rs, Cargo.lock, Cargo.toml, CODE_OF_CONDUCT.md, CODEOWNERS, CONTRIBUTING.md, deny.toml, LICENSE, ORG_CODE_OF_CONDUCT.md, README.md, RELEASES.md, rustfmt.toml, SECURITY.md

Full source tree staged with nested .git removed.

## Build Systems Detected

- .atlas/temporary/donors/wasmtime/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/assembler-x64/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/assembler-x64/fuzz/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/assembler-x64/meta/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/bforest/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/bitset/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/codegen/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/codegen/meta/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/codegen/shared/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/control/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/entity/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/filetests/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/frontend/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/fuzzgen/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/interpreter/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/isle/fuzz/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/isle/isle/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/isle/islec/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/isle/veri/aslp/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/isle/veri/caching/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/isle/veri/isaspec/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/isle/veri/test-macros/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/isle/veri/veri/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/jit/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/module/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/native/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/object/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/reader/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/serde/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/srcgen/Cargo.toml
- .atlas/temporary/donors/wasmtime/cranelift/umbrella/Cargo.toml
- .atlas/temporary/donors/wasmtime/crates/bench-api/Cargo.toml
- .atlas/temporary/donors/wasmtime/crates/c-api/Cargo.toml
- .atlas/temporary/donors/wasmtime/crates/c-api/CMakeLists.txt
- .atlas/temporary/donors/wasmtime/crates/c-api/artifact/Cargo.toml
- .atlas/temporary/donors/wasmtime/crates/c-api/tests/CMakeLists.txt
- .atlas/temporary/donors/wasmtime/crates/c-api-macros/Cargo.toml
- .atlas/temporary/donors/wasmtime/crates/cache/Cargo.toml
- .atlas/temporary/donors/wasmtime/crates/cli-flags/Cargo.toml

## Test / Benchmark Roots Detected

- .atlas/temporary/donors/wasmtime/benches
- .atlas/temporary/donors/wasmtime/fuzz
- .atlas/temporary/donors/wasmtime/tests
- .atlas/temporary/donors/wasmtime/cranelift/tests
- .atlas/temporary/donors/wasmtime/cranelift/assembler-x64/fuzz
- .atlas/temporary/donors/wasmtime/cranelift/bitset/tests
- .atlas/temporary/donors/wasmtime/cranelift/isle/fuzz
- .atlas/temporary/donors/wasmtime/cranelift/isle/isle/tests
- .atlas/temporary/donors/wasmtime/cranelift/isle/veri/aslp/tests
- .atlas/temporary/donors/wasmtime/cranelift/isle/veri/veri/tests
- .atlas/temporary/donors/wasmtime/cranelift/jit/tests
- .atlas/temporary/donors/wasmtime/cranelift/object/tests
- .atlas/temporary/donors/wasmtime/crates/c-api/tests
- .atlas/temporary/donors/wasmtime/crates/cache/tests
- .atlas/temporary/donors/wasmtime/crates/cache/src/worker/tests
- .atlas/temporary/donors/wasmtime/crates/component-macro/tests
- .atlas/temporary/donors/wasmtime/crates/core/tests
- .atlas/temporary/donors/wasmtime/crates/environ/fuzz
- .atlas/temporary/donors/wasmtime/crates/fuzzing/tests
- .atlas/temporary/donors/wasmtime/crates/fuzzing/wasm-spec-interpreter/tests
- .atlas/temporary/donors/wasmtime/crates/misc/component-async-tests/tests
- .atlas/temporary/donors/wasmtime/crates/wasi/tests
- .atlas/temporary/donors/wasmtime/crates/wasi/src/filesystem/primitives/tests
- .atlas/temporary/donors/wasmtime/crates/wasi-config/tests
- .atlas/temporary/donors/wasmtime/crates/wasi-http/tests
- .atlas/temporary/donors/wasmtime/crates/wasi-keyvalue/tests
- .atlas/temporary/donors/wasmtime/crates/wasi-nn/tests
- .atlas/temporary/donors/wasmtime/crates/wasi-tls/tests
- .atlas/temporary/donors/wasmtime/crates/wasmtime/tests
- .atlas/temporary/donors/wasmtime/crates/wiggle/tests
- .atlas/temporary/donors/wasmtime/crates/wizer/benches
- .atlas/temporary/donors/wasmtime/crates/wizer/fuzz
- .atlas/temporary/donors/wasmtime/crates/wizer/tests
- .atlas/temporary/donors/wasmtime/pulley/fuzz
- .atlas/temporary/donors/wasmtime/pulley/tests

## Major Subsystem Roots

.agents, .claude, .github, benches, ci, cranelift, crates, docs, examples, fuzz, pulley, scripts, src, supply-chain, tests, winch

## License Evidence

- .atlas/licenses/donors/wasmtime/LICENSE

## Census State

Status: COARSE_CENSUSED. This is an admission-stage inventory only. Deep census must classify algorithms, invariants, state/effect boundaries, execution behavior, tests, benchmarks, rejected ideas, and Atlas-native replacement gaps before absorption.

## Native Replacement

runtime wasm backend/sandbox and Cranelift integration reference

## G100 — terminal EXTERNAL_BOUNDARY (+ REFERENCE_ONLY); source extinct

Both roles are crate-dependency boundaries with no consumer yet. Cranelift is a native backend behind the pipeline contract's LIR adapter, and Atlas has no LIR. A Wasm/WASI runtime is one candidate SandboxBackend, and no sandbox backend code exists. The donor's Cargo-census cases are locked by synthetic fixtures, and its dedicated real-donor test now follows the corpus ledger instead of silently returning. The checkout was physically deleted. Evidence: `../../evidence/campaign/36-wasmtime.json`.
