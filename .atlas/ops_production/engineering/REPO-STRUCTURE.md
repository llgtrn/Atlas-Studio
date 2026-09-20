# Ops Repository and Documentation Structure

This document defines the **incubation repository structure** for an acquired/donor-derived Ops proving ground. It is not the terminal Chronica source layout.

An Ops repo begins from a complete real donor intake, preserves that donor as an auditable pushed Git baseline, then is gradually refounded as a Chronica-compatible product projection while legacy donor ownership is burned down.

Repository and artifact naming is governed by:

```text
docs/ops_production/contracts/NAMING-STANDARD.md
```

The governing distinction is:

```text
Repository name        = product / distribution identity
Filesystem path        = architectural + semantic location
Package/artifact name  = globally unique build identity
Source-file name       = local responsibility
Semantic identifier    = meaning
```

An Ops repository follows Chronica's structural grammar without pretending that Ops-owned source is already canonical Chronica ownership.

## 1. Baseline-first repository law

The first meaningful state of an acquired/deep-forked Ops should be:

```text
Ops Git remote
└── donor baseline commit/tag
    └── complete real donor tracked tree @ exact SHA/tag
```

Then migration proceeds forward:

```text
donor baseline
-> census/provenance/IP/brand records
-> Chronica conflict audit
-> vertical refactor commits
-> caller/test migration
-> donor path deletion
-> legacy counter reduction
-> zero donor-owned legacy implementation
```

Do not keep the donor only in a sibling checkout while committing a generated replacement into Ops. The donor baseline must be part of the Ops repository history and pushed to its remote.

Do not rewrite that baseline away later merely to make the repository look greenfield.

Naming normalization MUST NOT rewrite the donor baseline. Original donor names remain historical/provenance evidence.

## 2. Canonical Ops incubation topology

After refoundation begins, new Ops-owned implementation should converge toward the same structural grammar used by Chronica:

```text
<ops-repo>/
├── apps/
│   ├── ui/                 standalone product UI host when present
│   ├── api/                thin HTTP/API host when present
│   ├── cli/                optional local CLI
│   └── mcp/                MCP process/host when present
├── crates/
│   ├── core/
│   │   └── <domain>/       pure/product-local semantics, values, invariants, stable ports
│   ├── runtime/
│   │   └── <domain>/       orchestration, jobs, durable execution, recovery/reconciliation
│   └── adapter/
│       └── <domain-or-provider>/
│                           persistence, provider, protocol and transport mechanics
├── graph/                  static semantic declarations where appropriate
├── bindings/               implementation selection/static mappings
├── migrations/
├── deploy/
├── tools/
├── tests/
├── provenance/
├── legacy/                 optional temporary migration-only location
└── docs/
```

This is logical responsibility, not a requirement to create empty folders.

`legacy/` is optional and **must never become a permanent subsystem**. It may be useful to make the remaining donor surface explicit during refactor. If used, its expected long-run size is zero.

`cap/` is not a target physical layer. Capability remains derived semantic applicability, not a permanent namespace.

## 3. Path naming versus artifact naming

Filesystem paths should describe architectural location without redundantly repeating the repository brand or layer.

Prefer:

```text
TradingOps/
└── crates/
    ├── core/
    │   └── market/
    ├── runtime/
    │   └── execution/
    └── adapter/
        └── binance/
```

with globally identifiable package names such as:

```text
tradingops-core-market
tradingops-runtime-execution
tradingops-adapter-binance
```

Do not make the filesystem carry the same identity twice:

```text
crates/core/tradingops-core-market/
crates/runtime/tradingops-runtime-execution/
```

when the parent path already supplies repository/layer context.

Likewise, Ops-owned source should not be named `chronica-core-*`, `chronica-runtime-*` or `chronica-adapter-*` merely because it follows Chronica architecture. The `chronica-*` artifact namespace follows real canonical Chronica ownership after absorption.

See `contracts/NAMING-STANDARD.md` for the full naming grammar.

## 4. Complete donor intake before restructuring

The invariant is:

```text
COMPLETE DONOR TREE ACCOUNTED FOR
AND BASELINE PUSHED
BEFORE
DONOR TREE DISSOLVED / MOVED / RENAMED
```

The sequence is:

```text
clone complete donor @ exact SHA
-> import into Ops repo
-> push baseline commit/tag/branch
-> baseline build/test
-> source + schema + UI + docs + brand + license/IP census
-> record Chronica reference SHA
-> conflict audit
-> establish Ops product identity
-> refactor one vertical at a time
```

Selective copying is not full assimilation.

## 5. Chronica conflict audit belongs inside repo evolution

Before introducing a durable local owner for a major responsibility, compare with the current Chronica implementation/docs at an exact SHA.

Check at minimum:

```text
World/resource/state
relations/bindings
Capability Resolution
authority/normativity
execution/evidence/canonicalization
memory/context/sovereignty
machine/safety where relevant
recovery/reconciliation
Fabric boundary semantics
core/runtime/adapter placement
existing crate/package/service ownership
```

The Ops repository may keep product-local semantics, but it must not recreate a second universal Chronica kernel.

Record the result in `ops.manifest.yaml` and/or a current decision/reference document.

## 6. Legacy burn-down topology

Migration is not complete because a clean target directory exists. The old implementation must disappear after callers migrate.

For each vertical:

```text
old donor owner
-> new target owner
-> migrate callers
-> migrate tests/data
-> verify behavior
-> remove old donor owner
```

Track remaining donor ownership across:

```text
modules/files
public callers
DB/migrations
UI routes/components with business ownership
background jobs
policy/authority logic
docs/branding surfaces
```

Terminal condition:

```text
legacy_code_remaining = 0
legacy_public_callers_remaining = 0
legacy_architecture_ownership_remaining = 0
```

Required provenance/license/history remains outside this zero-legacy requirement.

Renaming alone does not count as burn-down when responsibility is unchanged.

## 7. Mandatory Chronica-style docs kernel

Every active Ops repository MUST maintain:

```text
<ops-repo>/docs/
├── README.md
├── INDEX.md
├── TEMPLATE.md
├── architecture/
│   ├── README.md
│   ├── constitution/
│   ├── foundation/
│   ├── governance/
│   ├── integration/
│   └── operations/
├── blueprints/
├── decisions/
├── guides/
└── references/
```

The local docs may own product-local user/workflow meaning, standalone behavior, provider details, UI composition, compatibility, donor/acquisition/provenance facts and migration decisions.

They MUST NOT redefine canonical Chronica semantics for World, Capability Resolution, Authority, Normativity, Execution, Evidence, Memory, Context, Sovereignty or Safety.

New ordinary Ops documents should use lower-kebab-case unless a repository/community convention requires an uppercase standard filename such as `README.md`, `AGENTS.md`, `LICENSE`, `NOTICE` or `CODEOWNERS`.

## 8. Dependency direction during incubation

Target responsibility flow:

```text
apps
  |
  v
runtime/application orchestration
  |
  v
core semantics + ports
  ^
  |
adapters implement provider/persistence/transport mechanics
```

More concretely:

```text
apps/*
   -> crates/runtime/*
   -> crates/core/*

crates/runtime/*
   -> bindings / adapter interfaces
   -> crates/adapter/* at composition/execution boundaries

crates/adapter/*
   -> crates/core/* semantic contracts where required
```

Core should not depend on provider/transport/database implementations.

## 9. One core, many surfaces

UI/API/CLI/MCP/Chronica integration must not each define independent business services.

Surface code stays thin:

```text
parse/validate transport input
resolve principal/scope
call shared semantic/runtime owner
map result/error to transport
```

MCP is a port over the same implementation, not a second core.

## 10. Chronica bridge during incubation

Chronica-specific integration may live behind an adapter/binding boundary for semantic mapping, identity/resource refs, authority calls, WorkRequest/WorkRun correlation, event/evidence publication, context/disclosure projection and sync/reconciliation.

The bridge proves compatibility. It must not become a second domain service layer.

Local package names still use the Ops repository identity until source ownership is actually absorbed into canonical Chronica.

## 11. UI / MCP / database refactor

Donor UI, MCP/API surfaces, DB schema and workers are all part of the burn-down, not exempt legacy islands.

Preserve behavior deliberately, but progressively move business ownership to the intended target owners. Provider/protocol mechanics may remain as adapters when that is their correct final responsibility.

Provider names may remain in adapter/provenance/binding paths. They should not become canonical semantic identifiers when an equivalent shared semantic exists.

## 12. Source-file naming

Source-file names describe local responsibility, not repository branding.

Rust examples:

```text
signing.rs
orders.rs
market.rs
journal.rs
reconciliation.rs
```

Avoid redundant names such as:

```text
chronica_binance_signing.rs
tradingops_market_orders.rs
```

when the path already provides that context.

TypeScript/JavaScript follows the repository's configured formatter/linter convention consistently. React component files may use `PascalCase.tsx`; generic new modules should use one consistent local convention and avoid repository-brand prefixes.

## 13. Semantic identifiers

Semantic operation identifiers encode meaning, not repository ownership.

Prefer:

```text
execution.place_order
conversation.close
reservation.confirm
payment.authorize
```

Avoid brand-prefixed shared semantics such as:

```text
chronica.execution.place_order
tradingops.execution.place_order
```

Repository/provider identity belongs in provenance, bindings, adapter IDs and evidence.

## 14. Naming normalization during refoundation

Do not run a blind whole-repository rename while real architectural ownership is still unclear.

Classify naming debt first:

```text
CORRECT_CANONICAL_IDENTITY
OPS_OWNED_BUT_CHRONICA_PREFIXED
DONOR_LEGACY
PROVIDER_BOUNDARY_VALID
PROVIDER_NAME_LEAK
STALE_LAYER_PREFIX
REDUNDANT_FILESYSTEM_PREFIX
DEAD_OR_DUPLICATE
```

Then migrate by homogeneous cohort when safe.

The preferred sequence is:

```text
classify responsibility
-> move/split/consolidate ownership
-> migrate callers
-> rename path/artifact to match the new owner
-> verify
-> delete obsolete owner
```

Do not use naming changes to hide architecture debt.

## 15. Terminal absorption into Chronica

Once the Ops reaches `ABSORPTION_READY`, the incubation folder model stops being the target.

Use `docs/ops_production/ABSORB-INTO-CHRONICA-GOAL.md` and dissolve retained responsibilities according to canonical Chronica placement:

```text
pure/shared semantics           -> crates/core
durable runtime behavior        -> crates/runtime
provider/protocol mechanics     -> crates/adapter
thin API/CLI/UI/MCP packaging   -> apps
static semantics                -> graph
static implementation mappings  -> bindings
deployment/bootstrap            -> deploy
repo/build tooling              -> tools
```

Only after canonical source ownership moves into Chronica should the corresponding canonical artifact adopt `chronica-*` identity.

Do **not** mechanically reproduce the Ops tree inside Chronica as a permanent product universe.

## 16. Standalone products after absorption

Source convergence does not remove standalone deployability. Chronica may build independent artifacts such as a product server, MCP host, UI or CLI from canonical Chronica owners.

An Ops distribution may continue using its product name even when the implementation it packages is canonically owned by Chronica.

## 17. New-code freeze

From adoption of `contracts/NAMING-STANDARD.md`, new Ops-owned code must not introduce fresh naming debt.

Do not create new local Ops packages named:

```text
chronica-core-*
chronica-runtime-*
chronica-adapter-*
chronica-cap-*
```

unless the artifact is explicitly canonical Chronica-owned or generated from that owner.

Do not create new `cap/` ownership.

Do not brand-prefix ordinary local source files.

Existing debt may migrate incrementally; new debt is forbidden.

## Final rule

> **Real donor code enters Ops Git first. Refactor then proceeds forward, conflict-aware and vertical-by-vertical, until donor-owned implementation reaches zero. Use one structural grammar across Ops and Chronica, but keep identity honest: paths express architectural location, Ops artifacts use the Ops namespace, `chronica-*` follows real canonical absorption, source files describe local responsibility, and semantic IDs describe meaning rather than brand.**
