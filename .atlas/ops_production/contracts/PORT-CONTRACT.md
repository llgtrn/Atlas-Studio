# Ops Port Contract

All external surfaces MUST converge on one application command/query layer. UI, API, CLI, MCP, workers, and Chronica integration are ports around the same Ops Core; they are not parallel implementations.

## Required shape

```text
                UI
                API
                CLI
                MCP
             Worker/Event
          Chronica Bridge
                 |
                 v
       Application Command/Query Layer
                 |
          Domain / Policy Logic
                 |
      Persistence + Provider Ports
```

## Hard invariant

```text
ONE OPS CORE
ONE COMMAND/QUERY SEMANTICS
MANY PORTS
```

Forbidden:

```text
MCP tool -> raw database mutation
UI route -> provider API bypassing application service
Chronica bridge -> separate business implementation
worker -> private domain transition not reachable through canonical application semantics
API handler -> copy/pasted validation/policy/state machine
```

## Query contract

Queries are non-effectful reads over authorized scope.

A query request should carry as applicable:

```text
query type
principal
purpose/scope
tenant/Holding context
filter/page/cursor
consistency/freshness expectation
correlation id
```

Query results should expose enough metadata to distinguish fresh canonical/local state from stale/rebuildable projection state.

## Command contract

Commands represent requested effects. A command should carry:

```text
command type
principal
scope/purpose
target resource
parameters
idempotency key
correlation id
causation id when derived
expected version/precondition when needed
authority context
```

The application layer owns validation, authorization/policy invocation, domain invariants, persistence, outbox/evidence creation, and deterministic error semantics.

## Surface responsibilities

### UI

Transforms human interaction into command/query envelopes and renders lifecycle/evidence. Browser state is not authority.

### API/CLI

Transport/parsing/authentication concerns only. They do not own business transitions.

### MCP

Maps tools/resources/prompts to the same command/query layer. MCP metadata or tool annotations are not authorization.

### Workers

Consume durable events/jobs and invoke the same application/domain owners. Retry behavior must preserve idempotency and correlation.

### Chronica Bridge

Maps shared semantics and connected authority/execution/evidence. It MUST NOT become a second domain service layer.

## Provider access

External SaaS, database, queue, machine, and other provider mechanics sit behind ports/adapters owned by the Ops Core. A surface may select a transport, but it does not directly own provider business behavior.

## Acceptance

For each public workflow prove at least:

```text
same command/query owner used by UI and MCP where both expose it
same domain transition used by standalone and connected mode
same idempotency/error semantics across ports
no surface-specific authorization bypass
no duplicate persistence mutation path
```

A production Ops that has feature parity by copying logic into each interface is not compliant.
