---
id: atlas.decision.0069.self-reconstruction-lane
type: decision
status: accepted
canonical: true
---
# ADR 0069 — SELF_RECONSTRUCTION: Atlas rebuilds bounded parts of itself from its census (G153)

## Context

Until now Atlas has only observed its own code. Construction was a set of contracts: COMPILER-IR-SCHEMAS, COMPILER-IR-PIPELINE and ATLAS-TO-ATLASX. DEBT-CONSTRUCTION_IR, DEBT-COMPILER_LOWERING and DEBT-BACKEND_ADAPTER had stood at M1 (contract only) since G38. Every construction node from M8 onward was MISSING.

Nothing tested whether what Atlas records about code is enough to build that code again.

The owner ordered a second permanent lane beside FULL_OSS_REPLAY:
- Atlas rebuilds parts of itself, with Rust as the bootstrap target.
- It works only through `.atlas`, a design, its comparison, a construction IR and a backend.
- The original source is an oracle and never an input.
- Every failure is a typed gap that feeds the debt ledger.

## Decision

1. **The lane.** SELF_RECONSTRUCTION (alias ATLAS_SELF_HOSTING):
   - It has its own ledger (`roadmap/SELF-RECONSTRUCTION.toml`), its own generation kind and its own contract (`contracts/SELF-RECONSTRUCTION.md`).
   - It is distinct from FULL_OSS_REPLAY, NEW_DONOR_PROGRESSION, FRONTIER_EXPANSION, HISTORICAL_DONOR_REVALIDATION and NATIVE_ATTACK.
   - It follows the self-hosting ladder SH0–SH6.
   - A level is reached only by a reconstruction verdict, never by an attempt.
   - Its generations are non-replay generations under the replay cadence.
2. **Construction IR v0** (`core::construction`, `atlas.construction-ir.v0`):
   - It is target-neutral and uses the HIR type vocabulary: ENUM and STRUCT kinds, dispatch kinds and signatures.
   - It is pre-AtlasX: it has no `atlasx_root_id` (DEBT-ATLASX).
   - Its lineage is the census records it came from.
   - An unobserved element stays `None` and carries a typed `ConstructionGap` owned by a debt.
   - v0 carries no bodies: no census record holds lowerable body semantics (M14). Every function is a `BODY_UNOBSERVED` gap.
3. **Construction input boundary.** Construction reads census records, a design and a comparison. Nothing else is possible:
   - `ConstructionInputKind` has no source variant.
   - The lifter takes records, not paths.
   - `validate_module` refuses lineage outside the inputs, an input missing from the container, a module without a design, and an unobserved element without its gap.
4. **The Rust backend** (`atlas.construction.rust-backend.v0`):
   - It is deterministic.
   - It emits a type only when its kind, derives, attributes, visibility and every variant's attributes were observed. It never guesses.
   - It records its derive-path assumptions.
   - It omits functions until bodies exist.
5. **Shadow and verification.**
   - The shadow is its own Cargo workspace under `.atlas/.cache/shadow`: gitignored, never admitted, pinned by the repository's lockfile.
   - **Behavioral equivalence** is a generated differential against the compiled original. It covers every derived capability over every variant pair. A `match` without a wildcard turns a missing variant into a build failure.
   - **Semantic equivalence** is Atlas's census of the shadow against the input records: definition, documentation, declared shape, and members in order.
   - The report records the backend, the toolchain (`rustc -V`), the oracle uses, the checks and the gaps.
   - The verdict is computed (`decide`), and a claimed verdict that differs is refused. The order is: mismatch, then gap, then variation, then equivalence.
6. **Target chosen by Atlas.**
   - `self-reconstruct candidates` ranks the core types and their methods from the world model: constructible (pure and call-closed by what Atlas knows) first, then more dependents, then smaller.
   - Of 118 core types, 38 are constructible. The first is `core::donor::MaterializationMode`: five unit variants, `is_local`, and two external callers.
   - `ConstraintVerdict` was excluded, because `all` has two unresolved std calls.
7. **Design without selection.**
   - Two candidate designs are proposed at one coordinate: the type with its methods, and the type alone.
   - They are compared by reconstruction scope, and the first is on the front.
   - It stays VALIDATED. A shadow is not a materialization, and no principal has selected anything. Nothing here fabricates a selection. Replacing an original (SH4) needs a SELECTED design.

## SR1: the first attempt and the gap it found

**Attempt 1**, over the G152 container: `CONSTRUCTION_GAP`, with 10 gaps and nothing emitted.

What Atlas knew:
- the type's name;
- its five variants, in order;
- the full signature of `is_local`.

What it did not know:
- whether the definition was an enum or a struct;
- its derives;
- its attributes (including `serde(rename_all)`, which decides the wire form);
- its visibility;
- its documentation;
- the variants' attributes;
- the body of `is_local`.

The backend emitted nothing rather than guess.

**The gap fed back into a native change in the same generation.** SYMBOL definitions of structs, enums, variants and fields now carry a `Declaration`:
- item kind;
- visibility;
- derives in order;
- other attributes as written;
- field shape.

They also carry their documentation. None of this is part of their identity, so record ids are unchanged. The container schema declares the new record kind as schema generation G153.

**Attempt 2**, over the G153 container (its self-scope verification ADMISSIBLE), used the same target, design coordinate and comparison. The result is `CONSTRUCTION_GAP` with one gap: `BODY_UNOBSERVED` on `is_local`.

The type was reconstructed from census records alone:
- **Behavioral checks, all EQUIVALENT** against the compiled original: clone, Debug, equality, order, serde in both directions, and an exhaustive match. That is 7 checks.
- **Semantic checks, all EQUIVALENT:** definition, documentation, declaration, and members in order. That is 4 checks.
- **Toolchain:** rustc 1.90.0.
- **Not reconstructed:** the doc comment's line wrapping. The census keeps rustdoc's summary and a line count, not the text's wrapping.

SH1 stays attempted, not reached: it is reached when `is_local` is reconstructed and verified. The pilot read the declaration once, after attempt 1, to confirm the gaps. That read is recorded in `pilot-reads.json`; construction never read source.

## Falsification

Ten mutants were run against the substrate:
- lineage outside inputs accepted;
- an unobserved kind guessed;
- unobserved derives guessed;
- gaps ignored by the verdict;
- an oracle accepted as input;
- derives left among attributes;
- the declaration put into the identity key;
- members ordered by name rather than declaration;
- a wildcard added to the differential;
- a silent gap accepted.

All ten were killed. The unobserved-kind mutant first survived. It showed that the backend's refusal read an unobserved kind as "no objection", and that only an earlier check stood between the backend and a guessed emission. The refusal now objects on its own.

## Consequences

- DEBT-CONSTRUCTION_IR moves from M1 to M2_PARTIAL, and its next attack is NA-SELF-RECONSTRUCTION-BODIES (queued).
- DEBT-TYPE gains declarations.
- DEBT-BACKEND_ADAPTER records toolchain identity per shadow artifact.
- G150's comparison measures are defined only over function roots. `UNRESOLVED_CALLS` cannot measure a design rooted at a type. SR1 therefore compares on reconstruction scope alone, and records the limit.
- SH1 is attempted, not reached, while any gap remains.
- The replay cadence is kept: G154 is FULL_OSS_REPLAY R3 (tree-sitter at the version Atlas links), and G155 is the native queue head.
