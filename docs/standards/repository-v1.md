---
id: atlas.standard.repository.v1
type: contract
status: active
canonical: true
---
# Repository Standard v1

Every active repository carries .atlas/repo.toml and declares one primary repository archetype.

Supported implementation archetypes are CANONICAL_PRODUCT, DEVELOPMENT_CELL, ENGINEERING_SUBSYSTEM and DOMAIN_SUBSYSTEM.

Implementation roots may differ because responsibilities differ. Atlas validates those declared semantic roots and never infers sovereignty/implementation permission from the repository name.

Documentation is different: **all Atlas-managed repositories use exactly the same atlas.docs.v1 control structure and admission gate with no archetype-specific exception.**

Therefore:

~~~text
implementation roots = responsibility-specific and manifest-declared
documentation roots  = universal and mandatory
coding admission      = RepoGate AND DocsGate AND exact SHA AND one selected repo
~~~

A repository may move physical implementation internals while preserving semantic root mappings, but it may not omit/rename mandatory documentation control paths or bypass DocsGate.
