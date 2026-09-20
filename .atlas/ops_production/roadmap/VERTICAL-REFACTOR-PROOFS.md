---
id: ops-production-roadmap-vertical-refactor-proofs
type: reference
status: active
canonical: true
---
# Vertical Refactor and Absorption Proofs

Do not refactor an Ops in one giant rewrite. Prove one real production vertical at a time from donor behavior to standalone Ops to Chronica-connected execution, then absorb the mature source into canonical Chronica ownership.

## P0 — donor reality

```text
clone donor
pin SHA/license
build/run baseline
census product/schema/routes/jobs/UI/integrations
```

Exit: `DONOR_BASELINE_PROVEN` or an explicit donor limitation record.

## P1 — one native Ops read vertical

```text
donor data/provider
-> Ops persistence/adapter
-> Ops application query
-> API or UI
```

Exit: no greenfield mock standing in for donor behavior.

## P2 — one native Ops effect vertical

```text
UI/API
-> Ops command
-> local policy
-> transaction/provider effect
-> receipt/evidence
```

Exit: `STANDALONE_PROVEN` for one effectful workflow.

## P3 — local MCP vertical

```text
Claude Code
-> local MCP
-> same Ops command/query
-> same policy/evidence
```

Exit: `MCP_PROVEN` with Chronica absent.

## P4 — Chronica resource mapping

Map the vertical's principal/resource/state/events into Chronica semantics.

Exit: stable IDs and mapping tests.

## P5 — Chronica authority/execution/evidence

```text
connected caller
-> Ops command
-> Chronica WorkRequest/Authority
-> Ops execution
-> receipt/evidence
-> reconciliation/canonical event
```

Exit: `AUTHORITY_PROVEN` + `EVIDENCE_PROVEN`.

## P6 — UI dual host

Run the same workspace where required:

```text
standalone Ops shell
Chronica shell/projection host
```

Exit: `UI_DUAL_HOST_PROVEN`, no duplicated domain UI logic.

## P7 — outage/reconnect

Prove connected Ops behavior during Chronica outage and deterministic reconciliation on reconnect.

Exit: `OFFLINE_RECONNECT_PROVEN`; no silent authority downgrade and no lost/duplicated effects.

## P8 — donor path retirement inside Ops

Migrate remaining callers for the selected slice and delete obsolete donor duplicate paths only after tests/callers prove convergence.

Exit: one fully assimilated donor-derived vertical inside the Ops incubator.

## P9 — expand public parity

Repeat P1-P8 until:

```text
100% public workflows mapped/tested
OR explicitly justified local-only/retired
```

Exit: public-surface parity matrix complete.

## P10 — incubation production hardening

Prove:

```text
upgrade/migration
backup/restore
recovery
security review
license/provenance review
MCP contract compatibility
UI host compatibility
Chronica bridge compatibility
observability/SLOs
```

Exit: `ABSORPTION_READY` when the source is stable enough for terminal convergence.

## P11 — freeze absorption baseline

Record:

```text
Ops exact source SHA
Chronica exact target SHA
donor exact SHA/license
public workflow matrix
standalone baseline
MCP baseline
connected baseline
contract versions
```

Exit: immutable differential baseline for the absorption run.

## P12 — complete source intake and map

Use `docs/ops_production/ABSORB-INTO-CHRONICA-GOAL.md`.

Account for the entire tracked Ops source tree and classify every meaningful responsibility into:

```text
CORE
RUNTIME
ADAPTER
APP_SURFACE
GRAPH
BINDING
OPS_DEPLOYMENT
TOOLING
PROVENANCE
EXTERNAL_PROVIDER
RETIRE
```

Exit: `ABSORPTION_CENSUS_COMPLETE` + `ABSORPTION_MAP_COMPLETE`.

## P13 — native Chronica absorption

Migrate verticals from the frozen Ops source into existing Chronica owners.

```text
source semantics       -> core
source durable runtime -> runtime
provider mechanics     -> adapter
UI/API/CLI/MCP         -> thin apps
static declarations    -> graph/bindings
deploy/tooling         -> ops/tools
```

Migrate real callers before removing duplicate paths.

Do not create a permanent `organs/`, product root, or copied donor monolith as terminal architecture.

Exit: `CHRONICA_NATIVE_ABSORPTION_PROVEN` for the absorbed scope.

## P14 — standalone artifact parity from Chronica

Build the standalone product surfaces from the Chronica monorepo.

Where promised by the old contract, prove the artifact works without a separate Chronica server process by composing native libraries/runtime components locally.

Exit: `STANDALONE_ARTIFACT_PARITY_PROVEN`.

## P15 — standalone MCP parity

Differential-test the old Ops MCP baseline against the Chronica-built MCP distribution.

Unless intentionally versioned, preserve:

```text
tool/resource names
schemas
risk classes
errors
idempotency
authority behavior
evidence semantics
transport behavior
```

Exit: `MCP_STANDALONE_PARITY_PROVEN`.

## P16 — connected parity

Prove the Chronica-native implementation retains connected identity/resource/authority/execution/evidence/reconciliation semantics without a separate Ops bridge owning duplicate business logic.

Exit: `CONNECTED_PARITY_PROVEN`.

## P17 — duplicate extinction

Delete/bound all obsolete duplicate source paths and migration shims whose callers/tests have converged.

Preserve donor and Ops provenance.

Exit: `DUPLICATE_SOURCE_PATHS_RETIRED` + `PROVENANCE_PRESERVED`.

## P18 — retire independent Ops source universe

When all retained behavior is Chronica-native and standalone distributions build from Chronica:

```text
archive / mirror / mark read-only the old Ops repo according to human policy
stop canonical feature development there
continue from Chronica as sole native source universe
```

Exit: `OPS_SOURCE_UNIVERSE_RETIRED`.

## Rule

> **The unit of incubation progress is a real donor-derived vertical. The terminal unit of convergence is that same proven behavior absorbed into canonical Chronica owners with standalone/MCP compatibility preserved and the duplicate source universe extinguished.**
