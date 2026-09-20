---
id: atlas.architecture.system
type: architecture
status: canonical
canonical: true
---
# Atlas Studio System Architecture

Responsibility roots: core owns typed semantics and invariants; runtime owns ingestion, corpus, compile, link, query, synthesis, materialization and verification algorithms; adapter owns Git/filesystem/parser/storage/provider mechanics; apps/ui owns the TypeScript projection; .atlas owns durable engineering knowledge.

Data flow: untrusted source -> security admission -> source observations -> corpus.atlas -> semantic compiler -> world.atlasx -> bounded query/projection -> analysis/synthesis/materialization -> verification/evidence -> re-observation.

Core performs no filesystem, network, subprocess, provider or UI work. Adapters never become semantic authority. UI owns camera, selection and rendering state, not engineering truth. The atlas-systemizer binary/API remains only a compatibility surface during migration to Atlas Studio identity.
