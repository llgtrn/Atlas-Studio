# MCP Authority Classification

An Ops MCP server is a first-class standalone product surface for Claude Code and other MCP clients. It may run with Chronica completely absent during incubation, and it may remain independently runnable **after source absorption** as a thin distribution built from the Chronica monorepo.

MCP is still a transport/interface layer, not an authority root.

## Tool classes

Every MCP tool MUST be classified.

### READ

Non-effectful retrieval of authorized state.

Examples:

```text
reservation.get
invoice.search
conversation.list
```

### ANALYZE

Produces analysis, recommendation, simulation, draft, or plan without causing a real external effect.

Examples:

```text
pricing.recommend
invoice.detect_anomaly
reservation.summarize
```

### EFFECTFUL

Changes local or external business reality.

Examples:

```text
message.send
reservation.cancel
price.update
invoice.approve
```

### HIGH_RISK_EFFECTFUL

Financial, physical, irreversible, safety-relevant, credential/security, or other high-risk effects requiring stronger policy/approval/evidence.

Examples:

```text
payment.refund
funds.transfer
machine.control
credential.rotate
```

## Standalone mode during incubation

```text
Claude Code
-> Ops MCP
-> local principal/auth context
-> local Ops policy/authority
-> application command
-> domain/provider effect
-> evidence/audit
```

Chronica is not required.

## Connected mode during incubation

For shared/Chronica-governed effects:

```text
Claude Code
-> Ops MCP
-> same Ops command layer
-> Chronica authority/execution path
-> Ops/provider execution
-> evidence/reconciliation
```

The MCP server MUST NOT provide a hidden bypass tool that calls the provider directly around connected authority.

## After source absorption into Chronica

Retiring the independent Ops source repository MUST NOT silently retire its MCP product contract.

The post-absorption shape is:

```text
Claude Code / MCP client
-> <ops-id>-mcp distribution built from Chronica monorepo
-> thin MCP registration/serialization layer
-> native Chronica command/query/runtime owners
-> local or connected policy/persistence/provider adapters by deployment mode
```

The MCP distribution may run without a separately running Chronica server process when the pre-absorption standalone contract promised independent operation. It may embed/compose the required Chronica libraries/runtime components in-process.

What disappears is the duplicate source implementation, not the MCP capability.

## Post-absorption compatibility gate

Unless a breaking version change is explicitly approved and declared, preserve:

```text
tool names
resource names
input/output schemas
classification
principal/scope requirements
error/status vocabulary
idempotency semantics
standalone authority behavior
evidence/receipt behavior
unknown-outcome behavior
stdio/network transport behavior promised by contract
```

Differential contract tests should compare the frozen pre-absorption MCP baseline with the Chronica-built MCP distribution.

An MCP tool that works only because the old Ops repo is still running has not been absorbed.

## Tool declaration

Each tool should document as applicable:

```text
classification
required principal/scope
read vs effect semantics
idempotency behavior
authority source by mode
approval requirement
expected evidence
error and unknown-outcome behavior
sensitivity/disclosure
```

Tool descriptions and protocol annotations help clients understand behavior but are not enforcement. Enforcement lives in local/native Chronica policy and application/runtime services.

## Local transport

Local `stdio` is a preferred baseline for Claude Code-style operation because it minimizes exposed network surface. Network MCP transports require explicit authentication, origin/network controls, secret handling, and audit.

## Failure semantics

Do not collapse these into one generic MCP error:

```text
forbidden
authority_unavailable
approval_required
stale_precondition
provider_unavailable
unknown_outcome
reconciliation_required
unsupported_contract_version
```

## Invariants

```text
MCP MAY OPERATE WITHOUT A RUNNING CHRONICA SERVER WHEN STANDALONE CONTRACT REQUIRES IT
MCP NEVER BECOMES THE BUSINESS CORE
MCP NEVER MINTS AUTHORITY
CONNECTED MCP NEVER BYPASSES CHRONICA-GOVERNED EFFECTS
SOURCE ABSORPTION MUST NOT BREAK STANDALONE MCP COMPATIBILITY
POST-ABSORPTION MCP MUST BUILD FROM CANONICAL CHRONICA SOURCE, NOT A DUPLICATE OPS IMPLEMENTATION
```
