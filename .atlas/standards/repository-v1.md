---
id: atlas.standard.repository.v1
type: contract
status: active
canonical: true
---
# Repository Standard v1

Every Atlas-managed repository uses .atlas/ as its canonical knowledge/control root and declares implementation responsibility roots in .atlas/repo.toml.

Physical implementation shape follows responsibility, but canonical knowledge paths and admission rules do not split into competing documentation universes. Generated caches are rebuildable. Mutation planning requires an exact base SHA and one canonical repository target per WorkRun.
