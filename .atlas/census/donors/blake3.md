---
id: donor-census-blake3
type: reference
status: active
canonical: true
---
# Donor Census: BLAKE3

## Source

- Remote: https://github.com/BLAKE3-team/BLAKE3.git
- Repository: BLAKE3-team/BLAKE3
- Commit: 6aab490a26124663329dfd3961b8469f8fdb158b
- Git tree: 9f05f8d64c9c2d64965f60d4f72a40852de4e072
- Branch: master
- Retrieved: 2026-09-21T03:00:34.9575636Z
- Staging mode: FULL_SOURCE_TREE
- Clone path: .atlas/temporary/donors/blake3 (corrected -- this record previously said
  `.atlas/temporary/blake3`, which does not exist; verified against the real filesystem while
  building `.atlas/genome/technology/blake3-content-addressing.md`)

## Coarse Inventory

- Files observed: 108
- Bytes observed: 1882379
- Languages/signals: Rust, C, TOML, Assembly, Markdown, CMake, Python, Shell, YAML, C/C++ header
- Top-level directories: .cargo, .github, b3sum, benches, c, media, reference_impl, src, test_vectors, tools
- Top-level files: .git-blame-ignore-revs, .gitignore, build.rs, Cargo.toml, CONTRIBUTING.md, LICENSE_A2, LICENSE_A2LLVM, LICENSE_CC0, README.md

Full source tree staged with nested .git removed.

## Build Systems Detected

- .atlas/temporary/blake3/Cargo.toml
- .atlas/temporary/blake3/b3sum/Cargo.toml
- .atlas/temporary/blake3/c/CMakeLists.txt
- .atlas/temporary/blake3/c/blake3_c_rust_bindings/Cargo.toml
- .atlas/temporary/blake3/c/dependencies/CMakeLists.txt
- .atlas/temporary/blake3/c/dependencies/tbb/CMakeLists.txt
- .atlas/temporary/blake3/reference_impl/Cargo.toml
- .atlas/temporary/blake3/test_vectors/Cargo.toml
- .atlas/temporary/blake3/tools/compiler_version/Cargo.toml
- .atlas/temporary/blake3/tools/instruction_set_support/Cargo.toml

## Test / Benchmark Roots Detected

- .atlas/temporary/blake3/benches
- .atlas/temporary/blake3/b3sum/tests
- .atlas/temporary/blake3/c/blake3_c_rust_bindings/benches

## Major Subsystem Roots

.cargo, .github, b3sum, benches, c, media, reference_impl, src, test_vectors, tools

## License Evidence

- .atlas/licenses/donors/blake3/LICENSE_A2
- .atlas/licenses/donors/blake3/LICENSE_A2LLVM
- .atlas/licenses/donors/blake3/LICENSE_CC0

## Census State

Status: COARSE_CENSUSED. This is an admission-stage inventory only. Deep census must classify algorithms, invariants, state/effect boundaries, execution behavior, tests, benchmarks, rejected ideas, and Atlas-native replacement gaps before absorption.

## Native Replacement

core identity/content-addressed hash strategy and shard integrity evidence
