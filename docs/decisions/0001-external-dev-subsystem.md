---
id: atlas.adr.0001
type: decision
status: accepted
canonical: true
---
# Atlas Systemizer is a development subsystem

Atlas Systemizer is external to Chronica runtime. Chronica calls the versioned `atlas-systemizer` CLI; it never imports Atlas crates. Atlas-derived graphs and reports are rebuildable ANALYZE projections and never canonical runtime truth or merge authority.
