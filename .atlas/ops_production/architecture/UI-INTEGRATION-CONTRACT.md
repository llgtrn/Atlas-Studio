# UI Integration Contract

Each Ops UI must support both standalone product use and integration into the Chronica human projection layer.

## One UI product, two hosts

Do not maintain separate UI implementations for standalone and Chronica-connected use.

Target:

```text
Ops UI components/routes/workspaces
        |
        +--> Standalone Ops shell
        |
        +--> Chronica host shell / embedded projection
```

The hosting shell may differ. The domain/workspace implementation should be shared.

## Standalone UI

Standalone mode owns its own:

```text
login/session
navigation shell
scope selection
notifications/inbox if needed
settings
local authority feedback
local evidence/history views
```

## Chronica-hosted UI

When mounted into Chronica, the host may provide:

```text
principal/session
Holding/scope
navigation context
shared design tokens
command/search integration
ContextEnvelope-aware AI surface
Chronica authority lifecycle
evidence navigation
shared identity/resource links
```

The Ops UI must consume these through explicit host contracts rather than reading Chronica internals directly.

## Projection rule

The UI renders Ops/Chronica state; it does not own canonical truth.

```text
browser cache != business truth
optimistic state != execution success
hidden control != authorization
toast != evidence
```

## Effectful UI

Buttons/forms/bulk actions submit the same Ops commands used by API/MCP.

Connected mode:

```text
UI intent
-> Ops command
-> Chronica authority/execution mapping when required
-> result/evidence
-> projection refresh
```

Standalone mode:

```text
UI intent
-> Ops command
-> local policy/authority
-> result/evidence
-> projection refresh
```

## Host interface

A production Ops UI should define a versioned host contract for capabilities such as:

```text
current principal
current scope/Holding
navigation
resource deep-linking
authority status rendering
evidence opening
theme/tokens
locale/timezone
notifications
telemetry hooks
feature/contract version
```

Avoid hard-coding one deployment URL or one parent DOM structure.

## Design-system integration

Chronica-hosted surfaces should conform to the Chronica frontend design/accessibility contracts under `docs/frontend/` while retaining Ops-specific semantic components.

Do not make every Ops visually identical by deleting domain usability. Share primitives/tokens/interaction laws; keep specialized workspaces where needed.

## Independent packaging

The UI may be delivered as:

```text
standalone web app
versioned component/package
route bundle
remote module
other explicit host adapter
```

The exact mechanism is implementation-specific. It must support independent versioning and rollback.

## Security

Embedding must not widen data access. Host-provided identity/scope is an input to authorization, not proof by itself. Server/runtime authorization remains authoritative.

## Acceptance

```text
[ ] same domain UI works standalone
[ ] same domain UI can be hosted by Chronica
[ ] no duplicate business logic between hosts
[ ] shared identity/resource links work
[ ] authority lifecycle is visible
[ ] errors/stale/forbidden/unknown outcome states are visible
[ ] accessibility/i18n remain valid
[ ] connected UI never becomes canonical truth
```
