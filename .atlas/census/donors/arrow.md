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

- .atlas/temporary/donors/arrow/cpp/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/meson.build
- .atlas/temporary/donors/arrow/cpp/examples/arrow/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/examples/minimal_build/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/examples/parquet/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/examples/parquet/meson.build
- .atlas/temporary/donors/arrow/cpp/examples/parquet/parquet_arrow/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/examples/tutorial_examples/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/src/arrow/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/src/arrow/meson.build
- .atlas/temporary/donors/arrow/cpp/src/arrow/acero/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/src/arrow/acero/meson.build
- .atlas/temporary/donors/arrow/cpp/src/arrow/adapters/orc/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/src/arrow/adapters/tensorflow/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/src/arrow/adapters/tensorflow/meson.build
- .atlas/temporary/donors/arrow/cpp/src/arrow/array/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/src/arrow/array/meson.build
- .atlas/temporary/donors/arrow/cpp/src/arrow/c/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/src/arrow/c/meson.build
- .atlas/temporary/donors/arrow/cpp/src/arrow/compute/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/src/arrow/compute/meson.build
- .atlas/temporary/donors/arrow/cpp/src/arrow/compute/kernels/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/src/arrow/compute/kernels/meson.build
- .atlas/temporary/donors/arrow/cpp/src/arrow/compute/row/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/src/arrow/compute/row/meson.build
- .atlas/temporary/donors/arrow/cpp/src/arrow/csv/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/src/arrow/csv/meson.build
- .atlas/temporary/donors/arrow/cpp/src/arrow/dataset/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/src/arrow/dataset/meson.build
- .atlas/temporary/donors/arrow/cpp/src/arrow/engine/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/src/arrow/engine/meson.build
- .atlas/temporary/donors/arrow/cpp/src/arrow/engine/substrait/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/src/arrow/engine/substrait/meson.build
- .atlas/temporary/donors/arrow/cpp/src/arrow/extension/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/src/arrow/extension/meson.build
- .atlas/temporary/donors/arrow/cpp/src/arrow/filesystem/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/src/arrow/filesystem/meson.build
- .atlas/temporary/donors/arrow/cpp/src/arrow/flight/CMakeLists.txt
- .atlas/temporary/donors/arrow/cpp/src/arrow/flight/meson.build
- .atlas/temporary/donors/arrow/cpp/src/arrow/flight/integration_tests/CMakeLists.txt

## Test / Benchmark Roots Detected

- .atlas/temporary/donors/arrow/testing
- .atlas/temporary/donors/arrow/cpp/src/arrow/testing
- .atlas/temporary/donors/arrow/cpp/src/arrow/flight/sql/odbc/tests
- .atlas/temporary/donors/arrow/cpp/src/gandiva/tests
- .atlas/temporary/donors/arrow/c_glib/test
- .atlas/temporary/donors/arrow/dev/archery/archery/tests
- .atlas/temporary/donors/arrow/dev/archery/archery/crossbow/tests
- .atlas/temporary/donors/arrow/dev/archery/archery/docker/tests
- .atlas/temporary/donors/arrow/dev/archery/archery/release/tests
- .atlas/temporary/donors/arrow/matlab/test
- .atlas/temporary/donors/arrow/python/pyarrow/tests
- .atlas/temporary/donors/arrow/r/tests
- .atlas/temporary/donors/arrow/ruby/red-arrow/test
- .atlas/temporary/donors/arrow/ruby/red-arrow-cuda/test
- .atlas/temporary/donors/arrow/ruby/red-arrow-dataset/test
- .atlas/temporary/donors/arrow/ruby/red-arrow-flight/test
- .atlas/temporary/donors/arrow/ruby/red-arrow-flight-sql/test
- .atlas/temporary/donors/arrow/ruby/red-arrow-format/test
- .atlas/temporary/donors/arrow/ruby/red-gandiva/test
- .atlas/temporary/donors/arrow/ruby/red-parquet/test

## Major Subsystem Roots

.claude, .github, c_glib, ci, cpp, dev, docs, format, matlab, python, r, ruby, testing

## License Evidence

- .atlas/licenses/donors/arrow/LICENSE.txt
- .atlas/licenses/donors/arrow/NOTICE.txt

## Census State

Status: COARSE_CENSUSED. This is an admission-stage inventory only. Deep census must classify algorithms, invariants, state/effect boundaries, execution behavior, tests, benchmarks, rejected ideas, and Atlas-native replacement gaps before absorption.

## Native Replacement

atlas columnar evidence storage and high-performance data layout reference

## G88 — terminal REFERENCE_ONLY; source extinct

The recorded question was measured on the real census container. It holds 15,155 census facts at 107.5 bytes each, and 80% of those bytes are TLV framing: every 2–3 byte string-index varint carries an 8-byte field header. Columnar grouping shrinks the section 5.0x raw, but only 1.4x once zlib or xz is applied. It does not touch the string table, which is half the container, and nothing reads the container (a gitignored local cache). The largest census family, 81,080 typed records, is not stored in the container at all.

Arrow adds nothing beyond the generic columnar idea:
- its dictionary encoding is the existing string table;
- its IPC and flatbuffers framing duplicate the wire contract;
- record batches serve vectorized compute, which Atlas does not perform.

ATLAS-SEMANTIC-COMPACTION.md now records the measurement: codec compression comes first. The checkout was physically deleted. Evidence: `../../evidence/campaign/24-arrow.json`.
