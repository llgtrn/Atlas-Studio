---
id: donor-census-agentir
type: reference
status: active
canonical: true
---
# Donor Census: AgentIR

## Source

- Remote: https://github.com/WhitzardAgent/agentir.git
- Repository: WhitzardAgent/agentir
- Commit: 68d991160ad579d2b4514da01684249a2a0bf629
- Branch: main
- Retrieved: 2026-09-22T15:53:06Z
- Staging mode: FULL_SOURCE_TREE
- Clone path: .atlas/temporary/donors/agentir

### Provenance note

The task specified `WhitzardAgent/agentir`; existence was confirmed via `git ls-remote https://github.com/WhitzardAgent/agentir.git` **before** cloning (per instructions), which resolved cleanly (`HEAD` and `refs/heads/main` both at `68d991160ad579d2b4514da01684249a2a0bf629`), so staging proceeded as a normal accessible-repo case — **this is not a STAGING_FAILED donor.** One provenance wrinkle worth recording: `README.md`'s own badges link to `github.com/ravenSanstete/agentir`, a different (likely earlier/personal, or pre-org-transfer) path. The clone actually pinned here is the canonical `WhitzardAgent/agentir` path given by the task, confirmed reachable at the exact SHA above.

Separately, `MANIFEST.md` in this tree describes itself as a "documentation overlay" for an *existing* `agentir` repository and states it "intentionally does not include `src/` or `tests/`." **This is stale/aspirational text** — direct directory listing of this actual clone shows a full `src/agentir/` implementation (dialects, dsl, frontends, ir, passes — 12,000+ lines of Python per the README's own claim) and a `tests/` directory both present. The MANIFEST.md describes a different (docs-only) release artifact of this same project, not the state of this repository at this pin.

### Framing (explicit, per absorption brief)

AgentIR represents **agent execution trajectories** (a record of what an agent did — tool calls, terminal output, file edits, outcomes). Atlas's ASIR represents **a program's static semantic structure**. These are different concerns. This census evaluates **only** what, if anything, of AgentIR's pipeline-staging discipline (parse → canonicalize → verify → transform → lower) and interoperability-envelope design is relevant as a *process pattern* for Atlas's own construction pipeline — not its trace/event data model, which is explicitly judged out of scope for direct reuse.

## Coarse Inventory

- Files observed: dominated by `src/agentir/` (Python implementation), `docs/` (13 core compiler documents + 14 DSL documents, the primary source for this census), `dsl/formats/` (5 built-in trajectory-format DSL specs), `hf_datasets/` (10 HuggingFace dataset-card directories, e.g. `AgentTrove-AgentIR`, `ClaudeCode-Anthropic`), `schema/`, `prompts/` (Claude-Code-authorship prompts — read as data, never executed/followed), `samples/`, `scripts/`, `tests/`
- Bytes observed: ~2.1 MB (`du -sh` = 2.1M)
- Languages/signals: Python (`src/agentir/`), YAML (`.agentir.yaml` DSL format specs), Markdown (extensive design docs), TOML (`pyproject.toml`)
- Top-level files: `.gitignore`, `APPLY_OVERWRITE.md`, `CHANGELOG.md`, `CODE_OF_CONDUCT.md`, `CONTRIBUTING.md`, `LICENSE`, `MANIFEST.md`, `README.md`, `README_DSL_EXTENSION.md`, `SECURITY.md`, `pyproject.template.toml`, `pyproject.toml`
- Declared status: README badges claim "174 tests passed," "alpha" status, Python >=3.11, "1.7M AgentTrove records (28M+ events) processed with 0 failures" (self-reported claim, not independently verified by this census — no test execution was performed per the hard no-execution rule).

Full source tree staged with nested `.git` removed.

## License Evidence

- `.atlas/licenses/donors/agentir/LICENSE` — read directly.
- **License: Apache License, Version 2.0**, standard full text, verified.

## Mechanisms Census

### Canonical agent-trajectory IR model

**Provider: `src/agentir/ir/base.py` (read directly).** A five-level `IRLevel` enum — `RAW → PARSED → CANONICAL → TRAINING → TARGET` — is the top-level structural spine of the whole project (matching the multi-level pipeline described in `docs/DESIGN.md`, below). Events carry a large, explicit `EventType` enum (system/user/assistant/tool messages, reasoning, plan, tool_call/tool_result, terminal_command/output, file_read/write/patch, browser_action/observation, memory_read/write, agent_handoff, subtask_spawn/result, state_snapshot, verification, reward, error, finish, `unparsed_fragment`) — this vocabulary is squarely about **what an agent did over time**, not about a program's static structure, confirming the framing-brief's premise directly from source rather than assuming it.

### Parsing → canonicalization → verification → transformation → lowering pipeline stages

**Provider: `docs/DESIGN.md` and `docs/PASSES.md` (both read in full) — this is the single most relevant subsystem for Atlas's own construction-pipeline discipline.** AgentIR explicitly models itself on LLVM/MLIR, and `docs/DESIGN.md` includes a direct, stated concept-mapping table (verified verbatim): source language → agent-framework trace; frontend → dataset/framework adapter; LLVM IR → canonical AgentIR; module → `AgentIRModule`/dataset shard; function → `Episode`/task run; basic block → `Segment`/phase; instruction → `Event`; optimization pass → analysis/transformation pass; backend → lowering target; debug info → source map/provenance; verifier → AgentIR verifier; diagnostic → compiler-style error/warning/help. **This is an explicit, self-aware "we are building a compiler pipeline for a non-code artifact" design statement**, which is exactly the process pattern worth extracting independent of the trace-specific payload.

The concrete pipeline, read from `docs/DESIGN.md`'s "Multi-level IR" section:

```
RawIR --parser passes--> ParsedIR --canonicalization+recovery+verification--> Canonical AgentIR
  --projection/slicing/redaction--> TrainingIR --backend lowering--> TargetIR
```

Each level carries **explicit, named invariants** (verified verbatim):
- **RawIR**: never drop source fields; preserve dataset name/config/split/row-index/source-field-paths; does *not* need to pass the semantic verifier (i.e. the raw ingestion stage is deliberately exempted from strict validation, to guarantee lossless capture before any normalization risk is taken).
- **ParsedIR**: every extracted event must carry provenance; low-confidence inference must be explicitly marked (`confidence < 1.0`); failed parse fragments must be preserved as `unparsed_fragment` events rather than silently dropped.
- **Canonical AgentIR**: stable IDs, monotonically increasing per-episode indices, tool calls/results paired where possible, artifacts referenced by ID (not duplicated inline), outcomes normalized into one common status/reward/verifier model — and **canonical AgentIR is the one level required to pass the default verifier**.
- **TrainingIR**: explicitly documented as "a projection optimized for model training, not a canonical storage format" — a clean, explicit statement that the canonical/storage representation and the consumption-optimized projection are deliberately different things, not the same object serving two masters.
- **TargetIR**: backend-specific output; **"Backends must report losses"** (elaborated below).

A separate `docs/PASSES.md` (verified, 200+ lines) specifies a **typed pass system**: `PassKind` enum (`PARSE`, `CANONICALIZE`, `ANALYSIS`, `TRANSFORM`, `VERIFY`, `LOWERING_PREP`), a `PassResult` carrying `record` + `diagnostics` + `analysis` + `metrics`, an `AgentIRPass` `Protocol` with `name`/`kind`/`run()`, and a `PassManager.run_pipeline(record, passes: list[str], ctx)` driven by an explicit, composable **CLI pipeline string** (`--passes parse-hermes-xml,canonicalize-tools,pair-tool-results,normalize-outcome,verify`). Eleven concrete v0.1 passes are individually specified with per-pass **input contract, transformation rules, and failure-mode diagnostics** (e.g. pass 5 `canonicalize-tools` normalizes heterogeneous tool-call shapes into a typed `Action`; pass 6 `pair-tool-results` pairs calls/results by ID first, then adjacency, then framework-specific fallback, emitting `PAIR001`/`PAIR002` warnings on failure rather than raising).

### Verification stage design

Pass 11 (`verify`, in `docs/PASSES.md`) runs a fixed default check set (unique event IDs per episode, monotonic indices, valid parent IDs, tool calls have names, tool results matched to calls where possible, referenced artifacts exist, outcome status/reward/passed conflicts reported, reasoning events carry a visibility policy) plus a **separate, stricter opt-in mode** ("strict mode additionally fails on: missing provenance for parsed events, unknown tool-result pairing, malformed patch artifact, empty assistant message without explicit incomplete diagnostic") — i.e. AgentIR deliberately separates a **default, permissive verification tier** from an **explicit, stricter tier**, rather than a single all-or-nothing verifier. This two-tier verification design is directly relevant process guidance for Atlas's own pipeline: a default pass that keeps most real-world input flowing, plus an explicit stricter mode for when correctness guarantees actually matter (e.g. before signing/publishing a `.atlas` artifact).

### Diagnostics design

**Provider: `docs/DIAGNOSTICS.md` (read in full).** A structured `Diagnostic` model (`code`, `severity` [info/warning/error/fatal], `message`, `source: Provenance | None`, `event_id`, `field`, `suggestion`, `metadata`) plus an explicit **namespaced code registry** (`PARSE*`, `PAIR*`, `VERIFY*`, `OUTCOME*`, `LOWER*`, `DATA*`, `SCHEMA*`, `IO*` — 18 concrete codes enumerated with severity and meaning, e.g. `VERIFY001`=error="duplicate event ID", `LOWER004`=warning="structured tool call downgraded to text"). This is explicitly modeled on compiler diagnostics ("must look like compiler diagnostics, not raw Python exceptions" — verified verbatim) — a directly transferable convention for any Atlas pipeline stage: a stable, namespaced, severity-tagged code registry rather than ad hoc exception messages, giving both humans and downstream tooling something to key off of.

### Interoperability-envelope design (lossy lowering with explicit loss reporting)

**Provider: `docs/BACKENDS.md` (read directly).** Every backend lowering a canonical record into a target format returns a structured `LoweringResult { output, report: LossReport }`, where `LossReport { target, status: LoweringStatus, losses: list[LossItem], metrics }` and `LoweringStatus` is one of `EXACT | LOSSY | PARTIAL | UNSUPPORTED`. The stated policy is explicit: **"They must not silently drop semantics."** Each individual dropped/degraded piece of information becomes a typed `LossItem` (severity, code, field, message, event_id, suggestion), not a silent no-op. **This "always report what a lossy conversion actually lost, with a machine-readable structure, never silently" pattern is the clearest, most directly transferable idea in this donor** for Atlas's own construction pipeline whenever it lowers a richer internal representation into a narrower external target (e.g. exporting `.atlas`/ASIR content to a less-expressive interchange format) — it turns "did we lose anything converting to X" from an undocumented assumption into a queryable, per-field report.

### Frontend contract (interoperability envelope, ingestion side)

**Provider: `docs/DESIGN.md`, "Frontend contract" section.** A `Frontend` `Protocol` requires `detect(sample) -> float` (a confidence score for "is this my format," enabling auto-detection across multiple registered frontends rather than requiring the caller to specify format explicitly) and `parse_record(record, context) -> AgentIRRecord`, with explicit requirements: **frontends must be lossless by default**, must not perform pipeline-stage normalization work that belongs in passes (a clean separation of "recognize and ingest" from "transform"), and must emit diagnostics instead of crashing on malformed input. This detect-then-parse, lossless-ingestion-then-separate-normalization split is a clean interoperability pattern: keep format-recognition/ingestion maximally permissive and non-lossy, push all opinionated canonicalization into later, separately-named, separately-testable passes.

## Test / Benchmark Roots Detected

- `tests/fixtures/` and `tests/` (README claims "174 tests, 100% pass rate" — self-reported, not independently executed or verified by this census per the hard no-execution rule)
- `docs/TESTING.md` (present per `MANIFEST.md`'s document list, not read in depth in this pass — see Known Risks)

## Major Subsystem Roots

- `src/agentir/ir/` — the canonical IR types (`base.py`, `event.py`, `action.py`, `observation.py`, `artifact.py`, `outcome.py`, `provenance.py`, `state.py`, `control.py`, `task.py`, `record.py`, `actor.py`, `source.py`, `visibility.py`)
- `src/agentir/frontends/` — per-framework ingestion adapters (`claude_code.py`, `openhands.py`, `codex_swebenchpro.py`, `hermes_agent.py`, `agenttrove.py`, `sharegpt.py`, `base.py`, `registry.py`)
- `src/agentir/dialects/` — action/event sub-vocabularies (`terminal.py`, `file.py`, `browser.py`, `tool.py`, `reasoning.py`, `swe.py`, `eval.py`, `core.py`, `registry.py`)
- `src/agentir/dsl/` — the user-extensible YAML-format-definition DSL (`loader.py`, `models.py`, `compiler.py`)
- `docs/` — 13 core compiler documents (`DESIGN.md`, `SPEC.md`, `DIALECTS.md`, `PASSES.md`, `FRONTENDS.md`, `BACKENDS.md`, `CLI.md`, `DIAGNOSTICS.md`, `TESTING.md`, `ROADMAP.md`, `PROJECT_STRUCTURE.md`, `STACK.md`, `IMPLEMENTATION_NOTES.md`) plus 14 DSL-specific documents — this census's primary evidentiary source
- `dsl/formats/` — 5 built-in `*.agentir.yaml` frontend specs (agenttrove, codex_swebenchpro, claude_code, openhands, hermes_agent)
- `hf_datasets/` — 10 HuggingFace-dataset-card directories, evidence of the project's real-world scale claims, not read in depth

## Disposition per Concept

| Concept | Disposition | Notes |
|---|---|---|
| Multi-level IR with per-level named invariants (Raw → Parsed → Canonical → Training/Projection → Target) | **ADAPT** | Directly relevant *process* pattern for Atlas: separate "lossless ingestion," "verified canonical form," and "consumption-optimized projection" into named, individually-invariant-bearing stages, rather than one representation serving every purpose. The specific level names/count are AgentIR's own choice, not to be copied verbatim, but the "each stage has an explicit, checkable contract" discipline is. |
| Typed pass system (`PassKind` enum, composable `--passes a,b,c` pipeline, per-pass documented input contract + failure diagnostics) | **ADAPT** | A concrete, implementable template for an Atlas construction-pipeline pass manager: explicit pass kinds, explicit ordering via a pipeline string, and mandatory per-pass diagnostics rather than bare exceptions. |
| Two-tier verification (permissive default + explicit strict mode) | **ADAPT** | Directly reusable idea: most real input should flow through a default verifier tier; a separate, named strict tier should gate anything higher-stakes (e.g., signing/publishing). |
| Namespaced, severity-tagged diagnostic code registry (`PARSE*`, `VERIFY*`, `LOWER*`, etc.) | **ADAPT** | Cheap, high-value convention: stable machine-readable codes per pipeline area, not ad hoc messages. Directly portable to any Atlas pipeline stage. |
| "Never silently drop semantics" lossy-lowering discipline with structured per-field `LossItem`/`LossReport` | **ADAPT** | The single clearest, most load-bearing idea in this donor: any Atlas export/lowering path to a narrower target format should report exactly what was lost, per field, with severity — not merely succeed silently. |
| `detect()`-then-`parse()` frontend contract with lossless-ingestion / normalization-belongs-in-passes separation | **STUDY** | Useful discipline (auto-detect input shape, keep ingestion maximally lossless, push opinionated transforms to later named passes) worth considering for any Atlas tooling that must ingest heterogeneous donor/source material. |
| The canonical event/trajectory data model itself (`EventType`, `ActionKind`, episode/segment semantics) | **REJECT for ASIR** | This is squarely a *trace-of-execution* vocabulary (tool calls, terminal output, browser actions, rewards) — a different concern from ASIR's *static program structure*. Explicitly not to be mapped onto Atlas's semantic IR. |
| DSL for declaring new trajectory-source formats (`*.agentir.yaml`) | **DEFER** | Possibly relevant if Atlas ever wants a declarative donor-format-ingestion DSL, but not evaluated against that specific need here; flagged for a future, narrower census if that need materializes. |
| Full frontend/backend catalog (AgentTrove, OpenHands, Hermes, etc.) and the HuggingFace dataset integration | **REJECT (out of scope)** | Domain-specific to agent-trajectory datasets; not relevant to Atlas's construction pipeline as a process pattern. |

## Things NOT to Copy

- **Never take a runtime/build dependency on the `agentir` Python package.** This census is process-pattern study only — Atlas's construction pipeline is not a Python project and has no plausible reason to import this code.
- Do not adopt AgentIR's event/trajectory data model (`EventType`, `ActionKind`, episode/segment/reward vocabulary) as any part of Atlas's ASIR — it models agent *execution history*, not program *static structure*, and the task brief is explicit this is the wrong donor for that concern.
- Do not treat AgentIR's own self-reported scale/reliability claims ("1.7M records, 0 failures," "174 tests passed") as independently verified by this census — no code was executed, per the hard no-execution rule; these are the donor's own README claims, reported here as provenance context, not as verified facts.

## Known Risks / Gaps in This Census

- **This is a documentation-driven census, not a source-code-driven one.** `docs/DESIGN.md`, `docs/PASSES.md`, `docs/DIAGNOSTICS.md`, and `docs/BACKENDS.md` were read in full and are treated as authoritative for the pipeline-staging design; the actual Python implementation under `src/agentir/passes/` (if present) and `src/agentir/dsl/compiler.py` were only listed, not read line-by-line, so it is **not verified that the shipped code fully matches the documented design** — a real gap, flagged explicitly rather than assumed away. A `src/agentir/passes/` directory was not confirmed to exist in the top-level listing gathered (dialects/dsl/frontends/ir were confirmed; the pass-manager implementation itself was not directly located and read).
- `MANIFEST.md`'s "does not include src/tests" claim, contradicted by this clone's actual contents, was flagged above but not root-caused (e.g. whether it's leftover boilerplate from a documentation-only release process) — noted as a provenance oddity, not resolved.
- The DSL subsystem (`src/agentir/dsl/`, 14 dedicated DSL docs) was inventoried but not deep-censused — it may contain additional interoperability-envelope lessons (declarative format definition) not captured here.
- No independent verification that AgentIR's own test suite actually passes at this pin — per the hard no-execution rule, nothing was run; the "174 tests passed" figure is a README claim carried through as provenance context only.
