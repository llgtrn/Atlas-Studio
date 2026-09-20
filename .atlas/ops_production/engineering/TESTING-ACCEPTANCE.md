# Ops Testing and Acceptance

Production readiness must prove both standalone usefulness and Chronica-connected convergence.

Ops testing inherits `docs/architecture/ENGINEERING-CONTRACT.md`: where an Ops workflow maps to a Chronica architecture primitive, the same semantic equation/invariant/performance contract applies. Ops does not create a separate performance universe.

## Test layers

```text
unit / domain invariants
property / generative invariants where applicable
application command/query tests
adapter/provider contract tests
persistence/migration tests
MCP tool/resource tests
API/CLI tests
UI/workspace tests
Chronica bridge contract tests
reconciliation/recovery tests
controlled workflow benchmark / scale fixture
end-to-end vertical proofs
production metric/evidence where claimed
```

## Standalone suite

Every release should prove:

```text
cold install/bootstrap
schema migrate
local auth/policy
critical read workflows
critical effectful workflows
MCP local operation
UI standalone operation
restart/recovery
backup/restore or equivalent recovery path
provider retry/idempotency behavior
```

## Connected suite

Prove:

```text
identity/resource mapping
shared state projection
Chronica authority decision path
WorkRequest/WorkRun correlation
provider effect execution
evidence publication
canonical event/reconciliation
reconnect after temporary outage
conflict/duplicate webhook handling
unknown-outcome handling
```

## Architecture contract inheritance

For each connected public workflow, identify the relevant Chronica architecture owner and any applicable stable contract IDs:

```text
ARCH-EQ-*
INV-*
ARCH-PERF-*
```

Examples:
- an effectful command inherits execution/authority/evidence invariants;
- a provider-backed action inherits reconciliation/unknown-outcome semantics;
- an AI-assisted workflow inherits ANALYZE != ACT and context-disclosure invariants;
- a machine workflow inherits the machine-safety envelope;
- a performance-sensitive absorbed workflow inherits the architecture performance contract rather than inventing an Ops-only metric with different semantics.

No Ops workflow may claim a performance win by bypassing Chronica authority, evidence, reconciliation, context disclosure, or other hard invariants.

## Surface parity

For each public workflow, test at least the shared application command/query once and verify that UI/API/MCP adapters do not change business semantics.

Do not require every surface to duplicate every test. Test the core deeply, then test each adapter's mapping.

For a measured workflow, compare the same semantic operation across surfaces. Transport overhead may differ, but changing business/authority semantics to improve one surface's latency is not parity.

## Donor regression

A deep-fork must maintain a donor-to-Ops behavior matrix. For every KEEP/ADAPT workflow, have executable or reproducible evidence that intended behavior survived.

When claiming a donor performance improvement, pin donor SHA, Ops SHA, fixture/workload, environment class, unit, and metric direction. Functional parity remains a prerequisite.

## Integration contract tests

Prefer contract fixtures for:

```text
semantic resource mapping
action mapping
event payload
evidence receipt
error/status vocabulary
version negotiation
architecture invariant mapping
```

These fixtures should be usable by both the Ops repo and Chronica-side adapter tests.

## Performance acceptance

Use the same maturity labels as Chronica architecture performance contracts:

```text
STRUCTURAL
metric/workload/runtime owner defined; no numeric claim

MEASURED
controlled benchmark + fixture + threshold + baseline identity exist

PRODUCTION
runtime metric/evidence exists for the same semantic quantity or an explicit mapping
```

Do not call a workflow performance-proven from a screenshot, ad-hoc developer timing, or an unrelated HTTP benchmark.

For connected mode, decompose latency where useful:

```text
local application/domain
+ Chronica semantic/authority admission
+ provider/adapter effect
+ evidence/reconciliation
```

This makes the cost of governance visible without creating pressure to bypass it.

## Negative-path tests

Required where relevant:

```text
unauthorized principal
wrong Holding/scope
expired delegation
idempotency replay
provider timeout
provider accepted but response lost
duplicate inbound event
stale version/concurrent update
Chronica unavailable
local DB restart
partial sync
forbidden disclosure
```

## MCP acceptance

Verify local MCP server can run without Chronica and effectful MCP tools use the same policy/application path as other surfaces.

## UI acceptance

Verify the same Ops workspace works in standalone host and Chronica host, with authority/error/stale/unknown outcome rendered correctly.

## CI acceptance order

Prefer fail-fast order:

```text
contract/topology
-> domain + architecture invariants
-> authority/safety/replay/reconciliation
-> affected measured performance contracts
-> broader integration/e2e
-> release/canary/production evidence
```

Performance does not compensate for an invariant failure.

## Maturity labels

Use honest labels such as:

```text
DONOR_BASELINE_PROVEN
VERTICAL_REFACTOR_PROVEN
STANDALONE_PROVEN
MCP_LOCAL_PROVEN
CHRONICA_MAPPING_PROVEN
CONNECTED_AUTHORITY_PROVEN
EVIDENCE_RECONCILIATION_PROVEN
UI_EMBED_PROVEN
RECOVERY_PROVEN
PERFORMANCE_MEASURED
PERFORMANCE_PRODUCTION_EVIDENCED
PRODUCTION
```

`PERFORMANCE_MEASURED` requires controlled benchmark evidence; `PERFORMANCE_PRODUCTION_EVIDENCED` requires production observability/evidence for the same semantic metric. Neither label overrides functional/authority/safety maturity.

Do not call an Ops `100% integrated` until the integration checklist and public-surface parity table are complete.
