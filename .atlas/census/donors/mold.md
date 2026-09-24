---
id: donor-census-mold
type: reference
status: active
canonical: true
---
# Donor Census: mold

## Source

- Remote: https://github.com/rui314/mold.git
- Repository: rui314/mold
- Commit: 89a9b334e8a7ddda367adf05591342b2766eb1e7
- Git tree: f2f8a0e22a34573c503540577b759831f0b6de96
- Branch: main
- Retrieved: 2026-09-21T03:00:34.9575636Z
- Staging mode: FULL_SOURCE_TREE
- Clone path: .atlas/temporary/donors/mold (corrected -- this record previously said `.atlas/temporary/mold`, which does not exist; verified against the real filesystem)

## Coarse Inventory

- Files observed: 741
- Bytes observed: 2433593
- Languages/signals: Shell, Rust, TOML, YAML, Markdown, C
- Top-level directories: .github, arch, c, cli, docs, src, tests
- Top-level files: .gitignore, build.rs, Cargo.lock, Cargo.toml, dist.sh, install-mold.sh, install-test-deps.sh, LICENSE, README.md, rust-toolchain.toml, rustfmt.toml

Full source tree staged with nested .git removed.

## Build Systems Detected

- .atlas/temporary/donors/mold/Cargo.toml
- .atlas/temporary/donors/mold/arch/arm32/Cargo.toml
- .atlas/temporary/donors/mold/arch/arm32be/Cargo.toml
- .atlas/temporary/donors/mold/arch/arm64/Cargo.toml
- .atlas/temporary/donors/mold/arch/arm64be/Cargo.toml
- .atlas/temporary/donors/mold/arch/i386/Cargo.toml
- .atlas/temporary/donors/mold/arch/loongarch32/Cargo.toml
- .atlas/temporary/donors/mold/arch/loongarch64/Cargo.toml
- .atlas/temporary/donors/mold/arch/m68k/Cargo.toml
- .atlas/temporary/donors/mold/arch/ppc32/Cargo.toml
- .atlas/temporary/donors/mold/arch/ppc64v1/Cargo.toml
- .atlas/temporary/donors/mold/arch/ppc64v2/Cargo.toml
- .atlas/temporary/donors/mold/arch/riscv32/Cargo.toml
- .atlas/temporary/donors/mold/arch/riscv32be/Cargo.toml
- .atlas/temporary/donors/mold/arch/riscv64/Cargo.toml
- .atlas/temporary/donors/mold/arch/riscv64be/Cargo.toml
- .atlas/temporary/donors/mold/arch/s390x/Cargo.toml
- .atlas/temporary/donors/mold/arch/sh4/Cargo.toml
- .atlas/temporary/donors/mold/arch/sh4be/Cargo.toml
- .atlas/temporary/donors/mold/arch/sparc64/Cargo.toml
- .atlas/temporary/donors/mold/arch/x86_64/Cargo.toml
- .atlas/temporary/donors/mold/cli/Cargo.toml
- .atlas/temporary/donors/mold/tests/Cargo.toml

## Test / Benchmark Roots Detected

- .atlas/temporary/donors/mold/tests

## Major Subsystem Roots

.github, arch, c, cli, docs, src, tests

## License Evidence

- .atlas/licenses/donors/mold/LICENSE

## Census State

Status: COARSE_CENSUSED. This is an admission-stage inventory only. Deep census must classify algorithms, invariants, state/effect boundaries, execution behavior, tests, benchmarks, rejected ideas, and Atlas-native replacement gaps before absorption.

## Native Replacement

compiler linker integration and post-link layout reference
