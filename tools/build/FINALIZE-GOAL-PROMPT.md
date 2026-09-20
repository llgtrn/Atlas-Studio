# Chronica — Finalize Goal Prompt (parallel subagents · full verify · tracking · audit)

> Paste this as a task prompt to drive the **entire** Chronica build to 100% verified, with parallel
> subagents, percentage tracking, dependency-enforced ordering, money-safety, and adversarial audit.
> It is the single master prompt composing the **4-layer execution stack**: `docs/` (source of truth) ·
> **spec-kit** (per-slice structure) · **superpowers** (execution methodology) · **parity harness** (the gate),
> under the constitution (`.specify/memory/constitution.md`). See [`docs/runbooks/spec-kit-integration.md`](../../docs/runbooks/spec-kit-integration.md).

---

## GOAL

Drive Chronica from spec to a verified, money-safe, AI-native enterprise OS. **DONE =** `pnpm parity:check`
prints ✓ PASS at verified=100% / 0 blockers, every money/side-effect capability has a
`financialControlTestId`, `pnpm parity:progress` shows 100% across SLICES + DOCS SPEC (all 148 files) +
MONEY, and `cargo test --workspace` + `pnpm test` are green. Track progress as PERCENTAGES, enforce
dependency order via the tooling, and adversarially audit every capability.

## HOW TO RUN THIS (read first — honest)

**This is a months-long build; it does NOT finish in one paste.** Context limits, real human/environment
decisions (SQL layer, provisioning ClickHouse/Coolify, money sign-offs), and sheer scale mean it runs
**wave-by-wave across many sessions**, with you in the loop. The parity registry is the durable memory between
sessions; this prompt is the stateless playbook.

**Each session:**
1. Run **`pnpm parity:next`** → it prints where you are (the %s), the next ready wave, and the exact 4-layer
   commands to run (spec-kit → superpowers → parity). This is your "paste-and-go" starting point every time.
2. Do that wave (Phase 1 kernel first if `kernel != ready`; then implement the ready crates per §ORCHESTRATION B).
3. Run **`pnpm parity:progress`** → report the climbing %s; resolve any blocker the gate names.
4. **Stop for review / decisions, then repeat from step 1** next session (or next wave). The agent should pause
   and ask whenever a real decision or environment action is required — it must not guess on those.

It is finished only when `pnpm parity:check` is ✓ PASS at 100%. Treat "100%" as the destination, `parity:next`
as the GPS, and each wave as one leg of the trip.

## THE 4-LAYER EXECUTION STACK (each layer one job — don't blur them)

| Layer | Job | Tool |
|---|---|---|
| **Design source of truth** — *what* to build | the real specs | `docs/` (155 specs, 36 slices) |
| **Per-slice structure** — spec→plan→tasks scaffold | makes /plan run | **spec-kit** (`specs/<slice>/`, `/speckit.*`, `pnpm speckit:slice N`) |
| **Execution methodology** — *how* to build each task well + recover when reality bites | adaptive + decisive | **superpowers** v5.1.0 skills (subagent-driven-development, test-driven-development, systematic-debugging, verification-before-completion, requesting-code-review, using-git-worktrees, writing-plans, executing-plans) |
| **Done-ness / order / money gate** — *whether* verified | the authority | **parity harness** (`pnpm parity:*`) |

## READ FIRST (authoritative)

- `.specify/memory/constitution.md` — binding principles (parity is the gate of record; `docs/` is the spec
  source of truth; Rust-only backend; money-safety; capability-parity; neutral naming; phase order).
- `docs/runbooks/spec-test-driven-parity.md` — the RED→GREEN→verify→flip→gate loop + dependency ordering + % tracking.
- `docs/runbooks/spec-kit-integration.md` — how spec-kit + superpowers + parity compose per slice (the 4-layer stack).
- The installed **superpowers** skills (auto-trigger; or read `~/.claude/plugins/.../superpowers/.../skills/*/SKILL.md`).
- `docs/implementation/14-vertical-slice-plan.md` (36 slices + sequencing) and `docs/implementation/00..12` (phases).
- `docs/ui/14-ux-evolution-plan.md` — Phase 9 UI (evolve the native Chronica UI, money-first, 24 surfaces).

## THE COCKPIT (use every wave)

```sh
pnpm parity:next       # ⭐ START HERE each session: where am I + next ready wave + exact 4-layer commands
pnpm parity:registry   # regenerate registry (additive; preserves per-spec/per-slice status)
pnpm parity:plan       # dependency-respecting READY wave (workable now) vs blocked
pnpm parity:progress   # PERCENTAGE dashboard: SLICES % + DOCS SPEC % (all 148 files) + per-area/crate/phase + MONEY %
pnpm parity:check      # the gate: ✓ PASS only at 100% verified / 0 blockers
pnpm parity:set slice|spec|prereq <selector> <status> [--test id] [--fin id]   # record progress
pnpm speckit:slice N   # spec-kit: materialize specs/<NNN-slice>/ (pointer to docs/) so /speckit.plan runs
# per-slice ritual: /speckit.plan · /speckit.tasks · /speckit.analyze  +  superpowers skills (TDD, subagent-driven-development, systematic-debugging, verification-before-completion)
```

## LOCKED INVARIANTS (the constitution + gate enforce them)

- **Rust-only backend/kernel** (`chronica-*`); TS/React = UI only; the TS/Playwright browser capability is the
  ONLY non-Rust, non-UI execution, behind a Rust boundary; **NO production Python**; ClickHouse/Coolify external.
- **Capability parity, NOT repo parity.** `donorRefs` are provenance only — never spawn "port repo X" agents.
- **Money safety:** every spend/commit = `SideEffectAction` → `chronica-policy` → `chronica-approvals` with a
  dollar threshold, writes `CostRecord`, audit-chained, reversible/irreversible flagged; money capabilities need a
  financial-control test (gate→approve→execute→record→audit→reconcile) to be `verified`.
- **Neutral naming** in all executing/generated code; no donor-branded routes/symbols/iframes.
- **TDD mandatory:** failing test first (tagged capabilityId) → implement minimum → run the slice Verify cmd → flip.
- **`docs/` is the spec source of truth;** `/speckit.specify` does NOT author new product specs.

## ORCHESTRATION (Workflow tool — parallelize by CRATE in dependency waves; full implement→verify→audit rigor)

**Why crate-keyed, not slice-keyed:** a vertical slice cuts *across* crates (it touches `chronica-core`, the
schema, `chronica-api`, a domain crate, and the UI), so parallel slice-agents collide on the same shared files.
Independent `chronica-*` crates share only `chronica-core` + the DB schema, so they parallelize with far fewer
merge conflicts — same assurance (3 review passes per work unit), more real throughput. A "work unit" below = the
capability rows whose `owningCrates` include a given crate.

**Dependency ordering is machine-enforced — use it.** The registry encodes `dependsOn` (+ `infraDeps`) per slice
and status-tracked prereq nodes (`phase0.kernel-skeleton`, `infra.clickhouse|coolify|postgres`). `pnpm parity:plan`
prints the ready wave vs blocked; the gate FAILS any slice marked green/verified before its deps are
`verified`/`ready` (ORDER-VIOLATION). Drive batches strictly from `parity:plan` — never hand-pick a blocked slice;
as each verifies, the next wave opens.

### PHASE 1 — SERIAL HARD BARRIER (1 agent, no fan-out)
Build the Rust kernel skeleton: the Cargo workspace, `chronica-core` (shared types, IDs, error model, event-bus +
persistence traits), `chronica-api` strangler over the existing TS server, the Rust SQL layer against the SHARED
PostgreSQL schema, and the kernel↔browser-capability transport. Gate: `cargo build --workspace` green + one real
endpoint served through Rust. Then `pnpm parity:set prereq kernel ready`. Do NOT fan out — it's a true serial
dependency; parallel agents only conflict. (Spec: `docs/implementation/02-phase-1-rust-kernel.md`,
`docs/architecture/02-monolith-architecture.md`, `03-rust-kernel-plan.md`.)

### THEN loop per wave until 100%:

**A) PLAN (1 agent).** `pnpm parity:plan` → group the ready capability rows by `owningCrates` into dependency-ordered
crate waves (build order from `docs/architecture/03-rust-kernel-plan.md`): wave 1 = leaf crates on `chronica-core`
only (`chronica-tools`, `chronica-policy`, `chronica-artifacts`, `chronica-observability`); wave 2 =
`chronica-runtime`, `chronica-scheduler`; wave 3 = `chronica-workflows`, `chronica-worker-supervisor`,
`chronica-approvals`; wave 4 = `chronica-company`, `chronica-strategy`, `chronica-osint`, `chronica-internet-hand`,
`chronica-memory`; wave 5 = `chronica-integrations`; then project-template + UI capabilities. Provision any infra
prereq a wave needs first (`pnpm parity:set prereq infra.clickhouse|coolify|postgres ready`).

**B) IMPLEMENT — one IMPLEMENTER per ready crate, in PARALLEL, each in its own git worktree** (`isolation:'worktree'`
so parallel agents never stomp shared files). Each drives ALL its in-wave capability rows through the **4-layer stack**:
`docs/` = source of truth · **spec-kit** = per-slice structure · **superpowers** = execution methodology · **parity** = gate.
Per slice:
  1. **Materialize the spec-kit feature shell:** `pnpm speckit:slice N` → creates `specs/<NNN-slice>/spec.md` (a THIN
     pointer that links to + summarizes the slice's canonical `docs/` specs; it does NOT re-author requirements) and
     pins `specs/feature.json` so `/speckit.plan` runs without a branch. This is what makes spec-kit a real engine while
     `docs/` stays the source of truth (Principle 0).
  2. **spec-kit structure:** `/speckit.plan` (plan AGAINST the slice's `docs/` specs: §6 target crate, §7 data models,
     §8 API, §10 runtime, §11 approval, §15 verify; honor Principles 2–6) → `/speckit.tasks` → `/speckit.analyze`.
  3. **superpowers execution methodology — USE THE SKILLS, don't hand-roll:** drive the tasks with
     `superpowers:subagent-driven-development` (dispatch fresh agents per task with two-stage review) +
     `superpowers:test-driven-development` (RED→GREEN→REFACTOR: failing test tagged with the `capabilityId` first, for
     EVERY row money or not, watch it fail, minimal code, watch it pass) + `superpowers:using-git-worktrees`. When ANY
     test fails or behavior is unexpected, STOP and use `superpowers:systematic-debugging` (Iron Law: no fixes without
     root-cause first) — this is the **flexible, decisive** handling of the unpredictable variables; do not patch symptoms.
  4. **Money rows** (`requiresFinancialControlTest`): also write + run the financial-control test
     (gate→approve→execute→CostRecord→audit→reconcile).
  5. **Decisive completion:** before claiming done, run `superpowers:verification-before-completion` + the row's
     `verifyCommands` (slice §15). Then record: `pnpm parity:set slice N verified --test … [--fin …]` and
     `pnpm parity:set spec <area/file> verified --test …` for the docs specs it satisfies; flip the parity ledger.
  After a worktree's crate is implemented, merge it back; resolve any `chronica-core`/schema conflicts in the
  orchestrator before the next wave (the only place conflicts can occur, rare because crates are leaf-isolated).

**C) VERIFY — one VERIFIER per crate**, the moment its implementer returns (pipeline, not barrier). Independent, from
a clean merge, using `superpowers:requesting-code-review` (severity-based blocking): re-run every row's verify command
+ `pnpm parity:check --slice N`; confirm each test exercises real behavior (not a stub), commands pass, money chain
real. Any fail → bounce to the implementer with the specific gap.

**D) AUDIT — one adversarial AUDITOR per crate** (default skeptical; tries to REFUTE "truly verified"). Per row: test
asserts real behavior (not `expect(true)`/tautology); NO production Python; neutral / `chronica`-native naming (no
donor-brand symbols); money/side-effect action genuinely blocks→approves→records→audits→reconciles; spec §11 approval
rules actually enforced in code (not just documented); no silent error-swallowing; capability registered under a
neutral ID. Any row refuted by ≥1 strong reason → revert it to `red`, bounce to implementer. **Money/side-effect rows
get a SECOND auditor** focused only on the financial-control chain.

**E) COMPLETENESS CRITIC (1 agent, after each wave).** Run `pnpm parity:progress` + `pnpm parity:registry` +
`pnpm parity:check --json`. **REPORT THE CLIMBING PERCENTAGES** — SLICES verified%/weighted%, DOCS SPEC % (all 148
files) + per-area %, MONEY %, per-crate/per-phase %. Flag any MISS (spec with no slice → author the slice+row),
MONEY-GAP, SPEC-NOT-MET, and cross-crate slices (run their `--slice N` gate). Feed the next wave. Loop B→E until
**100% verified / 0 blockers**.

### PHASE 9 — UI
After the backend a surface depends on is verified, evolve the native Paperclip UI per
`docs/ui/14-ux-evolution-plan.md` (Phase-0 design-token + shared-components barrier first, then per-surface
implement→design-critique→money-critique, money surfaces first). Read-only UI may precede write capabilities; the
money critic forbids faking working actions.

### PHASE 10/11
Retire any `temporary_reference_runtime` (port to Rust; audit zero production Python); then cross-crate verification +
financial-control validation + security/OSINT/internet-hand safety review + parity closeout to 100%.

## CONCURRENCY & SPEED

- Phase 1 is serial and unavoidable (the real critical path) — accept it; parallelizing only causes conflicts.
- Within a wave, spawn all ready crate-implementers at once (the workflow cap auto-throttles); verifier+auditor
  pipeline per crate. Never hand-pick a blocked slice (the gate raises ORDER-VIOLATION).

## DEFINITION OF DONE

`pnpm parity:check` → ✓ PASS, verified=100%, 0 blockers; `pnpm parity:progress` shows 100% on SLICES + DOCS SPEC
(148/148) + MONEY; every money capability has a `financialControlTestId`; `cargo build/test --workspace` +
`clippy -D warnings` + `pnpm -r typecheck` + `pnpm test` + e2e green; a full money flow (e.g. supplier order)
blocks→approves→executes→CostRecords→audits→reconciles in an integration test; analytics reconcile to the
Postgres ledger; the UI route audit shows no donor-branded/iframe routes. Verify the running app with the
`verify`/`run` skill — don't just build.

## REPORT (after every wave)

The percentages (climbing), slices + specs shipped, verifier/auditor verdicts, MISS/MONEY-GAP/SPEC-NOT-MET found,
next ready wave. Be honest — failures with output, skips stated, no fake `verified`.

---

### One-line kickoff (paste to start)

> Finalize Chronica per `.specify/memory/constitution.md` (4-layer stack: docs · spec-kit · superpowers · parity).
> Run **`pnpm parity:next`** to see where we are and the next wave; if the kernel isn't ready, build the Phase-1
> Rust kernel skeleton SERIALLY and `pnpm parity:set prereq kernel ready`. Then loop waves: `pnpm parity:next` →
> fan out one implementer per ready crate (git worktree), each running `pnpm speckit:slice N` → `/speckit.plan`/`tasks`/`analyze`
> → superpowers `subagent-driven-development` + `test-driven-development` (RED→GREEN→REFACTOR), `systematic-debugging`
> on any failure (root-cause, not symptom), `verification-before-completion` before done → per-crate verifier +
> adversarial auditor (money caps get a financial-control auditor) → completeness critic reporting `pnpm parity:progress`
> percentages — until `pnpm parity:check` is ✓ PASS at 100% (DOCS SPEC 148/148, MONEY 100%). Capability parity,
> Rust-only, money-safe, neutral-named. Pause for real decisions. Then Phase 9 UI. **It runs wave-by-wave across many
> sessions — `pnpm parity:next` resumes you each time.**

---

## Honest caveats

1. **This is months of engineering, not one paste.** It's a standing instruction run wave-by-wave across many
   sessions. Each wave is bounded and verifiable; the percentage dashboard is how you watch it converge.
2. **Phase 1 (Rust kernel) is the real bottleneck and is serial** — nothing parallelizes before `chronica-core`
   + the strangler exist. The barrier is deliberate; fanning out there only creates merge conflicts.
3. **The tooling makes "fully verify + track + audit" real, not the wording.** `parity:check` mathematically
   refuses "done" until 100%/0-blockers; `parity:plan` enforces order; `parity:progress` is the percentage
   tracker; the per-crate verifier + auditor agents are the audit. The prompt just wires the subagents to those gates.
