---
id: atlas.blueprint.mirror-parallelism
type: blueprint
status: active
canonical: true
---
# Mirror Repository Parallelism

## Objective

Allow distributed parallel coding without making fixed Development Cell repositories a prerequisite.

## Inputs

One canonical target repository, exact target SHA, bounded independent scopes and desired worker count.

## Flow

Resolve target SHA -> decide if parallelism is beneficial -> create N optional mirror repositories from the exact target tree -> assign non-overlapping scopes -> run repo-local CI -> produce patches/commits/evidence -> reconverge into target -> delete/archive mirrors when complete.

## Authority

Mirrors never become canonical. Atlas may propose/create mirrors through authorized GitHub tooling but cannot merge the target automatically.

## State

Mirror metadata lives in Atlas. Mirror repository content initially matches the target tree one-to-one at the pinned SHA.

## Failure and Recovery

Stale mirrors are regenerated/rebased; conflicting scopes are serialized; failed mirrors can be discarded without affecting target main.

## Evidence

Target SHA, mirror creation SHA, scope assignment, worker commits, CI results, conflict/rebase evidence and final target integration SHA.

## Verification

Verify one-to-one initial tree, exact base SHA, non-canonical status, bounded scope and successful reconvergence.
