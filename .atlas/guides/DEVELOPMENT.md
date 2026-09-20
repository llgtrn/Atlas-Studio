---
id: atlas.guide.development
type: runbook
status: active
canonical: true
---
# Atlas Development Guide

Before coding, read the canonical North Star, architecture, blueprint and contract and resolve the exact base SHA.

Implement one real primitive at a time. Prefer executable vertical slices over new documentation-only abstractions. Backend/compiler/runtime code is Rust. Frontend code is TypeScript/TSX. New backend semantics must not be added to transitional JavaScript tooling.

For donor work: pin revision -> capture license/provenance -> security admission -> census -> extract principles -> design Atlas-native contract -> implement -> test/benchmark -> update evidence. Rebuildable scans and graph reports belong in .atlas/.cache/.
