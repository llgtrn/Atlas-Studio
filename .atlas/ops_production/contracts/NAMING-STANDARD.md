---
id: ops-production-contracts-naming-standard
type: reference
status: active
canonical: true
---
# Ops Naming and Repository Identity Standard

Status: **ACTIVE OPS PRODUCTION CONTRACT — v1**

This contract defines naming, filesystem, package/artifact identity, source-file naming and Chronica/Ops namespace boundaries for every Chronica-related Ops repository.

The purpose is to make all Ops repositories feel like one ecosystem without pretending that an Ops proving repository is already canonical Chronica source ownership.

The core law is:

```text
Repository name        = product / distribution identity
Filesystem path        = architectural + semantic location
Package/artifact name  = globally unique build identity
Source-file name       = local responsibility
Semantic identifier    = meaning
Provider/donor name    = boundary/provenance vocabulary
```

These layers MUST NOT redundantly repeat the same identity unless global uniqueness actually requires it.

A repository path should explain **where** code lives. A package name should explain **what globally identifiable artifact** is being built. A semantic identifier should explain **what the operation means**.

---

## 1. Ops and Chronica are related, not the same source identity

An Ops repository is an incubation/refoundation/product repository. It is not canonical Chronica merely because it follows Chronica architecture.

Therefore local Ops packages MUST use the Ops identity while they remain Ops-owned.

Examples:

```text
Repository: TradingOps
Package:    tradingops-core-market
Package:    tradingops-runtime-execution
Package:    tradingops-adapter-binance

Repository: BnbOps
Package:    bnbops-core-reservation
Package:    bnbops-runtime-pricing
Package:    bnbops-adapter-channex

Repository: HelpdeskOps
Package:    helpdeskops-core-conversation
Package:    helpdeskops-runtime-routing
Package:    helpdeskops-adapter-email
```

Do NOT prematurely name Ops-owned packages:

```text
chronica-core-*
chronica-runtime-*
chronica-adapter-*
```

unless that artifact is actually canonical Chronica-owned source or an explicitly documented mirror/package generated from canonical Chronica ownership.

Chronica namespace adoption follows source/semantic absorption:

```text
Ops proves / refounds
-> Chronica admits reusable semantic/runtime ownership
-> canonical source moves into Chronica
-> artifact becomes chronica-*
```

The inverse is forbidden:

```text
Ops calls itself chronica-*
-> therefore claims canonical ownership
```

Naming never grants canonical authority.

---

## 2. Shared structural grammar across Chronica and Ops

Chronica and Ops SHOULD use the same top-level structural grammar where the responsibilities exist:

```text
<repo>/
├── apps/
├── crates/
│   ├── core/
│   ├── runtime/
│   └── adapter/
├── graph/
├── bindings/
├── migrations/
├── deploy/
├── tools/
├── tests/
├── provenance/
└── docs/
```

Do not create empty directories only to satisfy the diagram.

The structural meaning is:

```text
apps/       thin UI/API/CLI/MCP/product hosts
core/       pure or runtime-independent semantics, value objects, invariants, ports
runtime/    orchestration, durable execution, jobs, recovery, reconciliation
adapter/    provider/protocol/persistence/transport implementation mechanics
graph/      static graph semantics or declarations when appropriate
bindings/   implementation selection and static mapping
deploy/     deployment/bootstrap/runtime packaging
tools/      repository/build/refoundation tooling
provenance/ source, donor, license, admission and acquisition evidence
```

`cap/` is NOT a target architectural layer. Capability remains derived semantic applicability, not a permanent physical namespace.

---

## 3. Filesystem paths do not repeat the repository brand

Inside an Ops repository, paths SHOULD be semantic and responsibility-based.

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

Avoid:

```text
TradingOps/
└── crates/
    └── core/
        └── tradingops-core-market/
```

when the directory hierarchy already provides the repository and layer context.

Likewise, avoid redundant paths such as:

```text
crates/core/chronica-core-helpdesk-sla/
crates/runtime/chronica-runtime-trading-execution/
crates/adapter/chronica-adapter-binance/
```

as a long-run filesystem target.

The globally unique package/artifact name may still include the repository/layer namespace even when the directory path does not.

Example:

```text
path:    crates/core/market/
package: tradingops-core-market
```

---

## 4. Package and artifact naming grammar

### Ops-owned artifacts

Use:

```text
<ops-slug>-<layer>-<domain>[-<component>]
```

where:

```text
<ops-slug>   = tradingops | bnbops | helpdeskops | customerops | ...
<layer>      = core | runtime | adapter | app | tool
<domain>     = stable semantic/domain family
<component>  = optional bounded responsibility
```

Examples:

```text
tradingops-core-market
tradingops-runtime-execution
tradingops-adapter-binance
tradingops-app-mcp

bnbops-core-reservation
bnbops-runtime-pricing
bnbops-adapter-channex

helpdeskops-core-conversation
helpdeskops-runtime-automation
helpdeskops-adapter-postgres
```

### Canonical Chronica artifacts

After canonical absorption, Chronica-owned artifacts use:

```text
chronica-<layer>-<domain>[-<component>]
```

Examples:

```text
chronica-core-identity
chronica-runtime-execution
chronica-adapter-postgres-social
```

A package rename from `<ops-slug>-*` to `chronica-*` MUST correspond to a real ownership change, not branding alone.

---

## 5. Do not preserve historical donor crate boundaries by default

A donor package/module boundary is evidence, not architectural authority.

Do not mechanically transform:

```text
donor package A -> ops-core-A
donor package B -> ops-core-B
donor package C -> ops-core-C
```

if A/B/C are really one coherent owner.

During refoundation ask:

```text
Is this boundary semantically meaningful?
Does it own one responsibility?
Would independent versioning/building/testing be useful?
Or is it donor packaging that should collapse?
```

Prefer fewer coherent owners over hundreds of renamed fragments.

---

## 6. Source-file naming is local, not globally branded

Source files should describe their local responsibility.

Rust:

```text
snake_case.rs
```

Examples:

```text
signing.rs
market.rs
orders.rs
credentials.rs
journal.rs
reconciliation.rs
```

Do NOT use redundant source-file prefixes such as:

```text
chronica_binance_signing.rs
tradingops_market_orders.rs
helpdeskops_conversation_close.rs
```

when the module path already supplies that context.

TypeScript/JavaScript source should follow the repository's configured formatter/linter convention consistently. New generic non-component modules SHOULD prefer lower-kebab-case where no stronger local convention exists. React component files MAY use `PascalCase.tsx`.

---

## 7. Documentation naming

Within an Ops repository, new documentation directories and ordinary documents SHOULD use lower-kebab-case:

```text
docs/architecture/
docs/decisions/
docs/guides/
docs/references/

docs/guides/provider-onboarding.md
docs/decisions/market-order-reconciliation.md
```

Reserve conventional uppercase names for repository/community-standard files such as:

```text
README.md
AGENTS.md
LICENSE
NOTICE
CODEOWNERS
```

Existing Chronica contract filenames such as `SOURCE-ADMISSION.md` are legacy/canonical documentation identities and are not required to be mass-renamed during active refoundation. Naming cleanup must happen in a dedicated migration, not as unrelated churn inside product code work.

---

## 8. Semantic identifiers do not carry repository branding

Semantic IDs represent meaning, not source ownership.

Prefer:

```text
execution.place_order
conversation.close
reservation.confirm
payment.authorize
```

Avoid:

```text
chronica.execution.place_order
tradingops.execution.place_order
bnbops.reservation.confirm
```

when the meaning is intended to be shared/canonical.

Repository/provider identity belongs in provenance, bindings, adapter identity or evidence — not in the semantic operation name.

Provider-specific operations may remain boundary-local:

```text
provider operation: createOrder
canonical semantic: execution.place_order
```

---

## 9. Provider and donor vocabulary stays at the boundary

Provider/donor names are valid in:

```text
adapter modules
provider bindings
compatibility shims
provenance records
migration evidence
provider-specific tests
```

They SHOULD NOT leak into internal canonical semantics when an equivalent Chronica meaning exists.

Example:

```text
crates/adapter/binance/orders.rs
    createOrder
        ↓ map
execution.place_order
```

Likewise a donor term such as `resolve_ticket` may remain in a donor compatibility adapter while internal Ops/Chronica semantics use `conversation.close`.

---

## 10. Donor baseline is immutable evidence; naming applies after intake

For `DEEP_FORK` intake, do not rename the donor before the exact donor baseline is captured and pushed.

Correct sequence:

```text
clone donor @ exact SHA
-> preserve/push real donor baseline
-> census donor
-> source admission
-> semantic classification
-> refoundation
-> naming normalization together with ownership migration
```

The donor baseline preserves original names for provenance.

Naming standardization applies to the refounded Ops-owned implementation, not retroactively to immutable donor history.

---

## 11. Transitional naming debt

Existing repositories may contain mixed names such as:

```text
chronica-*
legacy donor names
old product names
historical cap names
provider names outside adapters
```

Do NOT perform blind mass renames merely to make grep output clean.

Classify each violation:

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

Naming debt should normally be removed together with real semantic/ownership refoundation so that names follow architecture rather than temporarily hiding architectural debt.

---

## 12. New-code freeze rule

From adoption of this contract forward, new Ops-owned code MUST NOT introduce new naming debt.

Do not create new local Ops packages named:

```text
chronica-core-*
chronica-runtime-*
chronica-adapter-*
chronica-cap-*
```

unless the artifact is explicitly canonical Chronica-owned or generated from canonical Chronica ownership.

Do not create new `cap/` ownership.

Do not add repository-brand prefixes to ordinary local source files.

Do not create provider-specific canonical semantic IDs.

Existing debt may be migrated incrementally; new debt is forbidden.

---

## 13. Absorption naming rule

When an Ops semantic/runtime implementation is admitted into canonical Chronica:

```text
Ops path / package
    ↓ semantic + ownership admission
Chronica canonical owner
    ↓ caller migration
Ops compatibility/product projection
```

The canonical Chronica package may use `chronica-*` identity.

The Ops distribution may continue to exist, but it should consume/project the canonical owner rather than preserve a permanent duplicate implementation merely to retain the old Ops package name.

---

## 14. Examples

### TradingOps

```text
TradingOps/
├── apps/
│   └── mcp/
├── crates/
│   ├── core/
│   │   └── market/
│   ├── runtime/
│   │   └── execution/
│   └── adapter/
│       ├── binance/
│       └── ccxt/
├── bindings/
├── graph/
├── provenance/
└── docs/
```

Possible package identities:

```text
tradingops-core-market
tradingops-runtime-execution
tradingops-adapter-binance
tradingops-app-mcp
```

### HelpdeskOps

```text
HelpdeskOps/
├── apps/
│   ├── ui/
│   ├── api/
│   └── mcp/
├── crates/
│   ├── core/
│   │   └── conversation/
│   ├── runtime/
│   │   ├── routing/
│   │   └── automation/
│   └── adapter/
│       ├── postgres/
│       ├── email/
│       └── channex/
└── docs/
```

Possible package identities:

```text
helpdeskops-core-conversation
helpdeskops-runtime-routing
helpdeskops-runtime-automation
helpdeskops-adapter-postgres
```

---

## 15. Review gate

For every new or renamed package/module ask:

```text
1. Does the filesystem path already provide this context?
2. Is the package/artifact name globally unique?
3. Does the name describe current responsibility rather than donor history?
4. Is Chronica namespace used only for canonical Chronica ownership?
5. Is provider vocabulary kept at the boundary?
6. Is the semantic identifier about meaning rather than brand?
7. Are we preserving an obsolete donor/crate boundary unnecessarily?
8. Does this naming change reflect real architecture rather than hide debt?
```

If the answer to 4, 5, 6 or 8 is wrong, stop and correct ownership before treating the rename as complete.

---

## Final rule

> **Use one structural grammar across Chronica and Ops, but keep source identity honest. Paths express architectural location, artifacts carry globally unique repository identity, source files describe local responsibility, semantic IDs describe meaning, and provider/donor names stay at boundaries. Ops does not become canonical Chronica by prefix; `chronica-*` follows real canonical ownership.**
