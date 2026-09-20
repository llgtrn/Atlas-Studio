---
id: atlas.architecture.responsibilities
type: architecture
status: active
canonical: true
---
# Atlas Studio Responsibilities

core owns pure typed engineering meaning: identities, epistemic classes, provenance/evidence contracts, ADL, ATLAS format semantics and ATLASX semantic contracts.

runtime owns execution algorithms: admission orchestration, ingest, corpus construction, incremental invalidation, semantic compile/link/query, technology comparison, synthesis, materialization and verification.

adapter owns external mechanics: repository/Git/filesystem observation, parsers, source-index exchange, package ecosystems, storage I/O, sandbox/provider/protocol boundaries.

apps/ui owns the TypeScript/TSX World Canvas and source/design/agent projections over bounded Rust-engine queries. tools is repository engineering only.
