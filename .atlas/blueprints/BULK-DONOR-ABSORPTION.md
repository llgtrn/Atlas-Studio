---
id: atlas.blueprint.bulk-donor-absorption
type: blueprint
status: active
canonical: true
---
# Bulk Donor Absorption

Atlas defaults to bulk donor staging when a capability roadmap already identifies a bounded donor set.

## Rule

Do not clone donors one-by-one as implementation reaches them. Clone the approved batch up front into `.atlas/temporary/<donor>/`, pin exact revision/license/provenance immediately, perform a cheap coarse census over the whole batch, then deep-census/deepfork according to the dependency roadmap.

```text
approved donor set
      ↓
bulk clone once
      ↓
pin SHA + license + provenance
      ↓
coarse census all donors
      ↓
capability/dependency graph
      ↓
deep census selected lane
      ↓
Atlas-native design
      ↓
deepfork / re-implementation
      ↓
compile + benchmark + proof + recensus
      ↓
ABSORBED
      ↓
delete donor checkout from .atlas/temporary
```

Deletion is incremental. Atlas does not wait for the entire donor corpus to be absorbed before removing already-extinguished donor source.

When donor knowledge is intended to inform Atlas Development Language or compiler primitives, absorption additionally follows `../contracts/DONOR-TO-LANGUAGE-GENESIS.md`: syntax/API copying is not absorption; the durable intermediate is a Technology Genome containing mechanisms, invariants, trade-offs and evidence.

## Extinction gate

A donor scope may be removed from `.atlas/temporary/` only when:

- required mechanisms/algorithms/invariants have been captured in ATLAS knowledge;
- provenance and license evidence are durable;
- the Atlas-native replacement exists in the intended owner;
- runtime dependency on donor source/library is zero for the absorbed scope;
- tests and material benchmarks pass where applicable;
- intended design and recensused implementation agree;
- remaining donor-only knowledge has either been captured or explicitly rejected.

Git history is not the only preservation mechanism. `*.atlas` must preserve the durable semantic/evidence result after donor checkout deletion.

## No permanent vendor forest

`.atlas/temporary/` is a workbench, not a vendor directory. Donor trees are expected to disappear as absorption proceeds.

## Current compiler-lane bulk batch

Stage together:

- facebook/zstd
- BLAKE3-team/BLAKE3
- rust-lang/rust
- google/flatbuffers
- llvm/llvm-project
- bytecodealliance/wasmtime
- bytecodealliance/wasm-tools
- gimli-rs/object
- bytecodealliance/regalloc2
- rui314/mold
- apache/arrow

They are not all deep-censused at once. Coarse census all; deep work follows roadmap dependencies.
