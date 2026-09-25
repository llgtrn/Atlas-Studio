---
id: donor-census-regalloc2
type: reference
status: active
canonical: true
---
# Donor Census: regalloc2

## Source

- Remote: https://github.com/bytecodealliance/regalloc2.git
- Repository: bytecodealliance/regalloc2
- Commit: 2fe490bc9dda433f70c54f90f7633ed929f693d9
- Git tree: 1c6edf753513b879f27b61515db153c46ac23844
- Branch: main
- Retrieved: 2026-09-21T03:00:34.9575636Z
- Staging mode: FULL_SOURCE_TREE
- Clone path: .atlas/temporary/donors/regalloc2 (corrected -- this record previously said `.atlas/temporary/regalloc2`, which does not exist; verified against the real filesystem)

## Coarse Inventory

- Files observed: 50
- Bytes observed: 640805
- Languages/signals: Rust, Markdown, TOML, YAML
- Top-level directories: .github, doc, fuzz, regalloc2-tool, src
- Top-level files: .gitignore, Cargo.toml, deny.toml, LICENSE, README.md

Full source tree staged with nested .git removed.

## Build Systems Detected

- .atlas/temporary/donors/regalloc2/Cargo.toml
- .atlas/temporary/donors/regalloc2/fuzz/Cargo.toml
- .atlas/temporary/donors/regalloc2/regalloc2-tool/Cargo.toml

## Test / Benchmark Roots Detected

- .atlas/temporary/donors/regalloc2/fuzz

## Major Subsystem Roots

.github, doc, fuzz, regalloc2-tool, src

## License Evidence

- .atlas/licenses/donors/regalloc2/LICENSE

## Census State

Status: COARSE_CENSUSED. This is an admission-stage inventory only. Deep census must classify algorithms, invariants, state/effect boundaries, execution behavior, tests, benchmarks, rejected ideas, and Atlas-native replacement gaps before absorption.

## Native Replacement

native backend register allocation and spill strategy reference

## G102 — terminal REFERENCE_ONLY; source extinct

No LIR or Machine IR stage exists. The external native backends (G96 LLVM, G100 Cranelift) allocate registers inside their own boundary. A future native Machine IR stage would evaluate the `regalloc2` crate as a dependency, or re-derive its allocator, at that time. The checkout was physically deleted. Evidence: `../../evidence/campaign/38-regalloc2.json`.
