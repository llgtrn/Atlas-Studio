---
id: donor-census-wasm-tools
type: reference
status: active
canonical: true
---
# Donor Census: Wasm Tools

## Source

- Remote: https://github.com/bytecodealliance/wasm-tools.git
- Repository: bytecodealliance/wasm-tools
- Commit: 5b9827a7eabc4e365c6e15ed2ab5059a3db8b215
- Git tree: 0f5620ec63d2dbe9de8d67714509867f7201af99
- Branch: main
- Retrieved: 2026-09-21T03:00:34.9575636Z
- Staging mode: FULL_SOURCE_TREE
- Clone path: .atlas/temporary/wasm-tools

## Coarse Inventory

- Files observed: 10168
- Bytes observed: 62455513
- Languages/signals: WIT, WAT, Rust, TOML, Markdown, YAML
- Top-level directories: .github, ci, crates, examples, fuzz, playground, src, tests
- Top-level files: .gitattributes, .gitignore, .gitmodules, build.rs, Cargo.lock, Cargo.toml, CODE_OF_CONDUCT.md, CODEOWNERS, CONTRIBUTING.md, LICENSE-APACHE, LICENSE-Apache-2.0_WITH_LLVM-exception, LICENSE-MIT, ORG_CODE_OF_CONDUCT.md, README.md, rustfmt.toml, SECURITY.md

Full source tree staged with nested .git removed.

## Build Systems Detected

- .atlas/temporary/wasm-tools/Cargo.toml
- .atlas/temporary/wasm-tools/crates/c-api/Cargo.toml
- .atlas/temporary/wasm-tools/crates/c-api/CMakeLists.txt
- .atlas/temporary/wasm-tools/crates/fuzz-stats/Cargo.toml
- .atlas/temporary/wasm-tools/crates/json-from-wast/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wasm-compose/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wasm-encoder/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wasm-metadata/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wasm-mutate/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wasm-mutate-stats/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wasm-shrink/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wasm-smith/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wasm-wave/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wasm-wave/contrib/vscode/package.json
- .atlas/temporary/wasm-tools/crates/wasmparser/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wasmprinter/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wast/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wat/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wit-component/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wit-component/dl/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wit-dylib/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wit-dylib/ffi/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wit-dylib/test-programs/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wit-dylib/test-programs/artifacts/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wit-encoder/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wit-parser/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wit-parser/fuzz/Cargo.toml
- .atlas/temporary/wasm-tools/crates/wit-smith/Cargo.toml
- .atlas/temporary/wasm-tools/examples/CMakeLists.txt
- .atlas/temporary/wasm-tools/fuzz/Cargo.toml
- .atlas/temporary/wasm-tools/playground/package.json
- .atlas/temporary/wasm-tools/playground/component/Cargo.toml

## Test / Benchmark Roots Detected

- .atlas/temporary/wasm-tools/fuzz
- .atlas/temporary/wasm-tools/tests
- .atlas/temporary/wasm-tools/crates/wasm-compose/tests
- .atlas/temporary/wasm-tools/crates/wasm-metadata/tests
- .atlas/temporary/wasm-tools/crates/wasm-mutate/tests
- .atlas/temporary/wasm-tools/crates/wasm-shrink/tests
- .atlas/temporary/wasm-tools/crates/wasm-smith/benches
- .atlas/temporary/wasm-tools/crates/wasm-smith/tests
- .atlas/temporary/wasm-tools/crates/wasm-wave/tests
- .atlas/temporary/wasm-tools/crates/wasmparser/benches
- .atlas/temporary/wasm-tools/crates/wasmparser/tests
- .atlas/temporary/wasm-tools/crates/wasmprinter/tests
- .atlas/temporary/wasm-tools/crates/wast/tests
- .atlas/temporary/wasm-tools/crates/wit-component/tests
- .atlas/temporary/wasm-tools/crates/wit-dylib/tests
- .atlas/temporary/wasm-tools/crates/wit-encoder/tests
- .atlas/temporary/wasm-tools/crates/wit-parser/fuzz
- .atlas/temporary/wasm-tools/crates/wit-parser/tests
- .atlas/temporary/wasm-tools/tests/snapshots/component-model/test

## Major Subsystem Roots

.github, ci, crates, examples, fuzz, playground, src, tests

## License Evidence

- .atlas/licenses/donors/wasm-tools/LICENSE-APACHE
- .atlas/licenses/donors/wasm-tools/LICENSE-Apache-2.0_WITH_LLVM-exception
- .atlas/licenses/donors/wasm-tools/LICENSE-MIT

## Census State

Status: COARSE_CENSUSED. This is an admission-stage inventory only. Deep census must classify algorithms, invariants, state/effect boundaries, execution behavior, tests, benchmarks, rejected ideas, and Atlas-native replacement gaps before absorption.

## Native Replacement

adapter/exchange wasm encoding validation and component-model tooling
