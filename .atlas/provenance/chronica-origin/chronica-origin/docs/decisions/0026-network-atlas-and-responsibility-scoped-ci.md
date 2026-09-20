---
id: adr.0026
type: decision
status: accepted
canonical: true
---
# ADR-0026 — System Atlas Is a Network Audit and Convergence Control Plane

## Context

Chronica is canonical, while distributed repositories whose legacy names often end in `Ops` are Development Cells that incubate Chronica Packages. Local Cell success is insufficient: a Cell can silently drift in donor accounting, filesystem grammar, language policy, authority/execution semantics, persistence ownership, documentation, CI, or its pinned Chronica reference.

System Atlas already measures local repository reality and Fleet already allocates donors and dispatches bounded engineering work. The missing step is to close these pieces into a continuous cross-repository engineering loop without creating another truth system or giving audit tooling execution authority.

## Decision

System Atlas extends across the registered Development Cell network.

The loop is:

```text
OBSERVE
  Chronica main + registered Cell heads + donor corpus + CI evidence
-> AUDIT
  topology + donor accounting + package contract + language + sovereignty + dependency direction
-> CLASSIFY
  drift | gap | regression | duplication | engineering opportunity
-> PLAN
  bounded repair / absorption / reconvergence task
-> ENGINEER
  isolated worker on owned scope
-> VERIFY
  responsibility-scoped CI + Mirror/Atlas evidence
-> INTEGRATE
  normal Git/branch protection/merge policy
-> RE-AUDIT
```

Atlas is therefore an engineering nervous system, not an authority plane.

### Canonical direction

```text
Chronica main
  -> architecture/contracts/expected SHA
  -> Development Cells

Development Cells
  -> measured report/evidence
  -> System Atlas/Fleet
  -> drift/opportunity plan
  -> candidate engineering work
  -> verified convergence
  -> Chronica main when semantics are universal
```

A Cell never becomes a peer canonical universe. Remote repository state is observed evidence, not canonical World truth.

### Closed-world donor accounting

Atlas must derive donor totals from `tools/refoundation/donor-corpus.yaml`. Counts are never hardcoded.

For every registered Cell it compares:

- donors allocated by Fleet;
- donors declared by the Cell Mirror contract;
- exact donor IDs/repositories/revisions;
- required `needed: YES` donor gaps;
- donor source/provenance/license/census evidence;
- unknown donors absent from the master corpus.

A donor found in a Cell but absent from the master corpus is a hard accounting violation.

### Engineering conformance

Remote audit reuses Cell Mirror invariants rather than inventing new semantics:

- Chronica-isomorphic responsibility roots;
- Rust backend under `core/runtime/adapter/organism`;
- TypeScript/TSX UI under `apps/ui`;
- no production dependency on `temporary/donors/**`;
- `canonical_runtime = CHRONICA_REQUIRED`;
- `standalone_sovereignty = false`;
- no competing World/Identity/Authority/Execution/Evidence/Memory root;
- explicit Chronica reference SHA and package identity.

### Audit is not authority

Atlas may detect, propose, prioritize and dispatch engineering work. It does not bypass Git integration rules, branch protection, code review, CI, authority, or evidence. A generated repair plan is ANALYZE. Repository mutation and merge are ACT.

### CI topology

Chronica CI is responsibility-scoped. The dependency closure is:

```text
core
  -> runtime
      -> organism
          -> adapter

docs / Atlas / UI are separate verification lanes
```

A change runs only the affected responsibility plus downstream dependents. A final aggregate check verifies that every required affected lane passed. Full-system/release suites remain explicit release or scheduled evidence, not the default cost of every small change.

## Consequences

Benefits:

- one registry can audit the whole Development Cell network;
- drift is detected continuously instead of by manual repository inspection;
- donor totals stay synchronized with the canonical corpus;
- engineering agents can consume precise bounded repair tasks;
- CI latency falls because unrelated subsystems do not rebuild by default;
- architecture and implementation behavior converge repeatedly rather than only during large refoundation waves.

Costs:

- cross-repository audit needs read access to private Cell repositories;
- Cells need standardized Mirror/package contracts and eventually standardized component CI;
- network audit can expose a large backlog of real drift;
- full-system tests still remain necessary at integration/release boundaries.

## Verification / implementation impact

Required tooling surfaces:

- Fleet registry is the network membership source.
- Network matrix is derived from the registry, never duplicated manually in CI.
- Network Cell Audit composes Cell Mirror + donor allocation/corpus checks.
- Scheduled/manual Network Atlas CI checks all active Cells.
- Component CI uses path/dependency-aware lanes with one aggregate result.
- Network reports remain generated projections and must be rebuildable.

This extends ADR-0017, ADR-0019 and the Development Cell / Package Fleet blueprint.
