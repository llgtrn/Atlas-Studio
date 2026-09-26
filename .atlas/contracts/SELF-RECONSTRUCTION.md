---
id: atlas.contract.self-reconstruction
type: contract
status: active
canonical: true
---
# Self-Reconstruction Contract (SELF_RECONSTRUCTION, alias ATLAS_SELF_HOSTING)

## Purpose

SELF_RECONSTRUCTION is the permanent lane in which Atlas rebuilds bounded parts of itself from what it knows about itself. What it knows is the typed census of its own source. The rebuilt part is then verified against the original.

It asks one question per attempt: can Atlas's knowledge of this target, alone, produce an equivalent target? The answer is a verdict with evidence. When the answer is no, the attempt names exactly what knowledge is missing. That is a typed `ConstructionGap`, and it feeds the debt ledger.

The lane is distinct from:
- FULL_OSS_REPLAY, which puts other people's code through successively stronger Atlas epochs;
- NEW_DONOR_PROGRESSION;
- FRONTIER_EXPANSION;
- HISTORICAL_DONOR_REVALIDATION;
- NATIVE_ATTACK.

It is recorded in `.atlas/roadmap/SELF-RECONSTRUCTION.toml`. A generation of this lane has kind `SELF_RECONSTRUCTION`.

## Self-hosting levels

| Level | Meaning |
|---|---|
| SH0 | Atlas censuses itself (self-recensus, since G57). |
| SH1 | A bounded shadow reconstruction of one self target is verified equivalent to the original. |
| SH2 | A whole module is reconstructed and verified. |
| SH3 | A whole crate is reconstructed and verified. |
| SH4 | A reconstructed part replaces its original, under admission by a SELECTED design. |
| SH5 | Atlas rebuilds its own construction path (census → IR → backend) from its own knowledge. |
| SH6 | Atlas reconstructs all of itself: a fixed point. |

A level is **reached** only by a verdict of `RECONSTRUCTED_EQUIVALENT` or `RECONSTRUCTED_WITH_DECLARED_VARIATION` at that level. An attempt with any other verdict leaves the level **attempted**. `level_reached` in the ledger is machine-checked against the attempts.

## Bootstrap target

`BOOTSTRAP_CONSTRUCTION_TARGET = RUST` (SYSTEM-CONTRACT `BACKEND_BOOTSTRAP_IS_RUST`).

The construction IR is target-neutral. The Rust backend is its first projection and is never its definition.

## Construction input boundary

Construction reads only:
- **CENSUS_RECORD**: typed records of a verified census container, by record id;
- **DESIGN**: a design over that container (`SelectedDesign`), by id;
- **COMPARISON**: the comparison that design was measured in, by id.

`ConstructionInputKind` has no variant for source text.

The lifter takes records, not paths. It cannot open a file.

The module lists every input. Every IR element carries lineage to the input records it came from. Validation refuses:
- lineage that reaches outside the declared inputs;
- an input that does not resolve in the container;
- a module without a design.

## Source independence

The original source is an **oracle**. It is read (`ORACLE_READ`) or run (`ORACLE_EXECUTION`) by verification only. Oracle uses are listed in the report and never among the construction inputs. A report citing an oracle reference as an input is refused (`ORACLE_AS_INPUT`).

A pilot that reads the original while working is recorded as having done so in the attempt's evidence. Construction stays mechanical from the records, so what the pilot read cannot enter the module.

Each function in the IR also carries the census's body fingerprint. It is kept so that a future reconstructed body can be reported as token-identical or not. It is never a construction input.

## Shadow artifact

A reconstruction is built as a **shadow**:
- in `.atlas/.cache/shadow/<target>` (gitignored, never admitted, never in census scope);
- as its own Cargo workspace, pinned by the repository's lockfile.

The shadow records:
- its path;
- its content hash;
- the backend;
- the toolchain that built and tested it (`rustc -V`);
- which IR elements the backend emitted and which it omitted, with the reason for each.

The backend never guesses. An element with anything unobserved is omitted, never approximated.

## Construction gaps

An element Atlas did not observe stays unobserved in the IR (`None`). It carries a typed gap: `ITEM_KIND_UNOBSERVED`, `DERIVES_UNOBSERVED`, `ATTRIBUTES_UNOBSERVED`, `VISIBILITY_UNOBSERVED`, `VARIANT_SHAPE_UNOBSERVED`, `BODY_UNOBSERVED` or `UNSUPPORTED`.

Each gap names the debt that owns closing it. An unobserved element without its gap is refused (`SILENT_GAP`).

The ledger's `gap_feedback` maps every gap kind to its debt and an attack:
- a gap the latest attempt still has is OPEN, and its attack must be queued;
- a gap no longer present is CLOSED, with the generation that closed it.

## Equivalence

- **Behavioral equivalence:** the shadow and the compiled original behave alike when run. For every derived capability of a type, a generated differential compares every variant pair: Debug, serde wire form both ways, equality, order, clone, default. A `match` over the original with no wildcard makes a variant the shadow lacks a build failure.
- **Semantic equivalence:** Atlas's census of the shadow says what the input records said. This covers:
  - the definition;
  - its documentation;
  - its declared shape (item kind, visibility, derives, attributes);
  - its members in order, with their documentation, attributes and shape.

## Verdicts

The verdict is computed from the evidence (`decide`). A claimed verdict that differs from it is refused. The rules apply in this order:

1. No shadow → `CONSTRUCTION_GAP` if gaps explain why, else `UNSUPPORTED`.
2. Any semantic mismatch → `SEMANTIC_MISMATCH`.
3. Any behavioral mismatch, or no equivalent evidence of either kind → `VERIFICATION_FAILED`.
4. Any gap → `CONSTRUCTION_GAP`. A partial reconstruction is never equivalent.
5. Declared variations → `RECONSTRUCTED_WITH_DECLARED_VARIATION`.
6. Otherwise → `RECONSTRUCTED_EQUIVALENT`.

A mismatch outranks a gap, because a wrong reconstruction is worse than an incomplete one.

## Design and authority

A shadow is built over a VALIDATED design, which is the output of `design candidates` and `design compare`. A shadow is not a materialization. Replacing an original with its reconstruction (SH4) needs a SELECTED design, with an authority event from a declared principal (ADR 0064). A provider never selects, and nothing in this lane fabricates a selection.

## Target selection

Atlas chooses the target from its own world model (`self-reconstruct candidates`). A core type and its methods are constructible when, by what Atlas knows:
- no function of the target has an effect, state, persistence or concurrency record;
- none has an unresolved call;
- none calls outside the target;
- none is unsafe or async.

Candidates are ordered as follows: constructible first, then more dependents, then fewer control blocks.

## Cadence

A SELF_RECONSTRUCTION generation is a non-replay generation. It interleaves with FULL_OSS_REPLAY under that lane's cadence: at most one non-replay generation between replays. It never pauses the replay campaign.

## Anti-cheat

The lane forbids, and the tests refuse:
- source text as construction input;
- lineage outside the inputs;
- a backend that fills an unobserved element;
- a partial reconstruction called equivalent;
- a shadow that was never compared with its original;
- a level claimed without a reconstruction;
- a gap without an owning debt;
- an open gap without a queued attack.

Copying the original file into the shadow is not reconstruction. A shadow's content hash is not evidence of equivalence; only the checks are.

## Implementation status

- G153 (ADR 0069): `core::construction` and `runtime::self_reconstruction`, and the CLI `self-reconstruct candidates|roots|attempt`.
- SYMBOL definitions carry their declared shape.
- SR1 targets `core::donor::MaterializationMode`.
- Bodies are the open gap (NA-SELF-RECONSTRUCTION-BODIES).
