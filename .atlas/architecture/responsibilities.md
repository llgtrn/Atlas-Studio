---
id: atlas.architecture.responsibilities
type: architecture
status: active
canonical: true
---
# Atlas Studio Responsibilities

`core` owns product-neutral typed meaning: identity, node/edge/binding, scope, state/event/temporal semantics, evidence/provenance, claim status, constraints/invariants, interface/capability/effect semantics, Atlas Genome contracts, ATLAS/ATLASX schemas and compiler IR types.

`runtime` owns algorithms: security admission, adaptive census, graph construction, incremental invalidation, research correlation, comparison, synthesis, design selection, ATLAS publication, ATLASX materialization, compiler passes, verification and recensus.

`adapter` owns external mechanics: repository/Git/filesystem access, language parsers, package ecosystems, DeepWiki/research/provider access, storage, sandbox/subprocess/toolchain interfaces and target platform mechanics.

`apps/ui` owns the TypeScript/TSX World Canvas and source/design/compiler/evidence projections. UI views are projections of bounded engine queries, never a second truth store.

Generated target systems remain sovereign repositories. Atlas may connect them through stable graph identities and bindings but may not silently merge ownership or authority boundaries.
