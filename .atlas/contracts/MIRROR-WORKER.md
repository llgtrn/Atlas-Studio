---
id: atlas.contract.mirror-worker
type: contract
status: active
canonical: true
---
# Mirror Worker Contract

## Hard Invariants

Mirror workers are optional, non-canonical and initially one-to-one with the target repository at an exact SHA.

## Interfaces

Atlas records target repo/SHA, proposed mirror repo, assigned scope and reconvergence plan.

## State and Durability

Target Git history remains canonical. Mirror history is temporary engineering evidence until integrated.

## Authority

Mirror workers cannot change target canonical state directly.

## Evidence

Creation SHA, tree equivalence, scope, worker commits, CI and final integration mapping.

## Recovery

Discard or regenerate a mirror from the target exact SHA.

## Verification

Verify initial tree equality, bounded scope, no independent canonical claims and successful reconvergence.
