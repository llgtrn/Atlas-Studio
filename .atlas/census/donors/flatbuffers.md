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
- Clone path: .atlas/temporary/flatbuffers

## Coarse Inventory

- Files observed: 1947
- Bytes observed: 13950171
- Languages/signals: TypeScript, Rust, Python, C/C++ header, Java, FlatBuffers schema, JavaScript, C++, Markdown, Go, Shell
- Top-level directories: .bazelci, .github, android, bazel, benchmarks, CMake, dart, docs, examples, go, goldens, grpc, include, java, js, kotlin, lobster, lua, mjs, net, nim, php, python, reflection, rust, samples, scripts, snap, src, swift, tests, ts
- Top-level files: .bazelignore, .bazelrc, .clang-format, .clang-tidy, .editorconfig, .gitattributes, .gitignore, .npmrc, build_defs.bzl, BUILD.bazel, CHANGELOG.md, CMakeLists.txt, composer.json, CONTRIBUTING.md, eslint.config.mjs, extensions.bzl, FlatBuffers.podspec, Formatters.md, library.json, LICENSE, MODULE.bazel, package.json, Package.swift, pnpm-lock.yaml, pnpm-workspace.yaml, README.md, SECURITY.md, swift.swiftformat, tsconfig.json, tsconfig.mjs.json, typescript.bzl

Full source tree staged with nested .git removed.

## Build Systems Detected

- .atlas/temporary/flatbuffers/BUILD.bazel
- .atlas/temporary/flatbuffers/CMakeLists.txt
- .atlas/temporary/flatbuffers/MODULE.bazel
- .atlas/temporary/flatbuffers/package.json
- .atlas/temporary/flatbuffers/android/build.gradle
- .atlas/temporary/flatbuffers/android/app/build.gradle
- .atlas/temporary/flatbuffers/android/app/src/main/cpp/CMakeLists.txt
- .atlas/temporary/flatbuffers/android/app/src/main/cpp/flatbuffers/CMakeLists.txt
- .atlas/temporary/flatbuffers/bazel/BUILD.bazel
- .atlas/temporary/flatbuffers/benchmarks/CMakeLists.txt
- .atlas/temporary/flatbuffers/go/BUILD.bazel
- .atlas/temporary/flatbuffers/grpc/BUILD.bazel
- .atlas/temporary/flatbuffers/grpc/pom.xml
- .atlas/temporary/flatbuffers/grpc/examples/ts/greeter/package.json
- .atlas/temporary/flatbuffers/grpc/flatbuffers-java-grpc/pom.xml
- .atlas/temporary/flatbuffers/grpc/samples/greeter/Makefile
- .atlas/temporary/flatbuffers/grpc/src/compiler/BUILD.bazel
- .atlas/temporary/flatbuffers/grpc/tests/BUILD
- .atlas/temporary/flatbuffers/grpc/tests/pom.xml
- .atlas/temporary/flatbuffers/include/codegen/BUILD.bazel
- .atlas/temporary/flatbuffers/java/pom.xml
- .atlas/temporary/flatbuffers/kotlin/build.gradle.kts
- .atlas/temporary/flatbuffers/kotlin/benchmark/build.gradle.kts
- .atlas/temporary/flatbuffers/kotlin/convention-plugins/build.gradle.kts
- .atlas/temporary/flatbuffers/kotlin/flatbuffers-kotlin/build.gradle.kts
- .atlas/temporary/flatbuffers/reflection/BUILD.bazel
- .atlas/temporary/flatbuffers/reflection/ts/BUILD.bazel
- .atlas/temporary/flatbuffers/rust/flatbuffers/Cargo.toml
- .atlas/temporary/flatbuffers/rust/flexbuffers/Cargo.toml
- .atlas/temporary/flatbuffers/rust/reflection/Cargo.toml
- .atlas/temporary/flatbuffers/src/BUILD.bazel
- .atlas/temporary/flatbuffers/swift/BUILD.bazel
- .atlas/temporary/flatbuffers/tests/BUILD.bazel
- .atlas/temporary/flatbuffers/tests/bazel_repository_test_dir/BUILD
- .atlas/temporary/flatbuffers/tests/bazel_repository_test_dir/MODULE.bazel
- .atlas/temporary/flatbuffers/tests/fuzzer/CMakeLists.txt
- .atlas/temporary/flatbuffers/tests/rust_no_std_compilation_test/Cargo.toml
- .atlas/temporary/flatbuffers/tests/rust_reflection_test/Cargo.toml
- .atlas/temporary/flatbuffers/tests/rust_serialize_test/Cargo.toml
- .atlas/temporary/flatbuffers/tests/rust_usage_test/Cargo.toml

## Test / Benchmark Roots Detected

- .atlas/temporary/flatbuffers/tests
- .atlas/temporary/flatbuffers/dart/test
- .atlas/temporary/flatbuffers/grpc/tests
- .atlas/temporary/flatbuffers/java/src/test
- .atlas/temporary/flatbuffers/tests/annotated_binary/tests
- .atlas/temporary/flatbuffers/tests/nim/tests
- .atlas/temporary/flatbuffers/tests/rust_usage_test/benches
- .atlas/temporary/flatbuffers/tests/rust_usage_test/tests
- .atlas/temporary/flatbuffers/tests/swift/Tests
- .atlas/temporary/flatbuffers/tests/swift/Wasm.tests/Tests
- .atlas/temporary/flatbuffers/tests/ts/com/company/test

## Major Subsystem Roots

.bazelci, .github, android, bazel, benchmarks, CMake, dart, docs, examples, go, goldens, grpc, include, java, js, kotlin, lobster, lua, mjs, net, nim, php, python, reflection, rust, samples, scripts, snap, src, swift, tests, ts

## License Evidence

- .atlas/licenses/donors/flatbuffers/LICENSE

## Census State

Status: COARSE_CENSUSED. This is an admission-stage inventory only. Deep census must classify algorithms, invariants, state/effect boundaries, execution behavior, tests, benchmarks, rejected ideas, and Atlas-native replacement gaps before absorption.

## Native Replacement

atlas binary schema/layout lessons and deterministic interchange boundaries
