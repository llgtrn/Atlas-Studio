# Ops Deployment: Local and Cloud

An Ops must have an independent deployment story and an optional connected deployment story.

## Deployment profiles

At minimum define:

```text
LOCAL_DEV
LOCAL_OPERATOR
STANDALONE_SERVER
CONNECTED_TO_CHRONICA
```

Optional profiles may include multi-node/cloud/edge deployments.

## Local operator profile

Designed for a developer/operator using Claude Code or another local client.

Typical shape:

```text
Ops DB/runtime
+ MCP stdio server
+ optional local UI
+ provider credentials from local secret mechanism
```

Chronica is not required.

## Standalone server profile

```text
Ops API/UI/MCP(optional network)
workers
DB/cache/object storage as required
local auth/policy
backup/recovery
observability
```

## Connected profile

Adds:

```text
Chronica bridge
identity/resource mapping config
authority/delegation config
event/evidence sync
reconciliation worker
connection health/status
```

The connected profile should be an additive deployment composition, not a separate fork.

## Config separation

Keep:

```text
product config
provider config
secret config
Chronica connection config
MCP transport config
UI host config
```

separate enough that enabling one integration does not require rewriting another.

## Version pinning

A deployment should be able to report:

```text
Ops source version
DB schema version
semantic contract version
MCP contract/protocol compatibility
UI package/version
Chronica bridge version
connected Chronica contract version
donor lineage version
```

## Health

Expose health separately for:

```text
Ops core
DB
workers
provider adapters
MCP server
Chronica bridge
sync/reconciliation
```

Chronica bridge unhealthy must not be reported as the whole standalone product being dead unless product policy actually requires connected operation.

## Rollback

Rollback planning includes:

```text
app version rollback
schema forward/backward compatibility
worker compatibility
MCP contract compatibility
UI host compatibility
Chronica bridge compatibility
queued outbox/reconciliation state
```

Do not roll back code across irreversible schema changes without a recovery plan.

## Network MCP

If exposing MCP over a network, follow the current MCP transport/authentication requirements, bind deliberately, validate origins where applicable, and never assume localhost security when listening on broader interfaces.

## Secrets

Chronica credentials, provider credentials, OAuth tokens, exchange/API keys, and private keys stay in secret management appropriate to the deployment. They are not canonical graph content and not frontend state.

## Deployment independence test

A release is deployable only when disabling/removing Chronica configuration still leaves the documented standalone profile functional.
