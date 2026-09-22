---
id: atlas.blueprint.bulk-donor-absorption
type: blueprint
status: active
canonical: true
---
# Bulk Donor Absorption

Atlas defaults to bulk donor staging when a capability roadmap already identifies a bounded donor set.

## Rule

Do not clone donors one-by-one as implementation reaches them. Clone the approved batch up front into `.atlas/temporary/donors/<donor>/`, pin exact revision/license/provenance immediately, perform a cheap coarse census over the whole batch, then deep-census/deepfork according to the dependency roadmap.

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
verify donor runtime/source dependency = 0
      ↓
physically delete donor source files from .atlas/temporary/donors/<donor>/
      ↓
verify donor source path is absent
      ↓
EXTINCT
```

Deletion is incremental. Atlas does not wait for the entire donor corpus to be absorbed before removing source for already-absorbed donor scopes.

**ABSORBED is not EXTINCT.** ABSORBED means the required knowledge, Atlas-native implementation and verification exist. EXTINCT is a later physical state reached only after the donor OSS source files have actually been deleted from Atlas-controlled working storage and their absence has been verified.

When donor knowledge is intended to inform Atlas Development Language or compiler primitives, absorption additionally follows `../contracts/DONOR-TO-LANGUAGE-GENESIS.md`: syntax/API copying is not absorption; the durable intermediate is a Technology Genome containing mechanisms, invariants, trade-offs and evidence.

## Extinction gate

A donor scope may enter ABSORBED only when:

- required mechanisms/algorithms/invariants have been captured in ATLAS knowledge;
- provenance and license evidence are durable;
- the Atlas-native replacement exists in the intended owner;
- runtime dependency on donor source/library is zero for the absorbed scope;
- tests and material benchmarks pass where applicable;
- intended design and recensused implementation agree;
- remaining donor-only knowledge has either been captured or explicitly rejected.

After those gates pass, extinction requires a separate destructive step:

1. delete the donor source tree/files from `.atlas/temporary/donors/<donor>/` for the extinct scope;
2. delete any Atlas-controlled local source archive, snapshot, vendor copy or cache that could substitute for that deleted donor tree;
3. verify the donor source path no longer exists in the active repository/worktree;
4. verify no runtime/build/test path imports, links, shells out to, reads, or otherwise depends on that deleted source;
5. retain only durable non-source knowledge required by Atlas: pinned revision identity, provenance, license obligations, typed semantic records, Technology Genomes, decisions and verification evidence;
6. only then mark the scope `EXTINCT`.

Leaving donor source files in place but unused, ignored, unreferenced, renamed, archived or hidden in another Atlas-controlled directory is **not extinction**.

A donor intentionally retained locally as a differential oracle/reference is `REFERENCE_ONLY` (or another explicit non-extinct state), not `EXTINCT`.

Physical deletion applies to the active Atlas-controlled source tree/workspace. It does not require rewriting historical Git objects unless a separate history-scrubbing policy explicitly requires that.

Git history is not the only preservation mechanism. `*.atlas` must preserve the durable semantic/evidence result after donor checkout deletion without requiring the deleted donor source tree to remain locally available.

## No permanent vendor forest

`.atlas/temporary/donors/` is a workbench, not a vendor directory. Donor trees are required to disappear as extinction proceeds.

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
