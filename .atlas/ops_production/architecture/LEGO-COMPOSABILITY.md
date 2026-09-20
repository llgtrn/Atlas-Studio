---
id: ops-production-architecture-lego-composability
type: reference
status: active
canonical: true
---
# Lego Composability

Ops repositories are production blocks, not temporary staging repos.

## A Lego block has five properties

1. **Independent** — deploys and operates without Chronica.
2. **Composable** — exposes stable ports and semantics.
3. **Replaceable** — another implementation can satisfy the same integration contract.
4. **Inspectable** — version, provenance, state, effects, and health are observable.
5. **Governable** — effectful behavior has identity, policy/authority, idempotency, evidence, and recovery.

## Internal architecture

```text
                 +----------------------+
                 |   Domain / App Core  |
                 +----------+-----------+
                            |
       +--------------------+--------------------+
       |          |          |          |         |
       v          v          v          v         v
      UI         API        CLI        MCP     Scheduler
       |                                             |
       +--------------------+------------------------+
                            |
                    Provider Adapters
                            |
                       external systems

                 Optional Chronica Bridge
                            |
                     Chronica substrate
```

The Chronica bridge is not allowed to become a second implementation of the domain core.

## Port rules

Each public surface adapts to the same commands/queries/events.

Wrong:

```text
UI logic A
MCP logic B
Chronica adapter logic C
```

Target:

```text
UI ----+
API ---+
CLI ---+--> Command/Query Service --> Domain Core
MCP ---+
Chronica Bridge --+
```

## Stable semantic contract

A block should publish a compact semantic contract describing:

```text
identity/principal model
resource types
state/lifecycle
commands
queries
events
evidence
errors/unknown outcomes
idempotency
version compatibility
```

Do not expose internal donor folders as the long-term public architecture.

## Composition between Ops

Standalone Ops may integrate directly through documented APIs/MCP when deliberately deployed that way.

When several Ops are connected to Chronica, shared world semantics should converge through Chronica rather than creating an invisible mesh of competing databases.

```text
BnbOps ----+
HelpdeskOps+--> Chronica world/execution/evidence
FinOps ----+
TradeOps --+
```

Point-to-point provider integration may still exist at adapter level, but must not silently become shared canonical truth.

## Replaceability

Chronica should depend on an Ops semantic integration contract, not its donor identity.

For example:

```text
Helpdesk semantic contract
   <- implementation A: deep-forked Chatwoot
   <- implementation B: future native implementation
```

Replacing the implementation should not require inventing a new Chronica universe.

## Versioning

Track independently:

```text
Ops source version
Ops semantic-contract version
DB schema version
MCP contract version
Chronica bridge version
UI integration version
donor lineage version
```

Compatibility must be explicit; "same repo" is not a version contract.

## Rule

> **A Lego block owns its implementation. Chronica owns the shared connected world. Ports connect them. No port becomes a second product core.**
