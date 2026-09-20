---
id: ops-production-contracts-integration-versioning
type: reference
status: active
canonical: true
---
# Ops Integration Versioning Contract

Ops releases, Chronica releases, MCP contracts, UI host contracts, and event schemas evolve independently. Do not bind compatibility to one monolithic version number.

## Required version dimensions

Every production Ops manifest declares at least:

```text
ops_version
ops_protocol_version
chronica_integration_version
event_envelope_version
mcp_contract_version
ui_host_contract_version
```

Additional provider/schema versions may be declared where necessary.

## Why independent versions matter

```text
BnbOps v8
HelpdeskOps v14
Chronica v31
MCP contract v2
Event envelope v1
```

must be able to coexist when their declared contracts are compatible.

## Compatibility rule

Connection begins with explicit contract negotiation or validation. Unsupported major versions fail closed with an actionable incompatibility error.

Do not silently:

```text
drop unknown required fields
reinterpret action semantics
fall back from Chronica authority to local authority
accept an incompatible event schema
mount an incompatible UI host contract
```

## Version policy

Use semantic compatibility principles:

```text
MAJOR   breaking semantic/behavioral change
MINOR   backward-compatible capability/field addition
PATCH   compatible correction with no contract break
```

A field addition is not automatically backward-compatible if old consumers would perform an unsafe default.

## Compatibility matrix

Each Ops release should maintain/test a compact matrix:

| Ops release | Chronica contract | MCP contract | Event envelope | UI host | Status |
|---|---|---|---|---|---|
| 1.4.x | 1.x | 1.x | 1.x | 1.x | supported |

This matrix may be generated from manifest/test evidence but must not claim untested compatibility.

## Migration

Breaking contract change requires:

```text
new version
migration/translation strategy where feasible
producer + consumer rollout order
rollback plan
test fixtures for old/new versions
explicit retirement boundary
```

Dual-read/dual-write compatibility is temporary migration machinery, not permanent architecture.

## Chronica integration

Chronica must integrate through the declared contract version rather than depending on an Ops internal database layout. Replacing one Ops implementation with another should remain possible when the same external semantic contract is satisfied.
