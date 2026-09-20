---
id: atlas.standard.ui.v1
type: contract
status: active
canonical: true
---
# Atlas UI Design Standard v1

Atlas Graph Studio is TypeScript/TSX only.

Open-source design projects are references for mechanics and interaction patterns; Atlas owns its final design grammar.

Reference themes:

- shadcn/ui: composition and copy-owned components;
- Radix Primitives: accessible interaction primitives;
- Lucide: icon grammar;
- Tailwind CSS: token/utility design mechanics;
- xyflow: node-based canvas interaction;
- ELK.js: layered graph layout;
- Cytoscape.js: graph interaction/analysis patterns.

The final Atlas UI centers on an engineering graph, not a dashboard-card collection.

Primary shell:

```text
Navigation / Repo Fleet
        |
Command Palette -- Workspace Tabs
        |
Graph Canvas -------- Inspector
        |
Timeline / Evidence / CI status
```

Every important node should expose provenance, evidence, canonicality status, responsibility, current implementation status, and available engineering actions.
