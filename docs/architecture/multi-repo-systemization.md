---
id: atlas.architecture.multi-repo-systemization
type: architecture
status: active
canonical: true
---
# Multi-repo Systemization

## System Model

Atlas treats every configured repository as a read-only node in one engineering fleet graph. The fleet is discovered from fleet/repos.yaml, not from ad-hoc repository names supplied by a coding worker.

## Responsibilities

Fleet connection resolves identity and exact HEAD for every registered repository. Coding admission selects one registered repository and one exact base SHA.

## Boundaries

Read-only observation may cover the whole fleet. Coding/mutation planning is exactly one repository per session. Atlas does not create a shared mutable monorepo and does not merge canonical branches.

## Runtime Ownership

atlas-fleet owns fleet connection and coding-session planning. Each repository owns its source/docs/evidence. Git/review owns integration.

## State and Effects

Fleet connection state is derived and rebuildable. A work plan is ANALYZE. Repository mutation is an external ACT and remains repo-local.

## Dependencies

Every managed repository must satisfy the same atlas.docs.v1 documentation contract before coding work can be prepared. Implementation root shape may differ by explicit repository archetype, but documentation control paths/headings do not vary.

The enforced flow is:

~~~text
fleet/repos.yaml
    ↓
connect all registered repos read-only
    ↓
resolve exact heads
    ↓
select ONE repo
    ↓
RepoGate + DocsGate
    ↓
exact base SHA
    ↓
bounded work packet
    ↓
isolated branch/worktree
    ↓
repo-local CI/evidence
    ↓
review/integration
    ↓
refresh fleet graph
~~~

Cross-repository change requirements are decomposed into sequential coding sessions: finish/reconcile the current repo, refresh the fleet, then open a new session for the next repo.
