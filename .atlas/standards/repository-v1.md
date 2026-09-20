---
id: atlas.standard.repository.v1
type: contract
status: active
canonical: true
---
# Repository Standard v1

Every Atlas-managed repository uses `.atlas/` as its canonical knowledge/control root and declares implementation responsibility roots in `.atlas/repo.toml`.

Every Atlas-generated repository additionally conforms to the Universal Engineering Graph Contract. Physical folders may vary by responsibility/domain, but the semantic spine does not:

```text
Identity → Scope → Node/Edge/Binding
         → State/Event/Temporal
         → Evidence/Provenance
         → Constraint/Invariant
         → Interface/Capability/Effect
         → Materialization
```

Each repository publishes a stable repository identity, revision, exported/imported graph identities and cross-repository bindings. This permits many repositories to compose into one federated engineering world while remaining sovereign mutation/version boundaries.

Generated caches are rebuildable. Mutation planning requires exact base SHA and one canonical repository target per WorkRun. Analysis may span the federated graph.

Backend/compiler/runtime is Rust during bootstrap; frontend is TypeScript/TSX; C is an explicit boundary, not a second application backend universe.
