# Ops Security and Sovereignty

Standalone composability must not weaken security. Connected composability must not widen the Chronica/Holding trust boundary.

## Standalone security

Each Ops owns and documents:

```text
identity/authentication
local authorization/policy
secret storage
session/token handling
transport security
provider credential handling
audit/evidence
backup/recovery
admin/diagnostic privilege
```

## Connected sovereignty

When connected to Chronica, the Ops receives only the minimum authorized projection needed for its purpose.

Do not implement:

```text
Chronica DB dump -> Ops
Holding memory dump -> Ops
Ops DB dump -> external AI
shared master credential across Ops
```

## Context disclosure

If the Ops provides AI/MCP-assisted analysis, distinguish:

```text
local durable state
Ops-authorized context
Chronica ContextEnvelope
external model disclosure
```

External providers receive only policy-gated context appropriate to the task/provider.

## MCP security

MCP is an operational protocol, not an authorization bypass.

- local stdio runs under the invoking user's local security context plus Ops policy;
- network transports require deliberate bind/auth/origin/security controls;
- privileged tools require explicit authorization;
- diagnostic/raw tools must not be exposed as ordinary tools by convenience;
- secrets never appear in resource listings or normal logs.

## Chronica authority

Connected effectful actions may require Chronica authority. The Ops must verify/consume the decision/delegation in a replay-safe way.

Do not trust a frontend or MCP client claim such as `authorized=true` without server-side verification.

## Holding isolation

Shared infrastructure must preserve tenant/Holding boundaries for:

```text
DB queries
cache keys
object storage
search indexes
background jobs
MCP sessions
telemetry
exports
AI context
sync/reconciliation
```

## Provider isolation

Provider adapters receive only the credentials/data required for that provider operation. Avoid one global integration token with unnecessary scope.

## Local independence and sovereignty

Standalone Ops may be deployed entirely inside a user's/local organization's environment. Connecting to Chronica is optional; the product must not secretly depend on Chronica-hosted analytics, auth, or telemetry unless that dependency is explicit in the profile.

## Audit

For high-risk effects retain enough evidence to answer:

```text
who requested it
what scope/resource
which authority/policy
what provider call/effect
what result/receipt
what data was disclosed
what version executed
what reconciliation concluded
```

## Rule

> **Lego composability is capability composition, not trust-boundary collapse.**
