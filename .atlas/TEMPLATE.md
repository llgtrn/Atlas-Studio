---
id: atlas.docs.template
type: contract
status: active
canonical: true
---
# Atlas Documentation Template

Every maintained Markdown document under `.atlas/` carries `id`, `type`, `status` and `canonical` frontmatter.

Markdown is the human-auditable contract and architecture layer. It is not the ATLAS binary format and is never a substitute for `*.atlas` or `*.atlasx/` semantics.

Canonical documentation states durable responsibility, invariants, format contracts, graph contracts and sequencing. Temporary progress notes, generated graph dumps, human-readable artifact exports and rebuildable scans are non-canonical projections.

When a document describes a hard requirement, it should map to Atlas Genome or another machine-enforced contract rather than relying on prose forever.
