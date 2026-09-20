---
id: atlas.blueprint.system
type: blueprint
status: active
canonical: true
---
# Atlas System Blueprint

## Objective

Turn a predefined multi-repository fleet into a disciplined, documentation-driven engineering environment while keeping coding bounded to one repository per session.

## Inputs

Fleet manifest, exact repository heads, .atlas/repo.toml, complete atlas.docs.v1 documentation, source trees, donor/reference provenance, tests and existing evidence.

## Flow

Connect all registered repositories read-only, resolve heads, audit docs and repo structure, build engineering graphs, select one repository, verify coding admission, emit one bounded work plan, run implementation and proof externally, reconcile docs, then refresh fleet state.

## Authority

Atlas may observe and analyze. Coding is allowed only after hard gates pass. Repository mutation is external ACT. Merge authority stays with normal Git/review governance.

## State

Canonical source and docs live in owning repositories. Atlas indexes, graphs, reports and plans are derived state.

## Failure and Recovery

Missing docs, stale SHA, broken references, invalid repository structure, connection failure or proof failure blocks coding or integration. Derived outputs are rebuildable and failed branches are recoverable through Git.

## Evidence

Every work plan cites selected repo, exact base SHA, docs standard, docs gate, repo gate, scope and verification requirements.

## Verification

Tests must prove docs rejection, cross-repo mutation rejection, exact-SHA enforcement, repo standard checks, CLI compatibility and subsystem runtime isolation.
