---
id: donor-census-rust
type: reference
status: active
canonical: true
---
# Donor Census: Rust

## Source

- Remote: https://github.com/rust-lang/rust.git
- Repository: rust-lang/rust
- Commit: d287eb7a292caa8abbe12051dce2ee707fe03200
- Git tree: 42480d7fe225a464090d5f55839c4b49bdd4963c
- Branch: main
- Retrieved: 2026-09-21T03:00:34.9575636Z
- Staging mode: FULL_SOURCE_TREE
- Clone path: .atlas/temporary/donors/rust (corrected -- this record previously said `.atlas/temporary/rust`, which does not exist; verified against the real filesystem)

## Coarse Inventory

- Files observed: 62938
- Bytes observed: 226270581
- Languages/signals: Rust, Markdown, TOML, JavaScript, Shell, C
- Top-level directories: .github, compiler, library, LICENSES, src, tests
- Top-level files: .clang-format, .editorconfig, .git-blame-ignore-revs, .gitattributes, .gitignore, .gitmodules, .ignore, .mailmap, AGENTS.md, bootstrap.example.toml, Cargo.lock, Cargo.toml, CLAUDE.md, CODE_OF_CONDUCT.md, configure, CONTRIBUTING.md, COPYRIGHT, INSTALL.md, LICENSE-APACHE, license-metadata.json, LICENSE-MIT, package.json, README.md, RELEASES.md, REUSE.toml, rust-bors.toml, rustfmt.toml, triagebot.toml, typos.toml, x, x.ps1, x.py, yarn.lock

Full source tree staged with nested .git removed.

## Build Systems Detected

- .atlas/temporary/rust/Cargo.toml
- .atlas/temporary/rust/configure
- .atlas/temporary/rust/package.json
- .atlas/temporary/rust/compiler/rustc/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_abi/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_arena/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_ast/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_ast_ir/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_ast_lowering/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_ast_passes/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_ast_pretty/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_attr_ir/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_attr_parsing/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_baked_icu_data/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_borrowck/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_builtin_macros/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_codegen_cranelift/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_codegen_cranelift/build_system/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_codegen_gcc/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_codegen_gcc/build_system/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_codegen_gcc/tests/cross_lang_lto/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_codegen_gcc/tests/hello-world/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_codegen_gcc/tests/hello-world/mylib/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_codegen_llvm/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_codegen_ssa/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_const_eval/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_crate_store/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_data_structures/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_driver/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_driver_impl/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_errors/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_error_codes/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_error_messages/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_expand/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_expand_queries/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_feature/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_fs_util/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_graphviz/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_hashes/Cargo.toml
- .atlas/temporary/rust/compiler/rustc_hir/Cargo.toml

## Test / Benchmark Roots Detected

- .atlas/temporary/rust/tests
- .atlas/temporary/rust/compiler/rustc_codegen_gcc/tests
- .atlas/temporary/rust/compiler/rustc_codegen_gcc/build_system/src/fuzz
- .atlas/temporary/rust/compiler/rustc_errors/src/markdown/tests
- .atlas/temporary/rust/compiler/rustc_pattern_analysis/tests
- .atlas/temporary/rust/compiler/rustc_thread_pool/tests
- .atlas/temporary/rust/library/test
- .atlas/temporary/rust/library/alloctests/benches
- .atlas/temporary/rust/library/alloctests/testing
- .atlas/temporary/rust/library/alloctests/tests
- .atlas/temporary/rust/library/alloctests/tests/testing
- .atlas/temporary/rust/library/compiler-builtins/builtins-test/benches
- .atlas/temporary/rust/library/compiler-builtins/builtins-test/tests
- .atlas/temporary/rust/library/compiler-builtins/builtins-test-intrinsics/tests
- .atlas/temporary/rust/library/compiler-builtins/crates/libm-macros/tests
- .atlas/temporary/rust/library/compiler-builtins/crates/symcheck/tests
- .atlas/temporary/rust/library/compiler-builtins/crates/update-api-list/tests
- .atlas/temporary/rust/library/compiler-builtins/libm-test/benches
- .atlas/temporary/rust/library/compiler-builtins/libm-test/tests
- .atlas/temporary/rust/library/coretests/benches
- .atlas/temporary/rust/library/coretests/tests
- .atlas/temporary/rust/library/portable-simd/crates/core_simd/tests
- .atlas/temporary/rust/library/portable-simd/crates/std_float/tests
- .atlas/temporary/rust/library/std/benches
- .atlas/temporary/rust/library/std/tests
- .atlas/temporary/rust/library/stdarch/crates/stdarch-verify/tests
- .atlas/temporary/rust/library/std_detect/tests
- .atlas/temporary/rust/src/bootstrap/src/core/build_steps/test
- .atlas/temporary/rust/src/bootstrap/src/utils/tests
- .atlas/temporary/rust/src/ci/citool/tests
- .atlas/temporary/rust/src/doc/rustc/src/tests
- .atlas/temporary/rust/src/doc/rustc-dev-guide/ci/tests
- .atlas/temporary/rust/src/doc/rustc-dev-guide/src/tests
- .atlas/temporary/rust/src/tools/clippy/tests
- .atlas/temporary/rust/src/tools/clippy/tests/example_integration_test/tests
- .atlas/temporary/rust/src/tools/linkchecker/tests
- .atlas/temporary/rust/src/tools/miri/tests
- .atlas/temporary/rust/src/tools/miri/priroda/tests
- .atlas/temporary/rust/src/tools/miri/test-cargo-miri/tests
- .atlas/temporary/rust/src/tools/remote-test-client/tests

## Major Subsystem Roots

.github, compiler, library, LICENSES, src, tests

## License Evidence

- .atlas/licenses/donors/rust/COPYRIGHT
- .atlas/licenses/donors/rust/LICENSE-APACHE
- .atlas/licenses/donors/rust/license-metadata.json
- .atlas/licenses/donors/rust/LICENSE-MIT

## Census State

Status: COARSE_CENSUSED. This is an admission-stage inventory only. Deep census must classify algorithms, invariants, state/effect boundaries, execution behavior, tests, benchmarks, rejected ideas, and Atlas-native replacement gaps before absorption.

## Native Replacement

compiler roadmap rust parity oracle and AtlasX bootstrap reference
