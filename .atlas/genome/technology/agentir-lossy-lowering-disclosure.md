---
id: atlas.genome.technology.agentir-lossy-lowering-disclosure
type: technology-genome
status: active
canonical: true
---
# Technology Genome: structured, per-field loss disclosure for lossy format lowering (AgentIR)

Donor: AgentIR (`WhitzardAgent/agentir`, commit `68d991160ad579d2b4514da01684249a2a0bf629`), process-
pattern lane (construction-pipeline discipline, not ASIR data-model reuse).

This record follows the mandatory schema in `.atlas/contracts/DONOR-TO-LANGUAGE-GENESIS.md`. It
restructures the existing, already-thorough deep census (`.atlas/census/donors/agentir.md`, 127 lines,
itself reading `docs/DESIGN.md`/`docs/PASSES.md`/`docs/DIAGNOSTICS.md`/`docs/BACKENDS.md` directly and
quoting verbatim) into the canonical genome shape, bounded to the census's own explicitly-named "single
clearest, most load-bearing idea in this donor" (line 80: the `LossReport`/`LossItem` lossy-lowering
discipline), re-verified directly against `docs/BACKENDS.md` in this pass, and cross-checked against
Atlas's own current diagnostic vocabulary rather than the census's disposition table alone.

## Capability / problem

Given a pipeline stage that converts a richer internal representation into a narrower external target
format (a "backend" in the donor's own compiler-pipeline framing), and where that conversion may
unavoidably be lossy for some inputs, make exactly what was lost — not merely *that* something might
have been lost — a queryable, machine-readable, per-field fact, rather than an undocumented assumption
a caller has to reverse-engineer by diffing input and output.

## Semantic mechanism (as observed in the donor, evidence read directly in this pass)

- **A structured, typed loss report replaces "hope nothing important was dropped"**
  (`docs/BACKENDS.md`, read directly, lines 3-35): every backend implements `lower_record(record,
  ctx) -> LoweringResult`, where `LoweringResult { output, report: LossReport }` and `LossReport
  { target: str, status: LoweringStatus, losses: list[LossItem], metrics: dict }`. `LoweringStatus`
  is a four-value enum — `EXACT | LOSSY | PARTIAL | UNSUPPORTED` — giving a coarse, at-a-glance
  severity before a caller even inspects the `losses` list.
- **Each individual dropped/degraded piece of information is its own typed record, not a log line**:
  `LossItem { severity: DiagnosticSeverity, code: str, field: str | None, ... }` (further fields per
  the existing census's own reading of `docs/DIAGNOSTICS.md`: `message`, `event_id`, `suggestion`).
  This means "what got lost" is queryable per-field and per-severity after the fact, not only
  discoverable by reading free-text output at conversion time.
- **The policy is stated as an explicit, non-negotiable rule, not a best-effort convention**
  (`docs/BACKENDS.md` line 3, quoted directly): *"Backends lower canonical AgentIR into target
  formats. They must not silently drop semantics. Every backend produces a `LossReport`."* — framed as
  a hard requirement on every backend, not an opt-in diagnostic feature some backends may skip.
- **This mechanism is the lowering-side half of a matched pair with the ingestion-side
  `detect()`-then-`parse()` frontend contract** (already recorded in the existing census, lines 82-84,
  not re-derived here): frontends must be lossless by default on the way *in*; backends must disclose
  losses explicitly on the way *out*. The donor's own architecture treats "never silently lose
  information" as symmetric across both boundaries of its pipeline, not only one.

## Required invariants

- A backend that cannot represent some piece of canonical information in its target format must record
  a `LossItem` for it — omission of a `LossItem` is treated by the donor's own stated policy as a
  correctness violation ("must not silently drop semantics"), not a missing-nice-to-have.
- `LoweringStatus` must reflect the actual worst-case outcome across all `losses` (i.e. a report cannot
  claim `EXACT` while `losses` is non-empty) — this specific cross-field consistency requirement is
  implied by the mechanism's purpose rather than spelled out as a separate rule in the excerpt read;
  flagged here as an invariant a native implementation must enforce explicitly (e.g. compute `status`
  from `losses` rather than let a caller set both independently and have them drift apart).

## Identity/scope model

Not applicable — this is a diagnostics/reporting mechanism, not an identity scheme.

## State/effect/resource model

Pure, per-conversion computation: `(canonical record, target) -> (output, LossReport)`. No shared
state; each `lower_record` call is independent, which is what makes per-field loss reporting tractable
(no cross-call bookkeeping needed to know what a given conversion dropped).

## Failure and recovery behavior

`LoweringStatus::Unsupported` is itself a typed, non-crashing outcome — a backend that cannot lower a
record at all still returns a `LoweringResult` with a report explaining why, rather than raising. This
is consistent with the existing census's own observation (line 68) that AgentIR passes generally
"emit diagnostics instead of crashing on malformed input" — the loss-reporting discipline and the
donor's broader error-handling posture are the same design choice applied consistently.

## Concurrency/temporal behavior

Not evaluated — no concurrency dimension identified in this mechanism.

## Performance characteristics

Not evaluated/benchmarked; out of scope for a documentation-derived process-pattern census.

## Portability/ABI constraints

Python/Pydantic-specific implementation (`BaseModel` classes); the *pattern* (a typed status enum plus
a list of typed, per-field loss records, returned alongside the conversion's actual output) is
language-independent and maps directly onto a Rust `enum`/`struct` pair with no donor-specific
dependency.

## Evidence references

- `.atlas/census/donors/agentir.md` (127 lines, existing deep census — primary prior evidentiary
  source; already quotes `docs/BACKENDS.md` verbatim and names this the donor's single most
  load-bearing idea)
- `.atlas/temporary/donors/agentir/docs/BACKENDS.md` (re-read directly in this pass, lines 1-37, to
  verify the exact `LossReport`/`LossItem`/`LoweringStatus`/`LoweringResult` shapes rather than relying
  on the census's quotation alone)
- `core/src/semantic/diagnostic.rs` (read fresh this pass) — Atlas's own current `DiagnosticCode`/
  `ExtractionDiagnostic` types, the direct comparison point below
- Confirmed via `grep` that no `LossReport`/`LossItem`/lowering-export mechanism exists anywhere in
  `core/src`, `adapter/src`, or `runtime/src` today — Atlas has no cross-format lowering/export path
  yet for this pattern to attach to.

## Donor revisions/licenses

WhitzardAgent/agentir, commit `68d991160ad579d2b4514da01684249a2a0bf629`, Apache License 2.0
(`.atlas/licenses/donors/agentir/LICENSE`, full text verified by the existing census).

## Known trade-offs

- Per-field, per-conversion loss reporting costs real implementation effort in every backend (each
  lowering path must enumerate and classify its own possible losses) versus a backend that simply does
  its best silently — the donor's own explicit, non-optional policy is evidence this cost was judged
  worth paying deliberately, not an oversight of a simpler alternative.
- A four-value coarse `LoweringStatus` alongside a detailed `losses` list gives both an at-a-glance
  summary and full detail, at the cost of needing the invariant above (status must actually reflect the
  losses list) enforced somewhere, or the two can silently disagree.

## Rejected alternatives (for this pass)

- AgentIR's actual event/trajectory data model, dialects, and frontend/backend catalog — explicitly
  out of scope, unchanged from the existing census's own framing: this is a *trace-of-execution*
  vocabulary, a different concern from Atlas's ASIR static-structure model, and the existing census's
  own REJECT verdict for that portion is not revisited here.
- The existing census's other ADAPT-flagged ideas (multi-level IR with per-stage invariants, the typed
  pass system, two-tier verification, the namespaced diagnostic-code registry, the detect-then-parse
  frontend contract) — deliberately left out of this record's scope, which is bounded to the single
  most load-bearing mechanism per this session's established convention. They remain queued in the
  existing census's own disposition table, which this record does not supersede for those items. Worth
  noting directly: the namespaced diagnostic-code-registry idea is **already substantially real** in
  Atlas today (`core::semantic::diagnostic::DiagnosticCode`, a closed enum with a stable `.as_str()`
  mapping, e.g. `PARSE_FAILURE`, `UNSUPPORTED_LANGUAGE_OR_PROFILE`) — an independent convergence worth
  recording so a future generation does not treat that item as a gap needing donor-inspired work when
  it is not one.
- Taking any part of the `agentir` Python package as a dependency — rejected, unchanged from the
  existing census's own "Things NOT to Copy" section (process-pattern study only, no plausible runtime
  reason to import Python code into a Rust construction pipeline).

## Dependency/extinction status

`STUDY_ONLY_NOT_A_DEPENDENCY` per `donor-corpus.toml`, unchanged by this record. No runtime/build
dependency exists today, consistent with every other lane donor this session has genome-captured.

## Decision

**`ABSORB_LATER`**, native-implementation-only: no Atlas pipeline stage today converts a richer
internal representation into a narrower external target format (confirmed by search — no lowering/
export mechanism exists at all yet, in any crate), so there is no concrete consumer for
`LossReport`/`LossItem` yet. This record's practical output, once such a stage exists (an
`.atlas`/ASIR export to a less-expressive interchange format is the most plausible future trigger,
per the existing census's own framing): give every lossy conversion path a typed status
(`Exact | Lossy | Partial | Unsupported`, computed from, never independently set alongside, its own
loss list) plus a `Vec` of typed, per-field loss records reusing Atlas's own existing
`DiagnosticCode`-style severity/code conventions (already real in `core::semantic::diagnostic`) rather
than inventing a parallel diagnostic vocabulary for export paths specifically.

Not `ABSORB_NOW`: no lowering/export consumer exists yet, and — as this record's own comparison found
— part of what the existing census flagged as still needing donor-inspired work (the diagnostic-code
registry) turns out to already be real in Atlas, reinforcing that checking current Atlas state before
treating a donor's ADAPT verdict as still-open work is itself a necessary step, not a formality.

`agentir`'s `census_status` in `donor-corpus.toml` remains `DEEP_CENSUSED`, unchanged by this record;
this genome record is added as new evidence, covering the lossy-lowering-disclosure mechanism
specifically. The multi-level-IR, typed-pass-system, two-tier-verification, and detect-then-parse
mechanisms the existing census also identified remain queued, not genome-captured, in this pass.

## G95 update

The campaign cycle (first-50 #31) ran this record's falsification. No consumer relies on the lossy fact projection. The `.atlas` census container, the one real narrowing export, has no external consumer. The donor is REFERENCE_ONLY and its checkout is physically deleted, so this record is now the reference for the first external export. Its commit is pinned at `68d991160ad579d2b4514da01684249a2a0bf629`.
