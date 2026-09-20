---
id: ops-production-architecture-mcp-server-contract
type: reference
status: active
canonical: true
---
# MCP Server Contract

Every Ops may expose an MCP server as an independent operational surface. This MCP server is **not dependent on Chronica** and can be used by Claude Code or another MCP client to operate the Ops locally.

## Core rule

MCP tools/resources call the same Ops application/domain layer as UI/API/CLI.

Wrong:

```text
MCP tool -> donor-specific direct DB/provider mutation
```

Target:

```text
MCP request
-> principal/context
-> Ops command/query service
-> local or connected policy/authority adapter
-> domain transition/provider execution
-> evidence/result
-> MCP response
```

## Local use

For local Claude Code workflows, support a local transport appropriate to the current MCP specification. `stdio` is the preferred baseline because the client can launch the server as a subprocess. A networked MCP transport may be added when needed.

For local network transport, bind conservatively and apply authentication/origin/security rules appropriate to the current MCP specification. Do not expose a local MCP server broadly by accident.

## Standalone independence

The MCP server must start and provide useful Ops operations when:

```text
Chronica URL = absent
Chronica credentials = absent
Chronica process = absent
```

The only required dependency is the Ops itself and its configured local/provider dependencies.

## Connected behavior

When Chronica is connected, the same MCP tool should not fork into a second business implementation.

Example:

```text
mcp: booking.update_price
-> Ops UpdatePrice command
-> ConnectedAuthorityAdapter
-> Chronica WorkRequest/Authority when policy requires
-> provider execution
-> evidence/reconciliation
```

Standalone deployment may instead use `LocalAuthorityAdapter`.

## Tool design

Prefer semantic tools:

```text
reservation.list
reservation.get
reservation.modify
message.reply
invoice.approve
customer.merge
position.close
```

Avoid exposing raw donor internals as permanent contract:

```text
execute_sql
call_vendor_endpoint
write_raw_table
run_internal_controller_method
```

Raw diagnostic/admin tools, if needed, must be explicitly privileged and classified local/developer-only.

## Resources

MCP resources may expose read-only structured views of Ops state. They are projections, not authority.

## Effect safety

Effectful tools need:

```text
principal
scope
input validation
policy/authority
idempotency where applicable
explicit result state
evidence/receipt
unknown-outcome handling
```

## Secrets

Local stdio credentials should come from environment/OS secret management or the Ops' secure config mechanism. Never return secrets through ordinary MCP resources or logs.

## MCP does not become the core API

MCP is one consumer-facing protocol. The Ops domain/application API must remain transport-independent so the product can also serve UI, API, workers, tests, and Chronica bridge.

## Acceptance

```text
[ ] server starts without Chronica
[ ] local Claude Code can discover/use read tools
[ ] effectful tool uses same Ops command path as UI/API
[ ] connected mode can route authority without changing tool semantics
[ ] MCP process restart does not lose durable business state
[ ] errors/unknown outcomes are explicit
[ ] secrets are not exposed
```

Official MCP specifications currently define `stdio` and Streamable HTTP as standard transports; implementation should pin/test the protocol version it actually supports rather than relying on undocumented behavior.
