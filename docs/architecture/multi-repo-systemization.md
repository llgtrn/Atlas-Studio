---
id: atlas.architecture.multi-repo-systemization
type: architecture
status: active
canonical: true
---
# Multi-repo systemization

Atlas Systemizer treats repositories as nodes in an engineering fleet graph.

The fleet is not a shared mutable monorepo.

```text
Fleet
  -> Repository
  -> exact base SHA
  -> bounded work packet
  -> isolated branch/worktree
  -> repo-local CI/evidence
  -> PR/integration
  -> refreshed fleet graph
```

Rules:

1. Parallelism is allowed across repositories.
2. Mutation is serial within one repository/base SHA lineage.
3. Every mutation plan pins an exact base SHA.
4. Shared mutable checkouts are forbidden.
5. Atlas may prepare branches/plans but has no merge authority.
6. Repository semantics come from `.atlas/repo.toml`, not from repository name.
7. Repository standards are archetype-based, not one universal folder tree.

This allows Chronica, Development Cells, Atlas-Systemizer, and future subsystems to remain structurally compatible without pretending they have identical responsibilities.
