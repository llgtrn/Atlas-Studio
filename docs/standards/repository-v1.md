---
id: atlas.standard.repository.v1
type: contract
status: active
canonical: true
---
# Repository Standard v1

Every active repository carries:

```text
.atlas/repo.toml
```

The manifest declares a primary archetype and maps semantic roots.

Supported initial archetypes:

- CANONICAL_PRODUCT
- DEVELOPMENT_CELL
- ENGINEERING_SUBSYSTEM
- DOMAIN_SUBSYSTEM

Atlas validates the archetype contract. It does not infer sovereignty or implementation permission from a repository name.

A semantic-root manifest is deliberately more stable than hardcoded folder assumptions: a future repository may move internals while keeping the same root roles and compatibility contract.
