---
id: donor-census-zstd
type: reference
status: active
canonical: true
---
# Donor Census: Zstandard

## Source

- Remote: https://github.com/facebook/zstd.git
- Repository: facebook/zstd
- Commit: 01b7154f1172432f8abe9b3bb9909e14a1176b7d
- Git tree: 7973821daa75236c9247c12881c39053a3a7cd47
- Branch: dev
- Retrieved: 2026-09-21T03:00:34.9575636Z
- Staging mode: FULL_SOURCE_TREE
- Clone path: .atlas/temporary/donors/zstd (corrected -- this record previously said `.atlas/temporary/zstd`, which does not exist; verified against the real filesystem)

## Coarse Inventory

- Files observed: 660
- Bytes observed: 8520156
- Languages/signals: C, C/C++ header, Shell, Markdown, C++, Python, YAML, CMake
- Top-level directories: .github, build, contrib, doc, examples, lib, programs, tests, zlibWrapper
- Top-level files: .buckconfig, .buckversion, .cirrus.yml, .gitattributes, .gitignore, CHANGELOG, CMakeLists.txt, CODE_OF_CONDUCT.md, CONTRIBUTING.md, COPYING, LICENSE, Makefile, Package.swift, README.md, SECURITY.md, TESTING.md

Full source tree staged with nested .git removed.

## Build Systems Detected

- .atlas/temporary/donors/zstd/CMakeLists.txt
- .atlas/temporary/donors/zstd/Makefile
- .atlas/temporary/donors/zstd/build/cmake/CMakeLists.txt
- .atlas/temporary/donors/zstd/build/cmake/contrib/CMakeLists.txt
- .atlas/temporary/donors/zstd/build/cmake/contrib/gen_html/CMakeLists.txt
- .atlas/temporary/donors/zstd/build/cmake/contrib/pzstd/CMakeLists.txt
- .atlas/temporary/donors/zstd/build/cmake/lib/CMakeLists.txt
- .atlas/temporary/donors/zstd/build/cmake/programs/CMakeLists.txt
- .atlas/temporary/donors/zstd/build/cmake/tests/CMakeLists.txt
- .atlas/temporary/donors/zstd/build/meson/meson.build
- .atlas/temporary/donors/zstd/build/meson/contrib/meson.build
- .atlas/temporary/donors/zstd/build/meson/contrib/gen_html/meson.build
- .atlas/temporary/donors/zstd/build/meson/contrib/pzstd/meson.build
- .atlas/temporary/donors/zstd/build/meson/lib/meson.build
- .atlas/temporary/donors/zstd/build/meson/programs/meson.build
- .atlas/temporary/donors/zstd/build/meson/tests/meson.build
- .atlas/temporary/donors/zstd/contrib/diagnose_corruption/Makefile
- .atlas/temporary/donors/zstd/contrib/externalSequenceProducer/Makefile
- .atlas/temporary/donors/zstd/contrib/gen_html/Makefile
- .atlas/temporary/donors/zstd/contrib/largeNbDicts/Makefile
- .atlas/temporary/donors/zstd/contrib/linux-kernel/Makefile
- .atlas/temporary/donors/zstd/contrib/linux-kernel/test/Makefile
- .atlas/temporary/donors/zstd/contrib/pzstd/Makefile
- .atlas/temporary/donors/zstd/contrib/recovery/Makefile
- .atlas/temporary/donors/zstd/contrib/seekable_format/examples/Makefile
- .atlas/temporary/donors/zstd/contrib/seekable_format/tests/Makefile
- .atlas/temporary/donors/zstd/contrib/seqBench/Makefile
- .atlas/temporary/donors/zstd/doc/educational_decoder/Makefile
- .atlas/temporary/donors/zstd/examples/Makefile
- .atlas/temporary/donors/zstd/lib/Makefile
- .atlas/temporary/donors/zstd/lib/dll/example/Makefile
- .atlas/temporary/donors/zstd/programs/Makefile
- .atlas/temporary/donors/zstd/tests/Makefile
- .atlas/temporary/donors/zstd/tests/fuzz/Makefile
- .atlas/temporary/donors/zstd/tests/fuzz/seq_prod_fuzz_example/Makefile
- .atlas/temporary/donors/zstd/tests/gzip/Makefile
- .atlas/temporary/donors/zstd/tests/regression/Makefile
- .atlas/temporary/donors/zstd/zlibWrapper/Makefile

## Test / Benchmark Roots Detected

- .atlas/temporary/donors/zstd/tests
- .atlas/temporary/donors/zstd/build/cmake/tests
- .atlas/temporary/donors/zstd/build/meson/tests
- .atlas/temporary/donors/zstd/contrib/linux-kernel/test
- .atlas/temporary/donors/zstd/contrib/pzstd/test
- .atlas/temporary/donors/zstd/contrib/pzstd/utils/test
- .atlas/temporary/donors/zstd/contrib/seekable_format/tests
- .atlas/temporary/donors/zstd/tests/fuzz

## Major Subsystem Roots

.github, build, contrib, doc, examples, lib, programs, tests, zlibWrapper

## License Evidence

- .atlas/licenses/donors/zstd/COPYING
- .atlas/licenses/donors/zstd/LICENSE

## Census State

Status: COARSE_CENSUSED. This is an admission-stage inventory only. Deep census must classify algorithms, invariants, state/effect boundaries, execution behavior, tests, benchmarks, rejected ideas, and Atlas-native replacement gaps before absorption.

## Native Replacement

runtime/storage compression and atlas shard compression primitives
