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

- .atlas/temporary/donors/rust/Cargo.toml
- .atlas/temporary/donors/rust/configure
- .atlas/temporary/donors/rust/package.json
- .atlas/temporary/donors/rust/compiler/rustc/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_abi/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_arena/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_ast/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_ast_ir/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_ast_lowering/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_ast_passes/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_ast_pretty/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_attr_ir/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_attr_parsing/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_baked_icu_data/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_borrowck/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_builtin_macros/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_codegen_cranelift/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_codegen_cranelift/build_system/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_codegen_gcc/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_codegen_gcc/build_system/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_codegen_gcc/tests/cross_lang_lto/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_codegen_gcc/tests/hello-world/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_codegen_gcc/tests/hello-world/mylib/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_codegen_llvm/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_codegen_ssa/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_const_eval/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_crate_store/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_data_structures/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_driver/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_driver_impl/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_errors/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_error_codes/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_error_messages/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_expand/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_expand_queries/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_feature/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_fs_util/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_graphviz/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_hashes/Cargo.toml
- .atlas/temporary/donors/rust/compiler/rustc_hir/Cargo.toml

## Test / Benchmark Roots Detected

- .atlas/temporary/donors/rust/tests
- .atlas/temporary/donors/rust/compiler/rustc_codegen_gcc/tests
- .atlas/temporary/donors/rust/compiler/rustc_codegen_gcc/build_system/src/fuzz
- .atlas/temporary/donors/rust/compiler/rustc_errors/src/markdown/tests
- .atlas/temporary/donors/rust/compiler/rustc_pattern_analysis/tests
- .atlas/temporary/donors/rust/compiler/rustc_thread_pool/tests
- .atlas/temporary/donors/rust/library/test
- .atlas/temporary/donors/rust/library/alloctests/benches
- .atlas/temporary/donors/rust/library/alloctests/testing
- .atlas/temporary/donors/rust/library/alloctests/tests
- .atlas/temporary/donors/rust/library/alloctests/tests/testing
- .atlas/temporary/donors/rust/library/compiler-builtins/builtins-test/benches
- .atlas/temporary/donors/rust/library/compiler-builtins/builtins-test/tests
- .atlas/temporary/donors/rust/library/compiler-builtins/builtins-test-intrinsics/tests
- .atlas/temporary/donors/rust/library/compiler-builtins/crates/libm-macros/tests
- .atlas/temporary/donors/rust/library/compiler-builtins/crates/symcheck/tests
- .atlas/temporary/donors/rust/library/compiler-builtins/crates/update-api-list/tests
- .atlas/temporary/donors/rust/library/compiler-builtins/libm-test/benches
- .atlas/temporary/donors/rust/library/compiler-builtins/libm-test/tests
- .atlas/temporary/donors/rust/library/coretests/benches
- .atlas/temporary/donors/rust/library/coretests/tests
- .atlas/temporary/donors/rust/library/portable-simd/crates/core_simd/tests
- .atlas/temporary/donors/rust/library/portable-simd/crates/std_float/tests
- .atlas/temporary/donors/rust/library/std/benches
- .atlas/temporary/donors/rust/library/std/tests
- .atlas/temporary/donors/rust/library/stdarch/crates/stdarch-verify/tests
- .atlas/temporary/donors/rust/library/std_detect/tests
- .atlas/temporary/donors/rust/src/bootstrap/src/core/build_steps/test
- .atlas/temporary/donors/rust/src/bootstrap/src/utils/tests
- .atlas/temporary/donors/rust/src/ci/citool/tests
- .atlas/temporary/donors/rust/src/doc/rustc/src/tests
- .atlas/temporary/donors/rust/src/doc/rustc-dev-guide/ci/tests
- .atlas/temporary/donors/rust/src/doc/rustc-dev-guide/src/tests
- .atlas/temporary/donors/rust/src/tools/clippy/tests
- .atlas/temporary/donors/rust/src/tools/clippy/tests/example_integration_test/tests
- .atlas/temporary/donors/rust/src/tools/linkchecker/tests
- .atlas/temporary/donors/rust/src/tools/miri/tests
- .atlas/temporary/donors/rust/src/tools/miri/priroda/tests
- .atlas/temporary/donors/rust/src/tools/miri/test-cargo-miri/tests
- .atlas/temporary/donors/rust/src/tools/remote-test-client/tests

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

## Campaign decision (G73, first-50 #14)

Measured: Atlas resolves **0 of 12,671** call sites. The pinned toolchain's `rust-analyzer scip` (1.90.0, same distribution as rustc) resolves a callable on the call's line for about **89%** of them. Stable rustc exposes no machine-readable resolution.

Terminal: **EXTERNAL_BOUNDARY**. Compiler-grade resolution comes from the toolchain oracle; it is never a hidden runtime dependency, and an absent tool means an absent engine, recorded as such. rustc resolver, typeck and borrowck internals are REFERENCE_ONLY. Integration is G74 (P0).

The checkout (62,925 files, 426 MB) was physically deleted; LLVM was accounted, never cloned. Evidence: `../../evidence/campaign/14-rust.json`.

