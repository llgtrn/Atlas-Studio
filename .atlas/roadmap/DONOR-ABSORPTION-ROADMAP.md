---
id: atlas.roadmap.donor-absorption
type: blueprint
status: active
canonical: true
---
# Donor Absorption and Extinction Roadmap

## Purpose

This roadmap turns the donor registry into an execution order for Atlas self-building.

Canonical inputs:

- `../references/donor-corpus.toml` — admitted donor identity, pinned revisions, licenses, provenance and current status;
- `DONOR-ABSORPTION-PLAN.toml` — machine-readable wave order and donor membership;
- `../blueprints/BULK-DONOR-ABSORPTION.md` — absorption/extinction gates;
- `../contracts/DONOR-TO-LANGUAGE-GENESIS.md` — Technology Genome and language-genesis rules.

## State machine

```text
STAGED
  ↓ pin SHA/license/provenance
COARSE_CENSUSED
  ↓ capability/dependency targeting
DEEP_CENSUS_ACTIVE
  ↓ typed mechanisms/invariants/evidence
TECHNOLOGY_GENOME_CAPTURED
  ↓ Atlas-native design + implementation
NATIVE_IMPLEMENTED
  ↓ differential tests/benchmarks + Atlas recensus
ABSORBED
  ↓ runtime/source dependency = 0
SOURCE_DELETED
  ↓ verify source path absent
EXTINCT
```

`ABSORBED` is not `EXTINCT`.

An EXTINCT scope MUST NOT retain its donor OSS source files under Atlas control. The active source path `.atlas/temporary/donors/<donor>/` for that extinct scope must be physically absent. Renaming, ignoring, unreferencing, archiving or moving the same source into another Atlas-controlled cache/vendor/snapshot directory does not satisfy extinction.

A donor retained locally as an explicit oracle/reference is not extinct.

Historical Git objects are outside this active-tree deletion rule unless a separate history-scrubbing policy is adopted.

## Recursion rule

Every semantic capability wave closes this loop:

```text
implement stronger Atlas census semantics
  ↓
re-census targeted donor scopes
  ↓
capture Technology Genomes
  ↓
compare mechanisms/invariants/trade-offs
  ↓
implement Atlas-native capability
  ↓
verify + benchmark
  ↓
re-census Atlas itself
  ↓
ABSORBED
  ↓
physically delete absorbed donor source scope
  ↓
verify absence
  ↓
EXTINCT
```

A wave may partially extinguish a donor. Unabsorbed scopes remain staged/reference material until their own gates close.

## Wave 0 — corpus accounting

All donors in `donor-corpus.toml` participate.

Goals:

- exact revision/license/provenance pinned;
- cheap whole-repository inventory;
- repository/language/module/build/dependency/capability map;
- source regions mapped to Atlas capabilities;
- no donor silently omitted.

This wave is breadth-first. It does not require deep semantics for every donor.

## Wave 1 — source intelligence and semantic graph

Donors:

`tree-sitter`, `rust-analyzer`, `scip`, `kythe`, `glean`, `joern`, `semgrep`, `ast-grep`, `sourcetrail`.

Primary learning:

- structural parsing and incremental syntax;
- stable symbol/type/function identity;
- cross-reference representation;
- call/control/data-flow graph construction;
- queryable fact/schema models;
- structural/semantic search and rewrite boundaries.

Atlas targets:

- `adapter/source`;
- `core/identity`, `core/schema`, universal graph;
- `runtime/census`, `runtime/normalize`, `runtime/query`.

Extinction is per absorbed mechanism. UI-only or unsupported scopes remain non-extinct until separately handled.

## Wave 2 — compiler semantic core

Donors:

`rust`, `llvm-project`, `wasmtime`, `wasm-tools`.

Primary learning:

- function/call/control/data-flow semantics;
- type/layout/ABI constraints;
- ownership/resource/concurrency semantics from Rust where applicable;
- SSA/IR legality and lowering;
- sandbox/capability boundaries;
- WASM validation/component representation.

Atlas targets:

- typed semantic families beyond R4.3.2;
- HIR/MIR/LIR foundations;
- resource/effect/authority semantics;
- bounded execution and exchange boundaries.

The locally retained rustc/LLVM/Wasmtime source may remain REFERENCE_ONLY while still required as an oracle. Such retained scopes are not EXTINCT.

## Wave 3 — incremental reasoning and build graph

Donors:

`salsa`, `datafrog`, `differential-dataflow`, `souffle`, `buck2`.

Primary learning:

- dependency invalidation;
- memoized/incremental queries;
- fixed-point and relational inference;
- differential graph maintenance;
- build/target identity and scheduling.

Atlas targets:

- `runtime/query`, `runtime/closure`;
- incremental recensus;
- dependency-aware materialization/build execution.

## Wave 4 — verification and safety

Donors:

`kani`, `miri`, `verus`.

Primary learning:

- model checking and proof obligations;
- execution-semantics validation;
- alias/UB/resource safety evidence;
- proof-oriented constraints.

Atlas targets:

- `runtime/verify`;
- semantic obligation/evidence models;
- safety gates for ownership/effect/concurrency semantics.

## Wave 5 — transformation and migration

Donors:

`openrewrite`, `c2rust`, `crubit`, `py2many`.

Primary learning:

- typed repeatable transformations;
- migration IR and semantic preservation;
- inter-language boundaries;
- before/after lineage.

Atlas targets:

- invention/refactor/migration planning;
- deterministic ChangeSet and transformation evidence;
- ADL/native semantic lowering where admitted.

## Wave 6 — binary, storage, data layout and native-backend mechanics

Donors:

`flatbuffers`, `arrow`, `zstd`, `blake3`, `object`, `regalloc2`, `mold`.

Primary learning:

- schema/layout and zero-copy trade-offs;
- columnar data and IPC representation;
- compression and content identity;
- object formats/relocations/debug structures;
- register allocation/spilling;
- linking and parallel binary layout.

Atlas targets:

- logical/wire ATLAS storage;
- content-addressed shards;
- evidence/data-layout optimization;
- Machine IR/backend/link stages.

## Wave 7 — security and trust

Donors:

`containers-image`, `podman`, `selinux`, `openscap`, `keycloak`, `keylime`, `clair`.

Primary learning:

- supply-chain admission;
- sandbox/rootless isolation;
- least privilege/access control;
- compliance policy;
- identity/authorization;
- attestation/integrity;
- vulnerability intelligence.

Atlas targets:

- typed security/authority semantics in `core`;
- policy/state machines in `runtime`;
- OS/provider mechanics in `adapter`.

## Wave 8 — Studio/editor projection

Donors:

`zed`, `opendesign`, `xyflow`, `cytoscape-js`, `elkjs`.

Primary learning:

- editor/workspace architecture;
- design authoring;
- graph interaction/rendering;
- automatic layout;
- agent/editor interaction.

Atlas targets:

- `apps/studio` projections over canonical Atlas semantics.

These donors must not become alternate owners of canonical truth.

## Physical extinction proof

Before a scope can transition from ABSORBED to EXTINCT, the verification record must establish all of the following:

- the exact absorbed donor/revision/scope;
- the Technology Genome/evidence/native replacement that supersedes it;
- zero runtime/build/test dependency on the donor source for that scope;
- deletion of the donor source files from the active Atlas-controlled donor path;
- absence of substitute local source archives/caches/snapshots/vendor copies;
- a post-delete check proving the source path is absent;
- Atlas recensus after deletion still passes the required invariants/tests/benchmarks.

Durable license/provenance/revision metadata remains after extinction. The source tree does not.
