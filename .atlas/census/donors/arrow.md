---
id: donor-census-arrow
type: reference
status: active
canonical: true
---
# Donor Census: Apache Arrow

## Source

- Remote: https://github.com/apache/arrow.git
- Repository: apache/arrow
- Commit: 70f5c269ae8d154f57f7a7ec6babe6e0685ed6ec
- Git tree: 8dcd79b2f5b2f745642541fdbe8177beeb1daf0b
- Branch: main
- Retrieved: 2026-09-21T03:00:34.9575636Z
- Staging mode: FULL_SOURCE_TREE
- Clone path: .atlas/temporary/donors/arrow (corrected -- this record previously said `.atlas/temporary/arrow`, which does not exist; verified against the real filesystem)

## Coarse Inventory

- Files observed: 5328
- Bytes observed: 67295797
- Languages/signals: C++, C/C++ header, Python, Shell, Markdown, YAML, C++ header, CMake
- Top-level directories: .claude, .github, c_glib, ci, cpp, dev, docs, format, matlab, python, r, ruby, testing
- Top-level files: .asf.yaml, .clang-format, .clang-tidy, .clang-tidy-ignore, .dockerignore, .editorconfig, .env, .gitattributes, .gitignore, .gitmodules, .hadolint.yaml, .pre-commit-config.yaml, .rubocop.yml, .shellcheckrc, CHANGELOG.md, cmake-format.py, CODE_OF_CONDUCT.md, compose.yaml, CONTRIBUTING.md, CPPLINT.cfg, LICENSE.txt, NOTICE.txt, README.md

Full source tree staged with nested .git removed.

## Build Systems Detected

- .atlas/temporary/arrow/cpp/CMakeLists.txt
- .atlas/temporary/arrow/cpp/meson.build
- .atlas/temporary/arrow/cpp/examples/arrow/CMakeLists.txt
- .atlas/temporary/arrow/cpp/examples/minimal_build/CMakeLists.txt
- .atlas/temporary/arrow/cpp/examples/parquet/CMakeLists.txt
- .atlas/temporary/arrow/cpp/examples/parquet/meson.build
- .atlas/temporary/arrow/cpp/examples/parquet/parquet_arrow/CMakeLists.txt
- .atlas/temporary/arrow/cpp/examples/tutorial_examples/CMakeLists.txt
- .atlas/temporary/arrow/cpp/src/arrow/CMakeLists.txt
- .atlas/temporary/arrow/cpp/src/arrow/meson.build
- .atlas/temporary/arrow/cpp/src/arrow/acero/CMakeLists.txt
- .atlas/temporary/arrow/cpp/src/arrow/acero/meson.build
- .atlas/temporary/arrow/cpp/src/arrow/adapters/orc/CMakeLists.txt
- .atlas/temporary/arrow/cpp/src/arrow/adapters/tensorflow/CMakeLists.txt
- .atlas/temporary/arrow/cpp/src/arrow/adapters/tensorflow/meson.build
- .atlas/temporary/arrow/cpp/src/arrow/array/CMakeLists.txt
- .atlas/temporary/arrow/cpp/src/arrow/array/meson.build
- .atlas/temporary/arrow/cpp/src/arrow/c/CMakeLists.txt
- .atlas/temporary/arrow/cpp/src/arrow/c/meson.build
- .atlas/temporary/arrow/cpp/src/arrow/compute/CMakeLists.txt
- .atlas/temporary/arrow/cpp/src/arrow/compute/meson.build
- .atlas/temporary/arrow/cpp/src/arrow/compute/kernels/CMakeLists.txt
- .atlas/temporary/arrow/cpp/src/arrow/compute/kernels/meson.build
- .atlas/temporary/arrow/cpp/src/arrow/compute/row/CMakeLists.txt
- .atlas/temporary/arrow/cpp/src/arrow/compute/row/meson.build
- .atlas/temporary/arrow/cpp/src/arrow/csv/CMakeLists.txt
- .atlas/temporary/arrow/cpp/src/arrow/csv/meson.build
- .atlas/temporary/arrow/cpp/src/arrow/dataset/CMakeLists.txt
- .atlas/temporary/arrow/cpp/src/arrow/dataset/meson.build
- .atlas/temporary/arrow/cpp/src/arrow/engine/CMakeLists.txt
- .atlas/temporary/arrow/cpp/src/arrow/engine/meson.build
- .atlas/temporary/arrow/cpp/src/arrow/engine/substrait/CMakeLists.txt
- .atlas/temporary/arrow/cpp/src/arrow/engine/substrait/meson.build
- .atlas/temporary/arrow/cpp/src/arrow/extension/CMakeLists.txt
- .atlas/temporary/arrow/cpp/src/arrow/extension/meson.build
- .atlas/temporary/arrow/cpp/src/arrow/filesystem/CMakeLists.txt
- .atlas/temporary/arrow/cpp/src/arrow/filesystem/meson.build
- .atlas/temporary/arrow/cpp/src/arrow/flight/CMakeLists.txt
- .atlas/temporary/arrow/cpp/src/arrow/flight/meson.build
- .atlas/temporary/arrow/cpp/src/arrow/flight/integration_tests/CMakeLists.txt

## Test / Benchmark Roots Detected

- .atlas/temporary/arrow/testing
- .atlas/temporary/arrow/cpp/src/arrow/testing
- .atlas/temporary/arrow/cpp/src/arrow/flight/sql/odbc/tests
- .atlas/temporary/arrow/cpp/src/gandiva/tests
- .atlas/temporary/arrow/c_glib/test
- .atlas/temporary/arrow/dev/archery/archery/tests
- .atlas/temporary/arrow/dev/archery/archery/crossbow/tests
- .atlas/temporary/arrow/dev/archery/archery/docker/tests
- .atlas/temporary/arrow/dev/archery/archery/release/tests
- .atlas/temporary/arrow/matlab/test
- .atlas/temporary/arrow/python/pyarrow/tests
- .atlas/temporary/arrow/r/tests
- .atlas/temporary/arrow/ruby/red-arrow/test
- .atlas/temporary/arrow/ruby/red-arrow-cuda/test
- .atlas/temporary/arrow/ruby/red-arrow-dataset/test
- .atlas/temporary/arrow/ruby/red-arrow-flight/test
- .atlas/temporary/arrow/ruby/red-arrow-flight-sql/test
- .atlas/temporary/arrow/ruby/red-arrow-format/test
- .atlas/temporary/arrow/ruby/red-gandiva/test
- .atlas/temporary/arrow/ruby/red-parquet/test

## Major Subsystem Roots

.claude, .github, c_glib, ci, cpp, dev, docs, format, matlab, python, r, ruby, testing

## License Evidence

- .atlas/licenses/donors/arrow/LICENSE.txt
- .atlas/licenses/donors/arrow/NOTICE.txt

## Census State

Status: COARSE_CENSUSED. This is an admission-stage inventory only. Deep census must classify algorithms, invariants, state/effect boundaries, execution behavior, tests, benchmarks, rejected ideas, and Atlas-native replacement gaps before absorption.

## Native Replacement

atlas columnar evidence storage and high-performance data layout reference
