---
id: donor-census-llvm-project
type: reference
status: active
canonical: true
---
# Donor Census: LLVM Project

## Source

- Remote: https://github.com/llvm/llvm-project.git
- Repository: llvm/llvm-project
- Commit: 0bd330675f9eb08126e467505a0800f167084473
- Git tree: 7989848d07457d796890b09d4c7b0f34219e7dba
- Branch: main
- Retrieved: 2026-09-21T03:00:34.9575636Z
- Staging mode: FULL_SOURCE_TREE
- Clone path: .atlas/temporary/donors/llvm-project

## Coarse Inventory

- Files observed: 184821
- Bytes observed: 2410758439
- Languages/signals: C++, C/C++ header, C, TableGen, MLIR, Python, CMake, Markdown, YAML, Shell, LLVM IR
- Top-level directories: .ci, .github, bolt, clang, clang-tools-extra, cmake, compiler-rt, cross-project-tests, flang, flang-rt, libc, libclc, libcxx, libcxxabi, libsycl, libunwind, lld, lldb, llvm, llvm-libgcc, mlir, offload, openmp, orc-rt, polly, runtimes, third-party, utils
- Top-level files: .clang-format, .clang-format-ignore, .clang-tidy, .git-blame-ignore-revs, .gitattributes, .gitignore, .mailmap, CODE_OF_CONDUCT.md, CONTRIBUTING.md, LICENSE.TXT, pyproject.toml, README.md, SECURITY.md

Full source checkout is physically available in the donor workbench and pinned to the recorded commit. The donor `.git` metadata is retained locally for this audit wave because `.atlas/temporary/donors/` is ignored and not staged into the Atlas commit.

## Build Systems Detected

- .atlas/temporary/donors/llvm-project/bolt/CMakeLists.txt
- .atlas/temporary/donors/llvm-project/clang/CMakeLists.txt
- .atlas/temporary/donors/llvm-project/clang-tools-extra/CMakeLists.txt
- .atlas/temporary/donors/llvm-project/compiler-rt/CMakeLists.txt
- .atlas/temporary/donors/llvm-project/flang/CMakeLists.txt
- .atlas/temporary/donors/llvm-project/lld/CMakeLists.txt
- .atlas/temporary/donors/llvm-project/lldb/CMakeLists.txt
- .atlas/temporary/donors/llvm-project/llvm/CMakeLists.txt
- .atlas/temporary/donors/llvm-project/mlir/CMakeLists.txt
- .atlas/temporary/donors/llvm-project/openmp/CMakeLists.txt

## Test / Benchmark Roots Detected

- .atlas/temporary/donors/llvm-project/bolt/test
- .atlas/temporary/donors/llvm-project/bolt/unittests
- .atlas/temporary/donors/llvm-project/clang/test
- .atlas/temporary/donors/llvm-project/clang/unittests
- .atlas/temporary/donors/llvm-project/clang-tools-extra/test
- .atlas/temporary/donors/llvm-project/clang-tools-extra/unittests
- .atlas/temporary/donors/llvm-project/compiler-rt/test
- .atlas/temporary/donors/llvm-project/compiler-rt/unittests
- .atlas/temporary/donors/llvm-project/flang/test
- .atlas/temporary/donors/llvm-project/flang/unittests

## Major Subsystem Roots

.ci, .github, bolt, clang, clang-tools-extra, cmake, compiler-rt, cross-project-tests, flang, flang-rt, libc, libclc, libcxx, libcxxabi, libsycl, libunwind, lld, lldb, llvm, llvm-libgcc, mlir, offload, openmp, orc-rt, polly, runtimes, third-party, utils

## License Evidence

- .atlas/licenses/donors/llvm-project/LICENSE.TXT

## Census State

Status: COARSE_CENSUSED. This is an admission-stage inventory only. Deep census must classify algorithms, invariants, state/effect boundaries, execution behavior, tests, benchmarks, rejected ideas, and Atlas-native replacement gaps before absorption.

## Native Replacement

external native backend roadmap and differential codegen oracle
