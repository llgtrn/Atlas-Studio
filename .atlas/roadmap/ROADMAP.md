---
id: atlas.roadmap
type: blueprint
status: active
canonical: true
---
# Atlas Studio Roadmap

## Now — Genome + Phase 0 foundation

- land Atlas Genome hard requirements and graph contract;
- implement global stable IDs, node/edge/binding/temporal/evidence primitives in Rust runtime types;
- finish secure admission and provenance/license handling;
- implement adaptive census scope engine;
- deepen source intelligence with code itself plus tests, DeepWiki/docs and scientific/spec evidence;
- implement real `*.atlas` binary schema, reader/writer, chunk index, integrity and transactional publication;
- make multi-repository federation a graph projection over sovereign repo partitions.

## Next — Phase 1

- deterministic `*.atlas → *.atlasx/` materializer;
- stable ATLASX repo tree and typed units;
- Rust backend, TypeScript frontend and bounded C-ABI lowering;
- compile/test/benchmark/recensus loop;
- differential graph-to-code verification.

## Then — Phase 2

- typed Atlas HIR/MIR;
- ownership/effect/temporal/concurrency analysis;
- semantic optimizer;
- optimized Rust/TS/C reference emission.

## Then — Phase 3

- Atlas LIR;
- LLVM/Cranelift/WASM backends;
- stable Atlas ABI;
- native execution without Rust source as mandatory intermediate;
- Rust path retained for audit/reference/differential proof.

## Then — Phase 4

- Atlas Machine IR;
- x86-64 backend;
- register allocation, instruction selection/scheduling and object emission;
- ARM64, then other targets as justified;
- hardware-aware whole-system specialization.

## Throughout

World Canvas evolves as a semantic map over the same graph with level-of-detail from federation/repository down to semantic atom. No compiler phase, UI, donor or generated repo may create a competing graph universe.
